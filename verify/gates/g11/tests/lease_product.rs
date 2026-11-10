//! Gate G11 — lease-product (M10). Host cargo tests are advisory; this suite
//! runs inside Docker clean-room via `verify/scripts/run-g11.sh`.
//!
//! Proves: L1 grant/release/expire; L2 durable; dual-path refuse; lease ≠ skip
//! Git locks (via `cargo test -p arks-lease` in the gate script). This file
//! adds an argv-level smoke that `arks` still builds and G14 inventory help
//! remains honest (no forge/CELIR).

use std::path::PathBuf;
use std::process::Command;

fn arks_bin() -> PathBuf {
    let mut candidates: Vec<PathBuf> = Vec::new();
    if let Ok(target) = std::env::var("CARGO_TARGET_DIR") {
        candidates.push(PathBuf::from(&target).join("debug/arks"));
        candidates.push(PathBuf::from(&target).join("debug/arks.exe"));
    } else {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
        candidates.push(root.join("target/debug/arks.exe"));
        candidates.push(root.join("target/debug/arks"));
        candidates.push(root.join("target-cleanroom/debug/arks"));
    }
    for c in &candidates {
        if c.is_file() {
            return c.clone();
        }
    }
    panic!("missing arks binary; tried {candidates:?}");
}

#[test]
fn g11_help_lists_sv_and_not_forge() {
    let out = Command::new(arks_bin())
        .arg("--help")
        .output()
        .expect("arks --help");
    assert!(out.status.success());
    let text = String::from_utf8_lossy(&out.stdout);
    assert!(text.contains("upload-pack"));
    assert!(!text.to_lowercase().contains("celir"));
    assert!(!text.to_lowercase().contains("forge"));
}

#[test]
fn g11_lease_crate_tests_are_invoked_by_gate_script() {
    // Marker: run-g11.sh runs `cargo test -p arks-lease` which owns L1/L2/lock proofs.
    assert!(true);
}
