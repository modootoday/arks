//! GitHub Actions REST API v3 client leaves (`workflow`, `run`, `cache`, `secret`, `variable`).
//!
//! Implements **E2** from `sot:root/v0-2-0-scope-expansion`:
//! - `workflow`: `list`, `view`, `run`, `enable`, `disable`
//! - `run`: `list`, `view`, `watch`, `rerun`, `cancel`, `delete`, `download` (with zip-slip rejection)
//! - `cache`: `list`, `delete`
//! - `variable`: `list`, `get`, `set`, `delete` (repo / env / org)
//! - `secret`: `list`, `set` (pure-Rust NaCl sealed box), `delete` (repo / env / org)
//!
//! Never spawns `git` or `gh`.

use super::common::enc;
use super::{Args, Ctx, Fail, Res, arr, emit, n, s, usage};
use serde_json::{Value, json};
use std::io::{Read, Write};
use std::path::{Component, Path, PathBuf};
use std::time::Duration;

pub(crate) const WORKFLOW_HELP: &str = "\
arks workflow — GitHub Actions workflows (E2)

USAGE:
  arks workflow list [-R owner/repo] [-a | --all] [-L <limit>] [--json <fields>]
  arks workflow view <workflow-id|filename> [-R owner/repo] [--json <fields>]
  arks workflow run <workflow-id|filename> [-R owner/repo] [-r <ref>] [-f key=value]...
  arks workflow enable <workflow-id|filename> [-R owner/repo]
  arks workflow disable <workflow-id|filename> [-R owner/repo]";

pub(crate) const RUN_HELP: &str = "\
arks run — GitHub Actions workflow runs (E2)

USAGE:
  arks run list [-R owner/repo] [-w <workflow>] [-b <branch>] [-s <status>] [-L <limit>] [--json <fields>]
  arks run view <run-id> [-R owner/repo] [--log | --log-failed] [--json <fields>]
  arks run watch <run-id> [-R owner/repo] [-i <seconds>] [--exit-status]
  arks run rerun <run-id> [-R owner/repo] [--failed]
  arks run cancel <run-id> [-R owner/repo]
  arks run delete <run-id> [-R owner/repo]
  arks run download <run-id> [-R owner/repo] [-n <name>] [-D <dir>]";

pub(crate) const CACHE_HELP: &str = "\
arks cache — GitHub Actions caches (E2)

USAGE:
  arks cache list [-R owner/repo] [-k <key-prefix>] [-r <ref>] [-L <limit>] [--json <fields>]
  arks cache delete <cache-id|key> [-R owner/repo]";

pub(crate) const VARIABLE_HELP: &str = "\
arks variable — GitHub Actions variables (E2)

USAGE:
  arks variable list [-R owner/repo] [-e <env> | -o <org>] [--json <fields>]
  arks variable get <name> [-R owner/repo] [-e <env> | -o <org>]
  arks variable set <name> [-b <body>] [-R owner/repo] [-e <env> | -o <org>]
  arks variable delete <name> [-R owner/repo] [-e <env> | -o <org>]";

pub(crate) const SECRET_HELP: &str = "\
arks secret — GitHub Actions secrets with client-side libsodium SealedBox encryption (E2)

USAGE:
  arks secret list [-R owner/repo] [-e <env> | -o <org>] [--json <fields>]
  arks secret set <name> [-b <body>] [-R owner/repo] [-e <env> | -o <org>]
  arks secret delete <name> [-R owner/repo] [-e <env> | -o <org>]";

pub(crate) fn workflow(ctx: &Ctx, args: &[String], out: &mut dyn Write) -> Res {
    let sub = args.first().map(String::as_str).unwrap_or("");
    let raw = args.get(1..).unwrap_or(&[]);
    match sub {
        "" | "-h" | "--help" | "help" => Err(Fail::Help),
        "list" | "ls" => {
            let a = Args::parse(
                raw,
                &[
                    "-R",
                    "--repo",
                    "-L",
                    "--limit",
                    "--json",
                    "-q",
                    "--jq",
                    "-t",
                    "--template",
                ],
                &["-a", "--all"],
            )?;
            let (o, r) = ctx.repo(&a)?;
            let limit = parse_limit(a.get(&["-L", "--limit"]), 50)?;
            let v = ctx.api(
                "GET",
                &format!("/repos/{o}/{r}/actions/workflows?per_page={limit}"),
                None,
            )?;
            let mut items: Vec<Value> = arr(v.get("workflows").unwrap_or(&v)).to_vec();
            if !a.has(&["-a", "--all"]) {
                items.retain(|w| s(w, "state") == "active");
            }
            if emit_if_json(&a, &Value::Array(items.clone()), out)? {
                return Ok(());
            }
            for w in &items {
                writeln!(out, "{}\t{}\t{}", s(w, "name"), s(w, "state"), n(w, "id"))?;
            }
            Ok(())
        }
        "view" => {
            let a = Args::parse(
                raw,
                &["-R", "--repo", "--json", "-q", "--jq", "-t", "--template"],
                &["-w", "--web"],
            )?;
            let id = a.pos.first().ok_or_else(|| {
                Fail::Usage("workflow view requires <workflow-id|filename>".into())
            })?;
            let (o, r) = ctx.repo(&a)?;
            let v = ctx.api(
                "GET",
                &format!("/repos/{o}/{r}/actions/workflows/{}", enc(id)),
                None,
            )?;
            if emit_if_json(&a, &v, out)? {
                return Ok(());
            }
            writeln!(out, "{} - {}", s(&v, "name"), s(&v, "path"))?;
            writeln!(out, "ID: {}", n(&v, "id"))?;
            writeln!(out, "State: {}", s(&v, "state"))?;
            Ok(())
        }
        "run" => {
            let a = Args::parse(
                raw,
                &[
                    "-R",
                    "--repo",
                    "-r",
                    "--ref",
                    "-f",
                    "--raw-field",
                    "-F",
                    "--field",
                ],
                &[],
            )?;
            let id = a.pos.first().ok_or_else(|| {
                Fail::Usage("workflow run requires <workflow-id|filename>".into())
            })?;
            let (o, r) = ctx.repo(&a)?;
            let git_ref = a.get(&["-r", "--ref"]).unwrap_or("main");
            let mut inputs = serde_json::Map::new();
            for kv in a.get_all(&["-f", "--raw-field", "-F", "--field"]) {
                let (k, v) = kv
                    .split_once('=')
                    .ok_or_else(|| Fail::Usage(format!("expected key=value, got {kv:?}")))?;
                inputs.insert(k.to_string(), Value::String(v.to_string()));
            }
            let body = if inputs.is_empty() {
                json!({ "ref": git_ref })
            } else {
                json!({ "ref": git_ref, "inputs": Value::Object(inputs) })
            };
            ctx.api(
                "POST",
                &format!("/repos/{o}/{r}/actions/workflows/{}/dispatches", enc(id)),
                Some(&body),
            )?;
            writeln!(
                out,
                "✓ Created workflow_dispatch event for {id} at {git_ref}"
            )?;
            Ok(())
        }
        "enable" | "disable" => {
            let a = Args::parse(raw, &["-R", "--repo"], &[])?;
            let id = a.pos.first().ok_or_else(|| {
                Fail::Usage(format!("workflow {sub} requires <workflow-id|filename>"))
            })?;
            let (o, r) = ctx.repo(&a)?;
            ctx.api(
                "PUT",
                &format!("/repos/{o}/{r}/actions/workflows/{}/{sub}", enc(id)),
                None,
            )?;
            writeln!(out, "✓ {sub}d workflow {id}")?;
            Ok(())
        }
        other => usage(format!("unknown workflow subcommand {other:?}")),
    }
}

pub(crate) fn run(ctx: &Ctx, args: &[String], out: &mut dyn Write) -> Res {
    let sub = args.first().map(String::as_str).unwrap_or("");
    let raw = args.get(1..).unwrap_or(&[]);
    match sub {
        "" | "-h" | "--help" | "help" => Err(Fail::Help),
        "list" | "ls" => {
            let a = Args::parse(
                raw,
                &[
                    "-R",
                    "--repo",
                    "-w",
                    "--workflow",
                    "-b",
                    "--branch",
                    "-s",
                    "--status",
                    "-L",
                    "--limit",
                    "--json",
                    "-q",
                    "--jq",
                    "-t",
                    "--template",
                ],
                &[],
            )?;
            let (o, r) = ctx.repo(&a)?;
            let limit = parse_limit(a.get(&["-L", "--limit"]), 20)?;
            let mut qs = vec![format!("per_page={limit}")];
            if let Some(b) = a.get(&["-b", "--branch"]) {
                qs.push(format!("branch={}", enc(b)));
            }
            if let Some(st) = a.get(&["-s", "--status"]) {
                qs.push(format!("status={}", enc(st)));
            }
            let base_path = match a.get(&["-w", "--workflow"]) {
                Some(wf) => format!("/repos/{o}/{r}/actions/workflows/{}/runs", enc(wf)),
                None => format!("/repos/{o}/{r}/actions/runs"),
            };
            let v = ctx.api("GET", &format!("{base_path}?{}", qs.join("&")), None)?;
            let items: Vec<Value> = arr(v.get("workflow_runs").unwrap_or(&v)).to_vec();
            if emit_if_json(&a, &Value::Array(items.clone()), out)? {
                return Ok(());
            }
            for item in &items {
                writeln!(
                    out,
                    "{}\t{}\t{}\t{}\t{}\t{}",
                    s(item, "status"),
                    s(item, "conclusion"),
                    s(item, "name"),
                    s(item, "head_branch"),
                    s(item, "event"),
                    n(item, "id")
                )?;
            }
            Ok(())
        }
        "view" => {
            let a = Args::parse(
                raw,
                &["-R", "--repo", "--json", "-q", "--jq", "-t", "--template"],
                &["--log", "--log-failed", "-v", "--verbose"],
            )?;
            let id = a
                .pos
                .first()
                .ok_or_else(|| Fail::Usage("run view requires <run-id>".into()))?;
            let (o, r) = ctx.repo(&a)?;
            if a.has(&["--log"]) || a.has(&["--log-failed"]) {
                let bytes = ctx.raw(
                    "GET",
                    &format!("/repos/{o}/{r}/actions/runs/{}/logs", enc(id)),
                    None,
                    "application/octet-stream",
                    "*/*",
                )?;
                out.write_all(&bytes)?;
                return Ok(());
            }
            let v = ctx.api(
                "GET",
                &format!("/repos/{o}/{r}/actions/runs/{}", enc(id)),
                None,
            )?;
            if emit_if_json(&a, &v, out)? {
                return Ok(());
            }
            writeln!(
                out,
                "{} #{} ({}) — status={} conclusion={}",
                s(&v, "name"),
                n(&v, "run_number"),
                n(&v, "id"),
                s(&v, "status"),
                s(&v, "conclusion")
            )?;
            Ok(())
        }
        "watch" => {
            let a = Args::parse(
                raw,
                &["-R", "--repo", "-i", "--interval"],
                &["--exit-status"],
            )?;
            let id = a
                .pos
                .first()
                .ok_or_else(|| Fail::Usage("run watch requires <run-id>".into()))?;
            let interval_secs: u64 = match a.get(&["-i", "--interval"]) {
                Some(v) => v
                    .parse()
                    .map_err(|_| Fail::Usage(format!("invalid --interval {v:?}")))?,
                None => 3,
            };
            let (o, r) = ctx.repo(&a)?;
            let path = format!("/repos/{o}/{r}/actions/runs/{}", enc(id));
            loop {
                let v = ctx.api("GET", &path, None)?;
                let status = s(&v, "status");
                let conclusion = s(&v, "conclusion");
                if status == "completed" {
                    writeln!(out, "✓ Run {id} completed with conclusion: {conclusion}")?;
                    if a.has(&["--exit-status"]) && conclusion != "success" {
                        return Err(Fail::Exit(1));
                    }
                    return Ok(());
                }
                if interval_secs > 0 {
                    std::thread::sleep(Duration::from_secs(interval_secs));
                }
            }
        }
        "rerun" => {
            let a = Args::parse(raw, &["-R", "--repo"], &["--failed", "-d", "--debug"])?;
            let id = a
                .pos
                .first()
                .ok_or_else(|| Fail::Usage("run rerun requires <run-id>".into()))?;
            let (o, r) = ctx.repo(&a)?;
            let suffix = if a.has(&["--failed"]) {
                "rerun-failed-jobs"
            } else {
                "rerun"
            };
            ctx.api(
                "POST",
                &format!("/repos/{o}/{r}/actions/runs/{}/{suffix}", enc(id)),
                None,
            )?;
            writeln!(out, "✓ Requested {suffix} for run {id}")?;
            Ok(())
        }
        "cancel" => {
            let a = Args::parse(raw, &["-R", "--repo"], &[])?;
            let id = a
                .pos
                .first()
                .ok_or_else(|| Fail::Usage("run cancel requires <run-id>".into()))?;
            let (o, r) = ctx.repo(&a)?;
            ctx.api(
                "POST",
                &format!("/repos/{o}/{r}/actions/runs/{}/cancel", enc(id)),
                None,
            )?;
            writeln!(out, "✓ Request to cancel workflow run {id} submitted.")?;
            Ok(())
        }
        "delete" => {
            let a = Args::parse(raw, &["-R", "--repo"], &[])?;
            let id = a
                .pos
                .first()
                .ok_or_else(|| Fail::Usage("run delete requires <run-id>".into()))?;
            let (o, r) = ctx.repo(&a)?;
            ctx.api(
                "DELETE",
                &format!("/repos/{o}/{r}/actions/runs/{}", enc(id)),
                None,
            )?;
            writeln!(out, "✓ Deleted workflow run {id}")?;
            Ok(())
        }
        "download" => {
            let a = Args::parse(raw, &["-R", "--repo", "-n", "--name", "-D", "--dir"], &[])?;
            let id = a
                .pos
                .first()
                .ok_or_else(|| Fail::Usage("run download requires <run-id>".into()))?;
            let (o, r) = ctx.repo(&a)?;
            let dest_root = PathBuf::from(a.get(&["-D", "--dir"]).unwrap_or("."));
            let name_filter = a.get(&["-n", "--name"]);
            let list = ctx.api(
                "GET",
                &format!("/repos/{o}/{r}/actions/runs/{}/artifacts", enc(id)),
                None,
            )?;
            let artifacts = arr(list.get("artifacts").unwrap_or(&list));
            let mut downloaded = 0usize;
            for art in artifacts {
                let art_name = s(art, "name");
                if let Some(want) = name_filter {
                    if art_name != want {
                        continue;
                    }
                }
                let art_id = n(art, "id");
                let zip_bytes = ctx.raw(
                    "GET",
                    &format!("/repos/{o}/{r}/actions/artifacts/{art_id}/zip"),
                    None,
                    "application/octet-stream",
                    "*/*",
                )?;
                let target_dir = if name_filter.is_some() {
                    dest_root.clone()
                } else {
                    let safe_name = validate_archive_entry_path(art_name).map_err(Fail::Api)?;
                    dest_root.join(safe_name)
                };
                extract_zip_safe(&zip_bytes, &target_dir).map_err(Fail::Api)?;
                downloaded += 1;
            }
            if downloaded == 0 {
                return Err(Fail::Api("no matching artifacts found for run".into()));
            }
            writeln!(
                out,
                "✓ Downloaded {downloaded} artifact(s) to {}",
                dest_root.display()
            )?;
            Ok(())
        }
        other => usage(format!("unknown run subcommand {other:?}")),
    }
}

pub(crate) fn cache(ctx: &Ctx, args: &[String], out: &mut dyn Write) -> Res {
    let sub = args.first().map(String::as_str).unwrap_or("");
    let raw = args.get(1..).unwrap_or(&[]);
    match sub {
        "" | "-h" | "--help" | "help" => Err(Fail::Help),
        "list" | "ls" => {
            let a = Args::parse(
                raw,
                &[
                    "-R",
                    "--repo",
                    "-k",
                    "--key",
                    "-r",
                    "--ref",
                    "-L",
                    "--limit",
                    "--json",
                    "-q",
                    "--jq",
                    "-t",
                    "--template",
                ],
                &[],
            )?;
            let (o, r) = ctx.repo(&a)?;
            let limit = parse_limit(a.get(&["-L", "--limit"]), 30)?;
            let mut qs = vec![format!("per_page={limit}")];
            if let Some(k) = a.get(&["-k", "--key"]) {
                qs.push(format!("key={}", enc(k)));
            }
            if let Some(rf) = a.get(&["-r", "--ref"]) {
                qs.push(format!("ref={}", enc(rf)));
            }
            let v = ctx.api(
                "GET",
                &format!("/repos/{o}/{r}/actions/caches?{}", qs.join("&")),
                None,
            )?;
            let items: Vec<Value> = arr(v.get("actions_caches").unwrap_or(&v)).to_vec();
            if emit_if_json(&a, &Value::Array(items.clone()), out)? {
                return Ok(());
            }
            for item in &items {
                writeln!(
                    out,
                    "{}\t{}\t{}\t{}",
                    n(item, "id"),
                    s(item, "key"),
                    n(item, "size_in_bytes"),
                    s(item, "ref")
                )?;
            }
            Ok(())
        }
        "delete" => {
            let a = Args::parse(raw, &["-R", "--repo"], &["--succeed-on-no-caches"])?;
            let id_or_key = a
                .pos
                .first()
                .ok_or_else(|| Fail::Usage("cache delete requires <cache-id|key>".into()))?;
            let (o, r) = ctx.repo(&a)?;
            let path = if id_or_key.chars().all(|c| c.is_ascii_digit()) {
                format!("/repos/{o}/{r}/actions/caches/{id_or_key}")
            } else {
                format!("/repos/{o}/{r}/actions/caches?key={}", enc(id_or_key))
            };
            ctx.api("DELETE", &path, None)?;
            writeln!(out, "✓ Deleted cache {id_or_key}")?;
            Ok(())
        }
        other => usage(format!("unknown cache subcommand {other:?}")),
    }
}

pub(crate) fn variable(ctx: &Ctx, args: &[String], out: &mut dyn Write) -> Res {
    let sub = args.first().map(String::as_str).unwrap_or("");
    let raw = args.get(1..).unwrap_or(&[]);
    match sub {
        "" | "-h" | "--help" | "help" => Err(Fail::Help),
        "list" | "ls" => {
            let a = Args::parse(
                raw,
                &[
                    "-R",
                    "--repo",
                    "-e",
                    "--env",
                    "-o",
                    "--org",
                    "--json",
                    "-q",
                    "--jq",
                    "-t",
                    "--template",
                ],
                &[],
            )?;
            let base = scope_base(ctx, &a, "variables")?;
            let v = ctx.api("GET", &base, None)?;
            let items: Vec<Value> = arr(v.get("variables").unwrap_or(&v)).to_vec();
            if emit_if_json(&a, &Value::Array(items.clone()), out)? {
                return Ok(());
            }
            for item in &items {
                writeln!(
                    out,
                    "{}\t{}\t{}",
                    s(item, "name"),
                    s(item, "value"),
                    s(item, "updated_at")
                )?;
            }
            Ok(())
        }
        "get" => {
            let a = Args::parse(
                raw,
                &[
                    "-R",
                    "--repo",
                    "-e",
                    "--env",
                    "-o",
                    "--org",
                    "--json",
                    "-q",
                    "--jq",
                    "-t",
                    "--template",
                ],
                &[],
            )?;
            let name = a
                .pos
                .first()
                .ok_or_else(|| Fail::Usage("variable get requires <name>".into()))?;
            let base = scope_base(ctx, &a, "variables")?;
            let v = ctx.api("GET", &format!("{base}/{}", enc(name)), None)?;
            if emit_if_json(&a, &v, out)? {
                return Ok(());
            }
            writeln!(out, "{}", s(&v, "value"))?;
            Ok(())
        }
        "set" => {
            let a = Args::parse(
                raw,
                &["-R", "--repo", "-e", "--env", "-o", "--org", "-b", "--body"],
                &[],
            )?;
            let name = a
                .pos
                .first()
                .ok_or_else(|| Fail::Usage("variable set requires <name>".into()))?;
            let val = read_value_or_stdin(a.get(&["-b", "--body"]), "variable")?;
            let base = scope_base(ctx, &a, "variables")?;
            let body = json!({ "name": name, "value": val });
            // Try PATCH update first; fall back to POST create if not found.
            match ctx.api("PATCH", &format!("{base}/{}", enc(name)), Some(&body)) {
                Ok(_) => {}
                Err(Fail::Api(msg)) if msg.contains("HTTP 404") => {
                    ctx.api("POST", &base, Some(&body))?;
                }
                Err(e) => return Err(e),
            }
            writeln!(out, "✓ Set variable {name}")?;
            Ok(())
        }
        "delete" | "remove" => {
            let a = Args::parse(raw, &["-R", "--repo", "-e", "--env", "-o", "--org"], &[])?;
            let name = a
                .pos
                .first()
                .ok_or_else(|| Fail::Usage("variable delete requires <name>".into()))?;
            let base = scope_base(ctx, &a, "variables")?;
            ctx.api("DELETE", &format!("{base}/{}", enc(name)), None)?;
            writeln!(out, "✓ Deleted variable {name}")?;
            Ok(())
        }
        other => usage(format!("unknown variable subcommand {other:?}")),
    }
}

pub(crate) fn secret(ctx: &Ctx, args: &[String], out: &mut dyn Write) -> Res {
    let sub = args.first().map(String::as_str).unwrap_or("");
    let raw = args.get(1..).unwrap_or(&[]);
    match sub {
        "" | "-h" | "--help" | "help" => Err(Fail::Help),
        "list" | "ls" => {
            let a = Args::parse(
                raw,
                &[
                    "-R",
                    "--repo",
                    "-e",
                    "--env",
                    "-o",
                    "--org",
                    "--json",
                    "-q",
                    "--jq",
                    "-t",
                    "--template",
                ],
                &[],
            )?;
            let base = scope_base(ctx, &a, "secrets")?;
            let v = ctx.api("GET", &base, None)?;
            let items: Vec<Value> = arr(v.get("secrets").unwrap_or(&v)).to_vec();
            if emit_if_json(&a, &Value::Array(items.clone()), out)? {
                return Ok(());
            }
            for item in &items {
                writeln!(out, "{}\t{}", s(item, "name"), s(item, "updated_at"))?;
            }
            Ok(())
        }
        "set" => {
            let a = Args::parse(
                raw,
                &["-R", "--repo", "-e", "--env", "-o", "--org", "-b", "--body"],
                &[],
            )?;
            let name = a
                .pos
                .first()
                .ok_or_else(|| Fail::Usage("secret set requires <name>".into()))?;
            let plaintext = read_value_or_stdin(a.get(&["-b", "--body"]), "secret")?;
            let base = scope_base(ctx, &a, "secrets")?;
            let pk = ctx.api("GET", &format!("{base}/public-key"), None)?;
            let key_id = s(&pk, "key_id");
            let key_b64 = s(&pk, "key");
            if key_id.is_empty() || key_b64.is_empty() {
                return Err(Fail::Api(
                    "public-key response missing required key_id or key".into(),
                ));
            }
            let encrypted_value =
                crate::sealed_box::seal_secret_base64(key_b64, plaintext.as_bytes())
                    .map_err(Fail::Api)?;
            let body = json!({
                "encrypted_value": encrypted_value,
                "key_id": key_id,
            });
            ctx.api("PUT", &format!("{base}/{}", enc(name)), Some(&body))?;
            writeln!(out, "✓ Set secret {name}")?;
            Ok(())
        }
        "delete" | "remove" => {
            let a = Args::parse(raw, &["-R", "--repo", "-e", "--env", "-o", "--org"], &[])?;
            let name = a
                .pos
                .first()
                .ok_or_else(|| Fail::Usage("secret delete requires <name>".into()))?;
            let base = scope_base(ctx, &a, "secrets")?;
            ctx.api("DELETE", &format!("{base}/{}", enc(name)), None)?;
            writeln!(out, "✓ Deleted secret {name}")?;
            Ok(())
        }
        other => usage(format!("unknown secret subcommand {other:?}")),
    }
}

fn emit_if_json(a: &Args<'_>, v: &Value, out: &mut dyn Write) -> Result<bool, Fail> {
    if a.get(&["--json"]).is_some()
        || a.get(&["-q", "--jq"]).is_some()
        || a.get(&["-t", "--template"]).is_some()
    {
        emit(out, v, a.get(&["-q", "--jq"]), a.get(&["-t", "--template"]))?;
        return Ok(true);
    }
    Ok(false)
}

fn scope_base(ctx: &Ctx, a: &Args<'_>, kind: &str) -> Result<String, Fail> {
    if let Some(org) = a.get(&["-o", "--org"]) {
        if a.get(&["-e", "--env"]).is_some() {
            return Err(Fail::Usage("cannot combine --org and --env".into()));
        }
        return Ok(format!("/orgs/{}/actions/{kind}", enc(org)));
    }
    let (o, r) = ctx.repo(a)?;
    if let Some(env_name) = a.get(&["-e", "--env"]) {
        Ok(format!(
            "/repos/{o}/{r}/environments/{}/{kind}",
            enc(env_name)
        ))
    } else {
        Ok(format!("/repos/{o}/{r}/actions/{kind}"))
    }
}

fn parse_limit(raw: Option<&str>, default: usize) -> Result<usize, Fail> {
    match raw {
        Some(v) => {
            let n: usize = v
                .parse()
                .map_err(|_| Fail::Usage(format!("invalid --limit {v:?}")))?;
            if n == 0 || n > 100 {
                return Err(Fail::Usage("--limit must be between 1 and 100".into()));
            }
            Ok(n)
        }
        None => Ok(default),
    }
}

fn read_value_or_stdin(flag_val: Option<&str>, label: &str) -> Result<String, Fail> {
    if let Some(v) = flag_val {
        return Ok(v.to_string());
    }
    let mut buf = String::new();
    std::io::stdin().read_to_string(&mut buf)?;
    let trimmed = buf.trim_end_matches(&['\r', '\n'][..]).to_string();
    if trimmed.is_empty() {
        return Err(Fail::Usage(format!(
            "expected {label} value via --body or standard input"
        )));
    }
    Ok(trimmed)
}

/// Strictly validate an archive entry path against zip-slip traversal.
/// Rejects empty paths, NUL bytes, backslashes, leading `/`, Windows drive prefixes,
/// and any `..` parent-directory components.
pub(crate) fn validate_archive_entry_path(raw: &str) -> Result<PathBuf, String> {
    if raw.is_empty() || raw.contains('\0') {
        return Err(format!("refusing invalid archive entry path: {raw:?}"));
    }
    if raw.starts_with('/') || raw.contains('\\') {
        return Err(format!(
            "refusing unsafe archive entry path (zip-slip): {raw:?}"
        ));
    }
    if raw.as_bytes().get(1) == Some(&b':') {
        return Err(format!(
            "refusing drive-qualified archive entry path: {raw:?}"
        ));
    }
    let mut clean = PathBuf::new();
    for comp in Path::new(raw).components() {
        match comp {
            Component::Normal(part) => clean.push(part),
            Component::CurDir => {}
            Component::ParentDir | Component::RootDir | Component::Prefix(_) => {
                return Err(format!(
                    "refusing archive entry path escaping destination (zip-slip): {raw:?}"
                ));
            }
        }
    }
    if clean.as_os_str().is_empty() {
        return Err(format!("refusing empty normalized archive path: {raw:?}"));
    }
    Ok(clean)
}

/// Minimal pure-Rust ZIP extractor supporting stored (0) and deflate (8) entries
/// with strict zip-slip validation on every entry before writing to disk.
pub(crate) fn extract_zip_safe(zip_bytes: &[u8], dest_dir: &Path) -> Result<(), String> {
    std::fs::create_dir_all(dest_dir).map_err(|e| format!("create {}: {e}", dest_dir.display()))?;
    let mut pos = 0usize;
    let mut extracted = 0usize;
    while pos + 30 <= zip_bytes.len() {
        let sig = u32::from_le_bytes([
            zip_bytes[pos],
            zip_bytes[pos + 1],
            zip_bytes[pos + 2],
            zip_bytes[pos + 3],
        ]);
        if sig != 0x0403_4b50 {
            break;
        }
        let method = u16::from_le_bytes([zip_bytes[pos + 8], zip_bytes[pos + 9]]);
        let comp_size = u32::from_le_bytes([
            zip_bytes[pos + 18],
            zip_bytes[pos + 19],
            zip_bytes[pos + 20],
            zip_bytes[pos + 21],
        ]) as usize;
        let name_len = u16::from_le_bytes([zip_bytes[pos + 26], zip_bytes[pos + 27]]) as usize;
        let extra_len = u16::from_le_bytes([zip_bytes[pos + 28], zip_bytes[pos + 29]]) as usize;
        let name_start = pos + 30;
        let data_start = name_start + name_len + extra_len;
        let data_end = data_start
            .checked_add(comp_size)
            .ok_or_else(|| "zip entry size overflow".to_string())?;
        if data_end > zip_bytes.len() {
            return Err("truncated zip entry".into());
        }
        let name = std::str::from_utf8(&zip_bytes[name_start..name_start + name_len])
            .map_err(|e| format!("non-UTF-8 zip entry name: {e}"))?;
        let is_dir = name.ends_with('/');
        let rel = validate_archive_entry_path(name.trim_end_matches('/'))?;
        let out_path = dest_dir.join(&rel);
        if is_dir {
            std::fs::create_dir_all(&out_path)
                .map_err(|e| format!("mkdir {}: {e}", out_path.display()))?;
        } else {
            if let Some(parent) = out_path.parent() {
                std::fs::create_dir_all(parent)
                    .map_err(|e| format!("mkdir {}: {e}", parent.display()))?;
            }
            let raw = &zip_bytes[data_start..data_end];
            let content = match method {
                0 => raw.to_vec(),
                8 => {
                    let mut dec = flate2::read::DeflateDecoder::new(raw);
                    let mut buf = Vec::new();
                    dec.read_to_end(&mut buf)
                        .map_err(|e| format!("deflate error on {name}: {e}"))?;
                    buf
                }
                other => return Err(format!("unsupported zip compression method {other}")),
            };
            std::fs::write(&out_path, content)
                .map_err(|e| format!("write {}: {e}", out_path.display()))?;
            extracted += 1;
        }
        pos = data_end;
    }
    if extracted == 0 && pos == 0 {
        return Err("invalid or empty zip archive".into());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use base64::Engine as _;
    use base64::engine::general_purpose::STANDARD as B64;
    use crypto_box::SecretKey;
    use crypto_box::aead::OsRng;
    use std::net::TcpListener;
    use std::sync::{Arc, Mutex};

    fn make_stored_zip(entries: &[(&str, &[u8])]) -> Vec<u8> {
        let mut out = Vec::new();
        for (name, data) in entries {
            let crc = crc32fast::hash(data);
            out.extend_from_slice(&0x0403_4b50u32.to_le_bytes());
            out.extend_from_slice(&20u16.to_le_bytes());
            out.extend_from_slice(&0u16.to_le_bytes());
            out.extend_from_slice(&0u16.to_le_bytes()); // stored
            out.extend_from_slice(&0u16.to_le_bytes());
            out.extend_from_slice(&0u16.to_le_bytes());
            out.extend_from_slice(&crc.to_le_bytes());
            out.extend_from_slice(&(data.len() as u32).to_le_bytes());
            out.extend_from_slice(&(data.len() as u32).to_le_bytes());
            out.extend_from_slice(&(name.len() as u16).to_le_bytes());
            out.extend_from_slice(&0u16.to_le_bytes());
            out.extend_from_slice(name.as_bytes());
            out.extend_from_slice(data);
        }
        out
    }

    #[test]
    fn zip_slip_paths_are_rejected_fail_closed() {
        for bad in [
            "../evil.txt",
            "a/../../evil.txt",
            "/etc/passwd",
            "a\\..\\evil.txt",
            "C:evil.txt",
            "",
        ] {
            assert!(
                validate_archive_entry_path(bad).is_err(),
                "expected rejection for {bad:?}"
            );
        }
        let evil_zip = make_stored_zip(&[("../escape.txt", b"pwn")]);
        let dir = std::env::temp_dir().join(format!("arks-zip-slip-test-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        assert!(extract_zip_safe(&evil_zip, &dir).is_err());
        assert!(!dir.parent().unwrap().join("escape.txt").exists());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn actions_rest_fixture_workflow_run_cache_variable_and_sealed_secret() {
        let sk = SecretKey::generate(&mut OsRng);
        let pk_b64 = B64.encode(sk.public_key().as_bytes());
        let received_secret: Arc<Mutex<Option<String>>> = Arc::new(Mutex::new(None));
        let received_srv = Arc::clone(&received_secret);

        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();
        let srv = std::thread::spawn(move || {
            for _ in 0..7 {
                let (mut stream, _) = listener.accept().unwrap();
                let req = crate::http_min::read_http_request(&mut stream).unwrap();
                let (status, body) = match (req.method.as_str(), req.path.as_str()) {
                    ("GET", p) if p.starts_with("/repos/o/r/actions/workflows") => (
                        200,
                        json!({"workflows": [{"id": 10, "name": "CI", "state": "active"}]}).to_string(),
                    ),
                    ("GET", p) if p.starts_with("/repos/o/r/actions/runs/42") => (
                        200,
                        json!({"id": 42, "run_number": 7, "name": "CI", "status": "completed", "conclusion": "success"}).to_string(),
                    ),
                    ("GET", p) if p.starts_with("/repos/o/r/actions/caches") => (
                        200,
                        json!({"actions_caches": [{"id": 99, "key": "cargo-1", "size_in_bytes": 1024, "ref": "refs/heads/main"}]}).to_string(),
                    ),
                    ("PATCH", "/repos/o/r/actions/variables/FOO") => {
                        (404, json!({"message": "Not Found"}).to_string())
                    }
                    ("POST", "/repos/o/r/actions/variables") => (201, String::new()),
                    ("GET", "/repos/o/r/actions/secrets/public-key") => (
                        200,
                        json!({"key_id": "k1", "key": pk_b64}).to_string(),
                    ),
                    ("PUT", "/repos/o/r/actions/secrets/API_KEY") => {
                        let v: Value = serde_json::from_slice(&req.body).unwrap();
                        assert_eq!(v["key_id"], "k1");
                        let raw = B64.decode(v["encrypted_value"].as_str().unwrap()).unwrap();
                        let plain = String::from_utf8(sk.unseal(&raw).unwrap()).unwrap();
                        *received_srv.lock().unwrap() = Some(plain);
                        (201, String::new())
                    }
                    (m, p) => panic!("unexpected fixture request {m} {p}"),
                };
                crate::http_min::write_http_response(
                    &mut stream,
                    status,
                    "OK",
                    "application/json",
                    body.as_bytes(),
                )
                .unwrap();
            }
        });

        let ctx = Ctx {
            base: format!("http://{addr}"),
            token: Some("tok".into()),
            default_repo: Some("o/r".into()),
        };
        let mut out = Vec::new();
        workflow(&ctx, &["list".into()], &mut out).unwrap();
        assert!(String::from_utf8_lossy(&out).contains("CI\tactive\t10"));

        out.clear();
        run(
            &ctx,
            &["watch".into(), "42".into(), "-i".into(), "0".into()],
            &mut out,
        )
        .unwrap();
        assert!(String::from_utf8_lossy(&out).contains("success"));

        out.clear();
        cache(&ctx, &["list".into()], &mut out).unwrap();
        assert!(String::from_utf8_lossy(&out).contains("cargo-1"));

        out.clear();
        variable(
            &ctx,
            &["set".into(), "FOO".into(), "-b".into(), "bar".into()],
            &mut out,
        )
        .unwrap();
        assert!(String::from_utf8_lossy(&out).contains("FOO"));

        out.clear();
        secret(
            &ctx,
            &[
                "set".into(),
                "API_KEY".into(),
                "-b".into(),
                "top-secret-42".into(),
            ],
            &mut out,
        )
        .unwrap();
        srv.join().unwrap();
        assert_eq!(
            received_secret.lock().unwrap().as_deref(),
            Some("top-secret-42")
        );
    }
}
