//! Existing git porcelain/plumbing match — preserved wiring for Claimed gates.
//! Invoked by the Registry GitPorcelainLayer (and OverlapFacade git rank-0).
//!
//! Inventory matches Spec tip `c5f977a` CLI surface; later matrix leaves land
//! in separate commits (not factory skeleton).

use crate::cmd;
use crate::error::{CliError, CliResult};

/// Dispatch a git-owned leaf through the historical command match.
/// Does **not** own hub/agent/flat-forge early exits — those stay in the Registry.
pub fn dispatch_git_leaf(cmd: &str, args: &[String]) -> CliResult<()> {
    match cmd {
        "help" | "--help" | "-h" => Ok(()),
        "version" => {
            println!("arks version {}", env!("CARGO_PKG_VERSION"));
            Ok(())
        }
        "init" | "init-db" => cmd::cmd_init(args),
        "hash-object" => cmd::cmd_hash_object(args),
        "cat-file" => cmd::cmd_cat_file(args),
        "update-ref" => cmd::cmd_update_ref(args),
        "rev-parse" => cmd::cmd_rev_parse(args),
        "add" | "stage" => cmd::cmd_add(args),
        "status" => cmd::cmd_status(args),
        "lock" => cmd::cmd_lock(args),
        "lease" => cmd::cmd_lease(args),
        "commit" => cmd::cmd_commit(args),
        "verify-commit" => cmd::cmd_verify_commit(args),
        "log" | "whatchanged" => cmd::cmd_log(args),
        "show" => cmd::cmd_show(args),
        "apply" => cmd::cmd_apply(args),
        "ls-remote" => cmd::cmd_ls_remote(args),
        "prune" => cmd::cmd_prune(args),
        "prune-packed" => cmd::cmd_prune_packed(args),
        "cherry" => cmd::cmd_cherry(args),
        "check-ref-format" => cmd::cmd_check_ref_format(args),
        "stripspace" => cmd::cmd_stripspace(args),
        "mktag" => cmd::cmd_mktag(args),
        "mktree" => cmd::cmd_mktree(args),
        "diff-files" => cmd::cmd_diff_files(args),
        "diff-index" => cmd::cmd_diff_index(args),
        "var" => cmd::cmd_var(args),
        "patch-id" => cmd::cmd_patch_id(args),
        "remote" => cmd::cmd_remote(args),
        "fetch" => cmd::cmd_fetch(args),
        "push" => cmd::cmd_push(args),
        "clone" => cmd::cmd_clone(args),
        "pull" => cmd::cmd_pull(args),
        "branch" => cmd::cmd_branch(args),
        "checkout" => cmd::cmd_checkout(args),
        "switch" => cmd::cmd_switch(args),
        "restore" => cmd::cmd_restore(args),
        "reset" => cmd::cmd_reset(args),
        "diff" => cmd::cmd_diff(args),
        "merge" => cmd::cmd_merge(args),
        "rm" => cmd::cmd_rm(args),
        "mv" => cmd::cmd_mv(args),
        "clean" => cmd::cmd_clean(args),
        "tag" => cmd::cmd_tag(args),
        "describe" => cmd::cmd_describe(args),
        "shortlog" => cmd::cmd_shortlog(args),
        "config" => cmd::cmd_config(args),
        "credential" => cmd::cmd_credential(args),
        "credential-wincred" => cmd::cmd_credential_wincred(args),
        "credential-libsecret" => cmd::cmd_credential_libsecret(args),
        "credential-osxkeychain" => cmd::cmd_credential_osxkeychain(args),
        "grep" => cmd::cmd_grep(args),
        "stash" => cmd::cmd_stash(args),
        "cherry-pick" => cmd::cmd_cherry_pick(args),
        "revert" => cmd::cmd_revert(args),
        "rebase" => cmd::cmd_rebase(args),
        "format-patch" => cmd::cmd_format_patch(args),
        "am" => cmd::cmd_am(args),
        "bisect" => cmd::cmd_bisect(args),
        "bundle" => cmd::cmd_bundle(args),
        "archive" => cmd::cmd_archive(args),
        "reflog" => cmd::cmd_reflog(args),
        "replace" => cmd::cmd_replace(args),
        "worktree" => cmd::cmd_worktree(args),
        "submodule" => cmd::cmd_submodule(args),
        "write-tree" => cmd::cmd_write_tree(args),
        "commit-tree" => cmd::cmd_commit_tree(args),
        "ls-files" => cmd::cmd_ls_files(args),
        "ls-tree" => cmd::cmd_ls_tree(args),
        "symbolic-ref" => cmd::cmd_symbolic_ref(args),
        "merge-base" => cmd::cmd_merge_base(args),
        "show-ref" => cmd::cmd_show_ref(args),
        "for-each-ref" => cmd::cmd_for_each_ref(args),
        "rev-list" => cmd::cmd_rev_list(args),
        "update-index" => cmd::cmd_update_index(args),
        "checkout-index" => cmd::cmd_checkout_index(args),
        "read-tree" => cmd::cmd_read_tree(args),
        "diff-tree" => cmd::cmd_diff_tree(args),
        "pack-objects" => cmd::cmd_pack_objects(args),
        "index-pack" => cmd::cmd_index_pack(args),
        "check-ignore" => cmd::cmd_check_ignore(args),
        "name-rev" => cmd::cmd_name_rev(args),
        "upload-pack" => cmd::cmd_upload_pack(args),
        "receive-pack" => cmd::cmd_receive_pack(args),
        "serve-http" => cmd::cmd_serve_http(args),
        "http-backend" => cmd::cmd_http_backend(args),
        "update-server-info" => cmd::cmd_update_server_info(args),
        "daemon" => cmd::cmd_daemon(args),
        "gc" => cmd::cmd_gc(args),
        "repack" => cmd::cmd_repack(args),
        "maintenance" => cmd::cmd_maintenance(args),
        "sparse-checkout" => cmd::cmd_sparse_checkout(args),
        "notes" => cmd::cmd_notes(args),
        "range-diff" => cmd::cmd_range_diff(args),
        "blame" => cmd::cmd_blame(args),
        "annotate" => cmd::cmd_annotate(args),
        "fsck" | "fsck-objects" => cmd::cmd_fsck(args),
        "count-objects" => cmd::cmd_count_objects(args),
        "pack-refs" => cmd::cmd_pack_refs(args),
        "verify-tag" => cmd::cmd_verify_tag(args),
        "backfill" | "filter-branch" => Err(CliError::NotImplemented(format!(
            "{cmd} excluded from alias inventory (M15 Fs-3 explicit exclude); see cli-flag-parity-full"
        ))),
        "gui" | "citool" | "gitk" => Err(CliError::NotImplemented(format!(
            "{cmd}: use external gitk/git-gui (FS7 handoff; not embedded in arks CLI)"
        ))),
        "scalar" => Err(CliError::NotImplemented(
            "scalar: separate large-repo helper (FS6); not claimed as arks porcelain".into(),
        )),
        "forge" | "host-env" | "lfs" => Err(CliError::NotImplemented(format!(
            "{cmd}: trimmed from product CLI (not git/git-daemon); arks never invokes git/gh — use external tools outside arks if needed"
        ))),
        other => Err(CliError::NotImplemented(format!(
            "command not implemented: {other}"
        ))),
    }
}

/// Flat forge client leaves (gh-shaped) — exclusive under Registry (not overlap).
/// Bodies are Partial stubs until a later forge-leaf commit wires `arks-hub`.
pub fn is_exclusive_forge_leaf(cmd: &str) -> bool {
    matches!(
        cmd,
        "auth"
            | "pr"
            | "issue"
            | "release"
            | "label"
            | "run"
            | "workflow"
            | "search"
            | "api"
            | "org"
            | "alias"
            | "completion"
            | "co"
            | "browse"
            | "gist"
            | "discussion"
            | "project"
            | "milestone"
            | "secret"
            | "variable"
            | "cache"
            | "ruleset"
            | "skill"
            | "agent-task"
            | "preview"
            | "licenses"
            | "attestation"
            | "codespace"
            | "ssh-key"
            | "gpg-key"
            | "extension"
            | "copilot"
            | "inbox"
            | "forge-config"
    )
}

pub fn is_overlap_leaf(cmd: &str) -> bool {
    matches!(cmd, "status" | "config" | "repo")
}

pub fn is_serve_leaf(cmd: &str) -> bool {
    // `serve` umbrella lands in a later clap-stub commit; tip surface is serve-http.
    matches!(cmd, "serve-http")
}
