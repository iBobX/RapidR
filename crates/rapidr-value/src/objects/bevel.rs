//! A QPANEL's bevels as RapidQ (Delphi's TPanel) draws them, the same in
//! both runtimes: the outer bevel (BevelOuter), BorderWidth pixels of
//! space, then the inner bevel (BevelInner), each BevelWidth pixels of
//! one-pixel 3D frames. `bvNone` 0, `bvLowered` 1, `bvRaised` 2, `bvSpace`
//! 3 (space, no lines). A panel's default: raised outer, no inner.

pub const BV_NONE: i64 = 0;
pub const BV_LOWERED: i64 = 1;
pub const BV_RAISED: i64 = 2;
pub const BV_SPACE: i64 = 3;

/// The light edge (Windows' button highlight) and the dark one (shadow).
pub const LIGHT: u32 = 0xFFFFFF;
pub const DARK: u32 = 0x808080;

/// One pixel-wide frame `inset` pixels inside the panel: its top and left
/// edges `top_left`, its bottom and right edges `bottom_right` (RGB).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Frame {
    pub inset: i64,
    pub top_left: u32,
    pub bottom_right: u32,
}

/// The frames of a panel with these settings, outermost first.
pub fn frames(outer: i64, inner: i64, bevel_width: i64, border_width: i64) -> Vec<Frame> {
    let width = bevel_width.clamp(0, 100);
    let mut out = Vec::new();
    let mut inset = 0;
    let mut bevel = |kind: i64, inset: &mut i64| {
        if kind == BV_NONE {
            return;
        }
        for _ in 0..width {
            match kind {
                BV_LOWERED => out.push(Frame { inset: *inset, top_left: DARK, bottom_right: LIGHT }),
                BV_RAISED => out.push(Frame { inset: *inset, top_left: LIGHT, bottom_right: DARK }),
                _ => {}
            }
            *inset += 1;
        }
    };
    bevel(outer, &mut inset);
    inset += border_width.clamp(0, 1000);
    bevel(inner, &mut inset);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_panel_is_raised() {
        assert_eq!(frames(BV_RAISED, BV_NONE, 1, 0), vec![Frame { inset: 0, top_left: LIGHT, bottom_right: DARK }]);
    }

    #[test]
    fn frame_and_space() {
        // QBEVEL's bsFrame, lowered: inner raised inside outer lowered.
        let f = frames(BV_LOWERED, BV_RAISED, 1, 0);
        assert_eq!(f, vec![Frame { inset: 0, top_left: DARK, bottom_right: LIGHT }, Frame { inset: 1, top_left: LIGHT, bottom_right: DARK }]);
        // bvSpace takes room, draws nothing; BorderWidth pushes the inner bevel in.
        let f = frames(BV_SPACE, BV_LOWERED, 2, 3);
        assert_eq!(f.iter().map(|x| x.inset).collect::<Vec<_>>(), vec![5, 6]);
        assert!(frames(BV_NONE, BV_NONE, 1, 0).is_empty());
    }

    #[test]
    fn inset_for_aligned_children() {
        let with = |props: &'static [(&str, i64)]| move |p: &str| props.iter().find(|(k, _)| *k == p).map_or(crate::Value::Null, |(_, v)| crate::v_int(*v));
        assert_eq!(client_inset(&with(&[])), 1);
        assert_eq!(client_inset(&with(&[("bevelinner", 2), ("bevelwidth", 2)])), 4);
        assert_eq!(client_inset(&with(&[("bevelouter", 0)])), 0);
        assert_eq!(client_inset(&with(&[("bevelouter", 1), ("bevelinner", 1)])), 2);
    }
}

/// How far inside a QPANEL its aligned children start, on every side
/// (Delphi's TCustomPanel.AdjustClientRect): BevelWidth for each bevel it
/// has, BorderWidth between them, and Windows' two-pixel client edge with
/// BorderStyle bsSingle. RC.EXE: a default panel's alLeft button at Left 1
/// and its alTop one at Top 1; with an inner raised bevel too and
/// BevelWidth 2, at 4; with no bevel, at 0. `get` gives a panel property,
/// Null while unset.
pub fn client_inset(get: &dyn Fn(&str) -> crate::Value) -> i64 {
    let prop = |p: &str| match get(p) {
        crate::Value::Null => default(p).unwrap_or(0),
        v => v.to_i64(),
    };
    let width = prop("bevelwidth").clamp(0, 100);
    let bevels = i64::from(prop("bevelouter") != BV_NONE) + i64::from(prop("bevelinner") != BV_NONE);
    let edge = if matches!(get("borderstyle"), crate::Value::Null) { 0 } else if get("borderstyle").to_i64() == 1 { 2 } else { 0 };
    bevels * width + prop("borderwidth").clamp(0, 1000) + edge
}

/// A panel property's value until the program sets it.
pub fn default(prop: &str) -> Option<i64> {
    Some(match prop {
        "bevelouter" => BV_RAISED,
        "bevelinner" => BV_NONE,
        "bevelwidth" => 1,
        "borderwidth" => 0,
        _ => return None,
    })
}

/// QBEVEL (`QBevel.inc`, Delphi's TBevel on a QPANEL) — what its Shape
/// and Style set, the include's rules: `bsBox` 1 is an outer bevel (Style
/// `bsLowered` 0 lowered, `bsRaised` 1 raised), `bsFrame` 6 two (lowered:
/// an etched line, raised: a bump), every other shape no bevel — the
/// lines of `bsTopLine` 2 … `bsRightLine` 5 are [`qbevel_lines`]; `bsSpacer`
/// 0 shows nothing. Setting Shape or Style sets BevelInner / BevelOuter
/// (inner, outer) to these.
pub fn qbevel_bevels(shape: i64, style: i64) -> (i64, i64) {
    match shape {
        1 => (BV_NONE, style + 1),
        6 => (2 - style, 1 + style),
        _ => (BV_NONE, BV_NONE),
    }
}

/// A QBEVEL's two lines for `bsTopLine` 2, `bsBottomLine` 3, `bsLeftLine`
/// 4 and `bsRightLine` 5 in a `w` × `h` bevel: (x, y, width, height,
/// light) one pixel thick, the first light when Style is raised (any
/// non-zero), dark otherwise, the second the other — at its top, bottom,
/// left or right edge, as the include's two-pixel canvas aligned there.
pub fn qbevel_lines(shape: i64, style: i64, w: i64, h: i64) -> Vec<(i64, i64, i64, i64, bool)> {
    let raised = style != 0;
    let (first, second) = match shape {
        2 => ((0, 0, w, 1), (0, 1, w, 1)),
        3 => ((0, h - 2, w, 1), (0, h - 1, w, 1)),
        4 => ((0, 0, 1, h), (1, 0, 1, h)),
        5 => ((w - 2, 0, 1, h), (w - 1, 0, 1, h)),
        _ => return Vec::new(),
    };
    vec![(first.0, first.1, first.2, first.3, raised), (second.0, second.1, second.2, second.3, !raised)]
}

/// The bevel properties a QBEVEL's Shape or Style store sets, with the
/// other read from `other` (Style for a Shape store, Shape for a Style
/// one); `None` for any other property.
pub fn qbevel_set(prop: &str, value: i64, other: i64) -> Option<[(&'static str, i64); 2]> {
    let (shape, style) = match prop {
        "shape" => (value, other),
        "style" => (other, value),
        _ => return None,
    };
    let (inner, outer) = qbevel_bevels(shape, style);
    Some([("bevelinner", inner), ("bevelouter", outer)])
}

#[cfg(test)]
mod qbevel_tests {
    use super::*;

    #[test]
    fn shapes_and_styles_as_the_include_sets_them() {
        assert_eq!(qbevel_bevels(1, 0), (BV_NONE, BV_LOWERED));
        assert_eq!(qbevel_bevels(1, 1), (BV_NONE, BV_RAISED));
        assert_eq!(qbevel_bevels(6, 0), (BV_RAISED, BV_LOWERED));
        assert_eq!(qbevel_bevels(6, 1), (BV_LOWERED, BV_RAISED));
        assert_eq!(qbevel_bevels(2, 1), (BV_NONE, BV_NONE));
        assert_eq!(qbevel_lines(3, 0, 50, 30), vec![(0, 28, 50, 1, false), (0, 29, 50, 1, true)]);
        assert_eq!(qbevel_lines(5, 1, 50, 30), vec![(48, 0, 1, 30, true), (49, 0, 1, 30, false)]);
        assert!(qbevel_lines(0, 0, 50, 30).is_empty());
        assert_eq!(qbevel_set("style", 1, 6), Some([("bevelinner", BV_LOWERED), ("bevelouter", BV_RAISED)]));
    }
}
