//! Forge client leaves for the C8 repo/issue rows ([[sot:root/arks-hub-repo-issue-depth]]).
//!
//! Native HTTP + in-process arks Git only; never spawns git/gh (rule 30).

mod common;
pub mod issue_artifact;
pub mod issue_develop;
mod local_git;
pub mod repo_catalog;
pub mod repo_clone_sync;
pub mod repo_contents;
pub mod repo_keys_links;
