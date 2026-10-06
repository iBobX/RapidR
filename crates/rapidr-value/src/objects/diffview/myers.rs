//! Myers' O(ND) difference algorithm ("An O(ND) Difference Algorithm and
//! Its Variations", 1986) in linear space: the middle snake found from
//! both ends, the two halves solved the same way (an explicit stack, no
//! recursion), as GNU diff's `compareseq` / `diag` do. Written for RapidR;
//! no dependency.
//!
//! Sequences are symbol ids below `alphabet` (lines interned by the
//! caller, or a line's characters). Before the search, symbols found on
//! one side only are set aside — they can't be in any common subsequence,
//! so the longest one is the same without them — which makes two texts
//! with nothing in common free and keeps the search on what can match.
//! A search that grows past a cost limit (pathological inputs: thousands
//! of edits with many repeated symbols) splits at the furthest-reaching
//! diagonal instead of the middle snake (GNU diff's "too expensive"
//! rule): the result is still a correct diff, just not always a minimal
//! one.

/// Which elements of each side aren't in the common subsequence found
/// (deleted from `a`, inserted into `b`).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Changes {
    pub a: Vec<bool>,
    pub b: Vec<bool>,
}

/// The cost (edit distance explored) past which a split is approximate.
const MIN_TOO_EXPENSIVE: usize = 1024;

/// Diffs `a` and `b` (every symbol `< alphabet`).
pub fn diff(a: &[u32], b: &[u32], alphabet: usize) -> Changes {
    let mut out = Changes { a: vec![true; a.len()], b: vec![true; b.len()] };
    // (only symbols both sides have can match)
    let mut seen = vec![0u8; alphabet];
    for &s in a {
        seen[s as usize] |= 1;
    }
    for &s in b {
        seen[s as usize] |= 2;
    }
    let ai: Vec<usize> = (0..a.len()).filter(|&i| seen[a[i] as usize] == 3).collect();
    let bi: Vec<usize> = (0..b.len()).filter(|&i| seen[b[i] as usize] == 3).collect();
    if ai.is_empty() || bi.is_empty() {
        return out;
    }
    let fa: Vec<u32> = ai.iter().map(|&i| a[i]).collect();
    let fb: Vec<u32> = bi.iter().map(|&i| b[i]).collect();
    let (ca, cb) = Search::new(&fa, &fb).run();
    for (k, &i) in ai.iter().enumerate() {
        out.a[i] = ca[k];
    }
    for (k, &i) in bi.iter().enumerate() {
        out.b[i] = cb[k];
    }
    out
}

/// The state of one diff: the sequences, the changed marks, the two
/// furthest-reaching vectors (indexed by diagonal `k = x - y`).
struct Search<'a> {
    a: &'a [u32],
    b: &'a [u32],
    ca: Vec<bool>,
    cb: Vec<bool>,
    fd: Vec<isize>,
    bd: Vec<isize>,
    /// Added to a diagonal to index `fd` / `bd`.
    off: isize,
    too_expensive: usize,
}

impl<'a> Search<'a> {
    fn new(a: &'a [u32], b: &'a [u32]) -> Self {
        let (n, m) = (a.len(), b.len());
        let size = n + m + 3;
        // (about the square root of the diagonals, as GNU diff takes it)
        let mut te = 1usize;
        let mut diags = size;
        while diags != 0 {
            diags >>= 2;
            te <<= 1;
        }
        Search { a, b, ca: vec![false; n], cb: vec![false; m], fd: vec![0; size], bd: vec![0; size], off: m as isize + 1, too_expensive: te.max(MIN_TOO_EXPENSIVE) }
    }

    fn run(mut self) -> (Vec<bool>, Vec<bool>) {
        let mut stack = vec![(0usize, self.a.len(), 0usize, self.b.len())];
        while let Some((mut xoff, mut xlim, mut yoff, mut ylim)) = stack.pop() {
            // (the common start and end are no edits)
            while xoff < xlim && yoff < ylim && self.a[xoff] == self.b[yoff] {
                xoff += 1;
                yoff += 1;
            }
            while xlim > xoff && ylim > yoff && self.a[xlim - 1] == self.b[ylim - 1] {
                xlim -= 1;
                ylim -= 1;
            }
            if xoff == xlim {
                self.cb[yoff..ylim].iter_mut().for_each(|c| *c = true);
            } else if yoff == ylim {
                self.ca[xoff..xlim].iter_mut().for_each(|c| *c = true);
            } else {
                let (xmid, ymid) = self.middle(xoff, xlim, yoff, ylim);
                stack.push((xmid, xlim, ymid, ylim));
                stack.push((xoff, xmid, yoff, ymid));
            }
        }
        (self.ca, self.cb)
    }

    /// Where an optimal path from (xoff, yoff) to (xlim, ylim) crosses the
    /// middle (both boxes non-empty, their ends not matching).
    fn middle(&mut self, xoff: usize, xlim: usize, yoff: usize, ylim: usize) -> (usize, usize) {
        let (a, b, off) = (self.a, self.b, self.off);
        let (xoff, xlim, yoff, ylim) = (xoff as isize, xlim as isize, yoff as isize, ylim as isize);
        let at = |k: isize| (k + off) as usize;
        let dmin = xoff - ylim;
        let dmax = xlim - yoff;
        let fmid = xoff - yoff;
        let bmid = xlim - ylim;
        let (mut fmin, mut fmax, mut bmin, mut bmax) = (fmid, fmid, bmid, bmid);
        let odd = (fmid - bmid) & 1 != 0;
        self.fd[at(fmid)] = xoff;
        self.bd[at(bmid)] = xlim;
        let mut cost = 0usize;
        loop {
            cost += 1;
            // Forward: each diagonal a step further.
            if fmin > dmin {
                fmin -= 1;
                self.fd[at(fmin - 1)] = -1;
            } else {
                fmin += 1;
            }
            if fmax < dmax {
                fmax += 1;
                self.fd[at(fmax + 1)] = -1;
            } else {
                fmax -= 1;
            }
            let mut d = fmax;
            while d >= fmin {
                let (tlo, thi) = (self.fd[at(d - 1)], self.fd[at(d + 1)]);
                let mut x = if tlo >= thi { tlo + 1 } else { thi };
                let mut y = x - d;
                while x < xlim && y < ylim && a[x as usize] == b[y as usize] {
                    x += 1;
                    y += 1;
                }
                self.fd[at(d)] = x;
                if odd && bmin <= d && d <= bmax && self.bd[at(d)] <= x {
                    return (x as usize, y as usize);
                }
                d -= 2;
            }
            // Backward.
            if bmin > dmin {
                bmin -= 1;
                self.bd[at(bmin - 1)] = isize::MAX;
            } else {
                bmin += 1;
            }
            if bmax < dmax {
                bmax += 1;
                self.bd[at(bmax + 1)] = isize::MAX;
            } else {
                bmax -= 1;
            }
            let mut d = bmax;
            while d >= bmin {
                let (tlo, thi) = (self.bd[at(d - 1)], self.bd[at(d + 1)]);
                let mut x = if tlo < thi { tlo } else { thi - 1 };
                let mut y = x - d;
                while x > xoff && y > yoff && a[x as usize - 1] == b[y as usize - 1] {
                    x -= 1;
                    y -= 1;
                }
                self.bd[at(d)] = x;
                if !odd && fmin <= d && d <= fmax && x <= self.fd[at(d)] {
                    return (x as usize, y as usize);
                }
                d -= 2;
            }
            if cost >= self.too_expensive {
                // The furthest-reaching point of either search.
                let (mut fxybest, mut fxbest) = (-1isize, 0isize);
                let mut d = fmax;
                while d >= fmin {
                    let mut x = self.fd[at(d)].min(xlim);
                    let mut y = x - d;
                    if ylim < y {
                        x = ylim + d;
                        y = ylim;
                    }
                    if fxybest < x + y {
                        fxybest = x + y;
                        fxbest = x;
                    }
                    d -= 2;
                }
                let (mut bxybest, mut bxbest) = (isize::MAX, 0isize);
                let mut d = bmax;
                while d >= bmin {
                    let mut x = self.bd[at(d)].max(xoff);
                    let mut y = x - d;
                    if y < yoff {
                        x = yoff + d;
                        y = yoff;
                    }
                    if x + y < bxybest {
                        bxybest = x + y;
                        bxbest = x;
                    }
                    d -= 2;
                }
                let (x, y) = if (xlim + ylim) - bxybest < fxybest - (xoff + yoff) { (fxbest, fxybest - fxbest) } else { (bxbest, bxybest - bxbest) };
                // (never the box's corner: both halves must shrink)
                if (x, y) != (xoff, yoff) && (x, y) != (xlim, ylim) {
                    return (x as usize, y as usize);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The longest common subsequence's length, by dynamic programming.
    fn lcs(a: &[u32], b: &[u32]) -> usize {
        let mut row = vec![0usize; b.len() + 1];
        for &x in a {
            let mut prev = 0;
            for (j, &y) in b.iter().enumerate() {
                let cur = row[j + 1];
                row[j + 1] = if x == y { prev + 1 } else { row[j + 1].max(row[j]) };
                prev = cur;
            }
        }
        row[b.len()]
    }

    /// The unchanged elements pair up in order with equal symbols.
    fn check(a: &[u32], b: &[u32], c: &Changes) -> usize {
        let ka: Vec<u32> = a.iter().zip(&c.a).filter(|(_, &ch)| !ch).map(|(s, _)| *s).collect();
        let kb: Vec<u32> = b.iter().zip(&c.b).filter(|(_, &ch)| !ch).map(|(s, _)| *s).collect();
        assert_eq!(ka, kb, "{a:?} / {b:?}: the kept elements differ");
        ka.len()
    }

    /// A small xorshift (tests need no dependency).
    struct Rng(u64);
    impl Rng {
        fn next(&mut self) -> u64 {
            self.0 ^= self.0 << 13;
            self.0 ^= self.0 >> 7;
            self.0 ^= self.0 << 17;
            self.0
        }
        fn below(&mut self, n: u64) -> u64 {
            self.next() % n
        }
    }

    #[test]
    fn edge_cases() {
        assert_eq!(diff(&[], &[], 1), Changes::default());
        assert_eq!(diff(&[1, 2], &[], 3), Changes { a: vec![true, true], b: vec![] });
        assert_eq!(diff(&[], &[0], 3), Changes { a: vec![], b: vec![true] });
        assert_eq!(diff(&[1, 2, 3], &[1, 2, 3], 4), Changes { a: vec![false; 3], b: vec![false; 3] });
        assert_eq!(diff(&[1, 2], &[3, 4], 5), Changes { a: vec![true; 2], b: vec![true; 2] });
        let c = diff(&[0, 1, 2, 3], &[0, 2, 3, 4], 5);
        assert_eq!(c, Changes { a: vec![false, true, false, false], b: vec![false, false, false, true] });
    }

    /// Property: on random small inputs the kept elements are a common
    /// subsequence as long as the longest one (a minimal diff).
    #[test]
    fn minimal_on_random_small_inputs() {
        let mut rng = Rng(0x9E37_79B9_7F4A_7C15);
        for round in 0..4000 {
            let alphabet = 1 + rng.below(if round % 3 == 0 { 3 } else { 8 }) as usize;
            let (n, m) = (rng.below(14) as usize, rng.below(14) as usize);
            let a: Vec<u32> = (0..n).map(|_| rng.below(alphabet as u64) as u32).collect();
            let b: Vec<u32> = if round % 2 == 0 {
                (0..m).map(|_| rng.below(alphabet as u64) as u32).collect()
            } else {
                // (an edited copy: the usual case)
                let mut b = a.clone();
                for _ in 0..rng.below(4) {
                    let at = rng.below(b.len() as u64 + 1) as usize;
                    if rng.below(2) == 0 && at < b.len() {
                        b.remove(at);
                    } else {
                        b.insert(at, rng.below(alphabet as u64) as u32);
                    }
                }
                b
            };
            let c = diff(&a, &b, alphabet);
            assert_eq!(check(&a, &b, &c), lcs(&a, &b), "{a:?} / {b:?}");
        }
    }

    /// Big inputs: a correct diff, and minimal when the edits are few.
    #[test]
    fn big_inputs() {
        let mut rng = Rng(42);
        let a: Vec<u32> = (0..3000).map(|_| rng.below(50) as u32).collect();
        let mut b = a.clone();
        for _ in 0..40 {
            let at = rng.below(b.len() as u64) as usize;
            b[at] = rng.below(50) as u32;
        }
        let c = diff(&a, &b, 50);
        assert_eq!(check(&a, &b, &c), lcs(&a, &b));
        // (nothing alike: everything changed, at once)
        let x: Vec<u32> = (0..20000).collect();
        let y: Vec<u32> = (20000..40000).collect();
        let c = diff(&x, &y, 40000);
        assert!(c.a.iter().all(|&ch| ch) && c.b.iter().all(|&ch| ch));
        // (random against random: a correct diff)
        let p: Vec<u32> = (0..4000).map(|_| rng.below(4) as u32).collect();
        let q: Vec<u32> = (0..4000).map(|_| rng.below(4) as u32).collect();
        let c = diff(&p, &q, 4);
        check(&p, &q, &c);
    }
}
