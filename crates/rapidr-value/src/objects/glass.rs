//! QGLASSFRAME: built into RapidQ's compiler (RC.EXE) and runtime —
//! UtilMind's freeware "Glassy Form" (Delphi's TGlassy), which "will draw
//! itself as transparent part of form (shows what be under form), with
//! possibility to set glass color and degree of the transparency". RapidQ's
//! manual: "makes your form look transparent and shaded. It is filled with
//! bugs. Use QFormEx in RapidQ2.inc instead."
//!
//! Its members are RC.EXE's (`rapidr_ast::fixed_members`): no Caption, Tag,
//! Font or OnPaint. Values as RC.EXE stores them: Transparency is a byte
//! (`-5` → 251, `256` → 0, `33.7` → 33; 60 to begin with), TransparentColor
//! the glass's colour (0, black), Moveable a Delphi Boolean kept as given
//! (1 to begin with; 5 reads 5), Handle read-only; 105 × 105.
//!
//! What it shows: the glass's colour laid over what's under it at
//! `100 − Transparency` percent (60: a 40 % shade). TGlassy showed the
//! screen under the form (a picture it took of the desktop); RapidR's
//! windows aren't see-through, so what's under it is the form's own
//! background (its parent's colour) — the shade without the desktop
//! (docs/desktop-host-plan.md). Moveable: dragging it with the left button
//! moves its form (TGlassy's way to move a form without a caption).

use crate::{v_int, Value};

/// Transparency until set.
pub const TRANSPARENCY: i64 = 60;

/// A stored value as RC.EXE keeps it, for the properties it keeps its own
/// way (`None`: as given).
pub fn stored(prop: &str, v: &Value) -> Option<Value> {
    match prop {
        "transparency" => Some(v_int(to_byte(v))),
        "transparentcolor" | "moveable" => Some(v_int(crate::objects::record::to_int32(v))),
        _ => None,
    }
}

/// A value stored into a Delphi Byte: cut toward zero, wrapped.
fn to_byte(v: &Value) -> i64 {
    crate::objects::record::to_int32(v).rem_euclid(256)
}

/// What the glass shows (RGB `0xRRGGBB`) over `under` (RGB): its colour
/// `tint` (RapidQ's `&HBBGGRR`) at `100 − transparency` percent
/// (transparency past 100 is all see-through).
pub fn shade(under: u32, tint_bgr: i64, transparency: i64) -> u32 {
    let t = transparency.clamp(0, 100) as u32;
    let b = tint_bgr as u32 & 0xFF_FFFF;
    let tint = ((b & 0xFF) << 16) | (b & 0xFF00) | ((b >> 16) & 0xFF);
    let mix = |shift: u32| {
        let a = (under >> shift) & 0xFF;
        let b = (tint >> shift) & 0xFF;
        ((a * t + b * (100 - t) + 50) / 100) << shift
    };
    mix(16) | mix(8) | mix(0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{v_int, Value};

    #[test]
    fn rc_exe_s_stores() {
        assert_eq!(stored("transparency", &v_int(-5)), Some(v_int(251)));
        assert_eq!(stored("transparency", &v_int(256)), Some(v_int(0)));
        assert_eq!(stored("transparency", &Value::Double(33.7)), Some(v_int(33)));
        assert_eq!(stored("moveable", &v_int(5)), Some(v_int(5)));
        assert_eq!(stored("color", &v_int(5)), None);
    }

    #[test]
    fn the_shade() {
        // (black glass, 60 % see-through, over white: 60 % grey)
        assert_eq!(shade(0xFFFFFF, 0, 60), 0x999999);
        assert_eq!(shade(0xFFFFFF, 0x0000FF, 0), 0xFF0000);
        assert_eq!(shade(0x123456, 0, 100), 0x123456);
        assert_eq!(shade(0x123456, 0, 251), 0x123456);
    }
}
