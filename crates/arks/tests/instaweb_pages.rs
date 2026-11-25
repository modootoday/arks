//! The instaweb repository browser pages over HTTP: status codes, content
//! types, commit ids, subjects, file lists and raw blob bytes.
//!
//! There is no stock oracle: the clean-room image ships gitweb.cgi but its
//! Perl CGI module is missing, so these checks follow the documentation.
//! Primary: <https://git-scm.com/docs/gitweb> (ACTIONS, AND URLS),
//! <https://git-scm.com/docs/git-instaweb>.

use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Output, Stdio};
use std::time::Duration;

fn arks(dir: &Path) -> Command {
    let mut c = Command::new(env!("CARGO_BIN_EXE_arks"));
    c.current_dir(dir)
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .env("ARKS_PERSONALITY", "git")
        .env("GIT_AUTHOR_NAME", "A U Thor")
        .env("GIT_AUTHOR_EMAIL", "author@example.com")
        .env("GIT_COMMITTER_NAME", "C O Mitter")
        .env("GIT_COMMITTER_EMAIL", "committer@example.com")
        .stdin(Stdio::null());
    c
}

fn ok_at(dir: &Path, args: &[&str], t: u64) -> String {
    let out: Output = arks(dir)
        .env("GIT_AUTHOR_DATE", format!("{t} +0000"))
        .env("GIT_COMMITTER_DATE", format!("{t} +0000"))
        .args(args)
        .output()
        .expect("run arks");
    assert!(
        out.status.success(),
        "{args:?} failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout).into_owned()
}

/// The corpus fixture's history shape (verify/fixtures/cli-flag-corpus),
/// built with arks: two branches, a binary file, an annotated tag and a
/// merge.
fn fixture(root: &Path) -> PathBuf {
    let d = root.join("repo");
    std::fs::create_dir_all(d.join("src")).unwrap();
    std::fs::create_dir_all(d.join("docs")).unwrap();
    let mut t = 1_700_000_000;
    let mut step = |args: &[&str]| {
        t += 3600;
        ok_at(&d, args, t)
    };
    step(&["init", "-q", "-b", "main"]);
    step(&["config", "commit.gpgsign", "false"]);
    std::fs::write(d.join("README.md"), "readme\n").unwrap();
    std::fs::write(d.join("src/a.txt"), "alpha\nbeta\ngamma\n").unwrap();
    std::fs::write(d.join("src/b.txt"), "one\ntwo\nthree\n").unwrap();
    step(&["add", "."]);
    step(&["commit", "-q", "-m", "initial import"]);
    step(&["tag", "t0"]);
    std::fs::write(d.join("src/a.txt"), "alpha\nbeta\nGAMMA\n").unwrap();
    std::fs::write(d.join("docs/x.md"), "# Docs\n").unwrap();
    step(&["add", "."]);
    step(&[
        "commit",
        "-q",
        "-m",
        "update a, add docs",
        "-m",
        "Body line one.\nBody line two.",
    ]);
    step(&["checkout", "-q", "-b", "feature"]);
    std::fs::write(d.join("src/b.txt"), "one\nTWO\nthree\nfour\n").unwrap();
    step(&["commit", "-q", "-am", "feature: change b"]);
    std::fs::write(d.join("src/data.bin"), b"bin\0ary\n").unwrap();
    step(&["add", "src/data.bin"]);
    step(&["commit", "-q", "-m", "feature: add <binary> & co"]);
    step(&["checkout", "-q", "main"]);
    std::fs::write(d.join("README.md"), "readme\nmore readme\n").unwrap();
    step(&["commit", "-q", "-am", "main: extend readme"]);
    step(&["tag", "-a", "v1", "-m", "release v1"]);
    step(&[
        "merge",
        "-q",
        "--no-ff",
        "feature",
        "-m",
        "Merge branch 'feature'",
    ]);
    std::fs::write(d.join("README.md"), "readme\nmore readme\nlast\n").unwrap();
    step(&["commit", "-q", "-am", "main: tail readme"]);
    step(&["config", "gitweb.owner", "A U Thor"]);
    std::fs::write(d.join(".git/description"), "corpus fixture\n").unwrap();
    d
}

fn free_port() -> u16 {
    TcpListener::bind("127.0.0.1:0")
        .unwrap()
        .local_addr()
        .unwrap()
        .port()
}

struct Server(Child, u16);

impl Drop for Server {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

fn serve(d: &Path) -> Server {
    let port = free_port();
    let child = arks(d)
        .env("ARKS_PERSONALITY", "arks")
        .args(["serve-http", "--gitweb", "--listen"])
        .arg(format!("127.0.0.1:{port}"))
        .arg(d.join(".git"))
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();
    for _ in 0..200 {
        if TcpStream::connect(("127.0.0.1", port)).is_ok() {
            break;
        }
        std::thread::sleep(Duration::from_millis(20));
    }
    Server(child, port)
}

struct Resp {
    status: u16,
    content_type: String,
    headers: String,
    body: Vec<u8>,
}

impl Resp {
    fn text(&self) -> String {
        String::from_utf8_lossy(&self.body).into_owned()
    }
}

fn get(port: u16, path: &str) -> Resp {
    let mut s = TcpStream::connect(("127.0.0.1", port)).unwrap();
    s.set_read_timeout(Some(Duration::from_secs(30))).unwrap();
    write!(
        s,
        "GET {path} HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\nConnection: close\r\n\r\n"
    )
    .unwrap();
    let mut raw = Vec::new();
    s.read_to_end(&mut raw).unwrap();
    let split = raw
        .windows(4)
        .position(|w| w == b"\r\n\r\n")
        .expect("header end");
    let headers = String::from_utf8_lossy(&raw[..split]).into_owned();
    let status = headers
        .split_whitespace()
        .nth(1)
        .and_then(|s| s.parse().ok())
        .unwrap();
    let content_type = headers
        .lines()
        .find_map(|l| l.strip_prefix("Content-Type: "))
        .unwrap_or("")
        .to_string();
    Resp {
        status,
        content_type,
        headers,
        body: raw[split + 4..].to_vec(),
    }
}

fn html(port: u16, path: &str) -> String {
    let r = get(port, path);
    assert_eq!(r.status, 200, "{path}: {}", r.text());
    assert!(
        r.content_type.starts_with("text/html"),
        "{path}: {}",
        r.content_type
    );
    r.text()
}

fn rev(d: &Path, r: &str) -> String {
    ok_at(d, &["rev-parse", r], 0).trim().to_string()
}

#[test]
fn project_pages_list_the_history() {
    let t = tempfile::tempdir().unwrap();
    let d = fixture(t.path());
    let srv = serve(&d);
    let port = srv.1;

    let list = html(port, "/");
    assert!(list.contains(".git") && list.contains("corpus fixture"));
    assert!(list.contains("A U Thor"));
    assert_eq!(html(port, "/gitweb.cgi"), list);

    let summary = html(port, "/?p=.git");
    for s in [
        "main: tail readme",
        "release v1",
        "feature",
        "main",
        "v1",
        "t0",
    ] {
        assert!(summary.contains(s), "summary lacks {s}");
    }

    // shortlog: every commit, in the default walk order.
    let order: Vec<String> = ok_at(&d, &["rev-list", "HEAD"], 0)
        .lines()
        .map(str::to_string)
        .collect();
    assert_eq!(order.len(), 7);
    let short = html(port, "/?p=.git;a=shortlog");
    let mut last = 0;
    for id in &order {
        let at = short.find(id.as_str()).unwrap_or_else(|| panic!("no {id}"));
        assert!(at >= last, "shortlog order differs at {id}");
        last = at;
    }
    assert!(short.contains("feature: add &lt;binary&gt; &amp; co"));
    assert!(!short.contains("<binary>"));
    // `h` limits the walk; `&` separates as well as `;`.
    let feat = html(port, "/?p=.git&a=shortlog&h=feature");
    assert!(feat.contains(&rev(&d, "feature")));
    assert!(!feat.contains(&rev(&d, "main")));
    // path_info form.
    assert_eq!(html(port, "/.git/shortlog/feature"), feat);

    let log = html(port, "/?p=.git;a=log");
    assert!(log.contains("Body line one.\nBody line two."));

    let heads = html(port, "/?p=.git;a=heads");
    assert!(heads.contains("refs/heads/feature") && heads.contains("refs/heads/main"));
    let tags = html(port, "/?p=.git;a=tags");
    assert!(tags.contains("release v1"));
    let v1 = rev(&d, "refs/tags/v1");
    assert!(tags.contains(&format!("a=tag;h={v1}")));
    let tag = html(port, &format!("/?p=.git;a=tag;h={v1}"));
    assert!(tag.contains("release v1") && tag.contains(&rev(&d, "v1^{}")));

    let index = get(port, "/?a=project_index");
    assert_eq!(index.status, 200);
    assert!(index.content_type.starts_with("text/plain"));
    assert_eq!(index.text(), ".git A+U+Thor\n");
}

#[test]
fn commit_pages_show_ids_and_changes() {
    let t = tempfile::tempdir().unwrap();
    let d = fixture(t.path());
    let srv = serve(&d);
    let port = srv.1;
    let head = rev(&d, "HEAD");
    let parent = rev(&d, "HEAD^");
    let tree = rev(&d, "HEAD^{tree}");

    let c = html(port, "/?p=.git;a=commit;h=HEAD");
    for s in [
        head.as_str(),
        &parent,
        &tree,
        "README.md",
        "main: tail readme",
    ] {
        assert!(c.contains(s), "commit lacks {s}");
    }
    assert!(c.contains("A U Thor &lt;author@example.com&gt;"));
    let merge = html(port, &format!("/?p=.git;a=commit;h={}", rev(&d, "HEAD^")));
    assert!(merge.contains(&rev(&d, "HEAD^^1")) && merge.contains(&rev(&d, "HEAD^^2")));

    let cd = html(port, &format!("/?p=.git;a=commitdiff;h={head}"));
    assert!(cd.contains("diff --git a/README.md b/README.md"));
    assert!(cd.contains("+last"));

    let plain = get(port, "/?p=.git;a=commitdiff_plain;h=HEAD");
    assert_eq!(plain.status, 200);
    assert!(plain.content_type.starts_with("text/plain"));
    let plain = plain.text();
    assert!(plain.contains("Subject: main: tail readme"));
    assert!(plain.contains(" more readme\n+last\n"));

    let patch = get(port, "/?p=.git;a=patch;h=HEAD").text();
    assert!(patch.starts_with(&format!("From {head} Mon Sep 17 00:00:00 2001\n")));
    assert!(patch.contains("Subject: [PATCH] main: tail readme\n"));
    assert!(patch.contains(" README.md | 1 +\n"));
    assert!(patch.contains("diff --git a/README.md b/README.md\n"));

    let rss = get(port, "/?p=.git;a=rss");
    assert_eq!(rss.status, 200);
    assert!(rss.content_type.starts_with("application/rss+xml"));
    assert!(
        rss.text()
            .contains(&format!("<guid isPermaLink=\"false\">{head}</guid>"))
    );
    let atom = get(port, "/?p=.git;a=atom");
    assert!(atom.content_type.starts_with("application/atom+xml"));
    let atom = atom.text();
    assert!(atom.contains("<title type=\"text\">main: tail readme</title>"));
    assert!(atom.contains(&format!(
        "http://127.0.0.1:{port}/?p=.git;a=commit;h={head}"
    )));
}

#[test]
fn tree_and_blob_pages_serve_files() {
    let t = tempfile::tempdir().unwrap();
    let d = fixture(t.path());
    let srv = serve(&d);
    let port = srv.1;

    let root = html(port, "/?p=.git;a=tree");
    for s in ["README.md", "src", "docs", "drwxr-xr-x", "-rw-r--r--"] {
        assert!(root.contains(s), "tree lacks {s}");
    }
    let src = html(port, "/?p=.git;a=tree;hb=feature;f=src");
    for s in ["a.txt", "b.txt", "data.bin"] {
        assert!(src.contains(s), "src tree lacks {s}");
    }
    // v1 predates the merge of `feature`.
    assert!(!html(port, "/?p=.git;a=tree;hb=v1;f=src").contains("data.bin"));
    assert!(html(port, "/.git/tree/feature:/src").contains("data.bin"));
    // A path with no action shows a tree or a file.
    assert!(html(port, "/.git/HEAD:/src").contains("b.txt"));
    assert!(html(port, "/.git/HEAD:/README.md").contains("more readme"));

    let blob = html(port, "/?p=.git;a=blob;hb=HEAD;f=src/b.txt");
    assert!(blob.contains("TWO") && blob.contains("id=\"l4\""));

    let raw = get(port, "/?p=.git;a=blob_plain;hb=HEAD;f=README.md");
    assert_eq!(raw.status, 200);
    assert!(raw.content_type.starts_with("text/plain"));
    assert_eq!(raw.body, b"readme\nmore readme\nlast\n");
    let bin = get(port, "/?p=.git;a=blob_plain;hb=feature;f=src/data.bin");
    assert_eq!(bin.content_type, "application/octet-stream");
    assert_eq!(bin.body, b"bin\0ary\n");
    assert!(bin.headers.contains("filename=\"data.bin\""));
    let by_id = get(
        port,
        &format!("/?p=.git;a=blob_plain;h={}", rev(&d, "HEAD:src/a.txt")),
    );
    assert_eq!(by_id.body, b"alpha\nbeta\nGAMMA\n");
}

#[test]
fn errors_and_git_http_keep_their_answers() {
    let t = tempfile::tempdir().unwrap();
    let d = fixture(t.path());
    let srv = serve(&d);
    let port = srv.1;
    for (path, status) in [
        ("/?p=.git;a=blame;f=README.md", 403),
        ("/?p=.git;a=blame_incremental;f=README.md", 403),
        ("/?p=.git;a=history;f=README.md", 501),
        ("/?p=.git;a=snapshot", 501),
        ("/?p=.git;a=bogus", 400),
        ("/?a=summary", 400),
        ("/?p=nope.git", 404),
        ("/?p=.git;a=commit;h=nosuchrev", 404),
        ("/?p=.git;a=commit;h=--all", 404),
        ("/?p=.git;a=blob;hb=HEAD;f=../../etc/passwd", 404),
        ("/?p=.git;a=blob;hb=HEAD;f=src", 400),
        ("/static/nothing.js", 404),
    ] {
        let r = get(port, path);
        assert_eq!(r.status, status, "{path}");
    }
    let css = get(port, "/static/gitweb.css");
    assert_eq!((css.status, css.content_type.as_str()), (200, "text/css"));
    // git HTTP is untouched beside the pages.
    let refs = get(port, "/info/refs");
    assert_eq!(refs.status, 200);
    assert!(refs.text().contains(&rev(&d, "main")));
    let smart = get(port, "/info/refs?service=git-upload-pack");
    assert_eq!(smart.status, 200);
    assert!(smart.content_type.contains("git-upload-pack-advertisement"));
}

#[cfg(unix)]
#[test]
fn instaweb_start_serves_the_pages() {
    let t = tempfile::tempdir().unwrap();
    let d = fixture(t.path());
    let port = free_port();
    let p = port.to_string();
    ok_at(&d, &["instaweb", "--local", "--port", &p, "--start"], 0);
    let summary = get(port, "/?p=.git;a=summary");
    ok_at(&d, &["instaweb", "--stop"], 0);
    assert_eq!(summary.status, 200);
    assert!(summary.text().contains("main: tail readme"));
    let conf = std::fs::read_to_string(d.join(".git/gitweb/instaweb.conf")).unwrap();
    assert!(conf.contains("server = arks serve-http --gitweb"));
}
