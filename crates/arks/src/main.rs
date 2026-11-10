//! `arks` — Wave L+H+Pa+Pb+Pc + Sv CLI.
//!
//! M9 = CLI-parity production (client). M10 leases via `arks-lease`. M11
//! production-server via Wave Sv (`upload-pack` / `receive-pack` / `serve-http`).
//! Cite: `.spec/sot/arks.sot.md`, `.spec/sot/cli-surface.sot.md`
//! No CELIR product path. No `gix`. No forge/host-env/LFS product CLI (trimmed; not git).

mod cmd;
mod error;
mod reflog;
mod util;
mod write_tree;

use error::{CliError, CliResult};
use std::env;
use std::process::ExitCode;

const HELP: &str = "\
arks — Wave L+H+Pa+Pb+Pc + Sv

Usage:
  arks [--version] [--help] <command> [<args>]

Commands:
  version       Print version string
  init          Create a repository
  hash-object   Compute object ID [-w]
  cat-file      Show object (-t | -s | -p)
  update-ref    Update or delete a ref [-d]
  rev-parse     Resolve revisions
  add           Stage file paths
  status        Show working-tree / index / HEAD summary
  lock          Inspect / clear stale Git *.lock files
  lease         Grant / release / inspect durable product leases (M13)
  commit        Record index as a commit (-m; -S SSH sign)
  verify-commit Verify SSH gpgsig on a commit
  log           Show first-parent history
  show          Show commit or object
  remote        Add, list, or remove remotes
  fetch         Download objects/refs (smart HTTP)
  push          Upload refs/objects (smart HTTP)
  clone         Clone a smart HTTP repository
  pull          Fetch + fast-forward merge, or --rebase
  branch        List / create / delete / rename branches
  checkout      Switch branches or restore paths
  switch        Switch branches
  restore       Restore working-tree / index paths
  reset         Reset HEAD (--soft/--mixed/--hard)
  diff          Show changes
  merge         Fast-forward merge [--ff-only]
  rm            Remove from index / working tree
  mv            Move / rename
  clean         Remove untracked files (-f) / dirs (-d)
  tag           Lightweight or annotated (-a -m) tags
  describe      Describe HEAD relative to tags
  shortlog      Author summary
  config        Local repository config
  credential    Fill / approve / reject credentials (plumbing)
  credential-wincred|libsecret|osxkeychain  First-party OS wallet helpers
  grep          Search tracked files
  stash         Stash push/list/pop/show/apply/drop/clear (-p FS3)
  cherry-pick   Apply commit or A..B (--continue/--abort)
  revert        Revert commit or A..B (clean inverse)
  rebase        Replay onto upstream (non-interactive)
  format-patch  Write numbered .patch files (-1 or A..B)
  am            Apply one format-patch file
  bisect        start / bad / good / reset / run
  bundle        create / list-heads / unbundle / verify
  archive       Tar archive of a tree (--format=tar)
  reflog        Show ref logs (show)
  replace       Create/list/delete refs/replace (FS5)
  worktree      Linked worktrees (add/list/remove/prune/move)
  submodule     Submodules (add/init/update/status/deinit)
  notes         Notes add/show/append/remove/list
  sparse-checkout  Cone init/set/list/disable/reapply
  blame         Line attribution for a path
  annotate      Synonym for blame
  fsck          Connectivity check (reachable subset)
  count-objects Count loose/pack objects (-v)
  pack-refs     Pack refs (--all)
  repack        Pack reachable objects (-ad; maint:gc lease)
  gc            Lease-honest consolidation (no --prune=now)
  maintenance   run (same as gc consolidation)
  range-diff    Compare two commit ranges (A..B / A...B)
  write-tree    Create tree from index (plumbing)
  commit-tree   Create commit from tree (plumbing)
  ls-files      List index paths (plumbing)
  ls-tree       List tree entries (plumbing)
  symbolic-ref  Read/set symbolic refs (plumbing)
  merge-base    Common ancestor (plumbing)
  show-ref      List refs (plumbing)
  for-each-ref  Format refs (plumbing)
  rev-list      List commits (plumbing)
  update-index  Update index entries (plumbing)
  read-tree     Load tree into index (plumbing)
  diff-tree     Compare trees (plumbing)
  pack-objects  Pack OIDs from stdin (plumbing)
  index-pack    Build pack index (plumbing)
  check-ignore  Test gitignore paths (plumbing)
  name-rev      Name a revision (plumbing)
  upload-pack         Serve fetch/clone duplex (Wave Sv/Sv2)
  receive-pack        Serve push duplex (Wave Sv/Sv2)
  serve-http          Smart+dumb HTTP listener (Wave Sv/Sv2)
  http-backend        CGI-shaped HTTP (Wave Sv/Sv2)
  update-server-info  Refresh dumb HTTP ads (Wave Sv2)
  daemon              git:// TCP daemon (Wave Sv2)
";

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("error: {e}");
            ExitCode::from(1)
        }
    }
}

fn print_version() {
    // Sy-5 implement: version string (git-version). Cite git(1) / git-version.
    println!("arks version {}", env!("CARGO_PKG_VERSION"));
}

fn run() -> CliResult<()> {
    let mut args: Vec<String> = env::args().skip(1).collect();
    if args.is_empty()
        || args.first().map(String::as_str) == Some("--help")
        || args.first().map(String::as_str) == Some("-h")
    {
        print!("{HELP}");
        return Ok(());
    }
    if args.first().map(String::as_str) == Some("--version") {
        print_version();
        return Ok(());
    }

    let cmd = args.remove(0);
    if cmd.starts_with('-') {
        return Err(CliError::NotImplemented(format!(
            "unsupported global option: {cmd}"
        )));
    }

    match cmd.as_str() {
        "help" | "--help" | "-h" => {
            print!("{HELP}");
            Ok(())
        }
        "version" => {
            print_version();
            Ok(())
        }
        "init" => cmd::cmd_init(&args),
        "hash-object" => cmd::cmd_hash_object(&args),
        "cat-file" => cmd::cmd_cat_file(&args),
        "update-ref" => cmd::cmd_update_ref(&args),
        "rev-parse" => cmd::cmd_rev_parse(&args),
        "add" => cmd::cmd_add(&args),
        "status" => cmd::cmd_status(&args),
        "lock" => cmd::cmd_lock(&args),
        "lease" => cmd::cmd_lease(&args),
        "commit" => cmd::cmd_commit(&args),
        "verify-commit" => cmd::cmd_verify_commit(&args),
        "log" => cmd::cmd_log(&args),
        "show" => cmd::cmd_show(&args),
        "remote" => cmd::cmd_remote(&args),
        "fetch" => cmd::cmd_fetch(&args),
        "push" => cmd::cmd_push(&args),
        "clone" => cmd::cmd_clone(&args),
        "pull" => cmd::cmd_pull(&args),
        "branch" => cmd::cmd_branch(&args),
        "checkout" => cmd::cmd_checkout(&args),
        "switch" => cmd::cmd_switch(&args),
        "restore" => cmd::cmd_restore(&args),
        "reset" => cmd::cmd_reset(&args),
        "diff" => cmd::cmd_diff(&args),
        "merge" => cmd::cmd_merge(&args),
        "rm" => cmd::cmd_rm(&args),
        "mv" => cmd::cmd_mv(&args),
        "clean" => cmd::cmd_clean(&args),
        "tag" => cmd::cmd_tag(&args),
        "describe" => cmd::cmd_describe(&args),
        "shortlog" => cmd::cmd_shortlog(&args),
        "config" => cmd::cmd_config(&args),
        "credential" => cmd::cmd_credential(&args),
        "credential-wincred" => cmd::cmd_credential_wincred(&args),
        "credential-libsecret" => cmd::cmd_credential_libsecret(&args),
        "credential-osxkeychain" => cmd::cmd_credential_osxkeychain(&args),
        "grep" => cmd::cmd_grep(&args),
        "stash" => cmd::cmd_stash(&args),
        "cherry-pick" => cmd::cmd_cherry_pick(&args),
        "revert" => cmd::cmd_revert(&args),
        "rebase" => cmd::cmd_rebase(&args),
        "format-patch" => cmd::cmd_format_patch(&args),
        "am" => cmd::cmd_am(&args),
        "bisect" => cmd::cmd_bisect(&args),
        "bundle" => cmd::cmd_bundle(&args),
        "archive" => cmd::cmd_archive(&args),
        "reflog" => cmd::cmd_reflog(&args),
        "replace" => cmd::cmd_replace(&args),
        "worktree" => cmd::cmd_worktree(&args),
        "submodule" => cmd::cmd_submodule(&args),
        "write-tree" => cmd::cmd_write_tree(&args),
        "commit-tree" => cmd::cmd_commit_tree(&args),
        "ls-files" => cmd::cmd_ls_files(&args),
        "ls-tree" => cmd::cmd_ls_tree(&args),
        "symbolic-ref" => cmd::cmd_symbolic_ref(&args),
        "merge-base" => cmd::cmd_merge_base(&args),
        "show-ref" => cmd::cmd_show_ref(&args),
        "for-each-ref" => cmd::cmd_for_each_ref(&args),
        "rev-list" => cmd::cmd_rev_list(&args),
        "update-index" => cmd::cmd_update_index(&args),
        "checkout-index" => cmd::cmd_checkout_index(&args),
        "read-tree" => cmd::cmd_read_tree(&args),
        "diff-tree" => cmd::cmd_diff_tree(&args),
        "pack-objects" => cmd::cmd_pack_objects(&args),
        "index-pack" => cmd::cmd_index_pack(&args),
        "check-ignore" => cmd::cmd_check_ignore(&args),
        "name-rev" => cmd::cmd_name_rev(&args),
        "upload-pack" => cmd::cmd_upload_pack(&args),
        "receive-pack" => cmd::cmd_receive_pack(&args),
        "serve-http" => cmd::cmd_serve_http(&args),
        "http-backend" => cmd::cmd_http_backend(&args),
        "update-server-info" => cmd::cmd_update_server_info(&args),
        "daemon" => cmd::cmd_daemon(&args),
        "gc" => cmd::cmd_gc(&args),
        "repack" => cmd::cmd_repack(&args),
        "maintenance" => cmd::cmd_maintenance(&args),
        "sparse-checkout" => cmd::cmd_sparse_checkout(&args),
        "notes" => cmd::cmd_notes(&args),
        "range-diff" => cmd::cmd_range_diff(&args),
        "blame" => cmd::cmd_blame(&args),
        "annotate" => cmd::cmd_annotate(&args),
        "fsck" => cmd::cmd_fsck(&args),
        "count-objects" => cmd::cmd_count_objects(&args),
        "pack-refs" => cmd::cmd_pack_refs(&args),
        "verify-tag" => cmd::cmd_verify_tag(&args),
        "backfill" | "filter-branch" => Err(CliError::NotImplemented(format!(
            "{cmd} excluded from alias inventory (M15 Fs-3 explicit exclude); see cli-flag-parity-full"
        ))),
        // FS7 closest: GUI tools are external; do not embed Tcl/Tk.
        "gui" | "citool" | "gitk" => Err(CliError::NotImplemented(format!(
            "{cmd}: use external gitk/git-gui (FS7 handoff; not embedded in arks CLI)"
        ))),
        // FS6 note: scalar is a separate enlistment helper — not core porcelain.
        "scalar" => Err(CliError::NotImplemented(
            "scalar: separate large-repo helper (FS6); not claimed as arks porcelain".into(),
        )),
        // Trimmed over-design (not stock git / git-daemon): forge, host-env, in-process lfs.
        "forge" | "host-env" | "lfs" => Err(CliError::NotImplemented(format!(
            "{cmd}: trimmed from product CLI (not git/git-daemon); use host gh / git-lfs externally if needed"
        ))),
        other => Err(CliError::NotImplemented(format!(
            "command not implemented: {other}"
        ))),
    }
}
