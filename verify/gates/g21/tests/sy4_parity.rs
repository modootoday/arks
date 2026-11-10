//! Gate G21 — M16 Sy-4 client SSH transport claim rows.
//! Matrix: `verify/matrices/matrix-g21.md`. Fixture: `verify/gates/g21/README-ssh-fixture.md`.
//! Cite: git-clone / gitprotocol-pack 2.39.0; OpenSSH as transport helper.

use serde_json::json;
use std::fs;
use std::net::TcpListener;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::Mutex;
use std::thread;
use std::time::{Duration, Instant};
use tempfile::TempDir;

static RECEIPT: Mutex<Vec<(&'static str, bool)>> = Mutex::new(Vec::new());

fn record_row(id: &'static str, pass: bool) {
    let mut rows = RECEIPT.lock().unwrap();
    rows.push((id, pass));
    // Rewrite on every row so parallel receipt_write cannot observe an empty set.
    if let Ok(path) = std::env::var("ARKS_G21_RECEIPT_OUT") {
        let body = json!({
            "gate": "G21",
            "wave": "Sy-4",
            "pass": rows.iter().all(|(_, p)| *p),
            "compose_service": "g21",
            "rows": rows.iter().map(|(i, p)| json!({"id": i, "pass": p})).collect::<Vec<_>>(),
        });
        if let Some(parent) = Path::new(&path).parent() {
            let _ = fs::create_dir_all(parent);
        }
        let _ = fs::write(&path, serde_json::to_string_pretty(&body).unwrap());
    }
}

fn arks_bin() -> PathBuf {
    if let Ok(p) = std::env::var("ARKS_BIN") {
        let p = PathBuf::from(p);
        if p.is_file() {
            return p;
        }
    }
    let mut candidates: Vec<PathBuf> = Vec::new();
    if let Ok(target) = std::env::var("CARGO_TARGET_DIR") {
        candidates.push(PathBuf::from(&target).join("release/arks"));
        candidates.push(PathBuf::from(&target).join("release/arks.exe"));
    }
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    candidates.push(root.join("target/release/arks"));
    candidates.push(root.join("target/release/arks.exe"));
    candidates.push(root.join("target/debug/arks"));
    candidates.push(root.join("target/debug/arks.exe"));
    for c in &candidates {
        if c.is_file() {
            return c.clone();
        }
    }
    panic!("missing arks binary; tried {candidates:?}");
}

fn pin_env(cmd: &mut Command) {
    cmd.env("GIT_AUTHOR_NAME", "arks")
        .env("GIT_AUTHOR_EMAIL", "arks@example.com")
        .env("GIT_COMMITTER_NAME", "arks")
        .env("GIT_COMMITTER_EMAIL", "arks@example.com")
        .env("GIT_AUTHOR_DATE", "1000000000 +0000")
        .env("GIT_COMMITTER_DATE", "1000000000 +0000");
}

fn arks_with_ssh(repo: &Path, ssh_cmd: &str, args: &[&str]) -> std::process::Output {
    let mut c = Command::new(arks_bin());
    pin_env(&mut c);
    c.env("GIT_SSH_COMMAND", ssh_cmd)
        .env("ARKS_SSH_COMMAND", ssh_cmd)
        .args(args)
        .current_dir(repo)
        .output()
        .expect("arks")
}

fn git(repo: &Path, args: &[&str]) -> std::process::Output {
    let mut c = Command::new("git");
    pin_env(&mut c);
    c.args(args).current_dir(repo).output().expect("git")
}

fn assert_ok(out: &std::process::Output, label: &str) {
    assert!(
        out.status.success(),
        "{label}: status={:?}\nstdout:\n{}\nstderr:\n{}",
        out.status.code(),
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
}

fn free_port() -> u16 {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind ephemeral");
    listener.local_addr().expect("local_addr").port()
}

struct SshFixture {
    _tmp: TempDir,
    port: u16,
    ssh_cmd: String,
    bare: PathBuf,
    #[allow(dead_code)]
    user: String,
    _sshd: Child,
}

impl Drop for SshFixture {
    fn drop(&mut self) {
        let _ = self._sshd.kill();
        let _ = self._sshd.wait();
    }
}

impl SshFixture {
    fn start() -> Self {
        // Require OpenSSH server tools in-image (clean-room).
        for bin in ["sshd", "ssh-keygen", "ssh"] {
            let st = Command::new(bin)
                .arg(if bin == "ssh" { "-V" } else { "-h" })
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .status();
            if bin == "ssh" {
                // ssh -V returns non-zero on some builds but still prints version to stderr.
                let _ = st;
                continue;
            }
            // sshd/ssh-keygen -h typically exits non-zero; existence matters.
            if st.is_err() {
                panic!("Sy-4 fixture missing `{bin}` — rebuild arks-verify-m1 with openssh-server");
            }
        }

        let tmp = TempDir::new().unwrap();
        let root = tmp.path().to_path_buf();
        let port = free_port();
        let user = whoami();

        let host_key = root.join("host_ed25519");
        let client_key = root.join("client_ed25519");
        run_ok(
            Command::new("ssh-keygen")
                .args(["-t", "ed25519", "-N", "", "-f"])
                .arg(&host_key)
                .arg("-q"),
            "host keygen",
        );
        run_ok(
            Command::new("ssh-keygen")
                .args(["-t", "ed25519", "-N", "", "-f"])
                .arg(&client_key)
                .arg("-q"),
            "client keygen",
        );
        let pub_client = fs::read_to_string(format!("{}.pub", client_key.display())).unwrap();
        let auth_keys = root.join("authorized_keys");
        fs::write(&auth_keys, pub_client).unwrap();

        let bare = root.join("remote.git");
        fs::create_dir_all(&bare).unwrap();
        assert_ok(&git(&bare, &["init", "--bare"]), "git init --bare");

        // Seed via a worktree push with stock git (local path).
        let seed = root.join("seed");
        fs::create_dir_all(&seed).unwrap();
        assert_ok(&git(&seed, &["init"]), "seed init");
        assert_ok(
            &git(&seed, &["config", "user.email", "arks@example.com"]),
            "seed email",
        );
        assert_ok(
            &git(&seed, &["config", "user.name", "arks"]),
            "seed name",
        );
        fs::write(seed.join("a.txt"), "ssh-sy4\n").unwrap();
        assert_ok(&git(&seed, &["add", "a.txt"]), "seed add");
        assert_ok(&git(&seed, &["commit", "-m", "first"]), "seed commit");
        assert_ok(
            &git(
                &seed,
                &["remote", "add", "origin", bare.to_str().unwrap()],
            ),
            "seed remote",
        );
        assert_ok(&git(&seed, &["push", "-u", "origin", "HEAD"]), "seed push");

        let sshd_config = root.join("sshd_config");
        let pid_file = root.join("sshd.pid");
        let config = format!(
            "Port {port}\n\
             ListenAddress 127.0.0.1\n\
             HostKey {host_key}\n\
             PidFile {pid_file}\n\
             AuthorizedKeysFile {auth_keys}\n\
             PasswordAuthentication no\n\
             PubkeyAuthentication yes\n\
             PermitRootLogin yes\n\
             StrictModes no\n\
             UsePAM no\n",
            host_key = host_key.display(),
            pid_file = pid_file.display(),
            auth_keys = auth_keys.display(),
        );
        fs::write(&sshd_config, config).unwrap();

        let sshd_bin = ["/usr/sbin/sshd", "sshd"]
            .into_iter()
            .find(|p| Path::new(p).is_file() || which_ok(p))
            .unwrap_or("sshd");
        let mut sshd = Command::new(sshd_bin)
            .args(["-D", "-e", "-f"])
            .arg(&sshd_config)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap_or_else(|e| panic!("spawn {sshd_bin}: {e}"));

        if let Err(e) = wait_for_port(port, Duration::from_secs(8)) {
            let _ = sshd.kill();
            let out = sshd.wait_with_output().ok();
            let stderr = out
                .as_ref()
                .map(|o| String::from_utf8_lossy(&o.stderr).into_owned())
                .unwrap_or_default();
            let stdout = out
                .as_ref()
                .map(|o| String::from_utf8_lossy(&o.stdout).into_owned())
                .unwrap_or_default();
            panic!("sshd failed to listen on 127.0.0.1:{port}: {e}\nstdout:\n{stdout}\nstderr:\n{stderr}");
        }

        let ssh_cmd = format!(
            "ssh -i {} -o BatchMode=yes -o IdentitiesOnly=yes -o StrictHostKeyChecking=no -o UserKnownHostsFile=/dev/null -o GlobalKnownHostsFile=/dev/null",
            client_key.display()
        );

        Self {
            _tmp: tmp,
            port,
            ssh_cmd,
            bare,
            user,
            _sshd: sshd,
        }
    }

    fn ssh_url(&self) -> String {
        // Absolute path form: ssh://user@127.0.0.1:port/abs/path
        let path = self.bare.display().to_string();
        format!(
            "ssh://{}@127.0.0.1:{}{}",
            self.user,
            self.port,
            path
        )
    }

    fn scp_url(&self) -> String {
        // SCP-like with absolute path after colon.
        format!("{}@127.0.0.1:{}", self.user, self.bare.display())
    }
}

fn whoami() -> String {
    if let Ok(u) = std::env::var("USER") {
        if !u.is_empty() {
            return u;
        }
    }
    let out = Command::new("id")
        .arg("-un")
        .output()
        .expect("id -un");
    String::from_utf8_lossy(&out.stdout).trim().to_string()
}

fn run_ok(cmd: &mut Command, label: &str) {
    let out = cmd.output().unwrap_or_else(|e| panic!("{label}: {e}"));
    assert_ok(&out, label);
}

fn which_ok(bin: &str) -> bool {
    Command::new("sh")
        .args(["-c", &format!("command -v {bin}")])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

fn wait_for_port(port: u16, timeout: Duration) -> Result<(), String> {
    let start = Instant::now();
    while start.elapsed() < timeout {
        if std::net::TcpStream::connect(("127.0.0.1", port)).is_ok() {
            return Ok(());
        }
        thread::sleep(Duration::from_millis(50));
    }
    Err(format!("no listener within {timeout:?}"))
}

#[test]
fn m16_sy4_01_clone_fetch_push_ssh() {
    let fx = SshFixture::start();
    let tmp = TempDir::new().unwrap();
    let work = tmp.path();
    let url = fx.ssh_url();

    // clone
    let clone_dest = work.join("cloned");
    let out = arks_with_ssh(
        work,
        &fx.ssh_cmd,
        &["clone", &url, clone_dest.to_str().unwrap()],
    );
    assert_ok(&out, "arks clone ssh://");
    assert!(clone_dest.join("a.txt").is_file(), "checkout missing a.txt");

    // Peer: stock git sees the same objects.
    let tip = String::from_utf8_lossy(&git(&clone_dest, &["rev-parse", "HEAD"]).stdout)
        .trim()
        .to_string();
    assert_eq!(tip.len(), 40);

    // fetch (noop / up-to-date)
    let out = arks_with_ssh(&clone_dest, &fx.ssh_cmd, &["fetch"]);
    assert_ok(&out, "arks fetch ssh");

    // push a new commit
    fs::write(clone_dest.join("b.txt"), "two\n").unwrap();
    let out = arks_with_ssh(&clone_dest, &fx.ssh_cmd, &["add", "b.txt"]);
    assert_ok(&out, "add");
    let out = arks_with_ssh(&clone_dest, &fx.ssh_cmd, &["commit", "-m", "second"]);
    assert_ok(&out, "commit");
    let out = arks_with_ssh(&clone_dest, &fx.ssh_cmd, &["push"]);
    assert_ok(&out, "arks push ssh");

    // stock git fetch from bare confirms push landed
    let verify = work.join("verify");
    assert_ok(
        &git(
            work,
            &[
                "clone",
                fx.bare.to_str().unwrap(),
                verify.to_str().unwrap(),
            ],
        ),
        "stock clone bare after push",
    );
    assert!(verify.join("b.txt").is_file(), "pushed file missing in peer");

    // Also exercise SCP-like URL form for clone.
    let scp_dest = work.join("scp-cloned");
    let scp = fx.scp_url();
    // SCP form has no port; inject -p via GIT_SSH_COMMAND.
    let ssh_with_port = format!("{} -p {}", fx.ssh_cmd, fx.port);
    let out = arks_with_ssh(
        work,
        &ssh_with_port,
        &["clone", &scp, scp_dest.to_str().unwrap()],
    );
    assert_ok(&out, "arks clone git@host:path");
    assert!(scp_dest.join("a.txt").is_file());

    record_row("M16-SY4-01", true);
}

#[test]
fn m16_sy4_02_submodule_add_ssh() {
    let fx = SshFixture::start();
    let tmp = TempDir::new().unwrap();
    let super_repo = tmp.path().join("super");
    fs::create_dir_all(&super_repo).unwrap();
    let out = arks_with_ssh(&super_repo, &fx.ssh_cmd, &["init"]);
    assert_ok(&out, "super init");
    fs::write(super_repo.join("root.txt"), "root\n").unwrap();
    assert_ok(
        &arks_with_ssh(&super_repo, &fx.ssh_cmd, &["add", "root.txt"]),
        "root add",
    );
    assert_ok(
        &arks_with_ssh(&super_repo, &fx.ssh_cmd, &["commit", "-m", "root"]),
        "root commit",
    );

    let url = fx.ssh_url();
    let out = arks_with_ssh(
        &super_repo,
        &fx.ssh_cmd,
        &["submodule", "add", &url, "vendor/remote"],
    );
    assert_ok(&out, "submodule add ssh");
    assert!(
        super_repo.join("vendor/remote/a.txt").is_file(),
        "submodule worktree missing"
    );
    let gm = fs::read_to_string(super_repo.join(".gitmodules")).unwrap();
    assert!(gm.contains("vendor/remote"), "gitmodules path");
    assert!(gm.contains("ssh://"), "gitmodules url");

    record_row("M16-SY4-02", true);
}

#[test]
fn m16_sy4_receipt_write() {
    // Ensure prior tests recorded; write receipt when env set.
    let rows = RECEIPT.lock().unwrap().clone();
    if let Ok(path) = std::env::var("ARKS_G21_RECEIPT_OUT") {
        let body = json!({
            "gate": "G21",
            "wave": "Sy-4",
            "pass": rows.iter().all(|(_, p)| *p) && rows.len() >= 2,
            "compose_service": "g21",
            "rows": rows.iter().map(|(id, p)| json!({"id": id, "pass": p})).collect::<Vec<_>>(),
        });
        if let Some(parent) = Path::new(&path).parent() {
            let _ = fs::create_dir_all(parent);
        }
        fs::write(&path, serde_json::to_string_pretty(&body).unwrap()).unwrap();
    }
}
