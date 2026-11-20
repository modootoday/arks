//! CLI command modules grouped by domain context (not wave letters).
//! Composition root: `main.rs` dispatches here; pure format/graph helpers live in library crates.

pub mod am;
pub mod apply;
pub mod archive;
pub mod attr;
pub mod backfill;
pub mod bisect;
pub mod blame;
pub mod branch;
pub mod branch_cmd;
pub mod bre;
pub mod bridge;
pub mod bundle;
pub mod cat_file;
pub mod clone;
pub mod commit;
pub mod config;
pub mod core;
pub mod credential;
pub mod credential_cache;
pub mod credential_os;
pub mod daemon;
pub mod diff_cmd;
pub mod diff_render;
pub mod diffcore;
pub mod fetch;
pub mod for_each_ref;
pub mod format_patch;
pub mod grep;
pub mod gui;
pub mod history;
pub mod hooks;
pub mod host_skew;
pub mod ignore;
pub mod index_plumbing;
pub mod inspect;
pub mod interactive_patch;
pub mod internal_helpers;
pub mod lease;
pub mod lock;
pub mod log_cmd;
pub mod log_format;
pub mod log_graph;
pub mod ls_files;
pub mod ls_remote;
pub mod ls_tree;
pub mod mail;
pub mod mailmap;
pub mod maint;
pub mod matrix_cmds;
pub mod matrix_rest;
pub mod merge;
pub mod merge_base;
pub mod merge_engine;
pub mod merge_helpers;
pub mod merge_many;
pub mod merge_strategies;
pub mod merge_subtree;
pub mod notes;
pub mod opt_parse;
pub mod opt_spec;
pub mod optparse;
pub mod pack_transport;
pub mod patch;
pub mod patch_id;
pub mod pick;
pub mod plumbing;
pub mod plumbing_ext;
pub mod porcelain_opts;
pub mod pull;
pub mod push;
pub mod range_diff;
pub mod raw_diff;
pub mod rebase;
pub mod remote;
pub mod remote_admin;
pub mod remote_helpers;
pub mod remote_local;
pub mod replace;
pub mod replay;
pub mod request_pull;
pub mod rev_list;
pub mod revwalk;
pub mod server;
pub mod shared;
pub mod sign;
pub mod sparse_checkout;
pub mod stash;
pub mod status_report;
pub mod submodule;
pub mod switch_tree;
pub mod tag;
pub mod trailers;
pub mod unified_diff;
pub mod update_check;
pub mod working_tree;
pub mod worktree;

pub use am::cmd_am;
pub use apply::cmd_apply;
pub use archive::{cmd_archive, encode_tag_payload, write_annotated_tag};
pub use backfill::cmd_backfill;
pub use bisect::cmd_bisect;
pub use blame::{cmd_annotate, cmd_blame};
pub use branch::{cmd_checkout, cmd_switch};
pub use branch_cmd::cmd_branch;
pub use bridge::{
    cmd_askpass, cmd_askyesno, cmd_difftool, cmd_difftool_helper, cmd_filter_branch, cmd_imap_send,
    cmd_instaweb, cmd_mergetool, cmd_quiltimport, cmd_send_email, cmd_web_browse,
};
pub use bundle::cmd_bundle;
pub use cat_file::cmd_cat_file;
pub use clone::cmd_clone;
pub use commit::cmd_commit;
pub use config::cmd_config;
pub use core::{cmd_add, cmd_hash_object, cmd_init, cmd_rev_parse, cmd_status, cmd_update_ref};
pub use credential::{
    cmd_credential, cmd_credential_libsecret, cmd_credential_osxkeychain, cmd_credential_wincred,
};
pub use credential_cache::{
    cmd_credential_cache, cmd_credential_cache_daemon, cmd_credential_helper_selector,
};
pub use credential_os::cmd_credential_manager;
pub use daemon::cmd_daemon;
pub use diff_cmd::cmd_diff;
pub use fetch::cmd_fetch;
pub use for_each_ref::cmd_for_each_ref;
pub use format_patch::cmd_format_patch;
pub use grep::cmd_grep;
pub use gui::{cmd_citool, cmd_gui, cmd_gui_askpass, cmd_gui_askyesno};
pub use history::{cmd_reflog, cmd_shortlog};
pub use host_skew::{
    cmd_diff_pairs, cmd_format_rev, cmd_history, cmd_last_modified, cmd_refs, cmd_replay_exp,
    cmd_repo, cmd_survey, cmd_url_parse,
};
pub use inspect::{cmd_count_objects, cmd_fsck, cmd_pack_refs};
pub use internal_helpers::{
    cmd_add_interactive_helper, cmd_bisect_helper, cmd_checkout_worker, cmd_env_helper,
    cmd_fsmonitor_daemon, cmd_pickaxe, cmd_sh_i18n_envsubst, cmd_submodule_helper, cmd_update,
    cmd_upload_archive_writer,
};
pub use lease::cmd_lease;
pub use lock::cmd_lock;
pub use log_cmd::{cmd_log, cmd_show, cmd_whatchanged};
pub use ls_files::cmd_ls_files;
pub use ls_remote::cmd_ls_remote;
pub use ls_tree::cmd_ls_tree;
pub use mail::{cmd_mailinfo, cmd_mailsplit};
pub use maint::{cmd_gc, cmd_maintenance, cmd_repack};
pub use matrix_cmds::{
    cmd_bugreport, cmd_check_attr, cmd_check_mailmap, cmd_check_ref_format, cmd_cherry, cmd_column,
    cmd_credential_store, cmd_diagnose, cmd_fmt_merge_msg, cmd_for_each_repo,
    cmd_get_tar_commit_id, cmd_hook, cmd_merge_file, cmd_merge_tree, cmd_mktag, cmd_mktree,
    cmd_pack_redundant, cmd_prune, cmd_prune_packed, cmd_shell, cmd_show_branch, cmd_show_index,
    cmd_stripspace, cmd_unpack_file, cmd_unpack_objects, cmd_upload_archive, cmd_var,
    cmd_verify_pack,
};
pub use matrix_rest::{
    cmd_commit_graph, cmd_fast_export, cmd_fast_import, cmd_http_fetch, cmd_http_push,
    cmd_multi_pack_index, cmd_rerere, cmd_subtree,
};
pub use merge::cmd_merge;
pub use merge_base::cmd_merge_base;
pub use merge_helpers::{cmd_merge_index, cmd_merge_one_file};
pub use merge_strategies::{
    cmd_merge_octopus, cmd_merge_ours, cmd_merge_recursive, cmd_merge_recursive_ours,
    cmd_merge_recursive_theirs, cmd_merge_resolve, cmd_merge_subtree,
};
pub use notes::cmd_notes;
pub use pack_transport::{cmd_fetch_pack, cmd_send_pack};
pub use patch_id::cmd_patch_id;
pub use pick::{cmd_cherry_pick, cmd_revert};
pub use plumbing::{
    cmd_check_ignore, cmd_checkout_index, cmd_commit_tree, cmd_index_pack, cmd_name_rev,
    cmd_pack_objects, cmd_read_tree, cmd_show_ref, cmd_symbolic_ref, cmd_update_index,
    cmd_write_tree,
};
pub use pull::cmd_pull;
pub use push::cmd_push;
pub use range_diff::cmd_range_diff;
pub use raw_diff::{cmd_diff_files, cmd_diff_index, cmd_diff_tree};
pub use rebase::cmd_rebase;
pub use remote_admin::cmd_remote;
pub use remote_helpers::{cmd_remote_curl, cmd_remote_ext, cmd_remote_fd};
pub use replace::cmd_replace;
pub use request_pull::cmd_request_pull;
pub use rev_list::cmd_rev_list;
pub use server::{
    cmd_http_backend, cmd_receive_pack, cmd_serve, cmd_serve_http, cmd_update_server_info,
    cmd_upload_pack,
};
pub use sign::{cmd_verify_commit, cmd_verify_tag};
pub use sparse_checkout::cmd_sparse_checkout;
pub use stash::cmd_stash;
pub use submodule::cmd_submodule;
pub use tag::{cmd_describe, cmd_tag};
pub use trailers::cmd_interpret_trailers;
pub use update_check::cmd_update_git_for_windows;
pub use working_tree::{cmd_clean, cmd_mv, cmd_reset, cmd_restore, cmd_rm};
pub use worktree::cmd_worktree;
