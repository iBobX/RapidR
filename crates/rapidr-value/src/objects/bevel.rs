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
