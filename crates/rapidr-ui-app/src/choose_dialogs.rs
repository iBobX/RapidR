//! QCOLORDIALOG and QFONTDIALOG, as `file_dialog.rs` is for Open / Save:
//! what the program asked for read from the component, and the answer
//! stored once the dialog closes (`rapidr_value::color_dialog`,
//! `rapidr_value::font_dialog`). The dialogs are the kernel's
//! (`Dialog::color` / `Dialog::font`), shown and waited for by `dialogs.rs`;
//! under a test `RAPIDR_TEST_COLOR_DIALOG` / `RAPIDR_TEST_FONT_DIALOG`
//! answer instead.

use rapidr_value::color_dialog as cd;
use rapidr_value::font_dialog as fd;
use rapidr_value::objects::font::Font;
use rapidr_value::{v_int, Value};

use crate::Program;

/// A dialog's Caption, else `default` (the system's title).
pub fn title<P: Program>(p: P, name: &str, default: &str) -> String {
    Some(p.get(name, "caption").to_string_val()).filter(|c| !c.is_empty()).unwrap_or_else(|| default.to_string())
}

/// A QCOLORDIALOG's `Colors(1 TO 16)`.
pub fn custom_colors<P: Program>(p: P, name: &str) -> [i64; 16] {
    cd::custom_colors(|i| match p.get(name, &format!("colors({i})")) {
        Value::Null => None,
        v => Some(v.to_i64()),
    })
}

/// QCOLORDIALOG `name`'s state as the program set it (Color, Colors,
/// Style).
pub fn color_state<P: Program>(p: P, name: &str) -> cd::State {
    let style = match p.get(name, "style") {
        Value::Null => cd::CD_NO_FULL_OPEN,
        v => v.to_i64(),
    };
    cd::State::new(p.get(name, "color").to_i64(), custom_colors(p, name), style)
}

/// `ColorDialog.Execute`'s answer: the colour picked (`None`: Cancel) into
/// Color, the custom colours the dialog kept into Colors(i) either way.
/// Execute's result: 1 when a colour was picked, as RAPIDQ2.INC's Execute.
pub fn color_answered<P: Program>(p: P, name: &str, color: Option<i64>, custom: [i64; 16]) -> Value {
    for (i, c) in custom.iter().enumerate() {
        p.set(name, &format!("colors({})", i + 1), v_int(*c));
    }
    match color {
        Some(c) => {
            p.set(name, "color", v_int(c & 0xFF_FFFF));
            v_int(1)
        }
        None => v_int(0),
    }
}

/// QFONTDIALOG `name`'s request: its font so far and its options.
pub fn font_request<P: Program>(p: P, name: &str) -> fd::Request {
    fd::request(&|prop| p.get(name, prop))
}

/// The faces a font dialog lists: the shared ones, and the system's
/// (`system`) unless a test runs (so tests see the same list everywhere).
pub fn font_names(system: impl FnOnce() -> Vec<String>) -> Vec<String> {
    let mut names: Vec<String> = fd::FONT_NAMES.iter().map(|s| s.to_string()).collect();
    if !crate::testhooks::under_test() {
        names.extend(system());
    }
    names
}

/// The font chosen into QFONTDIALOG `name`'s properties (Name, Size,
/// Color, the styles, and the flat FontName …).
pub fn font_store<P: Program>(p: P, name: &str, font: &Font) {
    for (prop, v) in fd::properties(font) {
        p.store(&name.to_lowercase(), prop, v);
    }
}

/// Apply (fdApplyButton) pressed in QFONTDIALOG `name`: the font so far
/// stored, then its OnApply.
pub fn font_applied<P: Program>(p: P, name: &str, font: &Font) {
    font_store(p, name, font);
    p.fire(name, "onapply");
}

/// `FontDialog.Execute`'s answer, the font chosen (`None`: Cancel),
/// stored. Execute's result: 1 when one was chosen.
pub fn font_answered<P: Program>(p: P, name: &str, chosen: Option<Font>) -> Value {
    match chosen {
        Some(f) => {
            font_store(p, name, &f);
            v_int(1)
        }
        None => v_int(0),
    }
}
