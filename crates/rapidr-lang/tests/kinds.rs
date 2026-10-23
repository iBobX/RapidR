//! The `kind` facet (docs/code-helpers-plan.md §2, E0 / E6): every colour,
//! file and font value in the registry says what it is, so the code editor's
//! helpers show and edit it and an AI writes a valid one.
//!
//! What counts as a colour, file or font value: a property of the type
//! (`color`, `font`, `picture`, an inspector editor `file` / `picture`), and
//! any property or parameter *named* like one (`FontColor`, `MaskColor`, the
//! canvas methods' `c`, `fc`, `bc`; `FileName`, `File`, `Filename`;
//! `FontName`). The few that are named so but hold something else are listed
//! here with the reason.

use rapidr_lang::kinds::{choices, FileKind, ValueKind};
use rapidr_lang::{Component, Type, BUILTINS, COMPONENTS, CONSTANT_GROUPS, GLOBALS, ITEMS};

/// (component, member, name): named like a colour, file or font but not one.
const NOT_VALUES: &[(&str, &str, &str, &str)] = &[
    ("RCODEEDITOR", "", "ColorScheme", "a colour scheme's name"),
    ("RCOLORDIALOG", "", "Colors", "the custom colours' count"),
    ("RFILEDIALOG", "", "FileTitle", "a file's name without its folder"),
    ("RFILEDIALOG", "", "Files", "the files picked, a list"),
    ("RPROJECT", "", "FileManager", "the file manager's name"),
    ("RAIPROJECTTOOLS", "", "ProjectFiles", "a list of the project's files"),
    ("RPRINTER", "", "Fonts", "the printer's fonts, a list"),
    ("RPLOT", "*", "color", "a chart's colours are names or #RRGGBB text"),
    ("RPLOT", "*", "colors", "a chart's colours are names or #RRGGBB text"),
    ("RSTRINGLIST", "*", "File", "a stream (LoadFromStream / SaveToStream)"),
    ("RCOMPORT", "Read", "QFile", "a stream"),
    ("*", "*", "Image", "a picture object (CopyRect)"),
];

fn exempt(component: &str, member: &str, name: &str) -> bool {
    NOT_VALUES.iter().any(|(c, m, n, _)| {
        (*c == "*" || c.eq_ignore_ascii_case(component)) && (*m == "*" || m.is_empty() || m.eq_ignore_ascii_case(member)) && n.eq_ignore_ascii_case(name)
    })
}

/// What a name says its value is: `color`, `file` or `font`.
fn named(name: &str) -> Option<&'static str> {
    let n = name.to_ascii_lowercase();
    let n = n.trim_end_matches(['$', '%', '&', '#', '!']);
    if n.ends_with("color") || n.ends_with("colour") || n.starts_with("color") && !n.ends_with("count") {
        return Some("color");
    }
    if matches!(n, "file" | "filename" | "file1" | "file2" | "outfile" | "helpfile" | "mainfile" | "sourcefile" | "currentfile") {
        return Some("file");
    }
    if matches!(n, "fontname") {
        return Some("font");
    }
    None
}

/// A canvas method's colour parameters, named for short (`c`, `fc` …).
fn canvas_color(method: &str, param: &str) -> bool {
    let p = param.to_ascii_lowercase();
    let m = method.to_ascii_lowercase();
    matches!(m.as_str(), "circle" | "fillrect" | "line" | "paint" | "pset" | "rectangle" | "roundrect" | "textout" | "textrect") && matches!(p.as_str(), "c" | "fc" | "bc" | "borderc" | "fill")
}

fn fits(kind: Option<&str>, want: &str) -> bool {
    match (kind, want) {
        (Some(k), "file") => k.starts_with("file:"),
        (Some(k), w) => k == w,
        (None, _) => false,
    }
}

fn all_components() -> impl Iterator<Item = &'static Component> {
    COMPONENTS.iter().chain(GLOBALS).chain(ITEMS)
}

#[test]
fn every_colour_file_and_font_value_has_its_kind() {
    let mut missing = Vec::new();
    for c in all_components() {
        for p in c.properties {
            let by_type = match (p.ty, p.editor) {
                (_, Some("file")) | (_, Some("picture")) | (Type::Picture, _) => Some("file"),
                (Type::Color, _) => Some("color"),
                (Type::Font, _) => Some("font"),
                _ => None,
            };
            // (a count, a size, a handle named after a colour or a file is a number)
            let numbers = matches!(p.ty, Type::Int) && named(p.name) != Some("color") || matches!(p.ty, Type::Bool | Type::Resource);
            let want = by_type.or(if numbers { None } else { named(p.name) });
            if let Some(w) = want {
                if !fits(p.kind, w) && !exempt(c.name, "", p.name) {
                    missing.push(format!("{}.{} ({w}): kind {:?}", c.name, p.name, p.kind));
                }
            }
        }
        for m in c.methods {
            for p in m.params {
                let name = p.name.trim_end_matches("()");
                let want = if canvas_color(m.name, name) { Some("color") } else { named(name) };
                if let Some(w) = want {
                    if !fits(m.arg_kind(name), w) && !exempt(c.name, m.name, name) {
                        missing.push(format!("{}.{}({name}) ({w}): kind {:?}", c.name, m.name, m.arg_kind(name)));
                    }
                }
            }
        }
    }
    for b in BUILTINS {
        for n in b.param_names() {
            if let Some(w) = named(&n) {
                if !fits(b.arg_kind(&n), w) {
                    missing.push(format!("{}({n}) ({w}): kind {:?}", b.name, b.arg_kind(&n)));
                }
            }
        }
    }
    assert!(missing.is_empty(), "values without their kind (add `kind` / `arg_kinds`, or list them in NOT_VALUES with why):\n{}", missing.join("\n"));
}

#[test]
fn kinds_name_real_things() {
    let check = |k: &'static str, what: &str| {
        let v = ValueKind::parse(k).unwrap_or_else(|| panic!("{what}: kind `{k}`"));
        for t in v.component_types() {
            assert!(rapidr_lang::component(t).is_some(), "{what}: kind `{k}` names `{t}`, no component");
        }
        if let ValueKind::Choice(set) = v {
            if set != "values" {
                assert!(!choices(set, None).0.is_empty(), "{what}: `{k}` has no constants");
            }
        }
    };
    for c in all_components() {
        for p in c.properties {
            if let Some(k) = p.kind {
                check(k, &format!("{}.{}", c.name, p.name));
                if let ValueKind::Choice("values") = ValueKind::parse(k).unwrap() {
                    assert!(!choices("values", Some(p)).0.is_empty(), "{}.{}: choice:values without values", c.name, p.name);
                }
            }
        }
        for m in c.methods {
            for (n, k) in m.arg_kinds {
                check(k, &format!("{}.{}({n})", c.name, m.name));
            }
        }
    }
    for b in BUILTINS {
        for (n, k) in b.arg_kinds {
            check(k, &format!("{}({n})", b.name));
        }
    }
}

/// (POLISH-2) Enabled, Visible and ShowHint are on / off on every component
/// that has them (a form's too): the boolean kind the debugger's "1 · on"
/// and the helpers' choices read. (QTRANSIMAGE's, typed `any`, are what
/// RapidQ's include declares.)
#[test]
fn on_off_properties_are_boolean_everywhere() {
    let mut wrong = Vec::new();
    for c in all_components() {
        for p in c.properties {
            if ["Enabled", "Visible", "ShowHint"].iter().any(|n| p.name.eq_ignore_ascii_case(n)) && !matches!(p.ty, Type::Bool | Type::Any) {
                wrong.push(format!("{}.{}: {:?}", c.name, p.name, p.ty));
            }
        }
    }
    assert!(wrong.is_empty(), "not on / off: {wrong:?}");
    let form = rapidr_lang::component("RFORM").unwrap();
    assert_eq!(form.property("Enabled").unwrap().kind, Some("choice:bool"));
    assert_eq!(form.property("Visible").unwrap().kind, Some("choice:bool"));
}

#[test]
fn the_kinds_the_helpers_rely_on() {
    let button = rapidr_lang::component("RBUTTON").unwrap();
    assert_eq!(button.property("Color").unwrap().kind, Some("color"));
    assert_eq!(button.property("Font").unwrap().kind, Some("font"));
    assert_eq!(button.property("BMP").unwrap().kind, Some("file:image"));
    assert_eq!(button.property("Align").unwrap().kind, Some("choice:values"));
    assert_eq!(button.property("Cursor").unwrap().kind, Some("cursor"));
    assert_eq!(button.property("Enabled").unwrap().kind, Some("choice:bool"));
    assert_eq!(button.property("Parent").unwrap().kind, Some("component:QFORM|QPANEL|QTABCONTROL"));
    assert_eq!(rapidr_lang::component("RFONT").unwrap().property("Name").unwrap().kind, Some("font"));
    let image = rapidr_lang::component("RIMAGE").unwrap();
    assert_eq!(image.method("Load").unwrap().arg_kind("filename"), Some("file:image"));
    assert_eq!(rapidr_lang::builtin("RGB").unwrap().kind, Some("color"));
    assert_eq!(rapidr_lang::builtin("PLAYWAV").unwrap().arg_kind("FileName"), Some("file:sound"));
    assert_eq!(rapidr_lang::builtin("MESSAGEDLG").unwrap().arg_kind("MsgButtons"), Some("choice:msgdlg-buttons"));
    assert_eq!(rapidr_lang::kinds::constant_kind("clRed"), Some("color"));
    assert_eq!(rapidr_lang::kinds::constant_kind("clBtnFace"), Some("color"));
    assert_eq!(rapidr_lang::kinds::constant_kind("alClient"), None);
    let (buttons, flags) = choices("msgdlg-buttons", None);
    assert!(flags && buttons.contains(&"mbYes"));
    assert!(CONSTANT_GROUPS.iter().filter(|g| g.name == "Colours").all(|g| g.kind == Some("color")));
    assert!(FileKind::Sound.matches("click.WAV"));
    assert_eq!(rapidr_lang::syntax_param_names("PLAYWAV FileName$|Resource_Handle, SndOption"), ["FileName", "SndOption"]);
    assert_eq!(rapidr_lang::syntax_param_names("MID$(Text$, Position, Num)"), ["Text", "Position", "Num"]);
    assert_eq!(rapidr_lang::syntax_param_names("LOCATE [Y%][, X%][, cursor]"), ["Y", "X", "cursor"]);
}
