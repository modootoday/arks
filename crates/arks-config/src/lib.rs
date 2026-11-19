//! Git config read-side for arks.
//!
//! Parses git-config files and merges them in stock precedence order
//! (system → global → repository → `GIT_CONFIG_COUNT` env → `GIT_CONFIG_PARAMETERS`),
//! last value wins. Also resolves author / committer identity.
//!
//! Primary: <https://git-scm.com/docs/git-config> (§SYNTAX, §FILES, §ENVIRONMENT),
//! <https://git-scm.com/docs/git-commit-tree> (§COMMIT INFORMATION).
//! Cite: `.spec/sot/cloud-agent-dropin.sot.md` §C.
//!
//! Partial: `includeIf` conditions are skipped (unconditional `include.path` is honored).

#![forbid(unsafe_code)]

mod ident;
mod parse;
mod set;

pub use ident::{Ident, IdentError, IdentRole, resolve_ident};
pub use parse::{ParseError, parse_config_text};
pub use set::{
    ConfigEntry, ConfigScope, ConfigSet, parse_bool, parse_config_parameters, push_cli_override,
};
