//! CopyMode: how `Draw`, `StretchDraw` and `CopyRect` combine the picture
//! they draw (the source) with the pixels already there (the destination),
//! and with the canvas's brush (the pattern) for the modes that use one.
//!
//! The `cmXXX` constants are Windows' ternary raster-operation codes
//! (wingdi.h); the byte that matters is the third (`0xCC` of `cmSrcCopy`
//! 0xCC0020): the result's truth table, one entry for each of the eight
//! ways the three inputs can be set — bit 0 the destination, bit 1 the
//! source, bit 2 the pattern. Applying it to a whole colour is eight AND/OR
//! terms, the same on every system, so a canvas gives the same pixels
//! interpreted, native and in a browser (RC.EXE, tests/conformance/cases/
//! rapidq_copymode.bas: the 15 modes of a `CopyRect`, `Draw` and
//! `StretchDraw` of a solid picture onto a solid one, all three alike).

use crate::Value;

/// The raster operations RapidQ names, as `(cmXXX code, truth table)`.
const MODES: [(i64, u8); 15] = [
    (0x00_00_42, 0x00),     // cmBlackness: every pixel black
    (0x55_00_09, 0x55),     // cmDstInvert: the destination inverted
    (0xC0_00_CA, 0xC0),     // cmMergeCopy: source AND pattern
    (0xBB_02_26, 0xBB),     // cmMergePaint: NOT source OR destination
    (0x33_00_08, 0x33),     // cmNotSrcCopy: NOT source
    (0x11_00_A6, 0x11),     // cmNotSrcErase: NOT (source OR destination)
    (0xF0_00_21, 0xF0),     // cmPatCopy: the pattern
    (0x5A_00_49, 0x5A),     // cmPatInvert: pattern XOR destination
    (0xFB_0A_09, 0xFB),     // cmPatPaint: destination OR pattern OR NOT source
    (0x88_00_C6, 0x88),     // cmSrcAnd: source AND destination
    (0xCC_00_20, 0xCC),     // cmSrcCopy: the source
    (0x44_03_28, 0x44),     // cmSrcErase: source AND NOT destination
    (0x66_00_46, 0x66),     // cmSrcInvert: source XOR destination
    (0xEE_00_86, 0xEE),     // cmSrcPaint: source OR destination
    (0xFF_00_62, 0xFF),     // cmWhiteness: every pixel white
];

/// The truth table of `cmSrcCopy`.
pub const SRC_COPY: u8 = 0xCC;

/// The truth table of a CopyMode value; `cmSrcCopy` when it is empty,
/// zero or not one of the 15 modes (a canvas whose CopyMode was never set,
/// or set to something that is not a mode, copies).
pub fn table_of(copy_mode: &Value) -> u8 {
    let code = match copy_mode {
        Value::Null => return SRC_COPY,
        v => v.to_i64(),
    };
    MODES.iter().find(|(c, _)| *c == code).map_or(SRC_COPY, |(_, t)| *t)
}

/// The colour `table` makes of destination `d`, source `s` and pattern `p`
/// (&HBBGGRR colours): each bit of the result is the table's entry for the
/// three inputs' bits.
pub fn apply(table: u8, d: u32, s: u32, p: u32) -> u32 {
    let mut out = 0u32;
    for i in 0..8u8 {
        if table & (1 << i) == 0 {
            continue;
        }
        let pick = |bit: u8, v: u32| if i & bit != 0 { v } else { !v };
        out |= pick(4, p) & pick(2, s) & pick(1, d);
    }
    out & 0x00FF_FFFF
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::v_int;

    /// Real RapidQ (RC.EXE, Windows 11): a destination pixel &H0F3CC5, a
    /// source pixel &H55AA33 and the brush &H0F3CC5 gave these.
    #[test]
    fn the_fifteen_modes_give_what_the_classic_compiler_gave() {
        let (d, s, p) = (0x0F3CC5, 0x55AA33, 0x0F3CC5);
        let expect: [(i64, u32); 15] = [
            (0x42, 0x000000),
            (0x550009, 0xF0C33A),
            (0xC000CA, 0x052801),
            (0xBB0226, 0xAF7DCD),
            (0x330008, 0xAA55CC),
            (0x1100A6, 0xA04108),
            (0xF00021, 0x0F3CC5),
            (0x5A0049, 0x000000),
            (0xFB0A09, 0xAF7DCD),
            (0x8800C6, 0x052801),
            (0xCC0020, 0x55AA33),
            (0x440328, 0x508232),
            (0x660046, 0x5A96F6),
            (0xEE0086, 0x5FBEF7),
            (0xFF0062, 0xFFFFFF),
        ];
        for (code, want) in expect {
            assert_eq!(apply(table_of(&v_int(code)), d, s, p), want, "mode {code:#x}");
        }
    }

    #[test]
    fn no_mode_is_a_copy() {
        assert_eq!(table_of(&Value::Null), SRC_COPY);
        assert_eq!(table_of(&v_int(0)), SRC_COPY);
        assert_eq!(table_of(&v_int(12345)), SRC_COPY);
    }
}
