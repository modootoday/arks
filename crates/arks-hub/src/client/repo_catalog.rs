//! `arks repo gitignore list|view` and `arks repo license list|view`.
//!
//! Peers: <https://cli.github.com/manual/gh_repo_gitignore_list>,
//! <https://cli.github.com/manual/gh_repo_gitignore_view>,
//! <https://cli.github.com/manual/gh_repo_license_list>,
//! <https://cli.github.com/manual/gh_repo_license_view>.
//! REST: `GET /gitignore/templates[/{name}]`, `GET /licenses[/{license}]`.

use super::common::{call, positionals, str_of, usage};
use serde_json::Value;
use std::process::ExitCode;

fn api(label: &str) -> Result<(String, String), ExitCode> {
    crate::require_api().map_err(|e| crate::exit_from_api_err(label, &e))
}

pub fn gitignore(args: &[String]) -> ExitCode {
    let sub = args.first().map(String::as_str).unwrap_or("");
    match sub {
        "" | "-h" | "--help" => {
            eprintln!("arks repo gitignore — list|view <template> (GET /gitignore/templates)");
            ExitCode::SUCCESS
        }
        "list" | "ls" => {
            let label = "repo gitignore list";
            let (base, tok) = match api(label) {
                Ok(v) => v,
                Err(c) => return c,
            };
            match call(&base, &tok, "GET", "/gitignore/templates", None, label) {
                Ok(Value::Array(names)) => {
                    for n in names.iter().filter_map(Value::as_str) {
                        println!("{n}");
                    }
                    ExitCode::SUCCESS
                }
                Ok(_) => usage(label, "unexpected response shape"),
                Err(c) => c,
            }
        }
        "view" => {
            let label = "repo gitignore view";
            let pos = positionals(&args[1..], &[]);
            let Some(name) = pos.first() else {
                return usage(
                    label,
                    "<template> required (see `arks repo gitignore list`)",
                );
            };
            let (base, tok) = match api(label) {
                Ok(v) => v,
                Err(c) => return c,
            };
            let path = format!("/gitignore/templates/{name}");
            match call(&base, &tok, "GET", &path, None, label) {
                Ok(v) => {
                    print!("{}", str_of(&v, "source"));
                    ExitCode::SUCCESS
                }
                Err(c) => c,
            }
        }
        other => usage(
            "repo gitignore",
            &format!("unknown subcommand `{other}` (list|view)"),
        ),
    }
}

pub fn license(args: &[String]) -> ExitCode {
    let sub = args.first().map(String::as_str).unwrap_or("");
    match sub {
        "" | "-h" | "--help" => {
            eprintln!("arks repo license — list|view <license-key|spdx-id> (GET /licenses)");
            ExitCode::SUCCESS
        }
        "list" | "ls" => {
            let label = "repo license list";
            let (base, tok) = match api(label) {
                Ok(v) => v,
                Err(c) => return c,
            };
            match call(&base, &tok, "GET", "/licenses", None, label) {
                Ok(Value::Array(items)) => {
                    for l in &items {
                        println!(
                            "{}\t{}\t{}",
                            str_of(l, "key"),
                            str_of(l, "spdx_id"),
                            str_of(l, "name")
                        );
                    }
                    ExitCode::SUCCESS
                }
                Ok(_) => usage(label, "unexpected response shape"),
                Err(c) => c,
            }
        }
        "view" => {
            let label = "repo license view";
            if super::common::has_flag(&args[1..], &["--web", "-w"]) {
                // Never spawn a browser: print the URL gh would open.
                println!("https://choosealicense.com/");
                eprintln!("note: arks prints the URL; no browser is launched");
                return ExitCode::SUCCESS;
            }
            let pos = positionals(&args[1..], &[]);
            let Some(key) = pos.first() else {
                return usage(label, "<license-key> or <spdx-id> required");
            };
            let (base, tok) = match api(label) {
                Ok(v) => v,
                Err(c) => return c,
            };
            let path = format!("/licenses/{}", key.to_ascii_lowercase());
            match call(&base, &tok, "GET", &path, None, label) {
                Ok(v) => {
                    print!("{}", str_of(&v, "body"));
                    ExitCode::SUCCESS
                }
                Err(c) => c,
            }
        }
        other => usage(
            "repo license",
            &format!("unknown subcommand `{other}` (list|view)"),
        ),
    }
}
