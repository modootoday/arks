//! `arks repo read-dir [<path>]` and `arks repo read-file <path>` (gh preview leaves).
//!
//! Peers: <https://cli.github.com/manual/gh_repo_read-dir>,
//! <https://cli.github.com/manual/gh_repo_read-file>.
//! REST: `GET /repos/{o}/{r}/contents/{path}?ref=`
//! (<https://docs.github.com/en/rest/repos/contents#get-repository-content>).
//!
//! Partial: `--json` fields `mode` / `modeOctal` (read-dir) are refused because
//! the contents API does not return a file mode; `--jq` / `--template` refused.
//! No pager is spawned; stdout always receives raw content.

use super::common::{
    Ctx, call, ctx, has_flag, json_fields, positionals, print_json, select, str_of, usage,
};
use base64::Engine as _;
use serde_json::{Map, Value};
use std::fs;
use std::io::Write;
use std::path::Path;
use std::process::ExitCode;

const VALUE_FLAGS: &[&str] = &[
    "-R",
    "--repo",
    "--json",
    "-q",
    "--jq",
    "-t",
    "--template",
    "--ref",
    "-o",
    "--output",
];
const DIR_FIELDS: &[&str] = &[
    "gitSHA",
    "gitType",
    "mode",
    "modeOctal",
    "name",
    "nameRaw",
    "path",
    "pathRaw",
    "size",
    "submodule",
    "type",
];
const FILE_FIELDS: &[&str] = &[
    "content",
    "downloadUrl",
    "encoding",
    "gitSHA",
    "gitUrl",
    "htmlUrl",
    "name",
    "path",
    "size",
    "type",
    "url",
];

fn contents_path(c: &Ctx, rel: &str, git_ref: Option<&str>) -> String {
    let rel = rel.trim_matches('/');
    let mut p = format!("/repos/{}/{}/contents", c.owner, c.repo);
    if !rel.is_empty() {
        p.push('/');
        p.push_str(rel);
    }
    if let Some(r) = git_ref {
        p.push_str("?ref=");
        p.push_str(r);
    }
    p
}

fn dir_entry_obj(v: &Value) -> Map<String, Value> {
    let kind = str_of(v, "type");
    let git_type = match kind {
        "dir" => "tree",
        "submodule" => "commit",
        _ => "blob",
    };
    let mut m = Map::new();
    m.insert(
        "gitSHA".into(),
        v.get("sha").cloned().unwrap_or(Value::Null),
    );
    m.insert("gitType".into(), Value::from(git_type));
    m.insert("name".into(), v.get("name").cloned().unwrap_or(Value::Null));
    m.insert(
        "nameRaw".into(),
        v.get("name").cloned().unwrap_or(Value::Null),
    );
    m.insert("path".into(), v.get("path").cloned().unwrap_or(Value::Null));
    m.insert(
        "pathRaw".into(),
        v.get("path").cloned().unwrap_or(Value::Null),
    );
    m.insert("size".into(), v.get("size").cloned().unwrap_or(Value::Null));
    m.insert("submodule".into(), Value::from(kind == "submodule"));
    m.insert("type".into(), Value::from(kind));
    m
}

pub fn read_dir(args: &[String]) -> ExitCode {
    let label = "repo read-dir";
    let fields = match json_fields(args, label, DIR_FIELDS) {
        Ok(f) => f,
        Err(c) => return c,
    };
    if let Some(f) = &fields
        && f.iter().any(|x| x == "mode" || x == "modeOctal")
    {
        return usage(
            label,
            "JSON fields mode/modeOctal are not available from the contents API (Partial)",
        );
    }
    let pos = positionals(args, VALUE_FLAGS);
    let rel = pos.first().map(String::as_str).unwrap_or("");
    let git_ref = crate::flag_value(args, &["--ref"]);
    let c = match ctx(args, label) {
        Ok(c) => c,
        Err(code) => return code,
    };
    let v = match call(
        &c.base,
        &c.tok,
        "GET",
        &contents_path(&c, rel, git_ref),
        None,
        label,
    ) {
        Ok(v) => v,
        Err(code) => return code,
    };
    let Value::Array(items) = v else {
        return usage(label, &format!("`{rel}` is not a directory"));
    };
    if let Some(f) = fields {
        let out: Vec<Value> = items
            .iter()
            .map(|e| select(&dir_entry_obj(e), &f))
            .collect();
        print_json(&Value::Array(out));
    } else {
        for e in &items {
            let suffix = if str_of(e, "type") == "dir" { "/" } else { "" };
            println!("{}{suffix}", str_of(e, "name"));
        }
    }
    ExitCode::SUCCESS
}

fn file_obj(v: &Value) -> Map<String, Value> {
    let g = |k: &str| v.get(k).cloned().unwrap_or(Value::Null);
    let mut m = Map::new();
    m.insert("content".into(), g("content"));
    m.insert("downloadUrl".into(), g("download_url"));
    m.insert("encoding".into(), g("encoding"));
    m.insert("gitSHA".into(), g("sha"));
    m.insert("gitUrl".into(), g("git_url"));
    m.insert("htmlUrl".into(), g("html_url"));
    m.insert("name".into(), g("name"));
    m.insert("path".into(), g("path"));
    m.insert("size".into(), g("size"));
    m.insert("type".into(), g("type"));
    m.insert("url".into(), g("url"));
    m
}

/// Decode a contents-API `content` field (base64, possibly line-wrapped).
pub fn decode_content(v: &Value) -> Result<Vec<u8>, String> {
    if str_of(v, "encoding") != "base64" {
        return Err(format!(
            "unsupported content encoding `{}`",
            str_of(v, "encoding")
        ));
    }
    let raw: String = str_of(v, "content")
        .chars()
        .filter(|c| !c.is_whitespace())
        .collect();
    base64::engine::general_purpose::STANDARD
        .decode(raw)
        .map_err(|e| format!("base64 decode: {e}"))
}

pub fn read_file(args: &[String]) -> ExitCode {
    let label = "repo read-file";
    let fields = match json_fields(args, label, FILE_FIELDS) {
        Ok(f) => f,
        Err(c) => return c,
    };
    let pos = positionals(args, VALUE_FLAGS);
    let Some(rel) = pos.first() else {
        return usage(label, "<path> required");
    };
    let git_ref = crate::flag_value(args, &["--ref"]);
    let c = match ctx(args, label) {
        Ok(c) => c,
        Err(code) => return code,
    };
    let v = match call(
        &c.base,
        &c.tok,
        "GET",
        &contents_path(&c, rel, git_ref),
        None,
        label,
    ) {
        Ok(v) => v,
        Err(code) => return code,
    };
    if v.is_array() || str_of(&v, "type") == "dir" {
        return usage(
            label,
            &format!("`{rel}` is a directory; use `arks repo read-dir`"),
        );
    }
    if let Some(f) = fields {
        print_json(&select(&file_obj(&v), &f));
        return ExitCode::SUCCESS;
    }
    let bytes = match decode_content(&v) {
        Ok(b) => b,
        Err(e) => return super::common::fail(label, &e),
    };
    if let Some(out) = crate::flag_value(args, &["--output", "-o"]) {
        let p = Path::new(out);
        if p.exists() && !has_flag(args, &["--clobber"]) {
            return super::common::fail(
                label,
                &format!("{out} already exists (use --clobber to overwrite)"),
            );
        }
        if let Some(parent) = p.parent().filter(|d| !d.as_os_str().is_empty()) {
            let _ = fs::create_dir_all(parent);
        }
        return match fs::write(p, &bytes) {
            Ok(()) => ExitCode::SUCCESS,
            Err(e) => super::common::fail(label, &format!("write {out}: {e}")),
        };
    }
    if bytes.contains(&0x1b) && !has_flag(args, &["--allow-escape-sequences"]) {
        return super::common::fail(
            label,
            "file contains terminal escape sequences; use --allow-escape-sequences or --output",
        );
    }
    let mut stdout = std::io::stdout().lock();
    if stdout
        .write_all(&bytes)
        .and_then(|_| stdout.flush())
        .is_err()
    {
        return ExitCode::from(1);
    }
    ExitCode::SUCCESS
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn decodes_wrapped_base64() {
        let v = json!({"encoding": "base64", "content": "aGVs\nbG8=\n"});
        assert_eq!(decode_content(&v).unwrap(), b"hello");
        let bad = json!({"encoding": "none", "content": ""});
        assert!(decode_content(&bad).is_err());
    }
}
