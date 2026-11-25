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

/// A port that is free now, for a daemon that binds it later.
///
/// Ports the OS hands out on its own (a bind to port 0, the source port of
/// every outgoing connect) come from its ephemeral range (Linux 32768-60999,
/// macOS and Windows 49152-65535). A port probed there and released can be
/// handed to any other process on the host before the daemon binds it, and
/// the test then talks to that process. Probe below those ranges instead,
/// from an offset that differs per test process.
fn free_port() -> u16 {
    use std::sync::atomic::{AtomicU32, Ordering};
    const LOW: u32 = 20_000;
    const SPAN: u32 = 12_000;
    static NEXT: AtomicU32 = AtomicU32::new(0);
    let seed = std::process::id().wrapping_mul(7_919);
    for _ in 0..SPAN {
        let n = NEXT.fetch_add(1, Ordering::Relaxed);
        let port = (LOW + seed.wrapping_add(n) % SPAN) as u16;
        if TcpListener::bind(("127.0.0.1", port)).is_ok() {
            return port;
        }
    }
    panic!("no free port in {LOW}..{}", LOW + SPAN);
}

/// Whether process `pid` holds a listening TCP socket on `port`, from
/// `/proc`. `None` where that can not be read.
#[cfg(target_os = "linux")]
fn owns_listener(pid: u32, port: u16) -> Option<bool> {
    let mut sockets = Vec::new();
    for table in ["/proc/net/tcp", "/proc/net/tcp6"] {
        let Ok(text) = std::fs::read_to_string(table) else {
            continue;
        };
        for line in text.lines().skip(1) {
            // sl local_address rem_address st ... inode (field 9); st 0A = LISTEN.
            let f: Vec<&str> = line.split_whitespace().collect();
            let local = f
                .get(1)
                .and_then(|a| a.rsplit_once(':'))
                .and_then(|(_, p)| u16::from_str_radix(p, 16).ok());
            if local == Some(port) && f.get(3) == Some(&"0A") {
                if let Some(inode) = f.get(9) {
                    sockets.push(format!("socket:[{inode}]"));
                }
            }
        }
    }
    let fds = std::fs::read_dir(format!("/proc/{pid}/fd")).ok()?;
    Some(
        fds.filter_map(Result::ok)
            .filter_map(|e| std::fs::read_link(e.path()).ok())
            .any(|link| sockets.iter().any(|s| link.as_os_str() == s.as_str())),
    )
}

#[cfg(not(target_os = "linux"))]
fn owns_listener(_pid: u32, _port: u16) -> Option<bool> {
    None
}

/// Whether `pid` still runs. A process that exited without being reaped (no
/// reaping init in a container) is a zombie and counts as gone.
#[cfg(target_os = "linux")]
fn alive(pid: u32) -> bool {
    std::fs::read_to_string(format!("/proc/{pid}/stat"))
        .ok()
        .and_then(|s| {
            s.rsplit_once(") ")
                .map(|(_, rest)| !rest.starts_with(['Z', 'X']))
        })
        .unwrap_or(false)
}

#[cfg(all(unix, not(target_os = "linux")))]
fn alive(pid: u32) -> bool {
    // SAFETY: signal 0 only checks that the process exists.
    unsafe { libc::kill(pid as libc::pid_t, 0) == 0 }
}

/// Wait until daemon process `pid` itself listens on `port`. A connect alone
/// is not enough: it succeeds against whatever process holds the port.
/// `false` when the daemon went away first, which is how it reports that it
/// lost the port to another process (`unable to allocate any listen sockets`).
fn serving(pid: u32, port: u16, mut running: impl FnMut() -> bool) -> bool {
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        if !running() {
            return false;
        }
        let ready = match owns_listener(pid, port) {
            Some(owned) => owned,
            // No view of socket owners here: any listener counts.
            None => TcpStream::connect(("127.0.0.1", port)).is_ok(),
        };
        if ready {
            return true;
        }
        assert!(
            Instant::now() < deadline,
            "daemon {pid} did not listen on {port}"
        );
        thread::sleep(Duration::from_millis(30));
    }
}

/// Attempts at starting a daemon whose probed port was taken before it bound.
const START_ATTEMPTS: usize = 5;

struct Running(Child);
impl Drop for Running {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

/// `arks daemon <args> --port=<free port>`, returned once that very process
/// listens; started again on another port when it lost the probed one.
fn start(args: &[String], stderr: &Path) -> (Running, u16) {
    for _ in 0..START_ATTEMPTS {
        let port = free_port();
        let mut child = arks()
            .arg("daemon")
            .args(args)
            .arg(format!("--port={port}"))
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(std::fs::File::create(stderr).unwrap())
            .spawn()
            .unwrap();
        let pid = child.id();
        if serving(pid, port, || matches!(child.try_wait(), Ok(None))) {
            return (Running(child), port);
        }
    }
    panic!("daemon {args:?} lost its port {START_ATTEMPTS} times");
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

// --inetd implies --syslog on unix; the windows_* tests cover the stderr default there.
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

// --inetd implies --syslog on unix; the windows_* tests cover the stderr default there.
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
    let args: Vec<String> = vec![
        "--listen=127.0.0.1".into(),
        format!("--base-path={}", root.path().display()),
        "--log-destination=none".into(),
        "--verbose".into(),
    ];
    let quiet = root.path().join("quiet.err");
    let (d, port) = start(&args, &quiet);
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

    let mut args: Vec<String> = vec![
        "--listen=127.0.0.1".into(),
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
    let (d, port) = start(&args, &loud);
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

/// A detached daemon, named by its pid file; terminated on drop.
#[cfg(unix)]
struct Detached(u32);

#[cfg(unix)]
impl Drop for Detached {
    fn drop(&mut self) {
        // SAFETY: plain signal to a process this test started.
        unsafe {
            libc::kill(self.0 as libc::pid_t, libc::SIGTERM);
        }
    }
}

/// `arks daemon --detach --pid-file=<pid_file> <args> --port=<free port>`.
///
/// As with git, the command forks and its parent exits 0 before the child
/// binds; the child writes the pid file and then binds. Exit status and a
/// connect therefore say nothing about whether that child got the port, so
/// readiness is the process named by the pid file listening on it. A child
/// that lost the probed port exits, and the daemon is started again.
#[cfg(unix)]
fn start_detached(args: &[String], pid_file: &Path) -> (Detached, u16) {
    for _ in 0..START_ATTEMPTS {
        let _ = std::fs::remove_file(pid_file);
        let port = free_port();
        let out = arks()
            .arg("daemon")
            .arg("--detach")
            .arg(format!("--pid-file={}", pid_file.display()))
            .args(args)
            .arg(format!("--port={port}"))
            .stdin(Stdio::null())
            .output()
            .unwrap();
        assert!(out.status.success(), "{out:?}");
        assert!(out.stderr.is_empty(), "{out:?}");
        let deadline = Instant::now() + Duration::from_secs(10);
        let pid: u32 = loop {
            let text = std::fs::read_to_string(pid_file).unwrap_or_default();
            // Complete once the line is terminated.
            if let Some(pid) = text.strip_suffix('\n').and_then(|t| t.parse().ok()) {
                break pid;
            }
            assert!(Instant::now() < deadline, "no pid in {pid_file:?}");
            thread::sleep(Duration::from_millis(20));
        };
        let daemon = Detached(pid);
        if serving(pid, port, || alive(pid)) {
            return (daemon, port);
        }
    }
    panic!("detached daemon lost its port {START_ATTEMPTS} times");
}

#[cfg(unix)]
#[test]
fn detach_writes_pid_of_serving_process() {
    let root = tempfile::TempDir::new().unwrap();
    bare_repo(root.path(), "pub.git", true);
    let pid_file = root.path().join("daemon.pid");
    let args: Vec<String> = vec![
        "--listen=127.0.0.1".into(),
        format!("--base-path={}", root.path().display()),
    ];
    let (daemon, port) = start_detached(&args, &pid_file);
    let pid = daemon.0;
    let stat = std::fs::read_to_string(format!("/proc/{pid}/stat")).unwrap_or_default();
    let reply = ask(port, b"git-upload-pack /pub.git\0");
    assert!(reply.contains("refs/heads/main"), "{reply:?}");
    drop(daemon);
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
    let args: Vec<String> = vec!["--listen=127.0.0.1".into(), "--user=nobody".into()];
    let (d, _port) = start(&args, &root.path().join("user.err"));
    let pid = d.0.id().to_string();
    let deadline = Instant::now() + Duration::from_secs(5);
    while proc_uid(&pid).as_deref() == Some("0") && Instant::now() < deadline {
        thread::sleep(Duration::from_millis(20));
    }
    let uid = proc_uid(&pid);
    assert!(
        uid.as_deref().is_some_and(|u| u != "0"),
        "still root or gone: {uid:?}"
    );
}

// Windows (matrix SoT §Windows): `--inetd` works with stderr as the implied log
// destination; syslog, `--detach` and `--user` are refused with exit 128.
#[cfg(windows)]
#[test]
fn windows_refuses_syslog_detach_and_user() {
    let cases: [(&[&str], &str); 7] = [
        (&["--syslog"], "fatal: syslog is not supported on Windows"),
        (
            &["--inetd", "--log-destination=syslog"],
            "fatal: syslog is not supported on Windows",
        ),
        (
            &["--detach"],
            "fatal: --detach not supported on this platform",
        ),
        (
            &["--user=nobody"],
            "fatal: --user not supported on this platform",
        ),
        (
            &["--user=nobody", "--group=nogroup"],
            "fatal: --user not supported on this platform",
        ),
        (
            &["--inetd", "--detach"],
            "fatal: --detach, --user and --group are incompatible with --inetd",
        ),
        (
            &["--inetd", "--port=1"],
            "fatal: --listen= and --port= are incompatible with --inetd",
        ),
    ];
    for (args, want) in cases {
        let out = daemon(args, b"");
        assert_eq!(out.status.code(), Some(128), "{args:?}: {out:?}");
        assert_eq!(
            String::from_utf8_lossy(&out.stderr).trim_end(),
            want,
            "{args:?}"
        );
    }
}

/// One upload-pack request for `path` (plus `host=` when given), then a flush.
#[cfg(windows)]
fn upload_request(path: &str, host: Option<&str>) -> Vec<u8> {
    let mut payload = format!("git-upload-pack {path}\0");
    if let Some(h) = host {
        payload.push_str(&format!("host={h}\0"));
    }
    let mut req = pkt(payload.as_bytes());
    req.extend_from_slice(b"0000");
    req
}

/// `C:\x` → `c:/x` (or `c:\x` → `C:/x`): same directory, other spelling.
#[cfg(windows)]
fn flip_drive_spelling(path: &str) -> String {
    let mut rest = path.replace('\\', "/");
    let first = rest.remove(0);
    let first = if first.is_ascii_uppercase() {
        first.to_ascii_lowercase()
    } else {
        first.to_ascii_uppercase()
    };
    format!("{first}{rest}")
}

#[cfg(windows)]
#[test]
fn windows_inetd_serves_one_connection_on_stdio() {
    let root = tempfile::TempDir::new().unwrap();
    bare_repo(root.path(), "pub.git", true);
    bare_repo(root.path(), "hidden.git", false);
    let base_dir = root.path().display().to_string();
    assert_eq!(base_dir.as_bytes().get(1), Some(&b':'), "{base_dir}");
    let base = format!("--base-path={base_dir}");
    let out = daemon(
        &["--inetd", &base],
        &upload_request("/pub.git", Some("localhost")),
    );
    assert_eq!(out.status.code(), Some(0), "{out:?}");
    assert!(String::from_utf8_lossy(&out.stdout).contains("refs/heads/main"));
    // Refusal: exit 255, ERR line on stdout, log lines on stderr (implied destination).
    let req = pkt(b"git-upload-pack /hidden.git\0");
    let out = daemon(
        &["--inetd", "--informative-errors", "--verbose", &base],
        &req,
    );
    assert_eq!(out.status.code(), Some(255), "{out:?}");
    assert_eq!(
        String::from_utf8_lossy(&out.stdout),
        err_pkt("ERR repository not exported: /hidden.git")
    );
    let err = String::from_utf8_lossy(&out.stderr);
    assert!(
        err.contains("] Request upload-pack for '/hidden.git'"),
        "{err}"
    );
    assert!(
        err.contains("hidden.git': repository not exported."),
        "{err}"
    );
    // --export-all serves the unmarked repository.
    let out = daemon(
        &["--inetd", "--export-all", &base],
        &upload_request("/hidden.git", None),
    );
    assert_eq!(out.status.code(), Some(0), "{out:?}");
    assert!(String::from_utf8_lossy(&out.stdout).contains("refs/heads/main"));
}

#[cfg(windows)]
#[test]
fn windows_inetd_drive_letter_paths_stay_under_the_base() {
    let root = tempfile::TempDir::new().unwrap();
    let base_dir = root.path().join("base");
    std::fs::create_dir_all(&base_dir).unwrap();
    bare_repo(&base_dir, "pub.git", true);
    bare_repo(root.path(), "outside.git", true);
    let shown = base_dir.display().to_string();
    let base = format!("--base-path={shown}");
    // `..` with either separator, and dot-and-space components, are aliased.
    for path in [
        "/..\\outside.git",
        "/../outside.git",
        "/pub.git\\..\\..\\outside.git",
        "/.. \\outside.git",
    ] {
        let out = daemon(
            &["--inetd", "--log-destination=stderr", &base],
            &upload_request(path, None),
        );
        assert_eq!(out.status.code(), Some(255), "{path}: {out:?}");
        assert_eq!(
            String::from_utf8_lossy(&out.stdout),
            err_pkt(&format!(
                "ERR access denied or repository not exported: {path}"
            ))
        );
        let err = String::from_utf8_lossy(&out.stderr);
        assert!(err.contains(&format!("'{path}': aliased")), "{err}");
    }
    // Whitelist entry with the other drive-letter case and forward slashes.
    let flipped = flip_drive_spelling(&shown);
    let out = daemon(
        &["--inetd", &base, &flipped],
        &upload_request("/pub.git", None),
    );
    assert_eq!(out.status.code(), Some(0), "{out:?}");
    let other = format!("{shown}\\elsewhere");
    let out = daemon(
        &["--inetd", &base, &other],
        &upload_request("/pub.git", None),
    );
    assert_eq!(out.status.code(), Some(255), "{out:?}");
    // --interpolated-path with %H%D under a drive letter (host is lower-cased).
    let vhost = root.path().join("vhost").join("example.com");
    std::fs::create_dir_all(&vhost).unwrap();
    bare_repo(&vhost, "v.git", true);
    let template = format!("--interpolated-path={}\\vhost\\%H%D", root.path().display());
    let out = daemon(
        &["--inetd", &template],
        &upload_request("/v.git", Some("Example.COM")),
    );
    assert_eq!(out.status.code(), Some(0), "{out:?}");
    assert!(String::from_utf8_lossy(&out.stdout).contains("refs/heads/main"));
    // A client host can not name a drive through %H: `[c:]` expands to `c`.
    let out = daemon(
        &["--inetd", "--verbose", "--interpolated-path=%H%D"],
        &upload_request("/v.git", Some("[c:]")),
    );
    assert_eq!(out.status.code(), Some(255), "{out:?}");
    let err = String::from_utf8_lossy(&out.stderr);
    assert!(err.contains("Interpolated dir 'c/v.git'"), "{err}");
}
