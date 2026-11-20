//! `arks daemon` process behaviour with `git daemon` semantics: argv errors,
//! `--inetd`, `--listen`/`--port`, log destinations, `--informative-errors`,
//! `--access-hook`, `--detach` + `--pid-file`.
//!
//! Primary: <https://git-scm.com/docs/git-daemon>. Expected strings were taken
//! from observing stock `git daemon`; stock git is not needed to run these.

use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Output, Stdio};
use std::thread;
use std::time::{Duration, Instant};

fn arks() -> Command {
    let mut c = Command::new(env!("CARGO_BIN_EXE_arks"));
    c.env("GIT_CONFIG_NOSYSTEM", "1")
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .env("ARKS_PERSONALITY", "git");
    c
}

fn daemon(args: &[&str], stdin: &[u8]) -> Output {
    let mut child = arks()
        .arg("daemon")
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn arks daemon");
    child
        .stdin
        .take()
        .unwrap()
        .write_all(stdin)
        .expect("write stdin");
    child.wait_with_output().expect("wait")
}

fn pkt(payload: &[u8]) -> Vec<u8> {
    let mut v = format!("{:04x}", payload.len() + 4).into_bytes();
    v.extend_from_slice(payload);
    v
}

/// Bare repository with one commit, created by `arks` itself.
fn bare_repo(root: &Path, name: &str, exported: bool) -> PathBuf {
    let work = root.join(format!("{name}-work"));
    std::fs::create_dir_all(&work).unwrap();
    let run = |args: &[&str]| {
        let out = arks().args(args).current_dir(&work).output().unwrap();
        assert!(out.status.success(), "{args:?}: {out:?}");
    };
    run(&["init", "-q", "-b", "main"]);
    std::fs::write(work.join("f"), b"hi\n").unwrap();
    run(&["add", "f"]);
    run(&[
        "-c",
        "user.name=t",
        "-c",
        "user.email=t@t",
        "commit",
        "-q",
        "-m",
        "init",
    ]);
    let bare = root.join(name);
    let out = arks()
        .args(["clone", "-q", "--bare"])
        .arg(&work)
        .arg(&bare)
        .output()
        .unwrap();
    assert!(out.status.success(), "{out:?}");
    if exported {
        std::fs::write(bare.join("git-daemon-export-ok"), b"").unwrap();
    }
    bare
}

fn err_pkt(text: &str) -> String {
    format!("{:04x}{text}", text.len() + 4)
}

fn free_port() -> u16 {
    TcpListener::bind("127.0.0.1:0")
        .unwrap()
        .local_addr()
        .unwrap()
        .port()
}

struct Running(Child);
impl Drop for Running {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

fn start(args: &[String], port: u16, stderr: &Path) -> Running {
    let child = arks()
        .arg("daemon")
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(std::fs::File::create(stderr).unwrap())
        .spawn()
        .unwrap();
    wait_port(port);
    Running(child)
}

fn wait_port(port: u16) {
    let deadline = Instant::now() + Duration::from_secs(10);
    while TcpStream::connect(("127.0.0.1", port)).is_err() {
        assert!(Instant::now() < deadline, "daemon did not listen on {port}");
        thread::sleep(Duration::from_millis(30));
    }
}

fn ask(port: u16, payload: &[u8]) -> String {
    let mut s = TcpStream::connect(("127.0.0.1", port)).unwrap();
    s.set_read_timeout(Some(Duration::from_secs(5))).unwrap();
    s.write_all(&pkt(payload)).unwrap();
    let mut buf = vec![0u8; 4096];
    let n = s.read(&mut buf).unwrap_or(0);
    String::from_utf8_lossy(&buf[..n]).to_string()
}

#[test]
fn argv_errors_match_git_daemon() {
    for args in [
        &["--port", "9418"][..],
        &["--bogus"],
        &["--port=abc"],
        &["-h"],
    ] {
        let out = daemon(args, b"");
        assert_eq!(out.status.code(), Some(129), "{args:?}");
        assert!(String::from_utf8_lossy(&out.stderr).contains("usage: arks daemon"));
    }
    let cases: [(&[&str], &str); 5] = [
        (&["--enable=bogus"], "fatal: No such service bogus"),
        (
            &["--log-destination=foo"],
            "fatal: unknown log destination 'foo'",
        ),
        (
            &["--strict-paths"],
            "fatal: option --strict-paths requires '<directory>' arguments",
        ),
        (&["--group=root"], "fatal: --group supplied without --user"),
        (
            &["--base-path=/nonexistent-arks-base"],
            "fatal: base-path '/nonexistent-arks-base' does not exist or is not a directory",
        ),
    ];
    for (args, want) in cases {
        let out = daemon(args, b"");
        assert_eq!(out.status.code(), Some(128), "{args:?}");
        assert_eq!(String::from_utf8_lossy(&out.stderr).trim_end(), want);
    }
}

// --inetd implies --syslog; arks has no Windows Event Log sink yet (A5 gap).
#[cfg(unix)]
#[test]
fn inetd_conflicts_log_to_the_chosen_destination() {
    // Default destination under --inetd is syslog: nothing on stderr, exit 1.
    let out = daemon(&["--inetd", "--port=1"], b"");
    assert_eq!(out.status.code(), Some(1));
    assert!(out.stderr.is_empty());
    let out = daemon(&["--inetd", "--detach"], b"");
    assert_eq!(out.status.code(), Some(1));
    let out = daemon(
        &["--inetd", "--log-destination=stderr", "--listen=127.0.0.1"],
        b"",
    );
    assert_eq!(out.status.code(), Some(128));
    assert_eq!(
        String::from_utf8_lossy(&out.stderr).trim_end(),
        "fatal: --listen= and --port= are incompatible with --inetd"
    );
}

// --inetd implies --syslog; arks has no Windows Event Log sink yet (A5 gap).
#[cfg(unix)]
#[test]
fn inetd_serves_one_connection_on_stdio() {
    let root = tempfile::TempDir::new().unwrap();
    bare_repo(root.path(), "pub.git", true);
    bare_repo(root.path(), "hidden.git", false);
    let base = format!("--base-path={}", root.path().display());
    let mut req = pkt(b"git-upload-pack /pub.git\0host=localhost\0");
    req.extend_from_slice(b"0000");
    let out = daemon(&["--inetd", &base], &req);
    assert_eq!(out.status.code(), Some(0), "{out:?}");
    let text = String::from_utf8_lossy(&out.stdout);
    assert!(text.contains("refs/heads/main"), "{text}");
    let req = pkt(b"git-upload-pack /hidden.git\0");
    let out = daemon(&["--inetd", "--informative-errors", &base], &req);
    assert_eq!(out.status.code(), Some(255));
    assert_eq!(
        String::from_utf8_lossy(&out.stdout),
        err_pkt("ERR repository not exported: /hidden.git")
    );
    let out = daemon(
        &["--inetd", "--log-destination=stderr", "--verbose", &base],
        &req,
    );
    assert_eq!(out.status.code(), Some(255));
    let err = String::from_utf8_lossy(&out.stderr);
    assert!(
        err.contains("] Request upload-pack for '/hidden.git'"),
        "{err}"
    );
    assert!(
        err.contains("/hidden.git': repository not exported."),
        "{err}"
    );
}

#[test]
fn host_port_listen_value_fails_like_git() {
    let out = daemon(&["--listen=127.0.0.1:19999"], b"");
    assert_eq!(out.status.code(), Some(128));
    let err = String::from_utf8_lossy(&out.stderr);
    assert!(
        err.contains("getaddrinfo() for 127.0.0.1:19999 failed"),
        "{err}"
    );
    assert!(
        err.contains("unable to allocate any listen sockets for host 127.0.0.1:19999 on port 9418"),
        "{err}"
    );
    assert!(
        err.trim_end()
            .ends_with("fatal: unable to allocate any listen sockets on port 9418"),
        "{err}"
    );
}

#[test]
fn tcp_errors_hook_and_log_destinations() {
    let root = tempfile::TempDir::new().unwrap();
    bare_repo(root.path(), "pub.git", true);
    bare_repo(root.path(), "hidden.git", false);
    let port = free_port();
    let args: Vec<String> = vec![
        "--listen=127.0.0.1".into(),
        format!("--port={port}"),
        format!("--base-path={}", root.path().display()),
        "--log-destination=none".into(),
        "--verbose".into(),
    ];
    let quiet = root.path().join("quiet.err");
    let d = start(&args, port, &quiet);
    assert_eq!(
        ask(port, b"git-upload-pack /hidden.git\0"),
        err_pkt("ERR access denied or repository not exported: /hidden.git")
    );
    assert!(ask(port, b"git-upload-pack /pub.git\0host=localhost\0").contains("refs/heads/main"));
    drop(d);
    let logged = std::fs::read_to_string(&quiet).unwrap();
    assert!(
        logged.is_empty(),
        "none destination must be silent: {logged}"
    );

    let port = free_port();
    let mut args: Vec<String> = vec![
        "--listen=127.0.0.1".into(),
        format!("--port={port}"),
        format!("--base-path={}", root.path().display()),
        "--informative-errors".into(),
    ];
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let hook = root.path().join("hook.sh");
        std::fs::write(
            &hook,
            "#!/bin/sh\ncase \"$2\" in *pub.git) echo \"no for $REMOTE_ADDR\"; exit 1;; esac\nexit 0\n",
        )
        .unwrap();
        std::fs::set_permissions(&hook, std::fs::Permissions::from_mode(0o755)).unwrap();
        args.push(format!("--access-hook={}", hook.display()));
    }
    args.push("--verbose".into());
    let loud = root.path().join("loud.err");
    let d = start(&args, port, &loud);
    assert_eq!(
        ask(port, b"git-upload-pack /nope.git\0"),
        err_pkt("ERR no such repository: /nope.git")
    );
    assert_eq!(
        ask(port, b"git-receive-pack /pub.git\0"),
        err_pkt("ERR service not enabled: /pub.git")
    );
    #[cfg(unix)]
    assert_eq!(
        ask(port, b"git-upload-pack /pub.git\0"),
        err_pkt("ERR no for 127.0.0.1: /pub.git")
    );
    drop(d);
    let logged = std::fs::read_to_string(&loud).unwrap();
    assert!(logged.contains("] Ready to rumble\n"), "{logged}");
    assert!(logged.contains("] Connection from 127.0.0.1:"), "{logged}");
    assert!(
        logged.contains("'receive-pack': service not enabled for '"),
        "{logged}"
    );
}

#[cfg(unix)]
#[test]
fn detach_writes_pid_of_serving_process() {
    let root = tempfile::TempDir::new().unwrap();
    bare_repo(root.path(), "pub.git", true);
    let port = free_port();
    let pid_file = root.path().join("daemon.pid");
    let out = arks()
        .arg("daemon")
        .arg("--detach")
        .arg(format!("--pid-file={}", pid_file.display()))
        .arg("--listen=127.0.0.1")
        .arg(format!("--port={port}"))
        .arg(format!("--base-path={}", root.path().display()))
        .stdin(Stdio::null())
        .output()
        .unwrap();
    assert!(out.status.success(), "{out:?}");
    assert!(out.stderr.is_empty());
    wait_port(port);
    let pid: u32 = std::fs::read_to_string(&pid_file)
        .unwrap()
        .trim()
        .parse()
        .unwrap();
    let stat = std::fs::read_to_string(format!("/proc/{pid}/stat")).unwrap_or_default();
    assert!(ask(port, b"git-upload-pack /pub.git\0").contains("refs/heads/main"));
    let _ = Command::new("kill").arg(pid.to_string()).status();
    if !stat.is_empty() {
        // Field 6 (session id) equals the pid after setsid.
        let after_comm = stat.rsplit_once(") ").unwrap().1;
        let fields: Vec<&str> = after_comm.split(' ').collect();
        assert_eq!(fields[3], pid.to_string(), "session leader: {stat}");
    }
}

/// Real uid from `/proc/<pid>/status` (Linux).
#[cfg(target_os = "linux")]
fn proc_uid(pid: &str) -> Option<String> {
    let status = std::fs::read_to_string(format!("/proc/{pid}/status")).ok()?;
    status
        .lines()
        .find_map(|l| l.strip_prefix("Uid:"))
        .and_then(|v| v.split_whitespace().next().map(str::to_string))
}

#[cfg(target_os = "linux")]
#[test]
fn user_drops_privileges_after_binding_when_root() {
    if proc_uid("self").as_deref() != Some("0") {
        return; // privilege drop is only meaningful as root
    }
    let out = daemon(&["--user=no-such-user-arks"], b"");
    assert_eq!(out.status.code(), Some(128));
    assert_eq!(
        String::from_utf8_lossy(&out.stderr).trim_end(),
        "fatal: user not found - no-such-user-arks"
    );
    let root = tempfile::TempDir::new().unwrap();
    let port = free_port();
    let args: Vec<String> = vec![
        "--listen=127.0.0.1".into(),
        format!("--port={port}"),
        "--user=nobody".into(),
    ];
    let d = start(&args, port, &root.path().join("user.err"));
    let pid = d.0.id().to_string();
    let deadline = Instant::now() + Duration::from_secs(5);
    while proc_uid(&pid).as_deref() == Some("0") && Instant::now() < deadline {
        thread::sleep(Duration::from_millis(20));
    }
    assert_ne!(proc_uid(&pid).as_deref(), Some("0"), "still root");
}
