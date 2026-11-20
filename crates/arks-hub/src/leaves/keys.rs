//! Key leaves: `arks ssh-key …`, `arks gpg-key …`, `arks repo deploy-key …`.
//!
//! Keys are validated locally ([`crate::key_formats`]) before upload so bad input
//! fails fast with exit 2; the forge validates again before storing.

use super::{Args, Ctx, EXPORT, Fail, Res, arr, json_out, n, read_input, row, s, usage};
use crate::key_formats::{parse_pgp_public_key, parse_ssh_public_key};
use serde_json::{Map, Value, json};
use std::io::Write;

pub(crate) const SSH_HELP: &str = "\
arks ssh-key — SSH keys on your arks forge account (gh-symmetric argv; arks-owned; host-tested Partial)

USAGE:
  arks ssh-key list
  arks ssh-key add [<key-file> | -] [-t <title>] [--type authentication|signing]
  arks ssh-key delete <id> [--yes]

Accepted key types: ssh-ed25519, ssh-rsa, ecdsa-sha2-nistp{256,384,521},
sk-ssh-ed25519@openssh.com, sk-ecdsa-sha2-nistp256@openssh.com.";

pub(crate) const GPG_HELP: &str = "\
arks gpg-key — OpenPGP keys on your arks forge account (gh-symmetric argv; arks-owned; host-tested Partial)

USAGE:
  arks gpg-key list
  arks gpg-key add [<key-file> | -] [-t <title>]
  arks gpg-key delete <key-id> [--yes]

Input must be an ASCII-armored public key (v4 or v6). Expiry is not evaluated.";

pub(crate) const DEPLOY_HELP: &str = "\
arks repo deploy-key — repository deploy keys (gh-symmetric argv; arks-owned; host-tested Partial)

USAGE:
  arks repo deploy-key list [-R owner/repo] [--json <fields>]
  arks repo deploy-key add <key-file> [-t <title>] [-w] [-R owner/repo]
  arks repo deploy-key delete <key-id> [-R owner/repo]

JSON fields: id, key, title, createdAt, readOnly";

const DEPLOY_JSON: &[&str] = &["id", "key", "title", "createdAt", "readOnly"];

pub(crate) fn run_ssh(ctx: &Ctx, args: &[String], out: &mut dyn Write) -> Res {
    let sub = args.first().map(String::as_str).unwrap_or("");
    let rest = args.get(1..).unwrap_or(&[]);
    match sub {
        "list" | "ls" => {
            Args::parse(rest, &[], &[])?;
            let mut keys: Vec<Value> = arr(&ctx.api("GET", "/user/keys", None)?).to_vec();
            keys.extend(
                arr(&ctx.api("GET", "/user/ssh_signing_keys", None)?)
                    .iter()
                    .cloned(),
            );
            for k in &keys {
                row(
                    out,
                    &[
                        s(k, "title"),
                        &n(k, "id").to_string(),
                        s(k, "key"),
                        s(k, "created_at"),
                        s(k, "key_type"),
                    ],
                )?;
            }
            Ok(())
        }
        "add" => {
            let a = Args::parse(rest, &["-t", "--title", "--type"], &[])?;
            let kind = a.get(&["--type"]).unwrap_or("authentication");
            let path = match kind {
                "authentication" => "/user/keys",
                "signing" => "/user/ssh_signing_keys",
                other => {
                    return usage(format!("invalid --type {other:?} (authentication|signing)"));
                }
            };
            let text = read_input(a.positionals().first().copied())?;
            let parsed = parse_ssh_public_key(&text).map_err(Fail::Usage)?;
            let title = a.get(&["-t", "--title"]).unwrap_or(&parsed.comment);
            ctx.api(
                "POST",
                path,
                Some(&json!({"title": title, "key": parsed.normalized()})),
            )?;
            writeln!(
                out,
                "Public key added to your account ({})",
                parsed.fingerprint
            )?;
            Ok(())
        }
        "delete" => {
            let a = Args::parse(rest, &[], &["-y", "--yes"])?;
            let id = id_arg(&a, "SSH key id")?;
            for base in ["/user/keys", "/user/ssh_signing_keys"] {
                let found = arr(&ctx.api("GET", base, None)?)
                    .iter()
                    .find(|k| n(k, "id") == id)
                    .cloned();
                if let Some(k) = found {
                    ctx.api("DELETE", &format!("{base}/{id}"), None)?;
                    writeln!(
                        out,
                        "SSH key \"{}\" ({id}) deleted from your account",
                        s(&k, "title")
                    )?;
                    return Ok(());
                }
            }
            Err(Fail::Api(format!("SSH key {id} not found")))
        }
        other => usage(format!("unknown ssh-key command `{other}`\n{SSH_HELP}")),
    }
}

fn id_arg(a: &Args<'_>, what: &str) -> Result<u64, Fail> {
    let p = a
        .positionals()
        .first()
        .copied()
        .ok_or_else(|| Fail::Usage(format!("{what} required")))?;
    p.parse()
        .map_err(|_| Fail::Usage(format!("invalid {what}: {p}")))
}

pub(crate) fn run_gpg(ctx: &Ctx, args: &[String], out: &mut dyn Write) -> Res {
    let sub = args.first().map(String::as_str).unwrap_or("");
    let rest = args.get(1..).unwrap_or(&[]);
    match sub {
        "list" | "ls" => {
            Args::parse(rest, &[], &[])?;
            for k in arr(&ctx.api("GET", "/user/gpg_keys", None)?) {
                let emails: Vec<&str> = arr(k.get("emails").unwrap_or(&Value::Null))
                    .iter()
                    .map(|e| s(e, "email"))
                    .collect();
                row(
                    out,
                    &[
                        &emails.join(", "),
                        s(k, "key_id"),
                        s(k, "fingerprint"),
                        s(k, "created_at"),
                        "-",
                    ],
                )?;
            }
            Ok(())
        }
        "add" => {
            let a = Args::parse(rest, &["-t", "--title"], &[])?;
            let text = read_input(a.positionals().first().copied())?;
            let parsed = parse_pgp_public_key(&text).map_err(Fail::Usage)?;
            let mut body = json!({"armored_public_key": parsed.armored});
            if let Some(t) = a.get(&["-t", "--title"]) {
                body["name"] = json!(t);
            }
            ctx.api("POST", "/user/gpg_keys", Some(&body))?;
            writeln!(out, "GPG key {} added to your account", parsed.key_id)?;
            Ok(())
        }
        "delete" => {
            let a = Args::parse(rest, &[], &["-y", "--yes"])?;
            let Some(want) = a.positionals().first().map(|k| k.to_ascii_uppercase()) else {
                return usage("key ID required");
            };
            let keys = ctx.api("GET", "/user/gpg_keys", None)?;
            let hit: Vec<&Value> = arr(&keys)
                .iter()
                .filter(|k| s(k, "key_id") == want || s(k, "fingerprint") == want)
                .collect();
            let k = match hit.as_slice() {
                [k] => *k,
                [] => return Err(Fail::Api(format!("unable to find GPG key {want}"))),
                _ => return Err(Fail::Api(format!("key ID {want} is ambiguous"))),
            };
            ctx.api("DELETE", &format!("/user/gpg_keys/{}", n(k, "id")), None)?;
            writeln!(out, "GPG key {} deleted from your account", s(k, "key_id"))?;
            Ok(())
        }
        other => usage(format!("unknown gpg-key command `{other}`\n{GPG_HELP}")),
    }
}

fn deploy_row(k: &Value) -> Map<String, Value> {
    let mut m = Map::new();
    m.insert("id".into(), json!(n(k, "id")));
    m.insert("key".into(), json!(s(k, "key")));
    m.insert("title".into(), json!(s(k, "title")));
    m.insert("createdAt".into(), json!(s(k, "created_at")));
    m.insert(
        "readOnly".into(),
        json!(k.get("read_only").and_then(Value::as_bool).unwrap_or(true)),
    );
    m
}

pub(crate) fn run_deploy(ctx: &Ctx, args: &[String], out: &mut dyn Write) -> Res {
    let sub = args.first().map(String::as_str).unwrap_or("");
    let rest = args.get(1..).unwrap_or(&[]);
    match sub {
        "list" | "ls" => {
            let a = Args::parse(rest, &[&["-R", "--repo"][..], EXPORT].concat(), &[])?;
            let (o, r) = ctx.repo(&a)?;
            let keys = ctx.api("GET", &format!("/repos/{o}/{r}/keys"), None)?;
            let rows: Vec<Map<String, Value>> = arr(&keys).iter().map(deploy_row).collect();
            if json_out(out, &a, &rows, DEPLOY_JSON, false)? {
                return Ok(());
            }
            for m in &rows {
                let ro = m["readOnly"].as_bool().unwrap_or(true);
                row(
                    out,
                    &[
                        &m["id"].to_string(),
                        m["title"].as_str().unwrap_or(""),
                        if ro { "read-only" } else { "read-write" },
                        m["key"].as_str().unwrap_or(""),
                        m["createdAt"].as_str().unwrap_or(""),
                    ],
                )?;
            }
            Ok(())
        }
        "add" => {
            let a = Args::parse(
                rest,
                &["-R", "--repo", "-t", "--title"],
                &["-w", "--allow-write"],
            )?;
            let (o, r) = ctx.repo(&a)?;
            let Some(file) = a.positionals().first().copied() else {
                return usage("key file required (use - for stdin)");
            };
            let parsed = parse_ssh_public_key(&read_input(Some(file))?).map_err(Fail::Usage)?;
            let title = a.get(&["-t", "--title"]).unwrap_or(&parsed.comment);
            let body = json!({
                "title": title,
                "key": parsed.normalized(),
                "read_only": !a.has(&["-w", "--allow-write"]),
            });
            ctx.api("POST", &format!("/repos/{o}/{r}/keys"), Some(&body))?;
            writeln!(out, "Deploy key added to {o}/{r}")?;
            Ok(())
        }
        "delete" => {
            let a = Args::parse(rest, &["-R", "--repo"], &[])?;
            let (o, r) = ctx.repo(&a)?;
            let id = id_arg(&a, "deploy key id")?;
            ctx.api("DELETE", &format!("/repos/{o}/{r}/keys/{id}"), None)?;
            writeln!(out, "Deploy key deleted from {o}/{r}")?;
            Ok(())
        }
        other => usage(format!(
            "unknown deploy-key command `{other}`\n{DEPLOY_HELP}"
        )),
    }
}
