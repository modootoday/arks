//! receive-pack service (push path).
//!
//! Claimed caps: `report-status`, `report-status-v2` (classic ok/ng; option
//! lines reserved for future proc-receive rewrites), `delete-refs`, `agent`.
//! Cite: <https://git-scm.com/docs/gitprotocol-capabilities>,
//! <https://git-scm.com/docs/git-receive-pack> (quarantine environment).

use crate::advertise::{encode_http_advertisement, encode_pack_advertisement};
use crate::error::ServerError;
use crate::hooks::{
    HookEnv, HookGate, run_post_receive, run_post_update, run_pre_receive, run_update,
};
use arks_oid::Oid;
use arks_pack::DecodedPack;
use arks_protocol::{Packet, decode_packet, decode_text_line, encode_packet};
use arks_refs::RefTarget;
use arks_repo::Repository;
use arks_store::{FilesystemGitStore, ObjectFormat, ObjectStore};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

const FORMAT: ObjectFormat = ObjectFormat::Sha1;

fn receive_refs_and_caps(
    repo: &Repository,
) -> Result<(Vec<(String, Oid)>, Vec<String>), ServerError> {
    let mut refs = repo
        .refs()
        .list_refs("refs/")
        .map_err(|e| ServerError::Msg(e.to_string()))?;
    refs.sort_by(|a, b| a.0.cmp(&b.0));
    if let Ok(oid) = repo.refs().resolve("HEAD") {
        if !refs.iter().any(|(n, _)| n == "HEAD") {
            refs.insert(0, ("HEAD".into(), oid));
        }
    } else if let Ok(RefTarget::Direct(oid)) = repo.head_target() {
        refs.insert(0, ("HEAD".into(), oid));
    }
    // Advertise only real caps. report-status-v2 without option lines matches
    // stock receive-pack when no proc-receive rewrite applies.
    // Cite: https://git-scm.com/docs/gitprotocol-capabilities
    let caps = vec![
        "report-status".into(),
        "report-status-v2".into(),
        "delete-refs".into(),
        "agent=arks/0.0.0".into(),
    ];
    Ok((refs, caps))
}

/// Smart-HTTP receive-pack advertisement.
pub fn receive_advertisement(repo: &Repository) -> Result<Vec<u8>, ServerError> {
    let (refs, caps) = receive_refs_and_caps(repo)?;
    encode_http_advertisement("git-receive-pack", &caps, &refs)
}

/// Pack-protocol duplex receive-pack advertisement (SSH / stdio).
pub fn receive_pack_advertisement(repo: &Repository) -> Result<Vec<u8>, ServerError> {
    let (refs, caps) = receive_refs_and_caps(repo)?;
    encode_pack_advertisement(&caps, &refs)
}

pub fn receive_pack(repo: &mut Repository, body: &[u8]) -> Result<Vec<u8>, ServerError> {
    let (commands, pack) = split_commands(body)?;
    let quarantine = Quarantine::create(repo.git_dir())?;
    let hook_env = HookEnv {
        quarantine_path: Some(quarantine.path.clone()),
    };

    if !pack.is_empty() {
        if !pack.starts_with(b"PACK") {
            quarantine.discard();
            return encode_report_status("unpack unpacker error", &[]);
        }
        let decoded = match DecodedPack::decode(&pack, FORMAT) {
            Ok(d) => d,
            Err(e) => {
                quarantine.discard();
                return encode_report_status(&format!("unpack {e}"), &[]);
            }
        };
        let mut qstore = FilesystemGitStore::new(&quarantine.path)
            .map_err(|e| ServerError::Msg(e.to_string()))?;
        for object in decoded.objects {
            qstore
                .put(FORMAT, &object.object)
                .map_err(|e| ServerError::Msg(e.to_string()))?;
        }
    }

    // Honor list: pre-receive once after unpack, before any ref update.
    // Cite: https://git-scm.com/docs/githooks , https://git-scm.com/docs/git-receive-pack
    let pre_stdin = hook_stdin_lines(&commands);
    if run_pre_receive(repo.git_dir(), &pre_stdin, &hook_env)? == HookGate::Decline {
        quarantine.discard();
        let declined: Vec<(String, Result<(), String>)> = commands
            .iter()
            .map(|cmd| (cmd.name.clone(), Err("pre-receive hook declined".into())))
            .collect();
        return encode_report_status_owned("ok", &declined);
    }

    // Migrate quarantined objects into the main store only after pre-receive.
    quarantine
        .migrate_into(repo)
        .map_err(|e| ServerError::Msg(e))?;

    let mut results: Vec<(String, Result<(), String>)> = Vec::new();
    let mut success_cmds: Vec<&ReceiveCommand> = Vec::new();
    for cmd in &commands {
        let old = match repo.refs().resolve(&cmd.name) {
            Ok(oid) => Some(oid),
            Err(_) => None,
        };
        // Expect old matches when peer sent non-zero old (fast-forward check deferred:
        // accept if old is None or matches cmd.old, else refuse).
        if !cmd.old.as_bytes().iter().all(|b| *b == 0) {
            match old {
                Some(oid) if oid != cmd.old => {
                    results.push((cmd.name.clone(), Err("non-fast-forward".into())));
                    continue;
                }
                None => {
                    results.push((cmd.name.clone(), Err("old oid mismatch".into())));
                    continue;
                }
                _ => {}
            }
        }
        let old_hex = cmd.old.to_hex();
        let new_hex = cmd.new.to_hex();
        if run_update(repo.git_dir(), &cmd.name, &old_hex, &new_hex, &hook_env)?
            == HookGate::Decline
        {
            results.push((cmd.name.clone(), Err("hook declined".into())));
            continue;
        }
        if is_delete(cmd) {
            match repo.refs_mut().delete(&cmd.name, old.as_ref()) {
                Ok(()) => {
                    results.push((cmd.name.clone(), Ok(())));
                    success_cmds.push(cmd);
                }
                Err(e) => results.push((cmd.name.clone(), Err(e.to_string()))),
            }
            continue;
        }
        match repo.refs_mut().update(&cmd.name, &cmd.new, old.as_ref()) {
            Ok(()) => {
                results.push((cmd.name.clone(), Ok(())));
                success_cmds.push(cmd);
            }
            Err(e) => results.push((cmd.name.clone(), Err(e.to_string()))),
        }
    }

    if !success_cmds.is_empty() {
        let post_stdin = hook_stdin_lines_refs(success_cmds.iter().copied());
        run_post_receive(repo.git_dir(), &post_stdin, &hook_env);
        let names: Vec<&str> = success_cmds.iter().map(|c| c.name.as_str()).collect();
        run_post_update(repo.git_dir(), &names, &hook_env);
    }

    encode_report_status_owned("ok", &results)
}

/// Quarantine object dir under `$GIT_DIR/objects` (git-receive-pack).
struct Quarantine {
    path: PathBuf,
}

impl Quarantine {
    fn create(git_dir: &Path) -> Result<Self, ServerError> {
        let objects = git_dir.join("objects");
        fs::create_dir_all(&objects)?;
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0);
        let path = objects.join(format!("incoming-{nanos}-{}", std::process::id()));
        fs::create_dir_all(&path)?;
        Ok(Self { path })
    }

    fn discard(self) {
        let _ = fs::remove_dir_all(&self.path);
    }

    fn migrate_into(self, repo: &mut Repository) -> Result<(), String> {
        // Walk quarantine loose objects and put into the main store.
        let Ok(entries) = fs::read_dir(&self.path) else {
            let _ = fs::remove_dir_all(&self.path);
            return Ok(());
        };
        for entry in entries.flatten() {
            let name = entry.file_name();
            let name = name.to_string_lossy();
            if name.len() != 2 {
                continue;
            }
            let fanout = entry.path();
            let Ok(objs) = fs::read_dir(&fanout) else {
                continue;
            };
            for obj in objs.flatten() {
                let fname = obj.file_name();
                let rest = fname.to_string_lossy();
                let hex = format!("{name}{rest}");
                let Ok(oid) = Oid::from_hex(&hex) else {
                    continue;
                };
                // Read via a store rooted at quarantine so packed/overlay stays honest.
                let qstore = FilesystemGitStore::open(&self.path);
                match qstore.get(&oid) {
                    Ok(raw) => {
                        repo.store_mut()
                            .put(FORMAT, &raw)
                            .map_err(|e| e.to_string())?;
                    }
                    Err(_) => continue,
                }
            }
        }
        let _ = fs::remove_dir_all(&self.path);
        Ok(())
    }
}

fn is_delete(cmd: &ReceiveCommand) -> bool {
    cmd.new.as_bytes().iter().all(|b| *b == 0)
}

fn hook_stdin_lines(commands: &[ReceiveCommand]) -> Vec<u8> {
    hook_stdin_lines_refs(commands.iter())
}

fn hook_stdin_lines_refs<'a, I>(commands: I) -> Vec<u8>
where
    I: IntoIterator<Item = &'a ReceiveCommand>,
{
    let mut out = Vec::new();
    for cmd in commands {
        out.extend(format!("{} {} {}\n", cmd.old.to_hex(), cmd.new.to_hex(), cmd.name).as_bytes());
    }
    out
}

struct ReceiveCommand {
    old: Oid,
    new: Oid,
    name: String,
}

fn split_commands(body: &[u8]) -> Result<(Vec<ReceiveCommand>, Vec<u8>), ServerError> {
    let mut offset = 0;
    let mut commands = Vec::new();
    while offset < body.len() {
        let (packet, n) =
            decode_packet(&body[offset..]).map_err(|e| ServerError::Protocol(e.to_string()))?;
        offset += n;
        match packet {
            Packet::Flush => break,
            Packet::Data(payload) => {
                let owned = if let Some(nul) = payload.iter().position(|b| *b == 0) {
                    std::str::from_utf8(&payload[..nul])
                        .map_err(|_| ServerError::Protocol("command not utf-8".into()))?
                        .to_string()
                } else {
                    decode_text_line(&payload).map_err(|e| ServerError::Protocol(e.to_string()))?
                };
                let text = owned.trim_end_matches('\n');
                let mut parts = text.splitn(3, ' ');
                let old_hex = parts
                    .next()
                    .ok_or_else(|| ServerError::Protocol("bad command".into()))?;
                let new_hex = parts
                    .next()
                    .ok_or_else(|| ServerError::Protocol("bad command".into()))?;
                let name = parts
                    .next()
                    .ok_or_else(|| ServerError::Protocol("bad command".into()))?;
                commands.push(ReceiveCommand {
                    old: Oid::from_hex(old_hex)
                        .map_err(|e| ServerError::Protocol(e.to_string()))?,
                    new: Oid::from_hex(new_hex)
                        .map_err(|e| ServerError::Protocol(e.to_string()))?,
                    name: name.to_string(),
                });
            }
            _ => {}
        }
    }
    Ok((commands, body[offset..].to_vec()))
}

fn encode_report_status_owned(
    unpack: &str,
    refs: &[(String, Result<(), String>)],
) -> Result<Vec<u8>, ServerError> {
    let borrowed: Vec<(&str, Result<(), &str>)> = refs
        .iter()
        .map(|(n, r)| {
            (
                n.as_str(),
                match r {
                    Ok(()) => Ok(()),
                    Err(e) => Err(e.as_str()),
                },
            )
        })
        .collect();
    encode_report_status(unpack, &borrowed)
}

fn encode_report_status(
    unpack: &str,
    refs: &[(&str, Result<(), &str>)],
) -> Result<Vec<u8>, ServerError> {
    // Classic unpack/ok/ng lines are valid report-status and report-status-v2
    // when no proc-receive rewrite options are needed.
    let mut out = Vec::new();
    out.extend(text_pkt(&format!("unpack {unpack}"))?);
    for (name, result) in refs {
        match result {
            Ok(()) => out.extend(text_pkt(&format!("ok {name}"))?),
            Err(err) => out.extend(text_pkt(&format!("ng {name} {err}"))?),
        }
    }
    out.extend(encode_packet(&Packet::Flush).map_err(|e| ServerError::Protocol(e.to_string()))?);
    Ok(out)
}

fn text_pkt(text: &str) -> Result<Vec<u8>, ServerError> {
    let mut payload = text.as_bytes().to_vec();
    if !payload.ends_with(b"\n") {
        payload.push(b'\n');
    }
    encode_packet(&Packet::Data(payload)).map_err(|e| ServerError::Protocol(e.to_string()))
}
