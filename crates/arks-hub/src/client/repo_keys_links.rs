//! `arks repo autolink create|delete|list|view` and
//! `arks repo deploy-key add|delete|list`.
//!
//! Peers: <https://cli.github.com/manual/gh_repo_autolink> (create/delete/list/view),
//! <https://cli.github.com/manual/gh_repo_deploy-key> (add/delete/list).
//! REST: <https://docs.github.com/en/rest/repos/autolinks>,
//! <https://docs.github.com/en/rest/deploy-keys/deploy-keys>.

use super::common::{
    Ctx, call, ctx, has_flag, json_fields, positionals, print_json, select, usage,
};
use serde_json::{Map, Value, json};
use std::fs;
use std::process::ExitCode;

const AUTOLINK_FIELDS: &[&str] = &["id", "isAlphanumeric", "keyPrefix", "urlTemplate"];
const DEPLOY_KEY_FIELDS: &[&str] = &["createdAt", "id", "key", "readOnly", "title"];
const VALUE_FLAGS: &[&str] = &[
    "-R",
    "--repo",
    "--json",
    "-q",
    "--jq",
    "-t",
    "--template",
    "--title",
];

fn autolink_obj(v: &Value) -> Map<String, Value> {
    let mut m = Map::new();
    m.insert("id".into(), v.get("id").cloned().unwrap_or(Value::Null));
    m.insert(
        "isAlphanumeric".into(),
        v.get("is_alphanumeric").cloned().unwrap_or(Value::Null),
    );
    m.insert(
        "keyPrefix".into(),
        v.get("key_prefix").cloned().unwrap_or(Value::Null),
    );
    m.insert(
        "urlTemplate".into(),
        v.get("url_template").cloned().unwrap_or(Value::Null),
    );
    m
}

fn deploy_key_obj(v: &Value) -> Map<String, Value> {
    let mut m = Map::new();
    m.insert(
        "createdAt".into(),
        v.get("created_at").cloned().unwrap_or(Value::Null),
    );
    m.insert("id".into(), v.get("id").cloned().unwrap_or(Value::Null));
    m.insert("key".into(), v.get("key").cloned().unwrap_or(Value::Null));
    m.insert(
        "readOnly".into(),
        v.get("read_only").cloned().unwrap_or(Value::Null),
    );
    m.insert(
        "title".into(),
        v.get("title").cloned().unwrap_or(Value::Null),
    );
    m
}

fn with_ctx(args: &[String], label: &str, f: impl FnOnce(&Ctx) -> ExitCode) -> ExitCode {
    match ctx(args, label) {
        Ok(c) => f(&c),
        Err(code) => code,
    }
}

pub fn autolink(args: &[String]) -> ExitCode {
    let sub = args.first().map(String::as_str).unwrap_or("");
    let rest = args.get(1..).unwrap_or(&[]);
    match sub {
        "" | "-h" | "--help" => {
            eprintln!(
                "arks repo autolink — create|delete|list|view (/repos/{{o}}/{{r}}/autolinks)"
            );
            ExitCode::SUCCESS
        }
        "list" | "ls" => {
            let label = "repo autolink list";
            if has_flag(rest, &["--web", "-w"]) {
                return usage(label, "--web is not implemented (no browser launch)");
            }
            let fields = match json_fields(rest, label, AUTOLINK_FIELDS) {
                Ok(f) => f,
                Err(c) => return c,
            };
            with_ctx(rest, label, |c| {
                let path = format!("/repos/{}/{}/autolinks", c.owner, c.repo);
                match call(&c.base, &c.tok, "GET", &path, None, label) {
                    Ok(Value::Array(items)) => {
                        if let Some(f) = fields {
                            let out: Vec<Value> =
                                items.iter().map(|v| select(&autolink_obj(v), &f)).collect();
                            print_json(&Value::Array(out));
                        } else {
                            for v in &items {
                                let m = autolink_obj(v);
                                println!(
                                    "{}\t{}\t{}\t{}",
                                    m["id"],
                                    m["keyPrefix"].as_str().unwrap_or(""),
                                    m["urlTemplate"].as_str().unwrap_or(""),
                                    m["isAlphanumeric"]
                                );
                            }
                        }
                        ExitCode::SUCCESS
                    }
                    Ok(_) => usage(label, "unexpected response shape"),
                    Err(code) => code,
                }
            })
        }
        "create" | "new" => {
            let label = "repo autolink create";
            let pos = positionals(rest, VALUE_FLAGS);
            let (Some(key), Some(tmpl)) = (pos.first(), pos.get(1)) else {
                return usage(label, "<keyPrefix> <urlTemplate> required");
            };
            let numeric = has_flag(rest, &["--numeric", "-n"]);
            with_ctx(rest, label, |c| {
                let path = format!("/repos/{}/{}/autolinks", c.owner, c.repo);
                let body = json!({
                    "key_prefix": key,
                    "url_template": tmpl,
                    "is_alphanumeric": !numeric,
                });
                match call(&c.base, &c.tok, "POST", &path, Some(&body), label) {
                    Ok(v) => {
                        println!(
                            "Created repository autolink {} on {}",
                            v.get("id").cloned().unwrap_or(Value::Null),
                            c.slug()
                        );
                        ExitCode::SUCCESS
                    }
                    Err(code) => code,
                }
            })
        }
        "view" => {
            let label = "repo autolink view";
            let fields = match json_fields(rest, label, AUTOLINK_FIELDS) {
                Ok(f) => f,
                Err(c) => return c,
            };
            let pos = positionals(rest, VALUE_FLAGS);
            let Some(id) = pos.first().filter(|s| s.parse::<u64>().is_ok()) else {
                return usage(label, "<id> required");
            };
            with_ctx(rest, label, |c| {
                let path = format!("/repos/{}/{}/autolinks/{id}", c.owner, c.repo);
                match call(&c.base, &c.tok, "GET", &path, None, label) {
                    Ok(v) => {
                        let m = autolink_obj(&v);
                        if let Some(f) = fields {
                            print_json(&select(&m, &f));
                        } else {
                            println!("Autolink in {}", c.slug());
                            println!();
                            println!("ID: {}", m["id"]);
                            println!("Key Prefix: {}", m["keyPrefix"].as_str().unwrap_or(""));
                            println!("URL Template: {}", m["urlTemplate"].as_str().unwrap_or(""));
                            println!("Alphanumeric: {}", m["isAlphanumeric"]);
                        }
                        ExitCode::SUCCESS
                    }
                    Err(code) => code,
                }
            })
        }
        "delete" => {
            let label = "repo autolink delete";
            let pos = positionals(rest, VALUE_FLAGS);
            let Some(id) = pos.first().filter(|s| s.parse::<u64>().is_ok()) else {
                return usage(label, "<id> required");
            };
            if !has_flag(rest, &["--yes"]) {
                // arks never prompts; gh's prompt is skipped only with --yes.
                return usage(label, "--yes required when not running interactively");
            }
            with_ctx(rest, label, |c| {
                let path = format!("/repos/{}/{}/autolinks/{id}", c.owner, c.repo);
                match call(&c.base, &c.tok, "DELETE", &path, None, label) {
                    Ok(_) => {
                        println!("Autolink {id} deleted from {}", c.slug());
                        ExitCode::SUCCESS
                    }
                    Err(code) => code,
                }
            })
        }
        other => usage(
            "repo autolink",
            &format!("unknown subcommand `{other}` (create|delete|list|view)"),
        ),
    }
}

pub fn deploy_key(args: &[String]) -> ExitCode {
    let sub = args.first().map(String::as_str).unwrap_or("");
    let rest = args.get(1..).unwrap_or(&[]);
    match sub {
        "" | "-h" | "--help" => {
            eprintln!("arks repo deploy-key — add|delete|list (/repos/{{o}}/{{r}}/keys)");
            ExitCode::SUCCESS
        }
        "list" | "ls" => {
            let label = "repo deploy-key list";
            let fields = match json_fields(rest, label, DEPLOY_KEY_FIELDS) {
                Ok(f) => f,
                Err(c) => return c,
            };
            with_ctx(rest, label, |c| {
                let path = format!("/repos/{}/{}/keys", c.owner, c.repo);
                match call(&c.base, &c.tok, "GET", &path, None, label) {
                    Ok(Value::Array(items)) => {
                        if let Some(f) = fields {
                            let out: Vec<Value> = items
                                .iter()
                                .map(|v| select(&deploy_key_obj(v), &f))
                                .collect();
                            print_json(&Value::Array(out));
                        } else {
                            for v in &items {
                                let m = deploy_key_obj(v);
                                let access = if m["readOnly"].as_bool().unwrap_or(true) {
                                    "read-only"
                                } else {
                                    "read-write"
                                };
                                println!(
                                    "{}\t{}\t{}\t{}\t{}",
                                    m["id"],
                                    m["title"].as_str().unwrap_or(""),
                                    access,
                                    m["key"].as_str().unwrap_or(""),
                                    m["createdAt"].as_str().unwrap_or("")
                                );
                            }
                        }
                        ExitCode::SUCCESS
                    }
                    Ok(_) => usage(label, "unexpected response shape"),
                    Err(code) => code,
                }
            })
        }
        "add" => {
            let label = "repo deploy-key add";
            let pos = positionals(rest, VALUE_FLAGS);
            let Some(file) = pos.first() else {
                return usage(label, "<key-file> required");
            };
            let key = match fs::read_to_string(file) {
                Ok(s) => s.trim().to_string(),
                Err(e) => return usage(label, &format!("read {file}: {e}")),
            };
            let title = crate::flag_value(rest, &["--title", "-t"]).unwrap_or("");
            let allow_write = has_flag(rest, &["--allow-write", "-w"]);
            with_ctx(rest, label, |c| {
                let path = format!("/repos/{}/{}/keys", c.owner, c.repo);
                let body = json!({"title": title, "key": key, "read_only": !allow_write});
                match call(&c.base, &c.tok, "POST", &path, Some(&body), label) {
                    Ok(v) => {
                        println!(
                            "Deploy key added to {} (id {})",
                            c.slug(),
                            v.get("id").cloned().unwrap_or(Value::Null)
                        );
                        ExitCode::SUCCESS
                    }
                    Err(code) => code,
                }
            })
        }
        "delete" => {
            let label = "repo deploy-key delete";
            let pos = positionals(rest, VALUE_FLAGS);
            let Some(id) = pos.first().filter(|s| s.parse::<u64>().is_ok()) else {
                return usage(label, "<key-id> required");
            };
            with_ctx(rest, label, |c| {
                let path = format!("/repos/{}/{}/keys/{id}", c.owner, c.repo);
                match call(&c.base, &c.tok, "DELETE", &path, None, label) {
                    Ok(_) => {
                        println!("Deploy key deleted from {}", c.slug());
                        ExitCode::SUCCESS
                    }
                    Err(code) => code,
                }
            })
        }
        other => usage(
            "repo deploy-key",
            &format!("unknown subcommand `{other}` (add|delete|list)"),
        ),
    }
}
