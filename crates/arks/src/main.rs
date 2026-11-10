//! `arks` — Wave L+H+Pa+Pb+Pc + Sv CLI.
//!
//! M9 = CLI-parity production (client). M10 leases via `arks-lease`. M11
//! production-server via Wave Sv (`upload-pack` / `receive-pack` / `serve`).
//! Cite: `.spec/sot/arks.sot.md`, `.spec/sot/cli-surface.sot.md`,
//! `.spec/sot/arks-cli-personality-factory.sot.md`,
//! `.spec/sot/arks-cli-output-exit-contract.sot.md`
//! No CELIR product path. No `gix`. Single release binary.
//!
//! Dispatch: PersonalityResolver → Registry / OverlapFacade → layers.
//! Git porcelain remains Claimed-path via factory::git_dispatch.

mod cmd;
mod error;
mod factory;
mod reflog;
mod util;
mod write_tree;

use std::process::ExitCode;

fn main() -> ExitCode {
    factory::dispatch_argv()
}
