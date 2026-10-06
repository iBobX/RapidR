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
//! Also here: the Color a program reads from a component whose Color it
//! never set ([`color_read`]) — RapidQ's system colours and ParentColor, as
//! RC.EXE shows them (docs/rapidq-ground-truth.md).

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
            // (a QLABEL's AutoSize is True until set, RC.EXE: crate::autosize)
            if type_name.eq_ignore_ascii_case("RLABEL") {
                put("autosize", v_bool(true));
            }
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
            put("shape", v_int(0));
            put("style", v_int(0));
            put("bevelouter", v_int(0));
            put("bevelinner", v_int(0));
        }
        // (I1) RDOCKMANAGER: crate::dock (DocumentMode "mdi": its model's).
        "RDOCKMANAGER" => {
            put("left", v_int(0));
            put("top", v_int(0));
            put("visible", v_bool(true));
            put("enabled", v_bool(true));
            put("align", v_int(0));
            put("hint", v_str(""));
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
            put("color", v_int(CL_BTN_FACE));
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
        // (the I/O lane's QCOMPORT: the runtime looks for its OnRxChar like
        // a timer's ticks — runtime-core io.rs, io_web.rs)
        "RCOMPORT" => put("enabled", v_bool(true)),
        // (the media objects' Timer is their model's: media.rs)
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
            put("borderstyle", v_int(2));
        }
        "RBUTTON" => {
            put("enabled", v_bool(true));
            put("visible", v_bool(true));
        }
        "RLABEL" => {
            put("visible", v_bool(true));
            put("alignment", v_int(0));
            put("fontcolor", v_int(0));
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
            put("pencolor", v_int(0));
            put("penwidth", v_int(1));
            put("brushcolor", v_int(0xFFFFFF));
            put("fontcolor", v_int(0));
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

/// clBtnFace, Windows' system colour COLOR_BTNFACE as a Delphi TColor
/// (`&H8000000F`): what a QFORM's and a QPANEL's Color reads until the
/// program sets one (RC.EXE).
pub const CL_BTN_FACE: i64 = -2147483633;
/// clWindow (`&H80000005`): the Color of most other components until set,
/// and of a QLABEL / QCANVAS / QGROUPBOX without a parent.
pub const CL_WINDOW: i64 = -2147483643;

/// What `component.Color` reads in a program, as RapidQ has it (RC.EXE on
/// Windows 11, docs/rapidq-ground-truth.md): the Color the program set
/// (`stored`, not Null: then None, the runtime's value stands); else for a
/// QFORM, QPANEL (QBEVEL) clBtnFace; for a QLABEL, QCANVAS or QGROUPBOX its
/// parent's Color, followed live (Delphi's ParentColor: `parent()` reads
/// it, None without a parent) and clWindow without one; for the other
/// components RC.EXE was asked about, clWindow. None too for a type
/// without such a default. The kernel draws a never-set Color as before —
/// the face for a form or panel, nothing for a label, the parent through a
/// canvas — which is what these system colours are on screen
/// (`crate::objects::form_color` turns a system colour into the theme's).
pub fn color_read(type_name: &str, stored: &Value, parent: impl FnOnce() -> Option<Value>) -> Option<Value> {
    if !matches!(stored, Value::Null) {
        return None;
    }
    match type_name.to_ascii_uppercase().as_str() {
        "RFORM" | "RPANEL" | "RBEVEL" => Some(v_int(CL_BTN_FACE)),
        "RLABEL" | "RCANVAS" | "RGROUPBOX" => Some(parent().unwrap_or_else(|| v_int(CL_WINDOW))),
        "RBUTTON" | "REDIT" | "RMEMO" | "RRICHEDIT" | "RLISTBOX" | "RCOMBOBOX" | "RSTRINGGRID" | "RSCROLLBOX" | "RTABCONTROL" => Some(v_int(CL_WINDOW)),
        _ => None,
    }
}

/// clWindowText (`&H80000008`): every component's Font.Color, and a new
/// QFONT's Color, until the program sets one (RC.EXE).
pub const CL_WINDOW_TEXT: i64 = -2147483640;

/// What `component.Font.Color` reads in a program, as RapidQ has it
/// (RC.EXE): the colour the program set (`set`: the runtime's mark), else
/// its parent's Font.Color followed live (Delphi's ParentFont, every
/// component) — `parent()`, None without a parent — and clWindowText at the
/// top. The kernel draws an unset one in the theme's text colour and a
/// parent's chosen one as RapidQ draws ParentFont (the kernel stores).
pub fn font_color_read(set: bool, value: Value, parent: impl FnOnce() -> Option<Value>) -> Value {
    if set {
        return value;
    }
    parent().unwrap_or_else(|| v_int(CL_WINDOW_TEXT))
}

/// Whether a component of `type_name` is a window of its own (Delphi's
/// TWinControl): what a form's Pixel reads over it is -1, as RC.EXE shows
/// (the form's canvas is clipped to its own client area); the graphic
/// controls — QLABEL, QCANVAS, QIMAGE — are drawn on the form, so Pixel
/// reads what they show.
pub fn is_windowed(type_name: &str) -> bool {
    matches!(
        type_name.to_ascii_uppercase().as_str(),
        "RPANEL" | "RBEVEL" | "RBUTTON" | "REDIT" | "RMEMO" | "RRICHEDIT" | "RLISTBOX" | "RCOMBOBOX" | "RSTRINGGRID" | "RSCROLLBOX" | "RTABCONTROL"
            | "RGROUPBOX" | "RCHECKBOX" | "RRADIOBUTTON" | "RLISTVIEW" | "RTREEVIEW" | "RFILELISTBOX" | "RDIRTREE" | "RSTATUSBAR" | "RTRACKBAR"
            | "RSCROLLBAR" | "RPROGRESSBAR" | "RHEADER" | "RDXSCREEN" | "RCODEEDITOR" | "RFORM"
    )
}

/// One of a form's children, as [`form_pixel`] needs it.
pub struct PixelChild {
    pub id: String,
    pub type_name: String,
    /// Left, Top, Width, Height in the form's client area.
    pub rect: (i64, i64, i64, i64),
    /// The Color the program set, if it did.
    pub color: Option<i64>,
}

/// What `Form.Pixel(x, y)` reads, as RapidQ's (RC.EXE): -1 while the form
/// isn't showing (before Show, after Close) and outside its client area;
/// -1 over a window of its own (a panel, a button, an edit …: Windows'
/// GetPixel on the form's clipped DC); over a graphic control what it
/// shows — a label its Color (none set: the form's `color`), a canvas its
/// pixel, an image its picture (white where there is none). None: the
/// form's own surface answers. `children` in creation order (the last on
/// top).
pub fn form_pixel(shown: bool, client: (i64, i64), x: i64, y: i64, color: i64, children: &[PixelChild]) -> Option<i64> {
    if !shown || x < 0 || y < 0 || x >= client.0 || y >= client.1 {
        return Some(-1);
    }
    let inside = |c: &&PixelChild| {
        let (l, t, w, h) = c.rect;
        x >= l && y >= t && x < l + w && y < t + h
    };
    if children.iter().filter(inside).any(|c| is_windowed(&c.type_name)) {
        return Some(-1);
    }
    for c in children.iter().rev().filter(inside) {
        let (l, t, _, _) = c.rect;
        match c.type_name.to_ascii_uppercase().as_str() {
            "RLABEL" => return Some(crate::objects::color_bgr(c.color.unwrap_or(color)) as i64),
            "RCANVAS" => {
                if let Some(p) = crate::objects::bitmap_pixel(&c.id, x - l, y - t) {
                    return Some(p as i64);
                }
            }
            "RIMAGE" => return Some(crate::objects::bitmap_pixel(&c.id, x - l, y - t).map_or(0xFFFFFF, i64::from)),
            _ => {}
        }
    }
    None
}

/// Whether a component of `type_name` takes its parent's Color while it
/// has none of its own (Delphi's ParentColor, as RC.EXE shows it).
pub fn parent_color(type_name: &str) -> bool {
    matches!(type_name.to_ascii_uppercase().as_str(), "RLABEL" | "RCANVAS" | "RGROUPBOX")
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
    fn font_colors_and_form_pixels_as_rapidq() {
        assert_eq!(font_color_read(false, Value::Null, || None), v_int(CL_WINDOW_TEXT));
        assert_eq!(font_color_read(false, v_int(0), || Some(v_int(0xFF))), v_int(0xFF));
        assert_eq!(font_color_read(true, v_int(0x123456), || Some(v_int(0xFF))), v_int(0x123456));
        let kids = [
            PixelChild { id: "p".into(), type_name: "RPANEL".into(), rect: (10, 10, 60, 40), color: None },
            PixelChild { id: "l".into(), type_name: "RLABEL".into(), rect: (150, 10, 60, 30), color: Some(0xFF00) },
            PixelChild { id: "l2".into(), type_name: "RLABEL".into(), rect: (150, 50, 60, 30), color: None },
        ];
        assert_eq!(form_pixel(false, (300, 200), 100, 100, CL_BTN_FACE, &kids), Some(-1), "not shown");
        assert_eq!(form_pixel(true, (300, 200), -1, 5, CL_BTN_FACE, &kids), Some(-1));
        assert_eq!(form_pixel(true, (300, 200), 300, 5, CL_BTN_FACE, &kids), Some(-1));
        assert_eq!(form_pixel(true, (300, 200), 30, 30, CL_BTN_FACE, &kids), Some(-1), "over a window");
        assert_eq!(form_pixel(true, (300, 200), 170, 20, CL_BTN_FACE, &kids), Some(0xFF00), "a label's Color");
        assert_eq!(form_pixel(true, (300, 200), 170, 60, 0xFF, &kids), Some(0xFF), "a label shows the form's");
        assert_eq!(form_pixel(true, (300, 200), 100, 150, CL_BTN_FACE, &kids), None, "the surface");
    }

    #[test]
    fn colors_as_rapidq_reads_them() {
        assert_eq!(color_read("RFORM", &Value::Null, || None), Some(v_int(CL_BTN_FACE)));
        assert_eq!(color_read("rpanel", &Value::Null, || None), Some(v_int(CL_BTN_FACE)));
        assert_eq!(color_read("RLABEL", &Value::Null, || None), Some(v_int(CL_WINDOW)));
        assert_eq!(color_read("RLABEL", &Value::Null, || Some(v_int(0xFF))), Some(v_int(0xFF)));
        assert_eq!(color_read("REDIT", &Value::Null, || Some(v_int(0xFF))), Some(v_int(CL_WINDOW)));
        assert_eq!(color_read("RFORM", &v_int(0xFF), || None), None, "the program's");
        assert_eq!(color_read("RTIMER", &Value::Null, || None), None);
        assert!(get(&desktop("RFORM"), "color").is_none());
    }
}
