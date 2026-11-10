//! Commit object payload encode/decode (claimed subset for M5 + FW-GPGSIG).
//!
//! Textual headers + blank line + message. Primary accepted encoding used by
//! stock Git; cite [[research:root/20261108071913]].
//! Optional headers (`gpgsig`, `encoding`, `mergetag`, …) use SP-prefixed
//! continuations per [gitformat-signature](https://git-scm.com/docs/gitformat-signature).
//! `gpgsig` is captured on decode and re-emitted on encode when set. Other
//! optional headers remain skip-only. Cryptographic create/verify of the
//! signature bytes is owned by the CLI (`ssh-keygen -Y` for `gpg.format=ssh`).

use crate::ObjectError;
use arks_oid::Oid;

/// Author / committer identity line fields.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Signature {
    pub name: String,
    pub email: String,
    pub timestamp: i64,
    pub timezone: String,
}

impl Signature {
    pub fn format_line(&self) -> String {
        format!(
            "{} <{}> {} {}",
            self.name, self.email, self.timestamp, self.timezone
        )
    }

    pub fn parse_line(line: &str) -> Result<Self, ObjectError> {
        // name <email> timestamp timezone
        let email_start = line
            .find(" <")
            .ok_or_else(|| ObjectError::BadCommit("missing email".into()))?;
        let email_end = line[email_start..]
            .find('>')
            .ok_or_else(|| ObjectError::BadCommit("unclosed email".into()))?
            + email_start;
        let name = line[..email_start].to_owned();
        let email = line[email_start + 2..email_end].to_owned();
        let rest = line[email_end + 1..].trim_start();
        let mut parts = rest.split_whitespace();
        let ts = parts
            .next()
            .ok_or_else(|| ObjectError::BadCommit("missing timestamp".into()))?;
        let tz = parts
            .next()
            .ok_or_else(|| ObjectError::BadCommit("missing timezone".into()))?;
        if parts.next().is_some() {
            return Err(ObjectError::BadCommit("extra signature fields".into()));
        }
        let timestamp: i64 = ts
            .parse()
            .map_err(|_| ObjectError::BadCommit("bad timestamp".into()))?;
        Ok(Self {
            name,
            email,
            timestamp,
            timezone: tz.to_owned(),
        })
    }
}

/// Parsed commit object payload.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Commit {
    pub tree: Oid,
    pub parents: Vec<Oid>,
    pub author: Signature,
    pub committer: Signature,
    /// Detached signature armor (BEGIN..END lines, LF-separated, no SP prefixes).
    /// Cite: gitformat-signature commit `gpgsig` embedding.
    pub gpgsig: Option<String>,
    pub message: String,
}

impl Commit {
    /// Encode the unsigned signing payload (commit without `gpgsig`).
    /// This is what stock Git feeds to the configured signer for `commit -S`
    /// (claimed: `ssh-keygen -Y sign` when `gpg.format=ssh`).
    pub fn signing_payload(&self) -> Result<Vec<u8>, ObjectError> {
        let mut unsigned = self.clone();
        unsigned.gpgsig = None;
        unsigned.encode()
    }

    pub fn encode(&self) -> Result<Vec<u8>, ObjectError> {
        let mut out = String::new();
        out.push_str("tree ");
        out.push_str(&self.tree.to_hex());
        out.push('\n');
        for p in &self.parents {
            out.push_str("parent ");
            out.push_str(&p.to_hex());
            out.push('\n');
        }
        out.push_str("author ");
        out.push_str(&self.author.format_line());
        out.push('\n');
        out.push_str("committer ");
        out.push_str(&self.committer.format_line());
        out.push('\n');
        if let Some(sig) = &self.gpgsig {
            encode_gpgsig_header(&mut out, sig)?;
        }
        out.push('\n');
        out.push_str(&self.message);
        Ok(out.into_bytes())
    }

    pub fn decode(bytes: &[u8]) -> Result<Self, ObjectError> {
        let text = std::str::from_utf8(bytes)
            .map_err(|_| ObjectError::BadCommit("commit not utf-8".into()))?;
        let (header, message) = match text.split_once("\n\n") {
            Some((h, m)) => (h, m.to_owned()),
            None => (text, String::new()),
        };
        let mut tree: Option<Oid> = None;
        let mut parents = Vec::new();
        let mut author: Option<Signature> = None;
        let mut committer: Option<Signature> = None;
        let mut gpgsig: Option<String> = None;
        let lines: Vec<&str> = header.lines().collect();
        let mut i = 0;
        while i < lines.len() {
            let line = lines[i];
            if let Some(rest) = line.strip_prefix("tree ") {
                if tree.is_some() {
                    return Err(ObjectError::BadCommit("duplicate tree".into()));
                }
                tree = Some(Oid::from_hex(rest.trim()).map_err(ObjectError::Oid)?);
                i += 1;
            } else if let Some(rest) = line.strip_prefix("parent ") {
                parents.push(Oid::from_hex(rest.trim()).map_err(ObjectError::Oid)?);
                i += 1;
            } else if let Some(rest) = line.strip_prefix("author ") {
                author = Some(Signature::parse_line(rest)?);
                i += 1;
            } else if let Some(rest) = line.strip_prefix("committer ") {
                committer = Some(Signature::parse_line(rest)?);
                i += 1;
            } else if let Some(rest) = line.strip_prefix("gpgsig ") {
                if gpgsig.is_some() {
                    return Err(ObjectError::BadCommit("duplicate gpgsig".into()));
                }
                let mut armor = String::new();
                armor.push_str(rest);
                armor.push('\n');
                i += 1;
                while i < lines.len() && lines[i].starts_with(' ') {
                    // SP-prefixed continuation; empty armor line is a single SP.
                    armor.push_str(&lines[i][1..]);
                    armor.push('\n');
                    i += 1;
                }
                gpgsig = Some(armor);
            } else if line.is_empty() {
                break;
            } else if line.starts_with(' ') {
                return Err(ObjectError::BadCommit(format!(
                    "orphan commit header continuation: {line}"
                )));
            } else {
                // Other optional headers (encoding / mergetag / …). Continuations
                // are SP-prefixed — cite gitformat-signature. Skipped for read
                // interop; not re-emitted by encode.
                i += 1;
                while i < lines.len() && lines[i].starts_with(' ') {
                    i += 1;
                }
            }
        }
        Ok(Self {
            tree: tree.ok_or_else(|| ObjectError::BadCommit("missing tree".into()))?,
            parents,
            author: author.ok_or_else(|| ObjectError::BadCommit("missing author".into()))?,
            committer: committer
                .ok_or_else(|| ObjectError::BadCommit("missing committer".into()))?,
            gpgsig,
            message,
        })
    }

    /// First line of the message (subject), trimmed.
    pub fn subject(&self) -> &str {
        self.message.lines().next().unwrap_or("").trim_end()
    }
}

/// Emit `gpgsig` with SP-prefixed continuations (gitformat-signature).
fn encode_gpgsig_header(out: &mut String, armor: &str) -> Result<(), ObjectError> {
    let trimmed = armor.trim_end_matches('\n');
    let mut lines = trimmed.split('\n');
    let first = lines
        .next()
        .ok_or_else(|| ObjectError::BadCommit("empty gpgsig".into()))?;
    out.push_str("gpgsig ");
    out.push_str(first);
    out.push('\n');
    for line in lines {
        out.push(' ');
        out.push_str(line);
        out.push('\n');
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn oid(n: u8) -> Oid {
        Oid::from_hex(&format!("{n:040x}")).unwrap()
    }

    fn sample_sig() -> Signature {
        Signature {
            name: "A".into(),
            email: "a@example.com".into(),
            timestamp: 1,
            timezone: "+0000".into(),
        }
    }

    #[test]
    fn roundtrip_root_commit() {
        let c = Commit {
            tree: oid(1),
            parents: vec![],
            author: sample_sig(),
            committer: sample_sig(),
            gpgsig: None,
            message: "init\n".into(),
        };
        let bytes = c.encode().unwrap();
        assert_eq!(Commit::decode(&bytes).unwrap(), c);
        assert_eq!(c.subject(), "init");
    }

    #[test]
    fn roundtrip_with_parent() {
        let c = Commit {
            tree: oid(2),
            parents: vec![oid(9)],
            author: Signature {
                name: "B".into(),
                email: "b@e.com".into(),
                timestamp: 42,
                timezone: "+0900".into(),
            },
            committer: Signature {
                name: "B".into(),
                email: "b@e.com".into(),
                timestamp: 42,
                timezone: "+0900".into(),
            },
            gpgsig: None,
            message: "second\n\nbody\n".into(),
        };
        assert_eq!(Commit::decode(&c.encode().unwrap()).unwrap(), c);
    }

    #[test]
    fn decode_skips_gpgsig_multiline_header() {
        // Shape from gitformat-signature: SP-prefixed continuations; empty
        // armor lines are a single SP (not a bare blank line).
        let mut raw = String::new();
        raw.push_str(&format!("tree {}\n", oid(1).to_hex()));
        raw.push_str(&format!("parent {}\n", oid(2).to_hex()));
        raw.push_str("author A U Thor <author@example.com> 1465981137 +0000\n");
        raw.push_str("committer C O Mitter <committer@example.com> 1465981137 +0000\n");
        raw.push_str("gpgsig -----BEGIN PGP SIGNATURE-----\n");
        raw.push_str(" Version: GnuPG v1\n");
        raw.push_str(" \n"); // empty armor line = SP + LF
        raw.push_str(" iQEcBAABAgAGBQJXYRjRAAoJEGEJLoW3InGJ3IwIAIY4SA6GxY3BjL60YyvsJPh/\n");
        raw.push_str(" -----END PGP SIGNATURE-----\n");
        raw.push('\n');
        raw.push_str("signed commit\n");
        let c = Commit::decode(raw.as_bytes()).unwrap();
        assert_eq!(c.tree, oid(1));
        assert_eq!(c.parents, vec![oid(2)]);
        assert_eq!(c.author.name, "A U Thor");
        assert_eq!(c.committer.name, "C O Mitter");
        assert_eq!(c.subject(), "signed commit");
        let sig = c.gpgsig.expect("gpgsig captured");
        assert!(sig.starts_with("-----BEGIN PGP SIGNATURE-----\n"));
        assert!(sig.contains("\n\n")); // empty armor line preserved
        assert!(sig.contains("-----END PGP SIGNATURE-----\n"));
    }

    #[test]
    fn roundtrip_ssh_gpgsig_header() {
        let armor = "-----BEGIN SSH SIGNATURE-----\n\
U1NIU0lHAAAAAQAAADMAAAALc3NoLWVkMjU1MTkAAAAgAAAA\n\
-----END SSH SIGNATURE-----\n";
        let c = Commit {
            tree: oid(3),
            parents: vec![],
            author: sample_sig(),
            committer: sample_sig(),
            gpgsig: Some(armor.to_owned()),
            message: "signed\n".into(),
        };
        let bytes = c.encode().unwrap();
        let text = std::str::from_utf8(&bytes).unwrap();
        assert!(text.contains("gpgsig -----BEGIN SSH SIGNATURE-----\n"));
        assert!(text.contains(" U1NIU0lHAAAAAQAAADMAAAALc3NoLWVkMjU1MTkAAAAgAAAA\n"));
        assert!(text.contains(" -----END SSH SIGNATURE-----\n"));
        let back = Commit::decode(&bytes).unwrap();
        assert_eq!(back.gpgsig.as_deref(), Some(armor));
        // Signing payload omits gpgsig.
        let payload = c.signing_payload().unwrap();
        let unsigned = Commit::decode(&payload).unwrap();
        assert!(unsigned.gpgsig.is_none());
        assert_eq!(unsigned.message, c.message);
        assert_eq!(unsigned.tree, c.tree);
    }

    #[test]
    fn decode_skips_encoding_header() {
        let raw = format!(
            "tree {}\n\
             author A <a@e.com> 1 +0000\n\
             committer A <a@e.com> 1 +0000\n\
             encoding UTF-8\n\
             \n\
             msg\n",
            oid(3).to_hex()
        );
        let c = Commit::decode(raw.as_bytes()).unwrap();
        assert_eq!(c.tree, oid(3));
        assert_eq!(c.subject(), "msg");
        assert!(c.gpgsig.is_none());
    }
}
