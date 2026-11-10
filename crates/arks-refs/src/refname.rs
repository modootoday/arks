//! Refname validation (`git-check-ref-format` subset).

/// Validate a hierarchical refname; `HEAD` is allowed as a special case without `/`.
///
/// Primary: <https://git-scm.com/docs/git-check-ref-format>
pub fn is_valid_refname(name: &str) -> bool {
    validate_refname(name, false)
}

/// Like [`is_valid_refname`] but permits a single path component (no `/`).
pub fn is_valid_refname_allow_onelevel(name: &str) -> bool {
    validate_refname(name, true)
}

fn validate_refname(name: &str, allow_onelevel: bool) -> bool {
    if name.is_empty() {
        return false;
    }
    if name == "HEAD" {
        return true;
    }
    if name.contains("..") || name.contains('\\') || name.contains("@{") {
        return false;
    }
    if name.ends_with('/') || name.ends_with('.') || name.ends_with(".lock") {
        return false;
    }
    if forbidden_bytes(name) {
        return false;
    }

    let components: Vec<&str> = name.split('/').collect();
    if components.is_empty() {
        return false;
    }

    for component in &components {
        if component.is_empty() {
            return false;
        }
        if component.starts_with('.') {
            return false;
        }
    }

    if !allow_onelevel && !name.contains('/') {
        return false;
    }

    true
}

fn forbidden_bytes(s: &str) -> bool {
    for c in s.chars() {
        if c.is_control() {
            return true;
        }
        match c {
            ' ' | '~' | '^' | ':' | '?' | '*' | '[' | '\\' => return true,
            _ => {}
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_heads_main() {
        assert!(is_valid_refname("refs/heads/main"));
    }

    #[test]
    fn accepts_head_special() {
        assert!(is_valid_refname("HEAD"));
    }

    #[test]
    fn rejects_dotdot() {
        assert!(!is_valid_refname("refs/heads/../secret"));
    }

    #[test]
    fn rejects_lock_suffix() {
        assert!(!is_valid_refname("refs/heads/main.lock"));
    }

    #[test]
    fn rejects_onelevel_without_flag() {
        assert!(!is_valid_refname("main"));
    }

    #[test]
    fn allow_onelevel_accepts_main() {
        assert!(is_valid_refname_allow_onelevel("main"));
    }

    #[test]
    fn rejects_at_brace() {
        assert!(!is_valid_refname("refs/heads/foo@{1}"));
    }
}
