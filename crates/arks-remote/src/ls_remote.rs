//! `ls-remote` — list refs from a remote advertisement without fetching objects.
//!
//! Cite: <https://git-scm.com/docs/git-ls-remote>

use crate::advertise::{Advertisement, content_type_is, parse_advertisement};
use crate::dumb::{looks_like_dumb_info_refs, parse_dumb_info_refs};
use crate::error::RemoteError;
use crate::git::{GitDaemonService, GitDaemonSession};
use crate::http::UreqTransport;
use crate::ssh::{SshService, SshSession};
use crate::transport::{HttpRequest, Transport, header, require_success};
use crate::url::{GitDaemonUrl, SshRemoteUrl, join_url};
use arks_oid::Oid;

const UPLOAD_AD: &str = "application/x-git-upload-pack-advertisement";

/// One advertised ref (`name`, `oid`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RemoteRef {
    pub name: String,
    pub oid: Oid,
}

/// List refs advertised by `url` (HTTP smart/dumb, SSH, `git://`).
pub fn ls_remote(url: &str) -> Result<Vec<RemoteRef>, RemoteError> {
    if let Ok(ssh) = SshRemoteUrl::parse(url) {
        let mut sess = SshSession::connect(&ssh, SshService::UploadPack)?;
        return Ok(from_ad(sess.read_advertisement()?));
    }
    if let Ok(git) = GitDaemonUrl::parse(url) {
        let mut sess = GitDaemonSession::connect(&git, GitDaemonService::UploadPack)?;
        return Ok(from_ad(sess.read_advertisement()?));
    }
    ls_remote_http(url)
}

fn from_ad(ad: Advertisement) -> Vec<RemoteRef> {
    ad.refs
        .into_iter()
        .map(|r| RemoteRef {
            name: r.name,
            oid: r.oid,
        })
        .collect()
}

fn ls_remote_http(url: &str) -> Result<Vec<RemoteRef>, RemoteError> {
    let mut transport = UreqTransport::new();
    let info_url = join_url(url, "info/refs?service=git-upload-pack");
    let info = transport.exchange(&HttpRequest {
        method: "GET".into(),
        url: info_url,
        headers: vec![("Accept".into(), UPLOAD_AD.into())],
        body: Vec::new(),
    })?;
    require_success(&info, "info/refs")?;
    let ct = header(&info.headers, "content-type");
    if looks_like_dumb_info_refs(ct, &info.body) {
        return Ok(from_ad(parse_dumb_info_refs(&info.body)?));
    }
    if !content_type_is(ct, "git-upload-pack-advertisement") {
        return Err(RemoteError::Protocol(
            "info/refs was not a git-upload-pack advertisement".into(),
        ));
    }
    Ok(from_ad(parse_advertisement(
        &info.body,
        "git-upload-pack",
    )?))
}
