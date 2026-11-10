//! `arks credential` — git-credential plumbing (fill / approve / reject).
//! First-party OS wallet helpers: `credential-wincred` / `credential-libsecret` /
//! `credential-osxkeychain`.
//!
//! Primary: <https://git-scm.com/docs/git-credential>
//! Cite: `.spec/sot/credential-helper-askpass.sot.md`

use crate::error::{CliError, CliResult};
use arks_remote::{
    Credential, CredentialSource, WalletKind, cmd_wallet_helper, credential_approve,
    credential_fill, credential_reject,
};
use std::io::{self, Read, Write};
use std::path::PathBuf;

pub fn cmd_credential(args: &[String]) -> CliResult<()> {
    let action = args
        .first()
        .map(String::as_str)
        .ok_or_else(|| CliError::Usage("credential <fill|approve|reject>".into()))?;
    let mut input = String::new();
    io::stdin().read_to_string(&mut input)?;
    let desc = Credential::parse(&input).map_err(|e| CliError::Msg(e.to_string()))?;
    let mut extra = Vec::new();
    if let Ok(p) = std::env::var("ARKS_CREDENTIAL_CONFIG") {
        extra.push(PathBuf::from(p));
    }
    let source = CredentialSource::discover(&extra);
    match action {
        "fill" => {
            let filled =
                credential_fill(desc, &source).map_err(|e| CliError::Msg(e.to_string()))?;
            io::stdout().write_all(filled.format_attributes().as_bytes())?;
            Ok(())
        }
        "approve" => {
            credential_approve(&desc, &source).map_err(|e| CliError::Msg(e.to_string()))?;
            Ok(())
        }
        "reject" => {
            credential_reject(&desc, &source).map_err(|e| CliError::Msg(e.to_string()))?;
            Ok(())
        }
        other => Err(CliError::Usage(format!(
            "credential: unknown action '{other}' (fill|approve|reject)"
        ))),
    }
}

pub fn cmd_credential_wincred(args: &[String]) -> CliResult<()> {
    cmd_wallet_helper(WalletKind::Wincred, args).map_err(|e| CliError::Msg(e.to_string()))
}

pub fn cmd_credential_libsecret(args: &[String]) -> CliResult<()> {
    cmd_wallet_helper(WalletKind::Libsecret, args).map_err(|e| CliError::Msg(e.to_string()))
}

pub fn cmd_credential_osxkeychain(args: &[String]) -> CliResult<()> {
    cmd_wallet_helper(WalletKind::Osxkeychain, args).map_err(|e| CliError::Msg(e.to_string()))
}
