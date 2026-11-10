//! FS5 `replace` — create / list / delete `refs/replace/*`.
//!
//! Cite: <https://git-scm.com/docs/git-replace>

use crate::cmd::shared::resolve_rev;
use crate::error::{CliError, CliResult};
use crate::util::{open_repo, refuse_unknown_flags};
pub fn cmd_replace(args: &[String]) -> CliResult<()> {
    // Claimed: `replace [-d] <object> [<replacement>]` and bare list.
    refuse_unknown_flags(args, &["-d", "--delete", "-l", "--list"])?;
    let delete = args.iter().any(|a| a == "-d" || a == "--delete");
    let list = args.iter().any(|a| a == "-l" || a == "--list")
        || args.iter().filter(|a| !a.starts_with('-')).count() == 0;
    let positional: Vec<&String> = args.iter().filter(|a| !a.starts_with('-')).collect();
    let mut repo = open_repo()?;
    // CORR-003: refuse under replace.lock
    let lock = repo.common_dir().join("refs/replace.lock");
    let lock_gd = repo.git_dir().join("refs/replace.lock");
    if lock.is_file() || lock_gd.is_file() {
        let p = if lock.is_file() { &lock } else { &lock_gd };
        return Err(CliError::Msg(format!(
            "{} (CORR-003 — concurrent replace-refs refuse; lease ≠ skip)",
            p.display()
        )));
    }
    if list && !delete {
        let refs = repo.refs().list_refs("refs/replace/")?;
        for (name, oid) in refs {
            let short = name.strip_prefix("refs/replace/").unwrap_or(&name);
            println!("{short} -> {}", oid.to_hex());
        }
        return Ok(());
    }
    if delete {
        let obj = positional
            .first()
            .ok_or_else(|| CliError::Usage("replace -d <object>".into()))?;
        let oid = resolve_rev(&repo, obj)?;
        let name = format!("refs/replace/{}", oid.to_hex());
        if repo.refs().resolve(&name).is_err() {
            return Err(CliError::Msg(format!(
                "replace: no replacement for {}",
                oid.to_hex()
            )));
        }
        repo.refs_mut().delete(&name, None)?;
        return Ok(());
    }
    if positional.len() != 2 {
        return Err(CliError::Usage(
            "replace <object> <replacement> | replace -d <object> | replace -l".into(),
        ));
    }
    let object = resolve_rev(&repo, positional[0])?;
    let replacement = resolve_rev(&repo, positional[1])?;
    if object == replacement {
        return Err(CliError::Msg(
            "replace: object and replacement are the same".into(),
        ));
    }
    let name = format!("refs/replace/{}", object.to_hex());
    repo.refs_mut().update(&name, &replacement, None)?;
    println!("Replaced {} with {}", object.to_hex(), replacement.to_hex());
    Ok(())
}
