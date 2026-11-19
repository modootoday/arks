//! Walk commit/tree/blob closure for pack generation.
//!
//! Gitlink entries are not objects in this repository and are skipped.

use crate::error::RemoteError;
use arks_object::{Commit, ObjectKind, RawObject, Tree, TreeMode};
use arks_oid::{Oid, OidKind};
use std::collections::HashSet;

#[cfg_attr(not(test), allow(dead_code))]
pub(crate) fn collect_closure<F>(tips: &[Oid], get: F) -> Result<Vec<RawObject>, RemoteError>
where
    F: FnMut(&Oid) -> Result<Option<RawObject>, RemoteError>,
{
    collect_closure_shallow(tips, &HashSet::new(), false, get)
}

/// Like [`collect_closure`], treating commits in `shallow` as parentless
/// (`$GIT_DIR/shallow`, gitrepository-layout).
pub(crate) fn collect_closure_shallow<F>(
    tips: &[Oid],
    shallow: &HashSet<Oid>,
    skip_missing: bool,
    mut get: F,
) -> Result<Vec<RawObject>, RemoteError>
where
    F: FnMut(&Oid) -> Result<Option<RawObject>, RemoteError>,
{
    let mut seen = HashSet::new();
    let mut stack: Vec<Oid> = tips.to_vec();
    let mut out = Vec::new();
    while let Some(oid) = stack.pop() {
        if !seen.insert(oid) {
            continue;
        }
        let Some(obj) = get(&oid)? else {
            if skip_missing {
                continue;
            }
            return Err(RemoteError::Protocol(format!(
                "missing object {}",
                oid.to_hex()
            )));
        };
        match obj.kind {
            ObjectKind::Commit => {
                let commit = Commit::decode(&obj.payload)
                    .map_err(|e| RemoteError::Protocol(e.to_string()))?;
                stack.push(commit.tree);
                if !shallow.contains(&oid) {
                    stack.extend(commit.parents);
                }
            }
            ObjectKind::Tree => {
                let tree = Tree::decode(&obj.payload, OidKind::Sha1)
                    .map_err(|e| RemoteError::Protocol(e.to_string()))?;
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

#[cfg(test)]
mod shallow_tests {
    use super::*;
    use arks_object::Signature;
    use arks_store::ObjectFormat;
    use std::collections::HashMap;

    #[test]
    fn shallow_boundary_skips_absent_parent() {
        let sig = Signature {
            name: "a".into(),
            email: "a@x".into(),
            timestamp: 0,
            timezone: "+0000".into(),
        };
        let tree = RawObject::new(ObjectKind::Tree, Vec::new());
        let tree_oid = tree.oid(ObjectFormat::Sha1).unwrap();
        let absent_parent = Oid::from_hex("10d387ea15b63ad6b5d5d7efc1a658adf1220b43").unwrap();
        let commit = Commit {
            tree: tree_oid,
            parents: vec![absent_parent],
            author: sig.clone(),
            committer: sig,
            gpgsig: None,
            message: "m\n".into(),
        };
        let cobj = RawObject::new(ObjectKind::Commit, commit.encode().unwrap());
        let coid = cobj.oid(ObjectFormat::Sha1).unwrap();
        let mut store: HashMap<Oid, RawObject> = HashMap::new();
        store.insert(tree_oid, tree);
        store.insert(coid, cobj);
        let get = |o: &Oid| Ok(store.get(o).cloned());
        assert!(
            collect_closure(&[coid], get).is_err(),
            "deep walk must fail closed"
        );
        let shallow: HashSet<Oid> = [coid].into_iter().collect();
        let objs = collect_closure_shallow(&[coid], &shallow, false, get).unwrap();
        assert_eq!(objs.len(), 2);
    }
}
