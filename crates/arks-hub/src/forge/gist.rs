//! Gists: named file maps owned by the forge user (`/gists`, `/gists/{id}`).
//!
//! arks stores a gist as a file map with metadata; it has no git history. PATCH
//! follows the REST shape: a `null` file entry deletes, `filename` renames,
//! `content` replaces or adds.

use super::{Req, RouteResult, invalid, iso8601, next_id, no_content, not_found};
use crate::store::{HubStore, now_secs};
use serde::{Deserialize, Deserializer, Serialize};
use serde_json::{Map, Value, json};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Gist {
    pub id: String,
    pub description: String,
    pub public: bool,
    /// File name → content. Older db files stored a bare name list.
    #[serde(deserialize_with = "de_files")]
    pub files: BTreeMap<String, GistFile>,
    #[serde(default)]
    pub owner: String,
    #[serde(default)]
    pub created_at: u64,
    #[serde(default)]
    pub updated_at: u64,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct GistFile {
    #[serde(default)]
    pub content: String,
}

fn de_files<'de, D: Deserializer<'de>>(d: D) -> Result<BTreeMap<String, GistFile>, D::Error> {
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum Files {
        Names(Vec<String>),
        Map(BTreeMap<String, GistFile>),
    }
    Ok(match Files::deserialize(d)? {
        Files::Names(names) => names
            .into_iter()
            .map(|n| (n, GistFile::default()))
            .collect(),
        Files::Map(m) => m,
    })
}

pub(crate) fn gist_json(g: &Gist, owner_fallback: &str) -> Value {
    let files: Map<String, Value> = g
        .files
        .iter()
        .map(|(name, f)| {
            (
                name.clone(),
                json!({
                    "filename": name,
                    "size": f.content.len(),
                    "content": f.content,
                }),
            )
        })
        .collect();
    let owner = if g.owner.is_empty() {
        owner_fallback
    } else {
        &g.owner
    };
    json!({
        "id": g.id,
        "description": g.description,
        "public": g.public,
        "files": files,
        "owner": {"login": owner},
        "created_at": iso8601(g.created_at),
        "updated_at": iso8601(g.updated_at),
    })
}

/// Validate a file name: non-empty, no path separators.
fn valid_name(name: &str) -> Result<(), String> {
    if name.trim().is_empty() {
        return Err("file name can't be blank".into());
    }
    if name.contains('/') || name.contains('\\') || name == "." || name == ".." {
        return Err(format!("file name `{name}` must not contain a path"));
    }
    Ok(())
}

impl HubStore {
    pub(crate) fn create_gist(
        &mut self,
        description: &str,
        public: bool,
        files: BTreeMap<String, GistFile>,
    ) -> Gist {
        let id = format!("gist{}", next_id(&mut self.db.next_gist));
        let now = now_secs();
        let g = Gist {
            id,
            description: description.into(),
            public,
            files,
            owner: self.actor.clone(),
            created_at: now,
            updated_at: now,
        };
        self.db.gists.push(g.clone());
        g
    }
}

/// Apply a REST `files` patch object to `files`.
fn apply_files_patch(
    files: &mut BTreeMap<String, GistFile>,
    patch: &Map<String, Value>,
) -> Result<(), String> {
    for (name, change) in patch {
        if change.is_null() {
            if files.remove(name).is_none() {
                return Err(format!("file `{name}` not found in gist"));
            }
            continue;
        }
        let content = change.get("content").and_then(Value::as_str);
        let rename = change
            .get("filename")
            .and_then(Value::as_str)
            .filter(|n| *n != name);
        match files.get(name).cloned() {
            Some(mut existing) => {
                if let Some(c) = content {
                    if c.trim().is_empty() {
                        return Err(format!("content of `{name}` can't be blank"));
                    }
                    existing.content = c.to_string();
                }
                if let Some(new_name) = rename {
                    valid_name(new_name)?;
                    if files.contains_key(new_name) {
                        return Err(format!("file `{new_name}` already exists in gist"));
                    }
                    files.remove(name);
                    files.insert(new_name.to_string(), existing);
                } else {
                    files.insert(name.clone(), existing);
                }
            }
            None => {
                valid_name(name)?;
                let c = content
                    .filter(|c| !c.trim().is_empty())
                    .ok_or_else(|| format!("content of new file `{name}` can't be blank"))?;
                let target = rename.unwrap_or(name);
                valid_name(target)?;
                files.insert(
                    target.to_string(),
                    GistFile {
                        content: c.to_string(),
                    },
                );
            }
        }
    }
    if files.is_empty() {
        return Err("a gist must keep at least one file".into());
    }
    Ok(())
}

pub(crate) fn route(st: &mut HubStore, req: &Req<'_>) -> RouteResult {
    if !(req.path == "/gists" || req.path.starts_with("/gists/")) {
        return Ok(None);
    }
    let Some(seg) = super::segments(req.path, "/gists") else {
        return Ok(None);
    };
    let login = st.db.user_login.clone();
    // Owner of a stored gist (empty in older db files = the forge's default user).
    let owned = |g: &Gist, who: &str| {
        (if g.owner.is_empty() {
            login.as_str()
        } else {
            g.owner.as_str()
        }) == who
    };
    let me = st.actor.clone();
    match (req.method, seg.as_slice()) {
        ("GET", []) => {
            let mut items: Vec<&Gist> = st.db.gists.iter().filter(|g| owned(g, &me)).collect();
            items.sort_by(|a, b| b.updated_at.cmp(&a.updated_at).then(b.id.cmp(&a.id)));
            let arr = items.into_iter().map(|g| gist_json(g, &login)).collect();
            Ok(Some((200, Value::Array(arr))))
        }
        ("POST", []) => {
            let v = req.json();
            let Some(obj) = v.get("files").and_then(Value::as_object) else {
                return Ok(Some(invalid(
                    "files must be an object of {name: {content}}",
                )));
            };
            if obj.is_empty() {
                return Ok(Some(invalid("a gist needs at least one file")));
            }
            let mut files = BTreeMap::new();
            for (name, f) in obj {
                if let Err(e) = valid_name(name) {
                    return Ok(Some(invalid(e)));
                }
                let Some(content) = f
                    .get("content")
                    .and_then(Value::as_str)
                    .filter(|c| !c.trim().is_empty())
                else {
                    return Ok(Some(invalid(format!("content of `{name}` can't be blank"))));
                };
                files.insert(
                    name.clone(),
                    GistFile {
                        content: content.to_string(),
                    },
                );
            }
            let desc = v.get("description").and_then(Value::as_str).unwrap_or("");
            let public = v.get("public").and_then(Value::as_bool).unwrap_or(false);
            let g = st.create_gist(desc, public, files);
            st.save()?;
            Ok(Some((201, gist_json(&g, &login))))
        }
        ("GET", [id]) => Ok(Some(
            st.db
                .gists
                .iter()
                .find(|g| g.id == *id && (g.public || owned(g, &me)))
                .map(|g| (200, gist_json(g, &login)))
                .unwrap_or_else(not_found),
        )),
        ("PATCH", [id]) => {
            let v = req.json();
            let Some(g) = st
                .db
                .gists
                .iter_mut()
                .find(|g| g.id == *id && owned(g, &me))
            else {
                return Ok(Some(not_found()));
            };
            let mut files = g.files.clone();
            if let Some(patch) = v.get("files").and_then(Value::as_object) {
                if let Err(e) = apply_files_patch(&mut files, patch) {
                    return Ok(Some(invalid(e)));
                }
            }
            g.files = files;
            if let Some(d) = v.get("description").and_then(Value::as_str) {
                g.description = d.to_string();
            }
            g.updated_at = now_secs();
            let out = gist_json(g, &login);
            st.save()?;
            Ok(Some((200, out)))
        }
        ("DELETE", [id]) => {
            let Some(pos) = st
                .db
                .gists
                .iter()
                .position(|g| g.id == *id && owned(g, &me))
            else {
                return Ok(Some(not_found()));
            };
            st.db.gists.remove(pos);
            st.save()?;
            Ok(Some(no_content()))
        }
        _ => Ok(Some(super::method_not_allowed())),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn legacy_name_list_loads_as_empty_files() {
        let g: Gist = serde_json::from_str(
            r#"{"id":"gist1","description":"d","public":true,"files":["a.txt","b.md"]}"#,
        )
        .expect("legacy gist");
        assert_eq!(g.files.len(), 2);
        assert!(g.files["a.txt"].content.is_empty());
        assert!(g.owner.is_empty());
    }

    #[test]
    fn files_patch_rename_delete_add() {
        let mut files = BTreeMap::new();
        files.insert(
            "a.txt".to_string(),
            GistFile {
                content: "A".into(),
            },
        );
        files.insert(
            "b.txt".to_string(),
            GistFile {
                content: "B".into(),
            },
        );
        let patch: Value = json!({
            "a.txt": {"filename": "c.txt"},
            "b.txt": null,
            "d.txt": {"content": "D"},
        });
        apply_files_patch(&mut files, patch.as_object().unwrap()).unwrap();
        assert_eq!(
            files.keys().cloned().collect::<Vec<_>>(),
            ["c.txt", "d.txt"]
        );
        assert_eq!(files["c.txt"].content, "A");
        let empty_all: Value = json!({"c.txt": null, "d.txt": null});
        assert!(apply_files_patch(&mut files, empty_all.as_object().unwrap()).is_err());
    }
}
