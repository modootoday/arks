//! macOS keychain backend: Security.framework generic passwords, service =
//! the `git:` target, account = the username (the convention Git Credential
//! Manager's macOS keychain store also uses).
//!
//! Primary: Apple Security framework keychain item API (`SecItemAdd`,
//! `SecItemCopyMatching`, `SecItemUpdate`, `SecItemDelete`, class
//! `kSecClassGenericPassword`) and Core Foundation (`CFDictionary`,
//! `CFString`, `CFData`), <https://developer.apple.com/documentation/security/keychain-services>.

use super::Failure;
use std::ffi::c_void;
use std::ptr;

type CFTypeRef = *const c_void;
type CFIndex = isize;
type OSStatus = i32;

#[repr(C)]
struct CFDictionaryCallBacks {
    _private: [u8; 0],
}

const UTF8: u32 = 0x0800_0100;
const ERR_SEC_SUCCESS: OSStatus = 0;
const ERR_SEC_ITEM_NOT_FOUND: OSStatus = -25300;
const ERR_SEC_DUPLICATE_ITEM: OSStatus = -25299;
const ERR_SEC_NOT_AVAILABLE: OSStatus = -25291;
const ERR_SEC_NO_SUCH_KEYCHAIN: OSStatus = -25294;
const ERR_SEC_NO_DEFAULT_KEYCHAIN: OSStatus = -25307;
const ERR_SEC_INTERACTION_NOT_ALLOWED: OSStatus = -25308;

#[link(name = "CoreFoundation", kind = "framework")]
unsafe extern "C" {
    static kCFBooleanTrue: CFTypeRef;
    static kCFTypeDictionaryKeyCallBacks: CFDictionaryCallBacks;
    static kCFTypeDictionaryValueCallBacks: CFDictionaryCallBacks;
    fn CFDictionaryCreate(
        alloc: CFTypeRef,
        keys: *const CFTypeRef,
        values: *const CFTypeRef,
        n: CFIndex,
        key_cb: *const CFDictionaryCallBacks,
        value_cb: *const CFDictionaryCallBacks,
    ) -> CFTypeRef;
    fn CFDictionaryGetValue(dict: CFTypeRef, key: CFTypeRef) -> CFTypeRef;
    fn CFStringCreateWithBytes(
        alloc: CFTypeRef,
        bytes: *const u8,
        n: CFIndex,
        encoding: u32,
        external: u8,
    ) -> CFTypeRef;
    fn CFStringGetLength(s: CFTypeRef) -> CFIndex;
    fn CFStringGetBytes(
        s: CFTypeRef,
        range: CFRange,
        encoding: u32,
        loss_byte: u8,
        external: u8,
        buf: *mut u8,
        max: CFIndex,
        used: *mut CFIndex,
    ) -> CFIndex;
    fn CFDataCreate(alloc: CFTypeRef, bytes: *const u8, n: CFIndex) -> CFTypeRef;
    fn CFDataGetLength(d: CFTypeRef) -> CFIndex;
    fn CFDataGetBytePtr(d: CFTypeRef) -> *const u8;
    fn CFGetTypeID(v: CFTypeRef) -> usize;
    fn CFStringGetTypeID() -> usize;
    fn CFDataGetTypeID() -> usize;
    fn CFRelease(v: CFTypeRef);
}

#[repr(C)]
#[derive(Clone, Copy)]
struct CFRange {
    location: CFIndex,
    length: CFIndex,
}

#[link(name = "Security", kind = "framework")]
unsafe extern "C" {
    static kSecClass: CFTypeRef;
    static kSecClassGenericPassword: CFTypeRef;
    static kSecAttrService: CFTypeRef;
    static kSecAttrAccount: CFTypeRef;
    static kSecAttrLabel: CFTypeRef;
    static kSecValueData: CFTypeRef;
    static kSecReturnData: CFTypeRef;
    static kSecReturnAttributes: CFTypeRef;
    static kSecMatchLimit: CFTypeRef;
    static kSecMatchLimitOne: CFTypeRef;
    fn SecItemAdd(attrs: CFTypeRef, result: *mut CFTypeRef) -> OSStatus;
    fn SecItemCopyMatching(query: CFTypeRef, result: *mut CFTypeRef) -> OSStatus;
    fn SecItemUpdate(query: CFTypeRef, update: CFTypeRef) -> OSStatus;
    fn SecItemDelete(query: CFTypeRef) -> OSStatus;
}

/// An owned Core Foundation object, released on drop.
struct Owned(CFTypeRef);

impl Drop for Owned {
    fn drop(&mut self) {
        if !self.0.is_null() {
            // SAFETY: `self.0` is an owned (+1) reference.
            unsafe { CFRelease(self.0) }
        }
    }
}

fn cf_string(s: &str) -> Owned {
    // SAFETY: `s` is valid UTF-8 of the given length.
    Owned(unsafe { CFStringCreateWithBytes(ptr::null(), s.as_ptr(), s.len() as CFIndex, UTF8, 0) })
}

fn cf_data(b: &[u8]) -> Owned {
    // SAFETY: `b` is a valid buffer of the given length.
    Owned(unsafe { CFDataCreate(ptr::null(), b.as_ptr(), b.len() as CFIndex) })
}

fn dict(pairs: &[(CFTypeRef, CFTypeRef)]) -> Owned {
    let keys: Vec<CFTypeRef> = pairs.iter().map(|p| p.0).collect();
    let values: Vec<CFTypeRef> = pairs.iter().map(|p| p.1).collect();
    // SAFETY: equal-length key/value arrays of CF objects; the CFType
    // callbacks retain them.
    Owned(unsafe {
        CFDictionaryCreate(
            ptr::null(),
            keys.as_ptr(),
            values.as_ptr(),
            pairs.len() as CFIndex,
            &kCFTypeDictionaryKeyCallBacks,
            &kCFTypeDictionaryValueCallBacks,
        )
    })
}

/// # Safety
/// `s` is null or a CFString.
unsafe fn rust_string(s: CFTypeRef) -> Option<String> {
    // SAFETY: type-checked before use.
    unsafe {
        if s.is_null() || CFGetTypeID(s) != CFStringGetTypeID() {
            return None;
        }
        let range = CFRange {
            location: 0,
            length: CFStringGetLength(s),
        };
        let mut used: CFIndex = 0;
        CFStringGetBytes(s, range, UTF8, 0, 0, ptr::null_mut(), 0, &mut used);
        let mut buf = vec![0u8; used as usize];
        CFStringGetBytes(s, range, UTF8, 0, 0, buf.as_mut_ptr(), used, &mut used);
        String::from_utf8(buf).ok()
    }
}

/// # Safety
/// `d` is null or a CFData.
unsafe fn rust_bytes(d: CFTypeRef) -> Option<Vec<u8>> {
    // SAFETY: type-checked before use.
    unsafe {
        if d.is_null() || CFGetTypeID(d) != CFDataGetTypeID() {
            return None;
        }
        let n = CFDataGetLength(d) as usize;
        let p = CFDataGetBytePtr(d);
        Some(if n == 0 {
            Vec::new()
        } else {
            std::slice::from_raw_parts(p, n).to_vec()
        })
    }
}

fn status(what: &str, s: OSStatus) -> Failure {
    let msg = format!("macOS keychain {what} failed (OSStatus {s})");
    match s {
        ERR_SEC_NOT_AVAILABLE
        | ERR_SEC_NO_SUCH_KEYCHAIN
        | ERR_SEC_NO_DEFAULT_KEYCHAIN
        | ERR_SEC_INTERACTION_NOT_ALLOWED => Failure::Unreachable(msg),
        _ => Failure::Other(msg),
    }
}

/// Query pairs for `service` (and `account` when given); the owned strings
/// must outlive the pairs.
fn base(service: &Owned, account: Option<&Owned>) -> Vec<(CFTypeRef, CFTypeRef)> {
    // SAFETY: reading framework-provided constant objects.
    let mut q = unsafe {
        vec![
            (kSecClass, kSecClassGenericPassword),
            (kSecAttrService, service.0),
        ]
    };
    if let Some(a) = account {
        // SAFETY: as above.
        q.push((unsafe { kSecAttrAccount }, a.0));
    }
    q
}

/// `get`: account and password of the first item for `service`.
pub(super) fn get(
    service: &str,
    account: Option<&str>,
) -> Result<Option<(Option<String>, String)>, Failure> {
    let svc = cf_string(service);
    let acct = account.map(cf_string);
    let mut q = base(&svc, acct.as_ref());
    // SAFETY: reading framework-provided constant objects.
    unsafe {
        q.push((kSecReturnAttributes, kCFBooleanTrue));
        q.push((kSecReturnData, kCFBooleanTrue));
        q.push((kSecMatchLimit, kSecMatchLimitOne));
    }
    let query = dict(&q);
    let mut out: CFTypeRef = ptr::null();
    // SAFETY: valid query; `out` receives a +1 dictionary on success.
    let s = unsafe { SecItemCopyMatching(query.0, &mut out) };
    if s == ERR_SEC_ITEM_NOT_FOUND {
        return Ok(None);
    }
    if s != ERR_SEC_SUCCESS {
        return Err(status("lookup", s));
    }
    let result = Owned(out);
    // SAFETY: `result` is the attributes dictionary SecItemCopyMatching returned.
    unsafe {
        let user = rust_string(CFDictionaryGetValue(result.0, kSecAttrAccount));
        let secret = rust_bytes(CFDictionaryGetValue(result.0, kSecValueData)).unwrap_or_default();
        Ok(Some((user, String::from_utf8_lossy(&secret).into_owned())))
    }
}

/// `store`: add the item, or update the password of the existing one.
pub(super) fn store(
    service: &str,
    label: &str,
    account: &str,
    secret: &str,
) -> Result<(), Failure> {
    let svc = cf_string(service);
    let acct = cf_string(account);
    let lbl = cf_string(label);
    let data = cf_data(secret.as_bytes());
    let mut add = base(&svc, Some(&acct));
    // SAFETY: reading framework-provided constant objects.
    unsafe {
        add.push((kSecAttrLabel, lbl.0));
        add.push((kSecValueData, data.0));
    }
    let attrs = dict(&add);
    // SAFETY: valid attribute dictionary; no result requested.
    let s = unsafe { SecItemAdd(attrs.0, ptr::null_mut()) };
    if s == ERR_SEC_SUCCESS {
        return Ok(());
    }
    if s != ERR_SEC_DUPLICATE_ITEM {
        return Err(status("store", s));
    }
    let query = dict(&base(&svc, Some(&acct)));
    // SAFETY: reading a framework-provided constant object.
    let update = dict(&[(unsafe { kSecValueData }, data.0)]);
    // SAFETY: valid query and update dictionaries.
    let s = unsafe { SecItemUpdate(query.0, update.0) };
    if s == ERR_SEC_SUCCESS {
        Ok(())
    } else {
        Err(status("update", s))
    }
}

/// Upper bound on delete rounds in [`erase`]; far above the accounts one
/// `service` holds in practice.
const ERASE_ROUNDS: usize = 256;

/// `erase`: delete every item for `service` (and `account`). With the macOS
/// file-based keychain one `SecItemDelete` call removes a single match (CI
/// on macOS 15 left a second account behind), so the delete repeats until no
/// item matches.
pub(super) fn erase(service: &str, account: Option<&str>) -> Result<(), Failure> {
    let svc = cf_string(service);
    let acct = account.map(cf_string);
    let query = dict(&base(&svc, acct.as_ref()));
    for _ in 0..ERASE_ROUNDS {
        // SAFETY: valid query dictionary.
        match unsafe { SecItemDelete(query.0) } {
            ERR_SEC_SUCCESS => continue,
            ERR_SEC_ITEM_NOT_FOUND => return Ok(()),
            s => return Err(status("delete", s)),
        }
    }
    Err(status(
        "delete (items still match after repeated deletes)",
        ERR_SEC_SUCCESS,
    ))
}
