//! Integration tests for native `arks lfs` CLI (E6).

use std::collections::HashMap;
use std::fs;
use std::io::{Read, Write};
use std::net::TcpListener;
use std::path::Path;
use std::process::{Command, Output, Stdio};
use std::sync::{Arc, Mutex};
use std::thread;

fn arks_bin() -> &'static str {
    env!("CARGO_BIN_EXE_arks")
}

fn run(dir: &Path, args: &[&str]) -> Output {
    Command::new(arks_bin())
        .current_dir(dir)
        .env("GIT_AUTHOR_NAME", "LFS Test")
        .env("GIT_AUTHOR_EMAIL", "lfs@example.com")
        .env("GIT_COMMITTER_NAME", "LFS Test")
        .env("GIT_COMMITTER_EMAIL", "lfs@example.com")
        .args(args)
        .output()
        .expect("spawn arks")
}

fn ok(dir: &Path, args: &[&str]) -> String {
    let out = run(dir, args);
    assert!(
        out.status.success(),
        "arks {:?} failed (status={:?}):\nstdout={}\nstderr={}",
        args,
        out.status.code(),
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout).trim().to_string()
}

fn run_stdin(dir: &Path, args: &[&str], input: &[u8]) -> Output {
    let mut child = Command::new(arks_bin())
        .current_dir(dir)
        .env("GIT_AUTHOR_NAME", "LFS Test")
        .env("GIT_AUTHOR_EMAIL", "lfs@example.com")
        .env("GIT_COMMITTER_NAME", "LFS Test")
        .env("GIT_COMMITTER_EMAIL", "lfs@example.com")
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn arks");
    child.stdin.as_mut().unwrap().write_all(input).unwrap();
    child.wait_with_output().unwrap()
}

type MockStore = Arc<Mutex<HashMap<String, Vec<u8>>>>;

/// Minimal loopback HTTP server implementing Git LFS Batch + Basic Transfer (`PUT`/`GET`).
fn spawn_mock_lfs_server(max_requests: usize) -> (String, MockStore) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = listener.local_addr().unwrap();
    let base_url = format!("http://{addr}/repo.git/info/lfs");
    let store: MockStore = Arc::new(Mutex::new(HashMap::new()));
    let store_bg = Arc::clone(&store);
    let base_bg = base_url.clone();

    thread::spawn(move || {
        for _ in 0..max_requests {
            let Ok((mut stream, _)) = listener.accept() else {
                break;
            };
            let mut raw = Vec::new();
            let mut buf = [0u8; 4096];
            let mut header_end = None;
            let mut content_len = 0usize;
            while header_end.is_none() {
                let Ok(n) = stream.read(&mut buf) else {
                    break;
                };
                if n == 0 {
                    break;
                }
                raw.extend_from_slice(&buf[..n]);
                if let Some(pos) = raw.windows(4).position(|w| w == b"\r\n\r\n") {
                    header_end = Some(pos + 4);
                    let hdr = String::from_utf8_lossy(&raw[..pos]);
                    for line in hdr.lines() {
                        if let Some((k, v)) = line.split_once(':') {
                            if k.trim().eq_ignore_ascii_case("content-length") {
                                content_len = v.trim().parse().unwrap_or(0);
                            }
                        }
                    }
                }
            }
            let Some(hend) = header_end else {
                continue;
            };
            while raw.len() < hend + content_len {
                let Ok(n) = stream.read(&mut buf) else {
                    break;
                };
                if n == 0 {
                    break;
                }
                raw.extend_from_slice(&buf[..n]);
            }
            let hdr_str = String::from_utf8_lossy(&raw[..hend]);
            let req_line = hdr_str.lines().next().unwrap_or("");
            let mut parts = req_line.split_whitespace();
            let method = parts.next().unwrap_or("");
            let path = parts.next().unwrap_or("");
            let body = &raw[hend..hend + content_len];

            if method == "POST" && path == "/repo.git/info/lfs/objects/batch" {
                let req_json: serde_json::Value = serde_json::from_slice(body).unwrap();
                let op = req_json["operation"].as_str().unwrap_or("download");
                let mut objs = Vec::new();
                for o in req_json["objects"].as_array().unwrap() {
                    let oid = o["oid"].as_str().unwrap();
                    let size = o["size"].as_u64().unwrap();
                    let href = format!("{base_bg}/objects/{oid}");
                    if op == "upload" {
                        objs.push(serde_json::json!({
                            "oid": oid,
                            "size": size,
                            "actions": { "upload": { "href": href } }
                        }));
                    } else {
                        objs.push(serde_json::json!({
                            "oid": oid,
                            "size": size,
                            "actions": { "download": { "href": href } }
                        }));
                    }
                }
                let resp_body = serde_json::json!({
                    "transfer": "basic",
                    "objects": objs
                })
                .to_string();
                let resp = format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: application/vnd.git-lfs+json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                    resp_body.len(),
                    resp_body
                );
                let _ = stream.write_all(resp.as_bytes());
            } else if method == "PUT" && path.starts_with("/repo.git/info/lfs/objects/") {
                let oid = path.trim_start_matches("/repo.git/info/lfs/objects/");
                store_bg
                    .lock()
                    .unwrap()
                    .insert(oid.to_string(), body.to_vec());
                let resp = "HTTP/1.1 200 OK\r\nContent-Length: 0\r\nConnection: close\r\n\r\n";
                let _ = stream.write_all(resp.as_bytes());
            } else if method == "GET" && path.starts_with("/repo.git/info/lfs/objects/") {
                let oid = path.trim_start_matches("/repo.git/info/lfs/objects/");
                if let Some(data) = store_bg.lock().unwrap().get(oid).cloned() {
                    let hdr = format!(
                        "HTTP/1.1 200 OK\r\nContent-Type: application/octet-stream\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                        data.len()
                    );
                    let _ = stream.write_all(hdr.as_bytes());
                    let _ = stream.write_all(&data);
                } else {
                    let _ = stream.write_all(
                        b"HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\nConnection: close\r\n\r\n",
                    );
                }
            } else {
                let _ = stream.write_all(
                    b"HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\nConnection: close\r\n\r\n",
                );
            }
        }
    });

    (base_url, store)
}

#[test]
fn lfs_clean_smudge_track_and_batch_push_pull_roundtrip() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();

    ok(dir, &["init", "-b", "main"]);
    let ver = ok(dir, &["lfs", "version"]);
    assert!(ver.contains("arks-lfs/"), "ver={ver}");

    ok(dir, &["lfs", "install", "--local"]);
    let cfg = fs::read_to_string(dir.join(".git/config")).unwrap();
    assert!(cfg.contains("[filter \"lfs\"]"), "cfg={cfg}");

    ok(dir, &["lfs", "track", "*.bin"]);
    let attrs = fs::read_to_string(dir.join(".gitattributes")).unwrap();
    assert!(attrs.contains("*.bin filter=lfs"), "attrs={attrs}");

    // Clean a binary payload into `.git/lfs/objects` and commit the pointer file.
    let asset = b"super-secret-weights-payload-1234567890\n";
    let clean_out = run_stdin(dir, &["lfs", "clean"], asset);
    assert!(clean_out.status.success());
    let ptr_bytes = clean_out.stdout;
    let ptr_str = String::from_utf8_lossy(&ptr_bytes);
    assert!(
        ptr_str.starts_with("version https://git-lfs.github.com/spec/v1\noid sha256:"),
        "ptr={ptr_str}"
    );

    // Validate with `lfs pointer --check`.
    let check_out = run_stdin(dir, &["lfs", "pointer", "--check"], &ptr_bytes);
    assert!(check_out.status.success());

    // Commit the pointer blob to the repo.
    fs::write(dir.join("model.bin"), &ptr_bytes).unwrap();
    ok(dir, &["add", "model.bin", ".gitattributes"]);
    ok(dir, &["commit", "-m", "add lfs model"]);

    let ls = ok(dir, &["lfs", "ls-files"]);
    assert!(ls.contains("* model.bin"), "ls={ls}");

    // Smudge returns the exact original bytes.
    let smudge_out = run_stdin(dir, &["lfs", "smudge"], &ptr_bytes);
    assert!(smudge_out.status.success());
    assert_eq!(smudge_out.stdout, asset);

    // Push to mock LFS Batch+Basic server (2 requests: batch + PUT), then wipe local store and pull (2 requests: batch + GET).
    let (endpoint, server_store) = spawn_mock_lfs_server(4);
    let push_out = Command::new(arks_bin())
        .current_dir(dir)
        .env("ARKS_LFS_ENDPOINT", &endpoint)
        .args(["lfs", "push"])
        .output()
        .unwrap();
    assert!(
        push_out.status.success(),
        "lfs push failed: {}",
        String::from_utf8_lossy(&push_out.stderr)
    );
    assert_eq!(server_store.lock().unwrap().len(), 1);

    // Wipe local LFS objects and verify ls-files shows `-` (missing).
    fs::remove_dir_all(dir.join(".git/lfs")).unwrap();
    let ls_missing = ok(dir, &["lfs", "ls-files"]);
    assert!(
        ls_missing.contains("- model.bin"),
        "ls_missing={ls_missing}"
    );

    // Pull restores object into local store and smudges `model.bin` in worktree!
    let pull_out = Command::new(arks_bin())
        .current_dir(dir)
        .env("ARKS_LFS_ENDPOINT", &endpoint)
        .args(["lfs", "pull"])
        .output()
        .unwrap();
    assert!(
        pull_out.status.success(),
        "lfs pull failed: {}",
        String::from_utf8_lossy(&pull_out.stderr)
    );
    assert_eq!(fs::read(dir.join("model.bin")).unwrap(), asset);

    // Untrack removes rule from .gitattributes.
    ok(dir, &["lfs", "untrack", "*.bin"]);
    let attrs_after = fs::read_to_string(dir.join(".gitattributes")).unwrap();
    assert!(!attrs_after.contains("*.bin filter=lfs"));
}
