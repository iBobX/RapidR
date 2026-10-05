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
        // QBEVEL (QBevel.inc's TYPE EXTENDS QPANEL): bsSpacer, bsLowered —
        // no bevels (crate::objects::bevel::qbevel_bevels).
        "RBEVEL" => {
            put("caption", v_str(""));
            put("left", v_int(0));
            put("top", v_int(0));
            put("visible", v_bool(true));
            put("color", v_int(CREATION_COLOR));
            put("shape", v_int(0));
            put("style", v_int(0));
            put("bevelouter", v_int(0));
            put("bevelinner", v_int(0));
        }
        // QGLASSFRAME (RC.EXE: 105 × 105, Transparency 60, TransparentColor
        // 0, Moveable 1, Color clBtnFace): crate::objects::glass.
        "RGLASSFRAME" => {
            put("left", v_int(0));
            put("top", v_int(0));
            put("visible", v_bool(true));
            put("enabled", v_bool(true));
            put("transparency", v_int(crate::objects::glass::TRANSPARENCY));
            put("transparentcolor", v_int(0));
            put("moveable", v_int(1));
            put("color", v_int(-2147483633));
            put("hint", v_str(""));
            put("showhint", v_bool(false));
            put("align", v_int(0));
            put("cursor", v_int(0));
        }
        // QDIGDISPLAY: a canvas showing its Display (objects::digdisplay).
        "RDIGDISPLAY" => {
            put("left", v_int(0));
            put("top", v_int(0));
            put("visible", v_bool(true));
            put("color", v_int(0));
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
    prop.eq_ignore_ascii_case("color") && matches!(value, Value::Integer(CREATION_COLOR)) && matches!(type_name, "RLABEL" | "RFORM" | "RPANEL" | "RBEVEL") && !program_set()
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
