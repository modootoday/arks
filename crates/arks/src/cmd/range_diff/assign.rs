//! Least-cost assignment for `range-diff` (the "linear assignment" step of
//! git-range-diff's documented algorithm).
//!
//! Primary: <https://git-scm.com/docs/git-range-diff#_algorithm>. The solver
//! is the shortest-augmenting-path method of R. Jonker and A. Volgenant, "A
//! shortest augmenting path algorithm for dense and sparse linear assignment
//! problems", Computing 38 (1987): column reduction, reduction transfer, two
//! rounds of augmenting row reduction, then augmentation.
//!
//! Where several assignments cost the same, stock git's choice depends on the
//! solver's scan order; the order here was fitted to observed `git range-diff`
//! output (creation-factor sweeps). Observed quirk kept on purpose: when the
//! first minimum scan of an augmentation already reaches a free column, the
//! augmenting path ends at the last column that scan visited, so stock git can
//! leave commits unpaired that a strict optimum would pair (for example three
//! old commits against one new one at any creation factor).

/// Assign each column to a row of the square `n`×`n` matrix `cost(col, row)`.
/// Returns `col → row` (`-1`: left unassigned by the quirk above).
// The scans grow `up` inside loops whose bound was fixed on entry, as the
// published algorithm does; index loops keep its notation.
#[allow(clippy::needless_range_loop, clippy::mut_range_bound)]
pub fn assign(n: usize, cost: &dyn Fn(usize, usize) -> i64) -> Vec<i64> {
    let mut col2row = vec![-1i64; n];
    let mut row2col = vec![-1i64; n];
    if n == 0 {
        return col2row;
    }
    if n == 1 {
        col2row[0] = 0;
        return col2row;
    }
    let c = |col: usize, row: usize| cost(col, row);
    let mut v = vec![0i64; n];
    // Column reduction, last column first: each column takes its cheapest
    // row (the first on ties) when that row is still free.
    let mut times = vec![0usize; n];
    for j in (0..n).rev() {
        let mut best = 0;
        for i in 1..n {
            if c(j, i) < c(j, best) {
                best = i;
            }
        }
        v[j] = c(j, best);
        times[best] += 1;
        if times[best] == 1 {
            row2col[best] = j as i64;
            col2row[j] = best as i64;
        } else {
            col2row[j] = -1;
        }
    }
    // Reduction transfer.
    let mut free: Vec<usize> = Vec::new();
    for i in 0..n {
        if times[i] == 0 {
            free.push(i);
        } else if times[i] == 1 {
            let j1 = row2col[i] as usize;
            let mut min = i64::MAX;
            for j in 0..n {
                if j != j1 {
                    min = min.min(c(j, i) - v[j]);
                }
            }
            v[j1] -= min;
        }
    }
    if free.is_empty() {
        return col2row;
    }
    // Augmenting row reduction, two rounds.
    for _ in 0..2 {
        let mut work = free.clone();
        let mut next: Vec<usize> = Vec::new();
        let mut k = 0;
        while k < work.len() {
            let i = work[k];
            k += 1;
            let mut j1 = 0usize;
            let mut u1 = c(0, i) - v[0];
            let mut j2: Option<usize> = None;
            let mut u2 = i64::MAX;
            for j in 1..n {
                let h = c(j, i) - v[j];
                if h < u2 {
                    if h > u1 {
                        u2 = h;
                        j2 = Some(j);
                    } else {
                        u2 = u1;
                        u1 = h;
                        j2 = Some(j1);
                        j1 = j;
                    }
                }
            }
            let j2 = match j2 {
                Some(j) => j,
                None => {
                    u2 = u1;
                    j1
                }
            };
            let mut i0 = col2row[j1];
            if u1 < u2 {
                v[j1] -= u2 - u1;
            } else if i0 >= 0 {
                j1 = j2;
                i0 = col2row[j1];
            }
            row2col[i] = j1 as i64;
            col2row[j1] = i as i64;
            if i0 >= 0 {
                if u1 < u2 {
                    k -= 1;
                    work[k] = i0 as usize;
                } else {
                    next.push(i0 as usize);
                }
            }
        }
        free = next;
    }
    // Augmentation.
    let mut d = vec![0i64; n];
    let mut pred = vec![0usize; n];
    let mut cols: Vec<usize> = (0..n).collect();
    for &f in &free {
        for j in 0..n {
            d[j] = c(j, f) - v[j];
            pred[j] = f;
            cols[j] = j;
        }
        let (mut low, mut up) = (0usize, 0usize);
        let mut last = 0usize;
        let mut min = 0i64;
        // The column the scans visited last (see the module comment).
        let mut seen: Option<usize> = None;
        let end: usize = 'search: loop {
            if up == low {
                last = low;
                min = d[cols[up]];
                up += 1;
                for k in up..n {
                    let j = cols[k];
                    seen = Some(j);
                    let h = d[j];
                    if h <= min {
                        if h < min {
                            up = low;
                            min = h;
                        }
                        cols[k] = cols[up];
                        cols[up] = j;
                        up += 1;
                    }
                }
                for k in low..up {
                    if col2row[cols[k]] < 0 {
                        break 'search seen.unwrap_or(cols[k]);
                    }
                }
            }
            let j1 = cols[low];
            low += 1;
            let i = col2row[j1] as usize;
            let u1 = c(j1, i) - v[j1] - min;
            for k in up..n {
                let j = cols[k];
                seen = Some(j);
                let h = c(j, i) - v[j] - u1;
                if h < d[j] {
                    d[j] = h;
                    pred[j] = i;
                    if h == min {
                        if col2row[j] < 0 {
                            break 'search j;
                        }
                        cols[k] = cols[up];
                        cols[up] = j;
                        up += 1;
                    }
                }
            }
        };
        for &j in &cols[..last] {
            v[j] += d[j] - min;
        }
        let mut j = end;
        loop {
            let i = pred[j];
            col2row[j] = i as i64;
            let prev = row2col[i];
            row2col[i] = j as i64;
            if i == f || prev < 0 {
                break;
            }
            j = prev as usize;
        }
    }
    col2row
}

#[cfg(test)]
mod tests {
    use super::*;

    fn total(m: &[Vec<i64>], a: &[i64]) -> i64 {
        a.iter()
            .enumerate()
            .map(|(col, &row)| m[row as usize][col])
            .sum()
    }

    #[test]
    fn solves_small_matrices_optimally() {
        // rows × cols, cost(col, row) = m[row][col].
        let m = vec![vec![4, 1, 3], vec![2, 0, 5], vec![3, 2, 2]];
        let a = assign(3, &|c, r| m[r][c]);
        assert_eq!(total(&m, &a), 5);
    }

    #[test]
    fn three_against_one_leaves_pairs_open() {
        // Observed stock behaviour: three old commits (columns 0..3) against
        // one new commit (row 0) stay unpaired even with huge creation costs.
        let m = [
            [13, 14, 7, 39],
            [89, 99, 29, 0],
            [89, 99, 29, 0],
            [89, 99, 29, 0],
        ];
        let a = assign(4, &|c, r| m[r][c]);
        assert!(a[..3].iter().all(|&r| r != 0));
    }
}
