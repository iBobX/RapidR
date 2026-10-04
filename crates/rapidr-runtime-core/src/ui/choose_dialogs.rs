//! QCOLORDIALOG and QFONTDIALOG, as `file_dialog.rs` is for Open / Save:
//! what the program asked for read from the component, the host's dialog
//! shown through a `pick` closure (the kernel's drawn dialogs), and the
//! answer stored the same way (`rapidr_value::color_dialog`,
//! `rapidr_value::font_dialog`). Under a test, `RAPIDR_TEST_COLOR_DIALOG` /
//! `RAPIDR_TEST_FONT_DIALOG` answer and nothing is shown.

use rapidr_value::color_dialog as cd;
use rapidr_value::font_dialog as fd;
use rapidr_value::objects::font::Font;

use crate::object::{rp_comp_get, rp_comp_set, rp_fire_event};
use crate::value::{v_int, Value};

/// A dialog's Caption, else `default` (the system's title).
pub fn title(name: &str, default: &str) -> String {
    Some(rp_comp_get(name, "caption").to_string_val()).filter(|c| !c.is_empty()).unwrap_or_else(|| default.to_string())
}

/// A QCOLORDIALOG's `Colors(1 TO 16)`.
pub fn custom_colors(name: &str) -> [i64; 16] {
    cd::custom_colors(|i| match rp_comp_get(name, &format!("colors({i})")) {
        Value::Null => None,
        v => Some(v.to_i64()),
    })
}

/// QCOLORDIALOG `name`'s state as the program set it (Color, Colors,
/// Style).
pub fn color_state(name: &str) -> cd::State {
    let style = match rp_comp_get(name, "style") {
        Value::Null => cd::CD_NO_FULL_OPEN,
        v => v.to_i64(),
    };
    cd::State::new(rp_comp_get(name, "color").to_i64(), custom_colors(name), style)
}

/// `ColorDialog.Execute`: the colour `pick` answers for the title and the
/// state (the test's answer instead under `RAPIDR_TEST_COLOR_DIALOG`;
/// `None`: Cancel) into Color, the custom colours it kept into Colors(i)
/// either way. 1 when a colour was picked, as RAPIDQ2.INC's Execute.
pub fn color_execute(name: &str, pick: impl FnOnce(&str, cd::State) -> (Option<i64>, [i64; 16])) -> Value {
    let state = color_state(name);
    let (color, custom) = match crate::ui::testhooks::color_dialog_answer() {
        Some(answer) => (answer, state.custom),
        None => pick(&title(name, "Color"), state),
    };
    for (i, c) in custom.iter().enumerate() {
        rp_comp_set(name, &format!("colors({})", i + 1), v_int(*c));
    }
    match color {
        Some(c) => {
            rp_comp_set(name, "color", v_int(c & 0xFF_FFFF));
            v_int(1)
        }
        None => v_int(0),
    }
}

/// QFONTDIALOG `name`'s request: its font so far and its options.
pub fn font_request(name: &str) -> fd::Request {
    fd::request(&|p| rp_comp_get(name, p))
}

/// The faces a font dialog lists: the shared ones, and the system's
/// (`system`) unless a test runs (so tests see the same list everywhere).
pub fn font_names(system: impl FnOnce() -> Vec<String>) -> Vec<String> {
    let mut names: Vec<String> = fd::FONT_NAMES.iter().map(|s| s.to_string()).collect();
    if !crate::ui::testhooks::under_test() {
        names.extend(system());
    }
    names
}

/// The font chosen into QFONTDIALOG `name`'s properties (Name, Size,
/// Color, the styles, and the flat FontName …).
pub fn font_store(name: &str, font: &Font) {
    for (p, v) in fd::properties(font) {
        crate::object::store_prop(&name.to_lowercase(), p, v);
    }
}

/// Apply (fdApplyButton) pressed in QFONTDIALOG `name`: the font so far
/// stored, then its OnApply.
pub fn font_applied(name: &str, font: &Font) {
    font_store(name, font);
    rp_fire_event(name, "onapply");
}

/// `FontDialog.Execute`: the font `pick` answers for the title and the
/// request (the test's answer instead under `RAPIDR_TEST_FONT_DIALOG`;
/// `None`: Cancel) stored. 1 when one was chosen.
pub fn font_execute(name: &str, pick: impl FnOnce(&str, fd::Request) -> Option<Font>) -> Value {
    let req = font_request(name);
    let chosen = match crate::ui::testhooks::font_dialog_answer() {
        Some(answer) => answer,
        None => pick(&title(name, "Font"), req),
    };
    match chosen {
        Some(f) => {
            font_store(name, &f);
            v_int(1)
        }
        None => v_int(0),
    }
}
