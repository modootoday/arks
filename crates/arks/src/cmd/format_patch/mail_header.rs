//! Header text of a format-patch mail: RFC 2047 Q-encoded words for a
//! non-ASCII `Subject:` or display name, RFC 822 quoting of a display name
//! that holds a special character, folding of a long plain `Subject:` and the
//! `MIME-Version` / `Content-Type` / `Content-Transfer-Encoding` block for an
//! 8-bit message.
//!
//! Cite: <https://git-scm.com/docs/git-format-patch> (`--[no-]encode-email-headers`,
//! `format.encodeEmailHeaders`); RFC 2047 §2 (encoded-word, 76-character
//! lines), §4.2 (Q encoding), §5 (`phrase` restrictions); RFC 5322 §2.2.3
//! (folding) and §3.2.4 (quoted-string). The encoded-word alphabet, the fold
//! points and the MIME trigger follow stock Git 2.39.5 output observed in the
//! clean-room image (the `.git/i18n` lines of the CLI flag corpus).

/// Longest line an encoded `Subject:` / `From:` header reaches (RFC 2047 §2).
use super::wrap::{Lead, wrap};

const ENCODED_LINE_MAX: usize = 76;
/// Column a plain `Subject:` is folded at.
const FOLD_WIDTH: usize = 78;
const OPEN: &str = "=?UTF-8?q?";
const CLOSE: &str = "?=";

/// Header block declaring an 8-bit UTF-8 body.
pub const MIME_8BIT: &str =
    "MIME-Version: 1.0\nContent-Type: text/plain; charset=UTF-8\nContent-Transfer-Encoding: 8bit\n";

/// Where an encoded word stands: `*text` (Subject) or a `phrase` (display name).
#[derive(Clone, Copy, PartialEq, Eq)]
enum Place {
    Subject,
    Phrase,
}

/// Text that cannot stand in a header verbatim: 8-bit bytes, or something
/// that a reader would take for an encoded word.
fn needs_encoding(text: &str) -> bool {
    !text.is_ascii() || text.contains("=?")
}

/// Whether `b` may stand for itself inside an encoded word. Space, controls,
/// `=`, `?` and `_` never do (RFC 2047 §4.2); in a phrase only letters,
/// digits and `!*+-/` do (§5 (3)).
fn literal(b: u8, place: Place) -> bool {
    if !(0x21..=0x7e).contains(&b) || matches!(b, b'=' | b'?' | b'_') {
        return false;
    }
    match place {
        Place::Subject => true,
        Place::Phrase => b.is_ascii_alphanumeric() || b"!*+-/".contains(&b),
    }
}

/// `text` as UTF-8 Q-encoded words, starting at byte column `col` of the
/// header line. A word closes before the line would pass 76 characters and
/// the next opens on a continuation line; a character is never split.
fn encoded_words(col: usize, text: &str, place: Place) -> String {
    let mut out = String::from(OPEN);
    let mut len = col + OPEN.len();
    let mut buf = [0u8; 4];
    for ch in text.chars() {
        let bytes = ch.encode_utf8(&mut buf).as_bytes();
        let piece: String = match bytes {
            [b] if literal(*b, place) => char::from(*b).to_string(),
            _ => bytes.iter().map(|b| format!("={b:02X}")).collect(),
        };
        if len + piece.len() + CLOSE.len() > ENCODED_LINE_MAX {
            out.push_str(CLOSE);
            out.push_str("\n ");
            out.push_str(OPEN);
            len = 1 + OPEN.len();
        }
        len += piece.len();
        out.push_str(&piece);
    }
    out.push_str(CLOSE);
    out
}

/// `Subject: <prefix><subject>`: Q-encoded when `encode` is on and the
/// subject needs it, otherwise folded. The prefix (`[PATCH n/m] `) is
/// written as given.
pub fn subject(prefix: &str, subject: &str, encode: bool) -> String {
    let head = format!("Subject: {prefix}");
    let value = if encode && needs_encoding(subject) {
        encoded_words(head.len(), subject, Place::Subject)
    } else {
        wrap(subject, Lead::At(head.chars().count()), 1, FOLD_WIDTH)
    };
    format!("{head}{value}\n")
}

/// A display name as an RFC 822 phrase: quoted, with `\` and `"` escaped,
/// when it holds one of `()<>@,;:\".[]`.
fn phrase(name: &str) -> String {
    if !name.bytes().any(|b| b"()<>@,;:\\\".[]".contains(&b)) {
        return name.to_string();
    }
    let mut q = String::from("\"");
    for c in name.chars() {
        if c == '"' || c == '\\' {
            q.push('\\');
        }
        q.push(c);
    }
    q.push('"');
    q
}

/// `<field>: <ident>` for an address header. The display name of
/// `Name <email>` is Q-encoded when `encode` is on and it needs it, else
/// quoted as a phrase when it must be; an ident without `<` is written as is.
pub fn address(field: &str, ident: &str, encode: bool) -> String {
    let head = format!("{field}: ");
    let value = match ident.find('<') {
        Some(lt) if !ident[..lt].trim().is_empty() => {
            let name = ident[..lt].trim_end();
            let rest = &ident[lt..];
            if encode && needs_encoding(name) {
                format!("{} {rest}", encoded_words(head.len(), name, Place::Phrase))
            } else {
                format!("{} {rest}", phrase(name))
            }
        }
        _ => ident.to_string(),
    };
    format!("{head}{value}\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    // Expected strings are stock Git 2.39.5 output from the clean-room image.

    #[test]
    fn subject_q_encoding() {
        assert_eq!(
            subject("[PATCH] ", "café naïve", true),
            "Subject: [PATCH] =?UTF-8?q?caf=C3=A9=20na=C3=AFve?=\n"
        );
        assert_eq!(
            subject("[PATCH] ", "café naïve", false),
            "Subject: [PATCH] café naïve\n"
        );
        assert_eq!(
            subject("[PATCH] ", "tab\té x", true),
            "Subject: [PATCH] =?UTF-8?q?tab=09=C3=A9=20x?=\n"
        );
        assert_eq!(
            subject("[PATCH] ", "😀 emoji", true),
            "Subject: [PATCH] =?UTF-8?q?=F0=9F=98=80=20emoji?=\n"
        );
    }

    #[test]
    fn subject_specials_and_folding() {
        assert_eq!(
            subject(
                "[PATCH] ",
                "é \"q\" _u_ =e ?q !*+-/ ,.;:()<>@[]\\ ~#$%&' end",
                true
            ),
            "Subject: [PATCH] =?UTF-8?q?=C3=A9=20\"q\"=20=5Fu=5F=20=3De=20=3Fq=20!*+-/=20?=\n =?UTF-8?q?,.;:()<>@[]\\=20~#$%&'=20end?=\n"
        );
        assert_eq!(
            subject("[PATCH] ", "ascii with =?utf-8?q?x?= inside", true),
            "Subject: [PATCH] =?UTF-8?q?ascii=20with=20=3D=3Futf-8=3Fq=3Fx=3F=3D=20insi?=\n =?UTF-8?q?de?=\n"
        );
        // 41 characters after `é` fill the line to 76; a 42nd moves on.
        let fits = format!("é{}", "x".repeat(41));
        assert_eq!(
            subject("[PATCH] ", &fits, true),
            format!("Subject: [PATCH] =?UTF-8?q?=C3=A9{}?=\n", "x".repeat(41))
        );
        let over = format!("é{}", "x".repeat(42));
        assert_eq!(
            subject("[PATCH] ", &over, true),
            format!(
                "Subject: [PATCH] =?UTF-8?q?=C3=A9{}?=\n =?UTF-8?q?x?=\n",
                "x".repeat(41)
            )
        );
        // The column counts the prefix in bytes.
        let p = format!("é{}", "x".repeat(48));
        assert_eq!(
            subject("[PÄTCH] ", &p, true),
            format!(
                "Subject: [PÄTCH] =?UTF-8?q?=C3=A9{}?=\n =?UTF-8?q?{}?=\n",
                "x".repeat(40),
                "x".repeat(8)
            )
        );
        // A multi-byte character is never split across words.
        let e = "é".repeat(65);
        let got = subject("[PATCH] ", &e, true);
        let lines: Vec<&str> = got.lines().collect();
        assert_eq!(lines.len(), 7);
        assert_eq!(
            lines[0],
            format!("Subject: [PATCH] =?UTF-8?q?{}?=", "=C3=A9".repeat(7))
        );
        assert_eq!(lines[1], format!(" =?UTF-8?q?{}?=", "=C3=A9".repeat(10)));
        assert_eq!(lines[6], format!(" =?UTF-8?q?{}?=", "=C3=A9".repeat(8)));
    }

    #[test]
    fn plain_subject_folding() {
        let base = "aaaaaaaaa bbbbbbbbb ccccccccc ddddddddd eeeeeeeee fffffffff";
        assert_eq!(
            subject("[PATCH] ", &format!("{base} g zz"), true),
            format!("Subject: [PATCH] {base} g\n zz\n")
        );
        assert_eq!(
            subject("[PATCH] ", &format!("{base} gg zz"), true),
            format!("Subject: [PATCH] {base}\n gg zz\n")
        );
        assert_eq!(
            subject(
                "[PATCH] ",
                &format!("{base}     spaced     words here and more more more"),
                true
            ),
            format!("Subject: [PATCH] {base}  \n   spaced     words here and more more more\n")
        );
        let long = "x".repeat(73);
        assert_eq!(
            subject("[PATCH] ", &format!("{long} yy"), true),
            format!("Subject: [PATCH] \n {long} yy\n")
        );
        assert_eq!(
            subject("[PATCH] ", &format!("{base}\ttab\ttab tab"), true),
            format!("Subject: [PATCH] {base}\n tab\ttab tab\n")
        );
        assert_eq!(
            subject("", &format!("{base} ggggggggg"), true),
            format!("Subject: {base} ggggggggg\n")
        );
        assert_eq!(
            subject("[PATCH 1/1] ", &format!("{base} ggggggggg"), true),
            "Subject: [PATCH 1/1] aaaaaaaaa bbbbbbbbb ccccccccc ddddddddd eeeeeeeee\n fffffffff ggggggggg\n"
        );
        let e = "ééé ".repeat(7);
        assert_eq!(
            subject("[PATCH] ", &format!("{base} {}", e.trim_end()), false),
            format!("Subject: [PATCH] {base}\n {}\n", e.trim_end())
        );
        assert_eq!(
            subject("[PATCH] ", "short", true),
            "Subject: [PATCH] short\n"
        );
    }

    #[test]
    fn address_names() {
        assert_eq!(
            address("From", "Jöhn Dœ <a@example.org>", true),
            "From: =?UTF-8?q?J=C3=B6hn=20D=C5=93?= <a@example.org>\n"
        );
        assert_eq!(
            address("From", "Jöhn Dœ <a@example.org>", false),
            "From: Jöhn Dœ <a@example.org>\n"
        );
        assert_eq!(
            address("From", "J. Dœ <a@example.org>", true),
            "From: =?UTF-8?q?J=2E=20D=C5=93?= <a@example.org>\n"
        );
        assert_eq!(
            address("From", "J. Dœ <a@example.org>", false),
            "From: \"J. Dœ\" <a@example.org>\n"
        );
        assert_eq!(
            address("From", "J. Doe <a@example.org>", true),
            "From: \"J. Doe\" <a@example.org>\n"
        );
        assert_eq!(
            address("From", "Doe, John <a@example.org>", true),
            "From: \"Doe, John\" <a@example.org>\n"
        );
        assert_eq!(
            address("From", "Jo \"x\" Doe <a@example.org>", true),
            "From: \"Jo \\\"x\\\" Doe\" <a@example.org>\n"
        );
        assert_eq!(
            address("From", "A\\B <a@example.org>", true),
            "From: \"A\\\\B\" <a@example.org>\n"
        );
        for plain in [
            "A'B", "A!B", "A#B", "A$B", "A=B", "A?B", "A_B", "A~B", "tab\tx",
        ] {
            assert_eq!(
                address("From", &format!("{plain} <a@example.org>"), true),
                format!("From: {plain} <a@example.org>\n")
            );
        }
        assert_eq!(
            address(
                "From",
                "é \"q\" _u_ =e ?q !*+-/ ,.;:()<>@[]\\ ~#$%& end <a@example.org>",
                true
            )
            .lines()
            .next(),
            Some("From: =?UTF-8?q?=C3=A9=20=22q=22=20=5Fu=5F=20=3De=20=3Fq=20!*+-/=20=2C=2E?=")
        );
        assert_eq!(
            address(
                "From",
                &format!("{} long name <a@example.org>", "é".repeat(51)),
                true
            ),
            format!(
                "From: =?UTF-8?q?{}?=\n{} =?UTF-8?q?{}=20long=20name?= <a@example.org>\n",
                "=C3=A9".repeat(9),
                format!(" =?UTF-8?q?{}?=\n", "=C3=A9".repeat(10)).repeat(4),
                "=C3=A9".repeat(2)
            )
        );
        assert_eq!(
            address("From", "<a@example.org>", true),
            "From: <a@example.org>\n"
        );
        assert_eq!(
            address("From", "a@example.org", true),
            "From: a@example.org\n"
        );
    }
}
