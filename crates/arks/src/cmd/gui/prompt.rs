//! `git gui--askpass` and `git gui--askyesno` as terminal prompts.
//!
//! git-gui installs these as `SSH_ASKPASS` / `GIT_ASKPASS` and `GIT_ASK_YESNO`
//! programs: askpass shows its arguments as the prompt and prints the answer
//! on standard output (exit 0; cancel exits 1); askyesno asks its arguments
//! as a yes/no question and exits 0 for yes, 1 for no. Here the prompt goes to
//! standard error and the answer is read from standard input (echo off for a
//! password on a terminal); end of input is "cancel" / "no".
//! Cite: <https://git-scm.com/docs/gitcredentials> (askpass),
//! <https://git-scm.com/docs/git-gui> (behaviour, not code).

use super::term::{read_hidden_line, read_line_stdin};
use crate::error::{CliError, CliResult};
use std::io::{IsTerminal, Write};

fn prompt_text(args: &[String], default: &str) -> String {
    let p = args.join(" ");
    if p.trim().is_empty() {
        default.to_string()
    } else {
        p
    }
}

fn show(prompt: &str) -> CliResult<()> {
    let mut err = std::io::stderr();
    write!(err, "{prompt}")?;
    if !prompt.ends_with(' ') {
        write!(err, " ")?;
    }
    err.flush()?;
    Ok(())
}

/// Whether the answer to `prompt` is a secret (anything but a user name).
fn is_secret(prompt: &str) -> bool {
    !prompt.to_ascii_lowercase().contains("username")
}

/// `git gui--askpass [<prompt>...]`.
pub fn cmd_gui_askpass(args: &[String]) -> CliResult<()> {
    let prompt = prompt_text(args, "Password:");
    show(&prompt)?;
    let tty = std::io::stdin().is_terminal();
    let answer = if tty && is_secret(&prompt) {
        let a = read_hidden_line()?;
        eprintln!();
        a
    } else {
        read_line_stdin()?
    };
    match answer {
        Some(a) => {
            println!("{a}");
            Ok(())
        }
        None => Err(CliError::silent(1)),
    }
}

/// `git gui--askyesno [<question>...]`.
pub fn cmd_gui_askyesno(args: &[String]) -> CliResult<()> {
    let question = prompt_text(args, "Continue?");
    let tty = std::io::stdin().is_terminal();
    loop {
        show(&format!("{} (y/n)", question.trim_end()))?;
        let Some(a) = read_line_stdin()? else {
            return Err(CliError::silent(1));
        };
        match a.trim().to_ascii_lowercase().as_str() {
            "y" | "yes" => return Ok(()),
            "n" | "no" => return Err(CliError::silent(1)),
            _ if tty => continue,
            _ => return Err(CliError::silent(1)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prompt_defaults_and_secrets() {
        assert_eq!(prompt_text(&[], "Password:"), "Password:");
        assert_eq!(
            prompt_text(&["Password".into(), "for".into(), "x:".into()], "P"),
            "Password for x:"
        );
        assert!(is_secret("Password for 'https://a@b': "));
        assert!(!is_secret("Username for 'https://b': "));
    }
}
