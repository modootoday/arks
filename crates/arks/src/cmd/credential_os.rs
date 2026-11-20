//! `git credential-manager` and `git credential-wincred` on every OS.
//!
//! Both are credential helpers: `<helper> get|store|erase` reads credential
//! attributes on standard input and, for `get`, prints `username=` /
//! `password=` lines. Other operations are ignored as gitcredentials asks of
//! helpers. The store is the OS one (Windows Credential Manager, macOS
//! keychain, Secret Service); without one the helper fails with a clear
//! error and stores nothing. See `arks_remote::os_store` for the backends and
//! the `ARKS_CREDENTIAL_BACKEND` test switch; the older
//! `ARKS_CREDENTIAL_WALLET_DIR` file override (G33 gate) still applies when
//! no backend is named.
//!
//! Primary: <https://git-scm.com/docs/gitcredentials> (custom helpers),
//! <https://git-scm.com/docs/git-credential>.
//! Plan: `.spec/plan/20261120000000-argv-symmetry-ladder.plan.md` (A3 design decisions 2).

use crate::error::{CliError, CliResult};
use std::io::{Read, Write};

fn usage(name: &str) -> CliError {
    CliError::Exit(129, format!("usage: git {name} <get|store|erase>"))
}

/// Run helper `name` (`credential-manager` / `credential-wincred`) with `args`.
pub fn run_helper(name: &str, args: &[String]) -> CliResult<()> {
    let Some(op) = args.first() else {
        return Err(usage(name));
    };
    match op.as_str() {
        "-h" | "--help" | "help" => return Err(usage(name)),
        "--version" | "version" if name == "credential-manager" => {
            println!("arks {name} version {}", env!("CARGO_PKG_VERSION"));
            return Ok(());
        }
        _ => {}
    }
    // The G33 wallet-directory test override keeps its file backend.
    let backend_unset = std::env::var_os("ARKS_CREDENTIAL_BACKEND").is_none();
    if backend_unset
        && std::env::var_os("ARKS_CREDENTIAL_WALLET_DIR").is_some_and(|d| !d.is_empty())
    {
        return arks_remote::cmd_wallet_helper(arks_remote::WalletKind::Wincred, args)
            .map_err(|e| CliError::Exit(1, format!("error: {name}: {e}")));
    }
    let mut input = String::new();
    std::io::stdin().read_to_string(&mut input)?;
    match arks_remote::run_os_store_helper(op, &input) {
        Ok(Some(out)) => {
            let mut so = std::io::stdout().lock();
            so.write_all(out.as_bytes())?;
            so.flush()?;
            Ok(())
        }
        Ok(None) => Ok(()),
        Err(e) => Err(CliError::Exit(1, format!("error: {name}: {e}"))),
    }
}

/// `git credential-manager <get|store|erase>`.
pub fn cmd_credential_manager(args: &[String]) -> CliResult<()> {
    run_helper("credential-manager", args)
}
