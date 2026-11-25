//! Mail messages for `send-email`: read a patch file (format-patch output or
//! any mbox message), collect automatic Cc addresses, and render the final
//! message with sender, recipients, date, Message-ID and threading headers.

/// One input message split into headers and body.
#[derive(Clone, Debug, Default)]
pub struct Parsed {
    /// Headers in order (folded lines joined).
    pub headers: Vec<(String, String)>,
    pub body: String,
}

impl Parsed {
    pub fn get(&self, name: &str) -> Option<&str> {
        self.headers
            .iter()
            .find(|(k, _)| k.eq_ignore_ascii_case(name))
            .map(|(_, v)| v.as_str())
    }

    pub fn get_all(&self, name: &str) -> Vec<String> {
        self.headers
            .iter()
            .filter(|(k, _)| k.eq_ignore_ascii_case(name))
            .map(|(_, v)| v.clone())
            .collect()
    }
}

/// Split one message (an mbox `From ` line first is skipped).
pub fn parse(text: &str) -> Parsed {
    let text = text.replace("\r\n", "\n");
    let mut lines = text.split_inclusive('\n').peekable();
    if lines.peek().is_some_and(|l| l.starts_with("From ")) {
        lines.next();
    }
    let mut headers: Vec<(String, String)> = Vec::new();
    let mut body = String::new();
    let mut in_body = false;
    for l in lines {
        if in_body {
            body.push_str(l);
            continue;
        }
        let t = l.trim_end_matches('\n');
        if t.is_empty() {
            in_body = true;
            continue;
        }
        if (t.starts_with(' ') || t.starts_with('\t')) && !headers.is_empty() {
            if let Some(last) = headers.last_mut() {
                last.1.push(' ');
                last.1.push_str(t.trim());
            }
        } else if let Some((k, v)) = t.split_once(':') {
            headers.push((k.trim().to_string(), v.trim().to_string()));
        } else {
            in_body = true;
            body.push_str(l);
        }
    }
    Parsed { headers, body }
}

/// Split an mbox into its messages (each starting at a `From ` line).
pub fn split_mbox(text: &str) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    let mut cur = String::new();
    for l in text.split_inclusive('\n') {
        if l.starts_with("From ") && !cur.trim().is_empty() {
            out.push(std::mem::take(&mut cur));
        }
        cur.push_str(l);
    }
    if !cur.trim().is_empty() {
        out.push(cur);
    }
    out
}

/// Split an address list on commas outside quotes and angle brackets.
pub fn split_addrs(s: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut cur = String::new();
    let (mut q, mut a) = (false, false);
    for c in s.chars() {
        match c {
            '"' => q = !q,
            '<' if !q => a = true,
            '>' if !q => a = false,
            ',' if !q && !a => {
                if !cur.trim().is_empty() {
                    out.push(cur.trim().to_string());
                }
                cur.clear();
                continue;
            }
            _ => {}
        }
        cur.push(c);
    }
    if !cur.trim().is_empty() {
        out.push(cur.trim().to_string());
    }
    out
}

/// The bare address of `Name <addr>` (or the value itself).
pub fn bare(addr: &str) -> String {
    match (addr.find('<'), addr.rfind('>')) {
        (Some(a), Some(b)) if a < b => addr[a + 1..b].trim().to_string(),
        _ => addr.trim().to_string(),
    }
}

/// Automatic Cc sources (`--suppress-cc` categories).
#[derive(Clone, Debug, Default)]
pub struct Suppress {
    pub author: bool,
    pub self_: bool,
    pub cc: bool,
    pub bodycc: bool,
    pub sob: bool,
    pub misc_by: bool,
    pub cccmd: bool,
}

impl Suppress {
    pub fn add(&mut self, cat: &str) -> bool {
        match cat {
            "author" => self.author = true,
            "self" => self.self_ = true,
            "cc" => self.cc = true,
            "bodycc" => self.bodycc = true,
            "sob" => self.sob = true,
            "misc-by" => self.misc_by = true,
            "cccmd" => self.cccmd = true,
            "body" => {
                self.sob = true;
                self.bodycc = true;
                self.misc_by = true;
            }
            "all" => {
                *self = Suppress {
                    author: true,
                    self_: true,
                    cc: true,
                    bodycc: true,
                    sob: true,
                    misc_by: true,
                    cccmd: true,
                };
            }
            _ => return false,
        }
        true
    }
}

/// Automatic Cc addresses of one message with the `(source) Adding cc:`
/// notes git prints for them.
pub fn auto_cc(p: &Parsed, sup: &Suppress, sender: &str, sob_cc: bool) -> Vec<(String, String)> {
    let me = bare(sender);
    let mut out: Vec<(String, String)> = Vec::new();
    let add = |addr: &str, src: &str, line: &str, out: &mut Vec<(String, String)>| {
        if sup.self_ && bare(addr).eq_ignore_ascii_case(&me) {
            return;
        }
        out.push((
            addr.to_string(),
            format!("({src}) Adding cc: {addr} from line '{line}'"),
        ));
    };
    if let Some(from) = p.get("From") {
        if !sup.author {
            for a in split_addrs(from) {
                add(&a, "mbox", &format!("From: {from}"), &mut out);
            }
        }
    }
    if !sup.cc {
        for cc in p.get_all("Cc") {
            for a in split_addrs(&cc) {
                add(&a, "mbox", &format!("Cc: {cc}"), &mut out);
            }
        }
    }
    for line in p.body.lines() {
        if line == "---" {
            break;
        }
        let Some((key, val)) = line.split_once(':') else {
            continue;
        };
        if key.contains(' ') || val.trim().is_empty() {
            continue;
        }
        let k = key.to_ascii_lowercase();
        let allowed = if k == "signed-off-by" {
            !sup.sob && sob_cc
        } else if k == "cc" {
            !sup.bodycc && sob_cc
        } else if k.ends_with("-by") {
            !sup.misc_by && sob_cc
        } else {
            false
        };
        if allowed && val.contains('@') {
            add(val.trim(), "body", line, &mut out);
        }
    }
    out
}

/// RFC 2822 date for `t` seconds since the epoch in UTC offset `tz`.
pub fn rfc2822(t: i64, tz_minutes: i32) -> String {
    let local = t + i64::from(tz_minutes) * 60;
    let days = local.div_euclid(86_400);
    let secs = local.rem_euclid(86_400);
    let (y, m, d) = civil(days);
    let wd = ["Thu", "Fri", "Sat", "Sun", "Mon", "Tue", "Wed"][days.rem_euclid(7) as usize];
    let mon = [
        "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
    ][(m - 1) as usize];
    let sign = if tz_minutes < 0 { '-' } else { '+' };
    let a = tz_minutes.abs();
    format!(
        "{wd}, {d} {mon} {y} {:02}:{:02}:{:02} {sign}{:02}{:02}",
        secs / 3600,
        secs % 3600 / 60,
        secs % 60,
        a / 60,
        a % 60
    )
}

/// RFC 2047 `Q` encoded words for a non-ASCII header value in `charset`,
/// each word at most 75 characters, folded onto continuation lines.
pub fn encode_header(value: &str, charset: &str) -> String {
    let open = format!("=?{charset}?q?");
    let room = 75usize.saturating_sub(open.len() + 2).max(4);
    let mut words: Vec<String> = Vec::new();
    let mut cur = String::new();
    for ch in value.chars() {
        let mut tok = String::new();
        if ch == ' ' {
            tok.push('_');
        } else if ch.is_ascii_alphanumeric() || "!*+-/".contains(ch) {
            tok.push(ch);
        } else {
            let mut buf = [0u8; 4];
            for b in ch.encode_utf8(&mut buf).bytes() {
                tok.push_str(&format!("={b:02X}"));
            }
        }
        if !cur.is_empty() && cur.len() + tok.len() > room {
            words.push(std::mem::take(&mut cur));
        }
        cur.push_str(&tok);
    }
    words.push(cur);
    words
        .iter()
        .map(|w| format!("{open}{w}?="))
        .collect::<Vec<_>>()
        .join("\n ")
}

/// RFC 2045 §6.7 quoted-printable body (lines end in `\n`; the SMTP layer
/// writes CRLF). Encoded lines stay within 76 characters.
pub fn quoted_printable(body: &str) -> String {
    let mut out = String::new();
    let mut segs: Vec<&str> = body.split('\n').collect();
    let trailing_nl = body.ends_with('\n');
    if trailing_nl {
        segs.pop();
    }
    for (n, seg) in segs.iter().enumerate() {
        let bytes = seg.as_bytes();
        let mut line = String::new();
        for (i, &b) in bytes.iter().enumerate() {
            let last = i + 1 == bytes.len();
            let tok = match b {
                b' ' | b'\t' if !last => (b as char).to_string(),
                33..=60 | 62..=126 => (b as char).to_string(),
                _ => format!("={b:02X}"),
            };
            if line.len() + tok.len() > 75 {
                out.push_str(&line);
                out.push_str("=\n");
                line.clear();
            }
            line.push_str(&tok);
        }
        out.push_str(&line);
        if n + 1 < segs.len() || trailing_nl {
            out.push('\n');
        }
    }
    out
}

/// RFC 2045 §6.8 base64 body in 76-character lines.
pub fn base64_body(body: &[u8]) -> String {
    use base64::Engine;
    let enc = base64::engine::general_purpose::STANDARD.encode(body);
    let mut out = String::new();
    for chunk in enc.as_bytes().chunks(76) {
        out.push_str(&String::from_utf8_lossy(chunk));
        out.push('\n');
    }
    out
}

fn civil(z: i64) -> (i64, i64, i64) {
    let z = z + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    (if m <= 2 { y + 1 } else { y }, m, d)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn addresses() {
        assert_eq!(
            split_addrs("\"Doe, J\" <j@x>, k@y"),
            vec!["\"Doe, J\" <j@x>".to_string(), "k@y".into()]
        );
        assert_eq!(bare("A <a@b>"), "a@b");
    }

    #[test]
    fn dates() {
        assert_eq!(rfc2822(1_700_000_000, 0), "Tue, 14 Nov 2023 22:13:20 +0000");
        assert_eq!(rfc2822(0, -300), "Wed, 31 Dec 1969 19:00:00 -0500");
    }

    #[test]
    fn encodings() {
        assert_eq!(
            encode_header("caf\u{e9} au lait", "UTF-8"),
            "=?UTF-8?q?caf=C3=A9_au_lait?="
        );
        let long = encode_header(&"\u{e9}".repeat(30), "UTF-8");
        assert!(long.lines().all(|l| l.trim_start().len() <= 75), "{long}");
        assert_eq!(long.matches("=C3=A9").count(), 30);
        assert_eq!(quoted_printable("a=b \nc\u{e9}\n"), "a=3Db=20\nc=C3=A9\n");
        let qp = quoted_printable(&"x".repeat(200));
        assert!(qp.lines().all(|l| l.len() <= 76), "{qp}");
        assert_eq!(qp.replace("=\n", ""), "x".repeat(200));
        assert_eq!(base64_body(b"hi\n"), "aGkK\n");
    }

    #[test]
    fn body_cc() {
        let p = parse(
            "From: A <a@x>\nSubject: s\n\nmsg\n\nSigned-off-by: B <b@x>\nAcked-by: C <c@x>\n---\nCc: no@x\n",
        );
        let got: Vec<String> = auto_cc(&p, &Suppress::default(), "A <a@x>", true)
            .into_iter()
            .map(|x| x.0)
            .collect();
        assert_eq!(got, vec!["A <a@x>", "B <b@x>", "C <c@x>"]);
    }
}
