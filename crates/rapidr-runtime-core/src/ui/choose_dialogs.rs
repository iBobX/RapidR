//! QCOLORDIALOG (and QFONTDIALOG) for every desktop host, as
//! `file_dialog.rs` is for Open / Save: what the program asked for read
//! from the component, the host's dialog shown through a `pick` closure
//! (the kernel's drawn dialog, FLTK's chooser), and the answer stored the
//! same way (`rapidr_value::color_dialog`). Under a test,
//! `RAPIDR_TEST_COLOR_DIALOG` answers and nothing is shown.

use rapidr_value::color_dialog as cd;

use crate::object::{rp_comp_get, rp_comp_set};
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
