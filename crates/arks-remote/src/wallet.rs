//! First-party OS wallet credential helpers (`wincred` / `libsecret` / `osxkeychain`).
//!
//! Primary: <https://git-scm.com/docs/gitcredentials>
//! Cite: `.spec/sot/credential-helper-askpass.sot.md`

use crate::credential::Credential;
use crate::error::RemoteError;
use std::fs;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

/// Known first-party OS wallet helper names (gitcredentials popular helpers).
pub fn is_os_wallet_helper(helper: &str) -> bool {
    matches!(
        wallet_helper_kind(helper),
        Some(WalletKind::Wincred | WalletKind::Libsecret | WalletKind::Osxkeychain)
    )
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WalletKind {
    Wincred,
    Libsecret,
    Osxkeychain,
}

/// Parse helper string (`wincred`, `libsecret --foo`, …) into a wallet kind.
pub fn wallet_helper_kind(helper: &str) -> Option<WalletKind> {
    let name = helper.trim().trim_start_matches('!');
    let first = name.split_whitespace().next().unwrap_or("");
    let base = Path::new(first)
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or(first);
    let base = base.strip_prefix("git-credential-").unwrap_or(base);
    let base = base.strip_prefix("credential-").unwrap_or(base);
    match base {
        "wincred" => Some(WalletKind::Wincred),
        "libsecret" => Some(WalletKind::Libsecret),
        "osxkeychain" => Some(WalletKind::Osxkeychain),
        _ => None,
    }
}

/// Run a first-party wallet helper operation.
pub fn run_wallet_helper(
    kind: WalletKind,
    operation: &str,
    desc: &Credential,
) -> Result<Option<Credential>, RemoteError> {
    match operation {
        "get" => Ok(wallet_get(kind, desc)?),
        "store" => {
            wallet_store(kind, desc)?;
            Ok(None)
        }
        "erase" => {
            wallet_erase(kind, desc)?;
            Ok(None)
        }
        _ => Ok(None),
    }
}

fn target_key(desc: &Credential) -> Option<String> {
    let protocol = desc.protocol.as_deref()?.trim();
    let host = desc.host.as_deref()?.trim();
    if protocol.is_empty() || host.is_empty() {
        return None;
    }
    // Default Git wallet helpers omit path unless useHttpPath (not Claimed as required).
    Some(format!("git:{protocol}://{host}"))
}

fn wallet_dir() -> Option<PathBuf> {
    if let Ok(p) = std::env::var("ARKS_CREDENTIAL_WALLET_DIR") {
        if !p.is_empty() {
            return Some(PathBuf::from(p));
        }
    }
    None
}

fn default_portable_dir() -> PathBuf {
    if let Ok(xdg) = std::env::var("XDG_DATA_HOME") {
        if !xdg.is_empty() {
            return PathBuf::from(xdg).join("arks").join("credentials");
        }
    }
    if let Ok(home) = std::env::var("HOME").or_else(|_| std::env::var("USERPROFILE")) {
        return PathBuf::from(home)
            .join(".local")
            .join("share")
            .join("arks")
            .join("credentials");
    }
    PathBuf::from(".arks-credentials")
}

fn file_path_for(dir: &Path, key: &str) -> PathBuf {
    let mut safe = String::new();
    for c in key.chars() {
        if c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == '.' {
            safe.push(c);
        } else {
            safe.push('_');
        }
    }
    dir.join(format!("{safe}.cred"))
}

fn file_get(dir: &Path, key: &str) -> Result<Option<Credential>, RemoteError> {
    let path = file_path_for(dir, key);
    if !path.is_file() {
        return Ok(None);
    }
    let text = fs::read_to_string(&path)
        .map_err(|e| RemoteError::Msg(format!("wallet read {}: {e}", path.display())))?;
    let got = Credential::parse_attributes(&text);
    if got.has_userpass() || got.username.is_some() || got.password.is_some() {
        Ok(Some(got))
    } else {
        Ok(None)
    }
}

fn file_store(dir: &Path, key: &str, desc: &Credential) -> Result<(), RemoteError> {
    fs::create_dir_all(dir)
        .map_err(|e| RemoteError::Msg(format!("wallet mkdir {}: {e}", dir.display())))?;
    let path = file_path_for(dir, key);
    let mut out = String::new();
    if let Some(u) = &desc.username {
        out.push_str(&format!("username={u}\n"));
    }
    if let Some(p) = &desc.password {
        out.push_str(&format!("password={p}\n"));
    }
    out.push('\n');
    let tmp = path.with_extension("cred.tmp");
    fs::write(&tmp, out.as_bytes())
        .map_err(|e| RemoteError::Msg(format!("wallet write {}: {e}", tmp.display())))?;
    fs::rename(&tmp, &path)
        .map_err(|e| RemoteError::Msg(format!("wallet rename {}: {e}", path.display())))?;
    Ok(())
}

fn file_erase(dir: &Path, key: &str) -> Result<(), RemoteError> {
    let path = file_path_for(dir, key);
    match fs::remove_file(&path) {
        Ok(()) => Ok(()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(RemoteError::Msg(format!(
            "wallet erase {}: {e}",
            path.display()
        ))),
    }
}

fn resolve_dir(kind: WalletKind) -> PathBuf {
    if let Some(d) = wallet_dir() {
        return d;
    }
    match kind {
        WalletKind::Libsecret => default_portable_dir(),
        WalletKind::Wincred | WalletKind::Osxkeychain => {
            // Non-matching / no override: still allow portable dir only when explicitly set.
            // Callers check platform before falling through here for store.
            default_portable_dir()
        }
    }
}

fn wallet_get(kind: WalletKind, desc: &Credential) -> Result<Option<Credential>, RemoteError> {
    let Some(key) = target_key(desc) else {
        return Ok(None);
    };
    if wallet_dir().is_some() {
        return file_get(&resolve_dir(kind), &key);
    }
    match kind {
        WalletKind::Wincred => {
            #[cfg(windows)]
            {
                if let Some(got) = wincred_get(&key)? {
                    return Ok(Some(got));
                }
            }
            Ok(None)
        }
        WalletKind::Osxkeychain => {
            #[cfg(target_os = "macos")]
            {
                if let Some(got) = osx_get(&key)? {
                    return Ok(Some(got));
                }
            }
            Ok(None)
        }
        WalletKind::Libsecret => {
            if let Some(got) = secret_tool_get(&key)? {
                return Ok(Some(got));
            }
            // Documented Linux portable backend when secret-tool absent.
            file_get(&default_portable_dir(), &key)
        }
    }
}

fn wallet_store(kind: WalletKind, desc: &Credential) -> Result<(), RemoteError> {
    let Some(key) = target_key(desc) else {
        return Ok(());
    };
    if !desc.has_userpass() {
        return Ok(());
    }
    if wallet_dir().is_some() {
        return file_store(&resolve_dir(kind), &key, desc);
    }
    match kind {
        WalletKind::Wincred => {
            #[cfg(windows)]
            {
                return wincred_store(&key, desc);
            }
            #[cfg(not(windows))]
            {
                let _ = key;
                Ok(())
            }
        }
        WalletKind::Osxkeychain => {
            #[cfg(target_os = "macos")]
            {
                return osx_store(&key, desc);
            }
            #[cfg(not(target_os = "macos"))]
            {
                let _ = key;
                Ok(())
            }
        }
        WalletKind::Libsecret => {
            if secret_tool_store(&key, desc)? {
                return Ok(());
            }
            file_store(&default_portable_dir(), &key, desc)
        }
    }
}

fn wallet_erase(kind: WalletKind, desc: &Credential) -> Result<(), RemoteError> {
    let Some(key) = target_key(desc) else {
        return Ok(());
    };
    if wallet_dir().is_some() {
        return file_erase(&resolve_dir(kind), &key);
    }
    match kind {
        WalletKind::Wincred => {
            #[cfg(windows)]
            {
                return wincred_erase(&key);
            }
            #[cfg(not(windows))]
            {
                let _ = key;
                Ok(())
            }
        }
        WalletKind::Osxkeychain => {
            #[cfg(target_os = "macos")]
            {
                return osx_erase(&key);
            }
            #[cfg(not(target_os = "macos"))]
            {
                let _ = key;
                Ok(())
            }
        }
        WalletKind::Libsecret => {
            let _ = secret_tool_erase(&key);
            file_erase(&default_portable_dir(), &key)
        }
    }
}

fn secret_tool_get(key: &str) -> Result<Option<Credential>, RemoteError> {
    let output = match Command::new("secret-tool")
        .args(["lookup", "arks", key])
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .output()
    {
        Ok(o) => o,
        Err(_) => return Ok(None),
    };
    if !output.status.success() {
        return Ok(None);
    }
    let pass = String::from_utf8_lossy(&output.stdout)
        .lines()
        .next()
        .unwrap_or("")
        .to_string();
    if pass.is_empty() {
        return Ok(None);
    }
    // Username may be stored as attribute via separate lookup; optional.
    let user_out = Command::new("secret-tool")
        .args(["lookup", "arks-user", key])
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .output();
    let username = user_out.ok().and_then(|o| {
        if o.status.success() {
            let u = String::from_utf8_lossy(&o.stdout)
                .lines()
                .next()
                .unwrap_or("")
                .to_string();
            if u.is_empty() { None } else { Some(u) }
        } else {
            None
        }
    });
    Ok(Some(Credential {
        username,
        password: Some(pass),
        ..Default::default()
    }))
}

fn secret_tool_store(key: &str, desc: &Credential) -> Result<bool, RemoteError> {
    let pass = match &desc.password {
        Some(p) if !p.is_empty() => p.clone(),
        _ => return Ok(false),
    };
    let label = format!("arks {key}");
    let mut child = match Command::new("secret-tool")
        .args(["store", "--label", &label, "arks", key])
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
    {
        Ok(c) => c,
        Err(_) => return Ok(false),
    };
    {
        let mut stdin = child
            .stdin
            .take()
            .ok_or_else(|| RemoteError::Msg("secret-tool stdin".into()))?;
        stdin
            .write_all(pass.as_bytes())
            .map_err(|e| RemoteError::Msg(format!("secret-tool write: {e}")))?;
    }
    let status = child
        .wait()
        .map_err(|e| RemoteError::Msg(format!("secret-tool wait: {e}")))?;
    if !status.success() {
        return Ok(false);
    }
    if let Some(user) = &desc.username {
        let mut child = match Command::new("secret-tool")
            .args([
                "store",
                "--label",
                &format!("arks-user {key}"),
                "arks-user",
                key,
            ])
            .stdin(Stdio::piped())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
        {
            Ok(c) => c,
            Err(_) => return Ok(true),
        };
        if let Some(mut stdin) = child.stdin.take() {
            let _ = stdin.write_all(user.as_bytes());
        }
        let _ = child.wait();
    }
    Ok(true)
}

fn secret_tool_erase(key: &str) -> bool {
    let ok = Command::new("secret-tool")
        .args(["clear", "arks", key])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false);
    let _ = Command::new("secret-tool")
        .args(["clear", "arks-user", key])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status();
    ok
}

/// CLI entry for `arks credential-<wallet> <get|store|erase>`.
pub fn cmd_wallet_helper(kind: WalletKind, args: &[String]) -> Result<(), RemoteError> {
    let op = args
        .first()
        .map(String::as_str)
        .unwrap_or("get");
    let mut input = String::new();
    std::io::stdin()
        .read_to_string(&mut input)
        .map_err(|e| RemoteError::Msg(format!("wallet stdin: {e}")))?;
    let desc = Credential::parse_attributes(&input);
    match run_wallet_helper(kind, op, &desc)? {
        Some(got) => {
            let mut out = got.format_attrs();
            if !out.ends_with('\n') {
                out.push('\n');
            }
            out.push('\n');
            print!("{out}");
        }
        None => {}
    }
    Ok(())
}

#[cfg(windows)]
mod wincred_ffi {
    use super::*;
    use std::ffi::OsStr;
    use std::os::windows::ffi::OsStrExt;
    use std::ptr;

    #[repr(C)]
    struct CredW {
        flags: u32,
        type_: u32,
        target_name: *mut u16,
        comment: *mut u16,
        last_written: i64,
        credential_blob_size: u32,
        credential_blob: *mut u8,
        persist: u32,
        attribute_count: u32,
        attributes: *mut u8,
        target_alias: *mut u16,
        user_name: *mut u16,
    }

    const CRED_TYPE_GENERIC: u32 = 1;
    const CRED_PERSIST_LOCAL_MACHINE: u32 = 2;

    #[link(name = "advapi32")]
    unsafe extern "system" {
        fn CredReadW(target: *const u16, type_: u32, flags: u32, credential: *mut *mut CredW)
            -> i32;
        fn CredWriteW(credential: *const CredW, flags: u32) -> i32;
        fn CredDeleteW(target: *const u16, type_: u32, flags: u32) -> i32;
        fn CredFree(buffer: *mut core::ffi::c_void);
    }

    fn wide(s: &str) -> Vec<u16> {
        OsStr::new(s).encode_wide().chain(Some(0)).collect()
    }

    pub fn wincred_get(key: &str) -> Result<Option<Credential>, RemoteError> {
        let target = wide(key);
        let mut cred: *mut CredW = ptr::null_mut();
        let ok = unsafe { CredReadW(target.as_ptr(), CRED_TYPE_GENERIC, 0, &mut cred) };
        if ok == 0 || cred.is_null() {
            return Ok(None);
        }
        let result = unsafe {
            let c = &*cred;
            let pass = if c.credential_blob.is_null() || c.credential_blob_size == 0 {
                String::new()
            } else {
                let slice =
                    std::slice::from_raw_parts(c.credential_blob, c.credential_blob_size as usize);
                String::from_utf8_lossy(slice).to_string()
            };
            let user = if c.user_name.is_null() {
                None
            } else {
                let mut len = 0usize;
                while *c.user_name.add(len) != 0 {
                    len += 1;
                }
                let slice = std::slice::from_raw_parts(c.user_name, len);
                Some(String::from_utf16_lossy(slice))
            };
            CredFree(cred as *mut _);
            Credential {
                username: user,
                password: if pass.is_empty() { None } else { Some(pass) },
                ..Default::default()
            }
        };
        if result.username.is_some() || result.password.is_some() {
            Ok(Some(result))
        } else {
            Ok(None)
        }
    }

    pub fn wincred_store(key: &str, desc: &Credential) -> Result<(), RemoteError> {
        let user = desc.username.clone().unwrap_or_default();
        let pass = desc.password.clone().unwrap_or_default();
        let mut target = wide(key);
        let mut user_w = wide(&user);
        let mut blob = pass.into_bytes();
        let cred = CredW {
            flags: 0,
            type_: CRED_TYPE_GENERIC,
            target_name: target.as_mut_ptr(),
            comment: ptr::null_mut(),
            last_written: 0,
            credential_blob_size: blob.len() as u32,
            credential_blob: blob.as_mut_ptr(),
            persist: CRED_PERSIST_LOCAL_MACHINE,
            attribute_count: 0,
            attributes: ptr::null_mut(),
            target_alias: ptr::null_mut(),
            user_name: user_w.as_mut_ptr(),
        };
        let ok = unsafe { CredWriteW(&cred, 0) };
        if ok == 0 {
            return Err(RemoteError::Msg("CredWriteW failed".into()));
        }
        Ok(())
    }

    pub fn wincred_erase(key: &str) -> Result<(), RemoteError> {
        let target = wide(key);
        let _ = unsafe { CredDeleteW(target.as_ptr(), CRED_TYPE_GENERIC, 0) };
        Ok(())
    }
}

#[cfg(windows)]
use wincred_ffi::{wincred_erase, wincred_get, wincred_store};

#[cfg(target_os = "macos")]
fn osx_get(key: &str) -> Result<Option<Credential>, RemoteError> {
    // security find-generic-password -s <service> -w
    let output = Command::new("security")
        .args(["find-generic-password", "-s", key, "-w"])
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .output()
        .map_err(|e| RemoteError::Msg(format!("security: {e}")))?;
    if !output.status.success() {
        return Ok(None);
    }
    let pass = String::from_utf8_lossy(&output.stdout)
        .trim()
        .to_string();
    if pass.is_empty() {
        return Ok(None);
    }
    let user_out = Command::new("security")
        .args(["find-generic-password", "-s", key, "-g"])
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output();
    let username = user_out.ok().and_then(|o| {
        let err = String::from_utf8_lossy(&o.stderr);
        for line in err.lines() {
            if let Some(rest) = line.strip_prefix("\"acct\"<blob>=") {
                let acct = rest.trim().trim_matches('"');
                if !acct.is_empty() {
                    return Some(acct.to_string());
                }
            }
        }
        None
    });
    Ok(Some(Credential {
        username,
        password: Some(pass),
        ..Default::default()
    }))
}

#[cfg(target_os = "macos")]
fn osx_store(key: &str, desc: &Credential) -> Result<(), RemoteError> {
    let user = desc.username.clone().unwrap_or_default();
    let pass = desc.password.clone().unwrap_or_default();
    let _ = Command::new("security")
        .args(["delete-generic-password", "-s", key])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status();
    let status = Command::new("security")
        .args([
            "add-generic-password",
            "-a",
            &user,
            "-s",
            key,
            "-w",
            &pass,
            "-U",
        ])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .status()
        .map_err(|e| RemoteError::Msg(format!("security add: {e}")))?;
    if !status.success() {
        return Err(RemoteError::Msg("security add-generic-password failed".into()));
    }
    Ok(())
}

#[cfg(target_os = "macos")]
fn osx_erase(key: &str) -> Result<(), RemoteError> {
    let _ = Command::new("security")
        .args(["delete-generic-password", "-s", key])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status();
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn kind_parse() {
        assert_eq!(wallet_helper_kind("wincred"), Some(WalletKind::Wincred));
        assert_eq!(
            wallet_helper_kind("libsecret"),
            Some(WalletKind::Libsecret)
        );
        assert_eq!(
            wallet_helper_kind("osxkeychain"),
            Some(WalletKind::Osxkeychain)
        );
        assert_eq!(
            wallet_helper_kind("git-credential-wincred"),
            Some(WalletKind::Wincred)
        );
        assert!(wallet_helper_kind("store").is_none());
    }

    #[test]
    fn file_wallet_roundtrip() {
        let dir = TempDir::new().unwrap();
        // Safety: test-only env for wallet backend selection.
        unsafe {
            std::env::set_var("ARKS_CREDENTIAL_WALLET_DIR", dir.path());
        }
        let desc = Credential {
            protocol: Some("https".into()),
            host: Some("example.com".into()),
            username: Some("alice".into()),
            password: Some("s3cret".into()),
            ..Default::default()
        };
        run_wallet_helper(WalletKind::Wincred, "store", &desc).unwrap();
        let got = run_wallet_helper(WalletKind::Wincred, "get", &desc)
            .unwrap()
            .expect("stored");
        assert_eq!(got.username.as_deref(), Some("alice"));
        assert_eq!(got.password.as_deref(), Some("s3cret"));
        run_wallet_helper(WalletKind::Libsecret, "erase", &desc).unwrap();
        // erase via libsecret name against same file dir
        let gone = run_wallet_helper(WalletKind::Osxkeychain, "get", &desc).unwrap();
        assert!(gone.is_none());
        unsafe {
            std::env::remove_var("ARKS_CREDENTIAL_WALLET_DIR");
        }
    }
}
