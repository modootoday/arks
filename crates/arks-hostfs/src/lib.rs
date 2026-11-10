//! Host filesystem honesty helpers for cloud-sync / placeholder / LSM hazards.
//!
//! - **FS-007** DriveFS / sync-root **detect + advisory** (+ optional refuse flag)
//! - **WIN-003** OneDrive Cloud Filter placeholder **detect + honesty**
//! - **FS-014** SELinux / AppArmor **detect + errno honesty** (not host-policy bypass)
//!
//! Cite: `.spec/research/20261110222000-remaining-gaps-ffi-external-unblock.research.md`
//!       `.spec/research/20261111001816-partial-completion-unblock-roadmap.research.md`
//!       `.spec/evidence/20261110221919-fs007-gdrive-host-advisory`

// Windows CfAPI / volume probes require thin `unsafe` FFI via the `windows` crate.
#![cfg_attr(not(windows), forbid(unsafe_code))]

use std::path::{Path, PathBuf};

/// Kind of cloud / sync substrate detected under a path.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CloudKind {
    /// Google Drive for desktop virtual volume (Drive File Stream).
    GoogleDriveFs,
    /// Microsoft OneDrive Files On-Demand / Cloud Filter API.
    OneDriveCfApi,
    /// Dropbox sync folder marker.
    Dropbox,
    /// Syncthing folder marker.
    Syncthing,
    /// Nextcloud / ownCloud desktop sync journal marker (advisory FS-007).
    Nextcloud,
}

impl CloudKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::GoogleDriveFs => "google-drive-fs",
            Self::OneDriveCfApi => "onedrive-cfapi",
            Self::Dropbox => "dropbox",
            Self::Syncthing => "syncthing",
            Self::Nextcloud => "nextcloud",
        }
    }

    pub fn catalog_id(self) -> &'static str {
        match self {
            Self::GoogleDriveFs | Self::Dropbox | Self::Syncthing | Self::Nextcloud => "FS-007",
            Self::OneDriveCfApi => "WIN-003",
        }
    }
}

/// Detection result — always **advisory** unless a future SoT Claimed slice says otherwise.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CloudDetection {
    pub kind: CloudKind,
    pub root: PathBuf,
    pub detail: String,
    /// Specfold honesty: detections are advisory; FS-007 full sync Claimed is not earned here.
    pub claimed: bool,
}

impl CloudDetection {
    pub fn advisory_note(&self) -> String {
        format!(
            "note: {} — detected {} at {} ({}); advisory only — do not Claim multi-device cloud-sync safety; prefer local disk for .git writers",
            self.kind.catalog_id(),
            self.kind.as_str(),
            self.root.display(),
            self.detail
        )
    }
}

/// Probe whether `path` lives under a known cloud-sync substrate.
pub fn detect_cloud_sync(path: &Path) -> Option<CloudDetection> {
    let abs = canonicalize_lossy(path);
    if let Some(d) = detect_drivefs(&abs) {
        return Some(d);
    }
    if let Some(d) = detect_onedrive_root(&abs) {
        return Some(d);
    }
    if let Some(d) = detect_marker_root(&abs, ".dropbox", CloudKind::Dropbox) {
        return Some(d);
    }
    if let Some(d) = detect_marker_root(&abs, ".stfolder", CloudKind::Syncthing) {
        return Some(d);
    }
    if let Some(d) = detect_marker_root(&abs, ".syncthing", CloudKind::Syncthing) {
        return Some(d);
    }
    // Nextcloud/ownCloud desktop clients leave a sync journal beside the tree.
    if let Some(d) = detect_marker_root(&abs, ".csync_journal.db", CloudKind::Nextcloud) {
        return Some(d);
    }
    if let Some(d) = detect_marker_root(&abs, ".owncloudsync.log", CloudKind::Nextcloud) {
        return Some(d);
    }
    None
}

/// WIN-003: is this path a Cloud Filter / OneDrive on-demand placeholder?
///
/// Non-Windows: always `Ok(false)`.
/// Windows without CfAPI attrs: `Ok(false)`.
pub fn is_onedrive_placeholder(path: &Path) -> Result<bool, HostFsError> {
    #[cfg(windows)]
    {
        windows_impl::is_recall_on_data_access(path)
    }
    #[cfg(not(windows))]
    {
        let _ = path;
        Ok(false)
    }
}

/// Host LSM / MAC policy substrate (FS-014) — detect only.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LsmKind {
    SelinuxEnforcing,
    SelinuxPermissive,
    ApparmorEnabled,
}

impl LsmKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::SelinuxEnforcing => "selinux-enforcing",
            Self::SelinuxPermissive => "selinux-permissive",
            Self::ApparmorEnabled => "apparmor-enabled",
        }
    }

    pub fn catalog_id(self) -> &'static str {
        "FS-014"
    }
}

/// Detect SELinux / AppArmor presence via sysfs (Linux). Non-Linux → empty.
pub fn detect_lsm() -> Vec<LsmKind> {
    let mut out = Vec::new();
    if let Ok(v) = std::fs::read_to_string("/sys/fs/selinux/enforce") {
        match v.trim() {
            "1" => out.push(LsmKind::SelinuxEnforcing),
            "0" => out.push(LsmKind::SelinuxPermissive),
            _ => {}
        }
    }
    if let Ok(v) = std::fs::read_to_string("/sys/module/apparmor/parameters/enabled") {
        if v.trim().eq_ignore_ascii_case("Y") {
            out.push(LsmKind::ApparmorEnabled);
        }
    }
    out
}

/// Env / config opt-in: refuse writers when `.git` is under a sync root.
///
/// Claimed slice = **detect + optional refuse**. Full multi-device sync safety
/// remains Impossible. Set `ARKS_HOST_ENV_REFUSE_SYNC=1` or pass CLI flag.
pub fn refuse_sync_env_requested() -> bool {
    matches!(
        std::env::var("ARKS_HOST_ENV_REFUSE_SYNC").as_deref(),
        Ok("1") | Ok("true") | Ok("TRUE") | Ok("yes") | Ok("YES")
    )
}

/// True when `path` (or its `.git`) sits under a detected cloud-sync substrate.
pub fn git_under_sync_root(path: &Path) -> Option<CloudDetection> {
    if let Some(d) = detect_cloud_sync(path) {
        return Some(d);
    }
    let git = path.join(".git");
    if git.exists() {
        return detect_cloud_sync(&git);
    }
    None
}

/// Combined honesty probe for a repo path (work tree or `.git`).
pub fn probe_repo_path(path: &Path) -> ProbeReport {
    let cloud = detect_cloud_sync(path).or_else(|| git_under_sync_root(path));
    let placeholder = is_onedrive_placeholder(path).unwrap_or(false);
    let lsm = detect_lsm();
    ProbeReport {
        path: path.to_path_buf(),
        cloud,
        onedrive_placeholder: placeholder,
        lsm,
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProbeReport {
    pub path: PathBuf,
    pub cloud: Option<CloudDetection>,
    pub onedrive_placeholder: bool,
    pub lsm: Vec<LsmKind>,
}

impl ProbeReport {
    pub fn notes(&self) -> Vec<String> {
        let mut out = Vec::new();
        if let Some(c) = &self.cloud {
            out.push(c.advisory_note());
        }
        if self.onedrive_placeholder {
            out.push(format!(
                "note: WIN-003 — OneDrive/CfAPI placeholder at {} (RECALL_ON_DATA_ACCESS); hydrate before relying on lstat/checkout; detect-only (not Claimed hydration); DriveFS G: ≠ WIN-003",
                self.path.display()
            ));
            out.push(
                "note: WIN-003 — on hydrate fail expect clear PermissionDenied/ERROR_CLOUD_FILE_* honesty; arks does not implement Files On-Demand hydration"
                    .into(),
            );
        }
        for l in &self.lsm {
            out.push(format!(
                "note: {} — detected {} on host; .lock rename EACCES/EPERM → clear PermissionDenied with LSM name; cannot bypass host policy inside arks",
                l.catalog_id(),
                l.as_str()
            ));
        }
        out
    }

    pub fn is_advisory_hazard(&self) -> bool {
        self.cloud.is_some() || self.onedrive_placeholder
    }

    /// Opt-in refuse when cloud-sync hazard is present (FS-007 Claimed detect+refuse slice).
    pub fn should_refuse_sync(&self, refuse_requested: bool) -> bool {
        refuse_requested && self.cloud.is_some()
    }

    pub fn errno_honesty_hint(&self) -> Option<String> {
        if self.onedrive_placeholder {
            return Some(
                "WIN-003: placeholder I/O may surface PermissionDenied / cloud-file errno — hydrate externally; not Claimed"
                    .into(),
            );
        }
        if !self.lsm.is_empty() {
            let names: Vec<&str> = self.lsm.iter().map(|l| l.as_str()).collect();
            return Some(format!(
                "FS-014: PermissionDenied on .lock rename may be host LSM ({})",
                names.join(",")
            ));
        }
        None
    }
}

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum HostFsError {
    #[error("I/O: {0}")]
    Io(String),
}

fn canonicalize_lossy(path: &Path) -> PathBuf {
    path.canonicalize().unwrap_or_else(|_| path.to_path_buf())
}

fn detect_drivefs(abs: &Path) -> Option<CloudDetection> {
    #[cfg(windows)]
    {
        windows_impl::detect_google_drive_fs(abs)
    }
    #[cfg(not(windows))]
    {
        let _ = abs;
        None
    }
}

fn detect_onedrive_root(abs: &Path) -> Option<CloudDetection> {
    // Path heuristic portable across OS; CfAPI placeholder check is separate.
    for comp in abs.components() {
        let s = comp.as_os_str().to_string_lossy();
        if s.eq_ignore_ascii_case("OneDrive") || s.to_ascii_lowercase().starts_with("onedrive -") {
            return Some(CloudDetection {
                kind: CloudKind::OneDriveCfApi,
                root: abs.to_path_buf(),
                detail: "path component matches OneDrive".into(),
                claimed: false,
            });
        }
    }
    None
}

fn detect_marker_root(abs: &Path, marker: &str, kind: CloudKind) -> Option<CloudDetection> {
    let mut cur = Some(abs);
    while let Some(dir) = cur {
        if dir.join(marker).exists() {
            return Some(CloudDetection {
                kind,
                root: dir.to_path_buf(),
                detail: format!("found marker `{marker}`"),
                claimed: false,
            });
        }
        cur = dir.parent();
    }
    None
}

#[cfg(windows)]
mod windows_impl {
    use super::*;
    use std::os::windows::ffi::OsStrExt;
    use windows::Win32::Storage::FileSystem::{
        FILE_ATTRIBUTE_RECALL_ON_DATA_ACCESS, GetFileAttributesW,
    };
    use windows::core::PCWSTR;

    pub(crate) fn is_recall_on_data_access(path: &Path) -> Result<bool, HostFsError> {
        let wide: Vec<u16> = path
            .as_os_str()
            .encode_wide()
            .chain(std::iter::once(0))
            .collect();
        // SAFETY: path is NUL-terminated wide string for GetFileAttributesW.
        let attrs = unsafe { GetFileAttributesW(PCWSTR(wide.as_ptr())) };
        if attrs == u32::MAX {
            return Err(HostFsError::Io(format!(
                "GetFileAttributesW failed for {}",
                path.display()
            )));
        }
        Ok((attrs & FILE_ATTRIBUTE_RECALL_ON_DATA_ACCESS.0) != 0)
    }

    pub(crate) fn detect_google_drive_fs(abs: &Path) -> Option<CloudDetection> {
        // Only flag volumes whose label is "Google Drive" (DriveFS virtual letter).
        // Do NOT treat "DriveFS installed" as meaning every fixed disk is cloud-synced.
        if !is_under_drive_letter(abs) {
            return None;
        }
        let root = volume_root(abs);
        let vol_name = volume_label(&root).unwrap_or_default();
        if !vol_name.eq_ignore_ascii_case("Google Drive") {
            return None;
        }
        let drivefs_alive = google_drive_fs_install_present();
        Some(CloudDetection {
            kind: CloudKind::GoogleDriveFs,
            root,
            detail: format!(
                "volume_label={vol_name:?}; DriveFS_install={drivefs_alive}; advisory streaming/mirror substrate (≠ Claimed FS-007)"
            ),
            claimed: false,
        })
    }

    fn is_under_drive_letter(abs: &Path) -> bool {
        abs.components()
            .next()
            .map(|c| {
                let s = c.as_os_str().to_string_lossy();
                s.as_bytes().get(1) == Some(&b':')
            })
            .unwrap_or(false)
    }

    fn volume_root(abs: &Path) -> PathBuf {
        abs.components()
            .next()
            .map(|c| {
                PathBuf::from(format!(
                    "{}\\",
                    c.as_os_str().to_string_lossy().trim_end_matches('\\')
                ))
            })
            .unwrap_or_else(|| PathBuf::from("\\"))
    }

    fn volume_label(root: &Path) -> Option<String> {
        use windows::Win32::Storage::FileSystem::GetVolumeInformationW;
        let wide: Vec<u16> = root
            .as_os_str()
            .encode_wide()
            .chain(std::iter::once(0))
            .collect();
        let mut name = [0u16; 64];
        let ok = unsafe {
            GetVolumeInformationW(
                PCWSTR(wide.as_ptr()),
                Some(&mut name),
                None,
                None,
                None,
                None,
            )
        };
        if ok.is_err() {
            return None;
        }
        let nul = name.iter().position(|&c| c == 0).unwrap_or(name.len());
        Some(String::from_utf16_lossy(&name[..nul]))
    }

    fn google_drive_fs_install_present() -> bool {
        Path::new(r"C:\Program Files\Google\Drive File Stream").is_dir()
            || std::env::var_os("LOCALAPPDATA")
                .map(|p| PathBuf::from(p).join("Google").join("DriveFS").is_dir())
                .unwrap_or(false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dropbox_marker_detection() {
        let dir = tempfile_dir();
        std::fs::create_dir_all(dir.join(".dropbox")).unwrap();
        let child = dir.join("proj").join("file.txt");
        std::fs::create_dir_all(child.parent().unwrap()).unwrap();
        std::fs::write(&child, b"x").unwrap();
        let d = detect_cloud_sync(&child).expect("dropbox");
        assert_eq!(d.kind, CloudKind::Dropbox);
        assert!(!d.claimed);
        assert!(d.advisory_note().contains("FS-007"));
    }

    #[test]
    fn syncthing_marker_detection() {
        let dir = tempfile_dir();
        std::fs::create_dir_all(dir.join(".stfolder")).unwrap();
        let d = detect_cloud_sync(&dir.join("a")).expect("syncthing");
        assert_eq!(d.kind, CloudKind::Syncthing);
    }

    #[test]
    fn nextcloud_journal_marker_detection() {
        let dir = tempfile_dir();
        std::fs::write(dir.join(".csync_journal.db"), b"x").unwrap();
        let d = detect_cloud_sync(&dir.join("proj")).expect("nextcloud");
        assert_eq!(d.kind, CloudKind::Nextcloud);
        assert!(!d.claimed);
        assert!(d.advisory_note().contains("FS-007"));
    }

    #[test]
    fn onedrive_path_component() {
        let dir = tempfile_dir();
        let od = dir.join("OneDrive").join("Documents");
        std::fs::create_dir_all(&od).unwrap();
        let d = detect_cloud_sync(&od).expect("onedrive path");
        assert_eq!(d.kind, CloudKind::OneDriveCfApi);
        assert!(d.advisory_note().contains("WIN-003"));
    }

    #[test]
    fn placeholder_false_on_normal_temp() {
        let dir = tempfile_dir();
        let f = dir.join("normal.txt");
        std::fs::write(&f, b"ok").unwrap();
        assert!(!is_onedrive_placeholder(&f).unwrap());
    }

    #[test]
    fn probe_report_notes_empty_on_local() {
        let dir = tempfile_dir();
        let r = probe_repo_path(&dir);
        assert!(!r.is_advisory_hazard());
        // LSM notes may appear on confined Linux hosts; cloud notes must be empty.
        assert!(r.cloud.is_none());
        assert!(!r.onedrive_placeholder);
    }

    #[test]
    fn refuse_sync_only_when_requested_and_cloud() {
        let dir = tempfile_dir();
        std::fs::create_dir_all(dir.join(".dropbox")).unwrap();
        let r = probe_repo_path(&dir);
        assert!(r.is_advisory_hazard());
        assert!(!r.should_refuse_sync(false));
        assert!(r.should_refuse_sync(true));
    }

    #[test]
    fn lsm_detect_does_not_panic() {
        let _ = detect_lsm();
    }

    fn tempfile_dir() -> PathBuf {
        let mut d = std::env::temp_dir();
        d.push(format!("arks-hostfs-{}", std::process::id()));
        d.push(format!("{}", uniq()));
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    fn uniq() -> u64 {
        use std::sync::atomic::{AtomicU64, Ordering};
        static N: AtomicU64 = AtomicU64::new(1);
        N.fetch_add(1, Ordering::Relaxed)
    }
}
