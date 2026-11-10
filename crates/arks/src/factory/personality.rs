//! Personality resolution: ARKS_PERSONALITY → argv0/shim → default arks.
//! Cite: `.spec/sot/arks-cli-personality-factory.sot.md`

use std::env;
use std::path::Path;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Personality {
    Git,
    Gh,
    Arks,
}

impl Personality {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Git => "git",
            Self::Gh => "gh",
            Self::Arks => "arks",
        }
    }

    pub fn parse(raw: &str) -> Option<Self> {
        match raw.trim().to_ascii_lowercase().as_str() {
            "git" => Some(Self::Git),
            "gh" => Some(Self::Gh),
            "arks" => Some(Self::Arks),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PersonalityResolveError {
    pub raw: String,
}

impl std::fmt::Display for PersonalityResolveError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "unknown ARKS_PERSONALITY=`{}` (expected git|gh|arks); fail-closed \
             ([[sot:root/arks-cli-personality-factory]])",
            self.raw
        )
    }
}

/// Resolve personality. `argv0` is the process argv[0] (may be path or shim basename).
pub fn resolve_personality(argv0: Option<&str>) -> Result<Personality, PersonalityResolveError> {
    if let Ok(raw) = env::var("ARKS_PERSONALITY") {
        if raw.trim().is_empty() {
            // empty → fall through
        } else {
            return Personality::parse(&raw).ok_or(PersonalityResolveError { raw });
        }
    }
    if let Some(a0) = argv0 {
        let base = Path::new(a0)
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or(a0);
        let lower = base.to_ascii_lowercase();
        let stem = lower
            .strip_suffix(".exe")
            .unwrap_or(lower.as_str());
        match stem {
            "git" => return Ok(Personality::Git),
            "gh" => return Ok(Personality::Gh),
            "arks" => return Ok(Personality::Arks),
            _ => {}
        }
    }
    Ok(Personality::Arks)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_case_insensitive() {
        assert_eq!(Personality::parse("GIT"), Some(Personality::Git));
        assert_eq!(Personality::parse("Gh"), Some(Personality::Gh));
        assert_eq!(Personality::parse("arks"), Some(Personality::Arks));
        assert_eq!(Personality::parse("nope"), None);
    }

    #[test]
    fn argv0_shim_basename() {
        // Without env (test may inherit); force via parse path only.
        assert_eq!(
            resolve_from_argv0_only(r"C:\bin\git.exe"),
            Personality::Git
        );
        assert_eq!(resolve_from_argv0_only("/usr/local/bin/gh"), Personality::Gh);
        assert_eq!(resolve_from_argv0_only("arks"), Personality::Arks);
        assert_eq!(resolve_from_argv0_only("weird"), Personality::Arks);
    }

    fn resolve_from_argv0_only(argv0: &str) -> Personality {
        let base = Path::new(argv0)
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or(argv0);
        let lower = base.to_ascii_lowercase();
        let stem = lower.strip_suffix(".exe").unwrap_or(lower.as_str());
        match stem {
            "git" => Personality::Git,
            "gh" => Personality::Gh,
            "arks" => Personality::Arks,
            _ => Personality::Arks,
        }
    }
}
