//! Serve-config reload in a running `arks serve-http` (queue unit Q21; bar in
//! [[sot:root/arks-claimed-residual-gates]] §Q21, B8 substitute).
//!
//! Windows has no SIGHUP, so the server watches the serve-config file mtime; the
//! `cfg(windows)` test is the receipt the `windows-2025` CI job runs. The unix twin
//! drives the same scenario through SIGHUP. Both hold a connection on the old
//! listen address, rewrite `bind=`, and require the new address to answer while the
//! held and a fresh old connection still complete (B7 dual-bind drain), and that the
//! reload does not re-trigger on its own config write.

use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::Path;
use std::process::{Child, Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

fn arks_bin() -> &'static str {
    env!("CARGO_BIN_EXE_arks")
}

fn free_port() -> u16 {
    TcpListener::bind("127.0.0.1:0")
        .unwrap()
        .local_addr()
        .unwrap()
        .port()
}

fn connect_within(port: u16, limit: Duration) -> TcpStream {
    let deadline = Instant::now() + limit;
    loop {
        match TcpStream::connect(("127.0.0.1", port)) {
            Ok(s) => return s,
            Err(e) if Instant::now() >= deadline => panic!("port {port} never answered: {e}"),
            Err(_) => thread::sleep(Duration::from_millis(50)),
        }
    }
}

fn http_response(mut s: TcpStream) -> String {
    s.set_read_timeout(Some(Duration::from_secs(10))).unwrap();
    let mut data = Vec::new();
    let _ = s.read_to_end(&mut data);
    String::from_utf8_lossy(&data).into_owned()
}

fn read(path: &Path) -> String {
    std::fs::read_to_string(path).unwrap_or_default()
}

struct Serve(Child);

impl Drop for Serve {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

/// Runs the reload scenario. `trigger` fires after the config rewrite (SIGHUP on
/// unix, nothing on Windows); `change_line` is the server's one-per-reload stderr
/// line and `signal_value` the `runtime_watch_signal` it records.
fn reload_scenario(trigger: impl FnOnce(&Child), change_line: &str, signal_value: &str) {
    let dir = tempfile::tempdir().unwrap();
    let bare = dir.path().join("repo.git");
    let init = Command::new(arks_bin())
        .args(["init", "-q", "--bare", bare.to_str().unwrap()])
        .env("ARKS_PERSONALITY", "git")
        .output()
        .unwrap();
    assert!(init.status.success(), "init: {init:?}");

    let config = dir.path().join("serve.conf");
    let err_path = dir.path().join("serve.err");
    let (old, new) = (free_port(), free_port());
    std::fs::write(&config, format!("bind=127.0.0.1:{old}\n")).unwrap();

    let child = Command::new(arks_bin())
        .args(["serve-http", "--listen", &format!("127.0.0.1:{old}")])
        .arg(&bare)
        .env("ARKS_SERVE_CONFIG", &config)
        .env("HOME", dir.path())
        .env("USERPROFILE", dir.path())
        .stdout(Stdio::null())
        .stderr(std::fs::File::create(&err_path).unwrap())
        .spawn()
        .unwrap();
    let serve = Serve(child);

    let mut held = connect_within(old, Duration::from_secs(20));
    held.write_all(b"GET /info/refs HTTP/1.1\r\nHost: held\r\n")
        .unwrap();

    // Leave a clear mtime step after the server took its baseline.
    thread::sleep(Duration::from_millis(1100));
    std::fs::write(&config, format!("bind=127.0.0.1:{new}\n")).unwrap();
    trigger(&serve.0);

    let mut fresh_new = connect_within(new, Duration::from_secs(20));
    let mut fresh_old = TcpStream::connect(("127.0.0.1", old)).expect("old listener drains");
    let request = b"GET /info/refs HTTP/1.1\r\nHost: q21\r\nConnection: close\r\n\r\n";
    fresh_old.write_all(request).unwrap();
    fresh_new.write_all(request).unwrap();
    held.write_all(b"Connection: close\r\n\r\n").unwrap();
    for (name, sock) in [
        ("held", held),
        ("fresh_old", fresh_old),
        ("fresh_new", fresh_new),
    ] {
        let resp = http_response(sock);
        assert!(
            resp.starts_with("HTTP/1.1 200 "),
            "{name}: {:?}\nstderr:\n{}",
            &resp[..resp.len().min(120)],
            read(&err_path)
        );
    }

    // The reload rewrites the config itself; that write must not trigger again.
    thread::sleep(Duration::from_millis(2500));
    let err = read(&err_path);
    assert_eq!(err.matches(change_line).count(), 1, "stderr:\n{err}");
    assert!(err.contains("live rebind dual-bind"), "stderr:\n{err}");
    let conf = read(&config);
    assert!(
        conf.contains(&format!("runtime_watch_signal={signal_value}")),
        "config:\n{conf}"
    );
    drop(serve);
}

#[cfg(windows)]
#[test]
fn windows_serve_config_mtime_reload_rebinds_without_signal() {
    reload_scenario(
        |_| {},
        "serve-config file-mtime change → apply/reload",
        "file-mtime (Windows; SIGHUP N/A)",
    );
}

#[cfg(unix)]
#[test]
fn unix_serve_config_sighup_reload_rebinds() {
    reload_scenario(
        |child| {
            // SAFETY: plain kill(2) on our own child pid.
            let rc = unsafe { libc::kill(child.id() as libc::pid_t, libc::SIGHUP) };
            assert_eq!(rc, 0, "kill SIGHUP");
        },
        "SIGHUP → serve config apply/reload",
        "SIGHUP (unix)",
    );
}
