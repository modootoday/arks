//! `git daemon` option model: services, per-service policy and daemon options.
//!
//! Primary: <https://git-scm.com/docs/git-daemon> (OPTIONS, SERVICES).

use super::log::DaemonLog;
use super::platform::{DaemonPlatform, StdPlatform};
use std::fmt;
use std::path::PathBuf;
use std::sync::Arc;

/// A `git://` service the daemon can run.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DaemonService {
    UploadPack,
    UploadArchive,
    ReceivePack,
}

impl DaemonService {
    pub const ALL: [Self; 3] = [Self::UploadPack, Self::UploadArchive, Self::ReceivePack];

    /// Service name used on the command line (`--enable=<service>`).
    pub fn name(self) -> &'static str {
        match self {
            Self::UploadPack => "upload-pack",
            Self::UploadArchive => "upload-archive",
            Self::ReceivePack => "receive-pack",
        }
    }

    /// Per-repository override key (`daemon.<service>` in the repository config).
    pub fn config_key(self) -> &'static str {
        match self {
            Self::UploadPack => "daemon.uploadpack",
            Self::UploadArchive => "daemon.uploadarch",
            Self::ReceivePack => "daemon.receivepack",
        }
    }

    pub fn from_name(name: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|s| s.name() == name)
    }

    /// Service named by a request line command (`git-upload-pack`).
    pub fn from_request(command: &str) -> Option<Self> {
        command.strip_prefix("git-").and_then(Self::from_name)
    }
}

/// Site-wide default for one service.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ServicePolicy {
    /// `--enable` / `--disable`.
    pub enabled: bool,
    /// `--allow-override` / `--forbid-override` (per-repository `daemon.<service>`).
    pub overridable: bool,
}

/// Site-wide policy of every service (git defaults: only upload-pack enabled,
/// every service overridable).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ServiceTable {
    pub upload_pack: ServicePolicy,
    pub upload_archive: ServicePolicy,
    pub receive_pack: ServicePolicy,
}

impl Default for ServiceTable {
    fn default() -> Self {
        let on = ServicePolicy {
            enabled: true,
            overridable: true,
        };
        let off = ServicePolicy {
            enabled: false,
            overridable: true,
        };
        Self {
            upload_pack: on,
            upload_archive: off,
            receive_pack: off,
        }
    }
}

impl ServiceTable {
    pub fn get(&self, service: DaemonService) -> ServicePolicy {
        match service {
            DaemonService::UploadPack => self.upload_pack,
            DaemonService::UploadArchive => self.upload_archive,
            DaemonService::ReceivePack => self.receive_pack,
        }
    }

    pub fn get_mut(&mut self, service: DaemonService) -> &mut ServicePolicy {
        match service {
            DaemonService::UploadPack => &mut self.upload_pack,
            DaemonService::UploadArchive => &mut self.upload_archive,
            DaemonService::ReceivePack => &mut self.receive_pack,
        }
    }
}

/// Request-serving options of `git daemon` (everything except socket setup and
/// process control, which the CLI owns).
#[derive(Clone)]
pub struct DaemonOptions {
    /// `--base-path=<path>`: prefix for every absolute request path.
    pub base_path: Option<PathBuf>,
    /// `--base-path-relaxed`: retry the raw request path when the prefixed one fails.
    pub base_path_relaxed: bool,
    /// `<directory>...`: whitelist compared with the validated repository path.
    pub whitelist: Vec<PathBuf>,
    /// `--strict-paths`: no suffix guessing; the whitelist must match exactly.
    pub strict_paths: bool,
    /// `--user-path` (`Some("")`) or `--user-path=<path>` (`Some(path)`).
    pub user_path: Option<String>,
    /// `--interpolated-path=<pathtemplate>`.
    pub interpolated_path: Option<String>,
    /// `--export-all`: skip the `git-daemon-export-ok` check.
    pub export_all: bool,
    /// `--enable`/`--disable`/`--allow-override`/`--forbid-override`.
    pub services: ServiceTable,
    /// `--[no-]informative-errors`.
    pub informative_errors: bool,
    /// `--access-hook=<path>`.
    pub access_hook: Option<PathBuf>,
    /// `--timeout=<n>` seconds for pack sub-requests (0 = none).
    pub timeout_secs: u64,
    /// `--init-timeout=<n>` seconds until the request line arrives (0 = none).
    pub init_timeout_secs: u64,
    /// `--max-connections=<n>` (0 = unlimited; git default 32).
    pub max_connections: usize,
    /// Log destination and `--verbose`.
    pub log: DaemonLog,
    /// OS lookups (home directories, host names).
    pub platform: Arc<dyn DaemonPlatform>,
}

impl Default for DaemonOptions {
    fn default() -> Self {
        Self {
            base_path: None,
            base_path_relaxed: false,
            whitelist: Vec::new(),
            strict_paths: false,
            user_path: None,
            interpolated_path: None,
            export_all: false,
            services: ServiceTable::default(),
            informative_errors: false,
            access_hook: None,
            timeout_secs: 0,
            init_timeout_secs: 0,
            max_connections: 32,
            log: DaemonLog::default(),
            platform: Arc::new(StdPlatform),
        }
    }
}

impl fmt::Debug for DaemonOptions {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("DaemonOptions")
            .field("base_path", &self.base_path)
            .field("base_path_relaxed", &self.base_path_relaxed)
            .field("whitelist", &self.whitelist)
            .field("strict_paths", &self.strict_paths)
            .field("user_path", &self.user_path)
            .field("interpolated_path", &self.interpolated_path)
            .field("export_all", &self.export_all)
            .field("services", &self.services)
            .field("informative_errors", &self.informative_errors)
            .field("access_hook", &self.access_hook)
            .field("timeout_secs", &self.timeout_secs)
            .field("init_timeout_secs", &self.init_timeout_secs)
            .field("max_connections", &self.max_connections)
            .field("log", &self.log)
            .finish_non_exhaustive()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn service_names_and_config_keys_follow_git_daemon() {
        assert_eq!(
            DaemonService::from_request("git-upload-pack"),
            Some(DaemonService::UploadPack)
        );
        assert_eq!(
            DaemonService::from_request("git-upload-archive"),
            Some(DaemonService::UploadArchive)
        );
        assert_eq!(DaemonService::from_request("git-foo"), None);
        assert_eq!(DaemonService::from_request("upload-pack"), None);
        assert_eq!(
            DaemonService::UploadArchive.config_key(),
            "daemon.uploadarch"
        );
        assert_eq!(
            DaemonService::ReceivePack.config_key(),
            "daemon.receivepack"
        );
        assert_eq!(DaemonService::from_name("bogus"), None);
    }

    #[test]
    fn defaults_match_git_daemon() {
        let t = ServiceTable::default();
        assert!(t.upload_pack.enabled);
        assert!(!t.upload_archive.enabled);
        assert!(!t.receive_pack.enabled);
        assert!(DaemonService::ALL.iter().all(|s| t.get(*s).overridable));
        let o = DaemonOptions::default();
        assert_eq!(o.max_connections, 32);
        assert_eq!(o.timeout_secs, 0);
        assert_eq!(o.init_timeout_secs, 0);
        assert!(!o.informative_errors);
    }
}
