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
    let o = run(
        &["credential-manager", "store"],
        FULL,
        &[("ARKS_CREDENTIAL_BACKEND", "none")],
    );
    assert_eq!(o.status.code(), Some(1));
    let err = String::from_utf8_lossy(&o.stderr);
    assert!(err.contains("no OS credential store is reachable"), "{err}");
    // On Linux without a session bus the Secret Service is unreachable too.
    if cfg!(target_os = "linux") {
        let o = run(
            &["credential-wincred", "get"],
            QUERY,
            &[
                ("DBUS_SESSION_BUS_ADDRESS", ""),
                ("XDG_RUNTIME_DIR", "/nonexistent"),
            ],
        );
        assert_eq!(o.status.code(), Some(1), "{o:?}");
        assert!(String::from_utf8_lossy(&o.stderr).contains("D-Bus"));
    }
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
