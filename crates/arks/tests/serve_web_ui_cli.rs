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
    assert!(body_d.contains("diff-ln"), "missing dual line numbers");
    assert!(
        body_d.contains("raw-diff-payload"),
        "missing diff2html payload"
    );
    assert!(body_d.contains("second line"));

    // Vendored diff2html + app.js assets
    let (st_d2h_js, _, _) = http_get(addr, "/assets/diff2html.min.js");
    assert_eq!(st_d2h_js, 200);
    let (st_d2h_css, _, _) = http_get(addr, "/assets/diff2html.min.css");
    assert_eq!(st_d2h_css, 200);
    let (st_app_js, _, _) = http_get(addr, "/assets/app.js");
    assert_eq!(st_app_js, 200);

    // HTMX live /search partial + /blame/ + /compare/
    let (st_s, _, body_s) = http_get(addr, "/search?rev=main&q=xss");
    assert_eq!(st_s, 200);
    assert!(body_s.contains("xss.txt"), "search body={body_s}");

    let (st_bl, _, body_bl) = http_get(addr, "/blame/main/xss.txt");
    assert_eq!(st_bl, 200);
    assert!(body_bl.contains("blame-cell"), "missing blame attribution");

    let (st_cmp, _, body_cmp) = http_get(addr, "/compare/v0.1.0..main");
    assert_eq!(st_cmp, 200);
    assert!(body_cmp.contains("Compare"), "compare body={body_cmp}");

    // All 9-tab self-hosted forge routes return 200 with strict CSP
    for route in [
        "/issues",
        "/pulls",
        "/actions",
        "/projects",
        "/wiki",
        "/security",
        "/insights",
        "/settings",
    ] {
        let (st_f, hdr_f, _) = http_get(addr, route);
        assert_eq!(st_f, 200, "route {route} failed");
        assert!(
            hdr_f.contains("default-src 'self'"),
            "missing CSP on {route}"
        );
    }

    let (st_b, _, body_b) = http_get(addr, "/branches");
    assert_eq!(st_b, 200);
    assert!(body_b.contains("main"));

    let (st_t, _, body_t) = http_get(addr, "/tags");
    assert_eq!(st_t, 200);
    assert!(body_t.contains("v0.1.0"));

    let (st_r, _, body_r) = http_get(addr, "/refs");
    assert_eq!(st_r, 200);
    assert!(body_r.contains("main"));

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

fn http_post(addr: SocketAddr, path: &str, form_body: &str) -> (u16, String, String) {
    let mut stream = TcpStream::connect(addr).unwrap();
    let req = format!(
        "POST {path} HTTP/1.1\r\nHost: {addr}\r\nContent-Type: application/x-www-form-urlencoded\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{form_body}",
        form_body.len()
    );
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
fn serve_interactive_forge_issues_comments_and_pr_fast_forward_merge() {
    let tmp = tempfile::tempdir().unwrap();
    let repo = tmp.path().join("repo");
    fs::create_dir_all(&repo).unwrap();

    ok(&repo, &["init", "-b", "main"]);
    fs::write(repo.join("README.md"), "# Interactive Forge\n").unwrap();
    ok(&repo, &["add", "README.md"]);
    ok(&repo, &["commit", "-m", "base commit"]);
    let base_oid = ok(&repo, &["rev-parse", "HEAD"]);

    // Create a feature branch `feat/demo` with one commit ahead of main.
    ok(&repo, &["checkout", "-b", "feat/demo"]);
    fs::write(
        repo.join("feature.rs"),
        "pub fn greet() -> &'static str { \"hi\" }\n",
    )
    .unwrap();
    ok(&repo, &["add", "feature.rs"]);
    ok(&repo, &["commit", "-m", "add feature.rs"]);
    let feat_oid = ok(&repo, &["rev-parse", "HEAD"]);
    assert_ne!(base_oid, feat_oid);
    ok(&repo, &["checkout", "main"]);

    let git_dir = repo.join(".git");
    let (addr, stop, handle) = spawn_loopback_with_opts(
        git_dir.clone(),
        ServeHttpOptions {
            pages: Some(web_ui_page_handler(&git_dir)),
            ..ServeHttpOptions::default()
        },
    )
    .unwrap();

    // 1. Create an issue with XSS payload -> 303 redirect to /issues/1.
    let (st_create, hdr_create, _) = http_post(
        addr,
        "/issues",
        "title=Fix+%3Cscript%3Ealert(1)%3C%2Fscript%3E&author=alice&labels=bug%2Cweb&body=Details+here",
    );
    assert_eq!(st_create, 303, "hdr={hdr_create}");
    assert!(
        hdr_create.contains("Location: /issues/1"),
        "hdr={hdr_create}"
    );

    // 2. View issue #1 -> escaped title, open badge, labels.
    let (st_i1, _, body_i1) = http_get(addr, "/issues/1");
    assert_eq!(st_i1, 200);
    assert!(body_i1.contains("&lt;script&gt;alert(1)&lt;/script&gt;"));
    assert!(!body_i1.contains("<script>alert(1)</script>"));
    assert!(body_i1.contains("open"));
    assert!(body_i1.contains("alice"));
    assert!(body_i1.contains("Details here"));

    // 3. Post a comment on issue #1.
    let (st_cmt, hdr_cmt, _) = http_post(
        addr,
        "/issues/1/comment",
        "author=bob&body=LGTM+%3Cb%3Enow%3C%2Fb%3E",
    );
    assert_eq!(st_cmt, 303, "hdr={hdr_cmt}");
    let (_, _, body_i1_after) = http_get(addr, "/issues/1");
    assert!(body_i1_after.contains("bob"));
    assert!(body_i1_after.contains("LGTM &lt;b&gt;now&lt;/b&gt;"));

    // 4. Toggle issue #1 state open -> closed.
    let (st_tgl, hdr_tgl, _) = http_post(addr, "/issues/1/state", "");
    assert_eq!(st_tgl, 303, "hdr={hdr_tgl}");
    let (_, _, body_i1_closed) = http_get(addr, "/issues/1");
    assert!(body_i1_closed.contains("closed"));

    // 5. Inspect Pull Request detail for `feat/demo` (branch with `/`).
    let (st_pr, _, body_pr) = http_get(addr, "/pulls/feat/demo");
    assert_eq!(st_pr, 200);
    assert!(body_pr.contains("feat/demo"));
    assert!(body_pr.contains("feature.rs"));
    assert!(body_pr.contains("/pulls/feat/demo/merge"));

    // 6. Leave a PR review comment.
    let (st_pr_cmt, _, _) = http_post(
        addr,
        "/pulls/feat/demo/comment",
        "author=reviewer&body=Clean+fast-forward+change",
    );
    assert_eq!(st_pr_cmt, 303);
    let (_, _, body_pr2) = http_get(addr, "/pulls/feat/demo");
    assert!(body_pr2.contains("Clean fast-forward change"));

    // 7. Native fast-forward merge via POST /pulls/feat/demo/merge.
    let (st_merge, hdr_merge, _) = http_post(addr, "/pulls/feat/demo/merge", "");
    assert_eq!(st_merge, 303, "hdr={hdr_merge}");
    let main_after = ok(&repo, &["rev-parse", "refs/heads/main"]);
    assert_eq!(
        main_after, feat_oid,
        "refs/heads/main should have fast-forwarded to feat/demo"
    );

    // 8. Verify ForgeStore persisted atomically under `<git_dir>/arks-forge/store.json`.
    assert!(git_dir.join("arks-forge").join("store.json").is_file());

    // 9. Verify POST /git-upload-pack is NOT swallowed by the Web UI (returns protocol status, not 405).
    let (st_rpc, _, _) = http_post(addr, "/git-upload-pack", "");
    assert_ne!(st_rpc, 405, "Web UI must never intercept /git-upload-pack");

    stop.store(true, Ordering::SeqCst);
    let _ = handle.join();
}
