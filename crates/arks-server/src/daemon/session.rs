//! One `git://` connection: request checks in `git daemon` order, then the
//! service itself.
//!
//! Primary: <https://git-scm.com/docs/git-daemon> (SERVICES, `--informative-errors`,
//! `--access-hook`, per-repository `daemon.<service>` overrides). Check order and
//! error wording follow observed stock `git daemon`.

use super::access_hook::{HookRequest, run_access_hook};
use super::archive::{drain_upload_archive_args, serve_upload_archive_partial};
use super::options::{DaemonOptions, DaemonService};
use super::path::{HostContext, ResolvedRepo, resolve_request};
use super::request::ServiceRequest;
use crate::error::ServerError;
use crate::stdio::{receive_pack_stream, upload_pack_stream};
use arks_config::ConfigSet;
use arks_protocol::{Packet, encode_packet};
use arks_repo::Repository;
use std::io::{Read, Write};
use std::net::SocketAddr;

/// Generic refusal sent without `--informative-errors`.
const ACCESS_DENIED: &str = "access denied or repository not exported";

/// What happened to one request.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Outcome {
    /// The service ran to completion.
    Served,
    /// The request was refused (or the service failed).
    Refused,
}

/// Run every check, then the service, on `stream`.
pub(super) fn handle_request<S: Read + Write>(
    stream: &mut S,
    req: &ServiceRequest,
    opts: &DaemonOptions,
    peer: Option<SocketAddr>,
) -> Result<Outcome, ServerError> {
    let log = &opts.log;
    let Some(service) = DaemonService::from_request(&req.command).filter(|_| !req.path.is_empty())
    else {
        log.error(&format!("Protocol error: '{}'", req.line));
        return Ok(Outcome::Refused);
    };
    let dir = req.path.as_str();
    log.info(&format!("Request {} for '{dir}'", service.name()));
    let policy = opts.services.get(service);
    if !policy.enabled && !policy.overridable {
        log.error(&format!("'{}': service not enabled.", service.name()));
        return refuse(stream, opts, dir, "service not enabled");
    }
    let host = host_context(req, opts);
    let Some(repo) = resolve_request(opts, dir, host.as_ref()) else {
        return refuse(stream, opts, dir, "no such repository");
    };
    if !opts.export_all && !repo.git_dir.join("git-daemon-export-ok").exists() {
        log.error(&format!("'{}': repository not exported.", repo.validated));
        return refuse(stream, opts, dir, "repository not exported");
    }
    let mut enabled = policy.enabled;
    if policy.overridable {
        if let Some(v) = ConfigSet::load(Some(&repo.git_dir)).get_bool(service.config_key()) {
            enabled = v;
        }
    }
    if !enabled {
        log.error(&format!(
            "'{}': service not enabled for '{}'",
            service.name(),
            repo.validated
        ));
        return refuse(stream, opts, dir, "service not enabled");
    }
    if let Some(hook) = opts.access_hook.as_deref() {
        let hook_req = HookRequest {
            service: service.name(),
            validated: &repo.validated,
            git_dir: &repo.git_dir,
            host: host.as_ref(),
            peer,
        };
        if let Err(msg) = run_access_hook(hook, &hook_req) {
            let msg = msg.unwrap_or_else(|| "service rejected".to_string());
            return refuse(stream, opts, dir, &msg);
        }
    }
    run_service(stream, service, &repo, opts)
}

fn host_context(req: &ServiceRequest, opts: &DaemonOptions) -> Option<HostContext> {
    let attr = req.host.as_ref()?;
    let info = opts
        .platform
        .lookup_host(&attr.hostname)
        .unwrap_or_default();
    Some(HostContext {
        hostname: attr.hostname.clone(),
        canonical: info.canonical,
        ip: info.ip,
        port: attr.port.clone(),
    })
}

fn run_service<S: Read + Write>(
    stream: &mut S,
    service: DaemonService,
    repo: &ResolvedRepo,
    opts: &DaemonOptions,
) -> Result<Outcome, ServerError> {
    let mut repository = match Repository::open(&repo.git_dir) {
        Ok(r) => r,
        Err(e) => {
            opts.log.error(&format!("fatal: {e}"));
            return refuse(stream, opts, &repo.validated, "no such repository");
        }
    };
    let result = match service {
        DaemonService::UploadPack => upload_pack_stream(&repository, stream),
        DaemonService::ReceivePack => receive_pack_stream(&mut repository, stream),
        DaemonService::UploadArchive => drain_upload_archive_args(stream)
            .and_then(|args| serve_upload_archive_partial(&repository, &args, stream, false)),
    };
    match result {
        Ok(()) => Ok(Outcome::Served),
        Err(e) => {
            opts.log.error(&format!("fatal: {e}"));
            Ok(Outcome::Refused)
        }
    }
}

/// Send `ERR <msg>: <dir>` (generic message unless `--informative-errors`).
fn refuse<W: Write>(
    stream: &mut W,
    opts: &DaemonOptions,
    dir: &str,
    msg: &str,
) -> Result<Outcome, ServerError> {
    let msg = if opts.informative_errors {
        msg
    } else {
        ACCESS_DENIED
    };
    let pkt = encode_packet(&Packet::Data(format!("ERR {msg}: {dir}").into_bytes()))
        .map_err(|e| ServerError::Protocol(e.to_string()))?;
    stream.write_all(&pkt)?;
    stream.flush()?;
    Ok(Outcome::Refused)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::daemon::log::DaemonLog;
    use crate::daemon::request::parse_service_payload;
    use std::io::Cursor;
    use std::path::Path;
    use tempfile::TempDir;

    /// In-memory duplex: reads `input`, collects writes.
    struct Duplex {
        input: Cursor<Vec<u8>>,
        output: Vec<u8>,
    }
    impl Read for Duplex {
        fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
            self.input.read(buf)
        }
    }
    impl Write for Duplex {
        fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
            self.output.extend_from_slice(buf);
            Ok(buf.len())
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }

    fn bare(at: &Path, exported: bool) {
        std::fs::create_dir_all(at.join("objects")).unwrap();
        std::fs::create_dir_all(at.join("refs/heads")).unwrap();
        std::fs::write(at.join("HEAD"), b"ref: refs/heads/main\n").unwrap();
        std::fs::write(at.join("config"), b"[core]\n\tbare = true\n").unwrap();
        if exported {
            std::fs::write(at.join("git-daemon-export-ok"), b"").unwrap();
        }
    }

    fn ask(opts: &DaemonOptions, payload: &[u8]) -> (Outcome, String) {
        let req = parse_service_payload(payload, &opts.log).unwrap();
        let mut io = Duplex {
            input: Cursor::new(b"0000".to_vec()),
            output: Vec::new(),
        };
        let out = handle_request(&mut io, &req, opts, None).unwrap();
        (out, String::from_utf8_lossy(&io.output).to_string())
    }

    fn setup() -> (TempDir, DaemonOptions) {
        let root = TempDir::new().unwrap();
        bare(&root.path().join("pub.git"), true);
        bare(&root.path().join("hidden.git"), false);
        let noup = root.path().join("noup.git");
        bare(&noup, true);
        std::fs::write(
            noup.join("config"),
            b"[core]\n\tbare = true\n[daemon]\n\tuploadpack = false\n",
        )
        .unwrap();
        let arch = root.path().join("arch.git");
        bare(&arch, true);
        std::fs::write(arch.join("config"), b"[daemon]\n\tuploadarch\n").unwrap();
        let opts = DaemonOptions {
            base_path: Some(root.path().to_path_buf()),
            log: DaemonLog::memory(true),
            ..DaemonOptions::default()
        };
        (root, opts)
    }

    #[test]
    fn default_errors_are_generic() {
        let (_root, opts) = setup();
        for (payload, dir) in [
            (&b"git-upload-pack /nope.git\0"[..], "/nope.git"),
            (b"git-upload-pack /hidden.git\0", "/hidden.git"),
            (b"git-receive-pack /pub.git\0", "/pub.git"),
            (b"git-upload-pack /noup.git\0", "/noup.git"),
        ] {
            let (out, text) = ask(&opts, payload);
            assert_eq!(out, Outcome::Refused);
            let want = format!("ERR access denied or repository not exported: {dir}");
            assert_eq!(text, format!("{:04x}{want}", want.len() + 4));
        }
        let lines = opts.log.lines();
        assert!(
            lines
                .iter()
                .any(|l| l.ends_with("/hidden.git': repository not exported."))
        );
        assert!(
            lines
                .iter()
                .any(|l| l.starts_with("'receive-pack': service not enabled for '"))
        );
        assert!(
            lines
                .iter()
                .any(|l| l.starts_with("'upload-pack': service not enabled for '"))
        );
    }

    #[test]
    fn informative_errors_name_the_reason() {
        let (_root, mut opts) = setup();
        opts.informative_errors = true;
        let cases: [(&[u8], &str); 4] = [
            (
                b"git-upload-pack /nope.git\0",
                "no such repository: /nope.git",
            ),
            (
                b"git-upload-pack /hidden.git\0",
                "repository not exported: /hidden.git",
            ),
            (
                b"git-receive-pack /pub.git\0",
                "service not enabled: /pub.git",
            ),
            (b"git-upload-pack pub.git\0", "no such repository: pub.git"),
        ];
        for (payload, want) in cases {
            let (_, text) = ask(&opts, payload);
            assert!(text.ends_with(&format!("ERR {want}")), "{text}");
        }
    }

    #[test]
    fn per_repo_override_and_forbid_override() {
        let (_root, mut opts) = setup();
        opts.informative_errors = true;
        // daemon.uploadarch (bare key = true) enables upload-archive for arch.git.
        let (out, text) = ask(&opts, b"git-upload-archive /arch.git\0");
        assert_eq!(out, Outcome::Served, "{text}");
        assert!(!text.contains("service not enabled"), "{text}");
        let (_, text) = ask(&opts, b"git-upload-archive /pub.git\0");
        assert!(text.ends_with("ERR service not enabled: /pub.git"));
        // --forbid-override=upload-pack ignores daemon.uploadpack=false.
        opts.services.upload_pack.overridable = false;
        let (out, _) = ask(&opts, b"git-upload-pack /noup.git\0");
        assert_eq!(out, Outcome::Served);
        // --forbid-override=receive-pack with receive-pack disabled refuses early.
        opts.services.receive_pack.overridable = false;
        let (_, text) = ask(&opts, b"git-receive-pack /nope.git\0");
        assert!(text.ends_with("ERR service not enabled: /nope.git"));
        assert!(
            opts.log
                .lines()
                .contains(&"'receive-pack': service not enabled.".to_string())
        );
    }

    #[test]
    fn unknown_service_is_a_protocol_error_without_reply() {
        let (_root, opts) = setup();
        let (out, text) = ask(&opts, b"git-foo /pub.git\0");
        assert_eq!(out, Outcome::Refused);
        assert!(text.is_empty());
        assert!(
            opts.log
                .lines()
                .contains(&"Protocol error: 'git-foo /pub.git'".to_string())
        );
    }

    #[test]
    fn export_all_serves_unmarked_repository() {
        let (_root, mut opts) = setup();
        opts.export_all = true;
        let (out, text) = ask(&opts, b"git-upload-pack /hidden.git\0");
        assert_eq!(out, Outcome::Served, "{text}");
    }

    #[cfg(unix)]
    #[test]
    fn access_hook_decline_message_needs_informative_errors() {
        use std::os::unix::fs::PermissionsExt;
        let (root, mut opts) = setup();
        let hook = root.path().join("hook.sh");
        std::fs::write(&hook, "#!/bin/sh\necho 'nope from hook'\nexit 1\n").unwrap();
        std::fs::set_permissions(&hook, std::fs::Permissions::from_mode(0o755)).unwrap();
        opts.access_hook = Some(hook.clone());
        let (_, text) = ask(&opts, b"git-upload-pack /pub.git\0");
        assert!(text.ends_with("ERR access denied or repository not exported: /pub.git"));
        opts.informative_errors = true;
        let (_, text) = ask(&opts, b"git-upload-pack /pub.git\0");
        assert!(text.ends_with("ERR nope from hook: /pub.git"), "{text}");
        std::fs::write(&hook, "#!/bin/sh\nexit 2\n").unwrap();
        let (_, text) = ask(&opts, b"git-upload-pack /pub.git\0");
        assert!(text.ends_with("ERR service rejected: /pub.git"), "{text}");
        std::fs::write(&hook, "#!/bin/sh\nexit 0\n").unwrap();
        let (out, _) = ask(&opts, b"git-upload-pack /pub.git\0");
        assert_eq!(out, Outcome::Served);
    }
}
