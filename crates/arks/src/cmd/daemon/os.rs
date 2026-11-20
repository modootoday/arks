//! Process-level `git daemon` behaviour that needs the OS: `--detach`,
//! `--user`/`--group` privilege drop, `--inetd` peer address, and NSS-backed
//! lookups for `~user` and client host names.
//!
//! Primary: <https://git-scm.com/docs/git-daemon> (`--detach`, `--user`,
//! `--group`, `--inetd`, `--user-path`, `--interpolated-path`). `--detach` is a
//! single `fork` + `setsid` with stdio on `/dev/null`, as observed with stock
//! `git daemon`.

#[cfg(unix)]
pub(super) use unix::*;

#[cfg(not(unix))]
pub(super) use other::*;

#[cfg(unix)]
mod unix {
    use arks_server::{DaemonPlatform, HostInfo};
    use std::ffi::{CStr, CString};
    use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr, TcpStream};
    use std::os::unix::io::FromRawFd;
    use std::path::PathBuf;

    /// Resolved `--user` / `--group` (looked up before binding, applied after).
    #[derive(Debug)]
    pub struct Credentials {
        name: CString,
        uid: libc::uid_t,
        gid: libc::gid_t,
    }

    /// Look up `--user=<name>` (names only; numeric ids are not names) and the
    /// optional `--group=<name>`; the user's primary group otherwise.
    pub fn resolve_credentials(user: &str, group: Option<&str>) -> Result<Credentials, String> {
        let pw = passwd_entry(user).ok_or_else(|| format!("user not found - {user}"))?;
        let gid = match group {
            Some(g) => group_id(g).ok_or_else(|| format!("group not found - {g}"))?,
            None => pw.gid,
        };
        Ok(Credentials {
            name: CString::new(user).map_err(|_| format!("user not found - {user}"))?,
            uid: pw.uid,
            gid,
        })
    }

    /// `initgroups`, `setgid`, `setuid` — in that order.
    pub fn drop_privileges(c: &Credentials) -> Result<(), String> {
        // SAFETY: plain libc calls with a valid NUL-terminated name.
        let failed = unsafe {
            libc::initgroups(c.name.as_ptr(), c.gid as _) != 0
                || libc::setgid(c.gid) != 0
                || libc::setuid(c.uid) != 0
        };
        if failed {
            Err("cannot drop privileges".into())
        } else {
            Ok(())
        }
    }

    /// Fork once; the parent exits 0, the child starts a new session with stdio
    /// on `/dev/null`.
    pub fn detach() -> Result<(), String> {
        if thread_count() > 1 {
            return Err("--detach needs a single-threaded process".into());
        }
        // SAFETY: the process is single-threaded here, so fork is sound.
        match unsafe { libc::fork() } {
            -1 => Err(format!("fork failed: {}", std::io::Error::last_os_error())),
            0 => {
                // SAFETY: setsid/open/dup2/close on the child's own descriptors.
                unsafe {
                    if libc::setsid() == -1 {
                        return Err(format!(
                            "setsid failed: {}",
                            std::io::Error::last_os_error()
                        ));
                    }
                    let null = libc::open(c"/dev/null".as_ptr(), libc::O_RDWR);
                    if null >= 0 {
                        for fd in 0..3 {
                            libc::dup2(null, fd);
                        }
                        if null > 2 {
                            libc::close(null);
                        }
                    }
                }
                Ok(())
            }
            _ => std::process::exit(0),
        }
    }

    fn thread_count() -> usize {
        std::fs::read_dir("/proc/self/task")
            .map(|d| d.count())
            .unwrap_or(1)
    }

    /// Remote address when stdin is a socket (inetd); `None` for pipes.
    pub fn inetd_peer() -> Option<SocketAddr> {
        // SAFETY: dup returns a fresh descriptor that the TcpStream then owns.
        let fd = unsafe { libc::dup(0) };
        if fd < 0 {
            return None;
        }
        // SAFETY: `fd` is a valid, owned descriptor (closed when `s` drops).
        let s = unsafe { TcpStream::from_raw_fd(fd) };
        s.peer_addr().ok()
    }

    struct PasswdEntry {
        uid: libc::uid_t,
        gid: libc::gid_t,
        home: PathBuf,
    }

    fn passwd_entry(user: &str) -> Option<PasswdEntry> {
        let name = CString::new(user).ok()?;
        let mut buf = vec![0 as libc::c_char; 16 * 1024];
        // SAFETY: zeroed passwd is a valid out-parameter for getpwnam_r.
        let mut pwd: libc::passwd = unsafe { std::mem::zeroed() };
        let mut result: *mut libc::passwd = std::ptr::null_mut();
        // SAFETY: all pointers are valid for the call; buf outlives the read below.
        let rc = unsafe {
            libc::getpwnam_r(
                name.as_ptr(),
                &mut pwd,
                buf.as_mut_ptr(),
                buf.len(),
                &mut result,
            )
        };
        if rc != 0 || result.is_null() {
            return None;
        }
        // SAFETY: pw_dir points into `buf`, valid while `buf` lives.
        let home = unsafe { CStr::from_ptr(pwd.pw_dir) }
            .to_string_lossy()
            .into_owned();
        Some(PasswdEntry {
            uid: pwd.pw_uid,
            gid: pwd.pw_gid,
            home: PathBuf::from(home),
        })
    }

    fn group_id(group: &str) -> Option<libc::gid_t> {
        let name = CString::new(group).ok()?;
        let mut buf = vec![0 as libc::c_char; 16 * 1024];
        // SAFETY: zeroed group is a valid out-parameter for getgrnam_r.
        let mut grp: libc::group = unsafe { std::mem::zeroed() };
        let mut result: *mut libc::group = std::ptr::null_mut();
        // SAFETY: all pointers are valid for the call.
        let rc = unsafe {
            libc::getgrnam_r(
                name.as_ptr(),
                &mut grp,
                buf.as_mut_ptr(),
                buf.len(),
                &mut result,
            )
        };
        (rc == 0 && !result.is_null()).then_some(grp.gr_gid)
    }

    /// `getpwnam_r` / `getaddrinfo(AI_CANONNAME)` lookups.
    #[derive(Debug, Clone, Copy, Default)]
    pub struct LibcPlatform;

    impl DaemonPlatform for LibcPlatform {
        fn home_dir(&self, user: &str) -> Option<PathBuf> {
            passwd_entry(user).map(|p| p.home)
        }

        fn lookup_host(&self, host: &str) -> Option<HostInfo> {
            if host.is_empty() {
                return None;
            }
            let node = CString::new(host).ok()?;
            // SAFETY: zeroed addrinfo is the documented way to build hints.
            let mut hints: libc::addrinfo = unsafe { std::mem::zeroed() };
            hints.ai_family = libc::AF_UNSPEC;
            hints.ai_socktype = libc::SOCK_STREAM;
            hints.ai_flags = libc::AI_CANONNAME;
            let mut res: *mut libc::addrinfo = std::ptr::null_mut();
            // SAFETY: valid node/hints pointers; res is freed below.
            let rc =
                unsafe { libc::getaddrinfo(node.as_ptr(), std::ptr::null(), &hints, &mut res) };
            if rc != 0 || res.is_null() {
                return None;
            }
            // SAFETY: res is a valid list head returned by getaddrinfo.
            unsafe {
                let ai = &*res;
                let ip = sockaddr_ip(ai.ai_addr, ai.ai_family);
                let canonical = if ai.ai_canonname.is_null() {
                    None
                } else {
                    Some(
                        CStr::from_ptr(ai.ai_canonname)
                            .to_string_lossy()
                            .into_owned(),
                    )
                };
                libc::freeaddrinfo(res);
                ip.map(|ip| HostInfo {
                    canonical: canonical.unwrap_or_else(|| ip.to_string()),
                    ip: ip.to_string(),
                })
            }
        }
    }

    /// # Safety
    /// `addr` must point to a socket address of `family`.
    unsafe fn sockaddr_ip(addr: *const libc::sockaddr, family: libc::c_int) -> Option<IpAddr> {
        if addr.is_null() {
            return None;
        }
        match family {
            libc::AF_INET => {
                // SAFETY: caller guarantees an AF_INET sockaddr.
                let sin = unsafe { &*(addr as *const libc::sockaddr_in) };
                Some(IpAddr::V4(Ipv4Addr::from(u32::from_be(
                    sin.sin_addr.s_addr,
                ))))
            }
            libc::AF_INET6 => {
                // SAFETY: caller guarantees an AF_INET6 sockaddr.
                let sin6 = unsafe { &*(addr as *const libc::sockaddr_in6) };
                Some(IpAddr::V6(Ipv6Addr::from(sin6.sin6_addr.s6_addr)))
            }
            _ => None,
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn credentials_need_names_not_ids() {
            assert_eq!(
                resolve_credentials("0", None).unwrap_err(),
                "user not found - 0"
            );
            let root = resolve_credentials("root", None).unwrap();
            assert_eq!((root.uid, root.gid), (0, 0));
            assert_eq!(
                resolve_credentials("root", Some("no-such-group-arks")).unwrap_err(),
                "group not found - no-such-group-arks"
            );
        }

        #[test]
        fn libc_platform_resolves_home_and_numeric_hosts() {
            assert_eq!(
                LibcPlatform.home_dir("root"),
                passwd_entry("root").map(|p| p.home)
            );
            assert!(LibcPlatform.home_dir("no-such-user-arks").is_none());
            let h = LibcPlatform.lookup_host("127.0.0.1").unwrap();
            assert_eq!(h.ip, "127.0.0.1");
            assert_eq!(h.canonical, "127.0.0.1");
            assert!(LibcPlatform.lookup_host("nonexistent.invalid").is_none());
        }

        #[test]
        fn inetd_peer_is_none_for_non_socket_stdin() {
            // cargo test runs with stdin that is not a TCP socket.
            assert!(inetd_peer().is_none());
        }
    }
}

#[cfg(not(unix))]
mod other {
    use std::net::SocketAddr;

    #[derive(Debug)]
    pub struct Credentials;

    pub fn resolve_credentials(_user: &str, _group: Option<&str>) -> Result<Credentials, String> {
        Err("--user not supported on this platform".into())
    }

    pub fn drop_privileges(_c: &Credentials) -> Result<(), String> {
        Err("cannot drop privileges".into())
    }

    pub fn detach() -> Result<(), String> {
        Err("--detach not supported on this platform".into())
    }

    pub fn inetd_peer() -> Option<SocketAddr> {
        None
    }

    pub use arks_server::StdPlatform as LibcPlatform;
}
