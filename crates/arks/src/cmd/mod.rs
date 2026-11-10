//! CLI command modules grouped by domain context (not wave letters).
//! Composition root: `main.rs` dispatches here; pure format/graph helpers live in library crates.

pub mod archive;
pub mod bisect;
pub mod branch;
pub mod bundle;
pub mod config;
pub mod core;
pub mod credential;
pub mod grep;
pub mod history;
pub mod hooks;
pub mod host_skew;
pub mod inspect;
pub mod interactive_patch;
pub mod lease;
pub mod lock;
pub mod maint;
pub mod matrix_cmds;
pub mod matrix_rest;
pub mod merge;
pub mod merge_engine;
pub mod notes;
pub mod patch;
pub mod plumbing;
pub mod range_diff;
pub mod remote;
pub mod replace;
pub mod replay;
pub mod server;
pub mod shared;
pub mod sign;
pub mod sparse_checkout;
pub mod stash;
pub mod submodule;
pub mod tag;
pub mod working_tree;
pub mod worktree;

pub use archive::{cmd_archive, encode_tag_payload, write_annotated_tag};
pub use bisect::cmd_bisect;
pub use branch::{cmd_branch, cmd_checkout, cmd_switch};
pub use bundle::cmd_bundle;
pub use config::cmd_config;
pub use core::{
    cmd_add, cmd_cat_file, cmd_commit, cmd_hash_object, cmd_init, cmd_log, cmd_rev_parse, cmd_show,
    cmd_status, cmd_update_ref,
};
pub use credential::{
    cmd_credential, cmd_credential_libsecret, cmd_credential_osxkeychain, cmd_credential_wincred,
};
pub use grep::cmd_grep;
pub use history::{cmd_reflog, cmd_shortlog};
pub use inspect::{cmd_annotate, cmd_blame, cmd_count_objects, cmd_fsck, cmd_pack_refs};
pub use lease::cmd_lease;
pub use lock::cmd_lock;
pub use maint::{cmd_gc, cmd_maintenance, cmd_repack};
pub use host_skew::{
    cmd_diff_pairs, cmd_format_rev, cmd_history, cmd_last_modified, cmd_refs, cmd_replay_exp,
    cmd_survey, cmd_url_parse,
};
pub use matrix_cmds::{
    cmd_apply, cmd_bugreport, cmd_check_attr, cmd_check_mailmap, cmd_check_ref_format, cmd_cherry,
    cmd_column, cmd_credential_store, cmd_diagnose, cmd_diff_files, cmd_diff_index,
    cmd_fmt_merge_msg, cmd_for_each_repo, cmd_get_tar_commit_id, cmd_hook, cmd_interpret_trailers,
    cmd_ls_remote, cmd_mailinfo, cmd_mailsplit, cmd_merge_file, cmd_merge_tree, cmd_mktag,
    cmd_mktree, cmd_pack_redundant, cmd_patch_id, cmd_prune, cmd_prune_packed, cmd_request_pull,
    cmd_shell, cmd_show_branch, cmd_show_index, cmd_stripspace, cmd_unpack_file, cmd_unpack_objects,
    cmd_upload_archive, cmd_var, cmd_verify_pack,
};
pub use matrix_rest::{
    cmd_commit_graph, cmd_fast_export, cmd_fast_import, cmd_fetch_pack, cmd_http_fetch,
    cmd_http_push, cmd_multi_pack_index, cmd_rerere, cmd_send_pack, cmd_subtree,
};
pub use merge::{cmd_merge, cmd_pull};
pub use notes::cmd_notes;
pub use patch::{cmd_am, cmd_format_patch};
pub use plumbing::{
    cmd_check_ignore, cmd_checkout_index, cmd_commit_tree, cmd_diff_tree, cmd_for_each_ref,
    cmd_index_pack, cmd_ls_files, cmd_ls_tree, cmd_merge_base, cmd_name_rev, cmd_pack_objects,
    cmd_read_tree, cmd_rev_list, cmd_show_ref, cmd_symbolic_ref, cmd_update_index, cmd_write_tree,
};
pub use range_diff::cmd_range_diff;
pub use remote::{cmd_clone, cmd_fetch, cmd_push, cmd_remote};
pub use replace::cmd_replace;
pub use replay::{cmd_cherry_pick, cmd_rebase, cmd_revert};
pub use server::{
    cmd_daemon, cmd_http_backend, cmd_receive_pack, cmd_serve, cmd_serve_http,
    cmd_update_server_info, cmd_upload_pack,
};
pub use sign::{cmd_verify_commit, cmd_verify_tag};
pub use sparse_checkout::cmd_sparse_checkout;
pub use stash::cmd_stash;
pub use submodule::cmd_submodule;
pub use tag::{cmd_describe, cmd_tag};
pub use working_tree::{cmd_clean, cmd_diff, cmd_mv, cmd_reset, cmd_restore, cmd_rm};
pub use worktree::cmd_worktree;
