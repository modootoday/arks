//! `arks issue develop {<number> | <url>}` — linked branches for an issue.
//!
//! Peer: <https://cli.github.com/manual/gh_issue_develop>.
//! GraphQL (`POST /graphql`): `Issue.linkedBranches` and the
//! `createLinkedBranch(input: {issueId, oid, name, repositoryId})` mutation
//! (<https://docs.github.com/en/graphql/reference/issues>). Base commit comes
//! from `--base` (remote branch) or the branch repository's default branch.
//!
//! Partial: `--worktree` refuses; `--checkout` fetches `origin` in-process and
//! switches only when the work tree is clean. Never spawns git/gh.

use super::common::{ctx, fail, has_flag, issue_ref, positionals, usage};
use super::local_git;
use crate::http_min::http_exchange;
use arks_repo::Repository;
use serde_json::{Value, json};
use std::process::ExitCode;

const VALUE_FLAGS: &[&str] = &[
    "-R",
    "--repo",
    "-b",
    "--base",
    "--branch-repo",
    "-n",
    "--name",
    "--worktree",
];

const Q_LINKED: &str = "query IssueLinkedBranches($owner: String!, $name: String!, $number: Int!) { \
repository(owner: $owner, name: $name) { issue(number: $number) { linkedBranches(first: 100) { \
nodes { ref { name repository { nameWithOwner url } } } } } } }";

const Q_CONTEXT: &str = "query IssueDevelopContext($owner: String!, $name: String!, $number: Int!, \
$branchOwner: String!, $branchName: String!, $baseRef: String!) { \
repository(owner: $owner, name: $name) { id issue(number: $number) { id } } \
branchRepository: repository(owner: $branchOwner, name: $branchName) { id nameWithOwner url \
defaultBranchRef { name target { oid } } baseRef: ref(qualifiedName: $baseRef) { target { oid } } } }";

const M_CREATE: &str = "mutation CreateLinkedBranch($issueId: ID!, $oid: GitObjectID!, $name: String, \
$repositoryId: ID) { createLinkedBranch(input: {issueId: $issueId, oid: $oid, name: $name, \
repositoryId: $repositoryId}) { linkedBranch { id ref { name } } } }";

fn graphql(
    base: &str,
    tok: &str,
    op: &str,
    query: &str,
    vars: Value,
    label: &str,
) -> Result<Value, ExitCode> {
    let body = json!({"query": query, "operationName": op, "variables": vars}).to_string();
    match http_exchange(base, "POST", "/graphql", Some(tok), Some(&body)) {
        Ok((code, text)) => {
            let v: Value = serde_json::from_str(&text).unwrap_or(Value::Null);
            if let Some(errs) = v.get("errors").and_then(Value::as_array)
                && let Some(m) = errs
                    .first()
                    .and_then(|e| e.get("message"))
                    .and_then(Value::as_str)
            {
                return Err(fail(label, &format!("GraphQL: {m}")));
            }
            if code >= 400 {
                return Err(fail(label, &format!("HTTP {code}: {text}")));
            }
            Ok(v.get("data").cloned().unwrap_or(Value::Null))
        }
        Err(e) => Err(fail(label, &e)),
    }
}

fn ptr<'a>(v: &'a Value, p: &str) -> Option<&'a str> {
    v.pointer(p).and_then(Value::as_str)
}

pub fn run(args: &[String]) -> ExitCode {
    let label = "issue develop";
    if has_flag(args, &["-h", "--help"]) {
        eprintln!(
            "arks issue develop {{<number> | <url>}} [--list] [--name N] [--base B] [--branch-repo O/R] [--checkout]"
        );
        return ExitCode::SUCCESS;
    }
    if crate::flag_value(args, &["--worktree"]).is_some() {
        return usage(label, "--worktree is not implemented (Partial)");
    }
    let mut c = match ctx(args, label) {
        Ok(c) => c,
        Err(code) => return code,
    };
    let pos = positionals(args, VALUE_FLAGS);
    let Some((slug, number)) = pos.first().and_then(|a| issue_ref(a)) else {
        return usage(label, "{<number> | <url>} required");
    };
    if let Some((o, r)) = slug {
        c.owner = o;
        c.repo = r;
    }
    if has_flag(args, &["--list", "-l"]) {
        let vars = json!({"owner": c.owner, "name": c.repo, "number": number});
        let data = match graphql(
            &c.base,
            &c.tok,
            "IssueLinkedBranches",
            Q_LINKED,
            vars,
            label,
        ) {
            Ok(d) => d,
            Err(code) => return code,
        };
        let nodes = data
            .pointer("/repository/issue/linkedBranches/nodes")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        for n in &nodes {
            println!(
                "{}\t{}",
                ptr(n, "/ref/name").unwrap_or(""),
                ptr(n, "/ref/repository/url").unwrap_or("")
            );
        }
        return ExitCode::SUCCESS;
    }
    let branch_repo = crate::flag_value(args, &["--branch-repo"])
        .map(|s| {
            s.trim_end_matches('/')
                .trim_end_matches(".git")
                .rsplitn(3, '/')
                .take(2)
                .collect::<Vec<_>>()
        })
        .and_then(|v| (v.len() == 2).then(|| (v[1].to_string(), v[0].to_string())))
        .unwrap_or_else(|| (c.owner.clone(), c.repo.clone()));
    let base = crate::flag_value(args, &["--base", "-b"]);
    let vars = json!({
        "owner": c.owner, "name": c.repo, "number": number,
        "branchOwner": branch_repo.0, "branchName": branch_repo.1,
        "baseRef": base.map(|b| format!("refs/heads/{b}")).unwrap_or_else(|| "HEAD".into()),
    });
    let data = match graphql(
        &c.base,
        &c.tok,
        "IssueDevelopContext",
        Q_CONTEXT,
        vars,
        label,
    ) {
        Ok(d) => d,
        Err(code) => return code,
    };
    let oid = if base.is_some() {
        ptr(&data, "/branchRepository/baseRef/target/oid")
    } else {
        ptr(&data, "/branchRepository/defaultBranchRef/target/oid")
    };
    let Some(oid) = oid.map(str::to_string) else {
        return fail(
            label,
            &format!(
                "could not resolve base branch `{}`",
                base.unwrap_or("(default)")
            ),
        );
    };
    let (Some(issue_id), Some(repo_id)) = (
        ptr(&data, "/repository/issue/id").map(str::to_string),
        ptr(&data, "/branchRepository/id").map(str::to_string),
    ) else {
        return fail(label, "issue or repository not found");
    };
    let vars = json!({
        "issueId": issue_id,
        "oid": oid,
        "name": crate::flag_value(args, &["--name", "-n"]),
        "repositoryId": repo_id,
    });
    let data = match graphql(&c.base, &c.tok, "CreateLinkedBranch", M_CREATE, vars, label) {
        Ok(d) => d,
        Err(code) => return code,
    };
    let Some(name) = ptr(&data, "/createLinkedBranch/linkedBranch/ref/name").map(str::to_string)
    else {
        return fail(label, "createLinkedBranch returned no branch");
    };
    println!("{}/{}\t{name}", branch_repo.0, branch_repo.1);
    if has_flag(args, &["--checkout", "-c"]) {
        return checkout(&name, &oid, label);
    }
    ExitCode::SUCCESS
}

/// Fetch `origin` and switch to the new branch (clean work tree only).
fn checkout(name: &str, oid_hex: &str, label: &str) -> ExitCode {
    let cwd = std::env::current_dir().unwrap_or_else(|_| ".".into());
    let mut repo = match Repository::discover(&cwd) {
        Ok(r) => r,
        Err(e) => return fail(label, &format!("--checkout needs a local clone: {e}")),
    };
    match local_git::is_dirty(&repo) {
        Ok(false) => {}
        Ok(true) => return fail(label, "can't check out: the work tree has local changes"),
        Err(e) => return fail(label, &e),
    }
    if let Err(e) = local_git::fetch(&mut repo, "origin") {
        return fail(label, &e);
    }
    let Ok(oid) = arks_oid::Oid::from_hex(oid_hex) else {
        return fail(label, "bad commit id from forge");
    };
    let tracking = format!("refs/remotes/origin/{name}");
    let tip = repo.refs().resolve(&tracking).unwrap_or(oid);
    let full = format!("refs/heads/{name}");
    if let Err(e) = repo.refs_mut().update(&full, &tip, None) {
        return fail(label, &e.to_string());
    }
    let work = match repo.work_tree() {
        Some(w) => w.to_path_buf(),
        None => return fail(label, "bare repository has no work tree"),
    };
    if let Err(e) = arks_remote::checkout_commit(&repo, &work, &tip) {
        return fail(label, &e.to_string());
    }
    if let Err(e) = repo.refs_mut().set_symbolic("HEAD", &full) {
        return fail(label, &e.to_string());
    }
    eprintln!("Switched to branch '{name}'");
    ExitCode::SUCCESS
}
