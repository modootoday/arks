//! Author / committer identity (git-commit-tree §COMMIT INFORMATION).
//!
//! Order per role: `GIT_<ROLE>_NAME`/`_EMAIL` env → `<role>.name`/`.email`
//! → `user.name`/`user.email` → `EMAIL` env (email only). A missing name or
//! email is an error — arks never writes a placeholder identity.

use crate::set::ConfigSet;
use std::env;
use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IdentRole {
    Author,
    Committer,
}

impl IdentRole {
    fn env_prefix(self) -> &'static str {
        match self {
            IdentRole::Author => "GIT_AUTHOR",
            IdentRole::Committer => "GIT_COMMITTER",
        }
    }
    fn section(self) -> &'static str {
        match self {
            IdentRole::Author => "author",
            IdentRole::Committer => "committer",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Ident {
    pub name: String,
    pub email: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IdentError {
    pub role: IdentRole,
}

impl fmt::Display for IdentError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let who = match self.role {
            IdentRole::Author => "Author",
            IdentRole::Committer => "Committer",
        };
        write!(
            f,
            "{who} identity unknown\n\n*** Please tell me who you are.\n\nRun\n\n  git config --global user.email \"you@example.com\"\n  git config --global user.name \"Your Name\"\n\nto set your account's default identity."
        )
    }
}

impl std::error::Error for IdentError {}

pub fn resolve_ident(cfg: &ConfigSet, role: IdentRole) -> Result<Ident, IdentError> {
    let p = role.env_prefix();
    let sec = role.section();
    let pick = |env_key: String, role_key: String, user_key: &str| {
        env::var(env_key)
            .ok()
            .or_else(|| cfg.get(&role_key))
            .or_else(|| cfg.get(user_key))
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
    };
    let name = pick(format!("{p}_NAME"), format!("{sec}.name"), "user.name");
    let email = pick(format!("{p}_EMAIL"), format!("{sec}.email"), "user.email").or_else(|| {
        env::var("EMAIL")
            .ok()
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
    });
    match (name, email) {
        (Some(name), Some(email)) => Ok(Ident {
            name,
            email: email
                .trim_start_matches('<')
                .trim_end_matches('>')
                .to_string(),
        }),
        _ => Err(IdentError { role }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::set::ConfigScope;

    #[test]
    fn role_section_overrides_user() {
        let mut s = ConfigSet::new();
        s.push("user.name".into(), Some("U".into()), ConfigScope::Global);
        s.push("user.email".into(), Some("u@x".into()), ConfigScope::Global);
        s.push(
            "committer.name".into(),
            Some("C".into()),
            ConfigScope::Local,
        );
        // Env vars may be set by the outer test runner; only assert when absent.
        if env::var_os("GIT_COMMITTER_NAME").is_none()
            && env::var_os("GIT_COMMITTER_EMAIL").is_none()
        {
            let c = resolve_ident(&s, IdentRole::Committer).unwrap();
            assert_eq!((c.name.as_str(), c.email.as_str()), ("C", "u@x"));
        }
    }
}
