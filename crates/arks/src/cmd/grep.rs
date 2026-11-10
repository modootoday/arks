//! Search tracked files.

use crate::error::{CliError, CliResult};
use crate::util::{open_repo, read_index, refuse_unknown_flags, work_tree};
use std::fs;
pub fn cmd_grep(args: &[String]) -> CliResult<()> {
    refuse_unknown_flags(args, &["-P", "-n", "-i"])?;
    if args.iter().any(|a| a == "-P") {
        return Err(CliError::NotImplemented(
            "grep -P not claimed in Wave Pa".into(),
        ));
    }
    let ignore_case = args.iter().any(|a| a == "-i");
    let show_line = args.iter().any(|a| a == "-n");
    let positional: Vec<&String> = args.iter().filter(|a| !a.starts_with('-')).collect();
    if positional.is_empty() {
        return Err(CliError::Usage("grep [-n] [-i] <pattern>".into()));
    }
    let pattern = positional[0];
    let repo = open_repo()?;
    let work = work_tree(&repo)?;
    let index = read_index(&repo)?;
    for e in index.entries.iter().filter(|e| e.stage() == 0) {
        let abs = work.join(&e.path);
        if !abs.is_file() {
            continue;
        }
        let text = match fs::read_to_string(&abs) {
            Ok(t) => t,
            Err(_) => continue,
        };
        for (i, line) in text.lines().enumerate() {
            let hit = if ignore_case {
                line.to_lowercase().contains(&pattern.to_lowercase())
            } else {
                line.contains(pattern.as_str())
            };
            if hit {
                if show_line {
                    println!("{}:{}:{line}", e.path, i + 1);
                } else {
                    println!("{}:{line}", e.path);
                }
            }
        }
    }
    Ok(())
}
