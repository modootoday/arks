//! External tool bridges: commands that hand work to programs the user
//! configures (diff / merge tools, filter scripts, mail transports, web
//! browsers, password prompts) the way git documents, never to `git` itself.
//!
//! - `difftool` / `difftool--helper`, `mergetool` — tool selection in `tools`
//! - `proc` — `sh -c` launching, PATH lookup, prompts, temporary directories

pub mod askpass;
pub mod difftool;
pub mod filter_branch;
pub mod imap_send;
pub mod instaweb;
pub mod mergetool;
pub mod net;
pub mod proc;
pub mod quiltimport;
pub mod send_email;
pub mod tools;
pub mod web_browse;

pub use askpass::{cmd_askpass, cmd_askyesno};
pub use difftool::{cmd_difftool, cmd_difftool_helper};
pub use filter_branch::cmd_filter_branch;
pub use imap_send::cmd_imap_send;
pub use instaweb::cmd_instaweb;
pub use mergetool::cmd_mergetool;
pub use quiltimport::cmd_quiltimport;
pub use send_email::cmd_send_email;
pub use web_browse::cmd_web_browse;
