//! RapidR Studio's panels as public components (docs/ide-plan.md I1, lane
//! L-PANELS; their API: docs/ide-components.md §3.4–3.8): the models,
//! shared by every runtime — native, interpreted and the web — and drawn by
//! the UI kernel (`rapidr-ui-kernel`'s `components/panels/`), so a panel
//! looks and behaves the same everywhere.
//!
//! | Type | Model | What it is |
//! |---|---|---|
//! | RPROPERTYINSPECTOR | [`inspector`] | Delphi's object inspector: typed editors from the language registry, categories / A–Z, search, Anchors' pin editor, the Events page, multi-selection |
//! | RTOOLBOX | [`toolbox`] | the registry's components under "RapidQ" and "RapidR", with our icons; search, drag, double click |
//! | RPROJECTTREE | [`project_tree`] | an `.rrproj` project's files by kind, forms with their components |
//! | ROUTPUTCONSOLE | [`console`] | a program's output with ANSI (CLS, COLOR, LOCATE), the build log, problems |
//! | RTOOLBAR | [`toolbar`] | icon buttons, separators, toggles, an overflow menu, customizable |
//! | RCOMMANDPALETTE | [`palette`] | commands found by fuzzy search |
//!
//! Each model lives in a per-thread table by the component's (lowercase)
//! name ([`panel_models!`](crate::panel_models)); the kernel reads it to paint and turns what the user
//! does into a [`User`] action (`Container::Panel`), which the runtime glue
//! ([`runtime`]) carries out with the program's events — the same glue for
//! both runtimes, through the [`runtime::Runtime`] trait.
//!
//! Shared by all: [`fuzzy`] (the search boxes' matching) and [`rows`] (a
//! list's focus and keyboard moves). [`subject`] is the inspector's
//! interface to what it inspects (live components, a designer's selection).

pub mod console;
pub mod fuzzy;
pub mod inspector;
pub mod palette;
pub mod project_tree;
pub mod rows;
pub mod runtime;
pub mod subject;
pub mod toolbar;
pub mod toolbox;

/// The panels' type names (RapidR's).
pub const TYPES: [&str; 6] = ["RPROPERTYINSPECTOR", "RTOOLBOX", "RPROJECTTREE", "ROUTPUTCONSOLE", "RTOOLBAR", "RCOMMANDPALETTE"];

/// Whether `type_name` (any case) is one of the panels.
pub fn is_panel(type_name: &str) -> bool {
    TYPES.iter().any(|t| t.eq_ignore_ascii_case(type_name))
}

/// What the user did to a panel, from the kernel (`Container::Panel`) to
/// the runtime glue ([`runtime::rt_user`]).
#[derive(Clone, Debug, PartialEq)]
pub enum User {
    Inspector(inspector::User),
    Toolbox(toolbox::User),
    ProjectTree(project_tree::User),
    Console(console::User),
    ToolBar(toolbar::User),
    Palette(palette::User),
    /// An item picked from a list the runtime dropped for the panel
    /// ([`runtime::Runtime::drop_list`]): an inspector's enum, a toolbar's
    /// overflow menu.
    Picked(String),
}

/// A per-thread table of a panel's models by component name (lowercase):
/// `with`, `with_mut` (made on first use), `exists`, `remove`.
#[macro_export]
macro_rules! panel_models {
    ($model:ty) => {
        thread_local! {
            static MODELS: std::cell::RefCell<std::collections::HashMap<String, $model>> = std::cell::RefCell::new(std::collections::HashMap::new());
        }

        /// Reads panel `id`'s model (`None`: none yet).
        pub fn with<R>(id: &str, f: impl FnOnce(&$model) -> R) -> Option<R> {
            MODELS.with(|m| m.borrow().get(&id.to_ascii_lowercase()).map(f))
        }

        /// Changes panel `id`'s model (made on first use).
        pub fn with_mut<R>(id: &str, f: impl FnOnce(&mut $model) -> R) -> R {
            MODELS.with(|m| f(m.borrow_mut().entry(id.to_ascii_lowercase()).or_default()))
        }

        pub fn exists(id: &str) -> bool {
            MODELS.with(|m| m.borrow().contains_key(&id.to_ascii_lowercase()))
        }

        /// Forgets panel `id`'s model (its component went).
        pub fn remove(id: &str) {
            MODELS.with(|m| m.borrow_mut().remove(&id.to_ascii_lowercase()));
        }
    };
}

#[cfg(test)]
mod tests;
