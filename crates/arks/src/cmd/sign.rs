//! Commit/tag signature create/verify for FW-GPGSIG (SSH + OpenPGP + X.509).
//!
//! Spawns configured `gpg.*.program` — does not invent crypto.
//! Cite: gitformat-signature, git-commit `-S`, git-verify-commit, git-tag `-s`,
//! git-verify-tag, git-config `gpg.format` / `gpg.*.program`.

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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum GpgFormat {
    Ssh,
    OpenPgp,
    X509,
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

fn resolve_gpg_format(repo: &Repository) -> CliResult<GpgFormat> {
    let fmt = repo_config_get(repo, "gpg.format")
        .unwrap_or_else(|| "openpgp".into())
        .to_ascii_lowercase();
    match fmt.as_str() {
        "ssh" => Ok(GpgFormat::Ssh),
        "openpgp" | "gpg" => Ok(GpgFormat::OpenPgp),
        "x509" => Ok(GpgFormat::X509),
        other => Err(CliError::Msg(format!(
            "unsupported gpg.format={other:?} (claimed: ssh, openpgp, x509)"
        ))),
    }
}

/// Sign a commit object per configured `gpg.format` (SSH / OpenPGP / X.509).
pub(crate) fn sign_commit(
    repo: &Repository,
    commit: &mut Commit,
    key_override: Option<&str>,
) -> CliResult<()> {
    let payload = commit.signing_payload()?;
    let armor = sign_bytes(repo, &payload, key_override, "commit -S")?;
    commit.gpgsig = Some(armor);
    Ok(())
}

/// Sign arbitrary payload bytes (commit body or unsigned tag object).
pub(crate) fn sign_bytes(
    repo: &Repository,
    payload: &[u8],
    key_override: Option<&str>,
    label: &str,
) -> CliResult<String> {
    let fmt = resolve_gpg_format(repo)?;
    let signing_key = key_override
        .map(str::to_owned)
        .or_else(|| repo_config_get(repo, "user.signingKey"))
        .or_else(|| repo_config_get(repo, "user.signingkey"))
        .ok_or_else(|| CliError::Msg(format!("{label}: user.signingKey not set")))?;
    match fmt {
        GpgFormat::Ssh => {
            let program =
                repo_config_get(repo, "gpg.ssh.program").unwrap_or_else(|| "ssh-keygen".into());
            let key_path = resolve_ssh_signing_key(&signing_key)?;
            ssh_keygen_sign(&program, &key_path, payload)
        }
        GpgFormat::OpenPgp => {
            let program = repo_config_get(repo, "gpg.openpgp.program")
                .or_else(|| repo_config_get(repo, "gpg.program"))
                .unwrap_or_else(|| "gpg".into());
            gpg_detached_sign(&program, &signing_key, payload, label, "PGP SIGNATURE")
        }
        GpgFormat::X509 => {
            let program =
                repo_config_get(repo, "gpg.x509.program").unwrap_or_else(|| "gpgsm".into());
            gpg_detached_sign(&program, &signing_key, payload, label, "SIGNED MESSAGE")
        }
    }
}

fn resolve_ssh_signing_key(signing_key: &str) -> CliResult<PathBuf> {
    if let Some(literal) = signing_key.strip_prefix("key::") {
        return Err(CliError::NotImplemented(format!(
            "literal key:: signing keys not claimed (got key::{}…); use a key file path",
            literal.chars().take(12).collect::<String>()
        )));
    }
    if let Some(stem) = signing_key.strip_suffix(".pub") {
        let priv_path = PathBuf::from(stem);
        if !priv_path.is_file() {
            return Err(CliError::Msg(format!(
                "private key not found for {}: {}",
                signing_key,
                priv_path.display()
            )));
        }
        return Ok(priv_path);
    }
    let path = PathBuf::from(signing_key);
    if !path.is_file() {
        return Err(CliError::Msg(format!(
            "signing key file not found: {}",
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
        .map_err(|e| CliError::Msg(format!("failed to spawn {program}: {e}")))?;
    if !output.status.success() {
        let err = String::from_utf8_lossy(&output.stderr).trim().to_string();
        let _ = fs::remove_dir_all(&dir);
        return Err(CliError::Msg(format!(
            "{program} -Y sign failed (exit {}): {err}",
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
                "signature file missing after {program} -Y sign"
            )));
        }
    };
    let armor = fs::read_to_string(&sig_path)?;
    let _ = fs::remove_dir_all(&dir);
    if !armor.contains("BEGIN SSH SIGNATURE") {
        return Err(CliError::Msg(
            "signer did not produce SSH SIGNATURE armor".into(),
        ));
    }
    Ok(normalize_armor(&armor))
}

/// Git OpenPGP/X.509 shape: `program -bsau <key>` with payload on stdin → armor on stdout.
fn gpg_detached_sign(
    program: &str,
    key: &str,
    payload: &[u8],
    label: &str,
    expect_armor: &str,
) -> CliResult<String> {
    let mut child = Command::new(program)
        .args(["-bsau", key])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| CliError::Msg(format!("{label}: failed to spawn {program}: {e}")))?;
    {
        let mut stdin = child
            .stdin
            .take()
            .ok_or_else(|| CliError::Msg(format!("{label}: missing stdin")))?;
        stdin.write_all(payload)?;
    }
    let output = child
        .wait_with_output()
        .map_err(|e| CliError::Msg(format!("{label}: {program} wait failed: {e}")))?;
    if !output.status.success() {
        let err = String::from_utf8_lossy(&output.stderr).trim().to_string();
        return Err(CliError::Msg(format!(
            "{label}: {program} -bsau failed (exit {}): {err}",
            output.status
        )));
    }
    let armor = String::from_utf8_lossy(&output.stdout).to_string();
    let marker = format!("BEGIN {expect_armor}");
    if !armor.contains(&marker)
        && !(expect_armor == "PGP SIGNATURE" && armor.contains("BEGIN PGP MESSAGE"))
    {
        return Err(CliError::Msg(format!(
            "{label}: {program} did not produce {expect_armor} armor"
        )));
    }
    Ok(normalize_armor(&armor))
}

fn verify_bytes(repo: &Repository, armor: &str, payload: &[u8], label: &str) -> CliResult<String> {
    if armor.contains("BEGIN SSH SIGNATURE") {
        let program =
            repo_config_get(repo, "gpg.ssh.program").unwrap_or_else(|| "ssh-keygen".into());
        let allowed = repo_config_get(repo, "gpg.ssh.allowedSignersFile").ok_or_else(|| {
            CliError::Msg(format!(
                "{label}: gpg.ssh.allowedSignersFile needs to be configured and exist for ssh signature verification"
            ))
        })?;
        let allowed_path = PathBuf::from(&allowed);
        if !allowed_path.is_file() {
            return Err(CliError::Msg(format!(
                "{label}: allowedSignersFile not found: {allowed}"
            )));
        }
        // Prefer committer email when verifying commits; for tags, use tagger from caller via config.
        let principal = repo_config_get(repo, "user.email")
            .or_else(|| std::env::var("GIT_COMMITTER_EMAIL").ok())
            .unwrap_or_else(|| "git".into());
        return ssh_keygen_verify(&program, &allowed_path, &principal, armor, payload);
    }
    if armor.contains("BEGIN PGP SIGNATURE") || armor.contains("BEGIN PGP MESSAGE") {
        let program = repo_config_get(repo, "gpg.openpgp.program")
            .or_else(|| repo_config_get(repo, "gpg.program"))
            .unwrap_or_else(|| "gpg".into());
        return gpg_detached_verify(&program, armor, payload, label);
    }
    if armor.contains("BEGIN SIGNED MESSAGE") {
        let program = repo_config_get(repo, "gpg.x509.program").unwrap_or_else(|| "gpgsm".into());
        return gpg_detached_verify(&program, armor, payload, label);
    }
    Err(CliError::Msg(format!(
        "{label}: unrecognized signature armor"
    )))
}

fn verify_commit_object(repo: &Repository, commit: &Commit) -> CliResult<String> {
    let armor = commit
        .gpgsig
        .as_deref()
        .ok_or_else(|| CliError::Msg("verify-commit: commit has no gpgsig header".into()))?;
    let payload = commit.signing_payload()?;
    if armor.contains("BEGIN SSH SIGNATURE") {
        let program =
            repo_config_get(repo, "gpg.ssh.program").unwrap_or_else(|| "ssh-keygen".into());
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
        return ssh_keygen_verify(
            &program,
            &allowed_path,
            &commit.committer.email,
            armor,
            &payload,
        );
    }
    verify_bytes(repo, armor, &payload, "verify-commit")
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
        .map_err(|e| CliError::Msg(format!("failed to spawn {program}: {e}")))?;
    {
        let mut stdin = child
            .stdin
            .take()
            .ok_or_else(|| CliError::Msg("missing stdin".into()))?;
        stdin.write_all(payload)?;
    }
    let output = child
        .wait_with_output()
        .map_err(|e| CliError::Msg(format!("{program} wait failed: {e}")))?;
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
        return Err(CliError::Msg(format!("bad signature ({detail})")));
    }
    Ok(if !stderr.is_empty() { stderr } else { stdout })
}

/// Git OpenPGP/X.509 verify: `program --verify <sigfile> -` with payload on stdin.
fn gpg_detached_verify(
    program: &str,
    armor: &str,
    payload: &[u8],
    label: &str,
) -> CliResult<String> {
    let dir = tempfile_dir("arks-gpg-verify")?;
    let sig_path = dir.join("payload.sig");
    fs::write(&sig_path, normalize_armor(armor))?;
    let mut child = Command::new(program)
        .args(["--verify"])
        .arg(&sig_path)
        .arg("-")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| CliError::Msg(format!("{label}: failed to spawn {program}: {e}")))?;
    {
        let mut stdin = child
            .stdin
            .take()
            .ok_or_else(|| CliError::Msg(format!("{label}: missing stdin")))?;
        stdin.write_all(payload)?;
    }
    let output = child
        .wait_with_output()
        .map_err(|e| CliError::Msg(format!("{label}: {program} wait failed: {e}")))?;
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
        return Err(CliError::Msg(format!("{label}: bad signature ({detail})")));
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
        let msg = verify_commit_object(&repo, &commit)?;
        eprintln!("{msg}");
        if verbose {
            let payload = commit.signing_payload()?;
            print!("{}", String::from_utf8_lossy(&payload));
        }
    }
    Ok(())
}

/// Split a signed tag object into (unsigned_payload, armor).
pub(crate) fn split_signed_tag(payload: &[u8]) -> CliResult<(Vec<u8>, String)> {
    let text = std::str::from_utf8(payload)
        .map_err(|_| CliError::Msg("verify-tag: tag payload not utf-8".into()))?;
    let markers = [
        "-----BEGIN PGP SIGNATURE-----",
        "-----BEGIN PGP MESSAGE-----",
        "-----BEGIN SSH SIGNATURE-----",
        "-----BEGIN SIGNED MESSAGE-----",
    ];
    let mut cut = None;
    for m in markers {
        if let Some(i) = text.find(m) {
            cut = Some(match cut {
                Some(c) if c < i => c,
                _ => i,
            });
        }
    }
    let i = cut.ok_or_else(|| CliError::Msg("verify-tag: no signature armor found".into()))?;
    let unsigned = text.as_bytes()[..i].to_vec();
    let armor = normalize_armor(&text[i..]);
    Ok((unsigned, armor))
}

pub fn cmd_verify_tag(args: &[String]) -> CliResult<()> {
    use crate::cmd::shared::resolve_rev;
    use crate::util::{open_repo, refuse_unknown_flags};
    use arks_object::ObjectKind;

    refuse_unknown_flags(args, &["-v", "--verbose"])?;
    let verbose = args.iter().any(|a| a == "-v" || a == "--verbose");
    let revs: Vec<&String> = args.iter().filter(|a| !a.starts_with('-')).collect();
    if revs.is_empty() {
        return Err(CliError::Usage("verify-tag [-v] <tag>...".into()));
    }
    let repo = open_repo()?;
    for rev in revs {
        let oid = resolve_rev(&repo, rev)?;
        let obj = repo.get_object(&oid)?;
        if obj.kind != ObjectKind::Tag {
            return Err(CliError::Msg(format!("{rev}: not a tag object")));
        }
        let (unsigned, armor) = split_signed_tag(&obj.payload)?;
        let msg = verify_bytes(&repo, &armor, &unsigned, "verify-tag")?;
        eprintln!("{msg}");
        if verbose {
            print!("{}", String::from_utf8_lossy(&unsigned));
        }
    }
    Ok(())
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

    fn find_gpg() -> Option<PathBuf> {
        for c in [
            r"C:\Program Files\Git\usr\bin\gpg.exe",
            "/usr/bin/gpg",
            "gpg",
        ] {
            let p = PathBuf::from(c);
            if c == "gpg" {
                if Command::new("gpg").arg("--version").output().is_ok() {
                    return Some(p);
                }
            } else if p.is_file() {
                return Some(p);
            }
        }
        None
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
    fn openpgp_sign_verify_roundtrip_ephemeral_key() {
        // Host Windows MSYS gpg keygen is hang-prone — carrying evidence is Docker G38.
        // Opt-in only: ARKS_HOST_GPG=1.
        if std::env::var_os("ARKS_HOST_GPG").is_none() {
            eprintln!("skip openpgp host keygen (set ARKS_HOST_GPG=1 to force; prefer G38)");
            return;
        }
        let Some(gpg) = find_gpg() else {
            eprintln!("skip openpgp test: gpg not found");
            return;
        };
        let stamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0);
        let gpg_home_win = std::env::temp_dir().join(format!("arks-gpg-unit-{stamp}"));
        fs::create_dir_all(&gpg_home_win).unwrap();
        // MSYS gpg wants a Unix-style GNUPGHOME (`/c/Users/...`).
        let gpg_home = if cfg!(windows) {
            let s = gpg_home_win.to_string_lossy().replace('\\', "/");
            if let Some((drive, rest)) = s.split_once(':') {
                format!("/{}{}", drive.to_ascii_lowercase(), rest)
            } else {
                s
            }
        } else {
            gpg_home_win.to_string_lossy().to_string()
        };

        let status = Command::new(&gpg)
            .args([
                "--batch",
                "--pinentry-mode",
                "loopback",
                "--passphrase",
                "",
                "--quick-generate-key",
                "arks-unit <arks-unit@example.com>",
                "default",
                "default",
                "never",
            ])
            .env("GNUPGHOME", &gpg_home)
            .status()
            .expect("gpg keygen spawn");
        assert!(status.success(), "gpg keygen failed");

        let list = Command::new(&gpg)
            .args(["--list-secret-keys", "--with-colons"])
            .env("GNUPGHOME", &gpg_home)
            .output()
            .expect("gpg list");
        let out = String::from_utf8_lossy(&list.stdout);
        let fpr = out
            .lines()
            .find_map(|l| {
                let parts: Vec<_> = l.split(':').collect();
                if parts.first() == Some(&"fpr") && parts.len() > 9 {
                    Some(parts[9].to_string())
                } else {
                    None
                }
            })
            .expect("fingerprint");

        let mut commit = sample_commit("arks-unit@example.com");
        let payload = commit.signing_payload().unwrap();
        let mut child = Command::new(&gpg)
            .args(["-bsau", &fpr])
            .env("GNUPGHOME", &gpg_home)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        child.stdin.as_mut().unwrap().write_all(&payload).unwrap();
        let signed = child.wait_with_output().unwrap();
        assert!(signed.status.success(), "gpg -bsau failed");
        let armor = normalize_armor(&String::from_utf8_lossy(&signed.stdout));
        assert!(armor.contains("BEGIN PGP"));
        commit.gpgsig = Some(armor.clone());

        let msg = gpg_detached_verify(
            gpg.to_str().unwrap(),
            &armor,
            &commit.signing_payload().unwrap(),
            "test",
        );
        // verify inherits GNUPGHOME from process — set via env for the helper by
        // calling gpg directly with env (helper uses Command without GNUPGHOME).
        let dir = tempfile_dir("arks-gpg-v").unwrap();
        let sig_path = dir.join("payload.sig");
        fs::write(&sig_path, &armor).unwrap();
        let mut v = Command::new(&gpg)
            .args(["--verify"])
            .arg(&sig_path)
            .arg("-")
            .env("GNUPGHOME", &gpg_home)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        v.stdin
            .as_mut()
            .unwrap()
            .write_all(&commit.signing_payload().unwrap())
            .unwrap();
        let verified = v.wait_with_output().unwrap();
        assert!(
            verified.status.success(),
            "gpg --verify failed: {}",
            String::from_utf8_lossy(&verified.stderr)
        );
        let _ = msg;
        let _ = fs::remove_dir_all(&dir);
        let _ = fs::remove_dir_all(&gpg_home_win);
    }

    #[test]
    fn split_signed_tag_finds_armor() {
        let payload = b"\
object 04b871796dc0420f8e7561a895b52484b701d51a
type commit
tag signedtag
tagger C O Mitter <c@example.com> 1 +0000

msg
-----BEGIN PGP SIGNATURE-----
Version: test

abc
-----END PGP SIGNATURE-----
";
        let (unsigned, armor) = split_signed_tag(payload).unwrap();
        assert!(String::from_utf8_lossy(&unsigned).ends_with("msg\n"));
        assert!(armor.starts_with("-----BEGIN PGP SIGNATURE-----"));
    }
}
