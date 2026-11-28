//! Integration tests for `arks serve` embedded GitHub/GitLab SSR + HTMX v2 Web UI (E8).

use arks_server::{ServeHttpOptions, WEB_UI_CSP, spawn_loopback_with_opts, web_ui_page_handler};
use std::fs;
use std::io::{Read, Write};
use std::net::{SocketAddr, TcpStream};
use std::path::Path;
use std::process::{Command, Output};
use std::sync::atomic::Ordering;

fn arks_bin() -> &'static str {
    env!("CARGO_BIN_EXE_arks")
}

fn ok(dir: &Path, args: &[&str]) -> String {
    let out: Output = Command::new(arks_bin())
        .current_dir(dir)
        .env("GIT_AUTHOR_NAME", "WebUI Test")
        .env("GIT_AUTHOR_EMAIL", "webui@example.com")
        .env("GIT_COMMITTER_NAME", "WebUI Test")
        .env("GIT_COMMITTER_EMAIL", "webui@example.com")
        .args(args)
        .output()
        .expect("spawn arks");
    assert!(
        out.status.success(),
        "arks {:?} failed:\nstdout={}\nstderr={}",
        args,
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout).trim().to_string()
}

fn http_get(addr: SocketAddr, path: &str) -> (u16, String, String) {
    let mut stream = TcpStream::connect(addr).unwrap();
    let req = format!("GET {path} HTTP/1.1\r\nHost: {addr}\r\nConnection: close\r\n\r\n");
    stream.write_all(req.as_bytes()).unwrap();
    let mut raw = Vec::new();
    stream.read_to_end(&mut raw).unwrap();
    let text = String::from_utf8_lossy(&raw).into_owned();
    let (hdr, body) = text.split_once("\r\n\r\n").unwrap_or((&text, ""));
    let status: u16 = hdr
        .lines()
        .next()
        .and_then(|l| l.split_whitespace().nth(1))
        .and_then(|s| s.parse().ok())
        .unwrap_or(0);
    (status, hdr.to_string(), body.to_string())
}

#[test]
fn serve_embedded_web_ui_renders_escapes_xss_sets_csp_and_coexists_with_git() {
    let tmp = tempfile::tempdir().unwrap();
    let repo = tmp.path().join("repo");
    fs::create_dir_all(&repo).unwrap();

    ok(&repo, &["init", "-b", "main"]);
    fs::write(
        repo.join("README.md"),
        "# Demo Repo\n\n- Safe `arks` item\n",
    )
    .unwrap();
    fs::write(repo.join("xss.txt"), "<script>alert('xss')</script>\n").unwrap();
    ok(&repo, &["add", "README.md", "xss.txt"]);
    ok(&repo, &["commit", "-m", "initial <b>commit</b>"]);
    fs::write(
        repo.join("xss.txt"),
        "<script>alert('xss')</script>\nsecond line\n",
    )
    .unwrap();
    ok(&repo, &["add", "xss.txt"]);
    ok(&repo, &["commit", "-m", "update xss.txt"]);
    let head = ok(&repo, &["rev-parse", "HEAD"]);
    ok(&repo, &["tag", "v0.1.0"]);

    let git_dir = repo.join(".git");
    let (addr, stop, handle) = spawn_loopback_with_opts(
        git_dir.clone(),
        ServeHttpOptions {
            pages: Some(web_ui_page_handler(&git_dir)),
            ..ServeHttpOptions::default()
        },
    )
    .unwrap();

    // 1. Overview `/` has GitHub/GitLab tabs, HTMX hx-boost, strict CSP, README preview, and escapes HTML.
    let (st, hdr, body) = http_get(addr, "/");
    assert_eq!(st, 200);
    assert!(
        hdr.contains(&format!("Content-Security-Policy: {WEB_UI_CSP}")),
        "missing strict CSP header:\n{hdr}"
    );
    assert!(body.contains("hx-boost=\"true\""), "missing hx-boost");
    assert!(body.contains("/assets/htmx.min.js"), "missing htmx script");
    assert!(body.contains("/favicon.svg"), "missing favicon link");
    assert!(
        body.contains("<h1>Demo Repo</h1>"),
        "missing README preview"
    );
    assert!(body.contains("xss.txt"), "body={body}");

    // 2. Embedded CSS, HTMX v2 JS, and SVG favicon are served directly from binary.
    let (st_css, hdr_css, body_css) = http_get(addr, "/style.css");
    assert_eq!(st_css, 200);
    assert!(hdr_css.contains("text/css"), "hdr={hdr_css}");
    assert!(body_css.contains("--bg:"), "css={body_css}");

    let (st_js, hdr_js, body_js) = http_get(addr, "/assets/htmx.min.js");
    assert_eq!(st_js, 200);
    assert!(hdr_js.contains("application/javascript"), "hdr_js={hdr_js}");
    assert!(body_js.contains("htmx"), "htmx js missing");

    let (st_ico, hdr_ico, body_ico) = http_get(addr, "/favicon.svg");
    assert_eq!(st_ico, 200);
    assert!(hdr_ico.contains("image/svg+xml"), "hdr_ico={hdr_ico}");
    assert!(body_ico.contains("<svg"), "svg missing");

    // 3. Blob viewer escapes `<script>` payload and provides `#L1` anchors + `/raw/` download.
    let (st_blob, _, body_blob) = http_get(addr, "/blob/main/xss.txt");
    assert_eq!(st_blob, 200);
    assert!(
        body_blob.contains("&lt;script&gt;alert(&#x27;xss&#x27;)&lt;/script&gt;"),
        "body_blob={body_blob}"
    );
    assert!(
        !body_blob.contains("<script>alert"),
        "raw <script> tag leaked!"
    );
    assert!(body_blob.contains("id=\"L1\""), "missing line anchor");

    let (st_raw, _, body_raw) = http_get(addr, "/raw/main/xss.txt");
    assert_eq!(st_raw, 200);
    assert!(body_raw.contains("second line"));

    // 4. Commits, Commit Detail (with unified diff + escaping), Branches, Tags, Refs.
    let (st_c, _, body_c) = http_get(addr, "/commits/main");
    assert_eq!(st_c, 200);
    assert!(body_c.contains(&head[..7]));
    assert!(body_c.contains("&lt;b&gt;commit&lt;/b&gt;"));
    assert!(!body_c.contains("<b>commit</b>"));

    let (st_d, _, body_d) = http_get(addr, &format!("/commit/{head}"));
    assert_eq!(st_d, 200);
    assert!(
        body_d.contains("diff-add"),
        "missing colorized unified diff"
    );
    assert!(body_d.contains("second line"));

    let (st_b, _, body_b) = http_get(addr, "/branches");
    assert_eq!(st_b, 200);
    assert!(body_b.contains("main"));

    let (st_t, _, body_t) = http_get(addr, "/tags");
    assert_eq!(st_t, 200);
    assert!(body_t.contains("v0.1.0"));

    let (st_r, _, body_r) = http_get(addr, "/refs");
    assert_eq!(st_r, 200);
    assert!(body_r.contains("refs/heads/main"));

    // 5. Git smart HTTP `/info/refs?service=git-upload-pack` is untouched!
    let (st_git, hdr_git, body_git) = http_get(addr, "/info/refs?service=git-upload-pack");
    assert_eq!(st_git, 200);
    assert!(
        hdr_git.contains("application/x-git-upload-pack-advertisement"),
        "hdr_git={hdr_git}"
    );
    assert!(body_git.contains("refs/heads/main"));

    stop.store(true, Ordering::SeqCst);
    let _ = handle.join();

    // 6. With `--no-web-ui` (`pages: None`), `/` is not served as HTML Web UI.
    let (addr2, stop2, handle2) =
        spawn_loopback_with_opts(git_dir, ServeHttpOptions::default()).unwrap();
    let (st_off, _, _) = http_get(addr2, "/");
    assert_eq!(st_off, 404);
    stop2.store(true, Ordering::SeqCst);
    let _ = handle2.join();
}
