//! What a new component starts with, on every runtime: native builds and
//! the interpreter (runtime-core's `RpComponent::new`) and the web
//! (`object_web.rs`'s `rp_create_component`) fill a new component's
//! properties from [`creation`] (docs/web-host-plan.md, "The component
//! registry", step 2 — Stage W3).
//!
//! [`creation`] is the language registry's defaults (`rapidr_lang`: what a
//! program reads right after CREATE, RapidQ's value checked against RC.EXE —
//! docs/rapidq-ground-truth.md, "Values at creation"), then [`extras`]: the
//! few values the registry has no default for. A property is in one of the
//! two, never both (a test checks).
//!
//! Also here: the Color a program reads from a component whose Color it
//! never set ([`color_read`]) — RapidQ's system colours and ParentColor, as
//! RC.EXE shows them (docs/rapidq-ground-truth.md).

use crate::{v_bool, v_int, v_str, Value};

/// The properties a new component of type `type_name` (RapidR's name, any
/// case) has before the program sets any: the registry's defaults, then
/// [`extras`] (a docked one's Align too: QSTATUSBAR's alBottom …). Its
/// Width and Height: `crate::layout::default_size` — the runtimes add those.
pub fn creation(type_name: &str) -> Vec<(String, Value)> {
    let mut p = registry(type_name);
    p.extend(extras(type_name));
    p
}

/// The language registry's defaults for `type_name`'s properties, as a
/// program reads them right after CREATE / DIM. Not here: the members of
/// RapidR's extension sets (Anchors, MinWidth …: the runtimes answer them),
/// indexed and write-only properties, a Color [`color_read`] answers while
/// it's unset (RapidQ's system colours and ParentColor: never stored), and
/// the [`THEMED`] ones ([`unset_read`] reads them).
pub fn registry(type_name: &str) -> Vec<(String, Value)> {
    use rapidr_lang::Access;
    let Some(c) = rapidr_lang::component(type_name) else {
        return Vec::new();
    };
    let color_rule = color_read(type_name, &Value::Null, || None).is_some();
    c.properties
        .iter()
        .filter(|p| p.set.is_none() && !p.missing && p.indexed == 0 && p.access != Access::Write)
        .filter(|p| !(color_rule && p.name.eq_ignore_ascii_case("color")) && !themed(type_name, p.name))
        .filter_map(|p| Some((p.name.to_ascii_lowercase(), value_of(p.default?)?)))
        .collect()
}

fn value_of(d: rapidr_lang::DefaultValue) -> Option<Value> {
    use rapidr_lang::DefaultValue;
    Some(match d {
        DefaultValue::Int(i) => v_int(i),
        DefaultValue::Bool(b) => v_bool(b),
        DefaultValue::Float(f) => Value::Double(f),
        DefaultValue::Str(s) => v_str(s),
        DefaultValue::Expr(e) => v_int(rapidr_lang::eval_constant(e)?),
    })
}

/// Properties the theme draws its own way while the program hasn't set them
/// (a QGAUGE's BackColor: the classic look's white, a modern theme's track),
/// so a new component doesn't store them; a program reading one unset gets
/// the registry's default ([`unset_read`]: RapidQ's white).
pub const THEMED: &[(&str, &str)] = &[("RPROGRESSBAR", "backcolor")];

fn themed(type_name: &str, prop: &str) -> bool {
    THEMED.iter().any(|(t, p)| t.eq_ignore_ascii_case(type_name) && p.eq_ignore_ascii_case(prop))
}

/// What a program reads from a [`THEMED`] property it never set: the
/// registry's default (None: not one of them, the stored value stands).
pub fn unset_read(type_name: &str, prop: &str) -> Option<Value> {
    if !themed(type_name, prop) {
        return None;
    }
    rapidr_lang::component(type_name)?.property(prop)?.default.and_then(value_of)
}

/// What a new component of `type_name` (any case) starts with beyond the
/// registry's defaults: properties the registry has no default for (RapidR's
/// own drawing properties, the models' counters, members the registry
/// doesn't list yet).
pub fn extras(type_name: &str) -> Vec<(String, Value)> {
    let mut p: Vec<(String, Value)> = Vec::new();
    let mut put = |k: &str, v: Value| p.push((k.to_string(), v));
    match type_name.to_ascii_uppercase().as_str() {
        // QBEVEL (QBevel.inc's TYPE EXTENDS QPANEL): bsSpacer, bsLowered —
        // no bevels (crate::objects::bevel::qbevel_bevels).
        "RBEVEL" => {
            put("shape", v_int(0));
            put("style", v_int(0));
        }
        // RPLOT: a chart, shown when it's on a form (the UI kernel's
        // components::plot); its size is its model's (layout::default_size).
        "RPLOT" => {
            put("left", v_int(0));
            put("top", v_int(0));
            put("visible", v_bool(true));
        }
        // QDIGDISPLAY: a canvas showing its Display (objects::digdisplay).
        "RDIGDISPLAY" => put("color", v_int(0)),
        // (docked at the bottom: its Align, the registry's alBottom)
        "RSTATUSBAR" => {
            put("top", v_int(0));
            put("panelcount", v_int(0));
        }
        // Sections: crate::objects::header; a canvas to draw on.
        "RHEADER" => put("color", v_int(0xF0F0F0)),
        // (the I/O lane's QCOMPORT and the DirectX lane's QDXJOYSTICK: the
        // runtime looks for their events like a timer's ticks — io.rs,
        // io_web.rs, directx_web.rs)
        "RCOMPORT" | "RDXJOYSTICK" => put("enabled", v_bool(true)),
        "ROPENDIALOG" | "RSAVEDIALOG" => {
            put("filetitle", v_str(""));
            put("selcount", v_int(0));
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
        "RLABEL" => put("fontcolor", v_int(0)),
        "RCANVAS" => {
            put("pencolor", v_int(0));
            put("penwidth", v_int(1));
            put("brushcolor", v_int(0xFFFFFF));
            put("fontcolor", v_int(0));
        }
        "RDESIGNSURFACE" => {
            put("formcaption", v_str("Form1"));
            put("compcount", v_int(0));
            put("visible", v_bool(true));
        }
        "RSTRINGLIST" => {
            put("count", v_int(0));
            put("text", v_str(""));
        }
        "RTOOLBAR" => {
            put("width", v_int(0));
            put("height", v_int(32));
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
        "RPRINTER" => put("title", v_str("")),
        // (database.rs / database_web.rs keep the rest)
        "RSQLITE" => {
            put("connected", v_int(0));
            put("db", v_str(""));
            put("rowcount", v_int(0));
            put("colcount", v_int(0));
            put("fieldcount", v_int(0));
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
        "RLISTVIEW" => {
            put("itemindex", v_int(-1));
            put("items", v_str(""));
            put("count", v_int(0));
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
/// QFORM, QPANEL (QBEVEL) clBtnFace; for a QLABEL, QCANVAS, QGROUPBOX,
/// QBUTTON, QSCROLLBOX, QTABCONTROL, QGAUGE or QOVALBTN its parent's Color,
/// followed live (Delphi's ParentColor: `parent()` reads it, None without a
/// parent) and clWindow without one; for the other components RC.EXE was
/// asked about, clWindow. None too for a type
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
        "RLABEL" | "RCANVAS" | "RGROUPBOX" | "RBUTTON" | "RSCROLLBOX" | "RTABCONTROL" | "RPROGRESSBAR" | "ROVALBTN" => Some(parent().unwrap_or_else(|| v_int(CL_WINDOW))),
        "REDIT" | "RMEMO" | "RRICHEDIT" | "RLISTBOX" | "RCOMBOBOX" | "RSTRINGGRID" | "RDIRTREE" | "RFILELISTBOX" => Some(v_int(CL_WINDOW)),
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
            | "RSCROLLBAR" | "RPROGRESSBAR" | "RHEADER" | "RDXSCREEN" | "RCODEEDITOR" | "RDIFFVIEW" | "RFORM"
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
    matches!(type_name.to_ascii_uppercase().as_str(), "RLABEL" | "RCANVAS" | "RGROUPBOX" | "RBUTTON" | "RSCROLLBOX" | "RTABCONTROL" | "RPROGRESSBAR" | "ROVALBTN")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn get<'a>(p: &'a [(String, Value)], k: &str) -> Option<&'a Value> {
        p.iter().rev().find(|(n, _)| n == k).map(|(_, v)| v)
    }

    #[test]
    fn defaults_at_creation() {
        let f = creation("rform");
        // (RC.EXE: a new QFORM reads Left 0, Top 0, Visible 0, Enabled 1)
        assert_eq!(get(&f, "left"), Some(&v_int(0)));
        assert_eq!(get(&f, "visible"), Some(&v_bool(false)));
        assert_eq!(get(&f, "enabled"), Some(&v_bool(true)));
        assert!(get(&f, "color").is_none(), "an unset Color is color_read's");
        assert_eq!(get(&creation("RTIMER"), "interval"), Some(&v_int(1000)));
        assert_eq!(get(&creation("rfiledialog"), "filter"), Some(&v_str("All Files|*.*")));
        assert_eq!(get(&creation("RBUTTON"), "spacing"), Some(&v_int(4)));
        assert_eq!(get(&creation("RLISTBOX"), "copymode"), Some(&v_int(0xCC0020)));
        assert_eq!(get(&creation("RGLASSFRAME"), "color"), Some(&v_int(CL_BTN_FACE)));
        assert!(get(&creation("RCOLORDIALOG"), "colors(16)").is_some());
        assert!(creation("RUDT").is_empty());
        // (a gauge's BackColor: the theme's until set, RapidQ's white when read)
        assert!(get(&creation("RPROGRESSBAR"), "backcolor").is_none());
        assert_eq!(unset_read("RPROGRESSBAR", "BackColor"), Some(v_int(0xFFFFFF)));
        assert_eq!(unset_read("RBUTTON", "spacing"), None);
    }

    /// One place per default: a property the registry gives a default isn't
    /// in the hand list too.
    #[test]
    fn extras_never_repeat_the_registry() {
        let mut twice = Vec::new();
        for t in rapidr_lang::COMPONENT_TYPES {
            let reg = registry(t);
            for (k, v) in extras(t) {
                if let Some(r) = get(&reg, &k) {
                    twice.push(format!("{t}.{k}: registry {r:?}, extras {v:?}"));
                }
            }
        }
        assert!(twice.is_empty(), "{twice:#?}");
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
        // (RC.EXE: a button on a form follows the form's Color, one without a parent reads clWindow)
        assert_eq!(color_read("RBUTTON", &Value::Null, || Some(v_int(0xFF))), Some(v_int(0xFF)));
        assert_eq!(color_read("RBUTTON", &Value::Null, || None), Some(v_int(CL_WINDOW)));
        assert_eq!(color_read("RFORM", &v_int(0xFF), || None), None, "the program's");
        assert_eq!(color_read("RTIMER", &Value::Null, || None), None);
        assert!(get(&creation("RFORM"), "color").is_none());
    }
}
