//! `cbrt` and `cbrtf` for the Windows 7 builds (`*-win7-windows-gnu`).
//!
//! `msvcrt.dll` — the C runtime Windows 7 has — has no cube root, so the
//! linker takes `cbrt` and `cbrtf` from mingw-w64's library, whose versions
//! are derived from Cephes (Stephen Moshier's math library), a library whose
//! permission to copy is a vague one. RapidR ships nothing it cannot say a
//! plain licence for, so the program defines its own, here: the linker then
//! never reaches for the library's (an object's own definition comes before
//! an archive member). The method is the textbook one: a first guess from
//! the exponent, then Newton's iteration `y = (2y + a / y²) / 3`.
//!
//! The pure functions are compiled and tested on every host; the C names are
//! exported on the Windows 7 builds only.

/// The cube root of `x`, to within one unit in the last place (and exact for
/// the cubes of small whole numbers).
pub fn cube_root(x: f64) -> f64 {
    if x == 0.0 || !x.is_finite() {
        return x; // ±0, ±infinity, NaN: themselves
    }
    let a = x.abs();
    if a < f64::MIN_POSITIVE {
        // (subnormal: scaled by 2^54 into the normal range; the root by 2^18)
        return cube_root(x * 18014398509481984.0) / 262144.0;
    }
    // exponent/3 in the exponent bits: within a few percent of the root
    let mut y = f64::from_bits(a.to_bits() / 3 + 0x2A9F_7893_782D_A1CE);
    for _ in 0..5 {
        y = (2.0 * y + a / (y * y)) / 3.0;
    }
    // (a whole number's cube comes out as that whole number)
    let r = y.round();
    if r * r * r == a {
        y = r;
    }
    y.copysign(x)
}

/// The cube root of `x`, in single precision.
pub fn cube_root_f32(x: f32) -> f32 {
    cube_root(x as f64) as f32
}

/// The C names the standard library asks the linker for.
#[cfg(all(windows, target_vendor = "win7", target_env = "gnu"))]
mod exports {
    #[no_mangle]
    pub extern "C" fn cbrt(x: f64) -> f64 {
        super::cube_root(x)
    }

    #[no_mangle]
    pub extern "C" fn cbrtf(x: f32) -> f32 {
        super::cube_root_f32(x)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// How many representable numbers apart two finite values are.
    fn ulps(a: f64, b: f64) -> u64 {
        let key = |v: f64| {
            let bits = v.to_bits() as i64;
            if bits < 0 { i64::MIN - bits } else { bits }
        };
        key(a).abs_diff(key(b))
    }

    #[test]
    fn special_values_are_themselves() {
        assert_eq!(cube_root(0.0), 0.0);
        assert!(cube_root(-0.0).is_sign_negative());
        assert_eq!(cube_root(f64::INFINITY), f64::INFINITY);
        assert_eq!(cube_root(f64::NEG_INFINITY), f64::NEG_INFINITY);
        assert!(cube_root(f64::NAN).is_nan());
    }

    #[test]
    fn whole_cubes_come_out_whole() {
        for n in 1..=1000 {
            let n = n as f64;
            assert_eq!(cube_root(n * n * n), n, "{n}");
            assert_eq!(cube_root(-(n * n * n)), -n, "-{n}");
        }
        assert_eq!(cube_root_f32(27.0), 3.0);
    }

    /// Against the platform's own cube root, from the smallest number to the largest.
    #[test]
    fn within_one_ulp_of_the_platforms_across_the_whole_range() {
        let mut x = 5e-324_f64; // the smallest subnormal
        let mut worst = 0;
        while x < f64::MAX / 4.0 {
            for v in [x, x * 1.37, x * 2.9, x * 7.77] {
                worst = worst.max(ulps(cube_root(v), v.cbrt()));
                worst = worst.max(ulps(cube_root(-v), (-v).cbrt()));
            }
            x *= 3.1;
        }
        // (a few values on the edges)
        for v in [f64::MIN_POSITIVE, f64::MIN_POSITIVE / 2.0, f64::MAX, 1.0, 2.0, 0.5, 1e-300, 1e300, 123456.789] {
            worst = worst.max(ulps(cube_root(v), v.cbrt()));
        }
        assert!(worst <= 1, "{worst} ulps off");
    }

    #[test]
    fn single_precision() {
        for v in [1.0_f32, 2.0, 0.001, 123.456, 1e-30, 1e30, f32::MAX, f32::MIN_POSITIVE] {
            let (got, want) = (cube_root_f32(v), v.cbrt());
            assert!((got - want).abs() <= want.abs() * 2.0 * f32::EPSILON, "{v}: {got} vs {want}");
        }
    }
}
