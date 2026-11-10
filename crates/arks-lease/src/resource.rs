//! Cd-1 resource-id prefix helpers ([[adr:root/20261109032955]]).
//!
//! Opaque strings remain valid. Prefixed forms are the G18 oracle convention.
//! Holding a lease never skips Git locks on Git-compat paths.

/// Prefix for ref-namespace exclusive coordination.
pub const PREFIX_REFNS: &str = "refns:";

/// Prefix for index-writer exclusive coordination.
pub const PREFIX_INDEX: &str = "index:";

/// Prefix for linked-worktree / common-dir domain coordination (Cd-2).
pub const PREFIX_DOMAIN_WORKTREE: &str = "domain:worktree:";

/// Prefix for submodule git-dir domain coordination (Cd-2).
pub const PREFIX_DOMAIN_SUBMODULE: &str = "domain:submodule:";

/// Prefix for hook-critical-section budget coordination (Cd-3).
pub const PREFIX_HOOK: &str = "hook:";

/// Prefix for maintenance/gc writer coordination (Cd-3).
pub const PREFIX_MAINT: &str = "maint:";

/// Build a `refns:<git-ref-prefix>` resource id.
pub fn refns(prefix: impl AsRef<str>) -> String {
    let p = prefix.as_ref();
    if p.starts_with(PREFIX_REFNS) {
        p.to_string()
    } else {
        format!("{PREFIX_REFNS}{p}")
    }
}

/// Build an `index:<scope>` resource id.
pub fn index_scope(scope: impl AsRef<str>) -> String {
    let s = scope.as_ref();
    if s.starts_with(PREFIX_INDEX) {
        s.to_string()
    } else {
        format!("{PREFIX_INDEX}{s}")
    }
}

/// Build a `domain:worktree:<id>` resource id.
pub fn domain_worktree(id: impl AsRef<str>) -> String {
    let s = id.as_ref();
    if s.starts_with(PREFIX_DOMAIN_WORKTREE) {
        s.to_string()
    } else {
        format!("{PREFIX_DOMAIN_WORKTREE}{s}")
    }
}

/// Build a `domain:submodule:<path>` resource id.
pub fn domain_submodule(path: impl AsRef<str>) -> String {
    let s = path.as_ref();
    if s.starts_with(PREFIX_DOMAIN_SUBMODULE) {
        s.to_string()
    } else {
        format!("{PREFIX_DOMAIN_SUBMODULE}{s}")
    }
}

/// Build a `hook:<scope>` resource id.
pub fn hook_scope(scope: impl AsRef<str>) -> String {
    let s = scope.as_ref();
    if s.starts_with(PREFIX_HOOK) {
        s.to_string()
    } else {
        format!("{PREFIX_HOOK}{s}")
    }
}

/// Build a `maint:<scope>` resource id.
pub fn maint_scope(scope: impl AsRef<str>) -> String {
    let s = scope.as_ref();
    if s.starts_with(PREFIX_MAINT) {
        s.to_string()
    } else {
        format!("{PREFIX_MAINT}{s}")
    }
}

/// True when `id` uses a Cd-1..Cd-3 typed prefix.
pub fn is_typed(id: &str) -> bool {
    id.starts_with(PREFIX_REFNS)
        || id.starts_with(PREFIX_INDEX)
        || id.starts_with(PREFIX_DOMAIN_WORKTREE)
        || id.starts_with(PREFIX_DOMAIN_SUBMODULE)
        || id.starts_with(PREFIX_HOOK)
        || id.starts_with(PREFIX_MAINT)
}
