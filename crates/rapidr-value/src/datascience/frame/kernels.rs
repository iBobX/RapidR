//! The frame's kernels (docs/data-science-plan.md §2.4): sort, filter,
//! group, join, describe — on typed slices where the columns are typed, in
//! parallel for many rows, and giving exactly the answers the members have
//! always given (the same order for ties, the same sums added in the same
//! order).

use std::cmp::Ordering;
use std::collections::HashMap;
use std::hash::{BuildHasher, Hash, Hasher};
use std::sync::OnceLock;

use super::column::{Builder, Column, Data, NONE};
use super::{mean, num_text, par, percentile, std_dev, Frame, Op};

// ---------------------------------------------------------------------------
// Hashing
// ---------------------------------------------------------------------------

/// A fast hash (folded multiply) seeded once a run.
#[derive(Clone, Copy)]
pub(crate) struct Fold(u64);

impl Default for Fold {
    fn default() -> Fold {
        static SEED: OnceLock<u64> = OnceLock::new();
        Fold(*SEED.get_or_init(|| std::collections::hash_map::RandomState::new().hash_one(0x5eed_u64) | 1))
    }
}

pub(crate) struct FoldHasher(u64);

#[inline]
fn mix(x: u64) -> u64 {
    let r = (x ^ 0x2d35_8dcc_aa6c_78a5) as u128 * 0x9e37_79b9_7f4a_7c15u128;
    (r as u64) ^ ((r >> 64) as u64)
}

impl Hasher for FoldHasher {
    #[inline]
    fn write(&mut self, bytes: &[u8]) {
        let mut h = self.0;
        let (words, rest) = bytes.as_chunks::<8>();
        for w in words {
            h = mix(h ^ u64::from_le_bytes(*w));
        }
        if !rest.is_empty() {
            let mut w = [0u8; 8];
            w[..rest.len()].copy_from_slice(rest);
            h = mix(h ^ u64::from_le_bytes(w) ^ ((rest.len() as u64) << 59));
        }
        self.0 = h;
    }
    #[inline]
    fn write_u8(&mut self, x: u8) {
        self.0 = mix(self.0 ^ x as u64);
    }
    #[inline]
    fn write_u32(&mut self, x: u32) {
        self.0 = mix(self.0 ^ x as u64);
    }
    #[inline]
    fn write_u64(&mut self, x: u64) {
        self.0 = mix(self.0 ^ x);
    }
    #[inline]
    fn write_i64(&mut self, x: i64) {
        self.0 = mix(self.0 ^ x as u64);
    }
    #[inline]
    fn write_usize(&mut self, x: usize) {
        self.0 = mix(self.0 ^ x as u64);
    }
    #[inline]
    fn finish(&self) -> u64 {
        self.0
    }
}

impl BuildHasher for Fold {
    type Hasher = FoldHasher;
    fn build_hasher(&self) -> FoldHasher {
        FoldHasher(self.0)
    }
}

pub(crate) type Map<K, V> = HashMap<K, V, Fold>;

// ---------------------------------------------------------------------------
// Groups
// ---------------------------------------------------------------------------

/// No group (a null left out).
pub(crate) const NO_GROUP: u32 = u32::MAX;

/// Rows grouped by a key: each row's group (numbered in the order the
/// groups first appear), and each group's first row.
pub(crate) struct Groups {
    pub ids: Vec<u32>,
    pub first: Vec<usize>,
}

/// Rows grouped by `key(row)` (None: null — a group of its own when
/// `nulls`, else [`NO_GROUP`]): in parallel parts, each numbering its own
/// keys, then the parts' numbers made one in order (so groups are numbered
/// as they first appear in the whole column).
pub(crate) fn group_by_key<K: Hash + Eq + Copy + Send + Sync>(n: usize, nulls: bool, key: impl Fn(usize) -> Option<K> + Sync) -> Groups {
    let parts = par::ranges(n, par::parts(n), 1);
    // (each part: its rows' local numbers, its keys as they first appear)
    type Local<K> = (Vec<u32>, Vec<(Option<K>, usize)>);
    let locals: Vec<Local<K>> = par::map(&parts, |r| {
        let mut map: Map<Option<K>, u32> = Map::default();
        let mut ids = Vec::with_capacity(r.len());
        let mut keys = Vec::new();
        for i in r {
            let k = key(i);
            if k.is_none() && !nulls {
                ids.push(NO_GROUP);
                continue;
            }
            let id = *map.entry(k).or_insert_with(|| {
                keys.push((k, i));
                (keys.len() - 1) as u32
            });
            ids.push(id);
        }
        (ids, keys)
    });
    if locals.len() == 1 {
        let (ids, keys) = locals.into_iter().next().unwrap_or_default();
        return Groups { ids, first: keys.into_iter().map(|(_, r)| r).collect() };
    }
    let mut global: Map<Option<K>, u32> = Map::default();
    let mut first = Vec::new();
    let remaps: Vec<Vec<u32>> = locals
        .iter()
        .map(|(_, keys)| {
            keys.iter()
                .map(|&(k, row)| {
                    *global.entry(k).or_insert_with(|| {
                        first.push(row);
                        (first.len() - 1) as u32
                    })
                })
                .collect()
        })
        .collect();
    let mut ids = vec![0u32; n];
    let which: Vec<usize> = (0..parts.len()).collect();
    par::fill(&mut ids, &which, |&p| parts[p].len(), |&p, out| {
        for (o, &l) in out.iter_mut().zip(&locals[p].0) {
            *o = if l == NO_GROUP { NO_GROUP } else { remaps[p][l as usize] };
        }
    });
    Groups { ids, first }
}

/// Rows grouped by their cell's text (equal text: one group).
pub(crate) fn groups(col: &Column, nulls: bool) -> Groups {
    let n = col.len();
    let ok = |i: usize| col.is_valid(i);
    match &col.data {
        Data::Int(v) => group_by_key(n, nulls, |i| ok(i).then(|| v[i])),
        Data::Float(v) => group_by_key(n, nulls, |i| ok(i).then(|| v[i].to_bits())),
        Data::Bool(b) => group_by_key(n, nulls, |i| ok(i).then(|| b.get(i))),
        Data::Text(t) => group_by_key(n, nulls, |i| ok(i).then(|| t.bytes_of(i))),
    }
}

// ---------------------------------------------------------------------------
// Sorting
// ---------------------------------------------------------------------------

/// `v` sorted by `cmp` — a total order — in parallel runs merged pairwise.
pub(crate) fn par_sort_by<T: Copy + Send + Sync>(v: &mut [T], cmp: impl Fn(&T, &T) -> Ordering + Sync) {
    let n = v.len();
    let k = par::parts(n);
    if k <= 1 {
        v.sort_unstable_by(&cmp);
        return;
    }
    let mut runs = par::ranges(n, k, 1);
    par::fill(v, &runs, |r| r.len(), |_, part| part.sort_unstable_by(&cmp));
    let mut buf = v.to_vec();
    let mut in_v = true;
    while runs.len() > 1 {
        let merged: Vec<std::ops::Range<usize>> = runs.chunks(2).map(|p| p[0].start..p[p.len() - 1].end).collect();
        {
            let (src, dst): (&[T], &mut [T]) = if in_v { (&*v, &mut buf[..]) } else { (&buf[..], &mut *v) };
            let pairs: Vec<&[std::ops::Range<usize>]> = runs.chunks(2).collect();
            par::fill(dst, &pairs, |p| p[p.len() - 1].end - p[0].start, |p, out| {
                if p.len() == 1 {
                    out.copy_from_slice(&src[p[0].clone()]);
                } else {
                    merge(&src[p[0].clone()], &src[p[1].clone()], out, &cmp);
                }
            });
        }
        in_v = !in_v;
        runs = merged;
    }
    if !in_v {
        v.copy_from_slice(&buf);
    }
}

fn merge<T: Copy>(a: &[T], b: &[T], out: &mut [T], cmp: &impl Fn(&T, &T) -> Ordering) {
    let (mut i, mut j, mut k) = (0, 0, 0);
    while i < a.len() && j < b.len() {
        if cmp(&b[j], &a[i]) == Ordering::Less {
            out[k] = b[j];
            j += 1;
        } else {
            out[k] = a[i];
            i += 1;
        }
        k += 1;
    }
    out[k..k + a.len() - i].copy_from_slice(&a[i..]);
    k += a.len() - i;
    out[k..].copy_from_slice(&b[j..]);
}

/// A number as a key that sorts as the number does (-0 and 0 alike).
#[inline]
fn num_key(x: f64) -> u64 {
    let b = (x + 0.0).to_bits();
    if b >> 63 == 1 {
        !b
    } else {
        b | (1 << 63)
    }
}

/// The first 8 bytes of `s`, as a number that sorts as they do.
#[inline]
fn prefix(s: &[u8]) -> u64 {
    let mut w = [0u8; 8];
    let n = s.len().min(8);
    w[..n].copy_from_slice(&s[..n]);
    u64::from_be_bytes(w)
}

/// Cells `i` and `j` of `col` (neither null) compared as text.
#[inline]
pub(crate) fn cmp_text(col: &Column, i: usize, j: usize) -> Ordering {
    match &col.data {
        Data::Text(t) => t.bytes_of(i).cmp(t.bytes_of(j)),
        Data::Bool(b) => b.get(i).cmp(&b.get(j)),
        _ => col.get(i).cmp(&col.get(j)),
    }
}

/// Rows `i` and `j` of `col` compared (numbers as numbers in a numeric
/// column, else as text; nulls last).
fn cmp_rows(col: &Column, nums: Option<&[f64]>, i: usize, j: usize) -> Ordering {
    match (col.is_valid(i), col.is_valid(j)) {
        (false, false) => Ordering::Equal,
        (false, true) => Ordering::Greater,
        (true, false) => Ordering::Less,
        (true, true) => {
            if let Some(n) = nums {
                let (p, q) = (n[i], n[j]);
                if !p.is_nan() && !q.is_nan() {
                    return p.partial_cmp(&q).unwrap_or(Ordering::Equal);
                }
            }
            cmp_text(col, i, j)
        }
    }
}

/// `rows` (every row when None) in the order `keys` (column, descending)
/// sorts them: stably, nulls last, numbers as numbers in numeric columns,
/// text as text.
pub(crate) fn sort_rows(f: &Frame, keys: &[(usize, bool)], rows: Option<&[usize]>) -> Vec<usize> {
    let all: Vec<usize>;
    let rows = match rows {
        Some(r) => r,
        None => {
            all = (0..f.height).collect();
            &all
        }
    };
    if let [(c, desc)] = *keys {
        let col = &f.cols[c];
        let parts = par::ranges(rows.len(), par::parts(rows.len()), 1);
        if col.dtype().numeric() {
            // (key and row in one number: sorted as numbers, ties by row)
            let nums = col.nums();
            let split = par::map(&parts, |r| {
                let (mut keyed, mut nulls) = (Vec::with_capacity(r.len()), Vec::new());
                for &i in &rows[r] {
                    let x = nums[i];
                    if x.is_nan() {
                        nulls.push(i);
                    } else {
                        let k = num_key(x);
                        keyed.push(((if desc { !k } else { k }) as u128) << 64 | i as u128);
                    }
                }
                (keyed, nulls)
            });
            let mut keyed: Vec<u128> = Vec::with_capacity(rows.len());
            let mut nulls = Vec::new();
            for (k, n) in split {
                keyed.extend(k);
                nulls.extend(n);
            }
            par_sort_by(&mut keyed, |a, b| a.cmp(b));
            let mut out: Vec<usize> = keyed.iter().map(|&k| k as u64 as usize).collect();
            out.extend(nulls);
            return out;
        }
        if let Data::Text(t) = &col.data {
            let split = par::map(&parts, |r| {
                let (mut keyed, mut nulls) = (Vec::with_capacity(r.len()), Vec::new());
                for &i in &rows[r] {
                    if col.is_valid(i) {
                        keyed.push((prefix(t.bytes_of(i)), i));
                    } else {
                        nulls.push(i);
                    }
                }
                (keyed, nulls)
            });
            let mut keyed: Vec<(u64, usize)> = Vec::with_capacity(rows.len());
            let mut nulls = Vec::new();
            for (k, n) in split {
                keyed.extend(k);
                nulls.extend(n);
            }
            let text = |a: &(u64, usize), b: &(u64, usize)| a.0.cmp(&b.0).then_with(|| t.bytes_of(a.1).cmp(t.bytes_of(b.1)));
            if desc {
                par_sort_by(&mut keyed, |a, b| text(b, a).then(a.1.cmp(&b.1)));
            } else {
                par_sort_by(&mut keyed, |a, b| text(a, b).then(a.1.cmp(&b.1)));
            }
            let mut out: Vec<usize> = keyed.iter().map(|k| k.1).collect();
            out.extend(nulls);
            return out;
        }
    }
    // Any keys: each compared in turn (descending ones reversed, nulls
    // always last), ties by row.
    let numeric: Vec<Option<&[f64]>> = keys.iter().map(|&(c, _)| f.cols[c].dtype().numeric().then(|| f.cols[c].nums())).collect();
    let mut out = rows.to_vec();
    par_sort_by(&mut out, |&i, &j| {
        for (k, &(c, desc)) in keys.iter().enumerate() {
            let col = &f.cols[c];
            let o = if desc && col.is_valid(i) && col.is_valid(j) { cmp_rows(col, numeric[k], j, i) } else { cmp_rows(col, numeric[k], i, j) };
            if o != Ordering::Equal {
                return o;
            }
        }
        i.cmp(&j)
    });
    out
}

// ---------------------------------------------------------------------------
// Filtering
// ---------------------------------------------------------------------------

/// Whether `cell` (its number `n`: NaN if none) passes `cell OP value`:
/// numbers compare as numbers, text as text; a null cell never passes.
#[inline]
pub(crate) fn passes(cell: Option<&str>, n: f64, op: Op, value: &str, value_num: Option<f64>) -> bool {
    let Some(cell) = cell else { return false };
    let ord = || match value_num {
        Some(b) if !n.is_nan() => n.partial_cmp(&b).unwrap_or(Ordering::Equal),
        _ => cell.cmp(value),
    };
    match op {
        Op::Eq => ord() == Ordering::Equal,
        Op::Ne => ord() != Ordering::Equal,
        Op::Gt => ord() == Ordering::Greater,
        Op::Lt => ord() == Ordering::Less,
        Op::Ge => ord() != Ordering::Less,
        Op::Le => ord() != Ordering::Greater,
        Op::Contains => cell.contains(value),
        Op::Starts => cell.starts_with(value),
        Op::Ends => cell.ends_with(value),
    }
}

/// Whether `x OP b` holds for two numbers.
#[inline]
fn num_passes(x: f64, b: f64, op: Op) -> bool {
    match op {
        Op::Eq => x == b,
        Op::Ne => x != b,
        Op::Gt => x > b,
        Op::Lt => x < b,
        Op::Ge => x >= b,
        Op::Le => x <= b,
        _ => false,
    }
}

/// The rows of `col` that pass `cell OP value`, in order.
pub(crate) fn filter_rows(col: &Column, op: Op, value: &str, value_num: Option<f64>) -> Vec<usize> {
    let n = col.len();
    let parts = par::ranges(n, par::parts(n), 1);
    let numeric_op = !matches!(op, Op::Contains | Op::Starts | Op::Ends);
    let picked: Vec<Vec<usize>> = match (&col.data, value_num) {
        (Data::Int(v), Some(b)) if numeric_op => par::map(&parts, |r| r.filter(|&i| col.is_valid(i) && num_passes(v[i] as f64, b, op)).collect()),
        (Data::Float(v), Some(b)) if numeric_op => par::map(&parts, |r| r.filter(|&i| col.is_valid(i) && num_passes(v[i], b, op)).collect()),
        (Data::Text(t), _) => {
            let nums = if value_num.is_some() && numeric_op { col.nums() } else { &[][..] };
            par::map(&parts, |r| r.filter(|&i| col.is_valid(i) && passes(Some(t.get(i)), nums.get(i).copied().unwrap_or(f64::NAN), op, value, value_num)).collect())
        }
        _ => {
            let nums = col.nums();
            par::map(&parts, |r| {
                let mut s = String::new();
                r.filter(|&i| {
                    s.clear();
                    col.write(i, &mut s) && passes(Some(&s), nums[i], op, value, value_num)
                })
                .collect()
            })
        }
    };
    if picked.len() == 1 {
        return picked.into_iter().next().unwrap_or_default();
    }
    let mut out = Vec::with_capacity(picked.iter().map(Vec::len).sum());
    for p in picked {
        out.extend(p);
    }
    out
}

// ---------------------------------------------------------------------------
// Group-by
// ---------------------------------------------------------------------------

/// `groupby`: a row a group of column `g` (in the keys' order, nulls last),
/// the group column first, then every other column's `agg` of the group.
pub(crate) fn groupby(f: &Frame, g: usize, agg: &str) -> Frame {
    let gcol = &f.cols[g];
    let gr = groups(gcol, true);
    // (the groups in their keys' order; equal keys in the order they came)
    let firsts = sort_rows(f, &[(g, false)], Some(&gr.first));
    let order: Vec<u32> = firsts.iter().map(|&r| gr.ids[r]).collect();
    let ng = order.len();
    let mut rank = vec![0u32; ng];
    for (pos, &id) in order.iter().enumerate() {
        rank[id as usize] = pos as u32;
    }
    // (each row's group's place in the answer)
    let mut at = vec![0u32; gr.ids.len()];
    let parts = par::ranges(at.len(), par::parts(at.len()), 1);
    par::fill(&mut at, &parts, |r| r.len(), |r, out| {
        for (o, &id) in out.iter_mut().zip(&gr.ids[r.clone()]) {
            *o = rank[id as usize];
        }
    });
    let others: Vec<usize> = (0..f.width()).filter(|&c| c != g).collect();
    let mut cols = vec![gcol.gather(&firsts)];
    cols.extend(par::tasks(others.len(), |k| aggregate(&f.cols[others[k]], &at, ng, &firsts, agg)));
    Frame { columns: std::iter::once(g).chain(others.iter().copied()).map(|c| f.columns[c].clone()).collect(), cols, height: ng }
}

/// Column `col`'s `agg` of each group (`at[row]`: the row's group's place;
/// `firsts[place]`: its first row).
fn aggregate(col: &Column, at: &[u32], ng: usize, firsts: &[usize], agg: &str) -> Column {
    let numeric = col.dtype().numeric();
    let n = at.len();
    let texts = |vals: Vec<Option<f64>>| Column::from_cells(vals.into_iter().map(|v| v.map(num_text)));
    match agg {
        "count" => {
            let mut counts = vec![0i64; ng];
            for (i, &g) in at.iter().enumerate() {
                if col.is_valid(i) {
                    counts[g as usize] += 1;
                }
            }
            Column::new(Data::Int(counts.into()), None)
        }
        "first" => col.gather(firsts),
        "last" => {
            let mut last = vec![0usize; ng];
            for (i, &g) in at.iter().enumerate() {
                last[g as usize] = i;
            }
            col.gather(&last)
        }
        "sum" | "mean" | "std" if numeric => {
            let nums = col.nums();
            let (mut sums, mut counts) = (vec![0.0f64; ng], vec![0usize; ng]);
            for i in 0..n {
                let x = nums[i];
                if !x.is_nan() {
                    sums[at[i] as usize] += x;
                    counts[at[i] as usize] += 1;
                }
            }
            match agg {
                "sum" => texts(sums.into_iter().map(Some).collect()),
                "mean" => texts(sums.iter().zip(&counts).map(|(s, &c)| (c > 0).then(|| s / c as f64)).collect()),
                _ => {
                    let means: Vec<f64> = sums.iter().zip(&counts).map(|(s, &c)| s / c as f64).collect();
                    let mut sq = vec![0.0f64; ng];
                    for i in 0..n {
                        let x = nums[i];
                        if !x.is_nan() {
                            let g = at[i] as usize;
                            sq[g] += (x - means[g]).powi(2);
                        }
                    }
                    texts(sq.iter().zip(&counts).map(|(s, &c)| (c > 0).then(|| (s / c as f64).sqrt())).collect())
                }
            }
        }
        "min" | "max" if numeric => {
            let nums = col.nums();
            let pick = if agg == "min" { f64::min } else { f64::max };
            let mut acc: Vec<Option<f64>> = vec![None; ng];
            for i in 0..n {
                let x = nums[i];
                if !x.is_nan() {
                    let a = &mut acc[at[i] as usize];
                    *a = Some(a.map_or(x, |a| pick(a, x)));
                }
            }
            texts(acc)
        }
        "min" | "max" => {
            let want = if agg == "min" { Ordering::Less } else { Ordering::Greater };
            let mut best = vec![NONE; ng];
            for i in 0..n {
                if col.is_valid(i) {
                    let b = &mut best[at[i] as usize];
                    if *b == NONE || cmp_text(col, i, *b) == want {
                        *b = i;
                    }
                }
            }
            col.gather(&best)
        }
        "median" if numeric => {
            let nums = col.nums();
            let mut counts = vec![0usize; ng + 1];
            for i in 0..n {
                if !nums[i].is_nan() {
                    counts[at[i] as usize + 1] += 1;
                }
            }
            for g in 0..ng {
                counts[g + 1] += counts[g];
            }
            let mut next = counts.clone();
            let mut vals = vec![0.0f64; counts[ng]];
            for i in 0..n {
                let x = nums[i];
                if !x.is_nan() {
                    let g = at[i] as usize;
                    vals[next[g]] = x;
                    next[g] += 1;
                }
            }
            texts((0..ng).map(|g| (counts[g + 1] > counts[g]).then(|| percentile(&vals[counts[g]..counts[g + 1]], 0.5))).collect())
        }
        _ => Column::nulls(ng),
    }
}

// ---------------------------------------------------------------------------
// Joins
// ---------------------------------------------------------------------------

/// A join key: a number (equal numbers match: "1" and "1.0"), else text.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
enum JoinKey<'a> {
    Num(u64),
    Text(&'a [u8]),
}

/// Row `i`'s key in `col` (None: null).
#[inline]
fn join_key<'a>(col: &'a Column, nums: &[f64], i: usize) -> Option<JoinKey<'a>> {
    if !col.is_valid(i) {
        return None;
    }
    let n = nums[i];
    if n.is_nan() {
        return Some(JoinKey::Text(match &col.data {
            Data::Text(t) => t.bytes_of(i),
            Data::Bool(b) if b.get(i) => b"true",
            Data::Bool(_) => b"false",
            // (typed numbers are never NaN)
            _ => b"",
        }));
    }
    // (0.0 and -0.0 are one number)
    Some(JoinKey::Num(if n == 0.0 { 0 } else { n.to_bits() }))
}

/// A column's rows by key: each key's rows, in order.
struct Index<'x> {
    map: Map<JoinKey<'x>, u32>,
    /// Each row's key (NO_GROUP: null).
    ids: Vec<u32>,
    offs: Vec<usize>,
    rows: Vec<usize>,
}

impl<'x> Index<'x> {
    fn new(col: &'x Column) -> Index<'x> {
        let nums = col.nums();
        let gr = group_by_key(col.len(), false, |i| join_key(col, nums, i));
        let nk = gr.first.len();
        let mut map = Map::with_capacity_and_hasher(nk, Fold::default());
        for (id, &r) in gr.first.iter().enumerate() {
            if let Some(k) = join_key(col, nums, r) {
                map.insert(k, id as u32);
            }
        }
        let mut offs = vec![0usize; nk + 1];
        for &id in &gr.ids {
            if id != NO_GROUP {
                offs[id as usize + 1] += 1;
            }
        }
        for k in 0..nk {
            offs[k + 1] += offs[k];
        }
        let mut next = offs.clone();
        let mut rows = vec![0usize; offs[nk]];
        for (i, &id) in gr.ids.iter().enumerate() {
            if id != NO_GROUP {
                rows[next[id as usize]] = i;
                next[id as usize] += 1;
            }
        }
        Index { map, ids: gr.ids, offs, rows }
    }

    /// The rows with key `k` (none for a null key or a key it hasn't).
    #[inline]
    fn rows_of(&self, k: Option<JoinKey<'x>>) -> Option<&[usize]> {
        let id = *self.map.get(&k?)? as usize;
        Some(&self.rows[self.offs[id]..self.offs[id + 1]])
    }
}

/// Each row of `col` (in order, in parallel parts) paired with the rows of
/// `index` that have its key: (row, index row) — and (row, NONE) for a row
/// none has when `unmatched`.
fn probe<'x>(col: &'x Column, index: &Index<'x>, unmatched: bool) -> (Vec<usize>, Vec<usize>) {
    let nums = col.nums();
    let n = col.len();
    let parts = par::ranges(n, par::parts(n), 1);
    let found = par::map(&parts, |r| {
        let (mut mine, mut theirs) = (Vec::with_capacity(r.len()), Vec::with_capacity(r.len()));
        for i in r {
            match index.rows_of(join_key(col, nums, i)) {
                Some(rows) => {
                    for &j in rows {
                        mine.push(i);
                        theirs.push(j);
                    }
                }
                None if unmatched => {
                    mine.push(i);
                    theirs.push(NONE);
                }
                None => {}
            }
        }
        (mine, theirs)
    });
    if found.len() == 1 {
        return found.into_iter().next().unwrap_or_default();
    }
    let total = found.iter().map(|f| f.0.len()).sum();
    let (mut mine, mut theirs) = (Vec::with_capacity(total), Vec::with_capacity(total));
    for (m, t) in found {
        mine.extend(m);
        theirs.extend(t);
    }
    (mine, theirs)
}

/// The cells of `a` at `ar`, and where `ar` has no row, of `b` at `br`.
fn coalesce(a: &Column, ar: &[usize], b: &Column, br: &[usize]) -> Column {
    let mut out = Builder::new(ar.len());
    let mut s = String::new();
    for (&i, &j) in ar.iter().zip(br) {
        s.clear();
        let ok = if i != NONE { a.write(i, &mut s) } else { j != NONE && b.write(j, &mut s) };
        out.push(ok.then_some(s.as_str()));
    }
    out.finish()
}

/// Whether `rows` is every row of an `n`-row frame, in order.
fn every_row(rows: &[usize], n: usize) -> bool {
    rows.len() == n && rows.iter().enumerate().all(|(k, &i)| k == i)
}

/// `merge`: `a` joined with `b` on column `on` (`how`: inner, left, right,
/// outer, cross). The key column once; `b`'s other columns after `a`'s, a
/// name both have taking `_right`. Keys match as text, or as numbers ("1"
/// and "1.0"); a null key matches nothing. Rows come in `a`'s order (`b`'s
/// for a right join), each with its matches in the other's order.
pub(crate) fn join(a: &Frame, b: &Frame, on: &str, how: &str) -> Result<Frame, String> {
    let cross = how == "cross";
    let (ka, kb) = if cross {
        (usize::MAX, usize::MAX)
    } else {
        match (a.col(on), b.col(on)) {
            (Some(x), Some(y)) => (x, y),
            _ => return Err(format!("no column \"{on}\" in both frames")),
        }
    };
    // Which rows pair up: (a's row, b's row), NONE for an unmatched row of
    // the other.
    let (ar, br) = if cross {
        let mut ar = Vec::with_capacity(a.height * b.height);
        let mut br = Vec::with_capacity(a.height * b.height);
        for i in 0..a.height {
            for j in 0..b.height {
                ar.push(i);
                br.push(j);
            }
        }
        (ar, br)
    } else if how == "right" {
        let index = Index::new(&a.cols[ka]);
        let (br, ar) = probe(&b.cols[kb], &index, true);
        (ar, br)
    } else {
        let index = Index::new(&b.cols[kb]);
        let (mut ar, mut br) = probe(&a.cols[ka], &index, matches!(how, "left" | "outer" | "full"));
        if matches!(how, "outer" | "full") {
            let mut used = vec![false; index.offs.len() - 1];
            let (col, nums) = (&a.cols[ka], a.cols[ka].nums());
            for i in 0..a.height {
                if let Some(k) = join_key(col, nums, i) {
                    if let Some(&id) = index.map.get(&k) {
                        used[id as usize] = true;
                    }
                }
            }
            for (j, &id) in index.ids.iter().enumerate() {
                if id == NO_GROUP || !used[id as usize] {
                    ar.push(NONE);
                    br.push(j);
                }
            }
        }
        (ar, br)
    };
    let (a_all, b_all) = (every_row(&ar, a.height), every_row(&br, b.height));
    let a_missing = !a_all && ar.contains(&NONE);
    let mut out = Frame { columns: a.columns.clone(), cols: Vec::with_capacity(a.width() + b.width()), height: ar.len() };
    for (c, col) in a.cols.iter().enumerate() {
        out.cols.push(if c == ka && a_missing {
            // (an unmatched row of b: its key in the key column)
            coalesce(col, &ar, &b.cols[kb], &br)
        } else if a_all {
            col.clone()
        } else {
            col.gather(&ar)
        });
    }
    for (c, col) in b.cols.iter().enumerate().filter(|(c, _)| *c != kb) {
        let mut n = b.columns[c].clone();
        if out.columns.contains(&n) {
            n += "_right";
        }
        out.columns.push(n);
        out.cols.push(if b_all { col.clone() } else { col.gather(&br) });
    }
    Ok(out)
}

// ---------------------------------------------------------------------------
// Describe, value counts
// ---------------------------------------------------------------------------

/// `describe`: the frame's numeric columns summarised (count, mean, std,
/// min, quartiles, max), a column each, in parallel.
pub(crate) fn describe(f: &Frame) -> Frame {
    let stats = ["count", "mean", "std", "min", "25%", "50%", "75%", "max"];
    let numeric: Vec<usize> = (0..f.width()).filter(|&c| f.dtype(c).numeric()).collect();
    let mut out = Frame::default();
    let s = out.add_column("statistic");
    out.set_column(s, stats.iter().map(|s| Some(s.to_string())).collect());
    let summaries = par::tasks(numeric.len(), |k| describe_column(&f.cols[numeric[k]]));
    for (k, &c) in numeric.iter().enumerate() {
        let col = out.add_column(&f.columns[c]);
        out.cols[col] = summaries[k].clone();
    }
    out
}

fn describe_column(col: &Column) -> Column {
    let mut v: Vec<f64> = col.nums().iter().copied().filter(|x| !x.is_nan()).collect();
    let (m, sd) = (mean(&v), std_dev(&v));
    let n = v.len();
    if n == 0 {
        return Column::from_cells(std::iter::once(Some("0".to_string())).chain(std::iter::repeat_n(None, 7)));
    }
    // (the order statistics the quartiles need, by selection: the values a
    // sort puts there)
    let quant = |p: f64| {
        let pos = p * (n - 1) as f64;
        (pos.floor() as usize, pos.ceil() as usize)
    };
    let mut ks: Vec<usize> = vec![0, n - 1];
    for p in [0.25, 0.5, 0.75] {
        let (lo, hi) = quant(p);
        ks.push(lo);
        ks.push(hi);
    }
    ks.sort_unstable();
    ks.dedup();
    let mut at = std::collections::BTreeMap::new();
    let mut from = 0;
    for &k in &ks {
        if k >= from {
            let (_, x, _) = v[from..].select_nth_unstable_by(k - from, f64::total_cmp);
            at.insert(k, *x);
            from = k + 1;
        }
    }
    let kth = |k: usize| at[&k];
    let q = |p: f64| {
        let (lo, hi) = quant(p);
        let s = [kth(lo), kth(hi)];
        // (as quantile_sorted reads a sorted list)
        let pos = p * (n - 1) as f64;
        s[0] + (s[1] - s[0]) * (pos - lo as f64)
    };
    let values = [n as f64, m, sd, kth(0), q(0.25), q(0.5), q(0.75), kth(n - 1)];
    Column::from_cells(values.iter().map(|x| Some(num_text(*x))))
}

/// `value_counts`: each value of column `c` and how many rows have it, most
/// first (equal counts in the order the values first appear); nulls left
/// out.
pub(crate) fn value_counts(f: &Frame, c: usize) -> Frame {
    let col = &f.cols[c];
    let gr = groups(col, false);
    let mut counts = vec![0i64; gr.first.len()];
    for &id in &gr.ids {
        if id != NO_GROUP {
            counts[id as usize] += 1;
        }
    }
    let mut order: Vec<usize> = (0..counts.len()).collect();
    order.sort_by(|a, b| counts[*b].cmp(&counts[*a]));
    let rows: Vec<usize> = order.iter().map(|&g| gr.first[g]).collect();
    let mut out = Frame::default();
    let a = out.add_column(&f.columns[c]);
    let b = out.add_column("count");
    out.height = rows.len();
    out.cols[a] = col.gather(&rows);
    out.cols[b] = Column::new(Data::Int(order.iter().map(|&g| counts[g]).collect::<Vec<_>>().into()), None);
    out
}

/// How many different values (not null) column `c` has.
pub(crate) fn nunique(col: &Column) -> usize {
    groups(col, false).first.len()
}

