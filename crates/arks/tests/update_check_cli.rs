//! `arks update-git-for-windows` checks a release feed for a newer arks and
//! only reports it. Feeds here are local files and a local HTTP server; the
//! tests never contact the internet (`ARKS_UPDATE_URL` is always set).

use std::io::{Read, Write};
use std::net::TcpListener;
use std::process::{Command, Output};

fn run(args: &[&str], url: &str) -> Output {
    Command::new(env!("CARGO_BIN_EXE_arks"))
        .arg("update-git-for-windows")
        .args(args)
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .env("ARKS_PERSONALITY", "git")
        .env("ARKS_UPDATE_URL", url)
        .output()
        .unwrap()
}

fn stdout(o: &Output) -> String {
    String::from_utf8_lossy(&o.stdout).into_owned()
}

#[test]
fn file_feed_newer_and_current() {
    let t = tempfile::tempdir().unwrap();
    let newer = t.path().join("newer.json");
    std::fs::write(
        &newer,
        r#"{"tag_name":"v999.0.0","html_url":"https://example.invalid/arks/v999","prerelease":false}"#,
    )
    .unwrap();
    let url = format!("file://{}", newer.display());
    for args in [&[][..], &["-y"], &["--yes", "--gui"], &["--quiet"]] {
        let o = run(args, &url);
        assert!(o.status.success(), "{args:?}: {o:?}");
        let out = stdout(&o);
        assert!(
            out.starts_with("A newer arks release is available: v999.0.0"),
            "{out}"
        );
        assert!(out.contains("https://example.invalid/arks/v999\n"), "{out}");
        assert!(out.contains("never installs Git for Windows"), "{out}");
    }
    let same = t.path().join("same.txt");
    std::fs::write(&same, format!("{}\n", env!("CARGO_PKG_VERSION"))).unwrap();
    let url = format!("file://{}", same.display());
    let o = run(&[], &url);
    assert!(o.status.success());
    assert_eq!(
        stdout(&o),
        format!("arks {} is up to date.\n", env!("CARGO_PKG_VERSION"))
    );
    let o = run(&["--quiet"], &url);
    assert!(o.status.success());
    assert!(o.stdout.is_empty());
}

#[test]
fn testing_counts_prereleases() {
    let t = tempfile::tempdir().unwrap();
    let f = t.path().join("list.json");
    std::fs::write(
        &f,
        r#"[{"tag_name":"v999.0.0-rc1","prerelease":true},{"tag_name":"v0.0.0"}]"#,
    )
    .unwrap();
    let url = format!("file://{}", f.display());
    assert!(stdout(&run(&[], &url)).contains("is up to date"));
    assert!(stdout(&run(&["--testing"], &url)).contains("v999.0.0-rc1"));
}

#[test]
fn local_http_feed() {
    let l = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = l.local_addr().unwrap().port();
    let server = std::thread::spawn(move || {
        let (mut s, _) = l.accept().unwrap();
        let mut buf = [0u8; 2048];
        let n = s.read(&mut buf).unwrap();
        let req = String::from_utf8_lossy(&buf[..n]).into_owned();
        let body = r#"{"tag_name":"v999.1.0","html_url":"http://127.0.0.1/r"}"#;
        write!(
            s,
            "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
            body.len()
        )
        .unwrap();
        req
    });
    let o = run(&[], &format!("http://127.0.0.1:{port}/releases/latest"));
    let req = server.join().unwrap();
    assert!(req.starts_with("GET /releases/latest "), "{req}");
    assert!(o.status.success(), "{o:?}");
    assert!(stdout(&o).contains("v999.1.0"));
}

#[test]
fn errors() {
    let o = run(&[], "file:///nonexistent/feed");
    assert_eq!(o.status.code(), Some(1));
    let o = run(&[], "ftp://example.invalid/x");
    assert_eq!(o.status.code(), Some(1));
    let o = run(&["--bogus"], "file:///x");
    assert_eq!(o.status.code(), Some(129));
    let o = run(&["extra"], "file:///x");
    assert_eq!(o.status.code(), Some(129));
}
