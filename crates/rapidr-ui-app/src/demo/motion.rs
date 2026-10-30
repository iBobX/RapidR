//! How the demo's pointer and fingers move: a glide eased like a person's
//! hand (minimum jerk: slow start, quick middle, gentle landing) along a
//! slight arc, its length from the distance; typing with a natural rhythm
//! (a little uneven, slower after a space and after punctuation) that is
//! the same on every recording, so takes can be compared.

/// The minimum-jerk curve (10k³ − 15k⁴ + 6k⁵): how far along a glide is
/// at `k` (0 to 1) of its time — what a person's reach looks like.
pub fn ease(k: f64) -> f64 {
    let k = k.clamp(0.0, 1.0);
    k * k * k * (10.0 + k * (-15.0 + 6.0 * k))
}

/// How long a glide over `distance` logical pixels takes at `speed` (1
/// normal): 0.35 s and 0.45 ms a pixel, at most 1.1 s.
pub fn glide_ms(distance: f64, speed: f64) -> u64 {
    if distance < 1.0 {
        return 0;
    }
    ((350.0 + distance * 0.45).min(1100.0) / speed.max(0.05)).round() as u64
}

/// The pointer's place at `k` (0 to 1) of a glide from `a` to `b`: eased,
/// bowed a little to the side (an eighth of the way, at most 36 pixels;
/// none for short hops), as a hand doesn't move on a ruler.
pub fn glide_at(a: (f64, f64), b: (f64, f64), k: f64) -> (f64, f64) {
    let e = ease(k);
    let (dx, dy) = (b.0 - a.0, b.1 - a.1);
    let d = (dx * dx + dy * dy).sqrt();
    let bow = if d < 60.0 { 0.0 } else { (d * 0.08).min(36.0) };
    // (the bow peaks mid-way and is gone at both ends; to the right of the
    // way when going down, as a right hand curves)
    let side = (std::f64::consts::PI * e).sin() * bow;
    let (nx, ny) = if d > 0.0 { (-dy / d, dx / d) } else { (0.0, 0.0) };
    let sign = if dy >= 0.0 { -1.0 } else { 1.0 };
    (a.0 + dx * e + nx * side * sign, a.1 + dy * e + ny * side * sign)
}

/// A small, steady pseudo-random number in 0..1 for (`seed`, `i`): the
/// same rhythm every recording.
fn jitter(seed: u64, i: usize) -> f64 {
    let mut x = seed ^ (i as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15);
    x ^= x >> 33;
    x = x.wrapping_mul(0xFF51_AFD7_ED55_8CCD);
    x ^= x >> 33;
    (x % 10_000) as f64 / 10_000.0
}

/// How long to wait after typing each character of `text` (ms), at
/// `speed` (1 natural: about 11 a second): 65 ms ± 20, more after a space
/// (a word ends), more after `.,;:!?` and a new line (a thought ends),
/// a little more for capitals and symbols (Shift).
pub fn typing_delays(text: &str, speed: f64, seed: u64) -> Vec<u64> {
    let speed = speed.max(0.05);
    text.chars()
        .enumerate()
        .map(|(i, c)| {
            let mut ms = 65.0 + (jitter(seed, i) - 0.5) * 40.0;
            if c == ' ' {
                ms += 45.0;
            } else if ".,;:!?".contains(c) {
                ms += 110.0;
            } else if c == '\n' {
                ms += 220.0;
            } else if c.is_uppercase() || "\"()[]{}<>=+-*/&$#@%".contains(c) {
                ms += 25.0;
            }
            (ms / speed).round().max(8.0) as u64
        })
        .collect()
}

/// A seed from a text (FNV-1a): each typed text keeps its own rhythm.
pub fn seed_of(text: &str) -> u64 {
    text.bytes().fold(0xcbf2_9ce4_8422_2325, |h, b| (h ^ u64::from(b)).wrapping_mul(0x100_0000_01b3))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_ease_starts_and_lands_gently() {
        assert_eq!(ease(0.0), 0.0);
        assert_eq!(ease(1.0), 1.0);
        assert!((ease(0.5) - 0.5).abs() < 1e-12);
        // (slow at both ends, fastest in the middle)
        let step = |k: f64| ease(k + 0.01) - ease(k);
        assert!(step(0.0) < step(0.45) && step(0.98) < step(0.45));
        // (never past the target, never back)
        let mut last = 0.0;
        for i in 0..=100 {
            let e = ease(i as f64 / 100.0);
            assert!(e >= last && e <= 1.0);
            last = e;
        }
        assert_eq!(ease(-1.0), 0.0);
        assert_eq!(ease(2.0), 1.0);
    }

    #[test]
    fn glides_take_longer_for_longer_ways_up_to_a_limit() {
        assert_eq!(glide_ms(0.0, 1.0), 0);
        let near = glide_ms(100.0, 1.0);
        let far = glide_ms(900.0, 1.0);
        assert!(near >= 350 && near < far && far <= 1100, "{near} {far}");
        assert_eq!(glide_ms(3000.0, 1.0), 1100);
        assert!((glide_ms(900.0, 2.0) as i64 - (far / 2) as i64).abs() <= 1);
    }

    #[test]
    fn a_glide_bows_and_arrives_exactly() {
        let (a, b) = ((100.0, 100.0), (700.0, 500.0));
        assert_eq!(glide_at(a, b, 0.0), a);
        let end = glide_at(a, b, 1.0);
        assert!((end.0 - b.0).abs() < 1e-9 && (end.1 - b.1).abs() < 1e-9);
        // (mid-way it is off the straight line, by at most 36 pixels)
        let mid = glide_at(a, b, 0.5);
        let on_line = ((mid.0 - a.0) * (b.1 - a.1) - (mid.1 - a.1) * (b.0 - a.0)).abs() / ((b.0 - a.0).hypot(b.1 - a.1));
        assert!(on_line > 5.0 && on_line <= 36.0 + 1e-9, "{on_line}");
        // (a short hop goes straight)
        let hop = glide_at((0.0, 0.0), (30.0, 0.0), 0.5);
        assert!(hop.1.abs() < 1e-9);
    }

    #[test]
    fn typing_has_a_natural_steady_rhythm() {
        let text = "Hello, world. Ok";
        let a = typing_delays(text, 1.0, seed_of(text));
        assert_eq!(a, typing_delays(text, 1.0, seed_of(text)), "the same every recording");
        assert_eq!(a.len(), text.chars().count());
        let avg = a.iter().sum::<u64>() as f64 / a.len() as f64;
        assert!((60.0..130.0).contains(&avg), "{avg}");
        // (after the comma and the full stop, longer than after a letter)
        assert!(a[5] > a[1] + 60 && a[12] > a[2] + 60);
        // (uneven, but never frantic)
        assert!(a.iter().all(|&ms| ms >= 40));
        assert!(a.iter().collect::<std::collections::HashSet<_>>().len() > 4);
        // (twice as fast: half as long)
        let fast = typing_delays(text, 2.0, seed_of(text));
        assert!(fast.iter().zip(&a).all(|(f, s)| (*f as i64 - (*s as i64) / 2).abs() <= 1));
    }
}
