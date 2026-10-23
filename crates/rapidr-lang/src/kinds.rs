//! What a value *is* beyond its type: the registry's `kind` facet
//! (docs/code-helpers-plan.md §2). A property, a method's or builtin's
//! parameter, a builtin's result and a constant group may say it:
//!
//! - `color` — RapidQ's `&HBBGGRR` (an `RGB(…)`, a colour constant);
//! - `font` — a font (a Font property; a font's name);
//! - `file:image`, `file:sound`, `file:any` — a file's name;
//! - `shortcut`, `cursor`, `range`, `handler` (an event's SUB);
//! - `component:<TYPE>[|<TYPE>…]` — another component (`component:any`);
//! - `choice:<set>` — one of a set of constants: `values` (the property's
//!   own), `bool` (True / False), or a constant group's `set`
//!   (constants.toml: `msgdlg-buttons`, a flag set).
//!
//! The code editor's helpers show a value by its kind and open its editor;
//! the AI's export carries it, so a model writes a valid value.

use crate::{ConstantGroup, Property, CONSTANT_GROUPS};

/// A file's kind.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FileKind {
    Image,
    Sound,
    Any,
}

impl FileKind {
    /// The file name extensions of its kind (lower case; any: none).
    pub fn extensions(self) -> &'static [&'static str] {
        match self {
            FileKind::Image => &["png", "bmp", "jpg", "jpeg", "gif", "ico", "svg"],
            FileKind::Sound => &["wav", "mid", "midi", "mp3", "ogg"],
            FileKind::Any => &[],
        }
    }

    /// Whether `name` is a file of this kind (by its extension).
    pub fn matches(self, name: &str) -> bool {
        let ext = name.rsplit_once('.').map(|(_, e)| e.to_ascii_lowercase()).unwrap_or_default();
        self == FileKind::Any || self.extensions().contains(&ext.as_str())
    }
}

/// A kind, read.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ValueKind {
    Color,
    Font,
    File(FileKind),
    Shortcut,
    Cursor,
    Range,
    Handler,
    /// Another component, of one of these types (`QFORM|QPANEL`; "any").
    Component(&'static str),
    /// One of a set: `values`, `bool`, or a constant group's set.
    Choice(&'static str),
}

impl ValueKind {
    /// A kind as the registry writes it.
    pub fn parse(k: &'static str) -> Option<ValueKind> {
        Some(match k {
            "color" => ValueKind::Color,
            "font" => ValueKind::Font,
            "file:image" => ValueKind::File(FileKind::Image),
            "file:sound" => ValueKind::File(FileKind::Sound),
            "file:any" => ValueKind::File(FileKind::Any),
            "shortcut" => ValueKind::Shortcut,
            "cursor" => ValueKind::Cursor,
            "range" => ValueKind::Range,
            "handler" => ValueKind::Handler,
            _ => {
                if let Some(t) = k.strip_prefix("component:") {
                    ValueKind::Component(t)
                } else {
                    ValueKind::Choice(k.strip_prefix("choice:")?)
                }
            }
        })
    }

    /// A property's kind.
    pub fn of(p: &Property) -> Option<ValueKind> {
        p.kind.and_then(ValueKind::parse)
    }

    /// The component types a link takes (empty: any).
    pub fn component_types(self) -> Vec<&'static str> {
        match self {
            ValueKind::Component("any") => Vec::new(),
            ValueKind::Component(t) => t.split('|').collect(),
            _ => Vec::new(),
        }
    }
}

/// The constant group whose choice set is `id`.
pub fn choice_set(id: &str) -> Option<&'static ConstantGroup> {
    CONSTANT_GROUPS.iter().find(|g| g.set == Some(id))
}

/// A choice's constants and whether they are flags: a property's own
/// `values` (`choice:values`: flags for a `set` property), True / False
/// (`choice:bool`), or a constant set — every group's that has it (a
/// set whose constants RAPIDQ.INC's order puts in several groups:
/// `modal-result`).
pub fn choices(set: &str, property: Option<&Property>) -> (Vec<&'static str>, bool) {
    match set {
        "values" => property.map_or((Vec::new(), false), |p| (p.values.to_vec(), p.ty == crate::Type::Set)),
        "bool" => (vec!["True", "False"], false),
        id => choice_set(id).map_or((Vec::new(), false), |first| {
            let names = CONSTANT_GROUPS.iter().filter(|g| g.set == Some(id)).flat_map(|g| g.constants.iter().map(|(n, _)| *n)).collect();
            (names, first.flags)
        }),
    }
}

/// The kind every constant named `name` is (any case): the colours'
/// `color` (clRed, clBtnFace).
pub fn constant_kind(name: &str) -> Option<&'static str> {
    crate::constant(name).and_then(|(_, g)| g.kind)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn kinds_read() {
        assert_eq!(ValueKind::parse("color"), Some(ValueKind::Color));
        assert_eq!(ValueKind::parse("file:sound"), Some(ValueKind::File(FileKind::Sound)));
        assert_eq!(ValueKind::parse("component:QFORM|QPANEL").unwrap().component_types(), ["QFORM", "QPANEL"]);
        assert_eq!(ValueKind::parse("choice:bool"), Some(ValueKind::Choice("bool")));
        assert_eq!(ValueKind::parse("nonsense"), None);
        // (a set in several groups: all of them — MESSAGEDLG's DefaultButton)
        let (answers, flags) = choices("modal-result", None);
        assert!(!flags && ["mrNo", "mrYes", "mrCancel", "mrOk"].iter().all(|n| answers.contains(n)), "{answers:?}");
        assert!(FileKind::Image.matches("Logo.PNG") && !FileKind::Image.matches("a.wav") && FileKind::Any.matches("x"));
    }
}
