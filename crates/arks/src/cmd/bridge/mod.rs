//! External tool bridges: commands that hand work to programs the user
//! configures (diff / merge tools, filter scripts, mail transports, web
//! browsers, password prompts) the way git documents, never to `git` itself.
//!
//! - `difftool` / `difftool--helper`, `mergetool` — tool selection in `tools`
//! - `proc` — `sh -c` launching, PATH lookup, prompts, temporary directories

pub mod difftool;
pub mod mergetool;
pub mod proc;
pub mod tools;

pub use difftool::{cmd_difftool, cmd_difftool_helper};
pub use mergetool::cmd_mergetool;
