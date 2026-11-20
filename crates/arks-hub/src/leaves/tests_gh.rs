//! gh-symmetric core leaves (pr, issue, repo, release, label, search, ruleset)
//! against an in-process `arks serve` forge with Git content (host-run; not a
//! Docker receipt). Branch content is written straight into the forge's Git
//! directory with the arks Git crates, as a push would leave it.

use super::tests::{Forge, is_api, is_usage, json_of};
use super::{Fail, issue, label, pr, release, repo, ruleset, search};
use crate::forge::git::GitRepo;
use serde_json::json;

/// Create `local/<name>` with a README and a `feature` branch adding `a.txt`.
fn repo_with_feature(f: &Forge, name: &str) -> GitRepo {
    f.ok(
        repo::run,
        &["create", name, "--public", "--add-readme", "-d", "demo"],
    );
    let dir = f.dir.join("data/git/local").join(format!("{name}.git"));
    let mut g = GitRepo::open(&dir).expect("forge git dir");
    let main = g.resolve("main").expect("main tip");
    g.set_branch("feature", &main).unwrap();
    g.commit_files(
        "feature",
        &[("a.txt".into(), b"alpha\n".to_vec())],
        "Add a\n\nmore",
        "dev",
        1_800_000_000,
    )
    .unwrap();
    g
}

#[test]
fn pr_checks_rulesets_and_merge() {
    let f = Forge::start();
    let g = repo_with_feature(&f, "gx");
    let r = ["-R", "local/gx"];
    let url = f.ok(
        pr::run,
        &[
            "create", "-H", "feature", "-B", "main", "-t", "Add a", "-b", "body", "-l", "enh",
            r[0], r[1],
        ],
    );
    assert!(url.trim().ends_with("/local/gx/pull/1"), "{url}");
    assert!(
        f.ok(pr::run, &["diff", "1", "--name-only", r[0], r[1]])
            .contains("a.txt")
    );
    let diff = f.ok(pr::run, &["diff", "1", r[0], r[1]]);
    assert!(
        diff.contains("+++ b/a.txt") && diff.contains("+alpha"),
        "{diff}"
    );
    let list = f.ok(pr::run, &["list", r[0], r[1]]);
    assert!(list.starts_with("1\tAdd a\tfeature\tOPEN\t"), "{list}");
    assert!(
        is_api(f.run(pr::run, &["checks", "1", r[0], r[1]])),
        "no checks yet"
    );

    let head = g.resolve("feature").unwrap().to_hex();
    f.api(
        "POST",
        &format!("/repos/local/gx/statuses/{head}"),
        Some(json!({"state": "pending", "context": "ci"})),
    );
    assert!(matches!(
        f.run(pr::run, &["checks", "1", r[0], r[1]]),
        Err(Fail::Exit(8))
    ));
    f.api(
        "POST",
        &format!("/repos/local/gx/statuses/{head}"),
        Some(json!({"state": "success", "context": "ci"})),
    );
    let checks = f.ok(pr::run, &["checks", "1", r[0], r[1]]);
    assert!(checks.starts_with("ci\tpass\t"), "{checks}");
    let cj = json_of(&f.ok(
        pr::run,
        &["checks", "1", "--json", "name,bucket", r[0], r[1]],
    ));
    assert_eq!(cj, json!([{"bucket": "pass", "name": "ci"}]));

    f.api("POST", "/repos/local/gx/rulesets", Some(json!({
        "name": "main-guard",
        "conditions": {"ref_name": {"include": ["~DEFAULT_BRANCH"], "exclude": []}},
        "rules": [{"type": "pull_request", "parameters": {"required_approving_review_count": 1}}],
    })));
    assert!(
        f.ok(ruleset::run, &["list", r[0], r[1]])
            .contains("main-guard")
    );
    assert!(
        f.ok(ruleset::run, &["check", "main", r[0], r[1]])
            .contains("1 rules apply to branch main")
    );
    match f.run(pr::run, &["merge", "1", "--merge", r[0], r[1]]) {
        Err(Fail::Api(m)) => assert!(m.contains("approving review"), "{m}"),
        other => panic!("merge should be blocked: {other:?}"),
    }
    assert!(
        is_usage(f.run(pr::run, &["merge", "1", r[0], r[1]])),
        "method required"
    );
    f.ok(pr::run, &["review", "1", "--approve", r[0], r[1]]);
    f.ok(pr::run, &["merge", "1", "--merge", r[0], r[1]]);
    let tip = g.resolve("main").unwrap();
    assert_eq!(g.commit(&tip).unwrap().parents.len(), 2, "merge commit");
    assert!(g.lookup(&tip, "a.txt").unwrap().is_some());
    let v = json_of(&f.ok(
        pr::run,
        &[
            "view",
            "1",
            "--json",
            "state,labels,mergeCommit",
            r[0],
            r[1],
        ],
    ));
    assert_eq!(v["state"], json!("MERGED"));
    assert_eq!(v["labels"][0]["name"], json!("enh"));
    assert_eq!(v["mergeCommit"]["oid"], json!(tip.to_hex()));
    let reverted = f.ok(pr::run, &["revert", "1", r[0], r[1]]);
    assert!(reverted.trim().ends_with("/pull/2"), "{reverted}");
    let rev_tip = g.resolve("revert-1-feature").unwrap();
    assert!(
        g.lookup(&rev_tip, "a.txt").unwrap().is_none(),
        "revert drops a.txt"
    );
    // gh refuses unknown flags; jq needs --json.
    assert!(is_usage(f.run(pr::run, &["list", "--nope"])));
    assert!(is_usage(
        f.run(pr::run, &["list", "--jq", ".x", r[0], r[1]])
    ));
}

#[test]
fn update_branch_squash_and_conflicts() {
    let f = Forge::start();
    let mut g = repo_with_feature(&f, "gu");
    let r = ["-R", "local/gu"];
    g.commit_files(
        "main",
        &[("b.txt".into(), b"beta\n".to_vec())],
        "Add b",
        "dev",
        1_800_000_100,
    )
    .unwrap();
    f.ok(
        pr::run,
        &[
            "create", "-H", "feature", "-t", "Add a", "-b", "", r[0], r[1],
        ],
    );
    f.ok(pr::run, &["update-branch", "1", r[0], r[1]]);
    let head = g.resolve("feature").unwrap();
    assert!(
        g.lookup(&head, "b.txt").unwrap().is_some(),
        "base merged into head"
    );
    f.ok(
        pr::run,
        &["merge", "1", "--squash", "-t", "Squashed", r[0], r[1]],
    );
    let tip = g.resolve("main").unwrap();
    let c = g.commit(&tip).unwrap();
    assert_eq!(c.parents.len(), 1);
    assert!(c.message.starts_with("Squashed"), "{}", c.message);
    // A path changed differently on both sides is a conflict → not mergeable.
    let main = g.resolve("main").unwrap();
    g.set_branch("x", &main).unwrap();
    g.commit_files(
        "x",
        &[("a.txt".into(), b"x\n".to_vec())],
        "x",
        "dev",
        1_800_000_200,
    )
    .unwrap();
    g.commit_files(
        "main",
        &[("a.txt".into(), b"y\n".to_vec())],
        "y",
        "dev",
        1_800_000_300,
    )
    .unwrap();
    f.ok(
        pr::run,
        &["create", "-H", "x", "-t", "X", "-b", "", r[0], r[1]],
    );
    match f.run(pr::run, &["merge", "2", "--rebase", r[0], r[1]]) {
        Err(Fail::Api(m)) => assert!(m.contains("not mergeable"), "{m}"),
        other => panic!("conflict expected: {other:?}"),
    }
}

#[test]
fn issue_develop_artifacts_and_search() {
    let f = Forge::start();
    let g = repo_with_feature(&f, "gi");
    let r = ["-R", "local/gi"];
    let url = f.ok(
        issue::run,
        &[
            "create",
            "-t",
            "Login fails",
            "-b",
            "trace",
            "-l",
            "bug",
            "-a",
            "@me",
            r[0],
            r[1],
        ],
    );
    assert!(url.trim().ends_with("/local/gi/issues/1"), "{url}");
    assert!(
        f.ok(issue::run, &["list", r[0], r[1]])
            .starts_with("1\tOPEN\tLogin fails\tbug\t")
    );
    f.ok(issue::run, &["develop", "1", "-n", "fix-login", r[0], r[1]]);
    assert!(
        g.resolve("refs/heads/fix-login").is_some(),
        "branch created on the forge"
    );
    assert!(
        f.ok(issue::run, &["develop", "1", "--list", r[0], r[1]])
            .contains("fix-login")
    );
    let plan = f.file("plan.md", "# plan\n");
    f.ok(
        issue::run,
        &[
            "artifact",
            "create",
            "1",
            &format!("{plan}#My plan"),
            "https://ex.invalid/run",
            r[0],
            r[1],
        ],
    );
    let arts = json_of(&f.ok(
        issue::run,
        &[
            "artifact",
            "list",
            "1",
            "--json",
            "number,name,type",
            r[0],
            r[1],
        ],
    ));
    assert_eq!(
        arts,
        json!([{"name": "My plan", "number": 1, "type": "generic"}, {"name": "ex.invalid", "number": 2, "type": "link"}])
    );
    f.ok(
        issue::run,
        &["artifact", "edit", "1", "1", "--body", "v2", r[0], r[1]],
    );
    assert!(
        f.ok(
            issue::run,
            &["artifact", "view", "1", "1", "--version", "1", r[0], r[1]]
        )
        .contains("# plan")
    );
    assert!(is_api(f.run(
        issue::run,
        &[
            "artifact",
            "edit",
            "1",
            "2",
            "--body",
            "not a url",
            r[0],
            r[1]
        ]
    )));
    assert!(
        is_usage(f.run(issue::run, &["artifact", "delete", "1", "2", r[0], r[1]])),
        "needs --yes"
    );
    f.ok(
        issue::run,
        &["artifact", "delete", "1", "2", "--yes", r[0], r[1]],
    );
    f.ok(
        issue::run,
        &[
            "close",
            "1",
            "-r",
            "not planned",
            "-c",
            "wontfix",
            r[0],
            r[1],
        ],
    );
    let v = json_of(&f.ok(
        issue::run,
        &[
            "view",
            "1",
            "--json",
            "state,stateReason,comments",
            r[0],
            r[1],
        ],
    ));
    assert_eq!(v["state"], json!("CLOSED"));
    assert_eq!(v["stateReason"], json!("NOT_PLANNED"));
    assert_eq!(v["comments"].as_array().map(Vec::len), Some(1));

    // Feature content is not on the default branch; no result is an error, like gh.
    assert!(is_api(
        f.run(search::run, &["code", "alpha", "--repo", "local/gi"])
    ));
    let code = f.ok(search::run, &["code", "demo", "--repo", "local/gi"]);
    assert!(code.contains("local/gi:README.md:demo"), "{code}");
    let commits = f.ok(search::run, &["commits", "Add", "--repo", "local/gi"]);
    assert!(commits.contains("\tAdd a\tdev\t"), "{commits}");
    let issues = f.ok(
        search::run,
        &[
            "issues",
            "--state",
            "closed",
            "--label",
            "bug",
            "--json",
            "number",
            "--jq",
            ".[].number",
        ],
    );
    assert_eq!(issues.trim(), "1");
    assert!(is_api(f.run(search::run, &["repos", "nomatch-anywhere"])));
}

#[test]
fn releases_labels_and_templates() {
    let f = Forge::start();
    let g = repo_with_feature(&f, "gr");
    let r = ["-R", "local/gr"];
    let bin = f.file("tool.bin", "bytes\n");
    f.ok(
        release::run,
        &[
            "create",
            "v1.0.0",
            &format!("{bin}#The tool"),
            "--generate-notes",
            r[0],
            r[1],
        ],
    );
    assert!(
        g.resolve("refs/tags/v1.0.0").is_some(),
        "tag created at the target"
    );
    let v = json_of(&f.ok(
        release::run,
        &[
            "view",
            "v1.0.0",
            "--json",
            "tagName,isDraft,assets,body",
            r[0],
            r[1],
        ],
    ));
    assert_eq!(v["isDraft"], json!(false));
    assert_eq!(v["assets"][0]["label"], json!("The tool"));
    assert!(v["body"].as_str().unwrap().contains("What's Changed"));
    let list = f.ok(release::run, &["list", r[0], r[1]]);
    assert!(list.starts_with("v1.0.0\tLatest\tv1.0.0\t"), "{list}");
    let out_dir = f.dir.join("dl");
    f.ok(
        release::run,
        &[
            "download",
            "v1.0.0",
            "-p",
            "tool*",
            "-D",
            out_dir.to_str().unwrap(),
            r[0],
            r[1],
        ],
    );
    assert_eq!(
        std::fs::read_to_string(out_dir.join("tool.bin")).unwrap(),
        "bytes\n"
    );
    assert!(
        is_api(f.run(
            release::run,
            &[
                "download",
                "v1.0.0",
                "-p",
                "tool*",
                "-D",
                out_dir.to_str().unwrap(),
                r[0],
                r[1]
            ]
        )),
        "exists without --clobber"
    );
    f.ok(
        release::run,
        &["upload", "v1.0.0", &bin, "--clobber", r[0], r[1]],
    );
    f.ok(
        release::run,
        &["delete-asset", "v1.0.0", "tool.bin", "-y", r[0], r[1]],
    );
    f.ok(
        release::run,
        &["delete", "v1.0.0", "--cleanup-tag", "-y", r[0], r[1]],
    );
    assert!(
        g.resolve("refs/tags/v1.0.0").is_none(),
        "--cleanup-tag removes the tag"
    );

    f.ok(
        label::run,
        &["create", "bug", "-c", "#FF0000", "-d", "broken", r[0], r[1]],
    );
    assert!(is_api(f.run(label::run, &["create", "bug", r[0], r[1]])));
    f.ok(
        label::run,
        &["create", "bug", "-c", "00ff00", "--force", r[0], r[1]],
    );
    assert!(
        f.ok(label::run, &["list", r[0], r[1]])
            .contains("bug\tbroken\t#00FF00")
    );

    assert!(
        f.ok(repo::run, &["gitignore", "list"])
            .lines()
            .any(|l| l == "Rust")
    );
    assert!(
        f.ok(repo::run, &["gitignore", "view", "rust"])
            .contains("/target/")
    );
    assert!(
        f.ok(repo::run, &["license", "view", "apache-2.0"])
            .contains("SPDX-License-Identifier: Apache-2.0")
    );
    assert!(is_api(f.run(repo::run, &["license", "view", "nope"])));
    let listing = f.ok(repo::run, &["read-dir", r[0], r[1]]);
    assert_eq!(listing.trim(), "README.md");
    assert_eq!(
        f.ok(
            repo::run,
            &["read-file", "a.txt", "--ref", "feature", r[0], r[1]]
        ),
        "alpha\n"
    );
    let clone = f.ok(repo::run, &["clone", "local/gr", "work"]);
    assert!(
        clone.contains("recipe=arks clone ") && clone.trim_end().ends_with(" work"),
        "{clone}"
    );
}
