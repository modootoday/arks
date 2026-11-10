//! Walk commit/tree/blob closure for pack generation.

use crate::error::ServerError;
use arks_object::{Commit, ObjectKind, RawObject, Tree, TreeMode};
use arks_oid::{Oid, OidKind};
use std::collections::HashSet;

pub fn collect_closure<F>(tips: &[Oid], mut get: F) -> Result<Vec<RawObject>, ServerError>
where
    F: FnMut(&Oid) -> Result<Option<RawObject>, ServerError>,
{
    let mut seen = HashSet::new();
    let mut stack: Vec<Oid> = tips.to_vec();
    let mut out = Vec::new();
    while let Some(oid) = stack.pop() {
        if !seen.insert(oid) {
            continue;
        }
        let Some(obj) = get(&oid)? else {
            return Err(ServerError::Protocol(format!(
                "missing object {}",
                oid.to_hex()
            )));
        };
        match obj.kind {
            ObjectKind::Commit => {
                let commit = Commit::decode(&obj.payload)
                    .map_err(|e| ServerError::Protocol(e.to_string()))?;
                stack.push(commit.tree);
                stack.extend(commit.parents);
            }
            ObjectKind::Tree => {
                let tree = Tree::decode(&obj.payload, OidKind::Sha1)
                    .map_err(|e| ServerError::Protocol(e.to_string()))?;
                for entry in tree.entries {
                    if entry.mode == TreeMode::Gitlink {
                        continue;
                    }
                    stack.push(entry.oid);
                }
            }
            ObjectKind::Tag => {
                if let Ok(text) = std::str::from_utf8(&obj.payload) {
                    for line in text.lines() {
                        if line.is_empty() {
                            break;
                        }
                        if let Some(hex) = line.strip_prefix("object ") {
                            if let Ok(id) = Oid::from_hex(hex.trim()) {
                                stack.push(id);
                            }
                        }
                    }
                }
            }
            ObjectKind::Blob => {}
        }
        out.push(obj);
    }
    Ok(out)
}
