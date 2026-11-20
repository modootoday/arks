//! Git content behind forge repositories (arks-owned; in-process, never spawns git).
//!
//! A forge repository `owner/name` maps to a Git directory under the forge git
//! root: `<root>/<owner>/<name>.git`, `<root>/<owner>/<name>` (bare or with a
//! `.git`), then `<root>/<name>.git` / `<root>/<name>`. The layout of each
//! directory is plain `gitrepository-layout(5)`; objects are read and written with
//! the arks object/ref crates (loose objects, `refs/heads/*`). Forge features that
//! need content (contents API, code and commit search, pull-request diff and merge,
//! fork, sync, repository bootstrap files) use this module; without a Git
//! directory they stay metadata-only and say so.
//!
//! Primary: <https://git-scm.com/docs/gitrepository-layout>,
//! <https://git-scm.com/docs/git-read-tree> (3-way merge, "trivial merge" cases).

use arks_object::{Commit, ObjectKind, RawObject, Signature, Tree, TreeEntry, TreeMode};
use arks_oid::Oid;
use arks_refs::RefTarget;
use arks_repo::Repository;
use arks_store::ObjectStore;
use std::collections::{BTreeMap, BTreeSet, HashSet};
use std::fs;
use std::path::{Path, PathBuf};

/// Path → (mode, blob oid) for every non-tree entry of a commit.
pub(crate) type Flat = BTreeMap<String, (TreeMode, Oid)>;

/// Locate the Git directory for `owner/name` under `root`.
pub(crate) fn repo_dir(root: &Path, owner: &str, name: &str) -> Option<PathBuf> {
    let cands = [
        root.join(owner).join(format!("{name}.git")),
        root.join(owner).join(name),
        root.join(format!("{name}.git")),
        root.join(name),
    ];
    cands.into_iter().find_map(|d| {
        if d.join("HEAD").is_file() && d.join("objects").is_dir() {
            Some(d)
        } else if d.join(".git").join("HEAD").is_file() {
            Some(d.join(".git"))
        } else {
            None
        }
    })
}

/// Create an empty bare repository `<root>/<owner>/<name>.git` (HEAD → `branch`).
pub(crate) fn init_bare(
    root: &Path,
    owner: &str,
    name: &str,
    branch: &str,
) -> Result<PathBuf, String> {
    let dir = root.join(owner).join(format!("{name}.git"));
    for sub in [
        "objects/info",
        "objects/pack",
        "refs/heads",
        "refs/tags",
        "info",
    ] {
        fs::create_dir_all(dir.join(sub)).map_err(|e| format!("init {}: {e}", dir.display()))?;
    }
    let w = |p: &str, s: String| fs::write(dir.join(p), s).map_err(|e| e.to_string());
    if !dir.join("HEAD").is_file() {
        w("HEAD", format!("ref: refs/heads/{branch}\n"))?;
    }
    if !dir.join("config").is_file() {
        w(
            "config",
            "[core]\n\trepositoryformatversion = 0\n\tfilemode = true\n\tbare = true\n".into(),
        )?;
    }
    if !dir.join("description").is_file() {
        w("description", format!("{owner}/{name} (arks forge)\n"))?;
    }
    Ok(dir)
}

/// Recursive directory copy (forks copy the source Git directory as-is).
pub(crate) fn copy_dir(src: &Path, dst: &Path) -> Result<(), String> {
    fs::create_dir_all(dst).map_err(|e| e.to_string())?;
    for e in fs::read_dir(src).map_err(|e| e.to_string())? {
        let e = e.map_err(|e| e.to_string())?;
        let to = dst.join(e.file_name());
        if e.file_type().map_err(|e| e.to_string())?.is_dir() {
            copy_dir(&e.path(), &to)?;
        } else if !to.exists() {
            fs::copy(e.path(), &to).map_err(|e| e.to_string())?;
        }
    }
    Ok(())
}

/// Union the object directories of two repositories (content-addressed files, so
/// copying absent loose objects and packs is always safe).
pub(crate) fn union_objects(src: &Path, dst: &Path) -> Result<(), String> {
    copy_dir(&src.join("objects"), &dst.join("objects"))
}

pub(crate) struct GitRepo {
    repo: Repository,
}

fn err<E: std::fmt::Display>(e: E) -> String {
    e.to_string()
}

impl GitRepo {
    pub fn open(dir: &Path) -> Result<Self, String> {
        Ok(Self {
            repo: Repository::open(dir).map_err(err)?,
        })
    }

    /// Branch HEAD points at (`main` when unreadable).
    pub fn default_branch(&self) -> String {
        match self.repo.head_target() {
            Ok(RefTarget::Symbolic(t)) => t.trim_start_matches("refs/heads/").to_string(),
            _ => "main".into(),
        }
    }

    pub fn branches(&self) -> Vec<(String, Oid)> {
        self.repo
            .refs()
            .list_refs("refs/heads/")
            .unwrap_or_default()
            .into_iter()
            .map(|(n, o)| (n.trim_start_matches("refs/heads/").to_string(), o))
            .collect()
    }

    /// Resolve a branch, tag, `refs/…` name, `HEAD`, or a full object id to a commit.
    pub fn resolve(&self, rev: &str) -> Option<Oid> {
        let refs = self.repo.refs();
        let rev = rev.trim();
        let direct = if rev.is_empty() || rev == "HEAD" {
            refs.resolve("HEAD").ok()
        } else {
            [
                rev.to_string(),
                format!("refs/heads/{rev}"),
                format!("refs/tags/{rev}"),
            ]
            .iter()
            .find_map(|n| refs.resolve(n).ok())
            .or_else(|| Oid::from_hex(rev).ok())
        }?;
        self.peel_commit(direct)
    }

    fn peel_commit(&self, mut oid: Oid) -> Option<Oid> {
        for _ in 0..8 {
            let obj = self.repo.get_object(&oid).ok()?;
            match obj.kind {
                ObjectKind::Commit => return Some(oid),
                ObjectKind::Tag => {
                    let text = String::from_utf8_lossy(&obj.payload).into_owned();
                    oid = text
                        .lines()
                        .find_map(|l| l.strip_prefix("object "))
                        .and_then(|h| Oid::from_hex(h.trim()).ok())?;
                }
                _ => return None,
            }
        }
        None
    }

    pub fn commit(&self, oid: &Oid) -> Result<Commit, String> {
        let obj = self.repo.get_object(oid).map_err(err)?;
        Commit::decode(&obj.payload).map_err(err)
    }

    pub fn tree(&self, oid: &Oid) -> Result<Tree, String> {
        let obj = self.repo.get_object(oid).map_err(err)?;
        Tree::decode(&obj.payload, self.repo.oid_kind()).map_err(err)
    }

    pub fn blob(&self, oid: &Oid) -> Result<Vec<u8>, String> {
        Ok(self.repo.get_object(oid).map_err(err)?.payload)
    }

    /// Entry at `path` in the commit's tree; `""` is the root tree.
    pub fn lookup(&self, commit: &Oid, path: &str) -> Result<Option<(TreeMode, Oid)>, String> {
        let mut cur = (TreeMode::Tree, self.commit(commit)?.tree);
        for part in path.split('/').filter(|p| !p.is_empty()) {
            if cur.0 != TreeMode::Tree {
                return Ok(None);
            }
            let t = self.tree(&cur.1)?;
            match t.entries.into_iter().find(|e| e.name == part) {
                Some(e) => cur = (e.mode, e.oid),
                None => return Ok(None),
            }
        }
        Ok(Some(cur))
    }

    pub fn flatten(&self, commit: &Oid) -> Result<Flat, String> {
        let mut out = Flat::new();
        let root = self.commit(commit)?.tree;
        self.flatten_tree(&root, "", &mut out)?;
        Ok(out)
    }

    fn flatten_tree(&self, tree: &Oid, prefix: &str, out: &mut Flat) -> Result<(), String> {
        for e in self.tree(tree)?.entries {
            let path = if prefix.is_empty() {
                e.name.clone()
            } else {
                format!("{prefix}/{}", e.name)
            };
            if e.mode == TreeMode::Tree {
                self.flatten_tree(&e.oid, &path, out)?;
            } else {
                out.insert(path, (e.mode, e.oid));
            }
        }
        Ok(())
    }

    /// Commits reachable from `tips`, newest committer time first.
    pub fn log(&self, tips: &[Oid], limit: usize) -> Vec<(Oid, Commit)> {
        let mut seen = HashSet::new();
        let mut frontier: Vec<(i64, Oid, Commit)> = Vec::new();
        for t in tips {
            if seen.insert(*t) {
                if let Ok(c) = self.commit(t) {
                    frontier.push((c.committer.timestamp, *t, c));
                }
            }
        }
        let mut out = Vec::new();
        while out.len() < limit {
            let Some(i) = (0..frontier.len()).max_by_key(|i| frontier[*i].0) else {
                break;
            };
            let (_, oid, c) = frontier.swap_remove(i);
            for p in &c.parents {
                if seen.insert(*p) {
                    if let Ok(pc) = self.commit(p) {
                        frontier.push((pc.committer.timestamp, *p, pc));
                    }
                }
            }
            out.push((oid, c));
        }
        out
    }

    pub fn is_ancestor(&self, a: &Oid, b: &Oid) -> bool {
        arks_repo::ancestor_set(&self.repo, *b)
            .map(|s| s.contains(a))
            .unwrap_or(false)
    }

    pub fn merge_base(&self, a: &Oid, b: &Oid) -> Option<Oid> {
        arks_repo::merge_base_oid(&self.repo, a, b).ok().flatten()
    }

    /// Commits on `head` that are not on `base` (oldest first).
    pub fn range(&self, base: &Oid, head: &Oid) -> Vec<(Oid, Commit)> {
        let exclude = arks_repo::ancestor_set(&self.repo, *base).unwrap_or_default();
        let mut out: Vec<(Oid, Commit)> = self
            .log(&[*head], 10_000)
            .into_iter()
            .filter(|(o, _)| !exclude.contains(o))
            .collect();
        out.reverse();
        out
    }

    fn put(&mut self, kind: ObjectKind, payload: Vec<u8>) -> Result<Oid, String> {
        let fmt = self.repo.object_format();
        self.repo
            .store_mut()
            .put(fmt, &RawObject::new(kind, payload))
            .map_err(|e| format!("{e:?}"))
    }

    pub fn write_blob(&mut self, bytes: &[u8]) -> Result<Oid, String> {
        self.put(ObjectKind::Blob, bytes.to_vec())
    }

    /// Write nested trees for a flat path map; returns the root tree id.
    pub fn write_flat(&mut self, flat: &Flat) -> Result<Oid, String> {
        #[derive(Default)]
        struct Dir {
            files: Vec<(String, TreeMode, Oid)>,
            dirs: BTreeMap<String, Dir>,
        }
        let mut root = Dir::default();
        for (path, (mode, oid)) in flat {
            let mut d = &mut root;
            let parts: Vec<&str> = path.split('/').collect();
            for p in &parts[..parts.len() - 1] {
                d = d.dirs.entry((*p).to_string()).or_default();
            }
            d.files
                .push((parts[parts.len() - 1].to_string(), *mode, *oid));
        }
        fn write(g: &mut GitRepo, d: &Dir) -> Result<Oid, String> {
            let mut entries = Vec::new();
            for (name, sub) in &d.dirs {
                entries.push(TreeEntry {
                    mode: TreeMode::Tree,
                    name: name.clone(),
                    oid: write(g, sub)?,
                });
            }
            for (name, mode, oid) in &d.files {
                entries.push(TreeEntry {
                    mode: *mode,
                    name: name.clone(),
                    oid: *oid,
                });
            }
            let t = Tree::new(entries).map_err(err)?;
            g.put(ObjectKind::Tree, t.encode().map_err(err)?)
        }
        write(self, &root)
    }

    pub fn write_commit(
        &mut self,
        tree: Oid,
        parents: Vec<Oid>,
        message: &str,
        who: &str,
        when: i64,
    ) -> Result<Oid, String> {
        let sig = Signature {
            name: who.to_string(),
            email: format!("{who}@users.noreply.arks.invalid"),
            timestamp: when,
            timezone: "+0000".into(),
        };
        let msg = if message.ends_with('\n') {
            message.to_string()
        } else {
            format!("{message}\n")
        };
        let c = Commit {
            tree,
            parents,
            author: sig.clone(),
            committer: sig,
            gpgsig: None,
            message: msg,
        };
        self.put(ObjectKind::Commit, c.encode().map_err(err)?)
    }

    /// Point `refs/heads/<branch>` (or a full `refs/…` name) at `oid`.
    pub fn set_branch(&mut self, branch: &str, oid: &Oid) -> Result<(), String> {
        let name = if branch.starts_with("refs/") {
            branch.to_string()
        } else {
            format!("refs/heads/{branch}")
        };
        self.repo.refs_mut().update(&name, oid, None).map_err(err)
    }

    pub fn delete_branch(&mut self, branch: &str) -> Result<(), String> {
        let name = format!("refs/heads/{branch}");
        self.repo.refs_mut().delete(&name, None).map_err(err)
    }

    pub fn delete_tag(&mut self, tag: &str) -> Result<(), String> {
        let name = format!("refs/tags/{tag}");
        self.repo.refs_mut().delete(&name, None).map_err(err)
    }

    /// Add files on top of `branch` (creating it when absent) as one commit.
    pub fn commit_files(
        &mut self,
        branch: &str,
        files: &[(String, Vec<u8>)],
        message: &str,
        who: &str,
        when: i64,
    ) -> Result<Oid, String> {
        let parent = self.resolve(branch);
        let mut flat = match parent {
            Some(p) => self.flatten(&p)?,
            None => Flat::new(),
        };
        for (path, bytes) in files {
            let oid = self.write_blob(bytes)?;
            flat.insert(path.clone(), (TreeMode::File, oid));
        }
        let tree = self.write_flat(&flat)?;
        let c = self.write_commit(tree, parent.into_iter().collect(), message, who, when)?;
        self.set_branch(branch, &c)?;
        Ok(c)
    }
}

/// Path-level 3-way merge (the `git read-tree -m` trivial cases): a path changed
/// on one side only takes that side; identical changes agree; a path changed
/// differently on both sides is a conflict.
pub(crate) fn merge_flat(base: &Flat, ours: &Flat, theirs: &Flat) -> Result<Flat, Vec<String>> {
    let paths: BTreeSet<&String> = base
        .keys()
        .chain(ours.keys())
        .chain(theirs.keys())
        .collect();
    let mut out = Flat::new();
    let mut conflicts = Vec::new();
    for p in paths {
        let (b, o, t) = (base.get(p), ours.get(p), theirs.get(p));
        let pick = if o == t {
            o
        } else if o == b {
            t
        } else if t == b {
            o
        } else {
            conflicts.push(p.clone());
            continue;
        };
        if let Some(v) = pick {
            out.insert(p.clone(), *v);
        }
    }
    if conflicts.is_empty() {
        Ok(out)
    } else {
        Err(conflicts)
    }
}

/// One changed path between two flat trees.
pub(crate) struct Change {
    pub path: String,
    pub old: Option<(TreeMode, Oid)>,
    pub new: Option<(TreeMode, Oid)>,
}

pub(crate) fn changes(old: &Flat, new: &Flat) -> Vec<Change> {
    let paths: BTreeSet<&String> = old.keys().chain(new.keys()).collect();
    paths
        .into_iter()
        .filter(|p| old.get(*p) != new.get(*p))
        .map(|p| Change {
            path: p.clone(),
            old: old.get(p).copied(),
            new: new.get(p).copied(),
        })
        .collect()
}

fn mode_str(m: TreeMode) -> &'static str {
    m.as_str()
}

/// `diff --git` text for the changes (unified, 3 lines of context).
pub(crate) fn unified(g: &GitRepo, changes: &[Change]) -> Result<String, String> {
    let mut out = String::new();
    for c in changes {
        let p = &c.path;
        out.push_str(&format!("diff --git a/{p} b/{p}\n"));
        let short = |o: &Oid| o.to_hex()[..7].to_string();
        let zero = "0000000";
        match (c.old, c.new) {
            (None, Some((m, n))) => {
                out.push_str(&format!(
                    "new file mode {}\nindex {zero}..{}\n",
                    mode_str(m),
                    short(&n)
                ));
            }
            (Some((m, o)), None) => {
                out.push_str(&format!(
                    "deleted file mode {}\nindex {}..{zero}\n",
                    mode_str(m),
                    short(&o)
                ));
            }
            (Some((om, o)), Some((nm, n))) => {
                if om != nm {
                    out.push_str(&format!(
                        "old mode {}\nnew mode {}\n",
                        mode_str(om),
                        mode_str(nm)
                    ));
                }
                if o == n {
                    continue;
                }
                out.push_str(&format!("index {}..{}", short(&o), short(&n)));
                if om == nm {
                    out.push_str(&format!(" {}", mode_str(om)));
                }
                out.push('\n');
            }
            (None, None) => continue,
        }
        let read = |x: Option<(TreeMode, Oid)>| -> Result<Vec<u8>, String> {
            match x {
                Some((TreeMode::Gitlink, _)) | None => Ok(Vec::new()),
                Some((_, o)) => g.blob(&o),
            }
        };
        let (a, b) = (read(c.old)?, read(c.new)?);
        if a.contains(&0) || b.contains(&0) {
            out.push_str(&format!("Binary files a/{p} and b/{p} differ\n"));
            continue;
        }
        let from = if c.old.is_some() {
            format!("a/{p}")
        } else {
            "/dev/null".into()
        };
        let to = if c.new.is_some() {
            format!("b/{p}")
        } else {
            "/dev/null".into()
        };
        out.push_str(&format!("--- {from}\n+++ {to}\n"));
        out.push_str(&hunks(
            &String::from_utf8_lossy(&a),
            &String::from_utf8_lossy(&b),
        ));
    }
    Ok(out)
}

/// Unified hunks between two texts (LCS line diff, 3 lines of context).
pub(crate) fn hunks(a: &str, b: &str) -> String {
    let x: Vec<&str> = a.lines().collect();
    let y: Vec<&str> = b.lines().collect();
    // ops: (tag, line) where tag is ' ', '-', '+'
    let ops: Vec<(char, &str)> = if x.len() * y.len() > 4_000_000 {
        x.iter()
            .map(|l| ('-', *l))
            .chain(y.iter().map(|l| ('+', *l)))
            .collect()
    } else {
        let (n, m) = (x.len(), y.len());
        let mut lcs = vec![vec![0u32; m + 1]; n + 1];
        for i in (0..n).rev() {
            for j in (0..m).rev() {
                lcs[i][j] = if x[i] == y[j] {
                    lcs[i + 1][j + 1] + 1
                } else {
                    lcs[i + 1][j].max(lcs[i][j + 1])
                };
            }
        }
        let (mut i, mut j) = (0, 0);
        let mut ops = Vec::new();
        while i < n || j < m {
            if i < n && j < m && x[i] == y[j] {
                ops.push((' ', x[i]));
                i += 1;
                j += 1;
            } else if i < n && (j == m || lcs[i + 1][j] >= lcs[i][j + 1]) {
                ops.push(('-', x[i]));
                i += 1;
            } else {
                ops.push(('+', y[j]));
                j += 1;
            }
        }
        ops
    };
    const CTX: usize = 3;
    let changed: Vec<usize> = (0..ops.len()).filter(|k| ops[*k].0 != ' ').collect();
    let mut out = String::new();
    let mut k = 0;
    while k < changed.len() {
        let start = changed[k].saturating_sub(CTX);
        let mut end = (changed[k] + CTX + 1).min(ops.len());
        while k + 1 < changed.len() && changed[k + 1] <= end + CTX {
            k += 1;
            end = (changed[k] + CTX + 1).min(ops.len());
        }
        k += 1;
        let (mut a0, mut b0) = (1, 1);
        for op in &ops[..start] {
            match op.0 {
                ' ' => {
                    a0 += 1;
                    b0 += 1;
                }
                '-' => a0 += 1,
                _ => b0 += 1,
            }
        }
        let a_len = ops[start..end].iter().filter(|o| o.0 != '+').count();
        let b_len = ops[start..end].iter().filter(|o| o.0 != '-').count();
        let range = |s: usize, l: usize| {
            let s = if l == 0 { s - 1 } else { s };
            if l == 1 {
                format!("{s}")
            } else {
                format!("{s},{l}")
            }
        };
        out.push_str(&format!(
            "@@ -{} +{} @@\n",
            range(a0, a_len),
            range(b0, b_len)
        ));
        for (tag, line) in &ops[start..end] {
            out.push(*tag);
            out.push_str(line);
            out.push('\n');
        }
    }
    out
}

/// Breadth-first reachable object closure check helper for tests.
#[cfg(test)]
pub(crate) fn reachable_count(g: &GitRepo, tip: &Oid) -> usize {
    let mut q = std::collections::VecDeque::from([*tip]);
    let mut seen = HashSet::new();
    while let Some(o) = q.pop_front() {
        if !seen.insert(o) {
            continue;
        }
        if let Ok(c) = g.commit(&o) {
            q.extend(c.parents);
        }
    }
    seen.len()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hunks_have_git_shape() {
        let h = hunks("a\nb\nc\n", "a\nB\nc\nd\n");
        assert_eq!(h, "@@ -1,3 +1,4 @@\n a\n-b\n+B\n c\n+d\n");
        assert_eq!(hunks("", "x\n"), "@@ -0,0 +1 @@\n+x\n");
    }

    #[test]
    fn bare_repo_roundtrip_and_merge() {
        let root = std::env::temp_dir().join(format!("arks-forge-git-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        let dir = init_bare(&root, "o", "r", "main").unwrap();
        assert_eq!(repo_dir(&root, "o", "r"), Some(dir.clone()));
        let mut g = GitRepo::open(&dir).unwrap();
        let c1 = g
            .commit_files(
                "main",
                &[("README.md".into(), b"hi\n".to_vec())],
                "init",
                "u",
                1,
            )
            .unwrap();
        let c2 = g.commit_files("topic", &[], "noop", "u", 2).unwrap();
        assert_eq!(g.resolve("main"), Some(c1));
        assert!(g.is_ancestor(&c1, &c2) || g.resolve("topic") == Some(c2));
        g.set_branch("topic", &c1).unwrap();
        let c3 = g
            .commit_files(
                "topic",
                &[("src/a.rs".into(), b"fn a() {}\n".to_vec())],
                "a",
                "u",
                3,
            )
            .unwrap();
        assert!(g.is_ancestor(&c1, &c3));
        assert_eq!(reachable_count(&g, &c3), 2);
        let f = g.flatten(&c3).unwrap();
        assert!(f.contains_key("src/a.rs"));
        let (mode, _) = g.lookup(&c3, "src").unwrap().unwrap();
        assert_eq!(mode, TreeMode::Tree);
        let base = g.flatten(&c1).unwrap();
        let merged = merge_flat(&base, &base, &f).unwrap();
        assert_eq!(merged, f);
        let diff = unified(&g, &changes(&base, &f)).unwrap();
        assert!(
            diff.contains("+++ b/src/a.rs\n@@ -0,0 +1 @@\n+fn a() {}\n"),
            "{diff}"
        );
        let _ = fs::remove_dir_all(&root);
    }
}
