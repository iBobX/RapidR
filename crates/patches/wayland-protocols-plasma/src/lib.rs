//! RapidR's stand-in for `wayland-protocols-plasma` (Cargo.toml): the
//! names winit's Wayland backend uses for KDE's blur
//! (`blur::client::org_kde_kwin_blur{,_manager}`), generated from RapidR's
//! own protocol file under interface names no compositor announces. winit
//! binds the manager at start-up, finds none and never blurs, as on any
//! compositor without KDE's.

#![forbid(improper_ctypes, unsafe_op_in_unsafe_fn)]

pub mod blur {
    #[cfg(feature = "client")]
    pub mod client {
        pub mod org_kde_kwin_blur_manager {
            pub use super::generated::rapidr_no_blur_manager::{Request, RapidrNoBlurManager as OrgKdeKwinBlurManager};
        }
        pub mod org_kde_kwin_blur {
            pub use super::generated::rapidr_no_blur::{Request, RapidrNoBlur as OrgKdeKwinBlur};
        }

        mod generated {
            #![allow(dead_code, non_camel_case_types, unused_unsafe, unused_variables)]
            #![allow(non_upper_case_globals, non_snake_case, unused_imports)]
            #![allow(missing_docs, clippy::all)]
            use wayland_client;
            use wayland_client::protocol::*;

            pub mod __interfaces {
                use wayland_client::protocol::__interfaces::*;
                wayland_scanner::generate_interfaces!("./protocol/no-blur.xml");
            }
            use self::__interfaces::*;

            wayland_scanner::generate_client_code!("./protocol/no-blur.xml");
        }
    }
}
