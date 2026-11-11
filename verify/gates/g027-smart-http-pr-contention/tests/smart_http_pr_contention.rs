//! Gate G27 — arks smart HTTP as GitHub stand-in: thousands-class branch/PR
//! seed + N concurrent stock-Git clone/fetch/push; contended same-ref fail-closed.
//!
//! SoT: `.spec/sot/arks-smart-http-pr-contention-clean-room.sot.md`
//! Matrix: `verify/matrices/matrix-g027-smart-http-pr-contention.md`
//! Honesty: ≠ GitHub UX/Checks/merge-queue; ≠ live forge throughput.

use arks_hash::ObjectFormat;
use arks_object::{ObjectKind, RawObject};
use arks_oid::Oid;
use arks_refs::FilesystemRefStore;
use arks_store::{FilesystemGitStore, ObjectStore};
use std::fs;
use std::io::Write;
use std::net::TcpStream;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::{Arc, Barrier, Mutex};
use std::thread;
use std::time::Duration;
use tempfile::TempDir;

fn n_branches() -> usize {
    std::env::var("ARKS_G27_N_BRANCHES")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(2048)
}

fn n_pr() -> usize {
    std::env::var("ARKS_G27_N_PR")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(2048)
}

fn n_agents() -> usize {
    std::env::var("ARKS_G27_N_AGENTS")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(8)
}

fn require_git() {
    let out = Command::new("git").arg("--version").output().expect("git");
    assert!(out.status.success(), "stock git required for G27");
}

fn arks_bin() -> PathBuf {
    let mut candidates: Vec<PathBuf> = Vec::new();
    if let Ok(target) = std::env::var("CARGO_TARGET_DIR") {
        candidates.push(PathBuf::from(&target).join("debug/arks.exe"));
        candidates.push(PathBuf::from(&target).join("debug/arks"));
    } else {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../..");
        candidates.push(root.join("target/debug/arks.exe"));
        candidates.push(root.join("target/debug/arks"));
        candidates.push(root.join("target-cleanroom/debug/arks.exe"));
        candidates.push(root.join("target-cleanroom/debug/arks"));
    }
    for c in &candidates {
        if c.is_file() {
            return c.clone();
        }
    }
    panic!("missing arks binary; tried {candidates:?} (run: cargo build -p arks)");
}

fn arks(cwd: &Path, args: &[&str]) -> std::process::Output {
    Command::new(arks_bin())
        .args(args)
        .current_dir(cwd)
        .env("GIT_AUTHOR_NAME", "arks")
        .env("GIT_AUTHOR_EMAIL", "arks@example.com")
        .env("GIT_COMMITTER_NAME", "arks")
        .env("GIT_COMMITTER_EMAIL", "arks@example.com")
        .env("GIT_AUTHOR_DATE", "1000000000 +0000")
        .env("GIT_COMMITTER_DATE", "1000000000 +0000")
        .output()
        .expect("arks")
}

fn git(cwd: &Path, args: &[&str]) -> std::process::Output {
    Command::new("git")
        .args(args)
        .current_dir(cwd)
        .env("GIT_AUTHOR_NAME", "arks")
        .env("GIT_AUTHOR_EMAIL", "arks@example.com")
        .env("GIT_COMMITTER_NAME", "arks")
        .env("GIT_COMMITTER_EMAIL", "arks@example.com")
        .env("GIT_AUTHOR_DATE", "1000000000 +0000")
        .env("GIT_COMMITTER_DATE", "1000000000 +0000")
        .output()
        .expect("git")
}

fn assert_ok(out: &std::process::Output, label: &str) {
    assert!(
        out.status.success(),
        "{label}: status={:?}\nstdout={}\nstderr={}",
        out.status.code(),
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
}

fn free_port() -> u16 {
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    listener.local_addr().unwrap().port()
}

fn read_scale_field(prev: &str, key: &str) -> Option<usize> {
    let i = prev.find(key)?;
    let rest = &prev[i + key.len()..];
    let num: String = rest
        .chars()
        .skip_while(|c| c.is_whitespace())
        .take_while(|c| c.is_ascii_digit())
        .collect();
    num.parse().ok()
}

fn write_scale(branches: usize, pr: usize, agents: usize) {
    let Ok(path) = std::env::var("ARKS_G27_SCALE_OUT") else {
        return;
    };
    if let Some(parent) = Path::new(&path).parent() {
        let _ = fs::create_dir_all(parent);
    }
    // Merge maxima across cases (later cases must not zero earlier seed scale).
    let (mut b, mut p, mut a) = (branches, pr, agents);
    if let Ok(prev) = fs::read_to_string(&path) {
        if let Some(n) = read_scale_field(&prev, "\"n_branches\":") {
            b = b.max(n);
        }
        if let Some(n) = read_scale_field(&prev, "\"n_pr\":") {
            p = p.max(n);
        }
        if let Some(n) = read_scale_field(&prev, "\"n_agents\":") {
            a = a.max(n);
        }
    }
    let body = format!("{{\n  \"n_branches\": {b},\n  \"n_pr\": {p},\n  \"n_agents\": {a}\n}}\n");
    let _ = fs::write(&path, body);
}

struct ArksHttp {
    child: Child,
    url: String,
}

impl ArksHttp {
    fn start(bare: &Path) -> Self {
        let port = free_port();
        let listen = format!("127.0.0.1:{port}");
        let stderr_path = bare
            .parent()
            .unwrap_or(bare)
            .join(format!("serve-http-{port}.stderr"));
        let stderr_file = fs::File::create(&stderr_path).expect("serve-http stderr file");
        let child = Command::new(arks_bin())
            .args(["serve-http", "--listen", &listen, bare.to_str().unwrap()])
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::from(stderr_file))
            .spawn()
            .expect("spawn serve-http");
        let url = format!("http://{listen}/");
        let mut ready = false;
        for _ in 0..100 {
            if let Ok(mut s) = TcpStream::connect(format!("127.0.0.1:{port}")) {
                let _ = s.set_read_timeout(Some(Duration::from_secs(2)));
                let _ = s.set_write_timeout(Some(Duration::from_secs(2)));
                let _ = write!(
                    s,
                    "GET /info/refs?service=git-upload-pack HTTP/1.0\r\nHost: 127.0.0.1\r\n\r\n"
                );
                let mut buf = [0u8; 64];
                if let Ok(n) = std::io::Read::read(&mut s, &mut buf) {
                    let head = String::from_utf8_lossy(&buf[..n]);
                    if head.contains("200") || head.contains("application/x-git-upload-pack") {
                        ready = true;
                        break;
                    }
                }
            }
            thread::sleep(Duration::from_millis(50));
        }
        assert!(
            ready,
            "serve-http not ready on {listen}; stderr={}",
            fs::read_to_string(&stderr_path).unwrap_or_default()
        );
        Self { child, url }
    }
}

impl Drop for ArksHttp {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

fn objects_dir(git: &Path) -> PathBuf {
    git.join("objects")
}

fn seed_blob(store: &mut FilesystemGitStore, payload: &[u8]) -> Oid {
    let blob = RawObject::new(ObjectKind::Blob, payload.to_vec());
    store.put(ObjectFormat::Sha1, &blob).unwrap()
}

fn seed_bare_with_commit(root: &Path) -> (PathBuf, Oid) {
    let work = root.join("seed");
    fs::create_dir_all(&work).unwrap();
    assert_ok(&arks(&work, &["init"]), "arks init");
    fs::write(work.join("hello.txt"), b"hello g027-smart-http-pr-contention\n").unwrap();
    assert_ok(&arks(&work, &["add", "hello.txt"]), "arks add");
    assert_ok(&arks(&work, &["commit", "-m", "initial"]), "arks commit");
    let tip_out = git(&work, &["rev-parse", "HEAD"]);
    assert_ok(&tip_out, "rev-parse HEAD");
    let tip_hex = String::from_utf8_lossy(&tip_out.stdout).trim().to_string();
    let tip = Oid::from_hex(&tip_hex).expect("oid");

    let bare = root.join("server.git");
    assert_ok(
        &git(
            root,
            &[
                "clone",
                "--bare",
                work.join(".git").to_str().unwrap(),
                bare.to_str().unwrap(),
            ],
        ),
        "bare clone fixture",
    );
    // Allow receive-pack updates without denyCurrentBranch issues on bare.
    let _ = git(&bare, &["config", "http.receivepack", "true"]);
    let _ = git(&bare, &["config", "receive.denyCurrentBranch", "ignore"]);
    (bare, tip)
}

fn seed_thousands_refs(bare: &Path, tip: &Oid, n_br: usize, n_pr: usize) {
    let mut store = FilesystemGitStore::new(objects_dir(bare)).unwrap();
    let merge_oid = seed_blob(&mut store, b"g027-smart-http-pr-contention-pr-merge-marker");
    let mut refs = FilesystemRefStore::open(bare).unwrap();
    for i in 0..n_br {
        refs.update(&format!("refs/heads/br-{i:04}"), tip, None)
            .unwrap();
    }
    for i in 1..=n_pr {
        refs.update(&format!("refs/pull/{i}/head"), tip, None)
            .unwrap();
        if i % 2 == 0 {
            refs.update(&format!("refs/pull/{i}/merge"), &merge_oid, None)
                .unwrap();
        }
    }
}

fn count_ls_remote(url: &str, filter: impl Fn(&str) -> bool) -> usize {
    let out = Command::new("git")
        .args(["ls-remote", url])
        .output()
        .expect("ls-remote");
    assert!(
        out.status.success(),
        "ls-remote failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter(|l| {
            let parts: Vec<_> = l.split_whitespace().collect();
            parts.len() >= 2 && filter(parts[1])
        })
        .count()
}

/// G27-C01 — seed scale visible via arks smart HTTP ls-remote.
#[test]
fn g027_smart_http_pr_contention_c01_seed_scale_via_smart_http() {
    require_git();
    let n_br = n_branches();
    let n_pr = n_pr();
    assert!(n_br >= 2048, "minimum N_branches is 2048");
    assert!(n_pr >= 2048, "minimum N_pr is 2048");

    let root = TempDir::new().unwrap();
    let (bare, tip) = seed_bare_with_commit(root.path());
    seed_thousands_refs(&bare, &tip, n_br, n_pr);
    let server = ArksHttp::start(&bare);

    let br = count_ls_remote(&server.url, |r| r.starts_with("refs/heads/br-"));
    let heads = count_ls_remote(&server.url, |r| r.contains("/head") && r.starts_with("refs/pull/"));
    let merges = count_ls_remote(&server.url, |r| {
        r.contains("/merge") && r.starts_with("refs/pull/")
    });
    assert_eq!(br, n_br, "branch seed mismatch via ls-remote");
    assert_eq!(heads, n_pr, "PR head seed mismatch via ls-remote");
    assert_eq!(merges, n_pr / 2, "PR merge seed mismatch via ls-remote");

    let sample = git(root.path(), &["ls-remote", &server.url, "refs/pull/1/head"]);
    assert_ok(&sample, "sample ls-remote pull/1/head");
    assert!(
        String::from_utf8_lossy(&sample.stdout).contains(&tip.to_hex()),
        "sample tip mismatch"
    );

    write_scale(n_br, n_pr, n_agents());
}

/// G27-C02 — concurrent stock git clone against arks smart HTTP.
#[test]
fn g027_smart_http_pr_contention_c02_concurrent_clone() {
    require_git();
    let agents = n_agents();
    assert!(agents >= 8, "minimum N_agents is 8");

    let root = TempDir::new().unwrap();
    let (bare, tip) = seed_bare_with_commit(root.path());
    // Modest seed for clone matrix (full thousands covered in C01).
    seed_thousands_refs(&bare, &tip, 256, 256);
    let server = ArksHttp::start(&bare);
    let url = server.url.clone();
    let barrier = Arc::new(Barrier::new(agents));
    let results = Arc::new(Mutex::new(Vec::with_capacity(agents)));
    let mut handles = Vec::with_capacity(agents);
    for i in 0..agents {
        let barrier = Arc::clone(&barrier);
        let results = Arc::clone(&results);
        let url = url.clone();
        let dest = root.path().join(format!("clone-{i}"));
        handles.push(thread::spawn(move || {
            barrier.wait();
            let out = Command::new("git")
                .args(["clone", &url, dest.to_str().unwrap()])
                .output()
                .expect("clone");
            let ok = out.status.success();
            let body_ok = ok
                && fs::read_to_string(dest.join("hello.txt"))
                    .map(|b| b.replace("\r\n", "\n").starts_with("hello g027-smart-http-pr-contention"))
                    .unwrap_or(false);
            results.lock().unwrap().push((i, body_ok, out.status.code()));
        }));
    }
    for h in handles {
        h.join().expect("clone worker panicked");
    }
    let outcomes = results.lock().unwrap();
    let pass = outcomes.iter().filter(|(_, ok, _)| *ok).count();
    assert_eq!(
        pass, agents,
        "all clones must succeed; outcomes={outcomes:?}"
    );
    write_scale(256, 256, agents);
}

/// G27-C03 — concurrent fetch after tip advance.
#[test]
fn g027_smart_http_pr_contention_c03_concurrent_fetch() {
    require_git();
    let agents = n_agents();
    assert!(agents >= 8);

    let root = TempDir::new().unwrap();
    let (bare, tip) = seed_bare_with_commit(root.path());
    seed_thousands_refs(&bare, &tip, 128, 128);
    let server = ArksHttp::start(&bare);

    // Pre-clone agents.
    let mut clones = Vec::with_capacity(agents);
    for i in 0..agents {
        let dest = root.path().join(format!("fetch-clone-{i}"));
        assert_ok(
            &git(root.path(), &["clone", &server.url, dest.to_str().unwrap()]),
            &format!("pre-clone {i}"),
        );
        clones.push(dest);
    }

    // Advance tip via one client push of a new branch (visible on advertise).
    let pusher = &clones[0];
    fs::write(pusher.join("hello.txt"), b"hello fetch-advance\n").unwrap();
    assert_ok(&git(pusher, &["add", "hello.txt"]), "add advance");
    assert_ok(&git(pusher, &["commit", "-m", "advance"]), "commit advance");
    assert_ok(
        &git(pusher, &["push", "origin", "HEAD:refs/heads/advance"]),
        "push advance",
    );

    let barrier = Arc::new(Barrier::new(agents));
    let results = Arc::new(Mutex::new(Vec::with_capacity(agents)));
    let mut handles = Vec::with_capacity(agents);
    for (i, dest) in clones.into_iter().enumerate() {
        let barrier = Arc::clone(&barrier);
        let results = Arc::clone(&results);
        handles.push(thread::spawn(move || {
            barrier.wait();
            let out = git(&dest, &["fetch", "origin"]);
            let ok = out.status.success();
            let tip_ok = if ok {
                let rev = git(&dest, &["rev-parse", "origin/advance"]);
                rev.status.success()
            } else {
                false
            };
            results.lock().unwrap().push((i, tip_ok, out.status.code()));
        }));
    }
    for h in handles {
        h.join().expect("fetch worker panicked");
    }
    let outcomes = results.lock().unwrap();
    let pass = outcomes.iter().filter(|(_, ok, _)| *ok).count();
    assert_eq!(
        pass, agents,
        "all fetches must see advance; outcomes={outcomes:?}"
    );
    write_scale(128, 128, agents);
}

/// G27-C04 — concurrent distinct-ref pushes.
#[test]
fn g027_smart_http_pr_contention_c04_concurrent_distinct_push() {
    require_git();
    let agents = n_agents();
    assert!(agents >= 8);

    let root = TempDir::new().unwrap();
    let (bare, tip) = seed_bare_with_commit(root.path());
    seed_thousands_refs(&bare, &tip, 64, 64);
    let server = ArksHttp::start(&bare);

    let barrier = Arc::new(Barrier::new(agents));
    let results = Arc::new(Mutex::new(Vec::with_capacity(agents)));
    let mut handles = Vec::with_capacity(agents);
    for i in 0..agents {
        let barrier = Arc::clone(&barrier);
        let results = Arc::clone(&results);
        let url = server.url.clone();
        let dest = root.path().join(format!("push-clone-{i}"));
        handles.push(thread::spawn(move || {
            let clone = Command::new("git")
                .args(["clone", &url, dest.to_str().unwrap()])
                .output()
                .expect("clone");
            if !clone.status.success() {
                results.lock().unwrap().push((i, false, clone.status.code()));
                return;
            }
            fs::write(dest.join("agent.txt"), format!("agent-{i}\n")).unwrap();
            let _ = git(&dest, &["add", "agent.txt"]);
            let _ = git(&dest, &["commit", "-m", &format!("agent-{i}")]);
            barrier.wait();
            let branch = format!("refs/heads/agent-{i:03}");
            let pr = format!("refs/pull/{}/head", 9000 + i);
            let push_br = git(&dest, &["push", "origin", &format!("HEAD:{branch}")]);
            let push_pr = git(&dest, &["push", "origin", &format!("HEAD:{pr}")]);
            let ok = push_br.status.success() && push_pr.status.success();
            results
                .lock()
                .unwrap()
                .push((i, ok, push_br.status.code()));
        }));
    }
    for h in handles {
        h.join().expect("push worker panicked");
    }
    let outcomes = results.lock().unwrap();
    let pass = outcomes.iter().filter(|(_, ok, _)| *ok).count();
    assert_eq!(
        pass, agents,
        "all distinct pushes must succeed; outcomes={outcomes:?}"
    );

    // Server agrees via ls-remote.
    let agent_refs = count_ls_remote(&server.url, |r| r.starts_with("refs/heads/agent-"));
    assert_eq!(agent_refs, agents);
    let pr_extra = count_ls_remote(&server.url, |r| {
        r.starts_with("refs/pull/9") && r.ends_with("/head")
    });
    assert!(pr_extra >= agents, "expected agent PR heads; got {pr_extra}");
    write_scale(64, 64, agents);
}

/// G27-C05 — contended same-ref push fail-closed; tip stock-resolvable.
#[test]
fn g027_smart_http_pr_contention_c05_contended_same_ref_fail_closed() {
    require_git();
    let agents = n_agents();
    assert!(agents >= 8);

    let root = TempDir::new().unwrap();
    let (bare, tip) = seed_bare_with_commit(root.path());
    // Pre-create contended ref at known tip.
    {
        let mut refs = FilesystemRefStore::open(&bare).unwrap();
        refs.update("refs/heads/contended", &tip, None).unwrap();
    }
    let server = ArksHttp::start(&bare);

    let barrier = Arc::new(Barrier::new(agents));
    let results = Arc::new(Mutex::new(Vec::with_capacity(agents)));
    let mut handles = Vec::with_capacity(agents);
    for i in 0..agents {
        let barrier = Arc::clone(&barrier);
        let results = Arc::clone(&results);
        let url = server.url.clone();
        let dest = root.path().join(format!("race-clone-{i}"));
        handles.push(thread::spawn(move || {
            let clone = Command::new("git")
                .args(["clone", &url, dest.to_str().unwrap()])
                .output()
                .expect("clone");
            if !clone.status.success() {
                results.lock().unwrap().push((i, false, "clone-fail".into()));
                return;
            }
            // Fetch contended tip as base.
            let _ = git(&dest, &["fetch", "origin", "refs/heads/contended:contended"]);
            let _ = git(&dest, &["checkout", "-B", "contended", "contended"]);
            fs::write(dest.join("race.txt"), format!("race-{i}\n")).unwrap();
            let _ = git(&dest, &["add", "race.txt"]);
            let _ = git(&dest, &["commit", "-m", &format!("race-{i}")]);
            barrier.wait();
            let push = git(&dest, &["push", "origin", "HEAD:refs/heads/contended"]);
            let ok = push.status.success();
            let detail = if ok {
                "ok".into()
            } else {
                format!(
                    "fail:{}",
                    String::from_utf8_lossy(&push.stderr)
                        .lines()
                        .take(2)
                        .collect::<Vec<_>>()
                        .join("|")
                )
            };
            results.lock().unwrap().push((i, ok, detail));
        }));
    }
    for h in handles {
        h.join().expect("race worker panicked");
    }
    let outcomes = results.lock().unwrap();
    let wins = outcomes.iter().filter(|(_, ok, _)| *ok).count();
    assert!(
        wins <= 1,
        "at most one contended push may succeed; outcomes={outcomes:?}"
    );
    // Tip must remain stock-resolvable via smart HTTP (single consistent OID).
    let ls = Command::new("git")
        .args(["ls-remote", &server.url, "refs/heads/contended"])
        .output()
        .expect("ls-remote contended");
    assert!(
        ls.status.success(),
        "contended tip must resolve: {}",
        String::from_utf8_lossy(&ls.stderr)
    );
    let line = String::from_utf8_lossy(&ls.stdout);
    let oid = line.split_whitespace().next().unwrap_or("");
    assert_eq!(oid.len(), 40, "expected single 40-hex tip; got {line:?}");
    // If someone won, tip != original seed tip; if all failed, tip may still be seed.
    if wins == 1 {
        assert_ne!(oid, tip.to_hex(), "winner must have advanced tip");
    }
    write_scale(0, 0, agents);
}
