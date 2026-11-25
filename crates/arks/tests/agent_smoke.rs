//! `arks agent provider smoke` and `arks agent review --smoke` diagnostics (queue
//! unit Q21; bar in [[sot:root/arks-claimed-residual-gates]] §Q21). Host-advisory
//! twin of the Docker receipt, using the in-process `--mock` provider and a closed
//! loopback port for the failure exits.

use std::net::TcpListener;
use std::process::{Command, Output};

fn agent(args: &[&str]) -> Output {
    let home = tempfile::tempdir().unwrap();
    Command::new(env!("CARGO_BIN_EXE_arks"))
        .arg("agent")
        .args(args)
        .env("HOME", home.path())
        .env("USERPROFILE", home.path())
        .env_remove("ARKS_AGENT_PROVIDER")
        .env_remove("ARKS_AGENT_BASE_URL")
        .output()
        .unwrap()
}

fn stdout(o: &Output) -> String {
    String::from_utf8_lossy(&o.stdout).into_owned()
}

fn closed_url() -> String {
    let port = TcpListener::bind("127.0.0.1:0")
        .unwrap()
        .local_addr()
        .unwrap()
        .port();
    format!("http://127.0.0.1:{port}/v1")
}

#[test]
fn provider_smoke_mock_answers() {
    let out = agent(&["provider", "smoke", "--mock"]);
    let text = stdout(&out);
    assert!(out.status.success(), "{out:?}");
    assert!(text.contains("chat_http=200"), "{text}");
    assert!(text.contains("smoke_result=ok"), "{text}");
    assert!(!text.contains("Missing"), "{text}");
}

#[test]
fn provider_smoke_unreachable_exits_1() {
    let url = closed_url();
    let out = agent(&["provider", "smoke", "openai-compat", "--base-url", &url]);
    assert_eq!(out.status.code(), Some(1), "{out:?}");
    assert!(stdout(&out).contains("smoke_result=fail"), "{out:?}");
}

#[test]
fn review_smoke_mock_answers() {
    let out = agent(&["review", "--smoke", "--mock"]);
    let text = stdout(&out);
    assert!(out.status.success(), "{out:?}");
    assert!(text.contains("review_smoke=ok"), "{text}");
}

#[test]
fn review_smoke_unreachable_exits_1() {
    let url = closed_url();
    let out = agent(&["review", "--smoke", "--base-url", &url]);
    assert_eq!(out.status.code(), Some(1), "{out:?}");
    assert!(stdout(&out).contains("review_smoke=fail"), "{out:?}");
}

#[test]
fn review_smoke_honours_the_provider_flag() {
    // Real (non-mock) path against a closed port: the smoke fails, but the
    // provider it reports must be the one the flag named.
    let url = closed_url();
    let out = agent(&[
        "review",
        "--smoke",
        "--provider",
        "openai-compat",
        "--base-url",
        &url,
    ]);
    let text = stdout(&out);
    assert!(text.contains("provider=openai-compat"), "{text}");
    assert!(!text.contains("provider=ollama"), "{text}");
}
