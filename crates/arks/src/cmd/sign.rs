//! Commit signature create/verify for claimed FW-GPGSIG **SSH** subset.
//!
//! Spawns `gpg.ssh.program` (default `ssh-keygen`) — does not invent crypto.
//! Cite: gitformat-signature, git-commit `-S`, git-verify-commit, git-config
//! `gpg.format` / `gpg.ssh.*`. OpenPGP / X.509 remain refuse-closed.

use crate::error::{CliError, CliResult};
use arks_object::Commit;
use arks_repo::Repository;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

pub(crate) fn is_sign_flag(a: &str) -> bool {
    a == "-S"
        || a == "--gpg-sign"
        || a == "--no-gpg-sign"
        || a.starts_with("--gpg-sign=")
        || (a.starts_with("-S") && !a.starts_with("--"))
}

enum SignRequest {
    Yes { keyid: Option<String> },
    No,
    Auto,
}

fn parse_sign_flags(args: &[String]) -> SignRequest {
    let mut req = SignRequest::Auto;
    for a in args {
        if a == "--no-gpg-sign" {
            req = SignRequest::No;
        } else if a == "--gpg-sign" || a == "-S" {
            req = SignRequest::Yes { keyid: None };
        } else if let Some(key) = a.strip_prefix("--gpg-sign=") {
            req = SignRequest::Yes {
                keyid: Some(key.to_owned()),
            };
        } else if let Some(key) = a.strip_prefix("-S") {
            if !a.starts_with("--") {
                if key.is_empty() {
                    req = SignRequest::Yes { keyid: None };
                } else {
                    req = SignRequest::Yes {
                        keyid: Some(key.to_owned()),
                    };
                }
            }
        }
    }
    req
}

pub(crate) fn want_sign_commit(
    args: &[String],
    repo: &Repository,
) -> CliResult<(bool, Option<String>)> {
    match parse_sign_flags(args) {
        SignRequest::No => Ok((false, None)),
        SignRequest::Yes { keyid } => Ok((true, keyid)),
        SignRequest::Auto => {
            let on = repo_config_bool(repo, "commit.gpgsign")
                .or_else(|| repo_config_bool(repo, "commit.gpgSign"))
                .unwrap_or(false);
            let key = repo_config_get(repo, "user.signingKey")
                .or_else(|| repo_config_get(repo, "user.signingkey"));
            Ok((on, key))
        }
    }
}

pub(crate) fn sign_commit_ssh(
    repo: &Repository,
    commit: &mut Commit,
    key_override: Option<&str>,
) -> CliResult<()> {
    refuse_non_ssh(repo)?;
    let program = repo_config_get(repo, "gpg.ssh.program").unwrap_or_else(|| "ssh-keygen".into());
    let signing_key = key_override
        .map(str::to_owned)
        .or_else(|| repo_config_get(repo, "user.signingKey"))
        .or_else(|| repo_config_get(repo, "user.signingkey"))
        .ok_or_else(|| {
            CliError::Msg("commit -S: user.signingKey not set (gpg.format=ssh)".into())
        })?;
    let key_path = resolve_ssh_signing_key(&signing_key)?;
    let payload = commit.signing_payload()?;
    let armor = ssh_keygen_sign(&program, &key_path, &payload)?;
    commit.gpgsig = Some(armor);
    Ok(())
}

fn resolve_ssh_signing_key(signing_key: &str) -> CliResult<PathBuf> {
    if let Some(literal) = signing_key.strip_prefix("key::") {
        return Err(CliError::NotImplemented(format!(
            "commit -S: literal key:: signing keys not claimed (got key::{}…); use a key file path",
            literal.chars().take(12).collect::<String>()
        )));
    }
    if let Some(stem) = signing_key.strip_suffix(".pub") {
        let priv_path = PathBuf::from(stem);
        if !priv_path.is_file() {
            return Err(CliError::Msg(format!(
                "commit -S: private key not found for {}: {}",
                signing_key,
                priv_path.display()
            )));
        }
        return Ok(priv_path);
    }
    let path = PathBuf::from(signing_key);
    if !path.is_file() {
        return Err(CliError::Msg(format!(
            "commit -S: signing key file not found: {}",
            path.display()
        )));
    }
    Ok(path)
}

fn ssh_keygen_sign(program: &str, key_path: &Path, payload: &[u8]) -> CliResult<String> {
    let dir = tempfile_dir("arks-ssh-sign")?;
    let payload_path = dir.join("payload");
    fs::write(&payload_path, payload)?;
    let output = Command::new(program)
        .args(["-Y", "sign", "-n", "git", "-f"])
        .arg(key_path)
        .arg(&payload_path)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .map_err(|e| CliError::Msg(format!("commit -S: failed to spawn {program}: {e}")))?;
    if !output.status.success() {
        let err = String::from_utf8_lossy(&output.stderr).trim().to_string();
        let _ = fs::remove_dir_all(&dir);
        return Err(CliError::Msg(format!(
            "commit -S: {program} -Y sign failed (exit {}): {err}",
            output.status
        )));
    }
    let sig_path = payload_path.with_extension("sig");
    let sig_path = if sig_path.is_file() {
        sig_path
    } else {
        let alt = PathBuf::from(format!("{}.sig", payload_path.display()));
        if alt.is_file() {
            alt
        } else {
            let _ = fs::remove_dir_all(&dir);
            return Err(CliError::Msg(format!(
                "commit -S: signature file missing after {program} -Y sign"
            )));
        }
    };
    let armor = fs::read_to_string(&sig_path)?;
    let _ = fs::remove_dir_all(&dir);
    if !armor.contains("BEGIN SSH SIGNATURE") {
        return Err(CliError::Msg(
            "commit -S: signer did not produce SSH SIGNATURE armor".into(),
        ));
    }
    Ok(normalize_armor(&armor))
}

fn verify_commit_ssh(repo: &Repository, commit: &Commit) -> CliResult<String> {
    let armor = commit
        .gpgsig
        .as_deref()
        .ok_or_else(|| CliError::Msg("verify-commit: commit has no gpgsig header".into()))?;
    if armor.contains("BEGIN SSH SIGNATURE") {
        // claimed
    } else if armor.contains("BEGIN PGP SIGNATURE") || armor.contains("BEGIN PGP MESSAGE") {
        return Err(CliError::NotImplemented(
            "verify-commit: OpenPGP signatures unclaimed (gpg.format=ssh only)".into(),
        ));
    } else if armor.contains("BEGIN SIGNED MESSAGE") {
        return Err(CliError::NotImplemented(
            "verify-commit: X.509 signatures unclaimed (gpg.format=ssh only)".into(),
        ));
    } else {
        return Err(CliError::Msg(
            "verify-commit: unrecognized gpgsig armor".into(),
        ));
    }
    let program = repo_config_get(repo, "gpg.ssh.program").unwrap_or_else(|| "ssh-keygen".into());
    let allowed = repo_config_get(repo, "gpg.ssh.allowedSignersFile").ok_or_else(|| {
        CliError::Msg(
            "verify-commit: gpg.ssh.allowedSignersFile needs to be configured and exist for ssh signature verification"
                .into(),
        )
    })?;
    let allowed_path = PathBuf::from(&allowed);
    if !allowed_path.is_file() {
        return Err(CliError::Msg(format!(
            "verify-commit: allowedSignersFile not found: {allowed}"
        )));
    }
    let principal = commit.committer.email.clone();
    let payload = commit.signing_payload()?;
    ssh_keygen_verify(&program, &allowed_path, &principal, armor, &payload)
}

fn ssh_keygen_verify(
    program: &str,
    allowed_signers: &Path,
    principal: &str,
    armor: &str,
    payload: &[u8],
) -> CliResult<String> {
    let dir = tempfile_dir("arks-ssh-verify")?;
    let sig_path = dir.join("payload.sig");
    fs::write(&sig_path, normalize_armor(armor))?;
    let mut child = Command::new(program)
        .args(["-Y", "verify", "-f"])
        .arg(allowed_signers)
        .args(["-I", principal, "-n", "git", "-s"])
        .arg(&sig_path)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| CliError::Msg(format!("verify-commit: failed to spawn {program}: {e}")))?;
    {
        let mut stdin = child
            .stdin
            .take()
            .ok_or_else(|| CliError::Msg("verify-commit: missing stdin".into()))?;
        stdin.write_all(payload)?;
    }
    let output = child
        .wait_with_output()
        .map_err(|e| CliError::Msg(format!("verify-commit: {program} wait failed: {e}")))?;
    let _ = fs::remove_dir_all(&dir);
    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if !output.status.success() {
        let detail = if !stderr.is_empty() {
            stderr
        } else if !stdout.is_empty() {
            stdout
        } else {
            format!("exit {}", output.status)
        };
        return Err(CliError::Msg(format!(
            "verify-commit: bad signature ({detail})"
        )));
    }
    Ok(if !stderr.is_empty() { stderr } else { stdout })
}

pub fn cmd_verify_commit(args: &[String]) -> CliResult<()> {
    use crate::cmd::shared::resolve_rev;
    use crate::util::{open_repo, refuse_unknown_flags};
    use arks_object::ObjectKind;

    refuse_unknown_flags(args, &["-v", "--verbose"])?;
    let verbose = args.iter().any(|a| a == "-v" || a == "--verbose");
    let revs: Vec<&String> = args.iter().filter(|a| !a.starts_with('-')).collect();
    if revs.is_empty() {
        return Err(CliError::Usage("verify-commit [-v] <commit>...".into()));
    }
    let repo = open_repo()?;
    for rev in revs {
        let oid = resolve_rev(&repo, rev)?;
        let obj = repo.get_object(&oid)?;
        if obj.kind != ObjectKind::Commit {
            return Err(CliError::Msg(format!("{rev}: not a commit")));
        }
        let commit = Commit::decode(&obj.payload)?;
        let msg = verify_commit_ssh(&repo, &commit)?;
        eprintln!("{msg}");
        if verbose {
            let payload = commit.signing_payload()?;
            print!("{}", String::from_utf8_lossy(&payload));
        }
    }
    Ok(())
}

fn refuse_non_ssh(repo: &Repository) -> CliResult<()> {
    let fmt = repo_config_get(repo, "gpg.format")
        .unwrap_or_else(|| "openpgp".into())
        .to_ascii_lowercase();
    match fmt.as_str() {
        "ssh" => Ok(()),
        "openpgp" | "gpg" => Err(CliError::NotImplemented(
            "commit -S: OpenPGP (gpg.format=openpgp) unclaimed; set gpg.format=ssh".into(),
        )),
        "x509" => Err(CliError::NotImplemented(
            "commit -S: X.509 (gpg.format=x509) unclaimed; set gpg.format=ssh".into(),
        )),
        other => Err(CliError::Msg(format!(
            "commit -S: unsupported gpg.format={other:?} (claimed: ssh)"
        ))),
    }
}

fn normalize_armor(armor: &str) -> String {
    let mut s = armor.replace("\r\n", "\n");
    if !s.ends_with('\n') {
        s.push('\n');
    }
    s
}

fn tempfile_dir(prefix: &str) -> CliResult<PathBuf> {
    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    let dir = std::env::temp_dir().join(format!("{prefix}-{stamp}-{}", std::process::id()));
    fs::create_dir_all(&dir)?;
    Ok(dir)
}

fn repo_config_get(repo: &Repository, key: &str) -> Option<String> {
    let path = repo.git_dir().join("config");
    let text = fs::read_to_string(path).ok()?;
    config_get_simple(&text, key)
}

fn config_get_simple(text: &str, key: &str) -> Option<String> {
    let (section_norm, name) = key.rsplit_once('.')?;
    let name_l = name.to_ascii_lowercase();
    let mut cur_norm: Option<String> = None;
    for line in text.lines() {
        let t = line.trim();
        if t.starts_with('[') && t.ends_with(']') {
            cur_norm = Some(normalize_section_inner(&t[1..t.len() - 1]));
            continue;
        }
        if let Some(sec) = &cur_norm {
            if sec.eq_ignore_ascii_case(section_norm) {
                if let Some((k, v)) = t.split_once('=') {
                    if k.trim().eq_ignore_ascii_case(&name_l) {
                        return Some(v.trim().to_string());
                    }
                }
            }
        }
    }
    None
}

fn normalize_section_inner(inner: &str) -> String {
    let inner = inner.trim();
    if let Some((sec, rest)) = inner.split_once(' ') {
        let sub = rest.trim().trim_matches('"');
        format!("{sec}.{sub}")
    } else {
        inner.trim_matches('"').to_string()
    }
}

fn repo_config_bool(repo: &Repository, key: &str) -> Option<bool> {
    let v = repo_config_get(repo, key)?;
    match v.to_ascii_lowercase().as_str() {
        "1" | "true" | "yes" | "on" => Some(true),
        "0" | "false" | "no" | "off" => Some(false),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use arks_object::Signature;
    use arks_oid::Oid;

    fn oid(n: u8) -> Oid {
        Oid::from_hex(&format!("{n:040x}")).unwrap()
    }

    fn sample_commit(email: &str) -> Commit {
        let sig = Signature {
            name: "Test".into(),
            email: email.into(),
            timestamp: 1_700_000_000,
            timezone: "+0000".into(),
        };
        Commit {
            tree: oid(1),
            parents: vec![],
            author: sig.clone(),
            committer: sig,
            gpgsig: None,
            message: "signed\n".into(),
        }
    }

    fn gen_ephemeral_ed25519(dir: &Path) -> (PathBuf, PathBuf) {
        let priv_key = dir.join("id_ed25519");
        let status = Command::new("ssh-keygen")
            .args([
                "-t",
                "ed25519",
                "-f",
                priv_key.to_str().unwrap(),
                "-N",
                "",
                "-q",
                "-C",
                "arks-fw-gpgsig-test",
            ])
            .status()
            .expect("spawn ssh-keygen");
        assert!(status.success(), "ssh-keygen keygen failed");
        let pub_key = dir.join("id_ed25519.pub");
        assert!(pub_key.is_file());
        (priv_key, pub_key)
    }

    #[test]
    fn config_reads_gpg_ssh_subsection() {
        let text = "\
[gpg]
\tformat = ssh
[gpg \"ssh\"]
\tprogram = ssh-keygen
\tallowedSignersFile = /tmp/allowed_signers
[user]
\tsigningKey = /tmp/id_ed25519
";
        assert_eq!(
            config_get_simple(text, "gpg.format").as_deref(),
            Some("ssh")
        );
        assert_eq!(
            config_get_simple(text, "gpg.ssh.program").as_deref(),
            Some("ssh-keygen")
        );
        assert_eq!(
            config_get_simple(text, "gpg.ssh.allowedSignersFile").as_deref(),
            Some("/tmp/allowed_signers")
        );
        assert_eq!(
            config_get_simple(text, "user.signingKey").as_deref(),
            Some("/tmp/id_ed25519")
        );
    }

    #[test]
    fn ssh_sign_verify_roundtrip_ephemeral_key() {
        let dir = tempfile_dir("arks-ssh-unit").expect("tmpdir");
        let (priv_key, pub_key) = gen_ephemeral_ed25519(&dir);
        let pub_line = fs::read_to_string(&pub_key).unwrap();
        let principal = "signer@example.com";
        let allowed = dir.join("allowed_signers");
        fs::write(&allowed, format!("{principal} {}", pub_line.trim())).unwrap();

        let mut commit = sample_commit(principal);
        let payload = commit.signing_payload().unwrap();
        let armor = ssh_keygen_sign("ssh-keygen", &priv_key, &payload).expect("sign");
        assert!(armor.contains("BEGIN SSH SIGNATURE"));
        commit.gpgsig = Some(armor.clone());

        let encoded = commit.encode().unwrap();
        let decoded = Commit::decode(&encoded).unwrap();
        assert_eq!(decoded.gpgsig.as_deref(), Some(armor.as_str()));

        let msg = ssh_keygen_verify(
            "ssh-keygen",
            &allowed,
            principal,
            decoded.gpgsig.as_deref().unwrap(),
            &decoded.signing_payload().unwrap(),
        )
        .expect("verify");
        assert!(!msg.is_empty(), "unexpected empty verify output");

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn refuse_openpgp_armor_on_verify() {
        let commit = Commit {
            tree: oid(2),
            parents: vec![],
            author: Signature {
                name: "A".into(),
                email: "a@example.com".into(),
                timestamp: 1,
                timezone: "+0000".into(),
            },
            committer: Signature {
                name: "A".into(),
                email: "a@example.com".into(),
                timestamp: 1,
                timezone: "+0000".into(),
            },
            gpgsig: Some(
                "-----BEGIN PGP SIGNATURE-----\n\niQEcBAABAgAGBQJXYRjRAAoJ\n-----END PGP SIGNATURE-----\n"
                    .into(),
            ),
            message: "x\n".into(),
        };
        let dir = tempfile_dir("arks-ssh-refuse").expect("tmpdir");
        let git = dir.join(".git");
        fs::create_dir_all(git.join("objects")).unwrap();
        fs::create_dir_all(git.join("refs/heads")).unwrap();
        fs::write(git.join("HEAD"), b"ref: refs/heads/main\n").unwrap();
        fs::write(
            git.join("config"),
            "[core]\n\trepositoryformatversion = 0\n\tfilemode = true\n\tbare = false\n[gpg]\n\tformat = ssh\n",
        )
        .unwrap();
        let repo = Repository::discover(&dir).expect("discover");
        let err = verify_commit_ssh(&repo, &commit).expect_err("OpenPGP must refuse");
        let s = err.to_string();
        assert!(
            s.contains("OpenPGP") || s.contains("unclaimed"),
            "unexpected err: {s}"
        );
        let _ = fs::remove_dir_all(&dir);
    }
}
