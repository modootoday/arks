//! `git askpass <prompt>` and `git askyesno <question>`: the password and
//! yes/no prompt helpers (git-gui and Git for Windows ship them as dialogs).
//! arks asks on the terminal.
//!
//! Precedence follows gitcredentials(7) / git-config(1) `core.askPass`:
//! `GIT_ASKPASS`, then `core.askPass`, then `SSH_ASKPASS` name a program
//! that is run with the prompt as its argument and whose output is the
//! answer; without one the terminal is used (`/dev/tty`, input hidden for
//! anything that is not a user name prompt) unless `GIT_TERMINAL_PROMPT=0`.
//! `askyesno` uses `GIT_ASK_YESNO` the same way and exits 0 for yes, 1 for no.
//! A helper that names arks' own `askpass` / `askyesno` is skipped so the
//! lookup cannot recurse.

use super::proc;
use crate::error::{CliError, CliResult};
use std::io::{BufRead, Write};

const GUARD: &str = "ARKS_ASKPASS_ACTIVE";

pub fn cmd_askpass(args: &[String]) -> CliResult<()> {
    let prompt = args.join(" ");
    let prompt = if prompt.is_empty() {
        "Password: ".to_string()
    } else {
        prompt
    };
    if let Some(prog) = helper(&["GIT_ASKPASS"], Some("core.askPass"), &["SSH_ASKPASS"]) {
        let out = run_helper(&prog, &prompt)?;
        println!("{}", out.trim_end_matches(['\n', '\r']));
        return Ok(());
    }
    let hide = !prompt.to_ascii_lowercase().starts_with("username");
    let answer = terminal(&prompt, hide)?;
    println!("{answer}");
    Ok(())
}

pub fn cmd_askyesno(args: &[String]) -> CliResult<()> {
    let question = args.join(" ");
    if let Some(prog) = helper(&["GIT_ASK_YESNO"], None, &[]) {
        let mut c = proc::shell(
            &format!("{prog} \"$@\""),
            &prog,
            std::slice::from_ref(&question),
        );
        c.env(GUARD, "1");
        let st = proc::run(c)?;
        return if st.success() {
            Ok(())
        } else {
            Err(CliError::silent(1))
        };
    }
    let q = if question.is_empty() {
        "Continue? (y/n) ".to_string()
    } else if question.trim_end().ends_with("(y/n)") {
        format!("{} ", question.trim_end())
    } else {
        format!("{} (y/n) ", question.trim_end())
    };
    loop {
        let a = terminal(&q, false)?;
        match a.trim().to_ascii_lowercase().as_str() {
            "y" | "yes" => return Ok(()),
            "n" | "no" => return Err(CliError::silent(1)),
            _ => {}
        }
    }
}

/// The first configured helper program (environment before config).
fn helper(first: &[&str], cfg_key: Option<&str>, last: &[&str]) -> Option<String> {
    if std::env::var_os(GUARD).is_some() {
        return None;
    }
    let env = |k: &str| std::env::var(k).ok().filter(|v| !v.is_empty());
    let cfg = || {
        let key = cfg_key?;
        let repo = crate::util::open_repo().ok();
        arks_config::ConfigSet::load(repo.as_ref().map(|r| r.common_dir()))
            .get(key)
            .filter(|v| !v.is_empty())
    };
    first
        .iter()
        .find_map(|k| env(k))
        .or_else(cfg)
        .or_else(|| last.iter().find_map(|k| env(k)))
        .filter(|p| !is_self(p))
}

/// A helper value that runs arks' own prompt again.
fn is_self(prog: &str) -> bool {
    let words: Vec<&str> = prog.split_whitespace().collect();
    let Some(first) = words.first() else {
        return false;
    };
    let base = first.rsplit(['/', '\\']).next().unwrap_or(first);
    let own = std::env::current_exe()
        .ok()
        .and_then(|p| p.file_name().map(|n| n.to_string_lossy().into_owned()));
    let is_arks = base == "arks" || own.as_deref() == Some(base) || base == "git";
    (is_arks
        && words
            .get(1)
            .is_some_and(|w| *w == "askpass" || *w == "askyesno"))
        || matches!(base, "git-askpass" | "git-askyesno")
}

fn run_helper(prog: &str, prompt: &str) -> CliResult<String> {
    // Run through the shell so a value with arguments works too.
    let mut c = proc::shell(&format!("{prog} \"$@\""), prog, &[prompt.to_string()]);
    c.env(GUARD, "1")
        .stdin(std::process::Stdio::inherit())
        .stderr(std::process::Stdio::inherit());
    let out = c.output().map_err(|e| {
        CliError::Exit(
            1,
            format!("error: unable to run askpass helper {prog}: {e}"),
        )
    })?;
    if !out.status.success() {
        return Err(CliError::Exit(
            1,
            format!("error: unable to read askpass response from '{prog}'"),
        ));
    }
    Ok(String::from_utf8_lossy(&out.stdout).into_owned())
}

/// Ask on the controlling terminal (`hide`: no echo).
fn terminal(prompt: &str, hide: bool) -> CliResult<String> {
    let fail = || CliError::Exit(1, "fatal: could not read from terminal".into());
    if matches!(std::env::var("GIT_TERMINAL_PROMPT").as_deref(), Ok("0")) {
        return Err(CliError::Exit(
            1,
            "fatal: could not read from terminal: terminal prompts disabled".into(),
        ));
    }
    #[cfg(unix)]
    {
        let tty = std::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .open("/dev/tty")
            .map_err(|_| fail())?;
        let mut w = tty.try_clone().map_err(|_| fail())?;
        w.write_all(prompt.as_bytes()).map_err(|_| fail())?;
        w.flush().map_err(|_| fail())?;
        let guard = if hide { EchoOff::new(&tty) } else { None };
        let mut line = String::new();
        let n = std::io::BufReader::new(&tty)
            .read_line(&mut line)
            .map_err(|_| fail())?;
        drop(guard);
        if hide {
            let _ = w.write_all(b"\n");
        }
        if n == 0 {
            return Err(fail());
        }
        Ok(line.trim_end_matches(['\n', '\r']).to_string())
    }
    #[cfg(not(unix))]
    {
        let _ = hide;
        eprint!("{prompt}");
        let mut line = String::new();
        let n = std::io::stdin().read_line(&mut line).map_err(|_| fail())?;
        if n == 0 {
            return Err(fail());
        }
        Ok(line.trim_end_matches(['\n', '\r']).to_string())
    }
}

/// Terminal echo switched off for the guard's lifetime.
#[cfg(unix)]
struct EchoOff {
    fd: i32,
    saved: libc::termios,
}

#[cfg(unix)]
impl EchoOff {
    fn new(f: &std::fs::File) -> Option<Self> {
        use std::os::unix::io::AsRawFd;
        let fd = f.as_raw_fd();
        // SAFETY: tcgetattr/tcsetattr on an open terminal descriptor with a
        // zero-initialised termios buffer owned by this frame.
        unsafe {
            let mut t: libc::termios = std::mem::zeroed();
            if libc::tcgetattr(fd, &mut t) != 0 {
                return None;
            }
            let saved = t;
            t.c_lflag &= !libc::ECHO;
            if libc::tcsetattr(fd, libc::TCSANOW, &t) != 0 {
                return None;
            }
            Some(Self { fd, saved })
        }
    }
}

#[cfg(unix)]
impl Drop for EchoOff {
    fn drop(&mut self) {
        // SAFETY: restores the attributes read in `new` on the same descriptor.
        unsafe {
            libc::tcsetattr(self.fd, libc::TCSANOW, &self.saved);
        }
    }
}
