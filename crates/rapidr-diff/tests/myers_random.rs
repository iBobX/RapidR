//! Myers' diff on many random pairs: it never panics, and what it keeps is
//! the same on both sides. (A box much longer on one side made the reverse
//! search write past its diagonals: a Studio crash reviewing RAI's change,
//! 2026-11-02 — the first case below.)

#[test]
fn one_side_much_longer() {
    let a = vec![0, 0, 1, 1, 1, 0, 1, 1, 2, 2, 2, 1, 2, 2, 2, 0, 1, 0, 1, 2, 2, 1, 1, 2, 0, 2, 1, 0, 0, 0, 1, 0, 1, 2, 0, 1, 1, 0, 2, 2, 2, 2, 1, 2, 0, 0, 2, 2, 2, 2, 2, 2, 0, 2u32];
    let b = vec![0, 1, 0, 1, 0, 0, 0, 2u32];
    let c = rapidr_diff::myers::diff(&a, &b);
    assert_eq!(c.old.len(), a.len());
    assert_eq!(c.new.len(), b.len());
}

#[test]
fn random_pairs() {
    let mut seed: u64 = 0x9E37_79B9_7F4A_7C15;
    let mut rnd = move |n: u64| {
        seed ^= seed << 13;
        seed ^= seed >> 7;
        seed ^= seed << 17;
        seed % n
    };
    for it in 0..30_000u64 {
        let (la, lb, alpha) = (rnd(80) as usize, rnd(80) as usize, 1 + rnd(8));
        let a: Vec<u32> = (0..la).map(|_| rnd(alpha) as u32).collect();
        let b: Vec<u32> = (0..lb).map(|_| rnd(alpha) as u32).collect();
        let c = rapidr_diff::myers::diff(&a, &b);
        let kept_a: Vec<u32> = a.iter().zip(&c.old).filter(|(_, &d)| !d).map(|(v, _)| *v).collect();
        let kept_b: Vec<u32> = b.iter().zip(&c.new).filter(|(_, &i)| !i).map(|(v, _)| *v).collect();
        assert_eq!(kept_a, kept_b, "pair {it}: {a:?} / {b:?}");
    }
}
