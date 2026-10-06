//! Open / Save dialogs (QOPENDIALOG, QSAVEDIALOG, RAPIDQ2.INC's
//! QFILEDIALOG): what the program asked for read from the component
//! ([`Request`]: Caption or Title, Filter / FilterIndex, InitialDir, a
//! FileName preset, WarnIfOverWrite), and the answer stored once the
//! dialog closes ([`answered`]): FileName, FileTitle, Files(…), SelCount,
//! DefaultExt added to a saved name (`rapidr_value::file_dialog`). The
//! host shows its dialog (`Windows::ask_files`: the desktop's rfd sheet;
//! on the web the browser's own pickers, docs/web-host-plan.md §3.8) and the
//! program waits for it as for any dialog (`dialogs.rs`); under a test
//! `RAPIDR_TEST_FILE_DIALOG` answers instead.

use rapidr_value::file_dialog as fd;
use rapidr_value::{v_int, v_str, Value};

use crate::Program;

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
    /// DefaultExt ("txt"; "" none): added to a saved name without one —
    /// the name a browser's save picker proposes has it already.
    pub default_ext: String,
    /// (RapidR's) An Open dialog's PickFolder: a folder is chosen, not a
    /// file (FileName is the folder; on the web its files are read in).
    pub folder: bool,
}

/// Which file dialog component type `comp_type` is: (save, multi), or
/// `None` for another dialog (colour, font). QFILEDIALOG: Mode fdOpen 0 /
/// fdSave 1, MultiSelect for an Open.
pub fn kind<P: Program>(p: P, name: &str, comp_type: &str) -> Option<(bool, bool)> {
    match comp_type {
        "ROPENDIALOG" => Some((false, false)),
        "RSAVEDIALOG" => Some((true, false)),
        "RFILEDIALOG" => {
            let save = p.get(name, "mode").to_i64() == 1;
            Some((save, !save && p.get(name, "multiselect").to_bool()))
        }
        _ => None,
    }
}

/// Dialog `name`'s request, from its properties.
pub fn request<P: Program>(p: P, name: &str, save: bool, multi: bool) -> Request {
    let prop = |prop: &str| p.get(name, prop).to_string_val();
    let filters = fd::parse_filter(&prop("filter"));
    // (FilterIndex counts from 1)
    let index = p.get(name, "filterindex").to_i64();
    let filter_index = if filters.is_empty() { 0 } else { (index.max(1) - 1).min(filters.len() as i64 - 1) as usize };
    let warn = p.get(name, "warnifoverwrite");
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
        default_ext: prop("defaultext"),
        folder: !save && p.get(name, "pickfolder").to_bool(),
    }
}

/// `Dialog.Execute`'s answer, the paths picked (none: Cancel), stored in
/// dialog `name`: Execute's result, True (-1) when something was picked.
pub fn answered<P: Program>(p: P, name: &str, save: bool, multi: bool, paths: Vec<String>) -> Value {
    let mut paths: Vec<String> = paths.into_iter().filter(|p| !p.is_empty()).collect();
    if paths.is_empty() {
        return v_int(0);
    }
    if !multi {
        paths.truncate(1);
    }
    if save {
        paths[0] = fd::with_default_ext(&paths[0], &p.get(name, "defaultext").to_string_val());
    }
    let picked = fd::picked(&paths);
    p.set(name, "filename", v_str(&picked.file_name));
    p.set(name, "filetitle", v_str(&picked.file_title));
    p.set(name, "selcount", v_int(picked.sel_count));
    for (i, f) in picked.files.iter().enumerate() {
        p.set(name, &format!("files({i})"), v_str(f));
    }
    v_int(-1)
}
