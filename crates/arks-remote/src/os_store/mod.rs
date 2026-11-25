//! OS credential store behind `git credential-manager` and
//! `git credential-wincred` on every platform (owner decision A3-2).
//!
//! The helpers speak the credential-helper protocol (`get` / `store` /
//! `erase`, attributes on standard input). The backing store is the one the
//! OS provides, reached in-process: Windows Credential Manager through the
//! Win32 `Cred*` API ([`windows`]), the macOS keychain through
//! Security.framework ([`keychain`]), the freedesktop Secret Service over
//! D-Bus elsewhere on Unix ([`secret_service`] on [`dbus`]). When none is
//! reachable `store` refuses with an error instead of pretending to store
//! anything, while `get` answers nothing and `erase` has nothing to do.
//!
//! `ARKS_CREDENTIAL_BACKEND=fake` selects a test store (one file named by
//! `ARKS_CREDENTIAL_FAKE_STORE`, or process memory when unset);
//! `ARKS_CREDENTIAL_BACKEND=none` forces the "no store" state.
//!
//! Primary: <https://git-scm.com/docs/gitcredentials> (custom helpers),
//! <https://git-scm.com/docs/git-credential> (attribute format).
//! Cite: `.spec/sot/credential-helper-askpass.sot.md` MUST 7.

#[cfg(all(unix, not(target_os = "macos")))]
#[allow(unsafe_code)] // getuid() for D-Bus EXTERNAL authentication only.
mod dbus;
#[cfg(target_os = "macos")]
#[allow(unsafe_code)] // Security.framework / Core Foundation FFI only.
mod keychain;
#[cfg(all(unix, not(target_os = "macos")))]
mod secret_service;
#[allow(unsafe_code)] // Win32 Cred* FFI only (compiled on Windows).
mod windows;

use crate::credential::Credential;
use crate::error::RemoteError;
use std::path::PathBuf;

/// Why an operation could not be served.
#[derive(Debug)]
pub(crate) enum Failure {
    /// No store is reachable (the honest-refuse state).
    Unreachable(String),
    /// The store is reachable but the operation failed.
    Other(String),
}

impl Failure {
    fn into_remote(self) -> RemoteError {
        match self {
            Failure::Unreachable(why) => no_store(&why),
            Failure::Other(m) => RemoteError::Msg(m),
        }
    }
}

fn no_store(why: &str) -> RemoteError {
    RemoteError::Msg(format!("no OS credential store is reachable ({why})"))
}

/// The service a description names: protocol, host (with any port) and path.
pub(crate) struct Service {
    pub protocol: String,
    pub host: String,
    pub path: String,
}

impl Service {
    fn of(desc: &Credential) -> Option<Service> {
        let protocol = desc.protocol.as_deref().filter(|s| !s.is_empty())?;
        let host = desc.host.as_deref().filter(|s| !s.is_empty())?;
        Some(Service {
            protocol: protocol.into(),
            host: host.into(),
            path: desc
                .path
                .as_deref()
                .unwrap_or_default()
                .trim_end_matches('/')
                .into(),
        })
    }

    /// `git:<protocol>://<host>[/<path>]`, Git Credential Manager's target.
    fn target(&self) -> String {
        windows::target_name(self, None)
    }
}

/// The selected backing store.
#[derive(Debug)]
pub enum OsStore {
    /// Test store: entries in a file (or in memory when `None`).
    Fake(Option<PathBuf>),
    /// Windows Credential Manager (Win32 `Cred*`).
    Windows,
    /// macOS keychain (Security.framework).
    Keychain,
    /// freedesktop Secret Service over the D-Bus session bus.
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

fn select() -> Result<OsStore, Failure> {
    match std::env::var("ARKS_CREDENTIAL_BACKEND").as_deref() {
        Ok("fake") => {
            let path = std::env::var_os("ARKS_CREDENTIAL_FAKE_STORE")
                .filter(|p| !p.is_empty())
                .map(PathBuf::from);
            return Ok(OsStore::Fake(path));
        }
        Ok("none") => return Err(Failure::Unreachable("ARKS_CREDENTIAL_BACKEND=none".into())),
        Ok("") | Err(_) => {}
        Ok(other) => {
            return Err(Failure::Other(format!(
                "unknown ARKS_CREDENTIAL_BACKEND '{other}' (fake|none)"
            )));
        }
    }
    if cfg!(windows) {
        Ok(OsStore::Windows)
    } else if cfg!(target_os = "macos") {
        Ok(OsStore::Keychain)
    } else if cfg!(unix) {
        // Reachability is settled when the bus is dialed.
        Ok(OsStore::SecretService)
    } else {
        Err(Failure::Unreachable(
            "no OS credential store backend for this OS".into(),
        ))
    }
}

/// Pick the store for this process, or explain why there is none.
pub fn select_os_store() -> Result<OsStore, RemoteError> {
    select().map_err(Failure::into_remote)
}

// ---------- fake store ----------

/// One fake-store entry: the target and the account.
#[derive(Clone, Debug, PartialEq, Eq)]
struct Entry {
    target: String,
    username: String,
    password: String,
}

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

fn fake_load(path: &Option<PathBuf>) -> Result<Vec<Entry>, Failure> {
    let Some(p) = path else {
        return Ok(MEMORY.with(|m| m.borrow().clone()));
    };
    let text = match std::fs::read_to_string(p) {
        Ok(t) => t,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => String::new(),
        Err(e) => return Err(Failure::Other(format!("{}: {e}", p.display()))),
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

fn fake_save(path: &Option<PathBuf>, entries: Vec<Entry>) -> Result<(), Failure> {
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
    std::fs::write(p, out).map_err(|e| Failure::Other(format!("{}: {e}", p.display())))
}

fn fake_matches(e: &Entry, target: &str, user: Option<&str>) -> bool {
    e.target == target && user.is_none_or(|u| u == e.username)
}

// ---------- per-OS dispatch ----------

mod platform {
    use super::*;

    type Found = Option<(Option<String>, String)>;

    #[cfg(windows)]
    pub fn get(
        store: &OsStore,
        svc: &Service,
        desc: &Credential,
        user: Option<&str>,
    ) -> Result<Found, Failure> {
        let _ = desc;
        match store {
            OsStore::Windows => windows::get(svc, user),
            _ => Err(elsewhere(store)),
        }
    }
    #[cfg(windows)]
    pub fn store(
        store: &OsStore,
        svc: &Service,
        desc: &Credential,
        user: &str,
        pass: &str,
    ) -> Result<(), Failure> {
        let _ = desc;
        match store {
            OsStore::Windows => windows::store(svc, user, pass),
            _ => Err(elsewhere(store)),
        }
    }
    #[cfg(windows)]
    pub fn erase(
        store: &OsStore,
        svc: &Service,
        desc: &Credential,
        user: Option<&str>,
    ) -> Result<(), Failure> {
        let _ = desc;
        match store {
            OsStore::Windows => windows::erase(svc, user),
            _ => Err(elsewhere(store)),
        }
    }

    #[cfg(target_os = "macos")]
    pub fn get(
        store: &OsStore,
        svc: &Service,
        desc: &Credential,
        user: Option<&str>,
    ) -> Result<Found, Failure> {
        let _ = desc;
        match store {
            OsStore::Keychain => keychain::get(&svc.target(), user),
            _ => Err(elsewhere(store)),
        }
    }
    #[cfg(target_os = "macos")]
    pub fn store(
        store: &OsStore,
        svc: &Service,
        desc: &Credential,
        user: &str,
        pass: &str,
    ) -> Result<(), Failure> {
        let _ = desc;
        match store {
            OsStore::Keychain => keychain::store(&svc.target(), &label(svc), user, pass),
            _ => Err(elsewhere(store)),
        }
    }
    #[cfg(target_os = "macos")]
    pub fn erase(
        store: &OsStore,
        svc: &Service,
        desc: &Credential,
        user: Option<&str>,
    ) -> Result<(), Failure> {
        let _ = desc;
        match store {
            OsStore::Keychain => keychain::erase(&svc.target(), user),
            _ => Err(elsewhere(store)),
        }
    }

    #[cfg(all(unix, not(target_os = "macos")))]
    pub fn get(
        store: &OsStore,
        svc: &Service,
        desc: &Credential,
        user: Option<&str>,
    ) -> Result<Found, Failure> {
        let _ = (svc, user);
        match store {
            OsStore::SecretService => {
                Ok(secret_service::get(desc)?.and_then(|c| Some((c.username, c.password?))))
            }
            _ => Err(elsewhere(store)),
        }
    }
    #[cfg(all(unix, not(target_os = "macos")))]
    pub fn store(
        store: &OsStore,
        svc: &Service,
        desc: &Credential,
        user: &str,
        pass: &str,
    ) -> Result<(), Failure> {
        match store {
            OsStore::SecretService => secret_service::store(desc, &label(svc), user, pass),
            _ => Err(elsewhere(store)),
        }
    }
    #[cfg(all(unix, not(target_os = "macos")))]
    pub fn erase(
        store: &OsStore,
        svc: &Service,
        desc: &Credential,
        user: Option<&str>,
    ) -> Result<(), Failure> {
        let _ = (svc, user);
        match store {
            OsStore::SecretService => secret_service::erase(desc),
            _ => Err(elsewhere(store)),
        }
    }

    #[cfg(not(any(windows, unix)))]
    pub fn get(
        store: &OsStore,
        _: &Service,
        _: &Credential,
        _: Option<&str>,
    ) -> Result<Found, Failure> {
        Err(elsewhere(store))
    }
    #[cfg(not(any(windows, unix)))]
    pub fn store(
        store: &OsStore,
        _: &Service,
        _: &Credential,
        _: &str,
        _: &str,
    ) -> Result<(), Failure> {
        Err(elsewhere(store))
    }
    #[cfg(not(any(windows, unix)))]
    pub fn erase(
        store: &OsStore,
        _: &Service,
        _: &Credential,
        _: Option<&str>,
    ) -> Result<(), Failure> {
        Err(elsewhere(store))
    }

    #[cfg_attr(windows, allow(dead_code))]
    fn label(svc: &Service) -> String {
        format!("Git: {}", &svc.target()["git:".len()..])
    }

    fn elsewhere(store: &OsStore) -> Failure {
        Failure::Unreachable(format!("{} does not exist on this OS", store.name()))
    }
}

// ---------- protocol operations ----------

fn wanted_user(desc: &Credential) -> Option<&str> {
    desc.username.as_deref().filter(|u| !u.is_empty())
}

/// `get`: the stored username / password for the description, if any.
fn get(store: &OsStore, desc: &Credential) -> Result<Option<Credential>, Failure> {
    let Some(svc) = Service::of(desc) else {
        return Ok(None);
    };
    let user = wanted_user(desc);
    let found = match store {
        OsStore::Fake(p) => {
            let target = svc.target();
            fake_load(p)?
                .into_iter()
                .find(|e| fake_matches(e, &target, user))
                .map(|e| (Some(e.username), e.password))
        }
        _ => platform::get(store, &svc, desc, user)?,
    };
    Ok(found
        .filter(|(u, _)| user.is_none() || u.is_none() || u.as_deref() == user)
        .map(|(u, p)| Credential {
            username: u,
            password: Some(p),
            ..Default::default()
        }))
}

/// `store`: remember a complete username / password for the description.
fn store(store: &OsStore, desc: &Credential) -> Result<(), Failure> {
    let Some(svc) = Service::of(desc) else {
        return Ok(());
    };
    let (Some(user), Some(pass)) = (desc.username.as_deref(), desc.password.as_deref()) else {
        return Ok(());
    };
    match store {
        OsStore::Fake(p) => {
            let target = svc.target();
            let mut all = fake_load(p)?;
            all.retain(|e| !(e.target == target && e.username == user));
            all.push(Entry {
                target,
                username: user.into(),
                password: pass.into(),
            });
            fake_save(p, all)
        }
        _ => platform::store(store, &svc, desc, user, pass),
    }
}

/// `erase`: forget matching entries (a given password must match too).
fn erase(store: &OsStore, desc: &Credential) -> Result<(), Failure> {
    let Some(svc) = Service::of(desc) else {
        return Ok(());
    };
    if let Some(pw) = desc.password.as_deref() {
        let current = get(store, desc)?;
        if current.is_none_or(|c| c.password.as_deref() != Some(pw)) {
            return Ok(());
        }
    }
    let user = wanted_user(desc);
    match store {
        OsStore::Fake(p) => {
            let target = svc.target();
            let mut all = fake_load(p)?;
            all.retain(|e| !fake_matches(e, &target, user));
            fake_save(p, all)
        }
        _ => platform::erase(store, &svc, desc, user),
    }
}

/// Run one helper operation; unknown operations are ignored (gitcredentials).
/// Returns the attribute text to print for `get`.
///
/// With no store reachable, `store` fails with the reason, `get` answers
/// nothing and `erase` succeeds (nothing can be stored there).
pub fn run_os_store_helper(op: &str, input: &str) -> Result<Option<String>, RemoteError> {
    if !matches!(op, "get" | "store" | "erase") {
        return Ok(None);
    }
    let desc = Credential::parse_attributes(input);
    let result = select().and_then(|s| match op {
        "get" => get(&s, &desc).map(|c| {
            c.map(|c| {
                let mut out = String::new();
                if let Some(u) = c.username {
                    out.push_str(&format!("username={u}\n"));
                }
                if let Some(p) = c.password {
                    out.push_str(&format!("password={p}\n"));
                }
                out
            })
        }),
        "store" => store(&s, &desc).map(|_| None),
        _ => erase(&s, &desc).map(|_| None),
    });
    match result {
        Err(Failure::Unreachable(_)) if op != "store" => Ok(None),
        r => r.map_err(Failure::into_remote),
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
        let s = OsStore::Fake(Some(dir.path().join("s")));
        let full = desc("protocol=https\nhost=example.com\nusername=alice\npassword=p\tw\n");
        store(&s, &full).unwrap();
        let bob = desc("protocol=https\nhost=example.com\nusername=bob\npassword=b\n");
        store(&s, &bob).unwrap();
        let got = get(&s, &desc("protocol=https\nhost=example.com\n"))
            .unwrap()
            .unwrap();
        assert_eq!(got.username.as_deref(), Some("alice"));
        assert_eq!(got.password.as_deref(), Some("p\tw"));
        let got = get(
            &s,
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
            assert!(get(&s, &desc(d)).unwrap().is_none(), "{d}");
        }
        // Erase with a wrong password keeps the entry.
        erase(
            &s,
            &desc("protocol=https\nhost=example.com\nusername=bob\npassword=x\n"),
        )
        .unwrap();
        assert!(
            get(
                &s,
                &desc("protocol=https\nhost=example.com\nusername=bob\n")
            )
            .unwrap()
            .is_some()
        );
        erase(&s, &desc("protocol=https\nhost=example.com\n")).unwrap();
        assert!(
            get(&s, &desc("protocol=https\nhost=example.com\n"))
                .unwrap()
                .is_none()
        );
    }

    #[test]
    fn incomplete_store_is_ignored() {
        let s = OsStore::Fake(None);
        store(&s, &desc("protocol=https\nhost=h\nusername=u\n")).unwrap();
        assert!(
            get(&s, &desc("protocol=https\nhost=h\n"))
                .unwrap()
                .is_none()
        );
        store(
            &s,
            &desc("protocol=https\nhost=h\nusername=u\npassword=p\n"),
        )
        .unwrap();
        assert!(
            get(&s, &desc("protocol=https\nhost=h\n"))
                .unwrap()
                .is_some()
        );
    }

    #[test]
    fn targets_carry_protocol_host_and_trimmed_path() {
        let t = |d: &str| Service::of(&desc(d)).map(|s| s.target());
        assert_eq!(
            t("protocol=https\nhost=h:8443\n").as_deref(),
            Some("git:https://h:8443")
        );
        assert_eq!(
            t("protocol=https\nhost=h\npath=o/r.git/\n").as_deref(),
            Some("git:https://h/o/r.git")
        );
        assert_eq!(t("host=h\n"), None);
    }

    #[test]
    fn unknown_operation_is_ignored() {
        assert_eq!(run_os_store_helper("frobnicate", "").unwrap(), None);
    }
}
