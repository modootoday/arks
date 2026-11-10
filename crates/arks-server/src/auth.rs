//! Optional HTTP Basic authentication (RFC 7617; transport concern).
//!
//! Cite: gitprotocol-http (auth as transport), RFC 7617 Basic,
//! `.spec/adr/20261109224236-optional-http-basic-auth-product-path.adr.md`.
//! Not forge OAuth / session cookies / Apache htpasswd crypt.

use crate::error::ServerError;
use std::fs;
use std::path::Path;

/// Default realm advertised in `WWW-Authenticate` challenges.
pub const BASIC_REALM: &str = "arks";

/// One plaintext Basic credential (`userid` + `password`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BasicCredential {
    pub username: String,
    pub password: String,
}

/// Configured Basic auth store (one or more credentials).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BasicAuthConfig {
    pub credentials: Vec<BasicCredential>,
    pub realm: String,
}

impl BasicAuthConfig {
    pub fn single(user: impl Into<String>, pass: impl Into<String>) -> Self {
        Self {
            credentials: vec![BasicCredential {
                username: user.into(),
                password: pass.into(),
            }],
            realm: BASIC_REALM.into(),
        }
    }

    /// Parse `user:pass` (split on first `:` so passwords MAY contain `:`).
    pub fn parse_user_pass(spec: &str) -> Result<Self, ServerError> {
        let (user, pass) = split_user_pass(spec)?;
        Ok(Self::single(user, pass))
    }

    /// Load plaintext credential file: one `user:pass` per line; `#` comments / blanks skipped.
    /// Not Apache htpasswd crypt/bcrypt — plaintext only.
    pub fn from_file(path: impl AsRef<Path>) -> Result<Self, ServerError> {
        let text = fs::read_to_string(path.as_ref())
            .map_err(|e| ServerError::Msg(format!("basic-auth file: {e}")))?;
        let mut credentials = Vec::new();
        for (idx, line) in text.lines().enumerate() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            let (user, pass) = split_user_pass(line)
                .map_err(|e| ServerError::Msg(format!("basic-auth file line {}: {e}", idx + 1)))?;
            credentials.push(BasicCredential {
                username: user,
                password: pass,
            });
        }
        if credentials.is_empty() {
            return Err(ServerError::Msg(
                "basic-auth file has no credentials".into(),
            ));
        }
        Ok(Self {
            credentials,
            realm: BASIC_REALM.into(),
        })
    }

    pub fn challenge_header(&self) -> String {
        format!("Basic realm=\"{}\"", self.realm.replace('"', ""))
    }

    /// Validate `Authorization` header value (e.g. `Basic dXNlcjpwYXNz`).
    pub fn check_authorization(&self, authorization: Option<&str>) -> bool {
        let Some(raw) = authorization.map(str::trim).filter(|s| !s.is_empty()) else {
            return false;
        };
        let Some(rest) = raw
            .strip_prefix("Basic ")
            .or_else(|| raw.strip_prefix("basic "))
        else {
            return false;
        };
        let Ok(decoded) = base64_decode(rest.trim().as_bytes()) else {
            return false;
        };
        let Ok(pair) = String::from_utf8(decoded) else {
            return false;
        };
        let Some((user, pass)) = pair.split_once(':') else {
            return false;
        };
        self.credentials.iter().any(|c| {
            constant_eq(c.username.as_bytes(), user.as_bytes())
                && constant_eq(c.password.as_bytes(), pass.as_bytes())
        })
    }
}

fn split_user_pass(spec: &str) -> Result<(String, String), ServerError> {
    let spec = spec.trim();
    let Some((user, pass)) = spec.split_once(':') else {
        return Err(ServerError::Msg(
            "basic-auth needs user:pass (colon-separated)".into(),
        ));
    };
    if user.is_empty() {
        return Err(ServerError::Msg("basic-auth username empty".into()));
    }
    Ok((user.to_string(), pass.to_string()))
}

fn constant_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        // Still walk max length so length leaks are not free; result false.
        let mut acc = 1u8;
        let n = a.len().max(b.len());
        for i in 0..n {
            let x = a.get(i).copied().unwrap_or(0);
            let y = b.get(i).copied().unwrap_or(0);
            acc |= x ^ y;
        }
        return false && acc == 0;
    }
    let mut acc = 0u8;
    for (x, y) in a.iter().zip(b.iter()) {
        acc |= x ^ y;
    }
    acc == 0
}

fn base64_decode(input: &[u8]) -> Result<Vec<u8>, ()> {
    fn val(c: u8) -> Result<u8, ()> {
        match c {
            b'A'..=b'Z' => Ok(c - b'A'),
            b'a'..=b'z' => Ok(c - b'a' + 26),
            b'0'..=b'9' => Ok(c - b'0' + 52),
            b'+' => Ok(62),
            b'/' => Ok(63),
            _ => Err(()),
        }
    }
    let mut cleaned = Vec::with_capacity(input.len());
    for &c in input {
        if !c.is_ascii_whitespace() {
            cleaned.push(c);
        }
    }
    if cleaned.is_empty() || cleaned.len() % 4 != 0 {
        return Err(());
    }
    let mut out = Vec::with_capacity(cleaned.len() / 4 * 3);
    for chunk in cleaned.chunks_exact(4) {
        let pad = chunk.iter().filter(|&&c| c == b'=').count();
        if pad > 2 {
            return Err(());
        }
        let b0 = val(chunk[0])?;
        let b1 = val(chunk[1])?;
        let b2 = if chunk[2] == b'=' { 0 } else { val(chunk[2])? };
        let b3 = if chunk[3] == b'=' { 0 } else { val(chunk[3])? };
        let n =
            (u32::from(b0) << 18) | (u32::from(b1) << 12) | (u32::from(b2) << 6) | u32::from(b3);
        out.push(((n >> 16) & 0xff) as u8);
        if pad < 2 {
            out.push(((n >> 8) & 0xff) as u8);
        }
        if pad < 1 {
            out.push((n & 0xff) as u8);
        }
    }
    Ok(out)
}

/// Encode bytes as standard Base64 (no wrapping) — used by tests / header builders.
pub fn base64_encode(data: &[u8]) -> String {
    const T: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::new();
    let mut i = 0;
    while i + 3 <= data.len() {
        let n = (u32::from(data[i]) << 16) | (u32::from(data[i + 1]) << 8) | u32::from(data[i + 2]);
        out.push(T[((n >> 18) & 63) as usize] as char);
        out.push(T[((n >> 12) & 63) as usize] as char);
        out.push(T[((n >> 6) & 63) as usize] as char);
        out.push(T[(n & 63) as usize] as char);
        i += 3;
    }
    let rem = data.len() - i;
    if rem == 1 {
        let n = u32::from(data[i]) << 16;
        out.push(T[((n >> 18) & 63) as usize] as char);
        out.push(T[((n >> 12) & 63) as usize] as char);
        out.push('=');
        out.push('=');
    } else if rem == 2 {
        let n = (u32::from(data[i]) << 16) | (u32::from(data[i + 1]) << 8);
        out.push(T[((n >> 18) & 63) as usize] as char);
        out.push(T[((n >> 12) & 63) as usize] as char);
        out.push(T[((n >> 6) & 63) as usize] as char);
        out.push('=');
    }
    out
}

pub fn basic_authorization_header(user: &str, pass: &str) -> String {
    format!(
        "Basic {}",
        base64_encode(format!("{user}:{pass}").as_bytes())
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_valid_basic() {
        let cfg = BasicAuthConfig::single("alice", "s3cret");
        let hdr = basic_authorization_header("alice", "s3cret");
        assert!(cfg.check_authorization(Some(&hdr)));
    }

    #[test]
    fn rejects_missing_and_wrong() {
        let cfg = BasicAuthConfig::single("alice", "s3cret");
        assert!(!cfg.check_authorization(None));
        assert!(!cfg.check_authorization(Some("")));
        assert!(!cfg.check_authorization(Some("Bearer x")));
        assert!(!cfg.check_authorization(Some(&basic_authorization_header("alice", "nope"))));
        assert!(!cfg.check_authorization(Some(&basic_authorization_header("bob", "s3cret"))));
    }

    #[test]
    fn password_may_contain_colon() {
        let cfg = BasicAuthConfig::parse_user_pass("u:p:ass").unwrap();
        assert!(cfg.check_authorization(Some(&basic_authorization_header("u", "p:ass"))));
    }

    #[test]
    fn roundtrip_base64() {
        let raw = b"user:pass-word!";
        let enc = base64_encode(raw);
        assert_eq!(base64_decode(enc.as_bytes()).unwrap(), raw);
    }
}
