//! `arks serve ssh-smoke` diagnostic (queue unit Q21; bar in
//! [[sot:root/arks-claimed-residual-gates]] §Q21). Host-advisory twin of the Docker
//! receipt: banner check against a running `arks serve` SSH listener, the
//! `--git-pack` self-listener path, and a non-zero exit when nothing listens.

use std::net::{TcpListener, TcpStream};
use std::process::{Child, Command, Output, Stdio};
use std::thread;
use std::time::{Duration, Instant};

fn arks() -> Command {
    let mut c = Command::new(env!("CARGO_BIN_EXE_arks"));
    c.env("ARKS_PERSONALITY", "git");
    c
}

fn free_port() -> u16 {
    TcpListener::bind("127.0.0.1:0")
        .unwrap()
        .local_addr()
        .unwrap()
        .port()
}

fn stdout(o: &Output) -> String {
    String::from_utf8_lossy(&o.stdout).into_owned()
}

struct Serve(Child);

impl Drop for Serve {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

#[test]
fn ssh_smoke_reports_banner_of_running_serve() {
    let dir = tempfile::tempdir().unwrap();
    let bare = dir.path().join("repo.git");
    let init = arks()
        .args(["init", "-q", "--bare", bare.to_str().unwrap()])
        .output()
        .unwrap();
    assert!(init.status.success(), "init: {init:?}");
    let port = free_port();
    let listen = format!("127.0.0.1:{port}");
    let child = arks()
        .args(["serve", "--no-http", "--no-forge", "--ssh-listen", &listen])
        .arg(&bare)
        .env("HOME", dir.path())
        .env("USERPROFILE", dir.path())
        .env("ARKS_SERVE_CONFIG", dir.path().join("serve.conf"))
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();
    let _serve = Serve(child);
    let deadline = Instant::now() + Duration::from_secs(20);
    while TcpStream::connect(("127.0.0.1", port)).is_err() {
        assert!(Instant::now() < deadline, "ssh listener never opened");
        thread::sleep(Duration::from_millis(50));
    }

    let out = arks()
        .args(["serve", "ssh-smoke", "--listen", &listen])
        .env("ARKS_SERVE_CONFIG", dir.path().join("serve.conf"))
        .output()
        .unwrap();
    let text = stdout(&out);
    assert!(out.status.success(), "{out:?}");
    assert!(text.contains("ssh_banner=ok"), "{text}");
    assert!(text.contains("banner_preview=SSH-2.0-arks"), "{text}");
    assert!(!text.contains("Missing"), "{text}");
}

#[test]
fn ssh_smoke_git_pack_reads_kexinit() {
    let dir = tempfile::tempdir().unwrap();
    let out = arks()
        .args(["serve", "ssh-smoke", "--git-pack"])
        .current_dir(dir.path())
        .env("HOME", dir.path())
        .env("USERPROFILE", dir.path())
        .output()
        .unwrap();
    let text = stdout(&out);
    assert!(out.status.success(), "{out:?}");
    assert!(text.contains("server_kexinit=ok"), "{text}");
    assert!(text.contains("git_over_ssh_smoke=ok"), "{text}");
}

#[test]
fn ssh_smoke_fails_when_nothing_listens() {
    let port = free_port();
    let out = arks()
        .args([
            "serve",
            "ssh-smoke",
            "--listen",
            &format!("127.0.0.1:{port}"),
        ])
        .output()
        .unwrap();
    assert!(!out.status.success(), "{out:?}");
    assert!(stdout(&out).contains("ssh_banner=none"), "{out:?}");
}
