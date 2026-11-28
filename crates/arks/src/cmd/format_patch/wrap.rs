//! Greedy word wrapping of one line of text, as stock Git 2.39.5 lays out a
//! long plain `Subject:` (width 78, continuation indent 1) and the cover
//! letter's shortlog entries (width 72, indents 2 and 4).
//!
//! Rules observed in the clean-room image: a word ends at whitespace and is
//! placed when the column after it is at most `width`, or when it is the
//! first word of a line; a run of whitespace keeps all but the character the
//! break replaces; a tab advances to the next multiple of eight; a word
//! longer than a line stands alone on its line.

/// Where the first line starts.
#[derive(Clone, Copy)]
pub enum Lead {
    /// The line already holds `n` columns (a header name and prefix).
    At(usize),
    /// The line starts empty and is indented by `n` spaces.
    Indent(usize),
}

/// `text` wrapped to `width` columns; continuation lines are indented by
/// `indent` spaces. No newline follows the last line.
pub fn wrap(text: &str, lead: Lead, indent: usize, width: usize) -> String {
    let chars: Vec<char> = text.chars().collect();
    let mut out = String::new();
    // Start of the text not yet written: `Some(i)` continues the current
    // line from `i`; `None` is a fresh line that starts at `bol` after
    // `pad` spaces.
    let (mut w, mut pending, mut pad) = match lead {
        Lead::At(col) => (col, Some(0), 0),
        Lead::Indent(n) => (n, None, n),
    };
    let mut bol = 0;
    let mut i = 0;
    loop {
        let c = chars.get(i).copied();
        if c.is_some_and(|c| !c.is_whitespace()) {
            w += 1;
            i += 1;
            continue;
        }
        if w <= width || pending.is_none() {
            let start = match pending {
                Some(s) => s,
                None if c.is_none() && i == bol => return out,
                None => {
                    out.push_str(&" ".repeat(pad));
                    bol
                }
            };
            out.extend(&chars[start..i]);
            let Some(c) = c else {
                return out;
            };
            pending = Some(i);
            if c == '\t' {
                w |= 7;
            }
            w += 1;
            i += 1;
        } else {
            out.push('\n');
            let s = pending.take().unwrap_or(bol);
            bol = s + usize::from(chars[s].is_whitespace());
            i = bol;
            w = indent;
            pad = indent;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Expected strings are stock Git 2.39.5 cover-letter shortlog lines
    // (`  ` first, `    ` after) from the clean-room image.

    fn entry(s: &str) -> String {
        wrap(s, Lead::Indent(2), 4, 72)
    }

    #[test]
    fn shortlog_entries() {
        assert_eq!(entry("short one"), "  short one");
        assert_eq!(
            entry(
                "a very long subject line that has many words and goes beyond the seventy eight limit and then keeps on going"
            ),
            "  a very long subject line that has many words and goes beyond the\n    seventy eight limit and then keeps on going"
        );
        assert_eq!(
            entry(
                "tabs\there\tand  double  spaces  aaaaaaaaa bbbbbbbbb ccccccccc ddddddddd eeeeeeeee fff"
            ),
            "  tabs\there\tand  double  spaces  aaaaaaaaa bbbbbbbbb ccccccccc\n    ddddddddd eeeeeeeee fff"
        );
        let long = "x".repeat(78);
        assert_eq!(entry(&format!("{long} yy")), format!("  {long}\n    yy"));
        // 72 columns fit; 73 do not.
        assert_eq!(
            entry("aaaaaaaaa bbbbbbbbb ccccccccc ddddddddd eeeeeeeee fffffffff ggggggg hh"),
            "  aaaaaaaaa bbbbbbbbb ccccccccc ddddddddd eeeeeeeee fffffffff ggggggg hh"
        );
        assert_eq!(
            entry("aaaaaaaaa bbbbbbbbb ccccccccc ddddddddd eeeeeeeee fffffffff gggggggg hh"),
            "  aaaaaaaaa bbbbbbbbb ccccccccc ddddddddd eeeeeeeee fffffffff gggggggg\n    hh"
        );
    }
}
