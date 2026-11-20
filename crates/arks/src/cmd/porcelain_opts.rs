//! Option tables for the commit / branch / worktree porcelain family, one per
//! command (or command plus subcommand), transcribed from the option lists
//! stock git prints for `-h` (names, short letters, value kinds and negation
//! only; behaviour lives in each command). Observed on git 2.43, a superset of
//! the pin 2.39.5 lists.

use crate::cmd::opt_parse::{OptDef, Takes::*, o};

pub const ADD: &[OptDef] = &[
    o("dry-run", 'n', No, true),
    o("verbose", 'v', No, true),
    o("interactive", 'i', No, true),
    o("patch", 'p', No, true),
    o("edit", 'e', No, true),
    o("force", 'f', No, true),
    o("update", 'u', No, true),
    o("renormalize", ' ', No, true),
    o("intent-to-add", 'N', No, true),
    o("all", 'A', No, true),
    o("ignore-removal", ' ', No, true),
    o("refresh", ' ', No, true),
    o("ignore-errors", ' ', No, true),
    o("ignore-missing", ' ', No, true),
    o("sparse", ' ', No, true),
    o("chmod", ' ', Req, true),
    o("pathspec-from-file", ' ', Req, true),
    o("pathspec-file-nul", ' ', No, true),
];

pub const AM: &[OptDef] = &[
    o("interactive", 'i', No, true),
    o("no-verify", 'n', No, true),
    o("3way", '3', No, true),
    o("quiet", 'q', No, true),
    o("signoff", 's', No, true),
    o("utf8", 'u', No, true),
    o("keep", 'k', No, true),
    o("keep-non-patch", ' ', No, true),
    o("message-id", 'm', No, true),
    o("keep-cr", ' ', No, true),
    o("scissors", 'c', No, true),
    o("quoted-cr", ' ', Req, false),
    o("whitespace", ' ', Req, true),
    o("ignore-space-change", ' ', No, true),
    o("ignore-whitespace", ' ', No, true),
    o("directory", ' ', Req, true),
    o("exclude", ' ', Req, true),
    o("include", ' ', Req, true),
    o("-C", 'C', Req, false),
    o("-p", 'p', Req, false),
    o("patch-format", ' ', Req, true),
    o("reject", ' ', No, true),
    o("resolvemsg", ' ', No, true),
    o("continue", ' ', No, false),
    o("resolved", 'r', No, false),
    o("skip", ' ', No, false),
    o("abort", ' ', No, false),
    o("quit", ' ', No, false),
    o("show-current-patch", ' ', Opt, false),
    o("allow-empty", ' ', No, false),
    o("committer-date-is-author-date", ' ', No, true),
    o("ignore-date", ' ', No, true),
    o("rerere-autoupdate", ' ', No, true),
    o("gpg-sign", 'S', Opt, true),
    o("empty", ' ', Req, false),
];

pub const APPLY: &[OptDef] = &[
    o("exclude", ' ', Req, false),
    o("include", ' ', Req, false),
    o("-p", 'p', Req, false),
    o("no-add", ' ', No, true),
    o("stat", ' ', No, true),
    o("numstat", ' ', No, true),
    o("summary", ' ', No, true),
    o("check", ' ', No, true),
    o("index", ' ', No, true),
    o("intent-to-add", 'N', No, true),
    o("cached", ' ', No, true),
    o("unsafe-paths", ' ', No, true),
    o("apply", ' ', No, true),
    o("3way", '3', No, true),
    o("build-fake-ancestor", ' ', Req, true),
    o("-z", 'z', No, false),
    o("-C", 'C', Req, false),
    o("whitespace", ' ', Req, true),
    o("ignore-space-change", ' ', No, true),
    o("ignore-whitespace", ' ', No, true),
    o("reverse", 'R', No, true),
    o("unidiff-zero", ' ', No, true),
    o("reject", ' ', No, true),
    o("allow-overlap", ' ', No, true),
    o("verbose", 'v', No, true),
    o("quiet", 'q', No, true),
    o("inaccurate-eof", ' ', No, true),
    o("recount", ' ', No, true),
    o("directory", ' ', Req, true),
    o("allow-empty", ' ', No, true),
];

pub const BRANCH: &[OptDef] = &[
    o("verbose", 'v', No, true),
    o("quiet", 'q', No, true),
    o("track", 't', Opt, true),
    o("set-upstream-to", 'u', Req, true),
    o("unset-upstream", ' ', No, true),
    o("color", ' ', Opt, true),
    o("remotes", 'r', No, false),
    o("contains", ' ', Req, false),
    o("no-contains", ' ', Req, false),
    o("abbrev", ' ', Opt, true),
    o("all", 'a', No, false),
    o("delete", 'd', No, true),
    o("-D", 'D', No, false),
    o("move", 'm', No, true),
    o("-M", 'M', No, false),
    o("omit-empty", ' ', No, true),
    o("copy", 'c', No, true),
    o("-C", 'C', No, false),
    o("list", 'l', No, true),
    o("show-current", ' ', No, true),
    o("create-reflog", ' ', No, true),
    o("edit-description", ' ', No, true),
    o("force", 'f', No, true),
    o("merged", ' ', Req, false),
    o("no-merged", ' ', Req, false),
    o("column", ' ', Opt, true),
    o("sort", ' ', Req, true),
    o("points-at", ' ', Req, true),
    o("ignore-case", 'i', No, true),
    o("recurse-submodules", ' ', No, true),
    o("format", ' ', Req, true),
];

pub const CHECKOUT: &[OptDef] = &[
    o("-b", 'b', Req, false),
    o("-B", 'B', Req, false),
    o("-l", 'l', No, false),
    o("guess", ' ', No, true),
    o("overlay", ' ', No, true),
    o("quiet", 'q', No, true),
    o("recurse-submodules", ' ', Opt, true),
    o("progress", ' ', No, true),
    o("merge", 'm', No, true),
    o("conflict", ' ', Req, true),
    o("detach", 'd', No, true),
    o("track", 't', Opt, true),
    o("force", 'f', No, true),
    o("orphan", ' ', Req, true),
    o("overwrite-ignore", ' ', No, true),
    o("ignore-other-worktrees", ' ', No, true),
    o("ours", '2', No, false),
    o("theirs", '3', No, false),
    o("patch", 'p', No, true),
    o("ignore-skip-worktree-bits", ' ', No, true),
    o("pathspec-from-file", ' ', Req, true),
    o("pathspec-file-nul", ' ', No, true),
];

pub const CHERRY_PICK: &[OptDef] = &[
    o("quit", ' ', No, false),
    o("continue", ' ', No, false),
    o("abort", ' ', No, false),
    o("skip", ' ', No, false),
    o("cleanup", ' ', Req, true),
    o("no-commit", 'n', No, true),
    o("edit", 'e', No, true),
    o("signoff", 's', No, true),
    o("mainline", 'm', Req, true),
    o("rerere-autoupdate", ' ', No, true),
    o("strategy", ' ', Req, true),
    o("strategy-option", 'X', Req, true),
    o("gpg-sign", 'S', Opt, true),
    o("-x", 'x', No, false),
    o("ff", ' ', No, true),
    o("allow-empty", ' ', No, true),
    o("allow-empty-message", ' ', No, true),
    o("keep-redundant-commits", ' ', No, true),
];

pub const CLEAN: &[OptDef] = &[
    o("quiet", 'q', No, true),
    o("dry-run", 'n', No, true),
    o("force", 'f', No, true),
    o("interactive", 'i', No, true),
    o("-d", 'd', No, false),
    o("exclude", 'e', Req, false),
    o("-x", 'x', No, false),
    o("-X", 'X', No, false),
];

pub const COMMIT: &[OptDef] = &[
    o("quiet", 'q', No, true),
    o("verbose", 'v', No, true),
    o("file", 'F', Req, true),
    o("author", ' ', Req, true),
    o("date", ' ', Req, true),
    o("message", 'm', Req, true),
    o("reedit-message", 'c', Req, true),
    o("reuse-message", 'C', Req, true),
    o("fixup", ' ', Req, true),
    o("squash", ' ', Req, true),
    o("reset-author", ' ', No, true),
    o("trailer", ' ', Req, false),
    o("signoff", 's', No, true),
    o("template", 't', Req, true),
    o("edit", 'e', No, true),
    o("cleanup", ' ', Req, true),
    o("status", ' ', No, true),
    o("gpg-sign", 'S', Opt, true),
    o("all", 'a', No, true),
    o("include", 'i', No, true),
    o("interactive", ' ', No, true),
    o("patch", 'p', No, true),
    o("only", 'o', No, true),
    o("no-verify", 'n', No, true),
    o("dry-run", ' ', No, true),
    o("short", ' ', No, true),
    o("branch", ' ', No, true),
    o("ahead-behind", ' ', No, true),
    o("porcelain", ' ', No, true),
    o("long", ' ', No, true),
    o("null", 'z', No, true),
    o("amend", ' ', No, true),
    o("no-post-rewrite", ' ', No, true),
    o("untracked-files", 'u', Opt, true),
    o("pathspec-from-file", ' ', Req, true),
    o("pathspec-file-nul", ' ', No, true),
    // Hidden from `-h` but accepted (git-commit manual).
    o("allow-empty", ' ', No, true),
    o("allow-empty-message", ' ', No, true),
];

pub const FORMAT_PATCH: &[OptDef] = &[
    o("numbered", 'n', No, true),
    o("no-numbered", 'N', No, false),
    o("signoff", 's', No, true),
    o("stdout", ' ', No, true),
    o("cover-letter", ' ', No, true),
    o("numbered-files", ' ', No, true),
    o("suffix", ' ', Req, true),
    o("start-number", ' ', Req, true),
    o("reroll-count", 'v', Req, true),
    o("filename-max-length", ' ', Req, true),
    o("rfc", ' ', No, true),
    o("cover-from-description", ' ', Req, true),
    o("description-file", ' ', Req, true),
    o("subject-prefix", ' ', Req, false),
    o("output-directory", 'o', Req, false),
    o("keep-subject", 'k', No, false),
    o("no-binary", ' ', No, true),
    o("zero-commit", ' ', No, true),
    o("ignore-if-in-upstream", ' ', No, true),
    o("no-stat", 'p', No, false),
    o("add-header", ' ', Req, true),
    o("to", ' ', Req, true),
    o("cc", ' ', Req, true),
    o("from", ' ', Opt, true),
    o("in-reply-to", ' ', Req, true),
    o("attach", ' ', Opt, true),
    o("inline", ' ', Opt, false),
    o("thread", ' ', Opt, true),
    o("signature", ' ', Req, true),
    o("base", ' ', Req, true),
    o("signature-file", ' ', Req, true),
    o("quiet", 'q', No, true),
    o("progress", ' ', No, true),
    o("interdiff", ' ', Req, true),
    o("range-diff", ' ', Req, true),
    o("creation-factor", ' ', Req, true),
    o("force-in-body-from", ' ', No, true),
];

pub const INIT: &[OptDef] = &[
    o("template", ' ', Req, true),
    o("bare", ' ', No, true),
    o("shared", ' ', Opt, false),
    o("quiet", 'q', No, true),
    o("separate-git-dir", ' ', Req, true),
    o("initial-branch", 'b', Req, true),
    o("object-format", ' ', Req, true),
];

pub const INTERPRET_TRAILERS: &[OptDef] = &[
    o("in-place", ' ', No, true),
    o("trim-empty", ' ', No, true),
    o("where", ' ', Req, true),
    o("if-exists", ' ', Req, true),
    o("if-missing", ' ', Req, true),
    o("only-trailers", ' ', No, true),
    o("only-input", ' ', No, true),
    o("unfold", ' ', No, true),
    o("parse", ' ', No, false),
    o("no-divider", ' ', No, true),
    o("trailer", ' ', Req, true),
];

pub const MAILINFO: &[OptDef] = &[
    o("-k", 'k', No, false),
    o("-b", 'b', No, false),
    o("message-id", 'm', No, true),
    o("-u", 'u', No, false),
    o("-n", 'n', No, false),
    o("encoding", ' ', Req, false),
    o("scissors", ' ', No, true),
    o("quoted-cr", ' ', Req, false),
];

pub const MERGE: &[OptDef] = &[
    o("-n", 'n', No, false),
    o("stat", ' ', No, true),
    o("summary", ' ', No, true),
    o("log", ' ', Opt, true),
    o("squash", ' ', No, true),
    o("commit", ' ', No, true),
    o("edit", 'e', No, true),
    o("cleanup", ' ', Req, true),
    o("ff", ' ', No, true),
    o("ff-only", ' ', No, false),
    o("rerere-autoupdate", ' ', No, true),
    o("verify-signatures", ' ', No, true),
    o("strategy", 's', Req, true),
    o("strategy-option", 'X', Req, true),
    o("message", 'm', Req, true),
    o("file", 'F', Req, false),
    o("into-name", ' ', Req, true),
    o("verbose", 'v', No, true),
    o("quiet", 'q', No, true),
    o("abort", ' ', No, true),
    o("quit", ' ', No, true),
    o("continue", ' ', No, true),
    o("allow-unrelated-histories", ' ', No, true),
    o("progress", ' ', No, true),
    o("gpg-sign", 'S', Opt, true),
    o("autostash", ' ', No, true),
    o("overwrite-ignore", ' ', No, true),
    o("signoff", ' ', No, true),
    o("no-verify", ' ', No, true),
];

pub const MV: &[OptDef] = &[
    o("verbose", 'v', No, true),
    o("dry-run", 'n', No, true),
    o("force", 'f', No, true),
    o("-k", 'k', No, false),
    o("sparse", ' ', No, true),
];

pub const NOTES: &[OptDef] = &[o("ref", ' ', Req, true)];

pub const NOTES__ADD: &[OptDef] = &[
    o("message", 'm', Req, false),
    o("file", 'F', Req, false),
    o("reedit-message", 'c', Req, false),
    o("reuse-message", 'C', Req, false),
    o("allow-empty", ' ', No, true),
    o("force", 'f', No, true),
    o("separator", ' ', Opt, true),
    o("stripspace", ' ', No, true),
];

pub const NOTES__APPEND: &[OptDef] = &[
    o("message", 'm', Req, false),
    o("file", 'F', Req, false),
    o("reedit-message", 'c', Req, false),
    o("reuse-message", 'C', Req, false),
    o("allow-empty", ' ', No, true),
    o("separator", ' ', Opt, true),
    o("stripspace", ' ', No, true),
];

pub const NOTES__COPY: &[OptDef] = &[
    o("force", 'f', No, true),
    o("stdin", ' ', No, true),
    o("for-rewrite", ' ', Req, true),
];

pub const NOTES__EDIT: &[OptDef] = &[
    o("message", 'm', Req, false),
    o("file", 'F', Req, false),
    o("reedit-message", 'c', Req, false),
    o("reuse-message", 'C', Req, false),
    o("allow-empty", ' ', No, true),
    o("separator", ' ', Opt, true),
    o("stripspace", ' ', No, true),
];

pub const NOTES__MERGE: &[OptDef] = &[
    o("verbose", 'v', No, true),
    o("quiet", 'q', No, true),
    o("strategy", 's', Req, true),
    o("commit", ' ', No, false),
    o("abort", ' ', No, false),
];

pub const NOTES__PRUNE: &[OptDef] = &[o("dry-run", 'n', No, true), o("verbose", 'v', No, true)];

pub const NOTES__REMOVE: &[OptDef] = &[
    o("ignore-missing", ' ', No, true),
    o("stdin", ' ', No, true),
];

pub const REBASE: &[OptDef] = &[
    o("onto", ' ', Req, true),
    o("keep-base", ' ', No, true),
    o("no-verify", ' ', No, true),
    o("quiet", 'q', No, true),
    o("verbose", 'v', No, true),
    o("no-stat", 'n', No, true),
    o("signoff", ' ', No, true),
    o("committer-date-is-author-date", ' ', No, true),
    o("reset-author-date", ' ', No, true),
    o("-C", 'C', Req, false),
    o("ignore-whitespace", ' ', No, true),
    o("whitespace", ' ', Req, true),
    o("force-rebase", 'f', No, true),
    o("no-ff", ' ', No, true),
    o("continue", ' ', No, false),
    o("skip", ' ', No, false),
    o("abort", ' ', No, false),
    o("quit", ' ', No, false),
    o("edit-todo", ' ', No, false),
    o("show-current-patch", ' ', No, false),
    o("apply", ' ', No, false),
    o("merge", 'm', No, false),
    o("interactive", 'i', No, false),
    o("rerere-autoupdate", ' ', No, true),
    o("empty", ' ', Req, false),
    o("autosquash", ' ', No, true),
    o("update-refs", ' ', No, true),
    o("gpg-sign", 'S', Opt, true),
    o("autostash", ' ', No, true),
    o("exec", 'x', Req, true),
    o("rebase-merges", 'r', Opt, true),
    o("fork-point", ' ', No, true),
    o("strategy", 's', Req, true),
    o("strategy-option", 'X', Req, true),
    o("root", ' ', No, true),
    o("reschedule-failed-exec", ' ', No, true),
    o("reapply-cherry-picks", ' ', No, true),
    // Accepted by git though hidden from `-h`.
    o("ignore-date", ' ', No, true),
    o("keep-empty", ' ', No, true),
    o("allow-empty-message", ' ', No, true),
];

pub const RESET: &[OptDef] = &[
    o("quiet", 'q', No, true),
    o("no-refresh", ' ', No, true),
    o("mixed", ' ', No, false),
    o("soft", ' ', No, false),
    o("hard", ' ', No, false),
    o("merge", ' ', No, false),
    o("keep", ' ', No, false),
    o("recurse-submodules", ' ', Opt, true),
    o("patch", 'p', No, true),
    o("intent-to-add", 'N', No, true),
    o("pathspec-from-file", ' ', Req, true),
    o("pathspec-file-nul", ' ', No, true),
];

pub const RESTORE: &[OptDef] = &[
    o("source", 's', Req, true),
    o("staged", 'S', No, true),
    o("worktree", 'W', No, true),
    o("ignore-unmerged", ' ', No, true),
    o("overlay", ' ', No, true),
    o("quiet", 'q', No, true),
    o("recurse-submodules", ' ', Opt, true),
    o("progress", ' ', No, true),
    o("merge", 'm', No, true),
    o("conflict", ' ', Req, true),
    o("ours", '2', No, false),
    o("theirs", '3', No, false),
    o("patch", 'p', No, true),
    o("ignore-skip-worktree-bits", ' ', No, true),
    o("pathspec-from-file", ' ', Req, true),
    o("pathspec-file-nul", ' ', No, true),
];

pub const REVERT: &[OptDef] = &[
    o("quit", ' ', No, false),
    o("continue", ' ', No, false),
    o("abort", ' ', No, false),
    o("skip", ' ', No, false),
    o("cleanup", ' ', Req, true),
    o("no-commit", 'n', No, true),
    o("edit", 'e', No, true),
    o("signoff", 's', No, true),
    o("mainline", 'm', Req, true),
    o("rerere-autoupdate", ' ', No, true),
    o("strategy", ' ', Req, true),
    o("strategy-option", 'X', Req, true),
    o("gpg-sign", 'S', Opt, true),
    o("reference", ' ', No, true),
];

pub const RM: &[OptDef] = &[
    o("dry-run", 'n', No, true),
    o("quiet", 'q', No, true),
    o("cached", ' ', No, true),
    o("force", 'f', No, true),
    o("-r", 'r', No, false),
    o("ignore-unmatch", ' ', No, true),
    o("sparse", ' ', No, true),
    o("pathspec-from-file", ' ', Req, true),
    o("pathspec-file-nul", ' ', No, true),
];

pub const SPARSE_CHECKOUT__CHECK_RULES: &[OptDef] = &[
    o("-z", 'z', No, false),
    o("cone", ' ', No, true),
    o("rules-file", ' ', Req, true),
];

pub const SPARSE_CHECKOUT__INIT: &[OptDef] =
    &[o("cone", ' ', No, true), o("sparse-index", ' ', No, true)];

pub const SPARSE_CHECKOUT__SET: &[OptDef] = &[
    o("cone", ' ', No, true),
    o("sparse-index", ' ', No, true),
    o("skip-checks", ' ', No, false),
    o("stdin", ' ', No, false),
];

pub const STASH__APPLY: &[OptDef] = &[o("quiet", 'q', No, true), o("index", ' ', No, true)];

pub const STASH__DROP: &[OptDef] = &[o("quiet", 'q', No, true)];

pub const STASH__POP: &[OptDef] = &[o("quiet", 'q', No, true), o("index", ' ', No, true)];

pub const STASH__PUSH: &[OptDef] = &[
    o("keep-index", 'k', No, true),
    o("staged", 'S', No, true),
    o("patch", 'p', No, true),
    o("quiet", 'q', No, true),
    o("include-untracked", 'u', No, true),
    o("all", 'a', No, true),
    o("message", 'm', Req, true),
    o("pathspec-from-file", ' ', Req, true),
    o("pathspec-file-nul", ' ', No, true),
];

pub const STASH__SAVE: &[OptDef] = &[
    o("keep-index", 'k', No, true),
    o("staged", 'S', No, true),
    o("patch", 'p', No, true),
    o("quiet", 'q', No, true),
    o("include-untracked", 'u', No, true),
    o("all", 'a', No, true),
    o("message", 'm', Req, true),
];

pub const STASH__SHOW: &[OptDef] = &[
    o("include-untracked", 'u', No, true),
    o("only-untracked", ' ', No, false),
];

pub const STASH__STORE: &[OptDef] = &[o("quiet", 'q', No, true), o("message", 'm', Req, true)];

pub const STATUS: &[OptDef] = &[
    o("verbose", 'v', No, true),
    o("short", 's', No, true),
    o("branch", 'b', No, true),
    o("show-stash", ' ', No, true),
    o("ahead-behind", ' ', No, true),
    o("porcelain", ' ', Opt, true),
    o("long", ' ', No, true),
    o("null", 'z', No, true),
    o("untracked-files", 'u', Opt, true),
    o("ignored", ' ', Opt, true),
    o("ignore-submodules", ' ', Opt, true),
    o("column", ' ', Opt, true),
    o("no-renames", ' ', No, true),
    o("find-renames", 'M', Opt, false),
];

pub const SWITCH: &[OptDef] = &[
    o("create", 'c', Req, true),
    o("force-create", 'C', Req, true),
    o("guess", ' ', No, true),
    o("discard-changes", ' ', No, true),
    o("quiet", 'q', No, true),
    o("recurse-submodules", ' ', Opt, true),
    o("progress", ' ', No, true),
    o("merge", 'm', No, true),
    o("conflict", ' ', Req, true),
    o("detach", 'd', No, true),
    o("track", 't', Opt, true),
    o("force", 'f', No, true),
    o("orphan", ' ', Req, true),
    o("overwrite-ignore", ' ', No, true),
    o("ignore-other-worktrees", ' ', No, true),
];

pub const TAG: &[OptDef] = &[
    o("list", 'l', No, false),
    o("-n", 'n', Opt, false),
    o("delete", 'd', No, false),
    o("verify", 'v', No, false),
    o("annotate", 'a', No, true),
    o("message", 'm', Req, false),
    o("file", 'F', Req, true),
    o("edit", 'e', No, true),
    o("sign", 's', No, true),
    o("cleanup", ' ', Req, true),
    o("local-user", 'u', Req, true),
    o("force", 'f', No, true),
    o("create-reflog", ' ', No, true),
    o("column", ' ', Opt, true),
    o("contains", ' ', Req, false),
    o("no-contains", ' ', Req, false),
    o("merged", ' ', Req, false),
    o("no-merged", ' ', Req, false),
    o("omit-empty", ' ', No, true),
    o("sort", ' ', Req, true),
    o("points-at", ' ', Req, true),
    o("format", ' ', Req, true),
    o("color", ' ', Opt, true),
    o("ignore-case", 'i', No, true),
];

pub const WORKTREE__ADD: &[OptDef] = &[
    o("force", 'f', No, true),
    o("-b", 'b', Req, false),
    o("-B", 'B', Req, false),
    o("orphan", ' ', No, true),
    o("detach", 'd', No, true),
    o("checkout", ' ', No, true),
    o("lock", ' ', No, true),
    o("reason", ' ', Req, true),
    o("quiet", 'q', No, true),
    o("track", ' ', No, true),
    o("guess-remote", ' ', No, true),
];

pub const WORKTREE__LIST: &[OptDef] = &[
    o("porcelain", ' ', No, true),
    o("verbose", 'v', No, true),
    o("expire", ' ', Req, true),
    o("-z", 'z', No, false),
];

pub const WORKTREE__LOCK: &[OptDef] = &[o("reason", ' ', Req, true)];

pub const WORKTREE__MOVE: &[OptDef] = &[o("force", 'f', No, true)];

pub const WORKTREE__PRUNE: &[OptDef] = &[
    o("dry-run", 'n', No, true),
    o("verbose", 'v', No, true),
    o("expire", ' ', Req, true),
];

pub const WORKTREE__REMOVE: &[OptDef] = &[o("force", 'f', No, true)];

// Subcommands whose `-h` prints no option list: names from the git manual
// pages (git-sparse-checkout, git-mailsplit, git-bisect, git-stash).
pub const SPARSE_CHECKOUT__ADD: &[OptDef] =
    &[o("skip-checks", ' ', No, false), o("stdin", ' ', No, false)];

pub const SPARSE_CHECKOUT__REAPPLY: &[OptDef] =
    &[o("cone", ' ', No, true), o("sparse-index", ' ', No, true)];

pub const MAILSPLIT: &[OptDef] = &[
    o("-b", 'b', No, false),
    o("-d", 'd', Opt, false),
    o("-f", 'f', Opt, false),
    o("-o", 'o', Opt, false),
    o("keep-cr", ' ', No, false),
    o("mboxrd", ' ', No, false),
];

pub const BISECT__START: &[OptDef] = &[
    o("term-new", ' ', Req, false),
    o("term-bad", ' ', Req, false),
    o("term-old", ' ', Req, false),
    o("term-good", ' ', Req, false),
    o("no-checkout", ' ', No, false),
    o("first-parent", ' ', No, false),
];

pub const BISECT__TERMS: &[OptDef] = &[
    o("term-good", ' ', No, false),
    o("term-old", ' ', No, false),
    o("term-bad", ' ', No, false),
    o("term-new", ' ', No, false),
];

pub const STASH__CREATE: &[OptDef] = &[];
