//! `remote` and its subcommands (`add`, `rename`, `remove`/`rm`,
//! `set-head`, `set-branches`, `get-url`, `set-url`, `show`, `prune`,
//! `update`), editing `[remote "<name>"]` sections of the repository
//! config the way stock git lays them out.
//!
//! Primary: <https://git-scm.com/docs/git-remote>,
//! <https://git-scm.com/docs/git-config#_configuration_file> (syntax).

use crate::cmd::fetch::{configured_source, remote_names};
use crate::cmd::opt_parse::{Mode, Takes::*, o, parse};
use crate::cmd::remote_local::{delete_ref_any, parse_refspec};
use crate::error::{CliError, CliResult};
use crate::util::open_repo;
use arks_oid::Oid;
use std::fs;
use std::path::Path;

/// Line-level editor for one config file (local scope).
struct ConfigFile {
    lines: Vec<String>,
}

fn header_of(line: &str) -> Option<(String, Option<String>)> {
    let t = line.trim();
    if !t.starts_with('[') {
        return None;
    }
    let inner = &t[1..t.find(']')?];
    match inner.split_once(char::is_whitespace) {
        Some((sec, sub)) => Some((
            sec.to_ascii_lowercase(),
            Some(sub.trim().trim_matches('"').to_string()),
        )),
        None => match inner.split_once('.') {
            Some((sec, sub)) => Some((sec.to_ascii_lowercase(), Some(sub.to_string()))),
            None => Some((inner.to_ascii_lowercase(), None)),
        },
    }
}

fn key_of(line: &str) -> Option<(String, String)> {
    let t = line.trim();
    if t.is_empty() || t.starts_with('#') || t.starts_with(';') || t.starts_with('[') {
        return None;
    }
    match t.split_once('=') {
        Some((k, v)) => Some((k.trim().to_ascii_lowercase(), unquote(v.trim()))),
        None => Some((t.to_ascii_lowercase(), "true".into())),
    }
}

fn unquote(v: &str) -> String {
    let mut out = String::new();
    let mut quoted = false;
    let mut chars = v.chars();
    while let Some(c) = chars.next() {
        match c {
            '"' => quoted = !quoted,
            '\\' => match chars.next() {
                Some('n') => out.push('\n'),
                Some('t') => out.push('\t'),
                Some(o) => out.push(o),
                None => {}
            },
            '#' | ';' if !quoted => break,
            c => out.push(c),
        }
    }
    out.trim_end().to_string()
}

fn quote(v: &str) -> String {
    let needs = v.starts_with(char::is_whitespace)
        || v.ends_with(char::is_whitespace)
        || v.contains(['#', ';']);
    let esc = v.replace('\\', "\\\\").replace('"', "\\\"");
    if needs { format!("\"{esc}\"") } else { esc }
}

impl ConfigFile {
    fn load(path: &Path) -> Self {
        let text = fs::read_to_string(path).unwrap_or_default();
        Self {
            lines: text.lines().map(String::from).collect(),
        }
    }

    fn save(&self, path: &Path) -> CliResult<()> {
        let mut s = self.lines.join("\n");
        if !s.is_empty() {
            s.push('\n');
        }
        fs::write(path, s)?;
        Ok(())
    }

    /// Index ranges (header line, end exclusive) of matching sections.
    fn sections(&self, sec: &str, sub: Option<&str>) -> Vec<(usize, usize)> {
        let mut out = Vec::new();
        let mut i = 0;
        while i < self.lines.len() {
            if let Some((s, b)) = header_of(&self.lines[i]) {
                let mut j = i + 1;
                while j < self.lines.len() && header_of(&self.lines[j]).is_none() {
                    j += 1;
                }
                if s == sec && b.as_deref() == sub {
                    out.push((i, j));
                }
                i = j;
            } else {
                i += 1;
            }
        }
        out
    }

    fn add(&mut self, sec: &str, sub: Option<&str>, key: &str, value: &str) {
        let line = format!("\t{key} = {}", quote(value));
        match self.sections(sec, sub).last() {
            Some(&(_, end)) => {
                // Insert after the last non-blank line of the section.
                let mut at = end;
                while at > 0 && self.lines[at - 1].trim().is_empty() {
                    at -= 1;
                }
                self.lines.insert(at, line);
            }
            None => {
                self.lines.push(match sub {
                    Some(b) => format!("[{sec} \"{b}\"]"),
                    None => format!("[{sec}]"),
                });
                self.lines.push(line);
            }
        }
    }

    /// Remove `key` entries whose value passes `keep == false`.
    fn unset_where(
        &mut self,
        sec: &str,
        sub: Option<&str>,
        key: &str,
        drop: &dyn Fn(&str) -> bool,
    ) -> usize {
        let mut removed = 0;
        for (start, end) in self.sections(sec, sub).into_iter().rev() {
            for i in (start + 1..end).rev() {
                if let Some((k, v)) = key_of(&self.lines[i]) {
                    if k == key.to_ascii_lowercase() && drop(&v) {
                        self.lines.remove(i);
                        removed += 1;
                    }
                }
            }
        }
        removed
    }

    fn get_all(&self, sec: &str, sub: Option<&str>, key: &str) -> Vec<String> {
        let mut out = Vec::new();
        for (start, end) in self.sections(sec, sub) {
            for l in &self.lines[start + 1..end] {
                if let Some((k, v)) = key_of(l) {
                    if k == key.to_ascii_lowercase() {
                        out.push(v);
                    }
                }
            }
        }
        out
    }

    fn remove_section(&mut self, sec: &str, sub: Option<&str>) {
        for (start, end) in self.sections(sec, sub).into_iter().rev() {
            self.lines.drain(start..end);
        }
    }

    fn rename_section(&mut self, sec: &str, old: &str, new: &str) {
        for (start, _) in self.sections(sec, Some(old)) {
            self.lines[start] = format!("[{sec} \"{new}\"]");
        }
    }

    /// Subsection names of `sec` whose `key` equals `value`.
    fn subs_with(&self, sec: &str, key: &str, value: &str) -> Vec<String> {
        let mut out: Vec<String> = Vec::new();
        let mut i = 0;
        while i < self.lines.len() {
            if let Some((s, Some(b))) = header_of(&self.lines[i]) {
                let mut j = i + 1;
                while j < self.lines.len() && header_of(&self.lines[j]).is_none() {
                    if s == sec
                        && key_of(&self.lines[j]).is_some_and(|(k, v)| k == key && v == value)
                        && !out.contains(&b)
                    {
                        out.push(b.clone());
                    }
                    j += 1;
                }
                i = j;
            } else {
                i += 1;
            }
        }
        out
    }
}

const USAGE: &str = "usage: git remote [-v | --verbose]\n   or: git remote add [-t <branch>] [-m <master>] [-f] [--tags | --no-tags] [--mirror=<fetch|push>] <name> <url>\n   or: git remote rename [--[no-]progress] <old> <new>\n   or: git remote remove <name>\n   or: git remote set-head <name> (-a | --auto | -d | --delete | <branch>)\n   or: git remote [-v | --verbose] show [-n] <name>\n   or: git remote prune [-n | --dry-run] <name>\n   or: git remote [-v | --verbose] update [-p | --prune] [(<group> | <remote>)...]\n   or: git remote set-branches [--add] <name> <branch>...\n   or: git remote get-url [--push] [--all] <name>\n   or: git remote set-url [--push] <name> <newurl> [<oldurl>]\n   or: git remote set-url --add <name> <newurl>\n   or: git remote set-url --delete <name> <url>";

fn no_such(name: &str) -> CliError {
    CliError::Exit(2, format!("error: No such remote: '{name}'"))
}

fn valid_remote_name(name: &str) -> bool {
    !name.is_empty()
        && !name.starts_with('-')
        && !name.contains(['~', '^', ':', '?', '*', '[', '\\', ' '])
        && !name.contains("..")
        && !name.ends_with('/')
        && !name.ends_with(".lock")
}

pub fn cmd_remote(args: &[String]) -> CliResult<()> {
    let p = parse(
        "remote",
        args,
        &[o("verbose", 'v', No, true)],
        Mode {
            stop_at_non_option: true,
            ..Mode::default()
        },
    )?;
    let operands = p.operands();
    let verbose = p.count("verbose") > 0;
    let Some(sub) = operands.first() else {
        return list_remotes(verbose);
    };
    let rest = &operands[1..];
    match sub.as_str() {
        "add" => remote_add(rest),
        "rename" => remote_rename(rest),
        "remove" | "rm" => remote_remove(rest),
        "set-head" => remote_set_head(rest),
        "set-branches" => remote_set_branches(rest),
        "get-url" => remote_get_url(rest),
        "set-url" => remote_set_url(rest),
        "show" => remote_show(rest, verbose),
        "prune" => remote_prune(rest),
        "update" => remote_update(rest, verbose),
        other => Err(CliError::Exit(
            129,
            format!("error: unknown subcommand: `{other}'\n{USAGE}"),
        )),
    }
}

fn list_remotes(verbose: bool) -> CliResult<()> {
    let repo = open_repo()?;
    let cfg = arks_config::ConfigSet::load(Some(repo.common_dir()));
    let mut out = String::new();
    for name in remote_names(&cfg) {
        if verbose {
            let urls = cfg.get_all(&format!("remote.{name}.url"));
            let push = cfg.get_all(&format!("remote.{name}.pushurl"));
            let fetch_url = urls.first().cloned().unwrap_or_default();
            out.push_str(&format!("{name}\t{fetch_url} (fetch)\n"));
            let pushes = if push.is_empty() { urls } else { push };
            for u in pushes {
                out.push_str(&format!("{name}\t{u} (push)\n"));
            }
        } else {
            out.push_str(&format!("{name}\n"));
        }
    }
    print!("{out}");
    Ok(())
}

fn remote_add(args: &[String]) -> CliResult<()> {
    let p = parse(
        "remote add",
        args,
        &[
            o("fetch", 'f', No, true),
            o("tags", ' ', No, true),
            o("track", 't', Req, true),
            o("master", 'm', Req, true),
            o("mirror", ' ', Opt, true),
        ],
        Mode::default(),
    )?;
    let operands = p.operands();
    if operands.len() != 2 {
        return Err(CliError::Exit(129, USAGE.into()));
    }
    let (name, url) = (&operands[0], &operands[1]);
    let mirror = match p.opts().rev().find(|(key, _, _)| *key == "mirror") {
        Some((_, _, true)) => None,
        Some((_, value, false)) => match value {
            None | Some("fetch") => Some("fetch"),
            Some("push") => Some("push"),
            Some(o) => return Err(CliError::fatal(format!("unknown mirror argument: {o}"))),
        },
        None => None,
    };
    if mirror.is_some() && (!p.values("track").is_empty() || p.value("master").is_some()) {
        return Err(CliError::fatal(
            "specifying branches to track makes sense only with fetch mirrors",
        ));
    }
    let repo = open_repo()?;
    let path = repo.git_dir().join("config");
    let cfg = arks_config::ConfigSet::load(Some(repo.common_dir()));
    if configured_source(&cfg, name).is_some() {
        return Err(CliError::Exit(
            3,
            format!("error: remote {name} already exists."),
        ));
    }
    if !valid_remote_name(name) {
        return Err(CliError::fatal(format!(
            "'{name}' is not a valid remote name"
        )));
    }
    let mut cf = ConfigFile::load(&path);
    let sub = Some(name.as_str());
    cf.add("remote", sub, "url", url);
    match mirror {
        Some("fetch") => cf.add("remote", sub, "fetch", "+refs/*:refs/*"),
        Some(_) => cf.add("remote", sub, "mirror", "true"),
        None => {
            let tracks = p.values("track");
            if tracks.is_empty() {
                cf.add(
                    "remote",
                    sub,
                    "fetch",
                    &format!("+refs/heads/*:refs/remotes/{name}/*"),
                );
            } else {
                for t in tracks {
                    cf.add(
                        "remote",
                        sub,
                        "fetch",
                        &format!("+refs/heads/{t}:refs/remotes/{name}/{t}"),
                    );
                }
            }
        }
    }
    match p.flag("tags") {
        Some(true) => cf.add("remote", sub, "tagOpt", "--tags"),
        Some(false) => cf.add("remote", sub, "tagOpt", "--no-tags"),
        None => {}
    }
    cf.save(&path)?;
    if p.on("fetch") {
        println!("Updating {name}");
        crate::cmd::fetch::cmd_fetch(std::slice::from_ref(name))?;
    }
    if let Some(m) = p.value("master") {
        let mut repo = open_repo()?;
        repo.refs_mut().set_symbolic(
            &format!("refs/remotes/{name}/HEAD"),
            &format!("refs/remotes/{name}/{m}"),
        )?;
    }
    Ok(())
}

fn remote_rename(args: &[String]) -> CliResult<()> {
    let p = parse(
        "remote rename",
        args,
        &[o("progress", ' ', No, true)],
        Mode::default(),
    )?;
    let operands = p.operands();
    if operands.len() != 2 {
        return Err(CliError::Exit(129, USAGE.into()));
    }
    let (old, new) = (&operands[0], &operands[1]);
    let mut repo = open_repo()?;
    let cfg = arks_config::ConfigSet::load(Some(repo.common_dir()));
    let Some(src) = configured_source(&cfg, old) else {
        return Err(no_such(old));
    };
    if configured_source(&cfg, new).is_some() {
        return Err(CliError::Exit(
            3,
            format!("error: remote {new} already exists."),
        ));
    }
    if !valid_remote_name(new) {
        return Err(CliError::fatal(format!(
            "'{new}' is not a valid remote name"
        )));
    }
    let path = repo.git_dir().join("config");
    let mut cf = ConfigFile::load(&path);
    cf.rename_section("remote", old, new);
    // Default-shaped fetch refspecs follow the rename.
    let from = format!("refs/remotes/{old}/");
    let to = format!("refs/remotes/{new}/");
    for spec in &src.config_specs {
        if spec.contains(&from) {
            let renamed = spec.replace(&from, &to);
            let target = spec.clone();
            cf.unset_where("remote", Some(new), "fetch", &move |v| v == target);
            cf.add("remote", Some(new), "fetch", &renamed);
        }
    }
    for b in cf.subs_with("branch", "remote", old) {
        cf.unset_where("branch", Some(&b), "remote", &|_| true);
        cf.add("branch", Some(&b), "remote", new);
    }
    for b in cf.subs_with("branch", "pushremote", old) {
        cf.unset_where("branch", Some(&b), "pushremote", &|_| true);
        cf.add("branch", Some(&b), "pushRemote", new);
    }
    cf.save(&path)?;
    let refs = repo.refs().list_refs(&from)?;
    let head_target = match repo.refs().read(&format!("{from}HEAD")) {
        Ok(arks_refs::RefTarget::Symbolic(t)) => Some(t),
        _ => None,
    };
    for (n, oid) in refs {
        if n == format!("{from}HEAD") {
            continue;
        }
        let renamed = format!("{to}{}", &n[from.len()..]);
        repo.refs_mut().update(&renamed, &oid, None)?;
        delete_ref_any(&mut repo, &n)?;
    }
    if let Some(t) = head_target {
        let _ = fs::remove_file(repo.common_dir().join(format!("{from}HEAD")));
        repo.refs_mut()
            .set_symbolic(&format!("{to}HEAD"), &t.replacen(&from, &to, 1))?;
    }
    Ok(())
}

fn remote_remove(args: &[String]) -> CliResult<()> {
    let p = parse("remote remove", args, &[], Mode::default())?;
    let operands = p.operands();
    if operands.len() != 1 {
        return Err(CliError::Exit(129, USAGE.into()));
    }
    let name = &operands[0];
    let mut repo = open_repo()?;
    let cfg = arks_config::ConfigSet::load(Some(repo.common_dir()));
    let Some(src) = configured_source(&cfg, name) else {
        return Err(no_such(name));
    };
    let path = repo.git_dir().join("config");
    let mut cf = ConfigFile::load(&path);
    for b in cf.subs_with("branch", "remote", name) {
        for k in ["remote", "merge", "pushremote"] {
            cf.unset_where("branch", Some(&b), k, &|_| true);
        }
        let empty = cf
            .sections("branch", Some(&b))
            .iter()
            .all(|(s, e)| self_empty(&cf.lines[s + 1..*e]));
        if empty {
            cf.remove_section("branch", Some(&b));
        }
    }
    cf.remove_section("remote", Some(name));
    cf.save(&path)?;
    // Drop the refs this remote's refspecs stored.
    let local = repo.refs().list_refs("refs/")?;
    for spec in src.config_specs.iter().map(|s| parse_refspec(s)) {
        for (n, _) in &local {
            let maps = spec.map_dst(n).is_some() || spec.dst.as_deref() == Some(n.as_str());
            if maps && n.starts_with("refs/remotes/") {
                delete_ref_any(&mut repo, n)?;
            }
        }
    }
    let head = repo.common_dir().join(format!("refs/remotes/{name}/HEAD"));
    let _ = fs::remove_file(head);
    Ok(())
}

fn self_empty(lines: &[String]) -> bool {
    lines.iter().all(|l| l.trim().is_empty())
}

/// The peer's `HEAD` branch (local peers only).
fn remote_head_branch(url: &str) -> Option<String> {
    let peer = arks_remote::open_local_remote(url).ok()?;
    match peer.head_target() {
        Ok(arks_refs::RefTarget::Symbolic(s)) => s.strip_prefix("refs/heads/").map(String::from),
        _ => None,
    }
}

fn remote_set_head(args: &[String]) -> CliResult<()> {
    let p = parse(
        "remote set-head",
        args,
        &[o("auto", 'a', No, true), o("delete", 'd', No, true)],
        Mode::default(),
    )?;
    let operands = p.operands();
    let Some(name) = operands.first() else {
        return Err(CliError::Exit(129, USAGE.into()));
    };
    let mut repo = open_repo()?;
    let cfg = arks_config::ConfigSet::load(Some(repo.common_dir()));
    let head_ref = format!("refs/remotes/{name}/HEAD");
    if p.on("delete") {
        let path = repo.common_dir().join(&head_ref);
        if path.is_file() {
            fs::remove_file(path)?;
        }
        return Ok(());
    }
    let branch = if p.on("auto") {
        let src = configured_source(&cfg, name).ok_or_else(|| no_such(name))?;
        match remote_head_branch(&src.url) {
            Some(b) => b,
            None => {
                return Err(CliError::Exit(
                    1,
                    "error: Cannot determine remote HEAD".into(),
                ));
            }
        }
    } else {
        match operands.get(1) {
            Some(b) => b.clone(),
            None => return Err(CliError::Exit(129, USAGE.into())),
        }
    };
    let target = format!("refs/remotes/{name}/{branch}");
    if repo.refs().resolve(&target).is_err() {
        return Err(CliError::Exit(
            1,
            format!("error: Not a valid ref: {target}"),
        ));
    }
    repo.refs_mut().set_symbolic(&head_ref, &target)?;
    if p.on("auto") {
        println!("{name}/HEAD set to {branch}");
    }
    Ok(())
}

fn remote_set_branches(args: &[String]) -> CliResult<()> {
    let p = parse(
        "remote set-branches",
        args,
        &[o("add", ' ', No, true)],
        Mode::default(),
    )?;
    let operands = p.operands();
    let Some(name) = operands.first() else {
        return Err(CliError::Exit(129, USAGE.into()));
    };
    let repo = open_repo()?;
    let cfg = arks_config::ConfigSet::load(Some(repo.common_dir()));
    if configured_source(&cfg, name).is_none() {
        return Err(no_such(name));
    }
    let path = repo.git_dir().join("config");
    let mut cf = ConfigFile::load(&path);
    if !p.on("add") {
        cf.unset_where("remote", Some(name), "fetch", &|_| true);
    }
    for b in &operands[1..] {
        cf.add(
            "remote",
            Some(name),
            "fetch",
            &format!("+refs/heads/{b}:refs/remotes/{name}/{b}"),
        );
    }
    cf.save(&path)
}

fn remote_get_url(args: &[String]) -> CliResult<()> {
    let p = parse(
        "remote get-url",
        args,
        &[o("push", ' ', No, true), o("all", ' ', No, true)],
        Mode::default(),
    )?;
    let operands = p.operands();
    let Some(name) = operands.first() else {
        return Err(CliError::Exit(129, USAGE.into()));
    };
    let repo = open_repo()?;
    let cfg = arks_config::ConfigSet::load(Some(repo.common_dir()));
    if configured_source(&cfg, name).is_none() {
        return Err(no_such(name));
    }
    let urls = cfg.get_all(&format!("remote.{name}.url"));
    let push = cfg.get_all(&format!("remote.{name}.pushurl"));
    let list = if p.on("push") && !push.is_empty() {
        push
    } else {
        urls
    };
    let shown: Vec<String> = if p.on("all") {
        list
    } else {
        list.into_iter().take(1).collect()
    };
    for u in shown {
        println!("{u}");
    }
    Ok(())
}

fn remote_set_url(args: &[String]) -> CliResult<()> {
    let p = parse(
        "remote set-url",
        args,
        &[
            o("push", ' ', No, true),
            o("add", ' ', No, true),
            o("delete", ' ', No, true),
        ],
        Mode::default(),
    )?;
    let operands = p.operands();
    let push = p.on("push");
    let key = if push { "pushurl" } else { "url" };
    let n = operands.len();
    if (p.on("add") || p.on("delete")) && n != 2 || !(2..=3).contains(&n) {
        return Err(CliError::Exit(129, USAGE.into()));
    }
    let name = &operands[0];
    let repo = open_repo()?;
    let cfg = arks_config::ConfigSet::load(Some(repo.common_dir()));
    if configured_source(&cfg, name).is_none() {
        return Err(no_such(name));
    }
    let path = repo.git_dir().join("config");
    let mut cf = ConfigFile::load(&path);
    let current = cf.get_all("remote", Some(name), key);
    let new = operands[1].clone();
    if p.on("add") {
        cf.add("remote", Some(name), key, &new);
        return cf.save(&path);
    }
    if p.on("delete") {
        let pat = new;
        let matching: Vec<&String> = current.iter().filter(|u| u.contains(&pat)).collect();
        if matching.is_empty() {
            return Err(CliError::fatal(format!("No such URL found: {pat}")));
        }
        if !push && matching.len() == current.len() {
            return Err(CliError::fatal("Will not delete all non-push URLs"));
        }
        cf.unset_where("remote", Some(name), key, &move |v| v.contains(&pat));
        return cf.save(&path);
    }
    let old = operands.get(2).cloned();
    match &old {
        Some(o) => {
            if !current.iter().any(|u| u.contains(o.as_str())) {
                return Err(CliError::fatal(format!("No such URL found: {o}")));
            }
            let first = current
                .iter()
                .find(|u| u.contains(o.as_str()))
                .cloned()
                .unwrap_or_default();
            replace_value(&mut cf, name, key, &first, &new);
        }
        None => match current.first() {
            Some(first) => {
                let first = first.clone();
                replace_value(&mut cf, name, key, &first, &new);
            }
            None => cf.add("remote", Some(name), key, &new),
        },
    }
    cf.save(&path)
}

/// Replace the first `key = old` line in the remote's section, in place.
fn replace_value(cf: &mut ConfigFile, name: &str, key: &str, old: &str, new: &str) {
    for (start, end) in cf.sections("remote", Some(name)) {
        for i in start + 1..end {
            if key_of(&cf.lines[i]).is_some_and(|(k, v)| k == key && v == old) {
                cf.lines[i] = format!("\t{key} = {}", quote(new));
                return;
            }
        }
    }
}

/// Tracking refs whose remote branch is gone (prune / show).
fn stale_refs(
    repo: &arks_repo::Repository,
    specs: &[String],
    remote_refs: &[(String, Oid)],
) -> CliResult<Vec<String>> {
    let local = repo.refs().list_refs("refs/")?;
    let mut out = Vec::new();
    for spec in specs
        .iter()
        .map(|s| parse_refspec(s))
        .filter(|s| !s.negative)
    {
        for (n, _) in &local {
            if let Some(src) = spec.map_dst(n) {
                if !remote_refs.iter().any(|(r, _)| *r == src) && !out.contains(n) {
                    out.push(n.clone());
                }
            }
        }
    }
    out.sort();
    Ok(out)
}

fn peer_refs(url: &str) -> CliResult<Vec<(String, Oid)>> {
    if arks_remote::is_local_remote(url) {
        let peer =
            arks_remote::open_local_remote(url).map_err(|e| CliError::fatal(e.to_string()))?;
        return Ok(peer.refs().list_refs("refs/")?);
    }
    Ok(arks_remote::ls_remote(url)
        .map_err(|e| CliError::fatal(e.to_string()))?
        .into_iter()
        .filter(|r| r.name.starts_with("refs/") && !r.name.ends_with("^{}"))
        .map(|r| (r.name, r.oid))
        .collect())
}

fn remote_prune(args: &[String]) -> CliResult<()> {
    let p = parse(
        "remote prune",
        args,
        &[o("dry-run", 'n', No, true)],
        Mode::default(),
    )?;
    let operands = p.operands();
    if operands.is_empty() {
        return Err(CliError::Exit(129, USAGE.into()));
    }
    let mut repo = open_repo()?;
    let cfg = arks_config::ConfigSet::load(Some(repo.common_dir()));
    for name in &operands {
        let src = configured_source(&cfg, name).ok_or_else(|| no_such(name))?;
        let remote_refs = peer_refs(&src.url)?;
        let stale = stale_refs(&repo, &src.config_specs, &remote_refs)?;
        if stale.is_empty() {
            continue;
        }
        println!("Pruning {name}");
        println!("URL: {}", src.url);
        for n in &stale {
            let short = n.strip_prefix("refs/remotes/").unwrap_or(n);
            if p.on("dry-run") {
                println!(" * [would prune] {short}");
            } else {
                delete_ref_any(&mut repo, n)?;
                println!(" * [pruned] {short}");
            }
        }
    }
    Ok(())
}

fn remote_update(args: &[String], verbose: bool) -> CliResult<()> {
    let p = parse(
        "remote update",
        args,
        &[o("prune", 'p', No, true)],
        Mode::default(),
    )?;
    let operands = p.operands();
    let mut fetch: Vec<String> = Vec::new();
    if verbose {
        fetch.push("-v".into());
    }
    if p.on("prune") {
        fetch.push("--prune".into());
    }
    if operands.is_empty() {
        let repo = open_repo()?;
        let cfg = arks_config::ConfigSet::load(Some(repo.common_dir()));
        if !cfg.get_all("remotes.default").is_empty() {
            fetch.push("--multiple".into());
            fetch.push("default".into());
        } else {
            let names: Vec<String> = remote_names(&cfg)
                .into_iter()
                .filter(|n| {
                    !cfg.get_bool(&format!("remote.{n}.skipdefaultupdate"))
                        .unwrap_or(false)
                })
                .collect();
            if names.len() == 1 {
                fetch.push(names[0].clone());
            } else {
                fetch.push("--multiple".into());
                fetch.extend(names);
            }
        }
    } else {
        fetch.push("--multiple".into());
        fetch.extend(operands.iter().cloned());
    }
    crate::cmd::fetch::cmd_fetch(&fetch)
}

fn remote_show(args: &[String], verbose: bool) -> CliResult<()> {
    let p = parse(
        "remote show",
        args,
        &[o("-n", 'n', No, false)],
        Mode::default(),
    )?;
    let operands = p.operands();
    if operands.is_empty() {
        return list_remotes(verbose);
    }
    let repo = open_repo()?;
    let cfg = arks_config::ConfigSet::load(Some(repo.common_dir()));
    let no_query = p.on("-n");
    let mut out = String::new();
    for name in &operands {
        let Some(src) = configured_source(&cfg, name) else {
            return Err(CliError::fatal(format!(
                "'{name}' does not appear to be a git repository\nfatal: Could not read from remote repository.\n\nPlease make sure you have the correct access rights\nand the repository exists."
            )));
        };
        let urls = cfg.get_all(&format!("remote.{name}.url"));
        let push = cfg.get_all(&format!("remote.{name}.pushurl"));
        out.push_str(&format!("* remote {name}\n"));
        out.push_str(&format!("  Fetch URL: {}\n", src.url));
        for u in if push.is_empty() { &urls } else { &push } {
            out.push_str(&format!("  Push  URL: {u}\n"));
        }
        let tracked_local: Vec<String> = {
            let local = repo.refs().list_refs("refs/remotes/")?;
            let specs: Vec<_> = src.config_specs.iter().map(|s| parse_refspec(s)).collect();
            let mut v: Vec<String> = local
                .iter()
                .filter_map(|(n, _)| specs.iter().find_map(|s| s.map_dst(n)))
                .filter_map(|s| s.strip_prefix("refs/heads/").map(String::from))
                .filter(|b| b != "HEAD")
                .collect();
            v.sort();
            v
        };
        let pulls = branches_merging(&cfg, name);
        if no_query {
            out.push_str("  HEAD branch: (not queried)\n");
            if !tracked_local.is_empty() {
                out.push_str(&format!(
                    "  Remote {}: (status not queried)\n",
                    if tracked_local.len() == 1 {
                        "branch"
                    } else {
                        "branches"
                    }
                ));
                for b in &tracked_local {
                    out.push_str(&format!("    {b}\n"));
                }
            }
            push_pull_section(&mut out, &pulls);
            out.push_str("  Local ref configured for 'git push' (status not queried):\n");
            out.push_str("    (matching) pushes to (matching)\n");
            continue;
        }
        let remote_refs = peer_refs(&src.url)?;
        let head = remote_head_branch(&src.url);
        out.push_str(&format!(
            "  HEAD branch: {}\n",
            head.as_deref().unwrap_or("(unknown)")
        ));
        let specs: Vec<_> = src.config_specs.iter().map(|s| parse_refspec(s)).collect();
        let mut rows: Vec<(String, String)> = Vec::new();
        for (n, _) in &remote_refs {
            let Some(b) = n.strip_prefix("refs/heads/") else {
                continue;
            };
            let Some(dst) = specs.iter().find_map(|s| s.map_src(n)) else {
                continue;
            };
            let status = if repo.refs().resolve(&dst).is_ok() {
                "tracked".to_string()
            } else {
                format!(
                    "new (next fetch will store in {})",
                    dst.strip_prefix("refs/")
                        .and_then(|d| d.rsplit_once('/'))
                        .map(|(a, _)| a.to_string())
                        .unwrap_or_default()
                )
            };
            rows.push((b.to_string(), status));
        }
        for s in stale_refs(&repo, &src.config_specs, &remote_refs)? {
            if let Some(b) = specs.iter().find_map(|sp| sp.map_dst(&s)) {
                let b = b.strip_prefix("refs/heads/").unwrap_or(&b).to_string();
                rows.push((b, "stale (use 'git remote prune' to remove)".into()));
            }
        }
        rows.sort();
        if !rows.is_empty() {
            out.push_str(&format!(
                "  Remote {}:\n",
                if rows.len() == 1 {
                    "branch"
                } else {
                    "branches"
                }
            ));
            let w = rows.iter().map(|r| r.0.len()).max().unwrap_or(0);
            for (b, s) in &rows {
                out.push_str(&format!("    {b:<w$} {s}\n"));
            }
        }
        push_pull_section(&mut out, &pulls);
        // Matching push: local branches that also exist on the peer.
        let local = repo.refs().list_refs("refs/heads/")?;
        let mut pushes = Vec::new();
        for (n, oid) in &local {
            let Some((_, roid)) = remote_refs.iter().find(|(r, _)| r == n) else {
                continue;
            };
            let b = n.trim_start_matches("refs/heads/").to_string();
            let status = if oid == roid {
                "up to date"
            } else if repo.get_object(roid).is_ok()
                && crate::cmd::shared::is_ancestor(&repo, roid, oid)?
            {
                "fast-forwardable"
            } else {
                "local out of date"
            };
            pushes.push((b, status));
        }
        if !pushes.is_empty() {
            out.push_str(&format!(
                "  Local {} configured for 'git push':\n",
                if pushes.len() == 1 { "ref" } else { "refs" }
            ));
            let w = pushes.iter().map(|r| r.0.len()).max().unwrap_or(0);
            for (b, s) in &pushes {
                out.push_str(&format!("    {b:<w$} pushes to {b:<w$} ({s})\n"));
            }
        }
    }
    print!("{out}");
    Ok(())
}

/// `branch.<b>.remote == name` branches with their merge targets.
fn branches_merging(cfg: &arks_config::ConfigSet, name: &str) -> Vec<(String, String)> {
    let mut out: Vec<(String, String)> = cfg
        .subsections("branch", "remote")
        .into_iter()
        .filter(|(_, r)| r == name)
        .filter_map(|(b, _)| {
            cfg.get(&format!("branch.{b}.merge"))
                .map(|m| (b.clone(), m.trim_start_matches("refs/heads/").to_string()))
        })
        .collect();
    out.sort();
    out.dedup();
    out
}

fn push_pull_section(out: &mut String, pulls: &[(String, String)]) {
    if pulls.is_empty() {
        return;
    }
    out.push_str(&format!(
        "  Local {} configured for 'git pull':\n",
        if pulls.len() == 1 {
            "branch"
        } else {
            "branches"
        }
    ));
    let w = pulls.iter().map(|r| r.0.len()).max().unwrap_or(0);
    for (b, m) in pulls {
        out.push_str(&format!("    {b:<w$} merges with remote {m}\n"));
    }
}
