//! CLI personality + CommandLayer registry skeleton.
//!
//! Cite: `.spec/sot/arks-cli-personality-factory.sot.md`,
//! `.spec/sot/arks-cli-output-exit-contract.sot.md`
//! Design/Partial — stubs exit non-zero; do not invent Claimed.

mod exit_code;
mod git_dispatch;
mod overlap;
mod personality;
mod registry;

pub use registry::dispatch_argv;
