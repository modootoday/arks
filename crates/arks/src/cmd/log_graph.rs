//! `log --graph`: the ASCII history graph drawn to the left of each commit.
//!
//! Clean-room implementation. Behaviour is taken from the git-log
//! documentation (`--graph`, which implies `--topo-order`; "History
//! Simplification"; "Commit Ordering") and from observing stock git output
//! on test repositories. No code from git's `graph.c` is used.
//!
//! Model: the graph is a row of *lanes*; each lane waits for one commit.
//! Every lane occupies two screen cells (the edge character and a gap).
//! Showing a commit replaces its lane with lanes for its parents, which
//! yields the lane row for the next commit. The screen rows emitted per
//! commit are:
//!
//! * `PreCommit` — only for octopus merges (three or more parents) that are
//!   not in the rightmost lane: lanes right of the commit are pushed
//!   outwards (`\`) to make room for the extra parent edges.
//! * `Commit` — the `*` row; octopus merges draw `-` / `.` towards their
//!   extra parents.
//! * `PostMerge` — merges only: one edge per parent (`|`, `\`, or `/` when
//!   the first parent lies to the left).
//! * `Collapsing` — edges walk left one cell per row (`/`, or `_` for a
//!   long horizontal run) until every edge sits in its lane.
//! * `Padding` — the steady state: one `|` per lane.
//!
//! Every row is padded with spaces to the width of the current commit's
//! layout.
//!
//! Primary: <https://git-scm.com/docs/git-log> (`--graph`, `--topo-order`).

/// Which kind of screen row the graph emits next.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Row {
    Padding,
    /// The previous commit's rows were not all shown (git draws `...`).
    Skip,
    PreCommit,
    Commit,
    PostMerge,
    Collapsing,
}

/// Graph state between commits. `T` identifies a commit.
pub(crate) struct Graph<T> {
    /// Lanes before the current commit is shown.
    before: Vec<T>,
    /// Lanes after the current commit is shown.
    after: Vec<T>,
    /// Per screen cell: the `after` lane the edge in that cell heads to.
    cells: Vec<Option<usize>>,
    /// Cells as they stood at the end of the last collapsing row.
    settled: Vec<Option<usize>>,
    /// Cells still holding meaningful edges.
    live: usize,
    /// Screen width every row is padded to.
    width: usize,
    commit: Option<T>,
    parents: Vec<T>,
    /// Lane index of the current / previous commit.
    at: usize,
    prev_at: usize,
    /// For merges: `Some(true)` when the first parent sits left of the
    /// commit (the merge leans left and needs no extra lane).
    leans_left: Option<bool>,
    /// Net lanes a merge adds to the right (`-1` when the second parent
    /// joins the lane immediately to the right).
    added: isize,
    prev_added: isize,
    row: Row,
    prev_row: Row,
    /// Pre-commit rows already emitted for an octopus merge.
    expansion: usize,
}

impl<T: Copy + Eq> Default for Graph<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T: Copy + Eq> Graph<T> {
    pub(crate) fn new() -> Self {
        Graph {
            before: Vec::new(),
            after: Vec::new(),
            cells: Vec::new(),
            settled: Vec::new(),
            live: 0,
            width: 0,
            commit: None,
            parents: Vec::new(),
            at: 0,
            prev_at: 0,
            leans_left: None,
            added: 0,
            prev_added: 0,
            row: Row::Padding,
            prev_row: Row::Padding,
            expansion: 0,
        }
    }

    /// Advance to `commit`, whose parents (in order) that are part of the
    /// shown history are `parents`.
    pub(crate) fn next_commit(&mut self, commit: T, parents: Vec<T>) {
        self.commit = Some(commit);
        self.parents = parents;
        self.prev_at = self.at;
        self.prev_added = self.added;
        self.added = 0;
        self.leans_left = None;
        self.before = std::mem::take(&mut self.after);
        self.cells = vec![None; 2 * (self.before.len() + self.parents.len()) + 2];
        self.width = 0;
        let n = self.before.len();
        let mut found = false;
        for i in 0..=n {
            let lane = match self.before.get(i) {
                Some(l) => *l,
                None if found => break,
                None => commit,
            };
            if lane == commit && !found {
                found = true;
                self.at = i;
                if self.parents.is_empty() {
                    self.width += 2;
                }
                for p in self.parents.clone() {
                    self.route(p, Some(i));
                }
            } else {
                self.route(lane, None);
            }
        }
        self.live = self.width;
        while self.live > 1 && self.cells[self.live - 1].is_none() {
            self.live -= 1;
        }
        self.expansion = 0;
        self.row = if self.row != Row::Padding {
            Row::Skip
        } else if self.wants_pre_commit() {
            Row::PreCommit
        } else {
            Row::Commit
        };
    }

    /// Give `oid` a lane in `after` (reusing an existing one) and record
    /// which screen cell its edge starts from. `from` is the commit's lane
    /// when `oid` is one of its parents.
    fn route(&mut self, oid: T, from: Option<usize>) {
        let lane = match self.after.iter().position(|l| *l == oid) {
            Some(l) => l,
            None => {
                self.after.push(oid);
                self.after.len() - 1
            }
        };
        let cell = match from {
            Some(at) if self.parents.len() > 1 && self.leans_left.is_none() => {
                // First parent of a merge: it decides the merge's shape.
                let dist = at as isize - lane as isize;
                let left = dist > 0;
                self.leans_left = Some(left);
                let n = self.parents.len() as isize;
                if left {
                    self.added = n - 2;
                    let back = if dist > 1 { 2 * dist - 3 } else { 1 };
                    (self.width as isize - back) as usize
                } else {
                    self.added = n - 1;
                    self.width += 2;
                    self.width - 2
                }
            }
            _ if self.added > 0 && self.width >= 2 && self.cells[self.width - 2] == Some(lane) => {
                // The edge joins the lane just placed; no new lane needed.
                self.added = -1;
                self.width - 2
            }
            _ => {
                self.width += 2;
                self.width - 2
            }
        };
        self.cells[cell] = Some(lane);
    }

    /// Parents of an octopus merge reached through `-` / `.` on the commit
    /// row: all but the two (three when leaning left) the post-merge row
    /// draws directly.
    fn dashed_parents(&self) -> usize {
        let direct = if self.leans_left == Some(true) { 3 } else { 2 };
        self.parents.len().saturating_sub(direct)
    }

    /// Octopus merges left of other lanes push those lanes right by one
    /// cell per pre-commit row, two cells per dashed parent.
    fn wants_pre_commit(&self) -> bool {
        self.parents.len() >= 3
            && self.at + 1 < self.before.len()
            && self.expansion < 2 * self.dashed_parents()
    }

    fn set_row(&mut self, row: Row) {
        self.prev_row = self.row;
        self.row = row;
    }

    /// True when the next row is the commit's `*` row.
    pub(crate) fn commit_row_next(&self) -> bool {
        self.row == Row::Commit
    }

    /// True once the current commit needs no more graph rows.
    pub(crate) fn finished(&self) -> bool {
        self.row == Row::Padding
    }

    /// Every edge is in (or one cell right of) its lane.
    fn settled_cells(&self) -> bool {
        self.cells[..self.live]
            .iter()
            .enumerate()
            .all(|(i, t)| t.is_none_or(|t| t == i / 2))
    }

    fn pad(&self, mut s: String) -> String {
        while s.len() < self.width {
            s.push(' ');
        }
        s
    }

    /// The next graph row (without newline), advancing the state.
    pub(crate) fn next_row(&mut self) -> String {
        let s = match self.row {
            // Steady state: the remembered previous row kind is kept.
            Row::Padding => "| ".repeat(self.after.len()),
            Row::Skip => {
                let next = if self.wants_pre_commit() {
                    Row::PreCommit
                } else {
                    Row::Commit
                };
                self.set_row(next);
                "...".to_string()
            }
            Row::PreCommit => self.pre_commit_row(),
            Row::Commit => self.commit_row(),
            Row::PostMerge => self.post_merge_row(),
            Row::Collapsing => self.collapsing_row(),
        };
        self.pad(s)
    }

    /// Row printed between two commit records (non-oneline formats): plain
    /// lanes when the commit row is next, otherwise the next row.
    pub(crate) fn separator_row(&mut self) -> String {
        if self.row != Row::Commit {
            return self.next_row();
        }
        let mut s = String::new();
        for lane in &self.before {
            s.push('|');
            if Some(*lane) == self.commit && self.parents.len() > 2 {
                s.push_str(&" ".repeat(2 * (self.parents.len() - 2)));
            } else {
                s.push(' ');
            }
        }
        self.prev_row = Row::Padding;
        self.pad(s)
    }

    /// Lanes as shown on the commit row: the commit's own lane is appended
    /// when it was not waited for.
    fn shown_lanes(&self) -> Vec<T> {
        let mut lanes = self.before.clone();
        if let Some(c) = self.commit {
            if !lanes.contains(&c) {
                lanes.push(c);
            }
        }
        lanes
    }

    fn pre_commit_row(&mut self) -> String {
        let mut s = String::new();
        let mut past = false;
        for (i, lane) in self.before.iter().enumerate() {
            if Some(*lane) == self.commit {
                past = true;
                s.push('|');
                s.push_str(&" ".repeat(self.expansion));
            } else if past && self.expansion == 0 {
                let lean = self.prev_row == Row::PostMerge && self.prev_at < i;
                s.push(if lean { '\\' } else { '|' });
            } else {
                s.push(if past { '\\' } else { '|' });
            }
            s.push(' ');
        }
        self.expansion += 1;
        if !self.wants_pre_commit() {
            self.set_row(Row::Commit);
        }
        s
    }

    fn commit_row(&mut self) -> String {
        let mut s = String::new();
        let mut past = false;
        for (i, lane) in self.shown_lanes().into_iter().enumerate() {
            if Some(lane) == self.commit {
                past = true;
                s.push('*');
                s.push_str(&self.octopus_dashes());
            } else if past && self.added > 1 {
                s.push('\\');
            } else if past && self.added == 1 {
                let lean =
                    self.prev_row == Row::PostMerge && self.prev_added > 0 && self.prev_at < i;
                s.push(if lean { '\\' } else { '|' });
            } else if self.prev_row == Row::Collapsing
                && self.settled.get(2 * i + 1).copied().flatten() == Some(i)
                && self
                    .cells
                    .get(2 * i)
                    .copied()
                    .flatten()
                    .is_some_and(|t| t < i)
            {
                s.push('/');
            } else {
                s.push('|');
            }
            s.push(' ');
        }
        let next = if self.parents.len() > 1 {
            Row::PostMerge
        } else if self.settled_cells() {
            Row::Padding
        } else {
            Row::Collapsing
        };
        self.set_row(next);
        s
    }

    /// `-` / `.` drawn right of an octopus merge's `*`, one pair per parent
    /// beyond those the post-merge row can draw directly.
    fn octopus_dashes(&self) -> String {
        let extra = self.dashed_parents();
        if extra == 0 {
            return String::new();
        }
        let mut s = "-".repeat(2 * extra - 1);
        s.push('.');
        s
    }

    fn post_merge_row(&mut self) -> String {
        let mut s = String::new();
        let first = self.parents.first().copied();
        let mut first_seen = false;
        let mut past = false;
        let lean_left = self.leans_left == Some(true);
        for (i, lane) in self.shown_lanes().into_iter().enumerate() {
            if Some(lane) == self.commit {
                past = true;
                // Edge glyphs per parent: `/` (first parent, leaning
                // left), `|` (straight down), then `\` for the rest.
                let glyphs: &[char] = if lean_left { &['/', '|'] } else { &['|'] };
                let n = self.parents.len();
                for k in 0..n {
                    match glyphs.get(k) {
                        Some(g) => s.push(*g),
                        None => {
                            s.push('\\');
                            if self.added > 0 || k + 1 < n {
                                s.push(' ');
                            }
                        }
                    }
                }
                if self.added == 0 {
                    s.push(' ');
                }
            } else if past {
                s.push(if self.added > 0 { '\\' } else { '|' });
                s.push(' ');
            } else {
                s.push('|');
                let hugs = lean_left && i + 1 == self.at;
                if !hugs {
                    s.push(if first_seen { '_' } else { ' ' });
                }
            }
            if Some(lane) == first {
                first_seen = true;
            }
        }
        let next = if self.settled_cells() {
            Row::Padding
        } else {
            Row::Collapsing
        };
        self.set_row(next);
        s
    }

    fn collapsing_row(&mut self) -> String {
        let n = self.live;
        let mut moved: Vec<Option<usize>> = vec![None; n];
        // (cell the run starts from, its target lane)
        let mut run: Option<(usize, usize)> = None;
        for i in 0..n {
            let Some(t) = self.cells[i] else { continue };
            let home = 2 * t;
            if home == i {
                moved[i] = Some(t);
            } else if moved[i - 1] == Some(t) {
                // Shares the edge just to the left: they merge here.
            } else {
                // Step one cell left, or two when crossing the edge there.
                let crossing = moved[i - 1].is_some();
                moved[i - 1 - usize::from(crossing)] = Some(t);
                // The first edge that moves may also draw a horizontal run
                // (`_`) over the lanes between it and its target.
                if run.is_none() {
                    run = Some((i - usize::from(crossing), t));
                    for j in (home + 3..i.saturating_sub(2)).step_by(2) {
                        moved[j] = Some(t);
                    }
                }
            }
        }
        self.settled = moved.clone();
        let mut live = n;
        if live > 0 && moved[live - 1].is_none() {
            live -= 1;
        }
        let mut s = String::new();
        let mut drew_run = false;
        // Run cells after the first and edges left of a drawn run end
        // here; they do not continue into the next row.
        for (i, cell) in moved.iter_mut().enumerate().take(live) {
            let Some(t) = *cell else {
                s.push(' ');
                continue;
            };
            if 2 * t == i {
                s.push('|');
            } else if run.is_some_and(|(st, tg)| tg == t && i + 1 != st) {
                if i != 2 * t + 3 {
                    *cell = None;
                }
                drew_run = true;
                s.push('_');
            } else {
                if drew_run && run.is_some_and(|(st, _)| i < st) {
                    *cell = None;
                }
                s.push('/');
            }
        }
        self.cells = moved;
        self.live = live;
        if self.settled_cells() {
            self.set_row(Row::Padding);
        }
        s
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Render `--graph --format=%s`-style output for a DAG given in display
    /// order as (name, parents).
    fn render(dag: &[(char, &str)]) -> String {
        let mut g = Graph::new();
        let mut out = String::new();
        for (c, ps) in dag {
            g.next_commit(*c, ps.chars().collect());
            out.push_str(&g.next_row());
            out.push(*c);
            out.push('\n');
            while !g.finished() {
                out.push_str(&g.next_row());
                out.push('\n');
            }
        }
        out
    }

    #[test]
    fn linear_history() {
        assert_eq!(
            render(&[('c', "b"), ('b', "a"), ('a', "")]),
            "* c\n* b\n* a\n"
        );
    }

    #[test]
    fn simple_merge() {
        let got = render(&[('m', "ab"), ('b', "r"), ('a', "r"), ('r', "")]);
        assert_eq!(got, "*   m\n|\\  \n| * b\n* | a\n|/  \n* r\n");
    }

    #[test]
    fn octopus_merge_and_late_tip() {
        // Expected text observed from stock git 2.43 on the same DAG.
        let got = render(&[
            ('m', "abc"),
            ('c', "r"),
            ('b', "r"),
            ('a', "r"),
            ('s', "r"),
            ('r', ""),
        ]);
        let want = "*-.   m\n|\\ \\  \n| | * c\n| * | b\n| |/  \n* / a\n|/  \n| * s\n|/  \n* r\n";
        assert_eq!(got, want);
    }
}
