//! `git config` (classic option interface of git 2.39).
//!
//! Locations (`--global`/`--system`/`--local`/`--worktree`/`--file`/`--blob`),
//! every action (`--get[-all|-regexp|-urlmatch]`, `--add`, `--replace-all`,
//! `--unset[-all]`, `--rename-section`, `--remove-section`, `--list`, `--edit`,
//! `--get-color[bool]`), types (`--type`, `--bool`, `--int`, `--bool-or-int`,
//! `--bool-or-str`, `--path`, `--expiry-date`, `color`) and output options
//! (`-z`, `--name-only`, `--show-origin`, `--show-scope`, `--includes`,
//! `--default`, `--fixed-value`). Exit codes follow git-config §DESCRIPTION
//! (1 invalid key / missing, 2 no section, 5 unset missing / multiple values).
//! Cite: <https://git-scm.com/docs/git-config>.

use crate::cmd::opt_parse::{self, OptDef, Takes::*, o};
use crate::error::{CliError, CliResult};
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

const SPEC: &[OptDef] = &[
    o("global", ' ', No, true),
    o("system", ' ', No, true),
    o("local", ' ', No, true),
    o("worktree", ' ', No, true),
    o("file", 'f', Req, true),
    o("blob", ' ', Req, true),
    o("get", ' ', No, true),
    o("get-all", ' ', No, true),
    o("get-regexp", ' ', No, true),
    o("get-urlmatch", ' ', No, true),
    o("replace-all", ' ', No, true),
    o("add", ' ', No, true),
    o("unset", ' ', No, true),
    o("unset-all", ' ', No, true),
    o("rename-section", ' ', No, true),
    o("remove-section", ' ', No, true),
    o("list", 'l', No, true),
    o("fixed-value", ' ', No, true),
    o("edit", 'e', No, true),
    o("get-color", ' ', No, true),
    o("get-colorbool", ' ', No, true),
    o("type", 't', Req, true),
    o("bool", ' ', No, false),
    o("int", ' ', No, false),
    o("bool-or-int", ' ', No, false),
    o("bool-or-str", ' ', No, false),
    o("path", ' ', No, false),
    o("expiry-date", ' ', No, false),
    o("null", 'z', No, true),
    o("name-only", ' ', No, true),
    o("includes", ' ', No, true),
    o("show-origin", ' ', No, true),
    o("show-scope", ' ', No, true),
    o("default", ' ', Req, true),
];

const ACTIONS: &[&str] = &[
    "get",
    "get-all",
    "get-regexp",
    "get-urlmatch",
    "replace-all",
    "add",
    "unset",
    "unset-all",
    "rename-section",
    "remove-section",
    "list",
    "edit",
    "get-color",
    "get-colorbool",
];

fn usage(msg: &str) -> CliError {
    let mut m = String::new();
    if !msg.is_empty() {
        m.push_str("error: ");
        m.push_str(msg);
        m.push('\n');
    }
    m.push_str("usage: git config [<options>]");
    CliError::Exit(129, m)
}

fn err(code: u8, msg: impl Into<String>) -> CliError {
    CliError::Exit(code, format!("error: {}", msg.into()))
}

/// Where to read or write.
#[derive(Debug, Clone)]
enum Loc {
    /// Merged view (system, global, local, worktree, command line).
    Merged,
    File {
        path: PathBuf,
        scope: &'static str,
    },
    Blob(String),
}

pub fn cmd_config(args: &[String]) -> CliResult<()> {
    let items = opt_parse::parse("config", args, SPEC, opt_parse::Mode::default())?;
    let pos = items.operands();

    // Location (last-wins per flag; more than one is an error).
    let mut locs: Vec<&str> = Vec::new();
    for l in ["global", "system", "local", "worktree"] {
        if items.on(l) {
            locs.push(l);
        }
    }
    let file = items.value("file");
    let blob = items.value("blob");
    if file.is_some() {
        locs.push("file");
    }
    if blob.is_some() {
        locs.push("blob");
    }
    if locs.len() > 1 {
        return Err(usage("only one config file at a time"));
    }

    // Action.
    let acts: Vec<&str> = ACTIONS.iter().copied().filter(|a| items.on(a)).collect();
    if acts.len() > 1 {
        return Err(usage("only one action at a time"));
    }
    let mut action = acts.first().copied().unwrap_or("");

    // Type: the last of --type / --bool / ... wins; --no-type resets.
    let mut ty: Option<String> = None;
    for (name, value, neg) in items.opts() {
        match name {
            "type" if neg => ty = None,
            "type" => ty = value.map(str::to_string),
            "bool" | "int" | "bool-or-int" | "bool-or-str" | "path" | "expiry-date" => {
                ty = Some(name.to_string())
            }
            _ => {}
        }
    }
    if let Some(t) = &ty {
        if !matches!(
            t.as_str(),
            "bool" | "int" | "bool-or-int" | "bool-or-str" | "path" | "expiry-date" | "color"
        ) {
            return Err(CliError::fatal(format!(
                "unrecognized --type argument, {t}"
            )));
        }
    }
    let null = items.on("null");
    let name_only = items.on("name-only");
    let show_origin = items.on("show-origin");
    let show_scope = items.on("show-scope");
    let fixed = items.on("fixed-value");
    let default = items.value("default");
    let includes = items.flag("includes");

    if action.is_empty() {
        action = match pos.len() {
            0 => return Err(usage("")),
            1 => "get",
            2 => "set",
            3 => "set-pattern",
            _ => return Err(usage("wrong number of arguments, should be from 1 to 3")),
        };
    } else {
        let (lo, hi) = match action {
            "get" | "get-all" | "get-regexp" | "unset" | "unset-all" | "get-color"
            | "get-colorbool" => (1, 2),
            "get-urlmatch" | "add" | "rename-section" => (2, 2),
            "replace-all" => (2, 3),
            "remove-section" => (1, 1),
            _ => (0, 0),
        };
        if pos.len() < lo || pos.len() > hi {
            let m = if lo == hi {
                format!("wrong number of arguments, should be {lo}")
            } else {
                format!("wrong number of arguments, should be from {lo} to {hi}")
            };
            return Err(usage(&m));
        }
    }
    if name_only && !matches!(action, "list" | "get-regexp") {
        return Err(usage(
            "--name-only is only applicable to --list or --get-regexp",
        ));
    }
    if (show_origin || show_scope) && !matches!(action, "get" | "get-all" | "get-regexp" | "list") {
        let which = if show_origin {
            "--show-origin"
        } else {
            "--show-scope"
        };
        return Err(usage(&format!(
            "{which} is only applicable to --get, --get-all, --get-regexp, and --list"
        )));
    }
    if default.is_some() && action != "get" {
        return Err(usage("--default is only applicable to --get"));
    }
    let has_pattern = match action {
        "get" | "get-all" | "get-regexp" | "unset" | "unset-all" => pos.len() == 2,
        "replace-all" | "set-pattern" => pos.len() == 3,
        _ => false,
    };
    if fixed && !has_pattern {
        return Err(usage("--fixed-value only applies with 'value-pattern'"));
    }

    let loc = resolve_loc(locs.first().copied(), file, blob)?;
    let out = Out {
        null,
        name_only,
        show_origin,
        show_scope,
    };
    let ctx = Ctx {
        ty: ty.as_deref(),
        fixed,
    };
    match action {
        "get" | "get-all" | "get-regexp" => {
            let inc = includes.unwrap_or(matches!(loc, Loc::Merged));
            // A missing `--file` reads as empty for lookups (exit 1, silent).
            let entries = match load(&loc, inc) {
                Err(CliError::Exit(128, m)) if m.contains("unable to read config file") => {
                    Vec::new()
                }
                other => other?,
            };
            get(&entries, action, &pos, &ctx, &out, default.as_deref())
        }
        "list" => {
            let inc = includes.unwrap_or(matches!(loc, Loc::Merged));
            let entries = load(&loc, inc)?;
            list(&entries, &out)
        }
        "get-urlmatch" => {
            let entries = load(&loc, includes.unwrap_or(true))?;
            get_urlmatch(&entries, &pos[0], &pos[1], &ctx)
        }
        "get-color" => {
            let entries = load(&loc, true)?;
            let v = entries
                .iter()
                .rev()
                .find(|e| e.key == canonical_lookup(&pos[0]))
                .and_then(|e| e.value.clone())
                .or_else(|| pos.get(1).cloned())
                .unwrap_or_default();
            let esc = parse_color(&v)
                .ok_or_else(|| CliError::fatal("unable to parse default color value"))?;
            print!("{esc}");
            Ok(())
        }
        "get-colorbool" => {
            let entries = load(&loc, true)?;
            colorbool(&entries, &pos)
        }
        "edit" => edit(&loc),
        _ => write_action(&loc, action, &pos, &ctx),
    }
}

struct Out {
    null: bool,
    name_only: bool,
    show_origin: bool,
    show_scope: bool,
}

struct Ctx<'a> {
    ty: Option<&'a str>,
    fixed: bool,
}

#[derive(Debug, Clone)]
struct Ent {
    key: String,
    value: Option<String>,
    scope: &'static str,
    origin: String,
}

fn git_dir() -> Option<PathBuf> {
    let cwd = std::env::current_dir().ok()?;
    arks_repo::Repository::discover(&cwd)
        .ok()
        .map(|r| r.git_dir().to_path_buf())
}

/// Common directory for a (possibly linked-worktree) git dir.
fn common_dir(gd: &Path) -> PathBuf {
    match fs::read_to_string(gd.join("commondir")) {
        Ok(s) => {
            let p = PathBuf::from(s.trim());
            if p.is_relative() { gd.join(p) } else { p }
        }
        Err(_) => gd.to_path_buf(),
    }
}

fn worktree_config_enabled(common: &Path) -> bool {
    let text = fs::read_to_string(common.join("config")).unwrap_or_default();
    arks_config::parse_config_text(&text)
        .ok()
        .and_then(|es| {
            es.into_iter()
                .rev()
                .find(|e| e.key == "extensions.worktreeconfig")
        })
        .map(|e| bool_value(e.value.as_deref()).unwrap_or(false))
        .unwrap_or(false)
}

fn system_path() -> PathBuf {
    if let Some(p) = std::env::var_os("GIT_CONFIG_SYSTEM") {
        return PathBuf::from(p);
    }
    if cfg!(windows) {
        let pd = std::env::var("PROGRAMDATA").unwrap_or_else(|_| "C:\\ProgramData".into());
        PathBuf::from(pd).join("Git").join("config")
    } else {
        PathBuf::from("/etc/gitconfig")
    }
}

fn home() -> Option<PathBuf> {
    std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .filter(|h| !h.is_empty())
        .map(PathBuf::from)
}

fn global_paths() -> Vec<PathBuf> {
    if let Some(p) = std::env::var_os("GIT_CONFIG_GLOBAL") {
        return vec![PathBuf::from(p)];
    }
    let mut out = Vec::new();
    let xdg = std::env::var_os("XDG_CONFIG_HOME")
        .filter(|x| !x.is_empty())
        .map(PathBuf::from)
        .or_else(|| home().map(|h| h.join(".config")));
    if let Some(x) = xdg {
        out.push(x.join("git").join("config"));
    }
    if let Some(h) = home() {
        out.push(h.join(".gitconfig"));
    }
    out
}

/// `--global` write target: `GIT_CONFIG_GLOBAL`, else `~/.gitconfig` unless
/// only the XDG file exists.
fn global_write_path() -> CliResult<PathBuf> {
    if let Some(p) = std::env::var_os("GIT_CONFIG_GLOBAL") {
        return Ok(PathBuf::from(p));
    }
    let paths = global_paths();
    let user = home()
        .map(|h| h.join(".gitconfig"))
        .ok_or_else(|| CliError::fatal("$HOME not set"))?;
    if !user.exists() {
        if let Some(x) = paths.first().filter(|x| x.exists()) {
            return Ok(x.clone());
        }
    }
    Ok(user)
}

fn resolve_loc(which: Option<&str>, file: Option<String>, blob: Option<String>) -> CliResult<Loc> {
    Ok(match which {
        None => match std::env::var_os("GIT_CONFIG") {
            Some(p) if !p.is_empty() => Loc::File {
                path: PathBuf::from(p),
                scope: "command",
            },
            _ => Loc::Merged,
        },
        Some("global") => Loc::File {
            path: global_write_path()?,
            scope: "global",
        },
        Some("system") => Loc::File {
            path: system_path(),
            scope: "system",
        },
        Some("local") => {
            let gd = git_dir().ok_or_else(|| {
                CliError::fatal("--local can only be used inside a git repository")
            })?;
            Loc::File {
                path: common_dir(&gd).join("config"),
                scope: "local",
            }
        }
        Some("worktree") => {
            let gd = git_dir().ok_or_else(|| {
                CliError::fatal("--worktree can only be used inside a git repository")
            })?;
            let common = common_dir(&gd);
            if worktree_config_enabled(&common) {
                Loc::File {
                    path: gd.join("config.worktree"),
                    scope: "worktree",
                }
            } else if common
                .join("worktrees")
                .read_dir()
                .map(|mut d| d.next().is_some())
                .unwrap_or(false)
            {
                return Err(CliError::fatal(
                    "--worktree cannot be used with multiple working trees unless the config\nextension worktreeConfig is enabled. Please read \"CONFIGURATION FILE\"\nsection in \"git help worktree\" for details",
                ));
            } else {
                Loc::File {
                    path: common.join("config"),
                    scope: "local",
                }
            }
        }
        Some("file") => Loc::File {
            path: PathBuf::from(file.unwrap_or_default()),
            scope: "command",
        },
        Some("blob") => Loc::Blob(blob.unwrap_or_default()),
        Some(_) => Loc::Merged,
    })
}

/// Display form of a config file path (relative to the cwd when below it).
fn shown(p: &Path) -> String {
    let cwd = std::env::current_dir().unwrap_or_default();
    match p.strip_prefix(&cwd) {
        Ok(r) if p.is_absolute() => r.display().to_string(),
        _ => p.display().to_string(),
    }
}

fn load_file(
    path: &Path,
    scope: &'static str,
    includes: bool,
    depth: usize,
    out: &mut Vec<Ent>,
    must_exist: bool,
) -> CliResult<()> {
    let text = match fs::read(path) {
        Ok(b) => String::from_utf8_lossy(&b).into_owned(),
        Err(e) => {
            if must_exist {
                return Err(CliError::fatal(format!(
                    "unable to read config file '{}': {}",
                    path.display(),
                    io_reason(&e)
                )));
            }
            return Ok(());
        }
    };
    let entries = arks_config::parse_config_text(&text).map_err(|e| {
        CliError::fatal(format!(
            "bad config line {} in file {}",
            e.line,
            shown(path)
        ))
    })?;
    for e in entries {
        if includes && depth < 10 && e.key == "include.path" {
            if let Some(v) = e.value.as_deref() {
                let inc = expand_tilde(v);
                let inc = if inc.is_relative() {
                    path.parent().unwrap_or(Path::new(".")).join(inc)
                } else {
                    inc
                };
                out.push(Ent {
                    key: e.key.clone(),
                    value: e.value.clone(),
                    scope,
                    origin: format!("file:{}", shown(path)),
                });
                load_file(&inc, scope, includes, depth + 1, out, false)?;
            }
            continue;
        }
        out.push(Ent {
            key: e.key,
            value: e.value,
            scope,
            origin: format!("file:{}", shown(path)),
        });
    }
    Ok(())
}

fn io_reason(e: &std::io::Error) -> &'static str {
    match e.kind() {
        std::io::ErrorKind::NotFound => "No such file or directory",
        std::io::ErrorKind::PermissionDenied => "Permission denied",
        _ => "Input/output error",
    }
}

fn load(loc: &Loc, includes: bool) -> CliResult<Vec<Ent>> {
    let mut out = Vec::new();
    match loc {
        Loc::File { path, scope } => {
            let must = *scope == "command";
            load_file(path, scope, includes, 0, &mut out, must)?;
        }
        Loc::Blob(spec) => {
            let repo = crate::util::open_repo()?;
            let oid = crate::cmd::shared::resolve_rev(&repo, spec)
                .map_err(|_| CliError::fatal(format!("unable to resolve config blob '{spec}'")))?;
            let obj = repo.get_object(&oid)?;
            let text = String::from_utf8_lossy(&obj.payload).into_owned();
            let entries = arks_config::parse_config_text(&text).map_err(|e| {
                CliError::Exit(
                    128,
                    format!(
                        "error: bad config line {} in blob {spec}\nfatal: error processing config file(s)",
                        e.line
                    ),
                )
            })?;
            for e in entries {
                out.push(Ent {
                    key: e.key,
                    value: e.value,
                    scope: "command",
                    origin: format!("blob:{spec}"),
                });
            }
        }
        Loc::Merged => {
            let nosystem = std::env::var("GIT_CONFIG_NOSYSTEM")
                .map(|v| bool_value(Some(&v)).unwrap_or(true))
                .unwrap_or(false);
            if !nosystem {
                load_file(&system_path(), "system", includes, 0, &mut out, false)?;
            }
            for p in global_paths() {
                load_file(&p, "global", includes, 0, &mut out, false)?;
            }
            if let Some(gd) = git_dir() {
                let common = common_dir(&gd);
                load_file(
                    &common.join("config"),
                    "local",
                    includes,
                    0,
                    &mut out,
                    false,
                )?;
                if worktree_config_enabled(&common) {
                    load_file(
                        &gd.join("config.worktree"),
                        "worktree",
                        includes,
                        0,
                        &mut out,
                        false,
                    )?;
                }
            }
            // Environment (`GIT_CONFIG_COUNT`) and `-c` / `GIT_CONFIG_PARAMETERS`.
            let set = arks_config::ConfigSet::load(None);
            for e in set.entries() {
                if matches!(
                    e.scope,
                    arks_config::ConfigScope::Env | arks_config::ConfigScope::Command
                ) {
                    out.push(Ent {
                        key: e.key.clone(),
                        value: e.value.clone(),
                        scope: "command",
                        origin: "command line:".into(),
                    });
                }
            }
        }
    }
    Ok(out)
}

fn expand_tilde(v: &str) -> PathBuf {
    if v == "~" {
        return home().unwrap_or_else(|| PathBuf::from(v));
    }
    if let Some(rest) = v.strip_prefix("~/") {
        if let Some(h) = home() {
            return h.join(rest);
        }
    }
    PathBuf::from(v)
}

/// Split and validate a user key (`git_config_parse_key`). Returns
/// (canonical key, section as typed, subsection, name as typed).
struct Key {
    canon: String,
    section: String,
    sub: Option<String>,
    name: String,
}

fn parse_key(key: &str) -> CliResult<Key> {
    let last = match key.rfind('.') {
        Some(i) if i > 0 => i,
        _ => {
            return Err(err(2, format!("key does not contain a section: {key}")));
        }
    };
    if last + 1 == key.len() {
        return Err(err(2, format!("key does not contain variable name: {key}")));
    }
    let first = key.find('.').unwrap_or(last);
    let section = &key[..first];
    let name = &key[last + 1..];
    let sub = (first != last).then(|| key[first + 1..last].to_string());
    let sec_ok = section
        .bytes()
        .all(|b| b.is_ascii_alphanumeric() || b == b'-');
    let name_ok = name.as_bytes()[0].is_ascii_alphabetic()
        && name.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-');
    let sub_ok = sub.as_deref().is_none_or(|s| !s.contains('\n'));
    if section.is_empty() || !sec_ok || !name_ok || !sub_ok {
        return Err(err(1, format!("invalid key: {key}")));
    }
    let canon = match &sub {
        Some(s) => format!(
            "{}.{s}.{}",
            section.to_ascii_lowercase(),
            name.to_ascii_lowercase()
        ),
        None => format!(
            "{}.{}",
            section.to_ascii_lowercase(),
            name.to_ascii_lowercase()
        ),
    };
    Ok(Key {
        canon,
        section: section.to_string(),
        sub,
        name: name.to_string(),
    })
}

/// Canonical form for lookups that do not validate (get-color slot).
fn canonical_lookup(key: &str) -> String {
    parse_key(key)
        .map(|k| k.canon)
        .unwrap_or_else(|_| key.to_ascii_lowercase())
}

/// git boolean (`git_config_bool`): true/yes/on/1.., false/no/off/0/"".
fn bool_value(v: Option<&str>) -> Option<bool> {
    let Some(v) = v else { return Some(true) };
    match v.to_ascii_lowercase().as_str() {
        "true" | "yes" | "on" => Some(true),
        "false" | "no" | "off" | "" => Some(false),
        _ => parse_int(v).map(|n| n != 0),
    }
}

/// git signed integer with optional k/m/g unit.
fn parse_int(v: &str) -> Option<i64> {
    let v = v.trim();
    if v.is_empty() {
        return None;
    }
    let (num, mult) = match v.chars().last().map(|c| c.to_ascii_lowercase()) {
        Some('k') => (&v[..v.len() - 1], 1024i64),
        Some('m') => (&v[..v.len() - 1], 1024 * 1024),
        Some('g') => (&v[..v.len() - 1], 1024 * 1024 * 1024),
        _ => (v, 1),
    };
    let n: i64 = num.parse().ok()?;
    n.checked_mul(mult)
}

/// Render a value under `--type` for output.
fn typed_out(ty: Option<&str>, key: &str, v: Option<&str>, origin: &str) -> CliResult<String> {
    match ty {
        Some("bool") => bool_value(v).map(|b| b.to_string()).ok_or_else(|| {
            CliError::fatal(format!(
                "bad boolean config value '{}' for '{key}'",
                v.unwrap_or("")
            ))
        }),
        Some("int") => {
            let raw = v.unwrap_or("");
            parse_int(raw).map(|n| n.to_string()).ok_or_else(|| {
                CliError::fatal(format!(
                    "bad numeric config value '{raw}' for '{key}' in {}: invalid unit",
                    origin_words(origin)
                ))
            })
        }
        Some("bool-or-int") => match v {
            None => Ok("true".into()),
            Some(s) => match parse_int(s) {
                Some(n) => Ok(n.to_string()),
                None => bool_value(Some(s)).map(|b| b.to_string()).ok_or_else(|| {
                    CliError::fatal(format!("bad boolean config value '{s}' for '{key}'"))
                }),
            },
        },
        Some("bool-or-str") => match v {
            None => Ok("true".into()),
            Some(s) => Ok(match bool_value(Some(s)) {
                Some(b) if parse_int(s).is_none() => b.to_string(),
                _ => s.to_string(),
            }),
        },
        Some("path") => match v {
            None => Err(CliError::Exit(
                128,
                format!("error: missing value for '{key}'\nfatal: bad config variable '{key}'"),
            )),
            Some(s) => Ok(expand_tilde(s).display().to_string()),
        },
        Some("expiry-date") => {
            let s = v.unwrap_or("");
            expiry_date(s).map(|t| t.to_string()).ok_or_else(|| {
                CliError::fatal(format!("'{s}' for '{key}' is not a valid timestamp"))
            })
        }
        Some("color") => {
            let s = v.unwrap_or("");
            parse_color(s).ok_or_else(|| {
                CliError::Exit(
                    128,
                    format!("error: invalid color value: {s}\nfatal: cannot parse color '{s}'"),
                )
            })
        }
        _ => Ok(v.unwrap_or("").to_string()),
    }
}

fn origin_words(origin: &str) -> String {
    match origin.strip_prefix("file:") {
        Some(p) => format!("file {p}"),
        None => match origin.strip_prefix("blob:") {
            Some(b) => format!("blob {b}"),
            None => "command line".into(),
        },
    }
}

/// Canonicalize a value being written under `--type`.
fn typed_in(ty: Option<&str>, key: &str, v: &str) -> CliResult<String> {
    match ty {
        Some("bool") => bool_value(Some(v))
            .map(|b| b.to_string())
            .ok_or_else(|| CliError::fatal(format!("bad boolean config value '{v}' for '{key}'"))),
        Some("int") => parse_int(v).map(|n| n.to_string()).ok_or_else(|| {
            CliError::fatal(format!(
                "bad numeric config value '{v}' for '{key}': invalid unit"
            ))
        }),
        Some("bool-or-int") => match parse_int(v) {
            Some(n) => Ok(n.to_string()),
            None => bool_value(Some(v)).map(|b| b.to_string()).ok_or_else(|| {
                CliError::fatal(format!("bad boolean config value '{v}' for '{key}'"))
            }),
        },
        Some("bool-or-str") => Ok(match bool_value(Some(v)) {
            Some(b) if parse_int(v).is_none() => b.to_string(),
            _ => v.to_string(),
        }),
        Some("expiry-date") => {
            if expiry_date(v).is_none() {
                return Err(CliError::fatal(format!(
                    "'{v}' for '{key}' is not a valid timestamp"
                )));
            }
            Ok(v.to_string())
        }
        Some("color") => {
            if parse_color(v).is_none() {
                return Err(CliError::Exit(
                    128,
                    format!("error: invalid color value: {v}\nfatal: cannot parse color '{v}'"),
                ));
            }
            Ok(v.to_string())
        }
        _ => Ok(v.to_string()),
    }
}

/// Approximate expiry date: `never`/`false` → 0, `now`/`all` → max, relative
/// `<n>.<unit>.ago`, or an absolute git date. Cite: git-config `--expiry-date`.
pub(crate) fn expiry_date(s: &str) -> Option<i64> {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);
    let t = s.trim().to_ascii_lowercase();
    match t.as_str() {
        "never" | "false" => return Some(0),
        "now" | "all" => return Some(i64::MAX),
        _ => {}
    }
    if let Some((secs, _)) = crate::cmd::shared::parse_git_date(&t) {
        return Some(secs);
    }
    let words: Vec<&str> = t.split(['.', ' ', '_']).filter(|w| !w.is_empty()).collect();
    if words.last() != Some(&"ago") || words.len() < 2 {
        return None;
    }
    let mut total = 0i64;
    let mut i = 0;
    while i + 1 < words.len() {
        let n: i64 = words[i].parse().ok()?;
        let unit = words[i + 1].trim_end_matches('s');
        let secs = match unit {
            "second" | "sec" => 1,
            "minute" | "min" => 60,
            "hour" => 3600,
            "day" => 86_400,
            "week" => 7 * 86_400,
            "month" => 30 * 86_400,
            "year" => 365 * 86_400,
            _ => return None,
        };
        total += n * secs;
        i += 2;
    }
    Some(now - total)
}

/// ANSI escape for a git color spec (`git_config_color`); `None` when invalid.
pub(crate) fn parse_color(spec: &str) -> Option<String> {
    let words: Vec<&str> = spec.split_whitespace().collect();
    if words.is_empty() {
        return Some(String::new());
    }
    let mut attrs: Vec<u32> = Vec::new();
    let mut fg: Option<String> = None;
    let mut bg: Option<String> = None;
    let mut reset = false;
    for w in words {
        let lw = w.to_ascii_lowercase();
        if lw == "reset" {
            reset = true;
            continue;
        }
        if let Some(c) = color_code(&lw) {
            if fg.is_none() {
                fg = Some(c.fg);
            } else if bg.is_none() {
                bg = Some(c.bg);
            } else {
                return None;
            }
            continue;
        }
        let (neg, base) = match lw.strip_prefix("no-").or_else(|| lw.strip_prefix("no")) {
            Some(b) if attr_code(b).is_some() => (true, b),
            _ => (false, lw.as_str()),
        };
        let code = attr_code(base)?;
        let code = if neg {
            if code == 1 { 22 } else { code + 20 }
        } else {
            code
        };
        if !attrs.contains(&code) {
            attrs.push(code);
        }
    }
    attrs.sort_unstable();
    let mut parts: Vec<String> = Vec::new();
    if reset {
        parts.push(String::new());
    }
    parts.extend(attrs.iter().map(|a| a.to_string()));
    if let Some(f) = fg {
        parts.push(f);
    }
    if let Some(b) = bg {
        parts.push(b);
    }
    if parts.is_empty() {
        return Some(String::new());
    }
    Some(format!("\x1b[{}m", parts.join(";")))
}

struct ColorCode {
    fg: String,
    bg: String,
}

fn color_code(w: &str) -> Option<ColorCode> {
    const NAMES: &[&str] = &[
        "black", "red", "green", "yellow", "blue", "magenta", "cyan", "white",
    ];
    if w == "normal" {
        return normal_code();
    }
    if w == "default" {
        return Some(ColorCode {
            fg: "39".into(),
            bg: "49".into(),
        });
    }
    if let Some(i) = NAMES.iter().position(|n| *n == w) {
        return Some(ColorCode {
            fg: (30 + i).to_string(),
            bg: (40 + i).to_string(),
        });
    }
    if let Some(rest) = w.strip_prefix("bright") {
        let i = NAMES.iter().position(|n| *n == rest)?;
        return Some(ColorCode {
            fg: (90 + i).to_string(),
            bg: (100 + i).to_string(),
        });
    }
    if let Some(hex) = w.strip_prefix('#') {
        if hex.len() == 6 && hex.bytes().all(|b| b.is_ascii_hexdigit()) {
            let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
            let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
            let b = u8::from_str_radix(&hex[4..6], 16).ok()?;
            return Some(ColorCode {
                fg: format!("38;2;{r};{g};{b}"),
                bg: format!("48;2;{r};{g};{b}"),
            });
        }
        return None;
    }
    if let Ok(n) = w.parse::<i32>() {
        if n == -1 {
            return normal_code();
        }
        if (0..8).contains(&n) {
            return Some(ColorCode {
                fg: (30 + n).to_string(),
                bg: (40 + n).to_string(),
            });
        }
        if (0..256).contains(&n) {
            return Some(ColorCode {
                fg: format!("38;5;{n}"),
                bg: format!("48;5;{n}"),
            });
        }
    }
    None
}

/// `normal` occupies a color slot without emitting a code.
fn normal_code() -> Option<ColorCode> {
    Some(ColorCode {
        fg: String::new(),
        bg: String::new(),
    })
}

fn attr_code(w: &str) -> Option<u32> {
    Some(match w {
        "bold" => 1,
        "dim" => 2,
        "italic" => 3,
        "ul" => 4,
        "blink" => 5,
        "reverse" => 7,
        "strike" => 9,
        _ => return None,
    })
}

type ValueMatch = Box<dyn Fn(Option<&str>) -> bool>;

fn value_matcher(pat: &str, fixed: bool) -> ValueMatch {
    if fixed {
        let p = pat.to_string();
        return Box::new(move |v| v.unwrap_or("") == p);
    }
    let (neg, body) = match pat.strip_prefix('!') {
        Some(b) => (true, b),
        None => (false, pat),
    };
    let re = crate::cmd::bre::Bre::with_mode(body, false, true);
    Box::new(move |v| re.is_match(v.unwrap_or("").as_bytes()) != neg)
}

fn emit_prefix(out: &mut Vec<u8>, e: &Ent, o: &Out) {
    let term = if o.null { b'\0' } else { b'\t' };
    if o.show_scope {
        out.extend_from_slice(e.scope.as_bytes());
        out.push(term);
    }
    if o.show_origin {
        out.extend_from_slice(e.origin.as_bytes());
        out.push(term);
    }
}

fn flush(buf: &[u8]) -> CliResult<()> {
    let mut so = std::io::stdout().lock();
    so.write_all(buf)?;
    so.flush()?;
    Ok(())
}

fn get(
    entries: &[Ent],
    action: &str,
    pos: &[String],
    ctx: &Ctx,
    o: &Out,
    default: Option<&str>,
) -> CliResult<()> {
    let vm = pos.get(1).map(|p| value_matcher(p, ctx.fixed));
    let mut buf = Vec::new();
    let mut hits: Vec<&Ent> = Vec::new();
    if action == "get-regexp" {
        let re = crate::cmd::bre::Bre::with_mode(&pos[0], true, true);
        for e in entries {
            if re.is_match(e.key.as_bytes()) && vm.as_ref().is_none_or(|m| m(e.value.as_deref())) {
                hits.push(e);
            }
        }
    } else {
        // Lookups report every malformed key with status 1.
        let k = parse_key(&pos[0]).map_err(|e| match e {
            CliError::Exit(_, m) => CliError::Exit(1, m),
            other => other,
        })?;
        for e in entries {
            if e.key == k.canon && vm.as_ref().is_none_or(|m| m(e.value.as_deref())) {
                hits.push(e);
            }
        }
        if action == "get" && hits.len() > 1 {
            hits = vec![hits[hits.len() - 1]];
        }
    }
    let eol = if o.null { b'\0' } else { b'\n' };
    for e in &hits {
        emit_prefix(&mut buf, e, o);
        if action == "get-regexp" {
            buf.extend_from_slice(e.key.as_bytes());
            if !o.name_only && (e.value.is_some() || ctx.ty.is_some()) {
                buf.push(if o.null { b'\n' } else { b' ' });
                let v = typed_out(ctx.ty, &e.key, e.value.as_deref(), &e.origin)?;
                buf.extend_from_slice(v.as_bytes());
            }
        } else {
            let v = typed_out(ctx.ty, &e.key, e.value.as_deref(), &e.origin)?;
            buf.extend_from_slice(v.as_bytes());
        }
        buf.push(eol);
    }
    if hits.is_empty() {
        if let Some(d) = default {
            let key = canonical_lookup(&pos[0]);
            let v = typed_out(ctx.ty, &key, Some(d), "command line:")?;
            buf.extend_from_slice(v.as_bytes());
            buf.push(eol);
            return flush(&buf);
        }
        return Err(CliError::silent(1));
    }
    flush(&buf)
}

fn list(entries: &[Ent], o: &Out) -> CliResult<()> {
    let mut buf = Vec::new();
    for e in entries {
        emit_prefix(&mut buf, e, o);
        buf.extend_from_slice(e.key.as_bytes());
        if !o.name_only {
            if let Some(v) = &e.value {
                buf.push(if o.null { b'\n' } else { b'=' });
                buf.extend_from_slice(v.as_bytes());
            }
        }
        buf.push(if o.null { b'\0' } else { b'\n' });
    }
    flush(&buf)
}

fn colorbool(entries: &[Ent], pos: &[String]) -> CliResult<()> {
    let get = |k: &str| {
        let k = canonical_lookup(k);
        entries
            .iter()
            .rev()
            .find(|e| e.key == k)
            .map(|e| e.value.clone().unwrap_or_else(|| "true".into()))
    };
    let raw = get(&pos[0])
        .or_else(|| get("color.ui"))
        .unwrap_or_else(|| "auto".into());
    let tty = match pos.get(1) {
        Some(v) => bool_value(Some(v)).unwrap_or(false),
        None => stdout_is_tty(),
    };
    let want = match raw.to_ascii_lowercase().as_str() {
        "always" => true,
        "never" => false,
        "auto" => tty,
        // Any other truth value means `auto` (git_config_colorbool).
        other => bool_value(Some(other)).unwrap_or(false) && tty,
    };
    if pos.len() == 2 {
        println!("{want}");
        return Ok(());
    }
    if want {
        Ok(())
    } else {
        Err(CliError::silent(1))
    }
}

fn stdout_is_tty() -> bool {
    use std::io::IsTerminal;
    std::io::stdout().is_terminal()
}

/// urlmatch (git-config `http.<url>.*`): best match per variable.
fn get_urlmatch(entries: &[Ent], name: &str, url: &str, ctx: &Ctx) -> CliResult<()> {
    let (section, var) = match name.split_once('.') {
        Some((s, v)) => (s.to_ascii_lowercase(), Some(v.to_ascii_lowercase())),
        None => (name.to_ascii_lowercase(), None),
    };
    let target = UrlParts::parse(url)
        .ok_or_else(|| CliError::fatal("invalid URL scheme name or missing '://' suffix"))?;
    // var -> (score, value)
    let mut best: Vec<(String, (usize, usize), Option<String>)> = Vec::new();
    for e in entries {
        let Some(rest) = e.key.strip_prefix(&format!("{section}.")) else {
            continue;
        };
        let (pattern, v) = match rest.rfind('.') {
            Some(i) => (Some(&rest[..i]), &rest[i + 1..]),
            None => (None, rest),
        };
        if var.as_deref().is_some_and(|w| w != v) {
            continue;
        }
        let score = match pattern {
            None => (0, 0),
            Some(p) => match UrlParts::parse(p).and_then(|pp| pp.matches(&target)) {
                Some(s) => s,
                None => continue,
            },
        };
        match best.iter_mut().find(|(k, _, _)| k == v) {
            Some(slot) => {
                if score >= slot.1 {
                    slot.1 = score;
                    slot.2 = e.value.clone();
                }
            }
            None => best.push((v.to_string(), score, e.value.clone())),
        }
    }
    if best.is_empty() {
        return Err(CliError::silent(1));
    }
    best.sort_by(|a, b| a.0.cmp(&b.0));
    let mut buf = String::new();
    for (v, _, val) in &best {
        let key = format!("{section}.{v}");
        let shown_v = typed_out(ctx.ty, &key, val.as_deref(), "command line:")?;
        if var.is_some() {
            buf.push_str(&shown_v);
        } else {
            buf.push_str(&key);
            if val.is_some() || ctx.ty.is_some() {
                buf.push(' ');
                buf.push_str(&shown_v);
            }
        }
        buf.push('\n');
    }
    print!("{buf}");
    Ok(())
}

struct UrlParts {
    scheme: String,
    user: Option<String>,
    host: String,
    port: Option<String>,
    path: String,
}

impl UrlParts {
    fn parse(u: &str) -> Option<Self> {
        let (scheme, rest) = u.split_once("://")?;
        let (auth, path) = match rest.find('/') {
            Some(i) => (&rest[..i], &rest[i..]),
            None => (rest, "/"),
        };
        let (user, hostport) = match auth.rsplit_once('@') {
            Some((u, h)) => (Some(u.split(':').next().unwrap_or(u).to_string()), h),
            None => (None, auth),
        };
        let (host, port) = match hostport.rsplit_once(':') {
            Some((h, p)) if p.bytes().all(|b| b.is_ascii_digit()) => (h, Some(p.to_string())),
            _ => (hostport, None),
        };
        let scheme = scheme.to_ascii_lowercase();
        let default_port = match scheme.as_str() {
            "http" => Some("80"),
            "https" => Some("443"),
            _ => None,
        };
        let port = port.filter(|p| Some(p.as_str()) != default_port);
        Some(Self {
            scheme,
            user,
            host: host.to_ascii_lowercase(),
            port,
            path: path.to_string(),
        })
    }

    /// (path length, user match) score when `self` (a config pattern) matches `t`.
    fn matches(&self, t: &UrlParts) -> Option<(usize, usize)> {
        if self.scheme != t.scheme || self.port != t.port {
            return None;
        }
        let ph: Vec<&str> = self.host.split('.').collect();
        let th: Vec<&str> = t.host.split('.').collect();
        if ph.len() != th.len() || ph.iter().zip(&th).any(|(p, h)| *p != "*" && p != h) {
            return None;
        }
        if let Some(u) = &self.user {
            if t.user.as_deref() != Some(u.as_str()) {
                return None;
            }
        }
        let pp = self.path.trim_end_matches('/');
        let tp = t.path.as_str();
        let ok = pp.is_empty()
            || tp == pp
            || (tp.starts_with(pp) && tp.as_bytes().get(pp.len()) == Some(&b'/'));
        if !ok {
            return None;
        }
        Some((pp.len() + 1, usize::from(self.user.is_some())))
    }
}

fn editor() -> String {
    if let Ok(e) = std::env::var("GIT_EDITOR") {
        return e;
    }
    let cwd = std::env::current_dir().unwrap_or_default();
    if let Some(e) = arks_config::ConfigSet::load_discovered(&cwd).get("core.editor") {
        return e;
    }
    for v in ["VISUAL", "EDITOR"] {
        if let Ok(e) = std::env::var(v) {
            if !e.is_empty() {
                return e;
            }
        }
    }
    "vi".into()
}

fn edit(loc: &Loc) -> CliResult<()> {
    let path = match loc {
        Loc::File { path, .. } => path.clone(),
        Loc::Blob(_) => return Err(CliError::fatal("editing blobs is not supported")),
        Loc::Merged => {
            let gd = git_dir().ok_or_else(|| CliError::fatal("not in a git directory"))?;
            common_dir(&gd).join("config")
        }
    };
    if !path.exists() {
        fs::write(&path, b"")?;
    }
    let ed = editor();
    if ed == ":" {
        return Ok(());
    }
    let st = std::process::Command::new("sh")
        .arg("-c")
        .arg(format!("{ed} \"$@\""))
        .arg(&ed)
        .arg(&path)
        .status()
        .map_err(|e| CliError::fatal(format!("unable to start editor '{ed}': {e}")))?;
    if !st.success() {
        return Err(CliError::fatal(format!(
            "there was a problem with the editor '{ed}'"
        )));
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// File editing.

/// One parsed span of a config file (byte ranges into the text).
#[derive(Debug, Clone)]
enum Span {
    Header {
        start: usize,
        /// End of the header token (an entry may follow on the same line).
        end: usize,
        line_end: usize,
        section: String,
        sub: Option<String>,
        /// Legacy `[section.sub]` form (subsection compared case-insensitively).
        legacy: bool,
    },
    Entry {
        start: usize,
        end: usize,
        canon: String,
        value: Option<String>,
        /// Index of the owning header span.
        header: Option<usize>,
    },
}

fn line_end(text: &str, from: usize) -> usize {
    text[from..]
        .find('\n')
        .map(|i| from + i + 1)
        .unwrap_or(text.len())
}

/// Parse `text` into spans (validated by `arks_config::parse_config_text` first).
fn spans(text: &str) -> Vec<Span> {
    let mut out = Vec::new();
    let mut header: Option<usize> = None;
    let mut cur_canon_prefix = String::new();
    let mut pos = 0;
    while pos < text.len() {
        let le = line_end(text, pos);
        let line = &text[pos..le];
        let trimmed_off = line.len() - line.trim_start().len();
        let mut rest_start = pos + trimmed_off;
        let rest = &text[rest_start..le];
        if rest.trim().is_empty() || rest.starts_with('#') || rest.starts_with(';') {
            pos = le;
            continue;
        }
        if rest.starts_with('[') {
            if let Some((section, sub, legacy, consumed)) = parse_header(rest) {
                out.push(Span::Header {
                    start: pos,
                    end: rest_start + consumed,
                    line_end: le,
                    section: section.clone(),
                    sub: sub.clone(),
                    legacy,
                });
                header = Some(out.len() - 1);
                cur_canon_prefix = match &sub {
                    Some(s) => format!("{}.{s}", section.to_ascii_lowercase()),
                    None => section.to_ascii_lowercase(),
                };
                rest_start += consumed;
                let after = text[rest_start..le].trim_start();
                if after.is_empty() || after.starts_with('#') || after.starts_with(';') {
                    pos = le;
                    continue;
                }
                rest_start = le - text[rest_start..le].trim_start().len();
            } else {
                pos = le;
                continue;
            }
        }
        // Entry: name [= value], value may continue over lines ending in `\`.
        let body = &text[rest_start..le];
        let name_end = body
            .find(|c: char| !(c.is_ascii_alphanumeric() || c == '-'))
            .unwrap_or(body.len());
        let name = &body[..name_end];
        let mut end = le;
        let after = body[name_end..].trim_start();
        let value = if let Some(v) = after.strip_prefix('=') {
            let mut logical = v.trim_end_matches(['\n', '\r']).to_string();
            while ends_with_backslash(&logical) && end < text.len() {
                logical.pop();
                let ne = line_end(text, end);
                logical.push_str(text[end..ne].trim_end_matches(['\n', '\r']));
                end = ne;
            }
            Some(decode_value(&logical))
        } else {
            None
        };
        out.push(Span::Entry {
            start: pos,
            end,
            canon: format!("{cur_canon_prefix}.{}", name.to_ascii_lowercase()),
            value,
            header,
        });
        pos = end;
    }
    out
}

fn ends_with_backslash(s: &str) -> bool {
    s.bytes().rev().take_while(|b| *b == b'\\').count() % 2 == 1
}

/// Parse `[section]`, `[section "sub"]` or `[section.sub]`.
/// Returns (section, sub, legacy, bytes consumed).
fn parse_header(s: &str) -> Option<(String, Option<String>, bool, usize)> {
    let body = &s[1..];
    let name_end = body.find(|c: char| !(c.is_ascii_alphanumeric() || c == '-' || c == '.'))?;
    let name = &body[..name_end];
    let rest = &body[name_end..];
    if rest.starts_with(']') {
        if let Some((sec, sub)) = name.split_once('.') {
            return Some((
                sec.to_string(),
                Some(sub.to_ascii_lowercase()),
                true,
                1 + name_end + 1,
            ));
        }
        return Some((name.to_string(), None, false, 1 + name_end + 1));
    }
    let ws = rest.len() - rest.trim_start().len();
    let q = rest.trim_start().strip_prefix('"')?;
    let mut sub = String::new();
    let mut chars = q.char_indices();
    while let Some((i, c)) = chars.next() {
        match c {
            '\\' => {
                if let Some((_, n)) = chars.next() {
                    sub.push(n);
                }
            }
            '"' => {
                if q[i + 1..].starts_with(']') {
                    let consumed = 1 + name_end + ws + 1 + i + 2;
                    return Some((name.to_string(), Some(sub), false, consumed));
                }
                return None;
            }
            c => sub.push(c),
        }
    }
    None
}

fn decode_value(src: &str) -> String {
    let mut out = String::new();
    let mut in_quote = false;
    let mut pending = String::new();
    let mut started = false;
    let mut chars = src.chars();
    while let Some(c) = chars.next() {
        match c {
            '"' => {
                out.push_str(&pending);
                pending.clear();
                in_quote = !in_quote;
                started = true;
            }
            '\\' => {
                out.push_str(&pending);
                pending.clear();
                started = true;
                match chars.next() {
                    Some('n') => out.push('\n'),
                    Some('t') => out.push('\t'),
                    Some('b') => {
                        out.pop();
                    }
                    Some(o) => out.push(o),
                    None => {}
                }
            }
            '#' | ';' if !in_quote => break,
            c if c.is_whitespace() && !in_quote => {
                if started {
                    pending.push(c);
                }
            }
            c => {
                out.push_str(&pending);
                pending.clear();
                out.push(c);
                started = true;
            }
        }
    }
    out
}

/// Encode a value for writing (`write_pair`).
fn encode_value(v: &str) -> String {
    let quote = v.starts_with(|c: char| c.is_ascii_whitespace())
        || v.ends_with(|c: char| c.is_ascii_whitespace())
        || v.contains(';')
        || v.contains('#');
    let mut s = String::new();
    if quote {
        s.push('"');
    }
    for c in v.chars() {
        match c {
            '\n' => s.push_str("\\n"),
            '\t' => s.push_str("\\t"),
            '"' => s.push_str("\\\""),
            '\\' => s.push_str("\\\\"),
            c => s.push(c),
        }
    }
    if quote {
        s.push('"');
    }
    s
}

fn header_text(section: &str, sub: Option<&str>) -> String {
    match sub {
        Some(s) => {
            let esc = s.replace('\\', "\\\\").replace('"', "\\\"");
            format!("[{section} \"{esc}\"]\n")
        }
        None => format!("[{section}]\n"),
    }
}

fn header_matches(sp: &Span, k: &Key) -> bool {
    let Span::Header {
        section,
        sub,
        legacy,
        ..
    } = sp
    else {
        return false;
    };
    if !section.eq_ignore_ascii_case(&k.section) {
        return false;
    }
    match (sub, &k.sub) {
        (None, None) => true,
        (Some(a), Some(b)) => {
            if *legacy {
                a.eq_ignore_ascii_case(b)
            } else {
                a == b
            }
        }
        _ => false,
    }
}

struct Target {
    path: PathBuf,
    lock: PathBuf,
}

fn write_target(loc: &Loc) -> CliResult<Target> {
    let path = match loc {
        Loc::File { path, .. } => path.clone(),
        Loc::Blob(_) => return Err(CliError::fatal("writing config blobs is not supported")),
        Loc::Merged => {
            let gd = git_dir().ok_or_else(|| CliError::fatal("not in a git directory"))?;
            common_dir(&gd).join("config")
        }
    };
    let mut l = path.clone().into_os_string();
    l.push(".lock");
    let lock = PathBuf::from(l);
    // WT-006 (M20 Cx-4): a held config.worktree.lock refuses worktree config writes.
    if let Loc::File {
        scope: "worktree" | "local",
        ..
    } = loc
    {
        if let Some(gd) = git_dir() {
            let wt_lock = gd.join("config.worktree.lock");
            if wt_lock.is_file() {
                return Err(CliError::Exit(
                    255,
                    format!(
                        "error: could not lock config file {}: File exists (WT-006 — config.worktree.lock held; concurrent worktree config edits refuse)",
                        shown(&gd.join("config.worktree"))
                    ),
                ));
            }
        }
    }
    Ok(Target { path, lock })
}

fn commit_text(t: &Target, text: &str) -> CliResult<()> {
    if let Some(parent) = t.path.parent() {
        if !parent.as_os_str().is_empty() && !parent.exists() {
            fs::create_dir_all(parent)?;
        }
    }
    // Never replace a device or directory (for example GIT_CONFIG_GLOBAL=/dev/null).
    if t.path.exists() && !t.path.is_file() {
        return Err(CliError::Exit(
            255,
            format!(
                "error: could not lock config file {}: not a regular file",
                shown(&t.path)
            ),
        ));
    }
    let mut f = match fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&t.lock)
    {
        Ok(f) => f,
        Err(e) => {
            let why = if e.kind() == std::io::ErrorKind::AlreadyExists {
                "File exists"
            } else {
                io_reason(&e)
            };
            return Err(CliError::Exit(
                255,
                format!(
                    "error: could not lock config file {}: {why}",
                    shown(&t.path)
                ),
            ));
        }
    };
    let res = f.write_all(text.as_bytes()).and_then(|_| f.flush());
    drop(f);
    if let Err(e) = res {
        let _ = fs::remove_file(&t.lock);
        return Err(e.into());
    }
    fs::rename(&t.lock, &t.path).inspect_err(|_| {
        let _ = fs::remove_file(&t.lock);
    })?;
    Ok(())
}

fn read_target(t: &Target) -> CliResult<String> {
    match fs::read(&t.path) {
        Ok(b) => {
            let text = String::from_utf8_lossy(&b).into_owned();
            if let Err(e) = arks_config::parse_config_text(&text) {
                return Err(CliError::Exit(
                    3,
                    format!(
                        "error: bad config line {} in file {}",
                        e.line,
                        shown(&t.path)
                    ),
                ));
            }
            Ok(text)
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(String::new()),
        Err(e) => Err(e.into()),
    }
}

/// Remove byte ranges and drop section headers left with no content.
fn remove_ranges(text: &str, sp: &[Span], remove: &[usize]) -> String {
    let mut drop: Vec<(usize, usize)> = Vec::new();
    for &i in remove {
        if let Span::Entry { start, end, .. } = sp[i] {
            drop.push((start, end));
        }
    }
    // Headers whose section body consisted only of removed entries.
    for (hi, h) in sp.iter().enumerate() {
        let Span::Header {
            start,
            end,
            line_end: hle,
            ..
        } = *h
        else {
            continue;
        };
        if hle != end && !text[end..hle].trim().is_empty() {
            // An entry shares the header line; keep unless that entry is removed.
            let same_line_removed = remove.iter().any(|&i| {
                matches!(sp[i], Span::Entry { start: s, header: Some(h2), .. } if h2 == hi && s == start)
            });
            if !same_line_removed {
                continue;
            }
        }
        let next_header = sp[hi + 1..]
            .iter()
            .find_map(|s| match s {
                Span::Header { start, .. } => Some(*start),
                _ => None,
            })
            .unwrap_or(text.len());
        let owned: Vec<usize> = sp
            .iter()
            .enumerate()
            .filter(|(_, s)| matches!(s, Span::Entry { header: Some(h2), .. } if *h2 == hi))
            .map(|(i, _)| i)
            .collect();
        if owned.is_empty() || !owned.iter().all(|i| remove.contains(i)) {
            continue;
        }
        // Everything between the header line and the next header must be removed entries.
        let mut cursor = hle;
        let mut clean = true;
        for &i in &owned {
            if let Span::Entry {
                start: s, end: e, ..
            } = sp[i]
            {
                if s > cursor {
                    clean = false;
                    break;
                }
                cursor = cursor.max(e);
            }
        }
        if clean && cursor == next_header {
            drop.push((start, hle));
        }
    }
    drop.sort_unstable();
    let mut out = String::new();
    let mut at = 0;
    for (s, e) in drop {
        if s < at {
            at = at.max(e);
            continue;
        }
        out.push_str(&text[at..s]);
        at = e;
    }
    out.push_str(&text[at..]);
    out
}

fn new_entry_line(k: &Key, value: &str) -> String {
    format!("\t{} = {}\n", k.name, encode_value(value))
}

/// Text with a new `k = value` entry added at git's insertion point.
fn insert_entry(text: &str, sp: &[Span], k: &Key, value: &str) -> String {
    let line = new_entry_line(k, value);
    // Last matching section; insert after its last entry (or its header).
    let mut at: Option<usize> = None;
    for (i, s) in sp.iter().enumerate() {
        if header_matches(s, k) {
            let Span::Header { line_end: hle, .. } = s else {
                continue;
            };
            let mut pos = *hle;
            for e in &sp[i + 1..] {
                match e {
                    Span::Entry { end, .. } => pos = *end,
                    Span::Header { .. } => break,
                }
            }
            at = Some(pos);
        }
    }
    let mut out = String::with_capacity(text.len() + line.len() + 32);
    match at {
        Some(pos) => {
            out.push_str(&text[..pos]);
            if !out.is_empty() && !out.ends_with('\n') {
                out.push('\n');
            }
            out.push_str(&line);
            out.push_str(&text[pos..]);
        }
        None => {
            out.push_str(text);
            if !out.is_empty() && !out.ends_with('\n') {
                out.push('\n');
            }
            out.push_str(&header_text(&k.section, k.sub.as_deref()));
            out.push_str(&line);
        }
    }
    out
}

fn write_action(loc: &Loc, action: &str, pos: &[String], ctx: &Ctx) -> CliResult<()> {
    let t = write_target(loc)?;
    let text = read_target(&t)?;
    let sp = spans(&text);
    match action {
        "rename-section" | "remove-section" => {
            return section_edit(&t, &text, &sp, action, pos);
        }
        _ => {}
    }
    let k = parse_key(&pos[0])?;
    let matching: Vec<usize> = sp
        .iter()
        .enumerate()
        .filter(|(_, s)| matches!(s, Span::Entry { canon, .. } if *canon == k.canon))
        .map(|(i, _)| i)
        .collect();
    let pat_idx = match action {
        "unset" | "unset-all" => 1,
        "replace-all" | "set-pattern" => 2,
        _ => usize::MAX,
    };
    let vm = pos.get(pat_idx).map(|p| value_matcher(p, ctx.fixed));
    let selected: Vec<usize> = matching
        .iter()
        .copied()
        .filter(|&i| match (&sp[i], &vm) {
            (Span::Entry { value, .. }, Some(m)) => m(value.as_deref()),
            _ => true,
        })
        .collect();
    let new_text = match action {
        "unset" | "unset-all" => {
            if selected.is_empty() {
                return Err(CliError::silent(5));
            }
            if action == "unset" && selected.len() > 1 {
                return Err(CliError::Exit(
                    5,
                    format!("warning: {} has multiple values", pos[0]),
                ));
            }
            remove_ranges(&text, &sp, &selected)
        }
        "add" => {
            let v = typed_in(ctx.ty, &k.canon, &pos[1])?;
            insert_entry(&text, &sp, &k, &v)
        }
        "set" | "set-pattern" | "replace-all" => {
            let v = typed_in(ctx.ty, &k.canon, &pos[1])?;
            if action != "replace-all" && selected.len() > 1 {
                return Err(CliError::Exit(
                    5,
                    format!(
                        "warning: {0} has multiple values\nerror: cannot overwrite multiple values with a single value\n       Use a regexp, --add or --replace-all to change {0}.",
                        pos[0]
                    ),
                ));
            }
            match selected.first() {
                None => insert_entry(&text, &sp, &k, &v),
                Some(&first) => {
                    let Span::Entry { start, end, .. } = sp[first] else {
                        unreachable!()
                    };
                    // Keep a header that shares the entry's line.
                    let shared = sp.iter().find_map(|s| match s {
                        Span::Header {
                            start: hs, end: he, ..
                        } if *hs == start => Some(*he),
                        _ => None,
                    });
                    let mut out = String::new();
                    match shared {
                        Some(he) => {
                            out.push_str(&text[..he]);
                            out.push('\n');
                        }
                        None => out.push_str(&text[..start]),
                    }
                    out.push_str(&new_entry_line(&k, &v));
                    let rest_spans: Vec<usize> = selected[1..].to_vec();
                    let tail = &text[end..];
                    if rest_spans.is_empty() {
                        out.push_str(tail);
                        out
                    } else {
                        // replace-all: drop the other matches.
                        let mut t2 = out;
                        let base = t2.len();
                        t2.push_str(tail);
                        let sp2 = spans(&t2);
                        let drop: Vec<usize> = sp2
                            .iter()
                            .enumerate()
                            .filter(|(_, s)| match s {
                                Span::Entry {
                                    start,
                                    canon,
                                    value,
                                    ..
                                } => {
                                    *start >= base
                                        && *canon == k.canon
                                        && vm.as_ref().is_none_or(|m| m(value.as_deref()))
                                }
                                _ => false,
                            })
                            .map(|(i, _)| i)
                            .collect();
                        remove_ranges(&t2, &sp2, &drop)
                    }
                }
            }
        }
        _ => return Err(usage("")),
    };
    commit_text(&t, &new_text)
}

fn section_edit(
    t: &Target,
    text: &str,
    sp: &[Span],
    action: &str,
    pos: &[String],
) -> CliResult<()> {
    let old = &pos[0];
    let (osec, osub) = match old.split_once('.') {
        Some((a, b)) => (a.to_string(), Some(b.to_string())),
        None => (old.clone(), None),
    };
    let matches = |s: &Span| -> bool {
        let Span::Header {
            section,
            sub,
            legacy,
            ..
        } = s
        else {
            return false;
        };
        section.eq_ignore_ascii_case(&osec)
            && match (sub, &osub) {
                (None, None) => true,
                (Some(a), Some(b)) => {
                    if *legacy {
                        a.eq_ignore_ascii_case(b)
                    } else {
                        a == b
                    }
                }
                _ => false,
            }
    };
    let new_hdr = if action == "rename-section" {
        let new = &pos[1];
        let (nsec, nsub) = match new.split_once('.') {
            Some((a, b)) => (a, Some(b)),
            None => (new.as_str(), None),
        };
        if nsec.is_empty()
            || !nsec.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-')
            || nsub.is_some_and(|s| s.contains('\n'))
        {
            return Err(CliError::Exit(
                255,
                format!("error: invalid section name: {new}"),
            ));
        }
        Some(header_text(nsec, nsub))
    } else {
        None
    };
    let mut out = String::new();
    let mut at = 0;
    let mut found = false;
    for (i, s) in sp.iter().enumerate() {
        if !matches(s) {
            continue;
        }
        let Span::Header {
            start,
            end,
            line_end: hle,
            ..
        } = *s
        else {
            continue;
        };
        found = true;
        out.push_str(&text[at..start]);
        match &new_hdr {
            Some(h) => {
                let rest = &text[end..hle];
                if rest.trim().is_empty() {
                    out.push_str(h);
                } else {
                    out.push_str(h.trim_end_matches('\n'));
                    out.push_str(rest);
                }
                at = hle;
            }
            None => {
                // Drop the section through the next header.
                let next = sp[i + 1..]
                    .iter()
                    .find_map(|s| match s {
                        Span::Header { start, .. } => Some(*start),
                        _ => None,
                    })
                    .unwrap_or(text.len());
                at = next;
            }
        }
    }
    if !found {
        return Err(CliError::fatal(format!("no such section: {old}")));
    }
    out.push_str(&text[at..]);
    commit_text(t, &out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn colors_match_git_shapes() {
        assert_eq!(parse_color("red").as_deref(), Some("\x1b[31m"));
        assert_eq!(parse_color("bold red ul").as_deref(), Some("\x1b[1;4;31m"));
        assert_eq!(
            parse_color("#ff0000 blue").as_deref(),
            Some("\x1b[38;2;255;0;0;44m")
        );
        assert_eq!(parse_color("bogus"), None);
    }

    #[test]
    fn encode_quotes_like_git() {
        assert_eq!(encode_value(" lead"), "\" lead\"");
        assert_eq!(encode_value("a\\nb;c"), "\"a\\\\nb;c\"");
        assert_eq!(encode_value("plain"), "plain");
    }

    #[test]
    fn unset_drops_emptied_section_only() {
        let text = "[a]\n\tk = 1\n[b]\n\tk = 2\n\n[c]\n";
        let sp = spans(text);
        let idx = sp
            .iter()
            .position(|s| matches!(s, Span::Entry { canon, .. } if canon == "a.k"))
            .unwrap();
        assert_eq!(remove_ranges(text, &sp, &[idx]), "[b]\n\tk = 2\n\n[c]\n");
        let idx = sp
            .iter()
            .position(|s| matches!(s, Span::Entry { canon, .. } if canon == "b.k"))
            .unwrap();
        assert_eq!(
            remove_ranges(text, &sp, &[idx]),
            "[a]\n\tk = 1\n[b]\n\n[c]\n"
        );
    }
}
