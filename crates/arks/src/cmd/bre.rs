//! Minimal POSIX basic regular expression matcher (unanchored search).
//!
//! git `grep`, `log --grep` and `log --author` default to basic regular
//! expressions. Claimed subset: literals, `.`, bracket expressions (ranges,
//! `^` negation), `*` on the previous atom, `^` / `$` anchors, `\` escapes,
//! and case folding (`-i`). Not supported: `\{m,n\}`, groups, back-references,
//! character classes such as `[:alpha:]`.
//! Primary: <https://pubs.opengroup.org/onlinepubs/9699919799/basedefs/V1_chap09.html#tag_09_03>.

#[derive(Debug, Clone)]
enum Atom {
    Lit(u8),
    Any,
    Set(Vec<(u8, u8)>, bool),
}

#[derive(Debug, Clone)]
struct Piece {
    atom: Atom,
    star: bool,
}

/// Compiled pattern.
#[derive(Debug, Clone)]
pub struct Bre {
    pieces: Vec<Piece>,
    anchor_start: bool,
    anchor_end: bool,
    icase: bool,
}

impl Bre {
    pub fn new(pat: &str, icase: bool) -> Self {
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
            let atom = match c {
                b'.' => {
                    i += 1;
                    Atom::Any
                }
                b'*' if !pieces.is_empty() => {
                    if let Some(last) = pieces.last_mut() {
                        last.star = true;
                    }
                    i += 1;
                    continue;
                }
                b'\\' if i + 1 < end => {
                    i += 2;
                    Atom::Lit(fold(b[i - 1]))
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
                        let lo = b[j];
                        if j + 2 < end && b[j + 1] == b'-' && b[j + 2] != b']' {
                            ranges.push((fold(lo), fold(b[j + 2])));
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
            pieces.push(Piece { atom, star: false });
        }
        Self {
            pieces,
            anchor_start,
            anchor_end,
            icase,
        }
    }

    /// True when the pattern matches anywhere in `hay`.
    pub fn is_match(&self, hay: &[u8]) -> bool {
        let folded;
        let hay = if self.icase {
            folded = hay.to_ascii_lowercase();
            &folded[..]
        } else {
            hay
        };
        if self.anchor_start {
            return self.match_here(0, hay, 0);
        }
        (0..=hay.len()).any(|s| self.match_here(0, hay, s))
    }

    fn atom_matches(atom: &Atom, c: u8) -> bool {
        match atom {
            Atom::Lit(l) => *l == c,
            Atom::Any => c != b'\n',
            Atom::Set(r, neg) => r.iter().any(|(lo, hi)| (*lo..=*hi).contains(&c)) != *neg,
        }
    }

    fn match_here(&self, pi: usize, hay: &[u8], pos: usize) -> bool {
        if pi == self.pieces.len() {
            return !self.anchor_end || pos == hay.len();
        }
        let p = &self.pieces[pi];
        if p.star {
            let mut n = 0;
            while pos + n < hay.len() && Self::atom_matches(&p.atom, hay[pos + n]) {
                n += 1;
            }
            loop {
                if self.match_here(pi + 1, hay, pos + n) {
                    return true;
                }
                if n == 0 {
                    return false;
                }
                n -= 1;
            }
        }
        pos < hay.len()
            && Self::atom_matches(&p.atom, hay[pos])
            && self.match_here(pi + 1, hay, pos + 1)
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
    }
}
