//! Integration tests for `arks serve` embedded single-binary SSR Web UI (E8).

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
    fs::write(repo.join("xss.txt"), "<script>alert('xss')</script>\n").unwrap();
    ok(&repo, &["add", "xss.txt"]);
    ok(&repo, &["commit", "-m", "initial <b>commit</b>"]);
    let head = ok(&repo, &["rev-parse", "HEAD"]);

    let git_dir = repo.join(".git");
    let (addr, stop, handle) = spawn_loopback_with_opts(
        git_dir.clone(),
        ServeHttpOptions {
            pages: Some(web_ui_page_handler(&git_dir)),
            ..ServeHttpOptions::default()
        },
    )
    .unwrap();

    // 1. Overview `/` has strict CSP and escapes HTML in commit subject & author.
    let (st, hdr, body) = http_get(addr, "/");
    assert_eq!(st, 200);
    assert!(
        hdr.contains(&format!("Content-Security-Policy: {WEB_UI_CSP}")),
        "missing strict CSP header:\n{hdr}"
    );
    assert!(body.contains("xss.txt"), "body={body}");
    assert!(body.contains("&lt;b&gt;commit&lt;/b&gt;"), "body={body}");
    assert!(!body.contains("<b>commit</b>"), "unescaped HTML in body");

    // 2. Embedded stylesheet `/style.css` is served from binary.
    let (st_css, hdr_css, body_css) = http_get(addr, "/style.css");
    assert_eq!(st_css, 200);
    assert!(hdr_css.contains("text/css"), "hdr={hdr_css}");
    assert!(body_css.contains("--bg:"), "css={body_css}");

    // 3. Blob viewer escapes `<script>` payload completely.
    let (st_blob, _, body_blob) = http_get(addr, "/blob/main/xss.txt");
    assert_eq!(st_blob, 200);
    assert!(
        body_blob.contains("&lt;script&gt;alert(&#x27;xss&#x27;)&lt;/script&gt;"),
        "body_blob={body_blob}"
    );
    assert!(!body_blob.contains("<script>"), "raw <script> tag leaked!");

    // 4. Commits, commit detail, and refs pages all render 200 OK.
    let (st_c, _, body_c) = http_get(addr, "/commits/main");
    assert_eq!(st_c, 200);
    assert!(body_c.contains(&head[..7]));

    let (st_d, _, body_d) = http_get(addr, &format!("/commit/{head}"));
    assert_eq!(st_d, 200);
    assert!(body_d.contains("xss.txt"));

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
