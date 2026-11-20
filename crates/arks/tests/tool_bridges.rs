//! A3 external tool bridges driven end to end with local stand-ins: an SMTP
//! sink and an IMAP server on loopback ports, scripted merge tools, filters,
//! askpass helpers and the instaweb server. No real mail leaves the host and
//! stock git is not needed.
//!
//! Primary: <https://git-scm.com/docs/git-send-email>,
//! <https://git-scm.com/docs/git-imap-send>, <https://git-scm.com/docs/git-mergetool>,
//! <https://git-scm.com/docs/git-filter-branch>, <https://git-scm.com/docs/git-quiltimport>,
//! <https://git-scm.com/docs/git-instaweb>, <https://git-scm.com/docs/git-web--browse>.

use std::io::{BufRead, BufReader, Write};
use std::net::TcpListener;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use std::sync::{Arc, Mutex};
use std::thread;

fn arks(dir: &Path) -> Command {
    let mut c = Command::new(env!("CARGO_BIN_EXE_arks"));
    c.current_dir(dir)
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .env("ARKS_PERSONALITY", "git")
        .env("GIT_AUTHOR_NAME", "A U Thor")
        .env("GIT_AUTHOR_EMAIL", "author@example.com")
        .env("GIT_COMMITTER_NAME", "C O Mitter")
        .env("GIT_COMMITTER_EMAIL", "committer@example.com")
        .env("GIT_AUTHOR_DATE", "1700000000 +0000")
        .env("GIT_COMMITTER_DATE", "1700000000 +0000")
        .env("FILTER_BRANCH_SQUELCH_WARNING", "1")
        .env_remove("GIT_ASKPASS")
        .env_remove("SSH_ASKPASS")
        .stdin(Stdio::null());
    c
}

fn run(dir: &Path, args: &[&str]) -> Output {
    arks(dir).args(args).output().expect("run arks")
}

fn ok(dir: &Path, args: &[&str]) -> String {
    let out = run(dir, args);
    assert!(
        out.status.success(),
        "{args:?} failed: {}\n{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout).into_owned()
}

/// A repository with two commits on `main` touching `a.txt`.
fn repo(root: &Path) -> PathBuf {
    let d = root.join("repo");
    std::fs::create_dir_all(&d).unwrap();
    ok(&d, &["init", "-q", "-b", "main"]);
    ok(&d, &["config", "commit.gpgsign", "false"]);
    std::fs::write(d.join("a.txt"), "one\n").unwrap();
    ok(&d, &["add", "a.txt"]);
    ok(&d, &["commit", "-q", "-m", "first change"]);
    std::fs::write(d.join("a.txt"), "one\ntwo\n").unwrap();
    ok(
        &d,
        &[
            "commit",
            "-q",
            "-am",
            "second change\n\nSigned-off-by: S O B <sob@example.com>",
        ],
    );
    d
}

#[test]
fn send_email_talks_smtp_to_a_local_sink() {
    let t = tempfile::tempdir().unwrap();
    let d = repo(t.path());
    let l = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = l.local_addr().unwrap().port();
    let log = Arc::new(Mutex::new(Vec::<String>::new()));
    let log2 = log.clone();
    let server = thread::spawn(move || {
        let (s, _) = l.accept().unwrap();
        let mut w = s.try_clone().unwrap();
        let mut r = BufReader::new(s);
        let mut say = |m: &str| w.write_all(format!("{m}\r\n").as_bytes()).unwrap();
        say("220 sink ESMTP");
        let mut in_data = false;
        let mut line = String::new();
        loop {
            line.clear();
            if r.read_line(&mut line).unwrap() == 0 {
                break;
            }
            let l = line.trim_end_matches(['\r', '\n']).to_string();
            log2.lock().unwrap().push(l.clone());
            if in_data {
                if l == "." {
                    in_data = false;
                    say("250 queued");
                }
                continue;
            }
            let up = l.to_ascii_uppercase();
            if up.starts_with("EHLO") {
                say("250-sink");
                say("250 AUTH PLAIN");
            } else if up.starts_with("AUTH PLAIN") {
                say("235 ok");
            } else if up.starts_with("DATA") {
                in_data = true;
                say("354 go");
            } else if up.starts_with("QUIT") {
                say("221 bye");
                break;
            } else {
                say("250 ok");
            }
        }
    });
    let port_s = port.to_string();
    let out = ok(
        &d,
        &[
            "send-email",
            "--to=list@example.com",
            "--from=Sender <sender@example.com>",
            "--smtp-server=127.0.0.1",
            "--smtp-server-port",
            &port_s,
            "--smtp-user=me",
            "--smtp-pass=secret",
            "--confirm=never",
            "--in-reply-to=<root@example.com>",
            "-2",
        ],
    );
    server.join().unwrap();
    assert!(out.contains("OK. Log says:"), "{out}");
    assert!(out.contains("Result: 250"), "{out}");
    let log = log.lock().unwrap().join("\n");
    assert!(log.contains("MAIL FROM:<sender@example.com>"), "{log}");
    assert!(log.contains("RCPT TO:<list@example.com>"), "{log}");
    assert!(log.contains("RCPT TO:<author@example.com>"), "{log}");
    assert!(log.contains("RCPT TO:<sob@example.com>"), "{log}");
    assert!(log.contains("Subject: [PATCH 1/2] first change"), "{log}");
    assert!(log.contains("Subject: [PATCH 2/2] second change"), "{log}");
    assert!(log.contains("In-Reply-To: <root@example.com>"), "{log}");
    assert!(log.contains("From: A U Thor <author@example.com>"), "{log}");
    assert!(log.contains("AUTH PLAIN"), "{log}");
}

#[test]
fn send_email_dry_run_and_sendmail_command() {
    let t = tempfile::tempdir().unwrap();
    let d = repo(t.path());
    let out = ok(
        &d,
        &[
            "send-email",
            "--dry-run",
            "--quiet",
            "--to=x@example.com",
            "--from=S <s@example.com>",
            "--smtp-server=127.0.0.1",
            "--smtp-server-port=1",
            "--confirm=never",
            "-1",
        ],
    );
    assert!(out.contains("Dry-Sent [PATCH] second change"), "{out}");
    let sink = t.path().join("sink");
    let script = t.path().join("fake-sendmail");
    std::fs::write(
        &script,
        format!(
            "#!/bin/sh\necho \"$@\" >> '{0}'\ncat >> '{0}'\n",
            sink.display().to_string().replace('\\', "/")
        ),
    )
    .unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&script, std::fs::Permissions::from_mode(0o755)).unwrap();
    }
    // --sendmail-cmd runs through the shell (as in git), where backslashes
    // are escapes; Windows accepts '/' in paths.
    let cmd = format!(
        "--sendmail-cmd={}",
        script.display().to_string().replace('\\', "/")
    );
    ok(
        &d,
        &[
            "send-email",
            "--to=x@example.com",
            "--from=S <s@example.com>",
            &cmd,
            "--confirm=never",
            "--suppress-cc=all",
            "-1",
        ],
    );
    let got = std::fs::read_to_string(&sink).unwrap();
    assert!(got.starts_with("-i x@example.com\n"), "{got}");
    assert!(got.contains("Subject: [PATCH] second change"), "{got}");
    assert!(!got.contains("sob@example.com>,"), "{got}");
}

#[test]
fn imap_send_appends_to_the_folder() {
    let t = tempfile::tempdir().unwrap();
    let d = repo(t.path());
    let mbox = ok(&d, &["format-patch", "--stdout", "-2"]);
    let l = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = l.local_addr().unwrap().port();
    let got = Arc::new(Mutex::new(Vec::<String>::new()));
    let got2 = got.clone();
    let server = thread::spawn(move || {
        let (s, _) = l.accept().unwrap();
        let mut w = s.try_clone().unwrap();
        let mut r = BufReader::new(s);
        w.write_all(b"* OK test IMAP ready\r\n").unwrap();
        let mut line = String::new();
        loop {
            line.clear();
            if r.read_line(&mut line).unwrap() == 0 {
                break;
            }
            let l = line.trim_end_matches(['\r', '\n']).to_string();
            let (tag, rest) = l.split_once(' ').unwrap();
            let up = rest.to_ascii_uppercase();
            if up.starts_with("CAPABILITY") {
                w.write_all(b"* CAPABILITY IMAP4rev1\r\n").unwrap();
            } else if up.starts_with("LOGIN") {
                got2.lock().unwrap().push(rest.to_string());
            } else if up.starts_with("APPEND") {
                let n: usize = rest
                    .rsplit('{')
                    .next()
                    .unwrap()
                    .trim_end_matches('}')
                    .parse()
                    .unwrap();
                w.write_all(b"+ go ahead\r\n").unwrap();
                let mut buf = vec![0u8; n + 2];
                std::io::Read::read_exact(&mut r, &mut buf).unwrap();
                got2.lock()
                    .unwrap()
                    .push(format!("{rest}\n{}", String::from_utf8_lossy(&buf[..n])));
            } else if up.starts_with("LOGOUT") {
                w.write_all(format!("* BYE\r\n{tag} OK done\r\n").as_bytes())
                    .unwrap();
                break;
            }
            w.write_all(format!("{tag} OK done\r\n").as_bytes())
                .unwrap();
        }
    });
    let host = "imap.host=imap://127.0.0.1".to_string();
    let port_c = format!("imap.port={port}");
    let mut child = arks(&d)
        .args([
            "-c",
            "imap.folder=Drafts",
            "-c",
            &host,
            "-c",
            &port_c,
            "-c",
            "imap.user=bob",
            "-c",
            "imap.pass=pw",
            "imap-send",
            "-q",
        ])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    child
        .stdin
        .take()
        .unwrap()
        .write_all(mbox.as_bytes())
        .unwrap();
    let out = child.wait_with_output().unwrap();
    server.join().unwrap();
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let got = got.lock().unwrap();
    assert_eq!(got[0], "LOGIN \"bob\" \"pw\"");
    assert_eq!(got.len(), 3, "{got:?}");
    assert!(got[1].starts_with("APPEND \"Drafts\" {"), "{}", got[1]);
    assert!(
        got[1].contains("Subject: [PATCH 1/2] first change\r\n"),
        "{}",
        got[1]
    );
    assert!(!got[1].contains("\nFrom "), "{}", got[1]);
    assert!(
        got[2].contains("Subject: [PATCH 2/2] second change"),
        "{}",
        got[2]
    );
}

#[test]
fn mergetool_stages_the_result_and_keeps_a_backup() {
    let t = tempfile::tempdir().unwrap();
    let d = repo(t.path());
    ok(&d, &["checkout", "-q", "-b", "other", "HEAD~1"]);
    std::fs::write(d.join("a.txt"), "one\nTHEIRS\n").unwrap();
    ok(&d, &["commit", "-q", "-am", "other side"]);
    ok(&d, &["checkout", "-q", "main"]);
    let m = run(&d, &["merge", "other"]);
    assert!(!m.status.success());
    let out = ok(
        &d,
        &[
            "-c",
            "merge.tool=x",
            "-c",
            "mergetool.x.cmd=cat \"$REMOTE\" > \"$MERGED\"",
            "-c",
            "mergetool.x.trustExitCode=true",
            "mergetool",
        ],
    );
    assert!(
        out.starts_with("Merging:\na.txt\n\nNormal merge conflict for 'a.txt':"),
        "{out}"
    );
    assert_eq!(
        std::fs::read_to_string(d.join("a.txt")).unwrap(),
        "one\nTHEIRS\n"
    );
    assert!(
        std::fs::read_to_string(d.join("a.txt.orig"))
            .unwrap()
            .contains("<<<<<<<")
    );
    let staged = ok(&d, &["ls-files", "-s"]);
    assert!(staged.contains(" 0\ta.txt"), "{staged}");
    assert!(!staged.contains(" 2\ta.txt"), "{staged}");
    let left: Vec<_> = std::fs::read_dir(&d)
        .unwrap()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_name().to_string_lossy().contains("_LOCAL_"))
        .collect();
    assert!(left.is_empty());
}

#[test]
fn filter_branch_rewrites_and_backs_up() {
    let t = tempfile::tempdir().unwrap();
    let d = repo(t.path());
    let before = ok(&d, &["rev-parse", "HEAD"]);
    let out = ok(
        &d,
        &[
            "filter-branch",
            "--msg-filter",
            "tr a-z A-Z",
            "--tree-filter",
            "echo added > new.txt",
            "HEAD",
        ],
    );
    assert!(out.contains("Ref 'refs/heads/main' was rewritten"), "{out}");
    assert_eq!(
        ok(&d, &["rev-parse", "refs/original/refs/heads/main"]),
        before
    );
    assert_eq!(
        ok(&d, &["log", "--format=%s"]),
        "SECOND CHANGE\nFIRST CHANGE\n"
    );
    assert_eq!(
        std::fs::read_to_string(d.join("new.txt")).unwrap(),
        "added\n"
    );
    assert!(!d.join(".git-rewrite").exists());
    // A second run needs -f because of the backup.
    let again = run(&d, &["filter-branch", "--msg-filter", "cat", "HEAD"]);
    assert_eq!(again.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&again.stderr).contains("A previous backup already exists"));
    let none = run(
        &d,
        &["filter-branch", "-f", "--subdirectory-filter", "nosuch"],
    );
    assert_eq!(none.status.code(), Some(2));
}

#[test]
fn quiltimport_commits_each_patch() {
    let t = tempfile::tempdir().unwrap();
    let d = repo(t.path());
    let p = d.join("patches");
    std::fs::create_dir_all(&p).unwrap();
    std::fs::write(
        p.join("add-b.patch"),
        "From: Q U Ilt <quilt@example.com>\nSubject: add b\n\nAdds b.\n---\ndiff --git a/b.txt b/b.txt\nnew file mode 100644\n--- /dev/null\n+++ b/b.txt\n@@ -0,0 +1 @@\n+bee\n",
    )
    .unwrap();
    std::fs::write(p.join("series"), "add-b.patch\n").unwrap();
    let out = ok(&d, &["quiltimport"]);
    assert_eq!(out, "add-b.patch\n");
    assert_eq!(
        ok(&d, &["log", "-1", "--format=%an <%ae>|%s|%b"]),
        "Q U Ilt <quilt@example.com>|add b|Adds b.\n\n"
    );
    assert_eq!(std::fs::read_to_string(d.join("b.txt")).unwrap(), "bee\n");
}

#[cfg(unix)]
#[test]
fn askpass_helpers_and_browser_commands() {
    let t = tempfile::tempdir().unwrap();
    let d = repo(t.path());
    let helper = t.path().join("helper");
    std::fs::write(&helper, "#!/bin/sh\necho \"answer to $1\"\n").unwrap();
    use std::os::unix::fs::PermissionsExt;
    std::fs::set_permissions(&helper, std::fs::Permissions::from_mode(0o755)).unwrap();
    let out = arks(&d)
        .env("GIT_ASKPASS", &helper)
        .args(["askpass", "Password:"])
        .output()
        .unwrap();
    assert_eq!(
        String::from_utf8_lossy(&out.stdout),
        "answer to Password:\n"
    );
    let h = helper.to_string_lossy().into_owned();
    let cfg = format!("core.askPass={h}");
    assert_eq!(
        ok(&d, &["-c", &cfg, "askpass", "Username:"]),
        "answer to Username:\n"
    );
    let yes = arks(&d)
        .env("GIT_ASK_YESNO", "true")
        .args(["askyesno", "ok?"])
        .status()
        .unwrap();
    let no = arks(&d)
        .env("GIT_ASK_YESNO", "false")
        .args(["askyesno", "ok?"])
        .status()
        .unwrap();
    assert_eq!((yes.code(), no.code()), (Some(0), Some(1)));
    let closed = arks(&d)
        .env("GIT_TERMINAL_PROMPT", "0")
        .args(["askpass", "Password:"])
        .status()
        .unwrap();
    assert_eq!(closed.code(), Some(1));
    assert_eq!(
        ok(
            &d,
            &[
                "-c",
                "browser.x.cmd=echo opened",
                "web--browse",
                "-b",
                "x",
                "http://h/a"
            ]
        ),
        "opened http://h/a\n"
    );
    // difftool--helper: the external-diff call with the difftool environment.
    let z = "0000000000000000000000000000000000000000";
    let helper = arks(&d)
        .env("GIT_DIFFTOOL_EXTCMD", "echo")
        .env("GIT_DIFFTOOL_NO_PROMPT", "true")
        .args([
            "difftool--helper",
            "a.txt",
            "/tmp/old",
            z,
            "100644",
            "a.txt",
            z,
            "100644",
        ])
        .output()
        .unwrap();
    assert_eq!(String::from_utf8_lossy(&helper.stdout), "/tmp/old a.txt\n");
    let prompted = arks(&d)
        .env("GIT_DIFFTOOL_EXTCMD", "echo")
        .env("GIT_DIFF_PATH_COUNTER", "2")
        .env("GIT_DIFF_PATH_TOTAL", "3")
        .args([
            "difftool--helper",
            "a.txt",
            "/tmp/old",
            z,
            "100644",
            "a.txt",
            z,
            "100644",
        ])
        .output()
        .unwrap();
    assert_eq!(
        String::from_utf8_lossy(&prompted.stdout),
        "\nViewing (2/3): 'a.txt'\nLaunch 'echo' [Y/n]? "
    );
}

#[cfg(unix)]
#[test]
fn instaweb_starts_and_stops_the_server() {
    let t = tempfile::tempdir().unwrap();
    let d = repo(t.path());
    let port = {
        let l = TcpListener::bind("127.0.0.1:0").unwrap();
        l.local_addr().unwrap().port()
    };
    let p = port.to_string();
    ok(&d, &["instaweb", "--local", "--port", &p, "--start"]);
    let pid = std::fs::read_to_string(d.join(".git/pid")).unwrap();
    assert!(pid.trim().parse::<u32>().is_ok());
    assert!(std::net::TcpStream::connect(("127.0.0.1", port)).is_ok());
    ok(&d, &["instaweb", "--stop"]);
    assert!(!d.join(".git/pid").exists());
    let mut down = false;
    for _ in 0..100 {
        if std::net::TcpStream::connect(("127.0.0.1", port)).is_err() {
            down = true;
            break;
        }
        thread::sleep(std::time::Duration::from_millis(20));
    }
    assert!(down, "server still listening after --stop");
}
