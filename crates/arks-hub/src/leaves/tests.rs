//! Client leaves against an in-process `arks serve` forge (host-run; not a Docker receipt).

use super::{Ctx, Fail, Res, autolink, discussion, gist, keys, project};
use crate::key_formats::fixtures::{pgp_v4, ssh_ed25519};
use crate::serve::{ForgeServeOptions, serve_forge_api};
use serde_json::{Value, json};
use std::io::Write;
use std::net::{TcpListener, TcpStream};
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

pub(super) type Leaf = fn(&Ctx, &[String], &mut dyn Write) -> Res;

pub(super) struct Forge {
    stop: Arc<AtomicBool>,
    handle: Option<thread::JoinHandle<Result<(), String>>>,
    pub(super) dir: PathBuf,
    ctx: Ctx,
}

impl Forge {
    pub(super) fn start() -> Self {
        static SEQ: AtomicU64 = AtomicU64::new(0);
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0);
        let dir = std::env::temp_dir().join(format!(
            "arks-hub-leaves-{}-{}-{nanos}",
            std::process::id(),
            SEQ.fetch_add(1, Ordering::SeqCst)
        ));
        std::fs::create_dir_all(&dir).unwrap();
        let port = TcpListener::bind("127.0.0.1:0")
            .and_then(|l| l.local_addr())
            .map(|a| a.port())
            .unwrap();
        let opts = ForgeServeOptions {
            listen: format!("127.0.0.1:{port}").parse().unwrap(),
            data_dir: dir.join("data"),
            rebind_desired: None,
            tls: None,
            git_root: None,
            git_url: None,
        };
        let stop = Arc::new(AtomicBool::new(false));
        let stop2 = stop.clone();
        let handle = thread::spawn(move || serve_forge_api(opts, stop2));
        for _ in 0..400 {
            if TcpStream::connect(("127.0.0.1", port)).is_ok() {
                break;
            }
            thread::sleep(Duration::from_millis(10));
        }
        let ctx = Ctx {
            base: format!("http://127.0.0.1:{port}"),
            token: Some("local-dev-token".into()),
            default_repo: Some("local/playground".into()),
        };
        Self {
            stop,
            handle: Some(handle),
            dir,
            ctx,
        }
    }

    /// Register `token` as account `login` on this forge and return its client context.
    pub(super) fn account(&self, login: &str, token: &str) -> Ctx {
        let body = json!({"token": token, "login": login}).to_string();
        crate::http_min::http_exchange(&self.ctx.base, "POST", "/login", None, Some(&body))
            .unwrap();
        Ctx {
            base: self.ctx.base.clone(),
            token: Some(token.into()),
            default_repo: self.ctx.default_repo.clone(),
        }
    }

    pub(super) fn run(&self, leaf: Leaf, argv: &[&str]) -> Result<String, Fail> {
        let args: Vec<String> = argv.iter().map(|s| s.to_string()).collect();
        let mut out = Vec::new();
        leaf(&self.ctx, &args, &mut out)?;
        Ok(String::from_utf8(out).unwrap())
    }

    pub(super) fn ok(&self, leaf: Leaf, argv: &[&str]) -> String {
        match self.run(leaf, argv) {
            Ok(s) => s,
            Err(e) => panic!("{argv:?} failed: {e:?}"),
        }
    }

    pub(super) fn file(&self, name: &str, content: &str) -> String {
        let p = self.dir.join(name);
        std::fs::write(&p, content).unwrap();
        p.to_string_lossy().into_owned()
    }

    pub(super) fn api(&self, method: &str, path: &str, body: Option<Value>) -> Value {
        self.ctx.api(method, path, body.as_ref()).unwrap()
    }

    pub(super) fn try_api(
        &self,
        method: &str,
        path: &str,
        body: Option<Value>,
    ) -> Result<Value, Fail> {
        self.ctx.api(method, path, body.as_ref())
    }
}

impl Drop for Forge {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::SeqCst);
        if let Some(h) = self.handle.take() {
            let _ = h.join();
        }
        let _ = std::fs::remove_dir_all(&self.dir);
    }
}

pub(super) fn is_usage<T: std::fmt::Debug>(r: Result<T, Fail>) -> bool {
    matches!(r, Err(Fail::Usage(_)))
}

pub(super) fn is_api<T: std::fmt::Debug>(r: Result<T, Fail>) -> bool {
    matches!(r, Err(Fail::Api(_)))
}

pub(super) fn json_of(s: &str) -> Value {
    serde_json::from_str(s).unwrap_or_else(|e| panic!("not JSON ({e}): {s}"))
}

#[test]
fn gist_leaves_round_trip() {
    let f = Forge::start();
    let a = f.file("a.txt", "alpha\n");
    let b = f.file("b.rs", "fn main() {}\n");
    let url = f.ok(
        gist::run,
        &["create", &a, &b, "-d", "demo gist", "--public"],
    );
    let id = url.trim().rsplit('/').next().unwrap().to_string();
    assert!(url.contains("/gists/"), "{url}");

    let list = f.ok(gist::run, &["list"]);
    assert!(
        list.contains(&format!("{id}\tdemo gist\t2 files\tpublic")),
        "{list}"
    );
    assert!(f.ok(gist::run, &["list", "--secret"]).is_empty());
    assert!(f.ok(gist::run, &["list", "--filter", "nomatch"]).is_empty());
    assert!(
        !f.ok(
            gist::run,
            &["list", "--filter", "fn main", "--include-content"]
        )
        .is_empty()
    );

    let files = f.ok(gist::run, &["view", &id, "--files"]);
    assert_eq!(files, "a.txt\nb.rs\n");
    assert_eq!(
        f.ok(gist::run, &["view", url.trim(), "-f", "a.txt"]),
        "alpha\n"
    );
    let full = f.ok(gist::run, &["view", &id]);
    assert!(
        full.starts_with("demo gist\n") && full.contains("fn main"),
        "{full}"
    );

    let new_a = f.file("new-a.txt", "alpha v2\n");
    f.ok(
        gist::run,
        &["edit", &id, &new_a, "-f", "a.txt", "-d", "edited"],
    );
    assert_eq!(f.ok(gist::run, &["view", &id, "-f", "a.txt"]), "alpha v2\n");
    let c = f.file("c.md", "# c\n");
    f.ok(gist::run, &["edit", &id, "--add", &c, "--remove", "b.rs"]);
    assert_eq!(f.ok(gist::run, &["view", &id, "--files"]), "a.txt\nc.md\n");
    assert!(is_usage(f.run(gist::run, &["edit", &id])));
    assert!(is_usage(f.run(gist::run, &["edit", &id, "-f", "a.txt"])));

    f.ok(gist::run, &["rename", &id, "c.md", "readme.md"]);
    assert_eq!(
        f.ok(gist::run, &["view", &id, "--files"]),
        "a.txt\nreadme.md\n"
    );
    assert!(is_api(f.run(gist::run, &["rename", &id, "nope", "x"])));

    let dest = f.dir.join("cloned");
    let dest_s = dest.to_string_lossy().into_owned();
    let out = f.ok(gist::run, &["clone", &id, &dest_s]);
    assert!(out.contains("recipe=cd "), "{out}");
    assert_eq!(
        std::fs::read_to_string(dest.join("readme.md")).unwrap(),
        "# c\n"
    );
    assert!(is_usage(f.run(gist::run, &["clone", &id, &dest_s])));

    f.ok(gist::run, &["delete", &id, "--yes"]);
    assert!(is_api(f.run(gist::run, &["view", &id])));
    assert!(is_usage(f.run(gist::run, &["list", "--bogus"])));
}

#[test]
fn discussion_leaves_round_trip() {
    let f = Forge::start();
    assert!(is_usage(f.run(discussion::run, &["create", "-b", "x"])));
    let url = f.ok(
        discussion::run,
        &[
            "create",
            "-t",
            "Ship plan",
            "-b",
            "first body",
            "-c",
            "Ideas",
        ],
    );
    assert!(
        url.trim()
            .ends_with("/repos/local/playground/discussions/1"),
        "{url}"
    );
    f.ok(discussion::run, &["create", "-t", "Other", "-b", "x"]);

    let list = f.ok(discussion::run, &["list"]);
    assert!(list.contains("1\tShip plan\tIdeas\tOPEN"), "{list}");
    let only_ideas = f.ok(discussion::run, &["list", "-c", "Ideas"]);
    assert_eq!(only_ideas.lines().count(), 1, "{only_ideas}");
    let j = json_of(&f.ok(
        discussion::run,
        &["list", "--json", "number,title,category"],
    ));
    assert_eq!(j.as_array().unwrap().len(), 2);
    assert!(is_usage(
        f.run(discussion::run, &["list", "--json", "nope"])
    ));

    f.ok(
        discussion::run,
        &["edit", "1", "-t", "Ship plan v2", "-c", "General"],
    );
    let body = f.file("c.md", "a comment\n");
    f.ok(discussion::run, &["comment", "1", "-F", &body]);
    assert!(is_usage(f.run(discussion::run, &["comment", "1"])));
    let view = f.ok(discussion::run, &["view", "1"]);
    assert!(view.contains("title:\tShip plan v2"), "{view}");
    assert!(view.contains("category:\tGeneral"), "{view}");
    // gh prints only the comments when --comments output is piped.
    let comments = f.ok(discussion::run, &["view", "1", "--comments"]);
    assert!(comments.contains("a comment"), "{comments}");
    let vj = json_of(&f.ok(discussion::run, &["view", "1", "--json", "comments,state"]));
    assert_eq!(vj["comments"].as_array().map(Vec::len), Some(1));
    assert_eq!(vj["state"], json!("OPEN"));
    assert!(is_api(f.run(discussion::run, &["view", "99"])));
    assert!(is_api(
        f.run(discussion::run, &["list", "-R", "local/missing-repo"])
    ));
}

#[test]
fn project_leaves_round_trip() {
    let f = Forge::start();
    let issue = f.api(
        "POST",
        "/repos/local/playground/issues",
        Some(json!({"title": "tracked issue", "body": "b"})),
    );
    let issue_url = f
        .ctx
        .url(&format!("/local/playground/issues/{}", issue["number"]));

    assert!(is_usage(f.run(project::run, &["create"])));
    let created = json_of(&f.ok(
        project::run,
        &["create", "--title", "Roadmap", "--format", "json"],
    ));
    let num = created["number"].as_u64().unwrap().to_string();
    let n = num.as_str();
    f.ok(
        project::run,
        &[
            "edit",
            n,
            "-d",
            "short",
            "--readme",
            "# hi",
            "--visibility",
            "PUBLIC",
        ],
    );
    let v = json_of(&f.ok(project::run, &["view", n, "--format", "json"]));
    assert_eq!(v["shortDescription"], json!("short"));
    assert_eq!(v["public"], json!(true));

    // Fields.
    let status = json_of(&f.ok(
        project::run,
        &[
            "field-create",
            n,
            "--name",
            "Status",
            "--data-type",
            "SINGLE_SELECT",
            "--single-select-options",
            "Todo,Done",
            "--format",
            "json",
        ],
    ));
    let status_id = status["id"].as_str().unwrap().to_string();
    let done_id = status["options"][1]["id"].as_str().unwrap().to_string();
    let note = f.ok(
        project::run,
        &["field-create", n, "--name", "Note", "--data-type", "TEXT"],
    );
    let note_id = note.split('\t').next().unwrap().to_string();
    assert!(is_usage(f.run(
        project::run,
        &["field-create", n, "--name", "X", "--data-type", "ITERATION"]
    )));
    let fields = f.ok(project::run, &["field-list", n]);
    assert!(fields.contains("Status\tSINGLE_SELECT") && fields.contains("Note\tTEXT"));

    // Items.
    let added = f.ok(project::run, &["item-add", n, "--url", &issue_url]);
    assert!(added.starts_with("Added item "), "{added}");
    let draft = json_of(&f.ok(
        project::run,
        &[
            "item-create",
            n,
            "--title",
            "Draft one",
            "--body",
            "d",
            "--format",
            "json",
        ],
    ));
    let draft_id = draft["id"].as_str().unwrap().to_string();
    f.ok(
        project::run,
        &[
            "item-edit",
            "--id",
            &draft_id,
            "--project-id",
            n,
            "--field-id",
            &status_id,
            "--single-select-option-id",
            &done_id,
        ],
    );
    f.ok(
        project::run,
        &[
            "item-edit",
            "--id",
            &draft_id,
            "--project-id",
            n,
            "--field-id",
            &note_id,
            "--text",
            "hello",
        ],
    );
    f.ok(
        project::run,
        &["item-edit", "--id", &draft_id, "--title", "Draft renamed"],
    );
    assert!(is_usage(f.run(
        project::run,
        &["item-edit", "--id", &draft_id, "--iteration-id", "1"]
    )));
    let items = json_of(&f.ok(project::run, &["item-list", n, "--format", "json"]));
    assert_eq!(items["totalCount"], json!(2));
    let d = items["items"]
        .as_array()
        .unwrap()
        .iter()
        .find(|i| i["id"] == json!(draft_id))
        .unwrap();
    assert_eq!(d["status"], json!("Done"));
    assert_eq!(d["note"], json!("hello"));
    assert_eq!(d["title"], json!("Draft renamed"));
    let plain = f.ok(project::run, &["item-list", n]);
    assert!(plain.contains("Issue\ttracked issue"), "{plain}");
    f.ok(
        project::run,
        &[
            "item-edit",
            "--id",
            &draft_id,
            "--project-id",
            n,
            "--field-id",
            &note_id,
            "--clear",
        ],
    );

    // Copy with drafts keeps fields and draft values.
    let copy = json_of(&f.ok(
        project::run,
        &[
            "copy",
            n,
            "--title",
            "Roadmap copy",
            "--drafts",
            "--format",
            "json",
        ],
    ));
    assert_eq!(copy["fields"]["totalCount"], json!(2));
    assert_eq!(copy["items"]["totalCount"], json!(1));

    // Archive / delete items, delete field.
    f.ok(project::run, &["item-archive", n, "--id", &draft_id]);
    assert_eq!(f.ok(project::run, &["item-list", n]).lines().count(), 1);
    f.ok(
        project::run,
        &["item-archive", n, "--id", &draft_id, "--undo"],
    );
    f.ok(project::run, &["item-delete", n, "--id", &draft_id]);
    assert_eq!(f.ok(project::run, &["item-list", n]).lines().count(), 1);
    f.ok(project::run, &["field-delete", "--id", &note_id]);
    assert!(!f.ok(project::run, &["field-list", n]).contains("Note"));

    // Link / unlink, template, close.
    let linked = f.ok(project::run, &["link", n, "-R", "local/playground"]);
    assert!(linked.contains("Linked 'local/playground'"), "{linked}");
    assert!(
        f.ok(project::run, &["view", n])
            .contains("repositories:\tlocal/playground")
    );
    f.ok(project::run, &["unlink", n, "-R", "local/playground"]);
    assert!(is_api(
        f.run(project::run, &["unlink", n, "-R", "local/playground"])
    ));
    assert!(is_usage(f.run(project::run, &["link", n, "-T", "team"])));
    f.ok(project::run, &["mark-template", n]);
    assert_eq!(
        json_of(&f.ok(project::run, &["view", n, "--format", "json"]))["template"],
        json!(true)
    );
    f.ok(project::run, &["close", n]);
    let open = f.ok(project::run, &["list"]);
    assert!(!open.contains("Roadmap\t"), "{open}");
    let all = f.ok(project::run, &["list", "--closed"]);
    assert!(all.contains("Roadmap\tclosed"), "{all}");
    f.ok(project::run, &["close", n, "--undo"]);

    let deleted = f.ok(project::run, &["delete", n]);
    assert!(deleted.contains("Deleted project"));
    assert!(is_api(f.run(project::run, &["view", n])));
}

#[test]
fn ssh_and_gpg_key_leaves() {
    let f = Forge::start();
    let k1 = f.file("id1.pub", &ssh_ed25519(1, "me@laptop"));
    let k2 = f.file("id2.pub", &ssh_ed25519(2, "signer"));
    let added = f.ok(keys::run_ssh, &["add", &k1, "-t", "laptop"]);
    assert!(added.contains("SHA256:"), "{added}");
    f.ok(keys::run_ssh, &["add", &k2, "--type", "signing"]);
    assert!(is_api(f.run(keys::run_ssh, &["add", &k1])));
    let bad = f.file("bad.pub", "ssh-dss AAAAB3Nz");
    assert!(is_usage(f.run(keys::run_ssh, &["add", &bad])));
    assert!(is_usage(
        f.run(keys::run_ssh, &["add", &k1, "--type", "other"])
    ));
    let list = f.ok(keys::run_ssh, &["list"]);
    assert!(list.contains("laptop\t1\tssh-ed25519 "), "{list}");
    assert!(
        list.contains("signer\t2\t") && list.contains("\tsigning"),
        "{list}"
    );
    f.ok(keys::run_ssh, &["delete", "2", "--yes"]);
    assert!(!f.ok(keys::run_ssh, &["list"]).contains("signer"));
    assert!(is_api(f.run(keys::run_ssh, &["delete", "2"])));

    let g = f.file("pub.asc", &pgp_v4(9, "Dev <dev@example.org>"));
    let added = f.ok(keys::run_gpg, &["add", &g, "-t", "work"]);
    let key_id = added.split_whitespace().nth(2).unwrap().to_string();
    assert_eq!(key_id.len(), 16, "{added}");
    assert!(is_api(f.run(keys::run_gpg, &["add", &g])));
    let not_armored = f.file("x.asc", "hello");
    assert!(is_usage(f.run(keys::run_gpg, &["add", &not_armored])));
    let list = f.ok(keys::run_gpg, &["list"]);
    assert!(
        list.starts_with(&format!("dev@example.org\t{key_id}\t")),
        "{list}"
    );
    f.ok(
        keys::run_gpg,
        &["delete", &key_id.to_ascii_lowercase(), "-y"],
    );
    assert!(f.ok(keys::run_gpg, &["list"]).is_empty());
}

#[test]
fn deploy_key_and_autolink_leaves() {
    let f = Forge::start();
    f.api("POST", "/user/repos", Some(json!({"name": "second"})));
    let k = f.file("deploy.pub", &ssh_ed25519(5, "ci"));
    let out = f.ok(keys::run_deploy, &["add", &k, "-t", "ci", "-w"]);
    assert!(out.contains("local/playground"), "{out}");
    assert!(is_api(
        f.run(keys::run_deploy, &["add", &k, "-R", "local/second"])
    ));
    let j = json_of(&f.ok(keys::run_deploy, &["list", "--json", "id,title,readOnly"]));
    assert_eq!(j, json!([{"id": 1, "title": "ci", "readOnly": false}]));
    assert!(
        f.ok(keys::run_deploy, &["list"])
            .contains("1\tci\tread-write\t")
    );
    assert!(
        f.ok(keys::run_deploy, &["list", "-R", "local/second"])
            .is_empty()
    );
    f.ok(keys::run_deploy, &["delete", "1"]);
    assert!(f.ok(keys::run_deploy, &["list"]).is_empty());

    let created = f.ok(
        autolink::run,
        &["create", "TICKET-", "https://tracker.example/t/<num>", "-n"],
    );
    assert!(created.starts_with("Created repository autolink 1 on local/playground"));
    assert!(is_api(f.run(
        autolink::run,
        &["create", "ticket-", "https://tracker.example/x/<num>"]
    )));
    assert!(is_usage(f.run(
        autolink::run,
        &["create", "BAD-", "https://tracker.example/no-placeholder"]
    )));
    let j = json_of(&f.ok(
        autolink::run,
        &["list", "--json", "keyPrefix,isAlphanumeric"],
    ));
    assert_eq!(
        j,
        json!([{"keyPrefix": "TICKET-", "isAlphanumeric": false}])
    );
    let view = f.ok(autolink::run, &["view", "1"]);
    assert!(
        view.contains("URL Template: https://tracker.example/t/<num>"),
        "{view}"
    );
    f.ok(autolink::run, &["delete", "1", "--yes"]);
    assert!(is_api(f.run(autolink::run, &["view", "1"])));
}

#[test]
fn leaves_without_token_map_to_auth_failure() {
    let ctx = Ctx {
        base: "http://127.0.0.1:9".into(),
        token: None,
        default_repo: Some("local/x".into()),
    };
    let mut out = Vec::new();
    let r = gist::run(&ctx, &["list".to_string()], &mut out);
    assert!(matches!(r, Err(Fail::Auth(_))));
}

/// User keys, gists and `@me` projects belong to the token's account (multi-agent
/// SoT concurrency contract item 5); a key stays unique across the forge.
#[test]
fn per_account_keys_gists_and_projects() {
    let f = Forge::start();
    let bob = f.account("bob", "tok-bob");
    let as_bob = |leaf: Leaf, argv: &[&str]| -> Result<String, Fail> {
        let args: Vec<String> = argv.iter().map(|s| s.to_string()).collect();
        let mut out = Vec::new();
        leaf(&bob, &args, &mut out)?;
        Ok(String::from_utf8(out).unwrap())
    };
    let key = f.file("id.pub", &ssh_ed25519(7, "local@example.com"));
    f.ok(keys::run_ssh, &["add", &key, "-t", "mine"]);
    let api = |c: &Ctx, path: &str| c.api("GET", path, None).unwrap();
    assert_eq!(api(&f.ctx, "/user/keys").as_array().unwrap().len(), 1);
    assert_eq!(api(&bob, "/user/keys"), json!([]));
    assert!(bob.api("DELETE", "/user/keys/1", None).is_err());
    assert!(
        as_bob(keys::run_ssh, &["add", &key, "-t", "dup"]).is_err(),
        "key in use"
    );
    let bkey = f.file("bob.pub", &ssh_ed25519(8, "bob@example.com"));
    as_bob(keys::run_ssh, &["add", &bkey, "-t", "bobs"]).unwrap();
    assert_eq!(api(&f.ctx, "/user/keys").as_array().unwrap().len(), 1);
    let a = f.file("a.txt", "alpha\n");
    let url = f.ok(gist::run, &["create", &a, "-d", "secret"]);
    let id = url.trim().rsplit('/').next().unwrap().to_string();
    assert!(as_bob(gist::run, &["list"]).unwrap().trim().is_empty());
    assert!(as_bob(gist::run, &["delete", &id, "--yes"]).is_err());
    assert!(f.ok(gist::run, &["list"]).contains("secret"));
    f.ok(project::run, &["create", "--title", "Mine"]);
    assert!(!as_bob(project::run, &["list"]).unwrap().contains("Mine"));
}
