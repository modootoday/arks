//! Diff core for the history / inspection commands: file pairs between
//! trees, the index and the work tree; the transformations git applies to
//! the diff queue (relative paths, `-R`, rename / copy detection, pickaxe,
//! `--diff-filter`, ordering); and every output format (see `emit`).
//!
//! Primary: <https://git-scm.com/docs/diff-options>,
//! <https://git-scm.com/docs/diff-format>, <https://git-scm.com/docs/gitdiffcore>.

pub mod combined;
pub mod emit;
pub mod emit_rd;
pub mod options;
pub mod rename;
pub mod sources;
pub mod xdiff;

use crate::cmd::bre::Bre;
use crate::error::{CliError, CliResult};
use arks_object::{ObjectKind, RawObject, Tree, TreeMode};
use arks_oid::Oid;
use arks_repo::Repository;
use options::{DiffOptions, Pickaxe};
use std::cell::RefCell;
use std::collections::{BTreeMap, HashMap};
use std::path::PathBuf;

pub const MODE_TREE: u32 = 0o040000;
pub const MODE_LINK: u32 = 0o120000;
pub const MODE_GITLINK: u32 = 0o160000;

pub fn tree_mode(m: TreeMode) -> u32 {
    match m {
        TreeMode::Tree => MODE_TREE,
        TreeMode::Executable => 0o100755,
        TreeMode::Symlink => MODE_LINK,
        TreeMode::Gitlink => MODE_GITLINK,
        TreeMode::File => 0o100644,
    }
}

/// One side of a file pair.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Side {
    pub path: String,
    pub mode: u32,
    /// `None`: work-tree content that was not hashed (shown as zeros).
    pub oid: Option<Oid>,
    /// Read the content from this work-tree file.
    pub work: Option<PathBuf>,
}

impl Side {
    pub fn blob(path: &str, mode: u32, oid: Oid) -> Self {
        Self {
            path: path.to_string(),
            mode,
            oid: Some(oid),
            work: None,
        }
    }

    pub fn is_tree(&self) -> bool {
        self.mode == MODE_TREE
    }
}

/// One entry of the diff queue.
#[derive(Clone, Debug)]
pub struct Pair {
    pub old: Option<Side>,
    pub new: Option<Side>,
    /// `R` / `C` after rename detection, else derived.
    pub status: Option<char>,
    /// Similarity (renames / copies) on the 0..=60000 scale.
    pub score: u32,
    /// Unmerged path (diff-files / diff against the index).
    pub unmerged: bool,
}

impl Pair {
    pub fn new(old: Option<Side>, new: Option<Side>) -> Self {
        Self {
            old,
            new,
            status: None,
            score: 0,
            unmerged: false,
        }
    }

    pub fn status(&self) -> char {
        if self.unmerged {
            return 'U';
        }
        if let Some(s) = self.status {
            return s;
        }
        match (&self.old, &self.new) {
            (None, _) => 'A',
            (_, None) => 'D',
            (Some(a), Some(b)) if a.mode & 0o170000 != b.mode & 0o170000 => 'T',
            _ => 'M',
        }
    }

    /// Path shown for single-name formats (the new side's, else the old).
    pub fn path(&self) -> &str {
        self.new
            .as_ref()
            .or(self.old.as_ref())
            .map(|s| s.path.as_str())
            .unwrap_or("")
    }

    pub fn old_path(&self) -> &str {
        self.old
            .as_ref()
            .or(self.new.as_ref())
            .map(|s| s.path.as_str())
            .unwrap_or("")
    }

    pub fn is_rename_or_copy(&self) -> bool {
        matches!(self.status, Some('R') | Some('C'))
    }
}

/// Cache key of one side's bytes: its object id or its worktree path.
type BlobKey = (Option<Oid>, Option<PathBuf>);

/// Blob content and object-name cache for one diff run.
pub struct Content<'r> {
    pub repo: &'r Repository,
    data: RefCell<HashMap<BlobKey, std::rc::Rc<Vec<u8>>>>,
    abbrevs: RefCell<HashMap<(Oid, usize), String>>,
}

impl<'r> Content<'r> {
    pub fn new(repo: &'r Repository) -> Self {
        Self {
            repo,
            data: RefCell::new(HashMap::new()),
            abbrevs: RefCell::new(HashMap::new()),
        }
    }

    /// Bytes of one side (symlinks: the link target).
    pub fn bytes(&self, s: &Side) -> CliResult<std::rc::Rc<Vec<u8>>> {
        let key = (s.oid, s.work.clone());
        if let Some(v) = self.data.borrow().get(&key) {
            return Ok(v.clone());
        }
        let v = match (&s.work, &s.oid) {
            (Some(p), _) => {
                if s.mode == MODE_LINK {
                    std::fs::read_link(p)
                        .map(|t| t.to_string_lossy().into_owned().into_bytes())
                        .unwrap_or_default()
                } else if s.mode == MODE_GITLINK {
                    Vec::new()
                } else {
                    std::fs::read(p).unwrap_or_default()
                }
            }
            (None, Some(o)) => {
                if s.mode == MODE_GITLINK {
                    Vec::new()
                } else {
                    self.repo.get_object(o)?.payload
                }
            }
            (None, None) => Vec::new(),
        };
        let v = std::rc::Rc::new(v);
        self.data.borrow_mut().insert(key, v.clone());
        Ok(v)
    }

    /// Provide the bytes of an object that is not in the repository
    /// (`log --remerge-diff` merge results).
    pub fn seed(&self, oid: Oid, bytes: Vec<u8>) {
        self.data
            .borrow_mut()
            .insert((Some(oid), None), std::rc::Rc::new(bytes));
    }

    /// Object id of a side, hashing work-tree content when needed.
    pub fn oid(&self, s: &Side) -> CliResult<Oid> {
        if let Some(o) = s.oid {
            return Ok(o);
        }
        let b = self.bytes(s)?;
        Ok(RawObject::new(ObjectKind::Blob, b.to_vec()).oid(self.repo.object_format())?)
    }

    pub fn abbrev(&self, oid: &Oid, min: usize) -> String {
        if min >= oid.to_hex().len() {
            return oid.to_hex();
        }
        if let Some(s) = self.abbrevs.borrow().get(&(*oid, min)) {
            return s.clone();
        }
        // An object not in the repository (work-tree content) cannot
        // collide with a longer prefix of an existing object's name.
        let s = if self.repo.get_object(oid).is_ok() {
            crate::cmd::shared::abbrev_hex(self.repo, oid, min)
        } else {
            oid.to_hex()[..min.max(4)].to_string()
        };
        self.abbrevs.borrow_mut().insert((*oid, min), s.clone());
        s
    }

    pub fn zero_hex(&self) -> String {
        "0".repeat(self.repo.oid_kind().hex_len())
    }

    /// A side holding in-memory `bytes` under `path` (no object name; raw
    /// output shows zeros), such as `range-diff`'s patch texts.
    pub fn virtual_side(&self, path: &str, bytes: Vec<u8>) -> Side {
        let mut data = self.data.borrow_mut();
        // A key no work-tree path can take.
        let key = PathBuf::from(format!("\0virtual/{}/{path}", data.len()));
        data.insert((None, Some(key.clone())), std::rc::Rc::new(bytes));
        Side {
            path: path.to_string(),
            mode: 0o100644,
            oid: None,
            work: Some(key),
        }
    }
}

/// Entries of one tree level: name -> (mode, oid).
pub fn tree_level(
    repo: &Repository,
    tree: Option<&Oid>,
) -> CliResult<BTreeMap<String, (u32, Oid)>> {
    let mut out = BTreeMap::new();
    if let Some(t) = tree {
        let obj = repo.get_object(t)?;
        for e in Tree::decode(&obj.payload, repo.oid_kind())?.entries {
            out.insert(e.name.clone(), (tree_mode(e.mode), e.oid));
        }
    }
    Ok(out)
}

/// Every non-tree entry under `tree`: path -> (mode, oid).
pub fn flat_tree(repo: &Repository, tree: &Oid) -> CliResult<BTreeMap<String, (u32, Oid)>> {
    fn walk(
        repo: &Repository,
        tree: &Oid,
        prefix: &str,
        out: &mut BTreeMap<String, (u32, Oid)>,
    ) -> CliResult<()> {
        for (name, (mode, oid)) in tree_level(repo, Some(tree))? {
            let path = if prefix.is_empty() {
                name
            } else {
                format!("{prefix}/{name}")
            };
            if mode == MODE_TREE {
                walk(repo, &oid, &path, out)?;
            } else {
                out.insert(path, (mode, oid));
            }
        }
        Ok(())
    }
    let mut out = BTreeMap::new();
    walk(repo, tree, "", &mut out)?;
    Ok(out)
}

/// Pathspec filter as used by the diff machinery (prefix / glob match;
/// a directory spec also keeps the tree entry itself).
pub fn spec_match(path: &str, specs: &[String]) -> bool {
    crate::cmd::log_format::path_in_specs(path, specs)
}

/// Can a tree at `dir` contain a path the specs select?
fn spec_may_descend(dir: &str, specs: &[String]) -> bool {
    specs.is_empty()
        || specs.iter().any(|s| {
            s.is_empty()
                || spec_match(dir, std::slice::from_ref(s))
                || s.starts_with(&format!("{dir}/"))
                || s.contains(['*', '?', '['])
        })
}

/// Tree-to-tree pairs (git `diff_tree`): with `recursive`, changed
/// subtrees are entered (`show_trees` also lists the trees themselves).
pub fn tree_pairs(
    repo: &Repository,
    a: Option<&Oid>,
    b: Option<&Oid>,
    recursive: bool,
    show_trees: bool,
    specs: &[String],
) -> CliResult<Vec<Pair>> {
    let mut out = Vec::new();
    tree_pairs_into(repo, a, b, "", recursive, show_trees, specs, &mut out)?;
    Ok(out)
}

#[allow(clippy::too_many_arguments)]
fn tree_pairs_into(
    repo: &Repository,
    a: Option<&Oid>,
    b: Option<&Oid>,
    prefix: &str,
    recursive: bool,
    show_trees: bool,
    specs: &[String],
    out: &mut Vec<Pair>,
) -> CliResult<()> {
    let (ea, eb) = (tree_level(repo, a)?, tree_level(repo, b)?);
    let mut names: Vec<&String> = ea.keys().chain(eb.keys()).collect();
    // Tree order: a directory sorts as if its name ended in '/'.
    let sort_key = |n: &String| -> Vec<u8> {
        let t = ea.get(n).or(eb.get(n)).is_some_and(|e| e.0 == MODE_TREE);
        let mut k = n.as_bytes().to_vec();
        if t {
            k.push(b'/');
        }
        k
    };
    names.sort_by_key(|n| sort_key(n));
    names.dedup();
    for name in names {
        let (x, y) = (ea.get(name).copied(), eb.get(name).copied());
        if x == y {
            continue;
        }
        let path = if prefix.is_empty() {
            name.clone()
        } else {
            format!("{prefix}/{name}")
        };
        let side = |e: Option<(u32, Oid)>| e.map(|(m, o)| Side::blob(&path, m, o));
        let is_tree = |e: Option<(u32, Oid)>| e.is_some_and(|e| e.0 == MODE_TREE);
        if recursive && (is_tree(x) || is_tree(y)) {
            if !spec_may_descend(&path, specs) {
                continue;
            }
            if show_trees && is_tree(x) && is_tree(y) && spec_match(&path, specs) {
                out.push(Pair::new(side(x), side(y)));
            }
            let sub = |e: Option<(u32, Oid)>| e.filter(|e| e.0 == MODE_TREE).map(|e| e.1);
            if !is_tree(x) && x.is_some() && spec_match(&path, specs) {
                out.push(Pair::new(side(x), None));
            }
            if show_trees && (is_tree(x) != is_tree(y)) && spec_match(&path, specs) {
                if is_tree(x) {
                    out.push(Pair::new(side(x), None));
                } else if is_tree(y) {
                    out.push(Pair::new(None, side(y)));
                }
            }
            tree_pairs_into(
                repo,
                sub(x).as_ref(),
                sub(y).as_ref(),
                &path,
                recursive,
                show_trees,
                specs,
                out,
            )?;
            if !is_tree(y) && y.is_some() && spec_match(&path, specs) {
                out.push(Pair::new(None, side(y)));
            }
            continue;
        }
        if !spec_match(&path, specs) && !(is_tree(x) || is_tree(y)) {
            continue;
        }
        if (is_tree(x) || is_tree(y)) && !spec_may_descend(&path, specs) {
            continue;
        }
        // A blob replaced by a tree (or back) is a delete plus an add.
        if x.is_some() && y.is_some() && is_tree(x) != is_tree(y) {
            out.push(Pair::new(side(x), None));
            out.push(Pair::new(None, side(y)));
        } else {
            out.push(Pair::new(side(x), side(y)));
        }
    }
    Ok(())
}

/// Pairs between two flat path maps (index, tree or work tree sides).
pub fn flat_pairs(a: &BTreeMap<String, Side>, b: &BTreeMap<String, Side>) -> Vec<Pair> {
    let mut keys: Vec<&String> = a.keys().chain(b.keys()).collect();
    keys.sort();
    keys.dedup();
    let mut out = Vec::new();
    for k in keys {
        let (x, y) = (a.get(k), b.get(k));
        let same = match (x, y) {
            (Some(x), Some(y)) => x.mode == y.mode && x.oid.is_some() && x.oid == y.oid,
            _ => false,
        };
        if !same {
            out.push(Pair::new(x.cloned(), y.cloned()));
        }
    }
    out
}

/// Rewrite pair paths for `--relative[=<prefix>]`: pairs outside the
/// prefix are dropped, the prefix is stripped from the rest.
pub fn apply_relative(pairs: &mut Vec<Pair>, prefix: &str) {
    if prefix.is_empty() {
        return;
    }
    let p = if prefix.ends_with('/') {
        prefix.to_string()
    } else {
        format!("{prefix}/")
    };
    pairs.retain(|x| {
        x.old.as_ref().is_none_or(|s| s.path.starts_with(&p))
            && x.new.as_ref().is_none_or(|s| s.path.starts_with(&p))
    });
    for x in pairs.iter_mut() {
        for s in [&mut x.old, &mut x.new].into_iter().flatten() {
            s.path = s.path[p.len()..].to_string();
        }
    }
}

/// `-R`: swap both sides of every pair.
pub fn apply_reverse(pairs: &mut [Pair]) {
    for x in pairs.iter_mut() {
        std::mem::swap(&mut x.old, &mut x.new);
    }
}

fn count_matches(hay: &[u8], needle: &[u8], re: Option<&Bre>) -> usize {
    match re {
        Some(r) => {
            let mut n = 0;
            let mut from = 0;
            while from <= hay.len() {
                match r.find_from(hay, from) {
                    Some((s, e)) => {
                        n += 1;
                        from = if e > s { e } else { e + 1 };
                    }
                    None => break,
                }
            }
            n
        }
        None => {
            if needle.is_empty() {
                return 0;
            }
            let mut n = 0;
            let mut i = 0;
            while i + needle.len() <= hay.len() {
                if &hay[i..i + needle.len()] == needle {
                    n += 1;
                    i += needle.len();
                } else {
                    i += 1;
                }
            }
            n
        }
    }
}

/// Does this pair satisfy the pickaxe (`-S` / `-G`) or `--find-object`?
pub fn pickaxe_hit(c: &Content<'_>, o: &DiffOptions, p: &Pair) -> CliResult<bool> {
    if let Some(obj) = &o.find_object {
        let want = crate::cmd::shared::resolve_rev(c.repo, obj)
            .map_err(|_| CliError::fatal(format!("unable to resolve '{obj}'")))?;
        let has = |s: &Option<Side>| s.as_ref().and_then(|s| s.oid) == Some(want);
        if has(&p.old) == has(&p.new) {
            return Ok(false);
        }
        if o.pickaxe.is_none() {
            return Ok(true);
        }
    }
    let Some(px) = &o.pickaxe else {
        return Ok(true);
    };
    let blob = |s: &Option<Side>| -> CliResult<Vec<u8>> {
        Ok(match s {
            Some(s) if !s.is_tree() => c.bytes(s)?.to_vec(),
            _ => Vec::new(),
        })
    };
    let (a, b) = (blob(&p.old)?, blob(&p.new)?);
    if !o.text && (xdiff::is_binary(&a) || xdiff::is_binary(&b)) && matches!(px, Pickaxe::Grep(_)) {
        return Ok(false);
    }
    match px {
        Pickaxe::Count(s) => {
            let re = o.pickaxe_regex.then(|| Bre::with_mode(s, false, true));
            Ok(count_matches(&a, s.as_bytes(), re.as_ref())
                != count_matches(&b, s.as_bytes(), re.as_ref()))
        }
        Pickaxe::Grep(r) => {
            let re = Bre::with_mode(r, false, true);
            let al = xdiff::split_lines(&a);
            let bl = xdiff::split_lines(&b);
            let ops = xdiff::diff_lines(&al, &bl, &Default::default());
            let (mut ai, mut bi) = (0, 0);
            for op in ops {
                let line: Option<&[u8]> = match op {
                    xdiff::Op::Eq => {
                        ai += 1;
                        bi += 1;
                        None
                    }
                    xdiff::Op::Del => {
                        ai += 1;
                        Some(al[ai - 1])
                    }
                    xdiff::Op::Ins => {
                        bi += 1;
                        Some(bl[bi - 1])
                    }
                };
                if let Some(l) = line {
                    if re.is_match(l.strip_suffix(b"\n").unwrap_or(l)) {
                        return Ok(true);
                    }
                }
            }
            Ok(false)
        }
    }
}

/// git `--diff-filter`: upper case selects, lower case excludes; `*`
/// selects all when any pair matched.
pub fn filter_passes(st: char, filter: &str) -> bool {
    let includes: Vec<char> = filter.chars().filter(|c| c.is_ascii_uppercase()).collect();
    let excluded = filter.contains(st.to_ascii_lowercase());
    (includes.is_empty() || includes.contains(&st)) && !excluded
}

/// Order the queue by an orderfile (`-O`): patterns in file order, each
/// path placed at its first matching pattern; unmatched paths last.
fn apply_orderfile(pairs: &mut [Pair], file: &str) -> CliResult<()> {
    let text = std::fs::read_to_string(file)
        .map_err(|e| CliError::fatal(format!("failed to read orderfile '{file}': {e}")))?;
    let pats: Vec<&str> = text
        .lines()
        .map(str::trim_end)
        .filter(|l| !l.is_empty() && !l.starts_with('#'))
        .collect();
    let rank = |path: &str| -> usize {
        for (i, p) in pats.iter().enumerate() {
            let pb = p.as_bytes();
            let mut cur = path;
            loop {
                if crate::cmd::branch_cmd::fnmatch(pb, cur.as_bytes()) {
                    return i;
                }
                match cur.rfind('/') {
                    Some(k) => cur = &cur[..k],
                    None => break,
                }
            }
        }
        pats.len()
    };
    pairs.sort_by_key(|p| rank(p.path()));
    Ok(())
}

/// Run the diffcore transformations on a queue (git `diffcore_std`).
pub fn transform(c: &Content<'_>, o: &DiffOptions, pairs: &mut Vec<Pair>) -> CliResult<()> {
    transform_with_sources(c, o, pairs, &[])
}

/// [`transform`] with the unmodified files for `--find-copies-harder`.
pub fn transform_with_sources(
    c: &Content<'_>,
    o: &DiffOptions,
    pairs: &mut Vec<Pair>,
    unchanged: &[Side],
) -> CliResult<()> {
    if let Some(r) = &o.relative {
        apply_relative(pairs, r);
    }
    if o.reverse {
        apply_reverse(pairs);
    }
    if o.detect != options::Detect::Off || o.break_rewrites.is_some() {
        rename::detect(c, o, pairs, unchanged)?;
    }
    if o.pickaxe.is_some() || o.find_object.is_some() {
        let mut keep = Vec::with_capacity(pairs.len());
        for p in pairs.iter() {
            keep.push(pickaxe_hit(c, o, p)?);
        }
        if o.pickaxe_all {
            if !keep.iter().any(|k| *k) {
                pairs.clear();
            }
        } else {
            let mut it = keep.into_iter();
            pairs.retain(|_| it.next().unwrap_or(false));
        }
    }
    if let Some(f) = &o.diff_filter {
        if f.contains('*') {
            let any = pairs
                .iter()
                .any(|p| filter_passes(p.status(), &f.replace('*', "")));
            if !any {
                pairs.clear();
            }
        } else {
            pairs.retain(|p| filter_passes(p.status(), f));
        }
    }
    if let Some(f) = &o.orderfile {
        apply_orderfile(pairs, f)?;
    }
    if let Some(r) = &o.rotate_to {
        if let Some(i) = pairs.iter().position(|p| p.path() >= r.as_str()) {
            pairs.rotate_left(i);
        } else if !pairs.is_empty() {
            return Err(CliError::fatal(format!("No such path '{r}' in the diff")));
        }
    }
    if let Some(r) = &o.skip_to {
        if let Some(i) = pairs.iter().position(|p| p.path() >= r.as_str()) {
            pairs.drain(..i);
        } else if !pairs.is_empty() {
            return Err(CliError::fatal(format!("No such path '{r}' in the diff")));
        }
    }
    Ok(())
}
