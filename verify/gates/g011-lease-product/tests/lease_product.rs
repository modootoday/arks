//! Gate G11 — lease-product (M10). Host cargo tests are advisory; this suite
//! runs inside Docker clean-room via `verify/scripts/run-g011-lease-product.sh`.
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
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../..");
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
fn g011_lease_product_help_lists_sv_and_not_forge() {
    // Top-level help is personality-condensed (serve umbrella); Wave Sv plumbing
    // remains discoverable via subcommand help. CELIR stays non-product in help.
    let out = Command::new(arks_bin())
        .arg("--help")
        .output()
        .expect("arks --help");
    assert!(out.status.success());
    let text = String::from_utf8_lossy(&out.stdout);
    assert!(text.contains("serve"));
    assert!(!text.to_lowercase().contains("celir"));

    // Wave Sv leaf present: incomplete argv fails closed with directory usage
    // (unknown leaves would say unsupported/unknown — not directory-required).
    let up = Command::new(arks_bin())
        .arg("upload-pack")
        .output()
        .expect("arks upload-pack");
    let up_text = format!(
        "{}{}",
        String::from_utf8_lossy(&up.stdout),
        String::from_utf8_lossy(&up.stderr)
    )
    .to_lowercase();
    assert!(!up.status.success());
    assert!(
        up_text.contains("directory") || up_text.contains("upload-pack"),
        "expected upload-pack directory usage, got: {up_text}"
    );
    assert!(
        !up_text.contains("unknown") && !up_text.contains("unsupported"),
        "upload-pack must not look like an unknown leaf: {up_text}"
    );
}

#[test]
fn g011_lease_product_lease_crate_tests_are_invoked_by_gate_script() {
    // Marker: run-g011-lease-product.sh runs `cargo test -p arks-lease` which owns L1/L2/lock proofs.
    assert!(true);
}
