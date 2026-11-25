//! `arks credential-manager` / `arks credential-wincred` on every OS: the
//! credential-helper protocol (`get` / `store` / `erase`) against the fake
//! store selected with `ARKS_CREDENTIAL_BACKEND=fake`, the honest refusal
//! when no OS store is reachable, and `credential.helper=<path-to-arks>
//! credential-manager` driven through `arks credential fill|approve|reject`.
//!
//! Primary: <https://git-scm.com/docs/gitcredentials>,
//! <https://git-scm.com/docs/git-credential>.

use std::io::Write;
use std::path::Path;
use std::process::{Command, Output, Stdio};

fn run(args: &[&str], stdin: &str, envs: &[(&str, &str)]) -> Output {
    let mut c = Command::new(env!("CARGO_BIN_EXE_arks"));
    c.args(args)
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .env("ARKS_PERSONALITY", "git")
        .env_remove("ARKS_CREDENTIAL_BACKEND")
        .env_remove("ARKS_CREDENTIAL_WALLET_DIR")
        .env_remove("ARKS_CREDENTIAL_FAKE_STORE")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    for (k, v) in envs {
        c.env(k, v);
    }
    let mut child = c.spawn().unwrap();
    child
        .stdin
        .take()
        .unwrap()
        .write_all(stdin.as_bytes())
        .unwrap();
    child.wait_with_output().unwrap()
}

fn fake(store: &Path) -> [(&'static str, String); 2] {
    [
        ("ARKS_CREDENTIAL_BACKEND", "fake".into()),
        ("ARKS_CREDENTIAL_FAKE_STORE", store.display().to_string()),
    ]
}

fn helper(name: &str, op: &str, input: &str, store: &Path) -> Output {
    let e = fake(store);
    let envs: Vec<(&str, &str)> = e.iter().map(|(k, v)| (*k, v.as_str())).collect();
    run(&[name, op], input, &envs)
}

const FULL: &str = "protocol=https\nhost=example.com\nusername=alice\npassword=s3cret\n\n";
const QUERY: &str = "protocol=https\nhost=example.com\n\n";

#[test]
fn get_store_erase_roundtrip() {
    for name in ["credential-manager", "credential-wincred"] {
        let t = tempfile::tempdir().unwrap();
        let s = t.path().join("store");
        let o = helper(name, "get", QUERY, &s);
        assert!(o.status.success(), "{o:?}");
        assert!(o.stdout.is_empty());
        let o = helper(name, "store", FULL, &s);
        assert!(o.status.success(), "{o:?}");
        let o = helper(name, "get", QUERY, &s);
        assert!(o.status.success());
        assert_eq!(
            String::from_utf8_lossy(&o.stdout),
            "username=alice\npassword=s3cret\n"
        );
        // A different user name does not match.
        let o = helper(
            name,
            "get",
            "protocol=https\nhost=example.com\nusername=bob\n\n",
            &s,
        );
        assert!(o.stdout.is_empty());
        // Unknown operations are ignored.
        let o = helper(name, "frobnicate", QUERY, &s);
        assert!(o.status.success() && o.stdout.is_empty());
        let o = helper(name, "erase", QUERY, &s);
        assert!(o.status.success());
        let o = helper(name, "get", QUERY, &s);
        assert!(o.stdout.is_empty(), "{name}: erased");
    }
}

#[test]
fn both_helpers_share_the_store() {
    let t = tempfile::tempdir().unwrap();
    let s = t.path().join("store");
    assert!(
        helper("credential-wincred", "store", FULL, &s)
            .status
            .success()
    );
    let o = helper("credential-manager", "get", QUERY, &s);
    assert_eq!(
        String::from_utf8_lossy(&o.stdout),
        "username=alice\npassword=s3cret\n"
    );
}

#[test]
fn no_store_refuses_honestly() {
    let none = [("ARKS_CREDENTIAL_BACKEND", "none")];
    let o = run(&["credential-manager", "store"], FULL, &none);
    assert_eq!(o.status.code(), Some(1));
    let err = String::from_utf8_lossy(&o.stderr);
    assert!(err.contains("no OS credential store is reachable"), "{err}");
    // `get` answers nothing and `erase` has nothing to do: both exit 0.
    for op in ["get", "erase"] {
        let o = run(&["credential-wincred", op], QUERY, &none);
        assert!(o.status.success(), "{op}: {o:?}");
        assert!(o.stdout.is_empty() && o.stderr.is_empty(), "{op}: {o:?}");
    }
    // On Linux without a session bus the Secret Service is unreachable too.
    if cfg!(target_os = "linux") {
        let no_bus = [
            ("DBUS_SESSION_BUS_ADDRESS", ""),
            ("XDG_RUNTIME_DIR", "/nonexistent"),
        ];
        let o = run(&["credential-manager", "store"], FULL, &no_bus);
        assert_eq!(o.status.code(), Some(1), "{o:?}");
        let err = String::from_utf8_lossy(&o.stderr);
        assert!(
            err.contains("no OS credential store is reachable") && err.contains("D-Bus"),
            "{err}"
        );
        let o = run(&["credential-wincred", "get"], QUERY, &no_bus);
        assert!(o.status.success() && o.stdout.is_empty(), "{o:?}");
        // A bus address that nothing listens on is unreachable as well.
        let t = tempfile::tempdir().unwrap();
        let addr = format!("unix:path={}", t.path().join("no-bus").display());
        let o = run(
            &["credential-manager", "store"],
            FULL,
            &[("DBUS_SESSION_BUS_ADDRESS", addr.as_str())],
        );
        assert_eq!(o.status.code(), Some(1), "{o:?}");
        assert!(String::from_utf8_lossy(&o.stderr).contains("no OS credential store is reachable"));
    }
}

/// A host no real credential uses: `arks-test-<random>.example.com`.
#[cfg(any(windows, target_os = "macos", target_os = "linux"))]
fn test_host() -> String {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    format!("arks-test-{:x}{:x}.example.com", nanos, std::process::id())
}

/// Erases every test entry for the host on drop, even when an assert fails.
#[cfg(any(windows, target_os = "macos"))]
struct EraseOnDrop(String);

#[cfg(any(windows, target_os = "macos"))]
impl Drop for EraseOnDrop {
    fn drop(&mut self) {
        let q = format!("protocol=https\nhost={}\n\n", self.0);
        let _ = run(&["credential-manager", "erase"], &q, &[]);
    }
}

/// Round trip against the real OS store (Windows Credential Manager or the
/// macOS keychain) under a unique test host, erased at the end.
#[cfg(any(windows, target_os = "macos"))]
#[test]
fn real_os_store_round_trip() {
    let host = test_host();
    let _cleanup = EraseOnDrop(host.clone());
    let q = format!("protocol=https\nhost={host}\n\n");
    let attrs =
        |u: &str, p: &str| format!("protocol=https\nhost={host}\nusername={u}\npassword={p}\n\n");
    let o = run(&["credential-manager", "get"], &q, &[]);
    assert!(o.status.success() && o.stdout.is_empty(), "{o:?}");
    let o = run(
        &["credential-manager", "store"],
        &attrs("alice", "s3cret"),
        &[],
    );
    assert!(o.status.success(), "{o:?}");
    let o = run(&["credential-wincred", "get"], &q, &[]);
    assert!(o.status.success(), "{o:?}");
    assert_eq!(
        String::from_utf8_lossy(&o.stdout),
        "username=alice\npassword=s3cret\n"
    );
    // Updating the same account replaces its password.
    let o = run(
        &["credential-manager", "store"],
        &attrs("alice", "n3w"),
        &[],
    );
    assert!(o.status.success(), "{o:?}");
    // A second account lives beside the first.
    let o = run(&["credential-manager", "store"], &attrs("bob", "b0b"), &[]);
    assert!(o.status.success(), "{o:?}");
    let by_user = |u: &str| format!("protocol=https\nhost={host}\nusername={u}\n\n");
    let o = run(&["credential-manager", "get"], &by_user("alice"), &[]);
    assert_eq!(
        String::from_utf8_lossy(&o.stdout),
        "username=alice\npassword=n3w\n"
    );
    let o = run(&["credential-manager", "get"], &by_user("bob"), &[]);
    assert_eq!(
        String::from_utf8_lossy(&o.stdout),
        "username=bob\npassword=b0b\n"
    );
    // Erase with a stale password keeps the entry; a plain erase removes all.
    let o = run(
        &["credential-manager", "erase"],
        &attrs("bob", "stale"),
        &[],
    );
    assert!(o.status.success(), "{o:?}");
    let o = run(&["credential-manager", "get"], &by_user("bob"), &[]);
    assert!(!o.stdout.is_empty(), "{o:?}");
    let o = run(&["credential-manager", "erase"], &q, &[]);
    assert!(o.status.success(), "{o:?}");
    for u in ["alice", "bob"] {
        let o = run(&["credential-manager", "get"], &by_user(u), &[]);
        assert!(o.status.success() && o.stdout.is_empty(), "{u}: {o:?}");
    }
}

/// Round trip against a Secret Service on a private session bus with an
/// isolated keyring; skipped when `dbus-run-session` or
/// `gnome-keyring-daemon` is not installed.
#[cfg(target_os = "linux")]
#[test]
fn secret_service_round_trip_on_private_bus() {
    let have = |p: &str| {
        std::env::var_os("PATH")
            .is_some_and(|ps| std::env::split_paths(&ps).any(|d| d.join(p).is_file()))
    };
    if !have("dbus-run-session") || !have("gnome-keyring-daemon") {
        eprintln!("skip: dbus-run-session or gnome-keyring-daemon not installed");
        return;
    }
    let t = tempfile::tempdir().unwrap();
    let host = test_host();
    let script = r#"
printf 'arks-test' | gnome-keyring-daemon --unlock --components=secrets >/dev/null 2>&1
q="protocol=https
host=$H
"
printf '%s\n' "$q" | "$ARKS" credential-manager get; echo "get0=$?"
printf '%susername=alice\npassword=s3cret\n\n' "$q" | "$ARKS" credential-manager store; echo "store=$?"
printf '%s\n' "$q" | "$ARKS" credential-wincred get; echo "get1=$?"
printf '%s\n' "$q" | "$ARKS" credential-manager erase; echo "erase=$?"
printf '%s\n' "$q" | "$ARKS" credential-manager get; echo "get2=$?"
"#;
    let o = Command::new("dbus-run-session")
        .args(["--", "sh", "-c", script])
        .env("ARKS", env!("CARGO_BIN_EXE_arks"))
        .env("ARKS_PERSONALITY", "git")
        .env("H", &host)
        .env("HOME", t.path())
        .env("XDG_DATA_HOME", t.path().join("data"))
        .env("XDG_CONFIG_HOME", t.path().join("config"))
        .env("XDG_RUNTIME_DIR", t.path())
        .env_remove("ARKS_CREDENTIAL_BACKEND")
        .env_remove("ARKS_CREDENTIAL_WALLET_DIR")
        .stdin(Stdio::null())
        .output()
        .unwrap();
    let out = String::from_utf8_lossy(&o.stdout);
    assert_eq!(
        out,
        "get0=0\nstore=0\nusername=alice\npassword=s3cret\nget1=0\nerase=0\nget2=0\n",
        "stderr: {}",
        String::from_utf8_lossy(&o.stderr)
    );
}

#[test]
fn argv_shapes() {
    let o = run(&["credential-wincred"], "", &[]);
    assert_eq!(o.status.code(), Some(129));
    let o = run(&["credential-manager", "--version"], "", &[]);
    assert!(o.status.success());
    assert!(String::from_utf8_lossy(&o.stdout).starts_with("arks credential-manager version "));
}

#[test]
fn used_as_credential_helper() {
    let t = tempfile::tempdir().unwrap();
    let s = t.path().join("store");
    let cfg = t.path().join("cfg");
    // Backslashes are escapes inside git config values; Windows accepts '/'.
    let exe = env!("CARGO_BIN_EXE_arks").replace('\\', "/");
    let helper = format!("!{exe} credential-manager");
    std::fs::write(&cfg, format!("[credential]\n\thelper = \"{helper}\"\n")).unwrap();
    let e = fake(&s);
    let mut envs: Vec<(&str, &str)> = e.iter().map(|(k, v)| (*k, v.as_str())).collect();
    let cfg_s = cfg.display().to_string();
    envs.push(("ARKS_CREDENTIAL_CONFIG", &cfg_s));
    envs.push(("GIT_TERMINAL_PROMPT", "0"));
    let o = run(&["credential", "approve"], FULL, &envs);
    assert!(o.status.success(), "{o:?}");
    let o = run(&["credential", "fill"], QUERY, &envs);
    assert!(o.status.success(), "{o:?}");
    let out = String::from_utf8_lossy(&o.stdout);
    assert!(
        out.contains("username=alice\n") && out.contains("password=s3cret\n"),
        "{out}"
    );
}
