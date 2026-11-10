//! `arks notes` — claimed Fs-3 subset (`add -m`, `show`).
//!
//! Primary: <https://git-scm.com/docs/git-notes>
//! Fanout 0 (path = full object hex) under `refs/notes/commits`.

use crate::cmd::shared::resolve_rev;
use crate::error::{CliError, CliResult};
use crate::util::{FORMAT, OID_KIND, open_repo, refuse_unknown_flags};
use arks_object::{ObjectKind, RawObject, Tree, TreeEntry, TreeMode};
use arks_repo::Repository;
use arks_store::ObjectStore;
use std::collections::HashMap;

const NOTES_REF: &str = "refs/notes/commits";

pub fn cmd_notes(args: &[String]) -> CliResult<()> {
    let sub = args
        .first()
        .map(String::as_str)
        .ok_or_else(|| CliError::Usage("notes <add|show> ...".into()))?;
    // M16 Sy-5: append/remove/list claimed; merge/edit interactive remain N/A.
    match sub {
        "add" => notes_add(&args[1..]),
        "show" => notes_show(&args[1..]),
        "append" => notes_append(&args[1..]),
        "remove" => notes_remove(&args[1..]),
        "list" => notes_list(&args[1..]),
        other => Err(CliError::NotImplemented(format!(
            "notes {other} not claimed (Sy-5: add/show/append/remove/list; merge/edit N/A)"
        ))),
    }
}

fn notes_add(args: &[String]) -> CliResult<()> {
    refuse_unknown_flags(args, &["-m"])?;
    let mut message: Option<String> = None;
    let mut positional: Vec<&str> = Vec::new();
    let mut i = 0;
    while i < args.len() {
        if args[i] == "-m" {
            let msg = args
                .get(i + 1)
                .ok_or_else(|| CliError::Usage("notes add -m <msg> [<object>]".into()))?;
            message = Some(msg.clone());
            i += 2;
            continue;
        }
        if args[i].starts_with('-') {
            return Err(CliError::NotImplemented(format!(
                "unsupported option: {}",
                args[i]
            )));
        }
        positional.push(args[i].as_str());
        i += 1;
    }
    let message = message.ok_or_else(|| CliError::Usage("notes add -m <msg> [<object>]".into()))?;
    let mut repo = open_repo()?;
    refuse_notes_contention(&repo)?;
    let target = match positional.first() {
        Some(rev) => resolve_rev(&repo, rev)?,
        None => repo.head_oid()?,
    };
    let blob = RawObject::new(ObjectKind::Blob, message.into_bytes());
    let note_oid = repo.store_mut().put(FORMAT, &blob)?;
    let mut entries = load_notes_entries(&repo)?;
    let key = target.to_hex();
    entries.insert(
        key.clone(),
        TreeEntry {
            mode: TreeMode::File,
            name: key,
            oid: note_oid,
        },
    );
    let tree = Tree::new(entries.into_values().collect())?;
    let tree_bytes = tree.encode()?;
    let tree_obj = RawObject::new(ObjectKind::Tree, tree_bytes);
    let tree_oid = repo.store_mut().put(FORMAT, &tree_obj)?;
    let old = repo.refs().resolve(NOTES_REF).ok();
    repo.refs_mut()
        .update(NOTES_REF, &tree_oid, old.as_ref())?;
    Ok(())
}

fn notes_show(args: &[String]) -> CliResult<()> {
    refuse_unknown_flags(args, &[])?;
    let repo = open_repo()?;
    let target = match args.first() {
        Some(rev) => resolve_rev(&repo, rev)?,
        None => repo.head_oid()?,
    };
    let entries = load_notes_entries(&repo)?;
    let entry = entries.get(&target.to_hex()).ok_or_else(|| {
        CliError::Msg(format!(
            "no note found for object {}",
            target.to_hex()
        ))
    })?;
    let obj = repo.get_object(&entry.oid)?;
    if obj.kind != ObjectKind::Blob {
        return Err(CliError::Msg("notes show: note object is not a blob".into()));
    }
    let text = String::from_utf8_lossy(&obj.payload);
    if text.ends_with('\n') {
        print!("{text}");
    } else {
        println!("{text}");
    }
    Ok(())
}

fn notes_append(args: &[String]) -> CliResult<()> {
    refuse_unknown_flags(args, &["-m"])?;
    let mut message: Option<String> = None;
    let mut positional: Vec<&str> = Vec::new();
    let mut i = 0;
    while i < args.len() {
        if args[i] == "-m" {
            let msg = args
                .get(i + 1)
                .ok_or_else(|| CliError::Usage("notes append -m <msg> [<object>]".into()))?;
            message = Some(msg.clone());
            i += 2;
            continue;
        }
        if args[i].starts_with('-') {
            return Err(CliError::NotImplemented(format!(
                "unsupported option: {}",
                args[i]
            )));
        }
        positional.push(args[i].as_str());
        i += 1;
    }
    let message = message.ok_or_else(|| CliError::Usage("notes append -m <msg> [<object>]".into()))?;
    let mut repo = open_repo()?;
    refuse_notes_contention(&repo)?;
    let target = match positional.first() {
        Some(rev) => resolve_rev(&repo, rev)?,
        None => repo.head_oid()?,
    };
    let key = target.to_hex();
    let mut entries = load_notes_entries(&repo)?;
    let mut body = if let Some(existing) = entries.get(&key) {
        let obj = repo.get_object(&existing.oid)?;
        let mut b = obj.payload.clone();
        if !b.ends_with(b"\n") {
            b.push(b'\n');
        }
        b.extend_from_slice(message.as_bytes());
        b
    } else {
        message.into_bytes()
    };
    if !body.ends_with(b"\n") {
        body.push(b'\n');
    }
    let blob = RawObject::new(ObjectKind::Blob, body);
    let note_oid = repo.store_mut().put(FORMAT, &blob)?;
    entries.insert(
        key.clone(),
        TreeEntry {
            mode: TreeMode::File,
            name: key,
            oid: note_oid,
        },
    );
    write_notes_tree(&mut repo, entries)?;
    Ok(())
}

fn notes_remove(args: &[String]) -> CliResult<()> {
    refuse_unknown_flags(args, &[])?;
    let mut repo = open_repo()?;
    refuse_notes_contention(&repo)?;
    let target = match args.first() {
        Some(rev) => resolve_rev(&repo, rev)?,
        None => repo.head_oid()?,
    };
    let key = target.to_hex();
    let mut entries = load_notes_entries(&repo)?;
    if entries.remove(&key).is_none() {
        return Err(CliError::Msg(format!(
            "no note found for object {}",
            target.to_hex()
        )));
    }
    write_notes_tree(&mut repo, entries)?;
    Ok(())
}

fn notes_list(args: &[String]) -> CliResult<()> {
    refuse_unknown_flags(args, &[])?;
    let repo = open_repo()?;
    let entries = load_notes_entries(&repo)?;
    let mut keys: Vec<_> = entries.keys().cloned().collect();
    keys.sort();
    for k in keys {
        if let Some(e) = entries.get(&k) {
            println!("{} {k}", e.oid.to_hex());
        }
    }
    Ok(())
}

/// REF-016 / MISC-017 (M20 Cx-3): notes ref lock + in-progress notes-merge state.
/// Cite: https://git-scm.com/docs/git-notes + api-lockfile
fn refuse_notes_contention(repo: &Repository) -> CliResult<()> {
    let git = repo.git_dir();
    let common = repo.common_dir();
    if git.join("NOTES_MERGE_WORKTREE").exists()
        || git.join("NOTES_MERGE_PARTIAL").is_file()
        || git.join("NOTES_MERGE_REF").is_file()
    {
        return Err(CliError::Msg(
            "notes refused: notes-merge state present (MISC-017 — concurrent notes merge; finish or abort; no dual-merge dialect)".into(),
        ));
    }
    let notes_lock = common.join("refs/notes/commits.lock");
    if notes_lock.is_file() {
        return Err(CliError::Msg(format!(
            "Unable to create '{}' (REF-016 — concurrent notes ref update fail-closed; lease ≠ skip)",
            notes_lock.display()
        )));
    }
    Ok(())
}

fn write_notes_tree(
    repo: &mut Repository,
    entries: HashMap<String, TreeEntry>,
) -> CliResult<()> {
    let tree = Tree::new(entries.into_values().collect())?;
    let tree_bytes = tree.encode()?;
    let tree_obj = RawObject::new(ObjectKind::Tree, tree_bytes);
    let tree_oid = repo.store_mut().put(FORMAT, &tree_obj)?;
    let old = repo.refs().resolve(NOTES_REF).ok();
    repo.refs_mut()
        .update(NOTES_REF, &tree_oid, old.as_ref())?;
    Ok(())
}

fn load_notes_entries(repo: &Repository) -> CliResult<HashMap<String, TreeEntry>> {
    let Ok(tree_oid) = repo.refs().resolve(NOTES_REF) else {
        return Ok(HashMap::new());
    };
    let obj = repo.get_object(&tree_oid)?;
    if obj.kind != ObjectKind::Tree {
        return Err(CliError::Msg(format!(
            "{NOTES_REF} does not point to a tree"
        )));
    }
    let tree = Tree::decode(&obj.payload, OID_KIND)?;
    let mut map = HashMap::new();
    for entry in tree.entries {
        // Fanout 0: entry name is the full object hex.
        // Fanout 1 (stock may rewrite): single-level dir + leaf — flatten for show/add.
        if entry.mode.is_tree() {
            let nested = repo.get_object(&entry.oid)?;
            let nested_tree = Tree::decode(&nested.payload, OID_KIND)?;
            for leaf in nested_tree.entries {
                let name = format!("{}{}", entry.name, leaf.name);
                map.insert(
                    name.clone(),
                    TreeEntry {
                        mode: leaf.mode,
                        name,
                        oid: leaf.oid,
                    },
                );
            }
        } else {
            map.insert(entry.name.clone(), entry);
        }
    }
    Ok(map)
}
