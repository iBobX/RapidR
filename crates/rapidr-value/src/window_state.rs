//! QFORM.WindowState (RapidQ manual, Appendix A: RW, wsNormal 0 at first,
//! wsMinimized 1, wsMaximized 2) — the same on every runtime:
//!
//! - Setting it shows the form so (a form not shown yet takes it when it
//!   shows); reading it answers what the window is — the user's own
//!   maximize, restore and minimize included.
//! - Maximized, Left / Top / Width / Height (and ClientWidth / Height) read
//!   the maximized bounds — the work area on the desktop, the page's
//!   viewport on the web — and OnResize fires as for a user's resize;
//!   restored, the bounds it had come back (Windows' rcNormalPosition).
//! - Minimized, the bounds stay what they were (Delphi reads a minimized
//!   form's normal bounds) and nothing is resized: no OnResize. The form
//!   stays Visible.
//!
//! Where the system does the maximizing (a real window) the runtimes ask it;
//! where it can't be asked (the kernel's headless host, the web) they move
//! the form themselves with [`change`].

pub const WS_NORMAL: i64 = 0;
pub const WS_MINIMIZED: i64 = 1;
pub const WS_MAXIMIZED: i64 = 2;

/// A value as a window state (another: wsNormal).
pub fn of(v: i64) -> i64 {
    if (WS_NORMAL..=WS_MAXIMIZED).contains(&v) {
        v
    } else {
        WS_NORMAL
    }
}

/// Left, Top, Width, Height.
pub type Bounds = (i64, i64, i64, i64);

/// A form going from state `from` to `to`, now at `current`, with the
/// bounds to restore it to saved (`saved`): its new bounds (`None`: they
/// don't change) and what's saved from now on. Maximizing saves the
/// bounds it had and takes `max` (the work area); restoring a maximized
/// form brings the saved ones back; minimizing changes nothing.
pub fn change(from: i64, to: i64, current: Bounds, saved: Option<Bounds>, max: Bounds) -> (Option<Bounds>, Option<Bounds>) {
    let (from, to) = (of(from), of(to));
    match (from, to) {
        (f, t) if f == t => (None, saved),
        (_, WS_MAXIMIZED) => {
            // (from minimized: the bounds it had before are the ones saved,
            // unless it was never maximized)
            let keep = if from == WS_NORMAL || saved.is_none() { current } else { saved.unwrap_or(current) };
            ((current != max).then_some(max), Some(keep))
        }
        (_, WS_NORMAL) => match saved {
            Some(normal) => ((normal != current).then_some(normal), None),
            None => (None, None),
        },
        // (minimized: as it was, maximized or not)
        _ => (None, saved),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maximize_restore_minimize() {
        let normal = (100, 80, 300, 200);
        let max = (0, 0, 1920, 1080);
        // maximize: the work area, the bounds saved; restore: they come back
        let (b, saved) = change(WS_NORMAL, WS_MAXIMIZED, normal, None, max);
        assert_eq!((b, saved), (Some(max), Some(normal)));
        assert_eq!(change(WS_MAXIMIZED, WS_NORMAL, max, saved, max), (Some(normal), None));
        // minimize: nothing moves (a maximized form stays so underneath)
        assert_eq!(change(WS_NORMAL, WS_MINIMIZED, normal, None, max), (None, None));
        assert_eq!(change(WS_MAXIMIZED, WS_MINIMIZED, max, Some(normal), max), (None, Some(normal)));
        assert_eq!(change(WS_MINIMIZED, WS_NORMAL, max, Some(normal), max), (Some(normal), None));
        // minimized, then maximized: the normal bounds saved stay
        assert_eq!(change(WS_MINIMIZED, WS_MAXIMIZED, normal, None, max), (Some(max), Some(normal)));
        // the same state: nothing; other values are wsNormal
        assert_eq!(change(WS_MAXIMIZED, WS_MAXIMIZED, max, Some(normal), max), (None, Some(normal)));
        assert_eq!(of(7), WS_NORMAL);
    }
}
