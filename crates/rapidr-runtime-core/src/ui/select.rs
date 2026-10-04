//! Which desktop host shows the program's windows: FLTK (`gui`) or the UI
//! kernel (`kernel`). Decided once, by the first call into the host,
//! before any window system starts: FLTK and winit both claim the
//! application (NSApplication on macOS), so one process never runs both.
//! With one host compiled in, that one; with both, `RAPIDR_HOST=kernel`
//! or `fltk` (the default).

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Backend {
    #[cfg(feature = "gui")]
    Fltk,
    #[cfg(feature = "kernel")]
    Kernel,
}

/// The host this process uses.
#[cfg(all(feature = "gui", feature = "kernel"))]
pub fn backend() -> Backend {
    static CHOSEN: std::sync::OnceLock<Backend> = std::sync::OnceLock::new();
    *CHOSEN.get_or_init(|| choose(std::env::var("RAPIDR_HOST").ok().as_deref()))
}

#[cfg(all(feature = "gui", not(feature = "kernel")))]
pub const fn backend() -> Backend {
    Backend::Fltk
}

#[cfg(all(feature = "kernel", not(feature = "gui")))]
pub const fn backend() -> Backend {
    Backend::Kernel
}

/// `RAPIDR_HOST`'s choice (either case); anything else is FLTK.
#[cfg(all(feature = "gui", feature = "kernel"))]
pub fn choose(host: Option<&str>) -> Backend {
    match host {
        Some(h) if h.trim().eq_ignore_ascii_case("kernel") => Backend::Kernel,
        _ => Backend::Fltk,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_one_host_compiled_in_or_rapidr_host() {
        #[cfg(all(feature = "gui", feature = "kernel"))]
        {
            assert_eq!(choose(None), Backend::Fltk);
            assert_eq!(choose(Some("fltk")), Backend::Fltk);
            assert_eq!(choose(Some("Kernel")), Backend::Kernel);
            assert_eq!(choose(Some("winit")), Backend::Fltk);
        }
        #[cfg(all(feature = "gui", not(feature = "kernel")))]
        assert_eq!(backend(), Backend::Fltk);
        #[cfg(all(feature = "kernel", not(feature = "gui")))]
        assert_eq!(backend(), Backend::Kernel);
        assert_eq!(backend(), backend());
    }
}
