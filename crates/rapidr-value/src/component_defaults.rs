//! What a new component starts with, as far as both runtimes' registries
//! already agree (docs/web-host-plan.md, "The component registry: not
//! unified", step 1 — Stage W3): the desktop's `RpComponent::new`
//! (runtime-core's `object.rs`) and the web's `rp_create_component`
//! (`object_web.rs`) both fill a new component's properties from
//! [`shared`] first, then add what only they give it. Nothing either
//! runtime gave a component changed: every property here had the same
//! value in both before.
//!
//! The rest (a RapidQ default only one runtime gives — the desktop's
//! QFORM Color and BorderStyle, QLABEL's Visible / Alignment / FontSize, the
//! web's QCOOLBTN …) is step 2: each difference becomes a conformance case
//! read on the three runtimes, RapidQ's value decided and moved here.
//!
//! Also here, for both kernel stores (runtime-core's `RtStore`, the web's
//! `WebStore`): which stored values the UI kernel reads as unset
//! ([`kernel_reads_unset`]).

use crate::{v_bool, v_int, v_str, Value};

/// The properties both runtimes give a new component of type `type_name`
/// (any case; a QFILEDIALOG's extras only for `RFILEDIALOG` written so, as
/// both registries checked), before their own.
pub fn shared(type_name: &str) -> Vec<(String, Value)> {
    let mut p: Vec<(String, Value)> = Vec::new();
    let mut put = |k: &str, v: Value| p.push((k.to_string(), v));
    match type_name.to_ascii_uppercase().as_str() {
        "RFORM" => {
            put("caption", v_str(""));
            put("left", v_int(100));
            put("top", v_int(100));
            // (hidden until shown, as in RapidQ)
            put("visible", v_bool(false));
            // (RC.EXE: a new QFORM's Enabled reads 1)
            put("enabled", v_bool(true));
            // (the WindowState lane's: wsNormal)
            put("windowstate", v_int(crate::window_state::WS_NORMAL));
        }
        "RBUTTON" | "RLABEL" | "RCHECKBOX" | "RRADIOBUTTON" => {
            put("caption", v_str(""));
            put("left", v_int(0));
            put("top", v_int(0));
        }
        "REDIT" | "RMEMO" | "RRICHEDIT" => {
            put("text", v_str(""));
            put("left", v_int(0));
            put("top", v_int(0));
        }
        // (items, selection, cells, pictures, pens …: the shared models)
        "RPANEL" | "RCOMBOBOX" | "RLISTBOX" | "RFILELISTBOX" | "RDIRTREE" | "RIMAGE" | "RCANVAS" | "RDXSCREEN" | "RSTRINGGRID" | "RTABCONTROL" | "RTREEVIEW"
        | "RGROUPBOX" | "RSPLITTER" | "RSCROLLBOX" | "RLISTVIEW" | "RTOOLBAR" | "RCODEEDITOR" => {
            put("left", v_int(0));
            put("top", v_int(0));
        }
        // (the input lane's: its size grip shows — RapidQ's default; docked
        // at the bottom: layout::default_align)
        "RSTATUSBAR" => {
            put("left", v_int(0));
            put("top", v_int(0));
            put("sizegrip", v_bool(true));
        }
        "RHEADER" => {
            // Sections: crate::objects::header; a canvas to draw on.
            put("left", v_int(0));
            put("top", v_int(0));
            put("color", v_int(0xF0F0F0));
        }
        "RPROGRESS" | "RPROGRESSBAR" => {
            put("left", v_int(0));
            put("top", v_int(0));
            put("min", v_int(0));
            put("max", v_int(100));
            put("position", v_int(0));
        }
        // QTIMER: Enabled is True by default (manual).
        "RTIMER" => {
            put("enabled", v_bool(true));
            put("interval", v_int(1000));
        }
        // (the DirectX lane's) QDXTIMER (manual: Enabled False, ActiveOnly
        // True; DelphiX's Interval 1000).
        "RDXTIMER" => {
            put("enabled", v_bool(false));
            put("interval", v_int(1000));
            put("activeonly", v_bool(true));
        }
        // (QDXSOUND: its sound's properties are the model's; these
        // DirectSound streaming settings only kept — manual's defaults)
        "RDXSOUND" => {
            put("autoupdate", v_bool(true));
            put("bufferlength", v_int(1000));
            put("stickyfocus", v_bool(false));
        }
        "RDXJOYSTICK" => put("enabled", v_bool(true)),
        "ROPENDIALOG" | "RSAVEDIALOG" | "RFILEDIALOG" => {
            put("filename", v_str(""));
            put("filetitle", v_str(""));
            put("filter", v_str(""));
            put("filterindex", v_int(1));
            put("initialdir", v_str(""));
            put("title", v_str(""));
            put("selcount", v_int(0));
            if type_name == "RFILEDIALOG" {
                put("caption", v_str("Open"));
                put("filter", v_str("All Files|*.*"));
                put("mode", v_int(0));
                put("multiselect", v_bool(false));
                put("warnifoverwrite", v_bool(true));
            }
        }
        // (the dialogs lane's: RAPIDQ2.INC's QColorDialog — Color 0, Style
        // cdNoFullOpen, its constructor's Colors(1 TO 16))
        "RCOLORDIALOG" => {
            put("color", v_int(0));
            put("style", v_int(crate::color_dialog::CD_NO_FULL_OPEN));
            for (i, c) in crate::color_dialog::DEFAULT_CUSTOM.iter().enumerate() {
                p.push((format!("colors({})", i + 1), v_int(*c)));
            }
        }
        // (TFontDialog's: the default QFONT, Options [fdEffects], no size
        // limits; FontCount)
        "RFONTDIALOG" => {
            for (k, v) in crate::font_dialog::defaults() {
                put(k, v);
            }
        }
        "RFILESTREAM" => {
            put("filename", v_str(""));
            put("position", v_int(0));
        }
        "RJSON" => {
            put("text", v_str(""));
            put("filename", v_str(""));
            put("count", v_int(0));
        }
        // (database.rs / database_web.rs keep the rest)
        "RSQLITE" => {
            put("connected", v_int(0));
            put("db", v_str(""));
            put("rowcount", v_int(0));
            put("colcount", v_int(0));
            put("fieldcount", v_int(0));
        }
        _ => {}
    }
    p
}

/// What only the desktop's registry gives a new component of type
/// `type_name` (any case), after [`shared`]: RapidQ's defaults the web's
/// DOM host doesn't give (QFORM's Color and BorderStyle, QLABEL's Visible,
/// Alignment, FontSize …) — step 2 of the registry's path decides each.
/// The web's kernel host gives them too, so the kernel draws and the program
/// reads what it does on the desktop.
pub fn desktop(type_name: &str) -> Vec<(String, Value)> {
    let mut p: Vec<(String, Value)> = Vec::new();
    let mut put = |k: &str, v: Value| p.push((k.to_string(), v));
    match type_name.to_ascii_uppercase().as_str() {
        "RFORM" => {
            put("color", v_int(0xFFFFFF));
            put("borderstyle", v_int(2));
        }
        "RBUTTON" => {
            put("enabled", v_bool(true));
            put("visible", v_bool(true));
        }
        "RLABEL" => {
            put("visible", v_bool(true));
            put("alignment", v_int(0));
            put("color", v_int(0xFFFFFF));
            put("fontcolor", v_int(0));
            put("fontsize", v_int(12));
        }
        "REDIT" => {
            put("enabled", v_bool(true));
            put("visible", v_bool(true));
            put("readonly", v_bool(false));
            put("maxlength", v_int(0));
        }
        "RPANEL" => {
            put("caption", v_str(""));
            put("visible", v_bool(true));
            put("color", v_int(0xFFFFFF));
        }
        "RCHECKBOX" => {
            put("checked", v_int(0));
            put("enabled", v_bool(true));
            put("visible", v_bool(true));
        }
        "RRADIOBUTTON" => {
            put("checked", v_int(0));
        }
        // Nothing more than both runtimes give them: QCOMBOBOX, QLISTBOX
        // (items and selection: crate::objects::list), QTIMER
        // (Enabled True, the manual's), the DirectX lane's QDXTIMER,
        // QDXSOUND and QDXJOYSTICK, QHEADER, QSTRINGGRID (cells, sizes and
        // selection: crate::objects::grid), QTABCONTROL,
        // QPROGRESS, QJSON, QTREEVIEW, the file / colour / font dialogs,
        // the menus.
        "RCOMBOBOX" | "RLISTBOX" | "RFILELISTBOX" | "RDIRTREE" | "RTIMER" | "RDXTIMER" | "RDXSOUND" | "RDXJOYSTICK" | "RHEADER" | "RSTRINGGRID" | "RTABCONTROL"
        | "RPROGRESS" | "RJSON" | "RTREEVIEW" | "ROPENDIALOG" | "RSAVEDIALOG" | "RFILEDIALOG" | "RCOLORDIALOG" | "RFONTDIALOG" | "RMAINMENU" | "RPOPUPMENU" => {}
        "RIMAGE" => {
            put("stretch", v_bool(false));
        }
        "RCANVAS" => {
            put("color", v_int(0xFFFFFF));
            put("pencolor", v_int(0));
            put("penwidth", v_int(1));
            put("brushcolor", v_int(0xFFFFFF));
            put("fontcolor", v_int(0));
            put("fontsize", v_int(12));
            put("fontname", v_str("Arial"));
        }
        // (the DirectX lane's)
        "RDXSCREEN" => {
            put("visible", v_bool(true));
        }
        "RDESIGNSURFACE" => {
            put("formcaption", v_str("Form1"));
            put("compcount", v_int(0));
            put("visible", v_bool(true));
        }
        "RCODEEDITOR" => {
            put("text", v_str(""));
            put("visible", v_bool(true));
        }
        "RGROUPBOX" => {
            put("caption", v_str(""));
            put("visible", v_bool(true));
        }
        "RMENUITEM" => {
            put("caption", v_str(""));
            put("enabled", v_bool(true));
            put("checked", v_bool(false));
        }
        "RSTATUSBAR" => {
            // Docked at the bottom (Align = alBottom) once it has a parent.
            put("simpletext", v_str(""));
            put("simplepanel", v_bool(false));
            put("panelcount", v_int(0));
        }
        "RRICHEDIT" | "RMEMO" => {
            put("readonly", v_bool(false));
        }
        "RFILESTREAM" => {
            put("size", v_int(0));
        }
        "RSTRINGLIST" => {
            put("count", v_int(0));
            put("text", v_str(""));
        }
        "RTOOLBAR" => {
            put("width", v_int(0));
            put("height", v_int(32));
        }
        "RSCROLLBAR" => {
            put("min", v_int(0));
            put("max", v_int(100));
            put("position", v_int(0));
        }
        "RDATETIMEPICKER" => {
            put("date", v_str(""));
            put("time", v_str(""));
        }
        "RUPDOWN" => {
            put("min", v_int(0));
            put("max", v_int(100));
            put("position", v_int(0));
        }
        "RPRINTER" => {
            put("title", v_str(""));
        }
        // Database components — properties managed by database.rs
        "RSQLITE" => {
            put("tablecount", v_int(0));
        }
        "RMYSQL" => {
            put("connected", v_int(0));
            put("host", v_str("localhost"));
            put("port", v_int(3306));
            put("user", v_str(""));
            put("password", v_str(""));
            put("db", v_str(""));
            put("rowcount", v_int(0));
            put("colcount", v_int(0));
            put("fieldcount", v_int(0));
            put("dbcount", v_int(0));
        }
        // Network components — properties managed by network.rs
        "RSOCKET" => {
            put("host", v_str(""));
            put("port", v_int(0));
            put("connected", v_int(0));
            put("timeout", v_int(5000));
        }
        "RSERVERSOCKET" => {
            put("host", v_str("0.0.0.0"));
            put("port", v_int(0));
            put("clientcount", v_int(0));
        }
        "RHTTP" => {
            put("host", v_str(""));
            put("port", v_int(80));
            put("url", v_str(""));
            put("statuscode", v_int(0));
            put("responsetext", v_str(""));
            put("responseheaders", v_str(""));
            put("timeout", v_int(5000));
            put("usessl", v_int(0));
        }
        "RSPLITTER" => {
            put("minsize", v_int(30));
            put("visible", v_bool(true));
        }
        "RSCROLLBOX" => {
            put("visible", v_bool(true));
        }
        "RLISTVIEW" => {
            put("itemindex", v_int(-1));
            put("items", v_str(""));
            put("count", v_int(0));
            put("visible", v_bool(true));
        }
        "RPROGRESSBAR" => {
            put("visible", v_bool(true));
        }
        // (an unknown type: nothing more)
        _ => {}
    }
    p
}

/// The Color the desktop's registry gives a new QLABEL, QFORM and QPANEL,
/// which no host paints.
pub const CREATION_COLOR: i64 = 0xFFFFFF;

/// Whether the UI kernel reads property `prop` of a `type_name` component,
/// stored as `value`, as never set (it then uses RapidQ's default): a
/// QLABEL's, QFORM's or QPANEL's creation-default white Color (a label has
/// no background; a form and a panel are the button face) — unless the
/// program set it (`program_set`: the registry's `__colorset`), white
/// included. Both kernel stores ask this (runtime-core's `RtStore`, the
/// web's `WebStore`).
pub fn kernel_reads_unset(type_name: &str, prop: &str, value: &Value, program_set: impl FnOnce() -> bool) -> bool {
    prop.eq_ignore_ascii_case("color") && matches!(value, Value::Integer(CREATION_COLOR)) && matches!(type_name, "RLABEL" | "RFORM" | "RPANEL") && !program_set()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn get<'a>(p: &'a [(String, Value)], k: &str) -> Option<&'a Value> {
        p.iter().rev().find(|(n, _)| n == k).map(|(_, v)| v)
    }

    #[test]
    fn both_runtimes_defaults() {
        let f = shared("rform");
        assert_eq!(get(&f, "left"), Some(&v_int(100)));
        assert_eq!(get(&f, "visible"), Some(&v_bool(false)));
        assert_eq!(get(&shared("RTIMER"), "interval"), Some(&v_int(1000)));
        // (a QFILEDIALOG's extras: only for the type written so)
        assert_eq!(get(&shared("RFILEDIALOG"), "filter"), Some(&v_str("All Files|*.*")));
        assert_eq!(get(&shared("rfiledialog"), "filter"), Some(&v_str("")));
        assert!(get(&shared("RCOLORDIALOG"), "colors(16)").is_some());
        assert!(shared("RUDT").is_empty());
    }

    #[test]
    fn a_creation_white_reads_unset() {
        let white = v_int(CREATION_COLOR);
        assert!(kernel_reads_unset("RLABEL", "Color", &white, || false));
        assert!(!kernel_reads_unset("RLABEL", "color", &white, || true));
        assert!(!kernel_reads_unset("RBUTTON", "color", &white, || false));
        assert!(!kernel_reads_unset("RLABEL", "color", &v_int(0xFF), || false));
    }
}
