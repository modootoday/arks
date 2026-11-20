//! OS credential store behind `git credential-manager` and
//! `git credential-wincred` on every platform (owner decision A3-2).
//!
//! The helpers speak the credential-helper protocol (`get` / `store` /
//! `erase`, attributes on standard input). The backing store is the one the
//! OS provides: Windows Credential Manager through the Win32 `Cred*` API, the
//! macOS keychain through the `security` tool, the freedesktop Secret Service
//! over D-Bus through libsecret's `secret-tool` (schema `org.git.Password`,
//! the one git's libsecret helper uses). When none is reachable the helper
//! refuses with an error instead of pretending to store anything.
//!
//! `ARKS_CREDENTIAL_BACKEND=fake` selects a test store (one file named by
//! `ARKS_CREDENTIAL_FAKE_STORE`, or process memory when unset);
//! `ARKS_CREDENTIAL_BACKEND=none` forces the "no store" refusal.
//!
//! Primary: <https://git-scm.com/docs/gitcredentials> (custom helpers),
//! <https://git-scm.com/docs/git-credential> (attribute format).
//! Cite: `.spec/sot/credential-helper-askpass.sot.md`.

use crate::credential::Credential;
use crate::error::RemoteError;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

/// One stored entry: the target (`git:<protocol>://<host>[/<path>]`) and the account.
#[derive(Clone, Debug, PartialEq, Eq)]
struct Entry {
    target: String,
    username: String,
    password: String,
}

/// The selected backing store.
#[derive(Debug)]
pub enum OsStore {
    /// Test store: entries in a file (or in memory when `None`).
    Fake(Option<PathBuf>),
    /// Windows Credential Manager (Win32 `CredReadW` / `CredWriteW` / `CredDeleteW`).
    Windows,
    /// macOS keychain via `/usr/bin/security`.
    Keychain,
    /// Secret Service over D-Bus via libsecret's `secret-tool`.
    SecretService,
}

impl OsStore {
    /// Human name for messages.
    pub fn name(&self) -> &'static str {
        match self {
            OsStore::Fake(_) => "fake test store",
            OsStore::Windows => "Windows Credential Manager",
            OsStore::Keychain => "macOS keychain",
            OsStore::SecretService => "Secret Service (D-Bus)",
        }
    }
}

fn on_path(prog: &str) -> bool {
    let Some(paths) = std::env::var_os("PATH") else {
        return false;
    };
    std::env::split_paths(&paths).any(|d| d.join(prog).is_file())
}

fn session_bus() -> bool {
    if std::env::var("DBUS_SESSION_BUS_ADDRESS").is_ok_and(|v| !v.is_empty()) {
        return true;
    }
    std::env::var_os("XDG_RUNTIME_DIR").is_some_and(|d| Path::new(&d).join("bus").exists())
}

/// Pick the store for this process, or explain why there is none.
pub fn select_os_store() -> Result<OsStore, RemoteError> {
    match std::env::var("ARKS_CREDENTIAL_BACKEND").as_deref() {
        Ok("fake") => {
            let path = std::env::var_os("ARKS_CREDENTIAL_FAKE_STORE")
                .filter(|p| !p.is_empty())
                .map(PathBuf::from);
            return Ok(OsStore::Fake(path));
        }
        Ok("none") => return Err(no_store("ARKS_CREDENTIAL_BACKEND=none")),
        Ok("") | Err(_) => {}
        Ok(other) => {
            return Err(RemoteError::Msg(format!(
                "unknown ARKS_CREDENTIAL_BACKEND '{other}' (fake|none)"
            )));
        }
    }
    if cfg!(windows) {
        return Ok(OsStore::Windows);
    }
    if cfg!(target_os = "macos") {
        return if Path::new("/usr/bin/security").is_file() {
            Ok(OsStore::Keychain)
        } else {
            Err(no_store("/usr/bin/security not found"))
        };
    }
    if !session_bus() {
        return Err(no_store("no D-Bus session bus for the Secret Service"));
    }
    if !on_path("secret-tool") {
        return Err(no_store(
            "secret-tool (libsecret) not found for the Secret Service",
        ));
    }
    Ok(OsStore::SecretService)
}

fn no_store(why: &str) -> RemoteError {
    RemoteError::Msg(format!("no OS credential store is reachable ({why})"))
}

fn target_of(desc: &Credential) -> Option<String> {
    let protocol = desc.protocol.as_deref().filter(|s| !s.is_empty())?;
    let host = desc.host.as_deref().filter(|s| !s.is_empty())?;
    let mut t = format!("git:{protocol}://{host}");
    if let Some(p) = desc.path.as_deref().filter(|s| !s.is_empty()) {
        t.push('/');
        t.push_str(p);
    }
    Some(t)
}

// ---------- fake store ----------

thread_local! {
    static MEMORY: std::cell::RefCell<Vec<Entry>> = const { std::cell::RefCell::new(Vec::new()) };
}

fn esc(s: &str) -> String {
    s.replace('%', "%25")
        .replace('\t', "%09")
        .replace('\n', "%0A")
}

fn unesc(s: &str) -> String {
    s.replace("%0A", "\n")
        .replace("%09", "\t")
        .replace("%25", "%")
}

fn fake_load(path: &Option<PathBuf>) -> Result<Vec<Entry>, RemoteError> {
    let Some(p) = path else {
        return Ok(MEMORY.with(|m| m.borrow().clone()));
    };
    let text = match std::fs::read_to_string(p) {
        Ok(t) => t,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => String::new(),
        Err(e) => return Err(RemoteError::Msg(format!("{}: {e}", p.display()))),
    };
    Ok(text
        .lines()
        .filter_map(|l| {
            let mut it = l.split('\t');
            Some(Entry {
                target: unesc(it.next()?),
                username: unesc(it.next()?),
                password: unesc(it.next()?),
            })
        })
        .collect())
}

fn fake_save(path: &Option<PathBuf>, entries: Vec<Entry>) -> Result<(), RemoteError> {
    let Some(p) = path else {
        MEMORY.with(|m| *m.borrow_mut() = entries);
        return Ok(());
    };
    let mut out = String::new();
    for e in &entries {
        out.push_str(&format!(
            "{}\t{}\t{}\n",
            esc(&e.target),
            esc(&e.username),
            esc(&e.password)
        ));
    }
    std::fs::write(p, out).map_err(|e| RemoteError::Msg(format!("{}: {e}", p.display())))
}

// ---------- Secret Service (secret-tool) ----------

fn ss_attrs(desc: &Credential, user: Option<&str>) -> Vec<String> {
    let mut a = vec!["xdg:schema".to_string(), "org.git.Password".to_string()];
    if let Some(p) = &desc.protocol {
        a.extend(["protocol".into(), p.clone()]);
    }
    if let Some(h) = &desc.host {
        let (server, port) = match h.rsplit_once(':') {
            Some((s, p)) if p.chars().all(|c| c.is_ascii_digit()) && !p.is_empty() => {
                (s.to_string(), Some(p.to_string()))
            }
            _ => (h.clone(), None),
        };
        a.extend(["server".into(), server]);
        if let Some(p) = port {
            a.extend(["port".into(), p]);
        }
    }
    if let Some(p) = desc.path.as_deref().filter(|s| !s.is_empty()) {
        a.extend(["object".into(), p.to_string()]);
    }
    if let Some(u) = user {
        a.extend(["user".into(), u.to_string()]);
    }
    a
}

fn secret_tool(
    args: &[String],
    stdin: Option<&str>,
) -> Result<(bool, String, String), RemoteError> {
    use std::io::Write;
    let mut child = Command::new("secret-tool")
        .args(args)
        .stdin(if stdin.is_some() {
            Stdio::piped()
        } else {
            Stdio::null()
        })
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| RemoteError::Msg(format!("secret-tool: {e}")))?;
    if let (Some(s), Some(mut w)) = (stdin, child.stdin.take()) {
        w.write_all(s.as_bytes())
            .map_err(|e| RemoteError::Msg(format!("secret-tool: {e}")))?;
    }
    let o = child
        .wait_with_output()
        .map_err(|e| RemoteError::Msg(format!("secret-tool: {e}")))?;
    Ok((
        o.status.success(),
        String::from_utf8_lossy(&o.stdout).into_owned(),
        String::from_utf8_lossy(&o.stderr).trim().to_string(),
    ))
}

/// `secret-tool search` output: `attribute.user = <name>` and `secret = <pw>` lines.
fn parse_search(text: &str) -> Option<(String, String)> {
    let mut user = None;
    let mut secret = None;
    for l in text.lines() {
        if let Some(v) = l.strip_prefix("attribute.user = ") {
            user.get_or_insert_with(|| v.to_string());
        } else if let Some(v) = l.strip_prefix("secret = ") {
            secret.get_or_insert_with(|| v.to_string());
        }
        if user.is_some() && secret.is_some() {
            break;
        }
    }
    Some((user?, secret?))
}

fn ss_get(desc: &Credential) -> Result<Option<Credential>, RemoteError> {
    let mut args = vec!["search".to_string(), "--unlock".to_string()];
    args.extend(ss_attrs(desc, desc.username.as_deref()));
    let (ok, out, _) = secret_tool(&args, None)?;
    if !ok {
        return Ok(None);
    }
    Ok(parse_search(&out).map(|(u, p)| Credential {
        username: Some(u),
        password: Some(p),
        ..Default::default()
    }))
}

fn ss_store(desc: &Credential, user: &str, pass: &str) -> Result<(), RemoteError> {
    let label = format!(
        "Git: {}://{}",
        desc.protocol.as_deref().unwrap_or(""),
        desc.host.as_deref().unwrap_or("")
    );
    let mut args = vec!["store".to_string(), "--label".to_string(), label];
    args.extend(ss_attrs(desc, Some(user)));
    let (ok, _, err) = secret_tool(&args, Some(pass))?;
    if ok {
        Ok(())
    } else {
        Err(RemoteError::Msg(format!(
            "Secret Service refused the store: {err}"
        )))
    }
}

fn ss_erase(desc: &Credential) -> Result<(), RemoteError> {
    let mut args = vec!["clear".to_string()];
    args.extend(ss_attrs(desc, desc.username.as_deref()));
    // `clear` with no match is not an error for the helper protocol.
    let _ = secret_tool(&args, None)?;
    Ok(())
}

// ---------- protocol operations ----------

fn matches(e: &Entry, target: &str, desc: &Credential) -> bool {
    e.target == target && desc.username.as_deref().is_none_or(|u| u == e.username)
}

/// `get`: the stored username / password for the description, if any.
pub fn os_store_get(store: &OsStore, desc: &Credential) -> Result<Option<Credential>, RemoteError> {
    let Some(target) = target_of(desc) else {
        return Ok(None);
    };
    let found = match store {
        OsStore::Fake(p) => fake_load(p)?
            .into_iter()
            .find(|e| matches(e, &target, desc))
            .map(|e| Credential {
                username: Some(e.username),
                password: Some(e.password),
                ..Default::default()
            }),
        OsStore::SecretService => ss_get(desc)?,
        OsStore::Windows => platform::windows_get(&target)?,
        OsStore::Keychain => platform::keychain_get(&target)?,
    };
    Ok(found
        .filter(|c| desc.username.is_none() || c.username.is_none() || c.username == desc.username))
}

/// `store`: remember a complete username / password for the description.
pub fn os_store_store(store: &OsStore, desc: &Credential) -> Result<(), RemoteError> {
    let Some(target) = target_of(desc) else {
        return Ok(());
    };
    let (Some(user), Some(pass)) = (desc.username.as_deref(), desc.password.as_deref()) else {
        return Ok(());
    };
    match store {
        OsStore::Fake(p) => {
            let mut all = fake_load(p)?;
            all.retain(|e| !(e.target == target && e.username == user));
            all.push(Entry {
                target,
                username: user.into(),
                password: pass.into(),
            });
            fake_save(p, all)
        }
        OsStore::SecretService => ss_store(desc, user, pass),
        OsStore::Windows => platform::windows_store(&target, desc),
        OsStore::Keychain => platform::keychain_store(&target, desc),
    }
}

/// `erase`: forget matching entries (a given password must match too).
pub fn os_store_erase(store: &OsStore, desc: &Credential) -> Result<(), RemoteError> {
    let Some(target) = target_of(desc) else {
        return Ok(());
    };
    if let Some(pw) = desc.password.as_deref() {
        let current = os_store_get(store, desc)?;
        if current.is_some_and(|c| c.password.as_deref() != Some(pw)) {
            return Ok(());
        }
    }
    match store {
        OsStore::Fake(p) => {
            let mut all = fake_load(p)?;
            all.retain(|e| !matches(e, &target, desc));
            fake_save(p, all)
        }
        OsStore::SecretService => ss_erase(desc),
        OsStore::Windows => {
            if os_store_get(store, desc)?.is_some() {
                platform::windows_erase(&target)?;
            }
            Ok(())
        }
        OsStore::Keychain => platform::keychain_erase(&target),
    }
}

/// Run one helper operation; unknown operations are ignored (gitcredentials).
/// Returns the attribute text to print for `get`.
pub fn run_os_store_helper(op: &str, input: &str) -> Result<Option<String>, RemoteError> {
    if !matches!(op, "get" | "store" | "erase") {
        return Ok(None);
    }
    let store = select_os_store()?;
    let desc = Credential::parse_attributes(input);
    match op {
        "get" => Ok(os_store_get(&store, &desc)?.map(|c| {
            let mut out = String::new();
            if let Some(u) = c.username {
                out.push_str(&format!("username={u}\n"));
            }
            if let Some(p) = c.password {
                out.push_str(&format!("password={p}\n"));
            }
            out
        })),
        "store" => os_store_store(&store, &desc).map(|_| None),
        _ => os_store_erase(&store, &desc).map(|_| None),
    }
}

mod platform {
    use super::*;

    #[cfg(windows)]
    pub fn windows_get(target: &str) -> Result<Option<Credential>, RemoteError> {
        crate::wallet::wincred_get(target)
    }
    #[cfg(windows)]
    pub fn windows_store(target: &str, desc: &Credential) -> Result<(), RemoteError> {
        crate::wallet::wincred_store(target, desc)
    }
    #[cfg(windows)]
    pub fn windows_erase(target: &str) -> Result<(), RemoteError> {
        crate::wallet::wincred_erase(target)
    }
    #[cfg(not(windows))]
    pub fn windows_get(_: &str) -> Result<Option<Credential>, RemoteError> {
        Err(no_store(
            "Windows Credential Manager exists only on Windows",
        ))
    }
    #[cfg(not(windows))]
    pub fn windows_store(_: &str, _: &Credential) -> Result<(), RemoteError> {
        Err(no_store(
            "Windows Credential Manager exists only on Windows",
        ))
    }
    #[cfg(not(windows))]
    pub fn windows_erase(_: &str) -> Result<(), RemoteError> {
        Err(no_store(
            "Windows Credential Manager exists only on Windows",
        ))
    }

    fn security(args: &[&str]) -> Result<std::process::Output, RemoteError> {
        Command::new("/usr/bin/security")
            .args(args)
            .stdin(Stdio::null())
            .output()
            .map_err(|e| RemoteError::Msg(format!("security: {e}")))
    }

    /// `security find-generic-password -s <target> -g`: account on stdout
    /// attributes (`"acct"<blob>="..."`), password on stderr (`password: "..."`).
    pub fn keychain_get(target: &str) -> Result<Option<Credential>, RemoteError> {
        let o = security(&["find-generic-password", "-s", target, "-g"])?;
        if !o.status.success() {
            return Ok(None);
        }
        let text = format!(
            "{}{}",
            String::from_utf8_lossy(&o.stdout),
            String::from_utf8_lossy(&o.stderr)
        );
        let mut user = None;
        let mut pass = None;
        for l in text.lines() {
            let l = l.trim();
            if let Some(v) = l.strip_prefix("\"acct\"<blob>=") {
                user = Some(v.trim_matches('"').to_string());
            } else if let Some(v) = l.strip_prefix("password: ") {
                pass = Some(v.trim_matches('"').to_string());
            }
        }
        Ok(pass.map(|p| Credential {
            username: user,
            password: Some(p),
            ..Default::default()
        }))
    }

    pub fn keychain_store(target: &str, desc: &Credential) -> Result<(), RemoteError> {
        let user = desc.username.as_deref().unwrap_or("");
        let pass = desc.password.as_deref().unwrap_or("");
        let o = security(&[
            "add-generic-password",
            "-U",
            "-a",
            user,
            "-s",
            target,
            "-w",
            pass,
        ])?;
        if o.status.success() {
            Ok(())
        } else {
            Err(RemoteError::Msg(format!(
                "macOS keychain refused the store: {}",
                String::from_utf8_lossy(&o.stderr).trim()
            )))
        }
    }

    pub fn keychain_erase(target: &str) -> Result<(), RemoteError> {
        let _ = security(&["delete-generic-password", "-s", target])?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn desc(input: &str) -> Credential {
        Credential::parse_attributes(input)
    }

    #[test]
    fn fake_file_roundtrip_and_matching() {
        let dir = tempfile::TempDir::new().unwrap();
        let store = OsStore::Fake(Some(dir.path().join("s")));
        let full = desc("protocol=https\nhost=example.com\nusername=alice\npassword=p\tw\n");
        os_store_store(&store, &full).unwrap();
        let bob = desc("protocol=https\nhost=example.com\nusername=bob\npassword=b\n");
        os_store_store(&store, &bob).unwrap();
        let got = os_store_get(&store, &desc("protocol=https\nhost=example.com\n"))
            .unwrap()
            .unwrap();
        assert_eq!(got.username.as_deref(), Some("alice"));
        assert_eq!(got.password.as_deref(), Some("p\tw"));
        let got = os_store_get(
            &store,
            &desc("protocol=https\nhost=example.com\nusername=bob\n"),
        )
        .unwrap()
        .unwrap();
        assert_eq!(got.password.as_deref(), Some("b"));
        // Other host / protocol / path: nothing.
        for d in [
            "protocol=https\nhost=other.com\n",
            "protocol=http\nhost=example.com\n",
            "protocol=https\nhost=example.com\npath=r.git\n",
        ] {
            assert!(os_store_get(&store, &desc(d)).unwrap().is_none(), "{d}");
        }
        // Erase with a wrong password keeps the entry.
        os_store_erase(
            &store,
            &desc("protocol=https\nhost=example.com\nusername=bob\npassword=x\n"),
        )
        .unwrap();
        assert!(
            os_store_get(
                &store,
                &desc("protocol=https\nhost=example.com\nusername=bob\n")
            )
            .unwrap()
            .is_some()
        );
        os_store_erase(&store, &desc("protocol=https\nhost=example.com\n")).unwrap();
        assert!(
            os_store_get(&store, &desc("protocol=https\nhost=example.com\n"))
                .unwrap()
                .is_none()
        );
    }

    #[test]
    fn incomplete_store_is_ignored() {
        let store = OsStore::Fake(None);
        os_store_store(&store, &desc("protocol=https\nhost=h\nusername=u\n")).unwrap();
        assert!(
            os_store_get(&store, &desc("protocol=https\nhost=h\n"))
                .unwrap()
                .is_none()
        );
        os_store_store(
            &store,
            &desc("protocol=https\nhost=h\nusername=u\npassword=p\n"),
        )
        .unwrap();
        assert!(
            os_store_get(&store, &desc("protocol=https\nhost=h\n"))
                .unwrap()
                .is_some()
        );
    }

    #[test]
    fn secret_service_attributes_and_search_output() {
        let a = ss_attrs(
            &desc("protocol=https\nhost=h.example:8443\npath=x/y.git\n"),
            Some("u"),
        );
        assert_eq!(
            a,
            [
                "xdg:schema",
                "org.git.Password",
                "protocol",
                "https",
                "server",
                "h.example",
                "port",
                "8443",
                "object",
                "x/y.git",
                "user",
                "u"
            ]
        );
        let out = "[/org/freedesktop/secrets/collection/login/3]\nlabel = Git: https://h\nsecret = pw\ncreated = 2026-01-01 00:00:00\nschema = org.git.Password\nattribute.protocol = https\nattribute.server = h\nattribute.user = alice\n";
        assert_eq!(parse_search(out), Some(("alice".into(), "pw".into())));
        assert_eq!(parse_search("label = x\n"), None);
    }

    #[test]
    fn unknown_operation_is_ignored() {
        assert_eq!(run_os_store_helper("frobnicate", "").unwrap(), None);
    }
}
