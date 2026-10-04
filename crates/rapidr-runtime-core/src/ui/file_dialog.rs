//! Open / Save dialogs (QOPENDIALOG, QSAVEDIALOG, RAPIDQ2.INC's
//! QFILEDIALOG) for every desktop host: what the program asked for read
//! from the component ([`Request`]: Caption or Title, Filter / FilterIndex,
//! InitialDir, a FileName preset, WarnIfOverWrite), the host's dialog shown
//! through a `pick` closure (FLTK's `NativeFileChooser`, the kernel's rfd
//! sheet), and the answer stored the same way: FileName, FileTitle,
//! Files(…), SelCount, DefaultExt added to a saved name
//! (`rapidr_value::file_dialog`). Under a test, `RAPIDR_TEST_FILE_DIALOG`
//! answers and nothing is shown.

use rapidr_value::file_dialog as fd;

use crate::object::{rp_comp_get, rp_comp_set};
use crate::value::{v_int, v_str, Value};

/// What an Open / Save dialog is asked to show.
#[derive(Clone, Debug, PartialEq)]
pub struct Request {
    pub save: bool,
    /// Several files may be picked (an Open dialog's MultiSelect).
    pub multi: bool,
    /// Caption, else Title (`None`: the system's).
    pub title: Option<String>,
    pub filters: Vec<fd::Filter>,
    /// The filter shown first (FilterIndex, from 0, within `filters`).
    pub filter_index: usize,
    /// InitialDir.
    pub dir: Option<String>,
    /// FileName's name (without its folder), proposed.
    pub file_name: Option<String>,
    /// A save asks before replacing a file (WarnIfOverWrite, by default).
    pub confirm_overwrite: bool,
}

/// Which file dialog component type `comp_type` is: (save, multi), or
/// `None` for another dialog (colour, font). QFILEDIALOG: Mode fdOpen 0 /
/// fdSave 1, MultiSelect for an Open.
pub fn kind(name: &str, comp_type: &str) -> Option<(bool, bool)> {
    match comp_type {
        "ROPENDIALOG" => Some((false, false)),
        "RSAVEDIALOG" => Some((true, false)),
        "RFILEDIALOG" => {
            let save = rp_comp_get(name, "mode").to_i64() == 1;
            Some((save, !save && rp_comp_get(name, "multiselect").to_bool()))
        }
        _ => None,
    }
}

/// Dialog `name`'s request, from its properties.
pub fn request(name: &str, save: bool, multi: bool) -> Request {
    let prop = |p: &str| rp_comp_get(name, p).to_string_val();
    let filters = fd::parse_filter(&prop("filter"));
    // (FilterIndex counts from 1)
    let index = rp_comp_get(name, "filterindex").to_i64();
    let filter_index = if filters.is_empty() { 0 } else { (index.max(1) - 1).min(filters.len() as i64 - 1) as usize };
    let warn = rp_comp_get(name, "warnifoverwrite");
    let some = |s: String| (!s.is_empty()).then_some(s);
    Request {
        save,
        multi,
        title: [prop("caption"), prop("title")].into_iter().find(|t| !t.is_empty()),
        filters,
        filter_index,
        dir: some(prop("initialdir")),
        file_name: some(fd::file_title(&prop("filename"))),
        confirm_overwrite: save && (matches!(warn, Value::Null) || warn.to_bool()),
    }
}

/// `Dialog.Execute`: the paths `pick` returns for the request (the test's
/// answer instead under `RAPIDR_TEST_FILE_DIALOG`; none: Cancel), stored
/// in the component. True (-1) when something was picked.
pub fn execute(name: &str, save: bool, multi: bool, pick: impl FnOnce(&Request) -> Vec<String>) -> Value {
    let mut paths = match crate::ui::testhooks::file_dialog_answer(multi) {
        Some(paths) => paths,
        None => pick(&request(name, save, multi)).into_iter().filter(|p| !p.is_empty()).collect(),
    };
    if paths.is_empty() {
        return v_int(0);
    }
    if !multi {
        paths.truncate(1);
    }
    if save {
        paths[0] = fd::with_default_ext(&paths[0], &rp_comp_get(name, "defaultext").to_string_val());
    }
    let picked = fd::picked(&paths);
    rp_comp_set(name, "filename", v_str(&picked.file_name));
    rp_comp_set(name, "filetitle", v_str(&picked.file_title));
    rp_comp_set(name, "selcount", v_int(picked.sel_count));
    for (i, f) in picked.files.iter().enumerate() {
        rp_comp_set(name, &format!("files({i})"), v_str(f));
    }
    v_int(-1)
}
