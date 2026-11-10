//! Host-advisory server unit tests (G15 is authoritative vs stock Git).

use super::*;
use arks_object::{Commit, ObjectKind, RawObject, Signature, Tree, TreeEntry, TreeMode};
use arks_repo::Repository;
use arks_store::{ObjectFormat, ObjectStore};
use std::sync::atomic::Ordering;
use std::thread;
use std::time::Duration;
use tempfile::TempDir;

fn seed_bare(root: &std::path::Path) -> (std::path::PathBuf, arks_oid::Oid) {
    let git = root.to_path_buf();
    std::fs::create_dir_all(git.join("objects")).unwrap();
    std::fs::create_dir_all(git.join("refs/heads")).unwrap();
    std::fs::write(git.join("HEAD"), b"ref: refs/heads/main\n").unwrap();
    let mut repo = Repository::open(&git).unwrap();
    let blob = RawObject::new(ObjectKind::Blob, b"hello server\n".to_vec());
    let blob_oid = repo.store_mut().put(ObjectFormat::Sha1, &blob).unwrap();
    let tree = Tree::new(vec![TreeEntry {
        mode: TreeMode::File,
        name: "hello.txt".into(),
        oid: blob_oid,
    }])
    .unwrap();
    let tree_obj = RawObject::new(ObjectKind::Tree, tree.encode().unwrap());
    let tree_oid = repo.store_mut().put(ObjectFormat::Sha1, &tree_obj).unwrap();
    let sig = Signature {
        name: "arks".into(),
        email: "arks@example.com".into(),
        timestamp: 1_700_000_000,
        timezone: "+0000".into(),
    };
    let commit = Commit {
        tree: tree_oid,
        parents: Vec::new(),
        author: sig.clone(),
        committer: sig,
        gpgsig: None,
        message: "initial\n".into(),
    };
    let commit_obj = RawObject::new(ObjectKind::Commit, commit.encode().unwrap());
    let commit_oid = repo
        .store_mut()
        .put(ObjectFormat::Sha1, &commit_obj)
        .unwrap();
    repo.refs_mut()
        .update("refs/heads/main", &commit_oid, None)
        .unwrap();
    (git, commit_oid)
}

#[test]
fn upload_advertisement_includes_main() {
    let dir = TempDir::new().unwrap();
    let (git, oid) = seed_bare(dir.path());
    let repo = Repository::open(&git).unwrap();
    let body = upload_advertisement(&repo).unwrap();
    let text = String::from_utf8_lossy(&body);
    assert!(text.contains("git-upload-pack"));
    assert!(text.contains(&oid.to_hex()));
    assert!(text.contains("refs/heads/main"));
    assert!(text.contains("side-band-64k"));
    assert!(!text.contains("agent=git/"));
}

#[test]
fn receive_advertises_delete_refs_and_report_status_v2() {
    let dir = TempDir::new().unwrap();
    let (git, _) = seed_bare(dir.path());
    let repo = Repository::open(&git).unwrap();
    let body = receive_advertisement(&repo).unwrap();
    let text = String::from_utf8_lossy(&body);
    assert!(text.contains("report-status"));
    assert!(text.contains("report-status-v2"));
    assert!(text.contains("delete-refs"));
    // O4: push-options advertised; FS2: proc-receive only when hook present.
    assert!(text.contains("push-options"));
    assert!(!text.contains("proc-receive"));
}

#[test]
fn receive_advertises_proc_receive_when_hook_present() {
    let dir = TempDir::new().unwrap();
    let (git, _) = seed_bare(dir.path());
    let hooks = git.join("hooks");
    std::fs::create_dir_all(&hooks).unwrap();
    let hook = hooks.join("proc-receive");
    std::fs::write(&hook, b"#!/bin/sh\nexit 0\n").unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = std::fs::metadata(&hook).unwrap().permissions();
        perms.set_mode(0o755);
        std::fs::set_permissions(&hook, perms).unwrap();
    }
    let repo = Repository::open(&git).unwrap();
    let body = receive_advertisement(&repo).unwrap();
    let text = String::from_utf8_lossy(&body);
    assert!(
        text.contains("proc-receive"),
        "FS2 must advertise proc-receive when hook is runnable: {text}"
    );
}

#[test]
fn smart_http_info_refs_upload() {
    let dir = TempDir::new().unwrap();
    let (git, _) = seed_bare(dir.path());
    let mut repo = Repository::open(&git).unwrap();
    let req = HttpRequest {
        method: "GET".into(),
        path: "/repo.git/info/refs".into(),
        query: "service=git-upload-pack".into(),
        body: Vec::new(),
        git_protocol: None,
        authorization: None,
        destination: None,
        lock_token: None,
    };
    let resp = handle_smart_http(&mut repo, &req).unwrap();
    assert_eq!(resp.status, 200);
    assert!(resp.content_type.contains("git-upload-pack-advertisement"));
}

#[test]
fn smart_http_basic_auth_challenge_and_accept() {
    let dir = TempDir::new().unwrap();
    let (git, _) = seed_bare(dir.path());
    let mut repo = Repository::open(&git).unwrap();
    let cfg = BasicAuthConfig::single("alice", "s3cret");
    let denied = HttpRequest {
        method: "GET".into(),
        path: "/info/refs".into(),
        query: "service=git-upload-pack".into(),
        body: Vec::new(),
        git_protocol: None,
        authorization: None,
        destination: None,
        lock_token: None,
    };
    let resp = handle_smart_http_auth(&mut repo, &denied, Some(&cfg)).unwrap();
    assert_eq!(resp.status, 401);
    assert_eq!(
        resp.www_authenticate.as_deref(),
        Some("Basic realm=\"arks\"")
    );

    let ok = HttpRequest {
        method: "GET".into(),
        path: "/info/refs".into(),
        query: "service=git-upload-pack".into(),
        body: Vec::new(),
        git_protocol: None,
        authorization: Some(basic_authorization_header("alice", "s3cret")),
        destination: None,
        lock_token: None,
    };
    let resp = handle_smart_http_auth(&mut repo, &ok, Some(&cfg)).unwrap();
    assert_eq!(resp.status, 200);
    assert!(resp.content_type.contains("git-upload-pack-advertisement"));
}

#[test]
fn loopback_serves_info_refs() {
    let dir = TempDir::new().unwrap();
    let (git, _) = seed_bare(dir.path());
    let (addr, stop, handle) = spawn_loopback(git).unwrap();
    thread::sleep(Duration::from_millis(50));
    let url = format!("http://{addr}/info/refs?service=git-upload-pack");
    let body = ureq_get(&url);
    stop.store(true, Ordering::SeqCst);
    let _ = handle.join();
    assert!(body.contains("refs/heads/main") || body.contains("# service=git-upload-pack"));
}

fn ureq_get(url: &str) -> String {
    // Avoid adding ureq to arks-server deps: raw TCP client for the smoke test.
    use std::io::{Read, Write};
    use std::net::TcpStream;
    let url = url.strip_prefix("http://").unwrap();
    let (hostport, path) = url.split_once('/').unwrap();
    let mut stream = TcpStream::connect(hostport).unwrap();
    let req = format!("GET /{path} HTTP/1.1\r\nHost: {hostport}\r\nConnection: close\r\n\r\n");
    stream.write_all(req.as_bytes()).unwrap();
    let mut buf = Vec::new();
    stream.read_to_end(&mut buf).unwrap();
    String::from_utf8_lossy(&buf).into_owned()
}

fn zero_oid() -> String {
    "0".repeat(40)
}

fn receive_create_branch_body(new_oid: &arks_oid::Oid, refname: &str) -> Vec<u8> {
    use arks_protocol::{Packet, encode_packet};
    let line = format!("{} {} {}\n", zero_oid(), new_oid.to_hex(), refname);
    let mut body = encode_packet(&Packet::Data(line.into_bytes())).unwrap();
    body.extend(encode_packet(&Packet::Flush).unwrap());
    body
}

fn install_hook(git: &std::path::Path, name: &str, script: &str) {
    let hooks = git.join("hooks");
    std::fs::create_dir_all(&hooks).unwrap();
    let path = hooks.join(name);
    std::fs::write(&path, script).unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = std::fs::metadata(&path).unwrap().permissions();
        perms.set_mode(0o755);
        std::fs::set_permissions(&path, perms).unwrap();
    }
}

#[test]
fn receive_pre_receive_hook_rejects_entire_push() {
    let dir = TempDir::new().unwrap();
    let (git, oid) = seed_bare(dir.path());
    install_hook(&git, "pre-receive", "#!/bin/sh\nexit 1\n");
    let mut repo = Repository::open(&git).unwrap();
    let body = receive_create_branch_body(&oid, "refs/heads/topic");
    let out = receive_pack(&mut repo, &body).unwrap();
    let text = String::from_utf8_lossy(&out);
    assert!(
        text.contains("pre-receive hook declined"),
        "expected decline in {text}"
    );
    assert!(repo.refs().resolve("refs/heads/topic").is_err());
}

#[test]
fn receive_update_hook_rejects_one_ref() {
    let dir = TempDir::new().unwrap();
    let (git, oid) = seed_bare(dir.path());
    install_hook(&git, "update", "#!/bin/sh\nexit 1\n");
    let mut repo = Repository::open(&git).unwrap();
    let body = receive_create_branch_body(&oid, "refs/heads/topic");
    let out = receive_pack(&mut repo, &body).unwrap();
    let text = String::from_utf8_lossy(&out);
    assert!(text.contains("hook declined"), "expected decline in {text}");
    assert!(repo.refs().resolve("refs/heads/topic").is_err());
}

#[test]
fn receive_post_receive_runs_after_success() {
    let dir = TempDir::new().unwrap();
    let (git, oid) = seed_bare(dir.path());
    let marker = git.join("post-receive-ran");
    // Isolate scratch under $GIT_DIR (per-push; no shared parallel scratch).
    install_hook(
        &git,
        "post-receive",
        "#!/bin/sh\ncat > \"$GIT_DIR/post-receive-ran\"\n",
    );
    let mut repo = Repository::open(&git).unwrap();
    let body = receive_create_branch_body(&oid, "refs/heads/topic");
    let out = receive_pack(&mut repo, &body).unwrap();
    let text = String::from_utf8_lossy(&out);
    assert!(
        text.contains("ok refs/heads/topic"),
        "expected ok in {text}"
    );
    assert_eq!(repo.refs().resolve("refs/heads/topic").unwrap(), oid);
    assert!(
        marker.is_file(),
        "post-receive should have written {}",
        marker.display()
    );
}
