//! Hub-native rows decided by queue unit Q18 ([[sot:root/arks-hub-gh-command-matrix]]
//! rules 12a–12c): `milestone` leaves against an in-process `arks serve` forge
//! (host-run; not a Docker receipt).

use super::tests::{Forge, is_api, is_usage, json_of};
use super::{issue, milestone, repo};
use serde_json::json;

#[test]
fn milestone_leaves_round_trip() {
    let f = Forge::start();
    f.ok(repo::run, &["create", "ms", "--public", "--add-readme"]);
    let r = ["-R", "local/ms"];

    let url = f.ok(
        milestone::run,
        &["create", "v1.0", "-d", "first cut", r[0], r[1]],
    );
    assert!(url.trim().ends_with("/local/ms/milestone/1"), "{url}");
    let url2 = f.ok(milestone::run, &["create", "--title", "v2.0", r[0], r[1]]);
    assert!(url2.trim().ends_with("/local/ms/milestone/2"), "{url2}");

    let list = f.ok(milestone::run, &["list", r[0], r[1]]);
    assert_eq!(list, "1\tv1.0\topen\tfirst cut\n2\tv2.0\topen\t\n");
    let one = f.ok(milestone::run, &["ls", "-L", "1", r[0], r[1]]);
    assert_eq!(one.lines().count(), 1, "{one}");
    let v = json_of(&f.ok(
        milestone::run,
        &["list", "--json", "number,title,state,url", r[0], r[1]],
    ));
    assert_eq!(v[0]["title"], json!("v1.0"));
    assert!(
        v[1]["url"]
            .as_str()
            .unwrap()
            .ends_with("/local/ms/milestone/2"),
        "{v}"
    );
    assert_eq!(
        f.ok(
            milestone::run,
            &["list", "--json", "title", "--jq", ".[1].title", r[0], r[1]]
        ),
        "v2.0\n"
    );

    // Edit by title, close by number, state filters.
    f.ok(
        milestone::run,
        &["edit", "v2.0", "--title", "v2", "-d", "later", r[0], r[1]],
    );
    f.ok(milestone::run, &["close", "1", r[0], r[1]]);
    f.ok(milestone::run, &["close", "v1.0", r[0], r[1]]); // already closed: no-op
    assert_eq!(
        f.ok(milestone::run, &["list", r[0], r[1]]),
        "2\tv2\topen\tlater\n"
    );
    assert_eq!(
        f.ok(milestone::run, &["list", "--state", "closed", r[0], r[1]]),
        "1\tv1.0\tclosed\tfirst cut\n"
    );
    assert_eq!(
        f.ok(milestone::run, &["list", "--state", "all", r[0], r[1]])
            .lines()
            .count(),
        2
    );
    f.ok(
        milestone::run,
        &["edit", "1", "--state", "open", r[0], r[1]],
    );
    assert_eq!(
        f.ok(milestone::run, &["list", r[0], r[1]]).lines().count(),
        2
    );

    // Delete clears the milestone from the issues that carried it.
    f.ok(
        issue::run,
        &["create", "-t", "bug", "-b", "b", "-m", "v2", r[0], r[1]],
    );
    let iv = json_of(&f.ok(
        issue::run,
        &["view", "1", "--json", "milestone", r[0], r[1]],
    ));
    assert_eq!(iv["milestone"]["title"], json!("v2"), "{iv}");
    f.ok(milestone::run, &["delete", "v2", "--yes", r[0], r[1]]);
    let iv = json_of(&f.ok(
        issue::run,
        &["view", "1", "--json", "milestone", r[0], r[1]],
    ));
    assert!(iv["milestone"].is_null(), "{iv}");
    assert_eq!(
        f.ok(milestone::run, &["list", "--state", "all", r[0], r[1]]),
        "1\tv1.0\topen\tfirst cut\n"
    );

    // Usage failures (exit 2 class) and forge failures (exit 1 class).
    for bad in [
        &["create", r[0], r[1]][..],
        &["create", "a", "--title", "b", r[0], r[1]],
        &["create", "--title", " ", r[0], r[1]],
        &["edit", "1", r[0], r[1]],
        &["edit", "1", "--state", "all", r[0], r[1]],
        &["edit", r[0], r[1], "--title", "x"],
        &["list", "--state", "merged", r[0], r[1]],
        &["list", "--nope", r[0], r[1]],
        &["close", "1", "2", r[0], r[1]],
        &["frobnicate", r[0], r[1]],
    ] {
        assert!(is_usage(f.run(milestone::run, bad)), "{bad:?}");
    }
    for missing in [
        &["close", "9", r[0], r[1]][..],
        &["delete", "nope", "--yes", r[0], r[1]],
        &["edit", "v9", "--title", "x", r[0], r[1]],
        &["list", "-R", "local/absent"],
    ] {
        assert!(is_api(f.run(milestone::run, missing)), "{missing:?}");
    }
}

#[test]
fn forge_validates_milestone_state() {
    let f = Forge::start();
    f.ok(repo::run, &["create", "mv", "--public"]);
    let created = f.api(
        "POST",
        "/repos/local/mv/milestones",
        Some(json!({"title": "t"})),
    );
    assert!(
        created["html_url"]
            .as_str()
            .unwrap()
            .ends_with("/local/mv/milestone/1"),
        "{created}"
    );
    let ctx_err = f.try_api(
        "PATCH",
        "/repos/local/mv/milestones/1",
        Some(json!({"state": "merged"})),
    );
    assert!(
        matches!(&ctx_err, Err(super::Fail::Api(m)) if m.contains("422")),
        "{ctx_err:?}"
    );
}
