//! Myers' difference algorithm, written for RapidR from the paper: Eugene
//! W. Myers, "An O(ND) Difference Algorithm and Its Variations",
//! Algorithmica 1 (1986), pp. 251–266. Nothing here comes from another
//! implementation (docs/licensing.md §8, item 6).
//!
//! The paper's terms, which the code uses:
//! - the **edit graph** of sequences A (length N) and B (length M) has a
//!   point (x, y) for 0 ≤ x ≤ N, 0 ≤ y ≤ M; a step right deletes A's x-th
//!   element, a step down inserts B's y-th, a diagonal step is free where
//!   the two elements are equal;
//! - **diagonal k** holds the points with x − y = k; a **snake** is a run
//!   of diagonal steps; a **D-path** starts at (0, 0) and takes D
//!   non-diagonal steps;
//! - §2: the furthest-reaching D-path on diagonal k extends the
//!   furthest-reaching (D − 1)-path on diagonal k − 1 (a step right) or
//!   k + 1 (a step down), whichever gets further, by its snake;
//! - §4b: searching forward from (0, 0) and in reverse from (N, M) at
//!   once, the two meet at D = ⌈D_total / 2⌉ on a **middle snake** that
//!   lies on an optimal path; the parts before and after it are solved the
//!   same way, in space linear in N + M.
//!
//! Two additions of RapidR's own, both keeping the result a correct diff:
//! - elements found on one side only can't be in any common subsequence,
//!   so they are set aside first (two texts with nothing in common cost
//!   nothing);
//! - a search that goes on past [`LIMIT`] steps (huge, very different
//!   inputs made of a few repeated elements) splits where its forward
//!   search got furthest instead of at the middle snake: the diff is then
//!   still correct, but may not be the smallest one.

/// Which elements of each side are not in the common subsequence found:
/// deleted from the old sequence, inserted into the new one.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Changes {
    pub old: Vec<bool>,
    pub new: Vec<bool>,
}

/// Search steps (D, per half) past which a box is split approximately.
pub const LIMIT: isize = 1024;

/// "No path reaches this diagonal" in the forward and the reverse search.
const NONE_FWD: isize = isize::MIN / 4;
const NONE_REV: isize = isize::MAX / 4;

/// The changes between `old` and `new` (sequences of symbol ids).
pub fn diff(old: &[u32], new: &[u32]) -> Changes {
    let mut out = Changes { old: vec![true; old.len()], new: vec![true; new.len()] };
    if old.is_empty() || new.is_empty() {
        return out;
    }
    // Only symbols both sides have can be kept.
    let alphabet = old.iter().chain(new).copied().max().unwrap_or(0) as usize + 1;
    let mut sides = vec![0u8; alphabet];
    for &s in old {
        sides[s as usize] |= 1;
    }
    for &s in new {
        sides[s as usize] |= 2;
    }
    let keep_old: Vec<usize> = (0..old.len()).filter(|&i| sides[old[i] as usize] == 3).collect();
    let keep_new: Vec<usize> = (0..new.len()).filter(|&j| sides[new[j] as usize] == 3).collect();
    if keep_old.is_empty() || keep_new.is_empty() {
        return out;
    }
    let a: Vec<u32> = keep_old.iter().map(|&i| old[i]).collect();
    let b: Vec<u32> = keep_new.iter().map(|&j| new[j]).collect();
    let (ca, cb) = Search::new(&a, &b).run();
    for (k, &i) in keep_old.iter().enumerate() {
        out.old[i] = ca[k];
    }
    for (k, &j) in keep_new.iter().enumerate() {
        out.new[j] = cb[k];
    }
    out
}

/// One comparison: the two sequences, what is changed so far, and the
/// furthest points of the forward and the reverse search per diagonal.
struct Search<'a> {
    a: &'a [u32],
    b: &'a [u32],
    del: Vec<bool>,
    ins: Vec<bool>,
    /// The forward search: the x where the furthest-reaching path on
    /// diagonal k ends (index k + `zero`).
    fwd: Vec<isize>,
    /// The reverse search: the smallest x a path from the box's far corner
    /// reaches on diagonal k.
    rev: Vec<isize>,
    zero: isize,
}

impl<'a> Search<'a> {
    fn new(a: &'a [u32], b: &'a [u32]) -> Search<'a> {
        // diagonals of any box: k in −(N + M) − 1 ..= (N + M) + 1
        let span = (a.len() + b.len()) as isize + 2;
        let size = (2 * span + 1) as usize;
        Search { a, b, del: vec![false; a.len()], ins: vec![false; b.len()], fwd: vec![NONE_FWD; size], rev: vec![NONE_REV; size], zero: span }
    }

    fn at(&self, k: isize) -> usize {
        (k + self.zero) as usize
    }

    /// Solves the whole comparison, box by box (an explicit stack: no
    /// recursion, so long inputs can't overflow the thread's stack).
    fn run(mut self) -> (Vec<bool>, Vec<bool>) {
        let mut boxes = vec![(0usize, self.a.len(), 0usize, self.b.len())];
        while let Some((mut x0, mut x1, mut y0, mut y1)) = boxes.pop() {
            // a common start and end are diagonal steps: kept
            while x0 < x1 && y0 < y1 && self.a[x0] == self.b[y0] {
                x0 += 1;
                y0 += 1;
            }
            while x0 < x1 && y0 < y1 && self.a[x1 - 1] == self.b[y1 - 1] {
                x1 -= 1;
                y1 -= 1;
            }
            if x0 == x1 {
                self.ins[y0..y1].iter_mut().for_each(|c| *c = true);
            } else if y0 == y1 {
                self.del[x0..x1].iter_mut().for_each(|c| *c = true);
            } else {
                let ((sx, sy), (ex, ey)) = self.middle_snake(x0, x1, y0, y1);
                // (the snake's own steps are diagonal: kept)
                boxes.push((ex, x1, ey, y1));
                boxes.push((x0, sx, y0, sy));
            }
        }
        (self.del, self.ins)
    }

    /// The middle snake of the box `x0..x1` × `y0..y1` (both sides non-empty,
    /// their first and last elements different), as its start and end
    /// points.
    fn middle_snake(&mut self, x0: usize, x1: usize, y0: usize, y1: usize) -> ((usize, usize), (usize, usize)) {
        let a = &self.a[x0..x1];
        let b = &self.b[y0..y1];
        let (n, m) = (a.len() as isize, b.len() as isize);
        // the reverse search's diagonals are centred on the far corner's
        let delta = n - m;
        let odd = delta.rem_euclid(2) == 1;
        let abs = |x: isize, y: isize| (x0 + x as usize, y0 + y as usize);

        // D = 0: the start's and the end's own snakes (none: the box's ends
        // differ)
        let i = self.at(0);
        self.fwd[i] = 0;
        let i = self.at(delta);
        self.rev[i] = n;
        let mut d = 1isize;
        loop {
            // the forward search's D-paths
            let mut k = -d;
            while k <= d {
                let mut x = NONE_FWD;
                // a step down from diagonal k + 1
                if k < d {
                    let from = self.fwd[self.at(k + 1)];
                    if from != NONE_FWD && from - k <= m {
                        x = from;
                    }
                }
                // a step right from diagonal k − 1
                if k > -d {
                    let from = self.fwd[self.at(k - 1)];
                    if from != NONE_FWD && from < n {
                        x = x.max(from + 1);
                    }
                }
                if x != NONE_FWD {
                    let (sx, sy) = (x, x - k);
                    let mut y = sy;
                    while x < n && y < m && a[x as usize] == b[y as usize] {
                        x += 1;
                        y += 1;
                    }
                    // overlap with the reverse search's (D − 1)-path:
                    // D_total = 2D − 1, and this snake is the middle one
                    if odd && (k - delta).abs() < d && x >= self.rev[self.at(k)] {
                        let i = self.at(k);
                        self.fwd[i] = x;
                        return (abs(sx, sy), abs(x, y));
                    }
                }
                let i = self.at(k);
                self.fwd[i] = x;
                k += 2;
            }
            // the reverse search's D-paths, from (N, M) back
            let mut k = delta - d;
            while k <= delta + d {
                let mut x = NONE_REV;
                // a step left from diagonal k + 1
                if k < delta + d {
                    let from = self.rev[self.at(k + 1)];
                    if from != NONE_REV && from > 0 {
                        x = from - 1;
                    }
                }
                // a step up from diagonal k − 1
                if k > delta - d {
                    let from = self.rev[self.at(k - 1)];
                    if from != NONE_REV && from - k >= 0 {
                        x = x.min(from);
                    }
                }
                if x != NONE_REV {
                    let (ex, ey) = (x, x - k);
                    let mut y = ey;
                    while x > 0 && y > 0 && a[x as usize - 1] == b[y as usize - 1] {
                        x -= 1;
                        y -= 1;
                    }
                    // overlap with the forward search's D-path:
                    // D_total = 2D, and this snake is the middle one
                    if !odd && k.abs() <= d && x <= self.fwd[self.at(k)] {
                        let i = self.at(k);
                        self.rev[i] = x;
                        return (abs(x, y), abs(ex, ey));
                    }
                }
                let i = self.at(k);
                self.rev[i] = x;
                k += 2;
            }
            if d >= LIMIT {
                // Too costly to be exact: split where the forward search got
                // furthest (x + y largest) — a point some D-path reaches, not
                // the box's start or end.
                let mut best = (0isize, 0isize);
                let mut k = -d;
                while k <= d {
                    let x = self.fwd[self.at(k)];
                    if x != NONE_FWD && x + (x - k) > best.0 + best.1 && x <= n && x - k <= m && (x, x - k) != (n, m) {
                        best = (x, x - k);
                    }
                    k += 2;
                }
                if best != (0, 0) {
                    return (abs(best.0, best.1), abs(best.0, best.1));
                }
            }
            d += 1;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The longest common subsequence's length (dynamic programming), the
    /// check of minimality.
    fn lcs(a: &[u32], b: &[u32]) -> usize {
        let mut row = vec![0usize; b.len() + 1];
        for &x in a {
            let mut diag = 0;
            for (j, &y) in b.iter().enumerate() {
                let up = row[j + 1];
                row[j + 1] = if x == y { diag + 1 } else { up.max(row[j]) };
                diag = up;
            }
        }
        row[b.len()]
    }

    /// The kept elements, in order, are the same on both sides (a correct
    /// diff); their count.
    fn kept(a: &[u32], b: &[u32], c: &Changes) -> usize {
        let ka: Vec<u32> = a.iter().zip(&c.old).filter(|(_, ch)| !**ch).map(|(s, _)| *s).collect();
        let kb: Vec<u32> = b.iter().zip(&c.new).filter(|(_, ch)| !**ch).map(|(s, _)| *s).collect();
        assert_eq!(ka, kb, "{a:?} / {b:?}: the kept elements differ");
        ka.len()
    }

    /// A small xorshift: tests need no dependency.
    struct Rng(u64);
    impl Rng {
        fn below(&mut self, n: u64) -> u64 {
            self.0 ^= self.0 << 13;
            self.0 ^= self.0 >> 7;
            self.0 ^= self.0 << 17;
            self.0 % n.max(1)
        }
    }

    #[test]
    fn small_cases() {
        assert_eq!(diff(&[], &[]), Changes::default());
        assert_eq!(diff(&[1, 2], &[]), Changes { old: vec![true, true], new: vec![] });
        assert_eq!(diff(&[], &[7]), Changes { old: vec![], new: vec![true] });
        assert_eq!(diff(&[1, 2, 3], &[1, 2, 3]), Changes { old: vec![false; 3], new: vec![false; 3] });
        assert_eq!(diff(&[1, 2], &[3, 4]), Changes { old: vec![true; 2], new: vec![true; 2] });
        // the paper's own example: A = abcabba, B = cbabac, D = 5
        let (a, b) = ([0, 1, 2, 0, 1, 1, 0], [2, 1, 0, 1, 0, 2]);
        let c = diff(&a, &b);
        assert_eq!(kept(&a, &b, &c), 4);
        assert_eq!(c.old.iter().filter(|x| **x).count() + c.new.iter().filter(|x| **x).count(), 5);
    }

    /// On random small inputs the kept elements are a longest common
    /// subsequence: the diff is minimal.
    #[test]
    fn minimal_on_random_inputs() {
        let mut rng = Rng(0x2545_F491_4F6C_DD1D);
        for round in 0..6000 {
            let alphabet = 1 + rng.below(if round % 3 == 0 { 3 } else { 9 });
            let (n, m) = (rng.below(16) as usize, rng.below(16) as usize);
            let a: Vec<u32> = (0..n).map(|_| rng.below(alphabet) as u32).collect();
            let b: Vec<u32> = if round % 2 == 0 {
                (0..m).map(|_| rng.below(alphabet) as u32).collect()
            } else {
                let mut b = a.clone();
                for _ in 0..rng.below(5) {
                    let at = rng.below(b.len() as u64 + 1) as usize;
                    if rng.below(2) == 0 && at < b.len() {
                        b.remove(at);
                    } else {
                        b.insert(at, rng.below(alphabet) as u32);
                    }
                }
                b
            };
            let c = diff(&a, &b);
            assert_eq!(kept(&a, &b, &c), lcs(&a, &b), "{a:?} / {b:?}");
        }
    }

    #[test]
    fn long_inputs() {
        let mut rng = Rng(99);
        // few edits in a long sequence: minimal
        let a: Vec<u32> = (0..4000).map(|_| rng.below(60) as u32).collect();
        let mut b = a.clone();
        for _ in 0..50 {
            let at = rng.below(b.len() as u64) as usize;
            b[at] = rng.below(60) as u32;
        }
        let c = diff(&a, &b);
        assert_eq!(kept(&a, &b, &c), lcs(&a, &b));
        // nothing in common: all changed, at once
        let x: Vec<u32> = (0..30_000).collect();
        let y: Vec<u32> = (30_000..60_000).collect();
        let c = diff(&x, &y);
        assert!(c.old.iter().all(|c| *c) && c.new.iter().all(|c| *c));
        // random against random over 3 symbols (past the limit): still a
        // correct diff
        let p: Vec<u32> = (0..12_000).map(|_| rng.below(3) as u32).collect();
        let q: Vec<u32> = (0..12_000).map(|_| rng.below(3) as u32).collect();
        let c = diff(&p, &q);
        kept(&p, &q, &c);
    }
}
