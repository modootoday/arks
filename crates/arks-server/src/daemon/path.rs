//! Request path → repository resolution with `git daemon` semantics.
//!
//! Primary: <https://git-scm.com/docs/git-daemon> (`--strict-paths`,
//! `--base-path`, `--base-path-relaxed`, `--user-path`, `--interpolated-path`,
//! `<directory>...`). Details not spelled out in the documentation (alias
//! rejection, suffix order, whitelist boundary, log wording) follow observed
//! stock `git daemon` behaviour; no git code is reused.

use super::log::DaemonLog;
use super::options::DaemonOptions;
use super::platform::DaemonPlatform;
use std::path::{Path, PathBuf};

/// Client host context for `%H %CH %IP %P` (hook arguments and templates).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(super) struct HostContext {
    pub hostname: String,
    pub canonical: String,
    pub ip: String,
    pub port: String,
}

/// A repository the request resolved to.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct ResolvedRepo {
    /// Validated path: request after rewriting, with the suffix that matched and
    /// `~user` left unexpanded (what the whitelist and the access hook see).
    pub validated: String,
    /// Filesystem git directory.
    pub git_dir: PathBuf,
}

/// Non-strict lookup suffixes, in order.
const SUFFIXES: [&str; 4] = ["/.git", "", ".git/.git", ".git"];

/// Resolve `dir` or log why it was refused (`None` = "no such repository").
pub(super) fn resolve_request(
    opts: &DaemonOptions,
    dir: &str,
    host: Option<&HostContext>,
) -> Option<ResolvedRepo> {
    let log = &opts.log;
    if is_aliased(dir) {
        log.error(&format!("'{dir}': aliased"));
        return None;
    }
    let mut relaxed_fallback = None;
    let candidate = if dir.starts_with('~') {
        let Some(user_path) = opts.user_path.as_deref() else {
            log.error(&format!("'{dir}': User-path not allowed"));
            return None;
        };
        rewrite_user_path(dir, user_path, log)
    } else if let (Some(template), Some(host)) = (opts.interpolated_path.as_deref(), host) {
        let out = interpolate(template, host, dir);
        log.info(&format!("Interpolated dir '{out}'"));
        out
    } else if let Some(base) = opts.base_path.as_deref() {
        if !dir.starts_with('/') {
            log.error(&format!(
                "'{dir}': Non-absolute path denied (base-path active)"
            ));
            return None;
        }
        if opts.base_path_relaxed {
            relaxed_fallback = Some(dir.to_string());
        }
        format!("{}{dir}", base.to_string_lossy())
    } else {
        dir.to_string()
    };
    let platform = opts.platform.as_ref();
    let found = enter_repo(&candidate, opts.strict_paths, platform).or_else(|| {
        relaxed_fallback
            .as_deref()
            .and_then(|raw| enter_repo(raw, opts.strict_paths, platform))
    });
    let Some(repo) = found else {
        let shown = relaxed_fallback.as_deref().unwrap_or(&candidate);
        log.error(&format!("'{shown}' does not appear to be a git repository"));
        return None;
    };
    if !whitelisted(&repo.validated, &opts.whitelist, opts.strict_paths) {
        log.error(&format!("'{}': not in directory list", repo.validated));
        return None;
    }
    Some(repo)
}

/// Paths must start with `/` or `~` and may not contain empty, `.` or `..`
/// components (a single trailing `/` is fine).
pub(super) fn is_aliased(dir: &str) -> bool {
    let Some(rest) = dir.strip_prefix('/').or_else(|| dir.strip_prefix('~')) else {
        return true;
    };
    let parts: Vec<&str> = rest.split('/').collect();
    let last = parts.len() - 1;
    parts
        .iter()
        .enumerate()
        .any(|(i, part)| (part.is_empty() && i != last) || *part == "." || *part == "..")
}

/// `~alice/foo` → `~alice/<user_path>/foo` when `--user-path=<user_path>`.
fn rewrite_user_path(dir: &str, user_path: &str, log: &DaemonLog) -> String {
    if user_path.is_empty() {
        return dir.to_string();
    }
    let (name, slash) = match dir.find('/') {
        Some(i) => (&dir[..i], &dir[i..]),
        None => (dir, ""),
    };
    log.info(&format!(
        "userpath <{user_path}>, request <{dir}>, namlen {}, restlen {}, slash <{slash}>",
        name.len(),
        slash.len()
    ));
    format!("{name}/{user_path}{slash}")
}

/// Expand `%H %CH %IP %P %D %%`; any other `%x` stays literal.
pub(super) fn interpolate(template: &str, host: &HostContext, dir: &str) -> String {
    let mut out = String::with_capacity(template.len() + dir.len());
    let mut rest = template;
    while let Some(i) = rest.find('%') {
        out.push_str(&rest[..i]);
        let tail = &rest[i + 1..];
        let (value, used) = if tail.starts_with('%') {
            ("%", 1)
        } else if tail.starts_with("CH") {
            (host.canonical.as_str(), 2)
        } else if tail.starts_with("IP") {
            (host.ip.as_str(), 2)
        } else if tail.starts_with('H') {
            (host.hostname.as_str(), 1)
        } else if tail.starts_with('P') {
            (host.port.as_str(), 1)
        } else if tail.starts_with('D') {
            (dir, 1)
        } else {
            ("%", 0)
        };
        out.push_str(value);
        rest = &tail[used..];
    }
    out.push_str(rest);
    out
}

/// Find the git directory for `path`.
fn enter_repo(path: &str, strict: bool, platform: &dyn DaemonPlatform) -> Option<ResolvedRepo> {
    if strict {
        let fs = PathBuf::from(path);
        return is_git_directory(&fs).then(|| ResolvedRepo {
            validated: path.to_string(),
            git_dir: fs,
        });
    }
    let trimmed = path.trim_end_matches('/');
    let trimmed = if trimmed.is_empty() && path.starts_with('/') {
        "/"
    } else {
        trimmed
    };
    let fs_base = expand_user(trimmed, platform)?;
    SUFFIXES.iter().find_map(|suffix| {
        let fs = PathBuf::from(format!("{fs_base}{suffix}"));
        is_git_directory(&fs).then(|| ResolvedRepo {
            validated: format!("{trimmed}{suffix}"),
            git_dir: fs,
        })
    })
}

/// `~user[/rest]` → `<home>/rest`; other paths unchanged.
fn expand_user(path: &str, platform: &dyn DaemonPlatform) -> Option<String> {
    let Some(rest) = path.strip_prefix('~') else {
        return Some(path.to_string());
    };
    let (user, tail) = match rest.find('/') {
        Some(i) => (&rest[..i], &rest[i..]),
        None => (rest, ""),
    };
    let home = platform.home_dir(user)?;
    Some(format!("{}{tail}", home.to_string_lossy()))
}

/// Minimal git-directory test: `HEAD`, `objects/` and `refs/`.
pub(super) fn is_git_directory(dir: &Path) -> bool {
    dir.join("HEAD").is_file() && dir.join("objects").is_dir() && dir.join("refs").is_dir()
}

/// Whitelist check on the validated path string (no canonicalisation, so a
/// whitelisted symlink works as in git).
pub(super) fn whitelisted(validated: &str, whitelist: &[PathBuf], strict: bool) -> bool {
    if whitelist.is_empty() {
        return true;
    }
    whitelist.iter().any(|entry| {
        let entry = entry.to_string_lossy();
        if strict {
            validated == entry
        } else {
            validated.starts_with(entry.as_ref())
                && matches!(validated.as_bytes().get(entry.len()), None | Some(b'/'))
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::daemon::platform::HostInfo;
    use std::sync::Arc;
    use tempfile::TempDir;

    struct FakePlatform(PathBuf);
    impl DaemonPlatform for FakePlatform {
        fn home_dir(&self, user: &str) -> Option<PathBuf> {
            (user == "alice").then(|| self.0.clone())
        }
        fn lookup_host(&self, _host: &str) -> Option<HostInfo> {
            None
        }
    }

    fn bare(at: &Path) {
        std::fs::create_dir_all(at.join("objects")).unwrap();
        std::fs::create_dir_all(at.join("refs")).unwrap();
        std::fs::write(at.join("HEAD"), b"ref: refs/heads/main\n").unwrap();
    }

    fn opts(root: &Path) -> DaemonOptions {
        DaemonOptions {
            log: DaemonLog::memory(true),
            platform: Arc::new(FakePlatform(root.join("home"))),
            ..DaemonOptions::default()
        }
    }

    fn s(p: &Path) -> String {
        p.to_string_lossy().to_string()
    }

    #[test]
    fn alias_rules_match_stock_git_daemon() {
        for ok in ["/pub.git", "/pub.git/", "/", "~root/x", "/.pub.git"] {
            assert!(!is_aliased(ok), "{ok}");
        }
        for bad in [
            "pub.git",
            "/pub.git/.",
            "/./pub.git",
            "/pub.git//",
            "//pub.git",
            "~root/../x",
            "~/x",
            "/pub.git/..",
            "~root/./x",
            "/../srv/pub.git",
        ] {
            assert!(is_aliased(bad), "{bad}");
        }
    }

    #[test]
    fn interpolation_expands_known_placeholders_only() {
        let host = HostContext {
            hostname: "localhost".into(),
            canonical: "localhost".into(),
            ip: "127.0.0.1".into(),
            port: "77".into(),
        };
        assert_eq!(
            interpolate("/v/%CH/%IP/%P/%%/%D/%X", &host, "/a.git"),
            "/v/localhost/127.0.0.1/77/%//a.git/%X"
        );
        assert_eq!(
            interpolate("/v/%H%D", &host, "/a.git"),
            "/v/localhost/a.git"
        );
    }

    #[test]
    fn base_path_and_suffixes() {
        let root = TempDir::new().unwrap();
        bare(&root.path().join("pub.git"));
        bare(&root.path().join("work/.git"));
        let mut o = opts(root.path());
        o.base_path = Some(root.path().to_path_buf());
        let r = resolve_request(&o, "/pub", None).unwrap();
        assert_eq!(r.validated, format!("{}/pub.git", s(root.path())));
        let r = resolve_request(&o, "/work/", None).unwrap();
        assert_eq!(r.validated, format!("{}/work/.git", s(root.path())));
        assert!(resolve_request(&o, "pub.git", None).is_none());
        assert!(resolve_request(&o, "/nope.git", None).is_none());
        let lines = o.log.lines();
        assert!(
            lines.contains(&"'pub.git': aliased".to_string()),
            "{lines:?}"
        );
        assert!(
            lines
                .iter()
                .any(|l| l.ends_with("/nope.git' does not appear to be a git repository"))
        );
    }

    #[test]
    fn base_path_relaxed_retries_raw_path() {
        let root = TempDir::new().unwrap();
        bare(&root.path().join("pub.git"));
        let abs = format!("{}/pub.git", s(root.path()));
        let mut o = opts(root.path());
        o.base_path = Some(root.path().to_path_buf());
        assert!(resolve_request(&o, &abs, None).is_none());
        o.base_path_relaxed = true;
        assert_eq!(resolve_request(&o, &abs, None).unwrap().validated, abs);
        assert!(resolve_request(&o, "/pub.git", None).is_some());
    }

    #[test]
    fn strict_paths_need_exact_names() {
        let root = TempDir::new().unwrap();
        bare(&root.path().join("pub.git"));
        let abs = format!("{}/pub.git", s(root.path()));
        let mut o = opts(root.path());
        o.strict_paths = true;
        o.whitelist = vec![PathBuf::from(&abs)];
        assert!(resolve_request(&o, &abs, None).is_some());
        assert!(resolve_request(&o, &format!("{}/pub", s(root.path())), None).is_none());
        assert!(resolve_request(&o, &format!("{abs}/"), None).is_none());
        assert!(
            o.log
                .lines()
                .iter()
                .any(|l| l.ends_with("/pub.git/': not in directory list"))
        );
    }

    #[test]
    fn whitelist_prefix_needs_slash_boundary() {
        let wl = vec![PathBuf::from("/srv/work"), PathBuf::from("/srv/pub.git")];
        assert!(whitelisted("/srv/work/.git", &wl, false));
        assert!(whitelisted("/srv/pub.git", &wl, false));
        assert!(!whitelisted("/srv/workshop.git", &wl, false));
        // A trailing slash in the whitelist entry needs a second slash (observed).
        assert!(!whitelisted(
            "/srv/work/.git",
            &[PathBuf::from("/srv/work/")],
            false
        ));
        assert!(!whitelisted("/srv/work/.git", &wl, true));
        assert!(whitelisted("/x", &[], true));
    }

    #[test]
    fn user_path_expands_home_and_keeps_validated_tilde() {
        let root = TempDir::new().unwrap();
        bare(&root.path().join("home/pubrepo.git"));
        bare(&root.path().join("home/public_git/r.git"));
        let mut o = opts(root.path());
        assert!(resolve_request(&o, "~alice/pubrepo", None).is_none());
        assert!(
            o.log
                .lines()
                .contains(&"'~alice/pubrepo': User-path not allowed".to_string())
        );
        o.user_path = Some(String::new());
        let r = resolve_request(&o, "~alice/pubrepo", None).unwrap();
        assert_eq!(r.validated, "~alice/pubrepo.git");
        assert_eq!(r.git_dir, root.path().join("home/pubrepo.git"));
        assert!(resolve_request(&o, "~bob/x", None).is_none());
        o.user_path = Some("public_git".into());
        let r = resolve_request(&o, "~alice/r.git", None).unwrap();
        assert_eq!(r.validated, "~alice/public_git/r.git");
        assert!(resolve_request(&o, "~alice/pubrepo.git", None).is_none());
        // Whitelist sees the unexpanded `~user` form (observed with stock git).
        o.whitelist = vec![root.path().join("home")];
        assert!(resolve_request(&o, "~alice/r.git", None).is_none());
    }

    #[test]
    fn interpolated_path_needs_host_and_beats_base_path() {
        let root = TempDir::new().unwrap();
        bare(&root.path().join("vhost/localhost/a.git"));
        bare(&root.path().join("pub.git"));
        let mut o = opts(root.path());
        o.base_path = Some(root.path().to_path_buf());
        o.interpolated_path = Some(format!("{}/vhost/%H%D", s(root.path())));
        let host = HostContext {
            hostname: "localhost".into(),
            ..HostContext::default()
        };
        assert!(resolve_request(&o, "/a.git", Some(&host)).is_some());
        // Without host= the base path applies.
        assert!(resolve_request(&o, "/pub.git", None).is_some());
        assert!(resolve_request(&o, "/pub.git", Some(&host)).is_none());
        assert!(
            o.log
                .lines()
                .iter()
                .any(|l| l.starts_with("Interpolated dir '")
                    && l.ends_with("/vhost/localhost/a.git'"))
        );
    }
}
