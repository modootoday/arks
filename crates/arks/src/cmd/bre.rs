//! POSIX regular expression matcher (unanchored search), basic and
//! extended syntax.
//!
//! git `grep`, `log --grep` and `log --author` default to basic regular
//! expressions; `-E` selects extended ones. Supported: literals, `.`,
//! bracket expressions (ranges, `^` negation, `[:alpha:]`-style classes),
//! `*`, intervals (`\{m,n\}` / `{m,n}`), groups (`\(..\)` / `(..)`),
//! alternation (`\|` / `|`), back-references `\1`..`\9`, `^` / `$`
//! anchors (also at line starts / ends inside a buffer), case folding and
//! the GNU extensions git's regex library accepts: `\+`, `\?` in basic
//! mode, `\w`, `\W`, `\s`, `\S`, `\b`, `\B`, `\<`, `\>`.
//!
//! Matching backtracks over a small syntax tree. `find_from` reports the
//! leftmost-longest match POSIX requires; `is_match` stops at the first.
//! Primary: <https://pubs.opengroup.org/onlinepubs/9699919799/basedefs/V1_chap09.html>.

#[derive(Debug, Clone)]
enum Node {
    Lit(u8),
    Any,
    Set(Vec<(u8, u8)>, bool),
    Bol,
    Eol,
    WordBoundary(bool),
    WordStart,
    WordEnd,
    Group(Box<Node>, usize),
    Backref(usize),
    Alt(Vec<Node>),
    Concat(Vec<Node>),
    Repeat(Box<Node>, usize, usize),
}

/// Compiled pattern.
#[derive(Debug, Clone)]
pub struct Bre {
    root: Node,
    groups: usize,
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
        "cntrl" => vec![(0, 0x1f), (0x7f, 0x7f)],
        "print" => vec![(0x20, 0x7e)],
        "graph" => vec![(0x21, 0x7e)],
        _ => Vec::new(),
    }
}

fn word_ranges() -> Vec<(u8, u8)> {
    vec![(b'a', b'z'), (b'A', b'Z'), (b'0', b'9'), (b'_', b'_')]
}

fn is_word(c: u8) -> bool {
    c.is_ascii_alphanumeric() || c == b'_'
}

struct Parser<'a> {
    b: &'a [u8],
    i: usize,
    extended: bool,
    icase: bool,
    groups: usize,
}

impl Parser<'_> {
    fn fold(&self, c: u8) -> u8 {
        if self.icase {
            c.to_ascii_lowercase()
        } else {
            c
        }
    }

    fn peek(&self) -> Option<u8> {
        self.b.get(self.i).copied()
    }

    fn at_alt(&self) -> bool {
        if self.extended {
            self.peek() == Some(b'|')
        } else {
            self.peek() == Some(b'\\') && self.b.get(self.i + 1) == Some(&b'|')
        }
    }

    fn at_close(&self) -> bool {
        if self.extended {
            self.peek() == Some(b')')
        } else {
            self.peek() == Some(b'\\') && self.b.get(self.i + 1) == Some(&b')')
        }
    }

    /// alternation := branch (`|` branch)*
    fn alternation(&mut self, depth: usize) -> Node {
        let mut alts = vec![self.branch(depth)];
        while self.at_alt() {
            self.i += if self.extended { 1 } else { 2 };
            alts.push(self.branch(depth));
        }
        if alts.len() == 1 {
            alts.pop().unwrap_or(Node::Concat(Vec::new()))
        } else {
            Node::Alt(alts)
        }
    }

    /// Interval body after `{` (ERE) / `\{` (BRE): `m`, `m,`, `m,n`.
    fn interval(&mut self) -> Option<(usize, usize)> {
        let start = self.i;
        let num = |p: &mut Self| -> Option<usize> {
            let s = p.i;
            while p.peek().is_some_and(|c| c.is_ascii_digit()) {
                p.i += 1;
            }
            std::str::from_utf8(&p.b[s..p.i]).ok()?.parse().ok()
        };
        let lo = num(self);
        let Some(lo) = lo else {
            self.i = start;
            return None;
        };
        let hi = if self.peek() == Some(b',') {
            self.i += 1;
            num(self).unwrap_or(usize::MAX)
        } else {
            lo
        };
        let closed = if self.extended {
            self.peek() == Some(b'}') && {
                self.i += 1;
                true
            }
        } else {
            self.peek() == Some(b'\\') && self.b.get(self.i + 1) == Some(&b'}') && {
                self.i += 2;
                true
            }
        };
        if !closed || hi < lo {
            self.i = start;
            return None;
        }
        Some((lo, hi))
    }

    /// Quantifier after an atom, if any.
    fn quantifier(&mut self) -> Option<(usize, usize)> {
        let c = self.peek()?;
        let n = self.b.get(self.i + 1).copied();
        match (c, self.extended) {
            (b'*', _) => {
                self.i += 1;
                Some((0, usize::MAX))
            }
            (b'+', true) => {
                self.i += 1;
                Some((1, usize::MAX))
            }
            (b'?', true) => {
                self.i += 1;
                Some((0, 1))
            }
            (b'{', true) => {
                self.i += 1;
                let r = self.interval();
                if r.is_none() {
                    self.i -= 1;
                }
                r
            }
            (b'\\', false) => match n {
                Some(b'+') => {
                    self.i += 2;
                    Some((1, usize::MAX))
                }
                Some(b'?') => {
                    self.i += 2;
                    Some((0, 1))
                }
                Some(b'{') => {
                    self.i += 2;
                    let r = self.interval();
                    if r.is_none() {
                        self.i -= 2;
                    }
                    r
                }
                _ => None,
            },
            _ => None,
        }
    }

    /// branch := piece*
    fn branch(&mut self, depth: usize) -> Node {
        let mut items: Vec<Node> = Vec::new();
        let branch_start = self.i;
        while self.i < self.b.len() && !self.at_alt() && !(depth > 0 && self.at_close()) {
            let at_start = self.i == branch_start;
            let c = self.b[self.i];
            // A quantifier with nothing before it is literal.
            if c == b'*' && (items.is_empty() || matches!(items.last(), Some(Node::Bol))) {
                self.i += 1;
                items.push(Node::Lit(b'*'));
                continue;
            }
            let atom = match c {
                b'^' if self.extended || at_start => {
                    self.i += 1;
                    Node::Bol
                }
                b'$' if self.extended || self.at_branch_end(self.i + 1, depth) => {
                    self.i += 1;
                    Node::Eol
                }
                b'.' => {
                    self.i += 1;
                    Node::Any
                }
                b'[' => self.bracket(),
                b'(' if self.extended => {
                    self.i += 1;
                    self.group(depth)
                }
                b'\\' if self.i + 1 < self.b.len() => {
                    let e = self.b[self.i + 1];
                    self.i += 2;
                    match e {
                        b'(' if !self.extended => self.group(depth),
                        b'1'..=b'9' => Node::Backref((e - b'0') as usize),
                        b'w' => Node::Set(word_ranges(), false),
                        b'W' => Node::Set(word_ranges(), true),
                        b's' => Node::Set(class_ranges("space"), false),
                        b'S' => Node::Set(class_ranges("space"), true),
                        b'b' => Node::WordBoundary(true),
                        b'B' => Node::WordBoundary(false),
                        b'<' => Node::WordStart,
                        b'>' => Node::WordEnd,
                        b'n' => Node::Lit(b'\n'),
                        b't' => Node::Lit(b'\t'),
                        other => Node::Lit(self.fold(other)),
                    }
                }
                _ => {
                    self.i += 1;
                    Node::Lit(self.fold(c))
                }
            };
            let mut atom = atom;
            while let Some((lo, hi)) = self.quantifier() {
                atom = Node::Repeat(Box::new(atom), lo, hi);
            }
            items.push(atom);
        }
        if items.len() == 1 {
            items.pop().unwrap_or(Node::Concat(Vec::new()))
        } else {
            Node::Concat(items)
        }
    }

    /// BRE `$` is an anchor only at the end of a branch.
    fn at_branch_end(&self, j: usize, depth: usize) -> bool {
        let rest = &self.b[j..];
        rest.is_empty() || rest.starts_with(b"\\|") || (depth > 0 && rest.starts_with(b"\\)"))
    }

    fn group(&mut self, depth: usize) -> Node {
        self.groups += 1;
        let idx = self.groups;
        let inner = self.alternation(depth + 1);
        if self.at_close() {
            self.i += if self.extended { 1 } else { 2 };
        }
        Node::Group(Box::new(inner), idx)
    }

    fn bracket(&mut self) -> Node {
        let b = self.b;
        let end = b.len();
        let mut j = self.i + 1;
        let neg = j < end && b[j] == b'^';
        if neg {
            j += 1;
        }
        let mut ranges = Vec::new();
        let mut first = true;
        while j < end && (first || b[j] != b']') {
            first = false;
            if b[j] == b'[' && b.get(j + 1) == Some(&b':') {
                let tail = &b[j + 2..];
                if let Some(close) = tail.windows(2).position(|w| w == b":]") {
                    let name = String::from_utf8_lossy(&tail[..close]);
                    ranges.extend(class_ranges(&name));
                    j += close + 4;
                    continue;
                }
            }
            let lo = b[j];
            if j + 2 < end && b[j + 1] == b'-' && b[j + 2] != b']' {
                let hi = b[j + 2];
                ranges.push((self.fold(lo), self.fold(hi)));
                if self.icase {
                    ranges.push((lo.to_ascii_uppercase(), hi.to_ascii_uppercase()));
                    ranges.push((lo.to_ascii_lowercase(), hi.to_ascii_lowercase()));
                }
                j += 3;
            } else {
                ranges.push((self.fold(lo), self.fold(lo)));
                j += 1;
            }
        }
        if j >= end {
            // Unterminated bracket: `[` is literal.
            self.i += 1;
            Node::Lit(b'[')
        } else {
            self.i = j + 1;
            Node::Set(ranges, neg)
        }
    }
}

type Caps = Vec<Option<(usize, usize)>>;

/// Matching state for one haystack.
struct Matcher<'h> {
    hay: &'h [u8],
    steps: usize,
}

/// Exhaustive (longest-match) searches give up after this many steps and
/// fall back to the first match.
const STEP_BUDGET: usize = 200_000;

impl Matcher<'_> {
    fn m(
        &mut self,
        n: &Node,
        pos: usize,
        caps: &mut Caps,
        k: &mut dyn FnMut(&mut Self, usize, &mut Caps) -> bool,
    ) -> bool {
        self.steps += 1;
        let hay = self.hay;
        match n {
            Node::Lit(c) => pos < hay.len() && hay[pos] == *c && k(self, pos + 1, caps),
            Node::Any => pos < hay.len() && hay[pos] != b'\n' && k(self, pos + 1, caps),
            Node::Set(r, neg) => {
                pos < hay.len()
                    && r.iter().any(|(lo, hi)| (*lo..=*hi).contains(&hay[pos])) != *neg
                    && k(self, pos + 1, caps)
            }
            Node::Bol => (pos == 0 || hay[pos - 1] == b'\n') && k(self, pos, caps),
            Node::Eol => (pos == hay.len() || hay[pos] == b'\n') && k(self, pos, caps),
            Node::WordBoundary(want) => {
                let before = pos > 0 && is_word(hay[pos - 1]);
                let after = pos < hay.len() && is_word(hay[pos]);
                ((before != after) == *want) && k(self, pos, caps)
            }
            Node::WordStart => {
                let before = pos > 0 && is_word(hay[pos - 1]);
                let after = pos < hay.len() && is_word(hay[pos]);
                !before && after && k(self, pos, caps)
            }
            Node::WordEnd => {
                let before = pos > 0 && is_word(hay[pos - 1]);
                let after = pos < hay.len() && is_word(hay[pos]);
                before && !after && k(self, pos, caps)
            }
            Node::Backref(i) => {
                let Some(Some((s, e))) = caps.get(*i).copied() else {
                    return false;
                };
                let len = e - s;
                pos + len <= hay.len()
                    && hay[s..e] == hay[pos..pos + len]
                    && k(self, pos + len, caps)
            }
            Node::Group(inner, i) => {
                let i = *i;
                self.m(inner, pos, caps, &mut |me, e, caps| {
                    let saved = caps[i];
                    caps[i] = Some((pos, e));
                    let r = k(me, e, caps);
                    caps[i] = saved;
                    r
                })
            }
            Node::Alt(v) => {
                for b in v {
                    if self.m(b, pos, caps, k) {
                        return true;
                    }
                }
                false
            }
            Node::Concat(v) => self.seq(v, pos, caps, k),
            Node::Repeat(inner, lo, hi) => self.rep(inner, *lo, *hi, 0, pos, caps, k),
        }
    }

    fn seq(
        &mut self,
        v: &[Node],
        pos: usize,
        caps: &mut Caps,
        k: &mut dyn FnMut(&mut Self, usize, &mut Caps) -> bool,
    ) -> bool {
        match v.split_first() {
            None => k(self, pos, caps),
            Some((first, rest)) => self.m(first, pos, caps, &mut |me, e, caps| {
                me.seq(rest, e, caps, k)
            }),
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn rep(
        &mut self,
        inner: &Node,
        lo: usize,
        hi: usize,
        count: usize,
        pos: usize,
        caps: &mut Caps,
        k: &mut dyn FnMut(&mut Self, usize, &mut Caps) -> bool,
    ) -> bool {
        if count < hi {
            // Greedy: one more iteration first; an empty iteration past the
            // minimum cannot make progress and is not taken.
            let more = self.m(inner, pos, caps, &mut |me, e, caps| {
                if e == pos && count >= lo {
                    return false;
                }
                me.rep(inner, lo, hi, count + 1, e, caps, k)
            });
            if more {
                return true;
            }
        }
        count >= lo && k(self, pos, caps)
    }
}

impl Bre {
    /// Basic regular expression.
    pub fn new(pat: &str, icase: bool) -> Self {
        Self::with_mode(pat, icase, false)
    }

    /// `extended` = POSIX ERE (`grep -E`).
    pub fn with_mode(pat: &str, icase: bool, extended: bool) -> Self {
        let mut p = Parser {
            b: pat.as_bytes(),
            i: 0,
            extended,
            icase,
            groups: 0,
        };
        let mut root = p.alternation(0);
        // Stray closing groups at top level are literal.
        while p.i < p.b.len() {
            let rest = p.alternation(0);
            let skip = if p.extended { 1 } else { 2 };
            let lit = Node::Lit(b')');
            p.i += skip.min(p.b.len() - p.i);
            root = Node::Concat(vec![root, lit, rest]);
        }
        Self {
            root,
            groups: p.groups,
            icase,
        }
    }

    fn folded<'a>(&self, hay: &'a [u8], buf: &'a mut Vec<u8>) -> &'a [u8] {
        if self.icase {
            *buf = hay.to_ascii_lowercase();
            buf
        } else {
            hay
        }
    }

    /// True when the pattern matches anywhere in `hay`.
    pub fn is_match(&self, hay: &[u8]) -> bool {
        let mut buf = Vec::new();
        let hay = self.folded(hay, &mut buf);
        let mut me = Matcher { hay, steps: 0 };
        let mut caps: Caps = vec![None; self.groups + 1];
        (0..=hay.len()).any(|s| me.m(&self.root, s, &mut caps, &mut |_, _, _| true))
    }

    /// Leftmost-longest match at or after `from`: `(start, end)`.
    pub fn find_from(&self, hay: &[u8], from: usize) -> Option<(usize, usize)> {
        let mut buf = Vec::new();
        let hay = self.folded(hay, &mut buf);
        let mut me = Matcher { hay, steps: 0 };
        let mut caps: Caps = vec![None; self.groups + 1];
        for s in from..=hay.len() {
            let mut best: Option<usize> = None;
            me.steps = 0;
            me.m(&self.root, s, &mut caps, &mut |me, e, _| {
                best = Some(best.map_or(e, |b| b.max(e)));
                // Keep exploring for a longer match while within budget.
                me.steps > STEP_BUDGET
            });
            if let Some(e) = best {
                return Some((s, e));
            }
        }
        None
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
        assert!(Bre::new("*a", false).is_match(b"x*a"));
        assert!(Bre::new("a$b", false).is_match(b"a$b"));
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

    #[test]
    fn groups_intervals_backrefs() {
        let e = |p: &str| Bre::with_mode(p, false, true);
        assert!(e("ext(end)?").is_match(b"main: extend readme"));
        assert!(e("(ab)+c").is_match(b"xababc"));
        assert!(!e("^(ab)+c").is_match(b"xababc"));
        assert!(e("a{2,3}").is_match(b"caab"));
        assert!(!e("ca{2,3}b").is_match(b"cab"));
        assert!(Bre::new("a\\{2\\}", false).is_match(b"aa"));
        assert!(Bre::new("\\(ab\\)\\1", false).is_match(b"abab"));
        assert!(!Bre::new("\\(ab\\)\\1", false).is_match(b"abac"));
        assert!(e("(a|b)(c|d)").is_match(b"xbd"));
        assert!(e("\\bfoo\\b").is_match(b"a foo b"));
        assert!(!e("\\bfoo\\b").is_match(b"afoob"));
        assert!(e("^b").is_match(b"a\nb"));
    }

    #[test]
    fn leftmost_longest() {
        let e = |p: &str| Bre::with_mode(p, false, true);
        assert_eq!(e("a|ab").find_from(b"xab", 0), Some((1, 3)));
        assert_eq!(e("(a|ab)(c|bcd)").find_from(b"abcd", 0), Some((0, 4)));
        assert_eq!(e("x*").find_from(b"abc", 0), Some((0, 0)));
    }
}
