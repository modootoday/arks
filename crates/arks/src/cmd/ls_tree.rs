//! `git ls-tree`: entries of a tree (`-r`, `-t`, `-d`), path patterns (a
//! trailing `/` lists a directory's contents), output as `-l` / `--long`,
//! `--name-only` / `--name-status`, `--object-only`, `--format` atoms,
//! `--abbrev`, `-z`, `--full-name` and `--full-tree` (paths are relative
//! to the current directory otherwise).
//!
//! Primary: <https://git-scm.com/docs/git-ls-tree>.

use crate::cmd::shared::resolve_rev;
use crate::error::{CliError, CliResult};
use crate::util::{open_repo, work_tree};
use arks_object::{ObjectKind, Tree, TreeMode};
use arks_oid::Oid;
use arks_repo::Repository;
use std::io::Write;

#[derive(Default)]
struct Opts {
    recursive: bool,
    show_trees: bool,
    only_trees: bool,
    long: bool,
    name_only: bool,
    object_only: bool,
    nul: bool,
    full_name: bool,
    full_tree: bool,
    abbrev: Option<usize>,
    format: Option<String>,
    specs: Vec<String>,
    /// Current directory below the top (`""` at the top or `--full-tree`).
    prefix: String,
}

fn mode_str(m: TreeMode) -> &'static str {
    match m {
        TreeMode::Tree => "040000",
        TreeMode::File => "100644",
        TreeMode::Executable => "100755",
        TreeMode::Symlink => "120000",
        TreeMode::Gitlink => "160000",
    }
}

fn kind_str(m: TreeMode) -> &'static str {
    match m {
        TreeMode::Tree => "tree",
        TreeMode::Gitlink => "commit",
        _ => "blob",
    }
}

struct Printer<'a> {
    repo: &'a Repository,
    o: &'a Opts,
    out: std::io::BufWriter<std::io::StdoutLock<'a>>,
}

impl Printer<'_> {
    fn hex(&self, oid: &Oid) -> String {
        match self.o.abbrev {
            Some(n) if n < 40 => crate::cmd::shared::abbrev_hex(self.repo, oid, n),
            _ => oid.to_hex(),
        }
    }

    fn shown_path(&self, path: &str) -> String {
        let p = if self.o.full_name || self.o.prefix.is_empty() {
            path.to_string()
        } else {
            let pre: Vec<&str> = self.o.prefix.split('/').collect();
            let comps: Vec<&str> = path.split('/').collect();
            let mut common = 0;
            while common < pre.len() && common < comps.len() && pre[common] == comps[common] {
                common += 1;
            }
            let mut s = "../".repeat(pre.len() - common);
            s.push_str(&comps[common..].join("/"));
            s
        };
        if self.o.nul {
            p
        } else {
            crate::cmd::diffcore::emit::quote_path(&p)
        }
    }

    fn size(&self, m: TreeMode, oid: &Oid, padded: bool) -> CliResult<String> {
        let s = if kind_str(m) == "blob" {
            self.repo.get_object(oid)?.payload.len().to_string()
        } else {
            "-".to_string()
        };
        Ok(if padded { format!("{s:>7}") } else { s })
    }

    fn entry(&mut self, m: TreeMode, oid: &Oid, path: &str) -> CliResult<()> {
        let term: &[u8] = if self.o.nul { b"\0" } else { b"\n" };
        let line = if let Some(f) = &self.o.format {
            let mut s = String::new();
            let mut rest = f.as_str();
            while let Some(i) = rest.find('%') {
                s.push_str(&rest[..i]);
                let tail = &rest[i..];
                if let Some(body) = tail.strip_prefix("%(") {
                    if let Some(close) = body.find(')') {
                        let v = match &body[..close] {
                            "objectmode" => mode_str(m).to_string(),
                            "objecttype" => kind_str(m).to_string(),
                            "objectname" => self.hex(oid),
                            "objectsize" => self.size(m, oid, false)?,
                            "objectsize:padded" => self.size(m, oid, true)?,
                            "path" => self.shown_path(path),
                            a => {
                                return Err(CliError::fatal(format!(
                                    "bad ls-tree format: element '{a}' does not start with '('"
                                )));
                            }
                        };
                        s.push_str(&v);
                        rest = &body[close + 1..];
                        continue;
                    }
                }
                if let Some(t) = tail.strip_prefix("%%") {
                    s.push('%');
                    rest = t;
                } else if let Some(t) = tail.strip_prefix("%n") {
                    s.push('\n');
                    rest = t;
                } else if tail.len() >= 4
                    && tail.as_bytes()[1] == b'x'
                    && tail[2..4].bytes().all(|b| b.is_ascii_hexdigit())
                {
                    s.push(u8::from_str_radix(&tail[2..4], 16).unwrap_or(b'%') as char);
                    rest = &tail[4..];
                } else {
                    s.push('%');
                    rest = &tail[1..];
                }
            }
            s.push_str(rest);
            s
        } else if self.o.object_only {
            self.hex(oid)
        } else if self.o.name_only {
            self.shown_path(path)
        } else if self.o.long {
            format!(
                "{} {} {} {}\t{}",
                mode_str(m),
                kind_str(m),
                self.hex(oid),
                self.size(m, oid, true)?,
                self.shown_path(path)
            )
        } else {
            format!(
                "{} {} {}\t{}",
                mode_str(m),
                kind_str(m),
                self.hex(oid),
                self.shown_path(path)
            )
        };
        self.out.write_all(line.as_bytes())?;
        self.out.write_all(term)?;
        Ok(())
    }

    fn tree(&mut self, tree_oid: &Oid, prefix: &str) -> CliResult<()> {
        let obj = self.repo.get_object(tree_oid)?;
        if obj.kind != ObjectKind::Tree {
            return Err(CliError::Msg(format!("not a tree: {}", tree_oid.to_hex())));
        }
        let tree = Tree::decode(&obj.payload, self.repo.oid_kind())?;
        for e in tree.entries {
            let path = if prefix.is_empty() {
                e.name.clone()
            } else {
                format!("{prefix}/{}", e.name)
            };
            let is_tree = e.mode == TreeMode::Tree;
            let specs = &self.o.specs;
            // Spec relations: equal / inside a spec dir / ancestor of a spec.
            let matched = specs.is_empty()
                || specs.iter().any(|s| {
                    let t = s.trim_end_matches('/');
                    t.is_empty() || path == t || path.starts_with(&format!("{t}/"))
                });
            let ancestor = specs.iter().any(|s| {
                let t = s.trim_end_matches('/');
                !t.is_empty() && t.starts_with(&format!("{path}/"))
            });
            let list_contents = specs.iter().any(|s| *s == format!("{path}/"));
            if !matched && !ancestor {
                continue;
            }
            let descend = is_tree && (ancestor || list_contents || (matched && self.o.recursive));
            let show = if is_tree {
                (matched && !descend)
                    || (descend && self.o.show_trees)
                    || (self.o.only_trees && matched)
            } else {
                matched && !self.o.only_trees
            };
            if show {
                self.entry(e.mode, &e.oid, &path)?;
            }
            if descend {
                self.tree(&e.oid, &path)?;
            }
        }
        Ok(())
    }
}

pub fn cmd_ls_tree(args: &[String]) -> CliResult<()> {
    // Cite: https://git-scm.com/docs/git-ls-tree
    let mut args = crate::util::split_short_bundles(args, "rtdlz", "");
    // `--format <format>`: join the separate value.
    if let Some(i) = args.iter().position(|a| a == "--format") {
        let v = args
            .get(i + 1)
            .cloned()
            .ok_or_else(|| CliError::Exit(129, "error: option `format' requires a value".into()))?;
        args.splice(i..i + 2, [format!("--format={v}")]);
    }
    let mut o = Opts::default();
    let mut positional: Vec<String> = Vec::new();
    let mut dashdash = false;
    for a in &args {
        if dashdash {
            positional.push(a.clone());
            continue;
        }
        match a.as_str() {
            "--" => dashdash = true,
            "-r" => o.recursive = true,
            "-t" => o.show_trees = true,
            "-d" => o.only_trees = true,
            "-l" | "--long" => o.long = true,
            "-z" => o.nul = true,
            "--name-only" | "--name-status" => o.name_only = true,
            "--object-only" => o.object_only = true,
            "--full-name" => o.full_name = true,
            "--no-full-name" => o.full_name = false,
            "--full-tree" => o.full_tree = true,
            "--no-full-tree" => o.full_tree = false,
            "--abbrev" => o.abbrev = Some(7),
            "--no-abbrev" => o.abbrev = None,
            x => {
                if let Some(v) = x.strip_prefix("--abbrev=") {
                    let n: usize = v.parse().map_err(|_| {
                        CliError::Exit(
                            129,
                            "error: option `abbrev' expects a numerical value".into(),
                        )
                    })?;
                    o.abbrev = Some(if n == 0 { 40 } else { n.clamp(4, 40) });
                } else if let Some(v) = x.strip_prefix("--format=") {
                    o.format = Some(v.to_string());
                } else if x.starts_with('-') && x.len() > 1 {
                    return Err(CliError::Exit(
                        129,
                        format!("error: unknown option `{}'", x.trim_start_matches('-')),
                    ));
                } else {
                    positional.push(a.clone());
                }
            }
        }
    }
    let modes = [o.long, o.name_only, o.object_only, o.format.is_some()];
    if modes.iter().filter(|m| **m).count() > 1 {
        return Err(CliError::Exit(
            129,
            "error: options '--name-only', '--name-status', '--object-only', '--long' and '--format' cannot be used together".into(),
        ));
    }
    let Some((rev, specs)) = positional.split_first() else {
        return Err(CliError::Exit(
            129,
            "usage: git ls-tree [<options>] <tree-ish> [<path>...]".into(),
        ));
    };
    let repo = open_repo()?;
    if !o.full_tree {
        if let Ok(work) = work_tree(&repo) {
            let canon = |p: &std::path::Path| p.canonicalize().unwrap_or_else(|_| p.to_path_buf());
            o.prefix = canon(&crate::util::cwd()?)
                .strip_prefix(canon(&work))
                .map(|r| r.to_string_lossy().replace('\\', "/"))
                .unwrap_or_default();
        }
    }
    // Patterns are relative to the current directory unless --full-tree.
    let normalize = |p: String| -> String {
        let trailing = p.ends_with('/');
        let mut parts: Vec<&str> = Vec::new();
        for seg in p.split('/') {
            match seg {
                "" | "." => {}
                ".." => {
                    parts.pop();
                }
                s => parts.push(s),
            }
        }
        let mut out = parts.join("/");
        if trailing && !out.is_empty() {
            out.push('/');
        }
        out
    };
    o.specs = specs
        .iter()
        .map(|s| {
            normalize(if o.prefix.is_empty() {
                s.clone()
            } else {
                format!("{}/{s}", o.prefix)
            })
        })
        .collect();
    if o.specs.is_empty() && !o.prefix.is_empty() {
        o.specs.push(format!("{}/", o.prefix));
    }
    let oid = resolve_rev(&repo, rev)
        .map_err(|_| CliError::fatal("Not a valid object name".to_string()))?;
    let tree_oid =
        crate::cmd::shared::peel_to_kind(&repo, &oid, crate::cmd::shared::ObjKind::Tree, rev)
            .map_err(|_| CliError::fatal("not a tree object"))?;
    let stdout = std::io::stdout();
    let mut p = Printer {
        repo: &repo,
        o: &o,
        out: std::io::BufWriter::new(stdout.lock()),
    };
    p.tree(&tree_oid, "")?;
    p.out.flush()?;
    Ok(())
}
