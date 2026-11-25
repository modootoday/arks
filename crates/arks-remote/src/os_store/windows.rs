//! Windows Credential Manager backend (generic credentials) with the target
//! names of Git Credential Manager.
//!
//! Primary: Win32 `CredReadW` / `CredWriteW` / `CredDeleteW` /
//! `CredEnumerateW` (<https://learn.microsoft.com/windows/win32/api/wincred/>);
//! target convention: Git Credential Manager
//! `src/Core/Interop/Windows/WindowsCredentialManager.cs`
//! (`CreateTargetName`, `IsMatch`, `AddOrUpdate`) and `docs/credstores.md`.

use super::Service;

/// The target name GCM's `CreateTargetName` gives `service` and `account`.
pub(super) fn target_name(svc: &Service, account: Option<&str>) -> String {
    let mut t = format!("git:{}://", svc.protocol);
    if let Some(a) = account.filter(|a| !a.trim().is_empty()) {
        t.push_str(&a.replace('@', "_"));
        t.push('@');
    }
    t.push_str(&svc.host);
    if !svc.path.is_empty() {
        t.push('/');
        t.push_str(&svc.path);
    }
    t
}

/// Drop the userinfo of `scheme://userinfo@host/...` (an `@` after the
/// first `/` past the scheme belongs to the path).
#[cfg_attr(not(windows), allow(dead_code))]
pub(super) fn strip_userinfo(url: &str) -> String {
    let Some(i) = url.find("://") else {
        return url.to_string();
    };
    let auth = i + 3;
    let end = url[auth..].find('/').map_or(url.len(), |j| auth + j);
    match url[auth..end].find('@') {
        Some(at) => format!("{}{}", &url[..auth], &url[auth + at + 1..]),
        None => url.to_string(),
    }
}

/// GCM's `IsMatch` for a credential of target `target` and user `user`.
#[cfg_attr(not(windows), allow(dead_code))]
pub(super) fn is_match(
    svc: &Service,
    account: Option<&str>,
    target: &str,
    user: Option<&str>,
) -> bool {
    if let Some(a) = account.filter(|a| !a.trim().is_empty())
        && user != Some(a)
    {
        return false;
    }
    let rest = target
        .strip_prefix("LegacyGeneric:target=")
        .unwrap_or(target);
    let Some(rest) = rest.strip_prefix("git:") else {
        return false;
    };
    let want = target_name(svc, None);
    strip_userinfo(rest).eq_ignore_ascii_case(&want["git:".len()..])
}

#[cfg(windows)]
mod api {
    use super::{Service, is_match, target_name};
    use crate::os_store::Failure;
    use std::ffi::c_void;
    use std::ptr;

    #[repr(C)]
    struct CredentialW {
        flags: u32,
        type_: u32,
        target_name: *mut u16,
        comment: *mut u16,
        /// `FILETIME`; laid out as wallet.rs declares it, so the shared
        /// `advapi32` imports keep one signature.
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
    const ERROR_NOT_FOUND: i32 = 1168;
    const ERROR_NO_SUCH_LOGON_SESSION: i32 = 1312;

    #[link(name = "advapi32")]
    unsafe extern "system" {
        fn CredReadW(target: *const u16, type_: u32, flags: u32, out: *mut *mut CredentialW)
        -> i32;
        fn CredWriteW(credential: *const CredentialW, flags: u32) -> i32;
        fn CredDeleteW(target: *const u16, type_: u32, flags: u32) -> i32;
        fn CredEnumerateW(
            filter: *const u16,
            flags: u32,
            count: *mut u32,
            out: *mut *mut *mut CredentialW,
        ) -> i32;
        fn CredFree(buffer: *mut c_void);
    }

    fn last_error() -> i32 {
        std::io::Error::last_os_error().raw_os_error().unwrap_or(0)
    }

    fn failure(what: &str) -> Failure {
        let code = last_error();
        let e = std::io::Error::from_raw_os_error(code);
        if code == ERROR_NO_SUCH_LOGON_SESSION {
            Failure::Unreachable(format!("Windows Credential Manager: {e}"))
        } else {
            Failure::Other(format!("Windows Credential Manager {what}: {e}"))
        }
    }

    fn wide(s: &str) -> Vec<u16> {
        s.encode_utf16().chain(Some(0)).collect()
    }

    /// # Safety
    /// `p` is null or points at a NUL-terminated UTF-16 string.
    unsafe fn from_wide(p: *const u16) -> Option<String> {
        if p.is_null() {
            return None;
        }
        let mut n = 0;
        // SAFETY: the caller promises a NUL terminator.
        while unsafe { *p.add(n) } != 0 {
            n += 1;
        }
        // SAFETY: `n` units were just read.
        Some(String::from_utf16_lossy(unsafe {
            std::slice::from_raw_parts(p, n)
        }))
    }

    /// One stored credential, copied out of the API's buffers.
    struct Found {
        target: String,
        user: Option<String>,
        secret: String,
    }

    /// # Safety
    /// `c` is a credential returned by `CredReadW` / `CredEnumerateW`.
    unsafe fn copy_out(c: &CredentialW) -> Found {
        // SAFETY: the API fills these fields with valid buffers or nulls.
        unsafe {
            let blob = if c.credential_blob.is_null() {
                &[][..]
            } else {
                std::slice::from_raw_parts(c.credential_blob, c.credential_blob_size as usize)
            };
            let units: Vec<u16> = blob
                .chunks_exact(2)
                .map(|b| u16::from_le_bytes([b[0], b[1]]))
                .collect();
            Found {
                target: from_wide(c.target_name).unwrap_or_default(),
                user: from_wide(c.user_name),
                secret: String::from_utf16_lossy(&units),
            }
        }
    }

    fn enumerate(svc: &Service, account: Option<&str>) -> Result<Vec<Found>, Failure> {
        let filter = wide(&format!("git:{}://*", svc.protocol));
        let mut count = 0u32;
        let mut list: *mut *mut CredentialW = ptr::null_mut();
        // SAFETY: valid out-pointers; the list is freed below.
        let ok = unsafe { CredEnumerateW(filter.as_ptr(), 0, &mut count, &mut list) };
        if ok == 0 {
            return if last_error() == ERROR_NOT_FOUND {
                Ok(Vec::new())
            } else {
                Err(failure("enumerate"))
            };
        }
        let mut out = Vec::new();
        for i in 0..count as usize {
            // SAFETY: the API returned `count` valid credential pointers.
            let c = unsafe { &**list.add(i) };
            if c.type_ != CRED_TYPE_GENERIC {
                continue;
            }
            // SAFETY: `c` came from CredEnumerateW.
            let f = unsafe { copy_out(c) };
            if is_match(svc, account, &f.target, f.user.as_deref()) {
                out.push(f);
            }
        }
        // SAFETY: `list` was allocated by CredEnumerateW.
        unsafe { CredFree(list.cast()) };
        Ok(out)
    }

    fn read(target: &str) -> Result<Option<Found>, Failure> {
        let t = wide(target);
        let mut c: *mut CredentialW = ptr::null_mut();
        // SAFETY: valid out-pointer; freed below.
        if unsafe { CredReadW(t.as_ptr(), CRED_TYPE_GENERIC, 0, &mut c) } == 0 {
            return if last_error() == ERROR_NOT_FOUND {
                Ok(None)
            } else {
                Err(failure("read"))
            };
        }
        // SAFETY: `c` came from CredReadW and is freed once copied.
        let f = unsafe {
            let f = copy_out(&*c);
            CredFree(c.cast());
            f
        };
        Ok(Some(f))
    }

    /// `get`: the first match.
    pub(in crate::os_store) fn get(
        svc: &Service,
        account: Option<&str>,
    ) -> Result<Option<(Option<String>, String)>, Failure> {
        Ok(enumerate(svc, account)?
            .into_iter()
            .next()
            .map(|f| (f.user, f.secret)))
    }

    /// `store`: GCM's `AddOrUpdate`.
    pub(in crate::os_store) fn store(
        svc: &Service,
        account: &str,
        secret: &str,
    ) -> Result<(), Failure> {
        let mut target = target_name(svc, None);
        if let Some(existing) = read(&target)? {
            if existing.user.as_deref() != Some(account) {
                target = target_name(svc, Some(account));
            } else if existing.secret == secret {
                return Ok(());
            }
        }
        let mut t = wide(&target);
        let mut u = wide(account);
        let mut blob: Vec<u8> = secret.encode_utf16().flat_map(u16::to_le_bytes).collect();
        let c = CredentialW {
            flags: 0,
            type_: CRED_TYPE_GENERIC,
            target_name: t.as_mut_ptr(),
            comment: ptr::null_mut(),
            last_written: 0,
            credential_blob_size: blob.len() as u32,
            credential_blob: blob.as_mut_ptr(),
            persist: CRED_PERSIST_LOCAL_MACHINE,
            attribute_count: 0,
            attributes: ptr::null_mut(),
            target_alias: ptr::null_mut(),
            user_name: u.as_mut_ptr(),
        };
        // SAFETY: every pointer in `c` outlives the call.
        if unsafe { CredWriteW(&c, 0) } == 0 {
            return Err(failure("write"));
        }
        Ok(())
    }

    /// `erase`: delete every match.
    pub(in crate::os_store) fn erase(svc: &Service, account: Option<&str>) -> Result<(), Failure> {
        for f in enumerate(svc, account)? {
            let t = wide(&f.target);
            // SAFETY: `t` is NUL-terminated.
            if unsafe { CredDeleteW(t.as_ptr(), CRED_TYPE_GENERIC, 0) } == 0
                && last_error() != ERROR_NOT_FOUND
            {
                return Err(failure("delete"));
            }
        }
        Ok(())
    }
}

#[cfg(windows)]
pub(super) use api::{erase, get, store};

#[cfg(test)]
mod tests {
    use super::*;

    fn svc(protocol: &str, host: &str, path: &str) -> Service {
        Service {
            protocol: protocol.into(),
            host: host.into(),
            path: path.into(),
        }
    }

    #[test]
    fn target_names_follow_gcm() {
        let s = svc("https", "example.com", "");
        assert_eq!(target_name(&s, None), "git:https://example.com");
        assert_eq!(target_name(&s, Some("a@b")), "git:https://a_b@example.com");
        let s = svc("https", "example.com:8443", "org/r.git");
        assert_eq!(
            target_name(&s, Some("u")),
            "git:https://u@example.com:8443/org/r.git"
        );
    }

    #[test]
    fn userinfo_strips_only_before_the_path() {
        assert_eq!(strip_userinfo("https://u@h/p"), "https://h/p");
        assert_eq!(strip_userinfo("https://h/p@q"), "https://h/p@q");
        assert_eq!(strip_userinfo("https://h"), "https://h");
    }

    #[test]
    fn matching_ignores_case_and_userinfo_but_not_the_user() {
        let s = svc("https", "Example.com", "");
        assert!(is_match(&s, None, "git:https://example.COM", Some("x")));
        assert!(is_match(
            &s,
            Some("u"),
            "git:https://u@example.com",
            Some("u")
        ));
        assert!(!is_match(
            &s,
            Some("u"),
            "git:https://example.com",
            Some("v")
        ));
        assert!(!is_match(
            &s,
            None,
            "git:https://example.com/repo",
            Some("u")
        ));
        assert!(!is_match(&s, None, "https://example.com", Some("u")));
        assert!(!is_match(&s, None, "git:http://example.com", Some("u")));
    }
}
