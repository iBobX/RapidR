//! Work split across threads (docs/data-science-plan.md §2.4): rows (or
//! columns) shared out to `std::thread::scope` workers — as many as the
//! machine has cores, at most 16 — and everything on the calling thread when
//! the work is small (under 50,000 rows a thread costs more than it
//! saves) and always on the web (`wasm32`: no threads there yet).

use std::ops::Range;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Mutex;

/// Fewest rows worth more than one thread.
static MIN_ROWS: AtomicUsize = AtomicUsize::new(50_000);
/// Fewest rows a part.
static PART_ROWS: AtomicUsize = AtomicUsize::new(4096);
/// Fewest bytes of CSV worth reading in more than one piece (about 50,000
/// rows).
static MIN_BYTES: AtomicUsize = AtomicUsize::new(2 << 20);

/// Most workers.
static MAX_THREADS: AtomicUsize = AtomicUsize::new(16);

/// The sizes worth splitting (a test makes them small, so that small
/// tables take the parallel paths too): rows worth more than one thread,
/// rows a part, bytes of CSV worth more than one piece; and the most
/// workers (16).
pub fn set_thresholds(min_rows: usize, part_rows: usize, min_bytes: usize, max_threads: usize) {
    MIN_ROWS.store(min_rows.max(1), Ordering::Relaxed);
    PART_ROWS.store(part_rows.max(1), Ordering::Relaxed);
    MIN_BYTES.store(min_bytes.max(2), Ordering::Relaxed);
    MAX_THREADS.store(max_threads.max(1), Ordering::Relaxed);
}

/// Fewest bytes of CSV worth reading in more than one piece.
pub(crate) fn min_bytes() -> usize {
    MIN_BYTES.load(Ordering::Relaxed)
}

/// Workers this machine runs at once (1 on the web).
pub(crate) fn threads() -> usize {
    #[cfg(target_arch = "wasm32")]
    {
        1
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        static N: std::sync::OnceLock<usize> = std::sync::OnceLock::new();
        (*N.get_or_init(|| std::thread::available_parallelism().map_or(1, |n| n.get()))).clamp(1, MAX_THREADS.load(Ordering::Relaxed))
    }
}

/// How many parts `n` rows split into: 1 under 50,000 rows, else one a
/// worker (each part at least a few thousand rows).
pub(crate) fn parts(n: usize) -> usize {
    if n < MIN_ROWS.load(Ordering::Relaxed) {
        1
    } else {
        threads().min(n / PART_ROWS.load(Ordering::Relaxed)).max(1)
    }
}

/// `0..n` in `k` ranges as even as can be (the first ones a row longer),
/// each starting at a multiple of `align` (so bitmaps' words don't straddle
/// two parts).
pub(crate) fn ranges(n: usize, k: usize, align: usize) -> Vec<Range<usize>> {
    let k = k.max(1);
    let align = align.max(1);
    let step = n.div_ceil(k).div_ceil(align) * align;
    let mut out = Vec::with_capacity(k);
    let mut start = 0;
    while start < n {
        let end = (start + step.max(1)).min(n);
        out.push(start..end);
        start = end;
    }
    if out.is_empty() {
        out.push(0..0);
    }
    out
}

/// `f` of each of `n` tasks, on the workers (tasks handed out in order as
/// workers come free); the answers in the tasks' order. One task, or one
/// worker: on this thread.
pub(crate) fn tasks<R: Send>(n: usize, f: impl Fn(usize) -> R + Sync) -> Vec<R> {
    let workers = threads().min(n);
    if workers <= 1 {
        return (0..n).map(f).collect();
    }
    let next = AtomicUsize::new(0);
    let done: Mutex<Vec<(usize, R)>> = Mutex::new(Vec::with_capacity(n));
    std::thread::scope(|s| {
        for _ in 0..workers {
            s.spawn(|| {
                let mut mine = Vec::new();
                loop {
                    let i = next.fetch_add(1, Ordering::Relaxed);
                    if i >= n {
                        break;
                    }
                    mine.push((i, f(i)));
                }
                done.lock().unwrap_or_else(|e| e.into_inner()).extend(mine);
            });
        }
    });
    let mut done = done.into_inner().unwrap_or_else(|e| e.into_inner());
    done.sort_unstable_by_key(|(i, _)| *i);
    done.into_iter().map(|(_, r)| r).collect()
}

/// `f` of each range, in parallel when there are several; the answers in
/// order.
pub(crate) fn map<R: Send>(ranges: &[Range<usize>], f: impl Fn(Range<usize>) -> R + Sync) -> Vec<R> {
    tasks(ranges.len(), |i| f(ranges[i].clone()))
}

/// `f` of each item with its own part of `out`: `out` cut into the
/// lengths `lens` (in order), the parts filled in parallel.
pub(crate) fn fill<T: Send, I: Sync>(out: &mut [T], items: &[I], lens: impl Fn(&I) -> usize, f: impl Fn(&I, &mut [T]) + Sync) {
    if items.len() <= 1 || threads() <= 1 {
        let mut rest = out;
        for it in items {
            let (part, tail) = rest.split_at_mut(lens(it));
            f(it, part);
            rest = tail;
        }
        return;
    }
    let mut parts: Vec<Mutex<(&I, &mut [T])>> = Vec::with_capacity(items.len());
    let mut rest = out;
    for it in items {
        let (part, tail) = rest.split_at_mut(lens(it));
        parts.push(Mutex::new((it, part)));
        rest = tail;
    }
    tasks(parts.len(), |i| {
        let mut g = parts[i].lock().unwrap_or_else(|e| e.into_inner());
        let (it, part) = &mut *g;
        f(it, part);
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ranges_cover_everything_once() {
        for n in [0, 1, 63, 64, 65, 1000, 100_003] {
            for k in [1, 2, 3, 16] {
                for align in [1, 64] {
                    let r = ranges(n, k, align);
                    assert_eq!(r.first().unwrap().start, 0);
                    assert_eq!(r.last().unwrap().end, n);
                    for w in r.windows(2) {
                        assert_eq!(w[0].end, w[1].start);
                        assert_eq!(w[1].start % align, 0);
                    }
                    assert!(r.len() <= k.max(1));
                }
            }
        }
    }

    #[test]
    fn tasks_keep_their_order() {
        let v = tasks(1000, |i| i * 2);
        assert_eq!(v, (0..1000).map(|i| i * 2).collect::<Vec<_>>());
        let mut out = vec![0usize; 10];
        fill(&mut out, &[3usize, 3, 4], |n| *n, |n, part| part.iter_mut().for_each(|x| *x = *n));
        assert_eq!(out, [3, 3, 3, 3, 3, 3, 4, 4, 4, 4]);
    }
}
