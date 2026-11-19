//! Minimal POSIX regular expression matcher (unanchored search).
//!
//! git `grep`, `log --grep` and `log --author` default to basic regular
//! expressions; `grep -E` selects extended ones. Claimed subset: literals,
//! `.`, bracket expressions (ranges, `^` negation, `[:alpha:]`-style
//! classes), `*`, `^` / `$` anchors, `\` escapes, case folding; in extended
//! mode (and as `\+` / `\?` / `\|` in basic mode, the GNU extensions git's
//! regex library accepts) also `+`, `?` and top-level `|` alternation.
//! Not supported: `{m,n}` intervals, groups, back-references.
//! Primary: <https://pubs.opengroup.org/onlinepubs/9699919799/basedefs/V1_chap09.html>.

#[derive(Debug, Clone)]
enum Atom {
    Lit(u8),
    Any,
    Set(Vec<(u8, u8)>, bool),
}

#[derive(Debug, Clone)]
struct Piece {
    atom: Atom,
    min: usize,
    max: usize,
}

#[derive(Debug, Clone)]
struct Branch {
    pieces: Vec<Piece>,
    anchor_start: bool,
    anchor_end: bool,
}

/// Compiled pattern (one or more alternatives).
#[derive(Debug, Clone)]
pub struct Bre {
    branches: Vec<Branch>,
    icase: bool,
}

fn class_ranges(name: &str) -> Vec<(u8, u8)> {
    match name {
        "alpha" => vec![(b'a', b'z'), (b'A', b'Z')],
        "digit" => vec![(b'0', b'9')],
        "alnum" => vec![(b'a', b'z'), (b'A', b'Z'), (b'0', b'9')],
        "upper" => vec![(b'A', b'Z')],
        "lower" => vec![(b'a', b'z')],
        "space" => vec![(b' ', b' '), (b'\t', b'\r')],
        "blank" => vec![(b' ', b' '), (b'\t', b'\t')],
        "punct" => vec![(b'!', b'/'), (b':', b'@'), (b'[', b'`'), (b'{', b'~')],
        "xdigit" => vec![(b'0', b'9'), (b'a', b'f'), (b'A', b'F')],
        _ => Vec::new(),
    }
}

/// Split at top-level alternation (`|` in ERE, `\|` in BRE).
fn split_alternatives(pat: &str, extended: bool) -> Vec<String> {
    let b = pat.as_bytes();
    let mut out = Vec::new();
    let mut cur = Vec::new();
    let mut i = 0;
    let mut in_bracket = false;
    while i < b.len() {
        let c = b[i];
        if in_bracket {
            cur.push(c);
            if c == b']' {
                in_bracket = false;
            }
            i += 1;
            continue;
        }
        if c == b'[' {
            in_bracket = true;
            cur.push(c);
            // A leading `]` (or `^]`) is literal inside brackets.
            if b.get(i + 1) == Some(&b'^') {
                cur.push(b'^');
                i += 1;
            }
            if b.get(i + 1) == Some(&b']') {
                cur.push(b']');
                i += 1;
            }
            i += 1;
            continue;
        }
        if extended && c == b'|' {
            out.push(String::from_utf8_lossy(&cur).into_owned());
            cur.clear();
            i += 1;
            continue;
        }
        if c == b'\\' && i + 1 < b.len() {
            if !extended && b[i + 1] == b'|' {
                out.push(String::from_utf8_lossy(&cur).into_owned());
                cur.clear();
                i += 2;
                continue;
            }
            cur.push(c);
            cur.push(b[i + 1]);
            i += 2;
            continue;
        }
        cur.push(c);
        i += 1;
    }
    out.push(String::from_utf8_lossy(&cur).into_owned());
    out
}

fn compile_branch(pat: &str, icase: bool, extended: bool) -> Branch {
    let b = pat.as_bytes();
    let mut i = 0;
    let anchor_start = b.first() == Some(&b'^');
    if anchor_start {
        i = 1;
    }
    let mut end = b.len();
    let anchor_end = end > i && b[end - 1] == b'$' && (end < 2 || b[end - 2] != b'\\');
    if anchor_end {
        end -= 1;
    }
    let fold = |c: u8| if icase { c.to_ascii_lowercase() } else { c };
    let mut pieces: Vec<Piece> = Vec::new();
    while i < end {
        let c = b[i];
        // Quantifiers apply to the previous piece.
        let quant = match c {
            b'*' => Some((0, usize::MAX, 1)),
            b'+' if extended => Some((1, usize::MAX, 1)),
            b'?' if extended => Some((0, 1, 1)),
            b'\\' if !extended && i + 1 < end && b[i + 1] == b'+' => Some((1, usize::MAX, 2)),
            b'\\' if !extended && i + 1 < end && b[i + 1] == b'?' => Some((0, 1, 2)),
            _ => None,
        };
        if let (Some((lo, hi, w)), Some(last)) = (quant, pieces.last_mut()) {
            last.min = lo;
            last.max = hi;
            i += w;
            continue;
        }
        let atom = match c {
            b'.' => {
                i += 1;
                Atom::Any
            }
            b'\\' if i + 1 < end => {
                i += 2;
                match b[i - 1] {
                    b'w' => Atom::Set(
                        vec![(b'a', b'z'), (b'A', b'Z'), (b'0', b'9'), (b'_', b'_')],
                        false,
                    ),
                    b's' => Atom::Set(class_ranges("space"), false),
                    other => Atom::Lit(fold(other)),
                }
            }
            b'[' => {
                let mut j = i + 1;
                let neg = j < end && b[j] == b'^';
                if neg {
                    j += 1;
                }
                let mut ranges = Vec::new();
                let mut first = true;
                while j < end && (first || b[j] != b']') {
                    first = false;
                    if b[j] == b'[' && b.get(j + 1) == Some(&b':') {
                        if let Some(close) = pat[j + 2..].find(":]") {
                            ranges.extend(class_ranges(&pat[j + 2..j + 2 + close]));
                            j += close + 4;
                            continue;
                        }
                    }
                    let lo = b[j];
                    if j + 2 < end && b[j + 1] == b'-' && b[j + 2] != b']' {
                        ranges.push((fold(lo), fold(b[j + 2])));
                        if icase {
                            ranges.push((lo.to_ascii_uppercase(), b[j + 2].to_ascii_uppercase()));
                        }
                        j += 3;
                    } else {
                        ranges.push((fold(lo), fold(lo)));
                        j += 1;
                    }
                }
                if j >= end {
                    // Unterminated bracket: treat `[` literally.
                    i += 1;
                    Atom::Lit(b'[')
                } else {
                    i = j + 1;
                    Atom::Set(ranges, neg)
                }
            }
            _ => {
                i += 1;
                Atom::Lit(fold(c))
            }
        };
        pieces.push(Piece {
            atom,
            min: 1,
            max: 1,
        });
    }
    Branch {
        pieces,
        anchor_start,
        anchor_end,
    }
}

impl Bre {
    /// Basic regular expression.
    pub fn new(pat: &str, icase: bool) -> Self {
        Self::with_mode(pat, icase, false)
    }

    /// `extended` = POSIX ERE (`grep -E`).
    pub fn with_mode(pat: &str, icase: bool, extended: bool) -> Self {
        Self {
            branches: split_alternatives(pat, extended)
                .iter()
                .map(|p| compile_branch(p, icase, extended))
                .collect(),
            icase,
        }
    }

    /// True when the pattern matches anywhere in `hay`.
    pub fn is_match(&self, hay: &[u8]) -> bool {
        self.find_from(hay, 0).is_some()
    }

    /// Leftmost match at or after `from`: `(start, end)`.
    pub fn find_from(&self, hay: &[u8], from: usize) -> Option<(usize, usize)> {
        let folded;
        let hay = if self.icase {
            folded = hay.to_ascii_lowercase();
            &folded[..]
        } else {
            hay
        };
        for s in from..=hay.len() {
            for br in &self.branches {
                if br.anchor_start && s != 0 {
                    continue;
                }
                if let Some(e) = br.match_here(0, hay, s) {
                    return Some((s, e));
                }
            }
        }
        None
    }
}

impl Branch {
    fn atom_matches(atom: &Atom, c: u8) -> bool {
        match atom {
            Atom::Lit(l) => *l == c,
            Atom::Any => c != b'\n',
            Atom::Set(r, neg) => r.iter().any(|(lo, hi)| (*lo..=*hi).contains(&c)) != *neg,
        }
    }

    /// End position of a match of pieces[pi..] starting at `pos` (greedy).
    fn match_here(&self, pi: usize, hay: &[u8], pos: usize) -> Option<usize> {
        if pi == self.pieces.len() {
            return (!self.anchor_end || pos == hay.len()).then_some(pos);
        }
        let p = &self.pieces[pi];
        let mut n = 0;
        while n < p.max && pos + n < hay.len() && Self::atom_matches(&p.atom, hay[pos + n]) {
            n += 1;
        }
        loop {
            if n < p.min {
                return None;
            }
            if let Some(e) = self.match_here(pi + 1, hay, pos + n) {
                return Some(e);
            }
            if n == 0 {
                return None;
            }
            n -= 1;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Bre;

    #[test]
    fn basic_forms() {
        assert!(Bre::new("feat", false).is_match(b"a feature"));
        assert!(!Bre::new("^feat", false).is_match(b"a feature"));
        assert!(Bre::new("re$", false).is_match(b"feature"));
        assert!(Bre::new("f.a*t", false).is_match(b"fet"));
        assert!(Bre::new("[a-c]x", false).is_match(b"zbx"));
        assert!(!Bre::new("[^a-c]x", false).is_match(b"bx"));
        assert!(Bre::new("THOR", true).is_match(b"A U Thor"));
        assert!(Bre::new("a\\.b", false).is_match(b"a.b"));
        assert!(!Bre::new("a\\.b", false).is_match(b"axb"));
        assert!(Bre::new("a+", false).is_match(b"a+"));
        assert!(!Bre::new("a+", false).is_match(b"aa"));
    }

    #[test]
    fn extended_forms() {
        assert!(Bre::with_mode("ab+c", false, true).is_match(b"abbc"));
        assert!(!Bre::with_mode("ab+c", false, true).is_match(b"ac"));
        assert!(Bre::with_mode("colou?r", false, true).is_match(b"color"));
        assert!(Bre::with_mode("cat|dog", false, true).is_match(b"hotdog"));
        assert!(Bre::new("cat\\|dog", false).is_match(b"hotdog"));
        assert!(Bre::new("[[:digit:]]x", false).is_match(b"a1x"));
        assert_eq!(Bre::new("b+", false).find_from(b"ab+c", 0), Some((1, 3)));
    }
}
