use super::*;
use crate::display::{DisplayList, Item};
use rapidr_value::objects::ops::Op;
use rapidr_value::theme::{CLASSIC, RAPIDR, RAPIDR_DARK, RAPIDR_HIGH_CONTRAST};

fn row(name: &str, kind: &str, text: &str) -> ValueRow {
    ValueRow::new(&ValueInfo { name, kind, text, ..Default::default() })
}

fn parts(r: &ValueRow) -> Vec<(&str, Part)> {
    r.parts().collect()
}

fn font() -> Font {
    Font { name: "Inter".into(), size: 9, color: 0, styles: 0 }
}

/// `r` painted in `theme`: the list.
fn painted(theme: &'static Theme, scale: f64, f: impl FnOnce(&mut Painter)) -> DisplayList {
    let mut list = DisplayList { size: (400, 40), scale, ..Default::default() };
    let mut p = Painter::new(&mut list);
    p.in_theme(theme, f);
    list
}

fn texts(list: &DisplayList) -> Vec<(String, u32)> {
    list.items
        .iter()
        .filter_map(|i| match i {
            Item::Op { op: Op::Text { text, color, .. }, .. } => Some((text.clone(), *color)),
            _ => None,
        })
        .collect()
}

fn images(list: &DisplayList) -> Vec<String> {
    list.items
        .iter()
        .filter_map(|i| match i {
            Item::Op { op: Op::Image { source, .. }, .. } => Some(source.clone()),
            _ => None,
        })
        .collect()
}

#[test]
fn kinds_from_the_debuggers_words() {
    let k = |kind: &str, text: &str| ValueKind::classify(kind, text, Hint::None);
    assert_eq!(k("Integer", "0"), ValueKind::Integer);
    assert_eq!(k("Double", "3.5"), ValueKind::Decimal);
    assert_eq!(k("f64", "3.5"), ValueKind::Decimal);
    assert_eq!(k("String", "\"Hi\""), ValueKind::Text);
    assert_eq!(k("str", "\"Hi\""), ValueKind::Text);
    assert_eq!(k("Boolean", "True"), ValueKind::Boolean);
    assert_eq!(k("Array", "Array(0 TO 9)"), ValueKind::Array);
    assert_eq!(k("Empty", ""), ValueKind::Empty);
    assert_eq!(k("i64", "(null)"), ValueKind::Integer);
    assert_eq!(k("Point", "(null)"), ValueKind::Empty);
    assert_eq!(k("Include", "(3 names)"), ValueKind::Include);
    assert_eq!(k("Components", "(2)"), ValueKind::Components);
    assert_eq!(k("Row", "1  |  2"), ValueKind::Row);
    assert_eq!(k("RListView", "\"Stats\" (RListView)"), ValueKind::Component);
    assert_eq!(k("RButton", "\"Button1\" (RButton)"), ValueKind::Component);
    // (a TYPE's value is its type's name; one named like no component)
    assert_eq!(k("Point", "Point"), ValueKind::Record);
    // What the registry knows wins: a colour, a handle, a constant.
    assert_eq!(ValueKind::classify("Integer", "-2147483640", Hint::Color), ValueKind::Color);
    assert_eq!(ValueKind::classify("Integer", "255", Hint::Color), ValueKind::Color);
    assert_eq!(ValueKind::classify("String", "\"red\"", Hint::Color), ValueKind::Text);
    assert_eq!(ValueKind::classify("Integer", "1234", Hint::Handle), ValueKind::Handle);
    assert_eq!(ValueKind::classify("Integer", "5", Hint::Constant("alClient")), ValueKind::Constant);
    // Every kind has an icon in the set.
    for kind in [
        ValueKind::Integer,
        ValueKind::Decimal,
        ValueKind::Text,
        ValueKind::Boolean,
        ValueKind::Array,
        ValueKind::Record,
        ValueKind::Component,
        ValueKind::Constant,
        ValueKind::Color,
        ValueKind::Handle,
        ValueKind::Empty,
        ValueKind::Row,
        ValueKind::Include,
        ValueKind::Components,
        ValueKind::More,
        ValueKind::Locals,
        ValueKind::Globals,
        ValueKind::Watch,
        ValueKind::CallStack,
        ValueKind::Breakpoint,
        ValueKind::Frame,
        ValueKind::CurrentFrame,
    ] {
        assert!(rapidr_icons::debug_kind(kind.icon_key()).is_some(), "{kind:?}");
    }
}

/// The rows Robert saw as plain text (2026-10-22), as they read now.
#[test]
fn the_plain_rows_styled() {
    let w = row("w", "Integer", "0");
    assert_eq!(parts(&w), [("w", Part::Name), (" = ", Part::Punctuation), ("0", Part::Number)]);
    assert_eq!(w.icon().map(|i| i.id), Some("debug/integer"));
    assert_eq!(w.accessible_name(), "w, whole number 0");

    let stats = row("Stats", "RListView", "\"Stats\" (RListView)");
    assert_eq!(stats.text(), "Stats = RListView");
    assert_eq!(stats.icon().map(|i| i.id), Some("components/listview"));
    assert_eq!(stats.accessible_name(), "Stats, RListView");
    let sender = row("Sender", "RButton", "\"Button1\" (RButton)");
    assert_eq!(parts(&sender), [("Sender", Part::Name), (" = ", Part::Punctuation), ("Button1", Part::Constant), (" · ", Part::Punctuation), ("RButton", Part::Type)]);
    assert_eq!(sender.accessible_name(), "Sender, RButton Button1");
    let df = row("df", "RDataFrame", "\"df\" (RDataFrame: 3 rows × 2 columns)");
    assert_eq!(parts(&df)[2..], [("RDataFrame", Part::Type), (": 3 rows × 2 columns", Part::Dim)]);

    let font_color = ValueRow::new(&ValueInfo { name: "FontColor", kind: "Integer", text: "-2147483640", hint: Hint::Color, ..Default::default() });
    assert_eq!(font_color.text(), "FontColor = clWindowText");
    assert_eq!(font_color.swatch(), Some(Swatch::System(8)));
    assert_eq!(font_color.accessible_name(), "FontColor, colour clWindowText");
    assert_eq!(font_color.icon().map(|i| i.id), Some("debug/integer"));
    let orange = ValueRow::new(&ValueInfo { name: "Color", kind: "Integer", text: &(0x0080FF).to_string(), hint: Hint::Color, ..Default::default() });
    assert_eq!(orange.text(), "Color = &H0080FF");
    assert_eq!(orange.accessible_name(), "Color, colour red 255, green 128, blue 0");
    let red = ValueRow::new(&ValueInfo { name: "Color", kind: "Integer", text: "255", hint: Hint::Color, ..Default::default() });
    assert_eq!(red.text(), "Color = clRed");
    // (not a colour after all: shown as the number it is)
    let odd = ValueRow::new(&ValueInfo { name: "Color", kind: "Integer", text: "-5", hint: Hint::Color, ..Default::default() });
    assert_eq!((odd.text(), odd.swatch()), ("Color = -5", None));

    let empty = row("Color", "Empty", "");
    assert_eq!(parts(&empty)[2], ("Empty", Part::Dim));
    assert_eq!(empty.icon().map(|i| i.id), Some("debug/empty"));
    assert_eq!(empty.accessible_name(), "Color, empty");

    let cols = ValueRow::new(&ValueInfo { name: "ColName", kind: "Array", text: "Array(0 TO 64)", count: 65, ..Default::default() });
    assert_eq!(cols.text(), "ColName = Array(0 TO 64) · 65 items");
    assert_eq!(cols.accessible_name(), "ColName, array 0 to 64, 65 items");
    let one = ValueRow::new(&ValueInfo { name: "a", kind: "Array", text: "Array(0 TO 0)", count: 1, ..Default::default() });
    assert!(one.text().ends_with("· 1 item"));

    let blank = row("AccessibleName", "String", "\"\"");
    assert_eq!(parts(&blank)[2], ("\"\"", Part::Dim));
    assert_eq!(blank.accessible_name(), "AccessibleName, empty text");
    let hi = row("Caption", "String", "\"Hi there\"");
    assert_eq!(parts(&hi)[2], ("\"Hi there\"", Part::String));
    assert_eq!(hi.accessible_name(), "Caption, text Hi there");

    let p = ValueRow::new(&ValueInfo { name: "p", kind: "Point", text: "Point", count: 2, ..Default::default() });
    assert_eq!((p.text(), p.icon().map(|i| i.id)), ("p = Point · 2 fields", Some("symbols/type")));
    let done = row("Done", "Boolean", "True");
    assert_eq!((parts(&done)[2], done.accessible_name()), (("True", Part::Keyword), "Done, True"));
    let align = ValueRow::new(&ValueInfo { name: "Align", kind: "Integer", text: "5", hint: Hint::Constant("alClient"), ..Default::default() });
    assert_eq!((align.text(), align.accessible_name()), ("Align = alClient (5)", "Align, constant alClient"));
    assert_eq!(align.icon().map(|i| i.id), Some("symbols/constant"));
    let hwnd = ValueRow::new(&ValueInfo { name: "Handle", kind: "Integer", text: "1234", hint: Hint::Handle, ..Default::default() });
    assert_eq!((hwnd.icon().map(|i| i.id), hwnd.accessible_name()), (Some("debug/handle"), "Handle, handle 1234"));
    let res = ValueRow::new(&ValueInfo { name: "Total", kind: "Double", text: "9.5", result: true, ..Default::default() });
    assert_eq!((res.icon().map(|i| i.id), res.accessible_name()), (Some("debug/result"), "Total, result, decimal number 9.5"));
    let more = row("… 40 more", "More", "");
    assert_eq!((more.text(), more.accessible_name()), ("… 40 more", "… 40 more"));

    let changed = ValueRow::new(&ValueInfo { name: "Lives", kind: "Integer", text: "2", changed: true, ..Default::default() });
    assert_eq!(changed.accessible_name(), "Lives, whole number 2, changed");

    let locals = ValueRow::section(ValueKind::Locals, "Locals");
    assert_eq!((locals.icon().map(|i| i.id), locals.accessible_name(), locals.description().as_str()), (Some("debug/locals"), "Locals", "locals"));
}

/// The value's parts in the theme's `debug.*` colours, its icon first;
/// selected, everything in the selection's ink; disabled, the theme's grey.
#[test]
fn painted_in_the_themes_tokens() {
    let f = font();
    let score = row("Score", "Integer", "42");
    let title = row("Title", "String", "\"Hi\"");
    for t in [&RAPIDR, &RAPIDR_DARK, &RAPIDR_HIGH_CONTRAST, &CLASSIC] {
        let c = ide_theme::values(t);
        let list = painted(t, 1.0, |p| {
            score.paint(p, (0, 0, 300, 18), &RowStyle::new(&f));
            title.paint(p, (0, 18, 300, 18), &RowStyle::new(&f));
        });
        assert_eq!(texts(&list), [("Score".into(), c.name), (" = ".into(), c.punctuation), ("42".into(), c.number), ("Title".into(), c.name), (" = ".into(), c.punctuation), ("\"Hi\"".into(), c.string)], "{}", t.name);
        let imgs = images(&list);
        assert_eq!(imgs.len(), 2);
        assert!(imgs[0].contains("debug/integer") && imgs[1].contains("debug/text"), "{imgs:?}");
        // (the icon is first: left of the text)
        let first = list.items.iter().position(|i| matches!(i, Item::Op { op: Op::Image { .. }, .. })).unwrap();
        let text_at = list.items.iter().position(|i| matches!(i, Item::Op { op: Op::Text { .. }, .. })).unwrap();
        assert!(first < text_at);
    }
    let selected = painted(&RAPIDR, 1.0, |p| {
        score.paint(p, (0, 0, 300, 18), &RowStyle { selected_ink: Some(0xFFFFFF), ..RowStyle::new(&f) });
    });
    assert!(texts(&selected).iter().all(|(_, c)| *c == 0xFFFFFF));
    let off = painted(&RAPIDR, 1.0, |p| {
        score.paint(p, (0, 0, 300, 18), &RowStyle { enabled: false, ..RowStyle::new(&f) });
    });
    assert!(texts(&off).iter().all(|(_, c)| *c == RAPIDR.gray_text));
}

/// A system colour's swatch is what it is in the theme (the program's when
/// given); a colour's frame shows on the window.
#[test]
fn swatches_are_the_themes_colours() {
    let f = font();
    let fc = ValueRow::new(&ValueInfo { name: "FontColor", kind: "Integer", text: "-2147483640", hint: Hint::Color, ..Default::default() });
    let fill_of = |list: &DisplayList| {
        list.items.iter().find_map(|i| match i {
            Item::Op { op: Op::Round { fill: Some(c), rect, .. }, .. } => Some((*c, *rect)),
            Item::Op { op: Op::Fill { color, rect }, .. } => Some((*color, *rect)),
            _ => None,
        })
    };
    for t in [&RAPIDR, &RAPIDR_DARK, &RAPIDR_HIGH_CONTRAST] {
        let list = painted(t, 1.0, |p| {
            fc.paint(p, (0, 0, 300, 18), &RowStyle::new(&f));
        });
        let (fill, rect) = fill_of(&list).expect("a swatch");
        assert_eq!(fill, t.text, "{}: clWindowText is the theme's text", t.name);
        assert_eq!((rect.2, rect.3), (12, 12));
        assert!(rapidr_value::theme::contrast(t.border_strong, t.window) >= 3.0);
    }
    // (the program runs in the classic look: its clWindowText is black)
    let list = painted(&RAPIDR_DARK, 1.0, |p| {
        fc.paint(p, (0, 0, 300, 18), &RowStyle { program_theme: Some(&CLASSIC), ..RowStyle::new(&f) });
    });
    assert_eq!(fill_of(&list).unwrap().0, 0x000000);
    let orange = ValueRow::new(&ValueInfo { name: "Color", kind: "Integer", text: &(0x0080FF).to_string(), hint: Hint::Color, ..Default::default() });
    let list = painted(&CLASSIC, 1.0, |p| {
        orange.paint(p, (0, 0, 300, 18), &RowStyle::new(&f));
    });
    assert_eq!(fill_of(&list).unwrap().0, 0xFF8000, "BGR &H0080FF is orange");
}

/// A changed value: highlighted (`debug.changedback` behind the value, not
/// the name; framed in high contrast), its colours kept, the changed mark
/// after it in `debug.changed`.
#[test]
fn a_changed_value_is_marked() {
    let f = font();
    let lives = ValueRow::new(&ValueInfo { name: "Lives", kind: "Integer", text: "2", changed: true, ..Default::default() });
    let c = ide_theme::values(&RAPIDR);
    let list = painted(&RAPIDR, 1.0, |p| {
        lives.paint(p, (0, 0, 300, 18), &RowStyle::new(&f));
    });
    assert_eq!(texts(&list), [("Lives".into(), c.name), (" = ".into(), c.punctuation), ("2".into(), c.number)]);
    let imgs = images(&list);
    assert!(imgs.len() == 2 && imgs[1].contains("glyphs/changed"), "{imgs:?}");
    // (its picture is the mark in the changed colour)
    assert!(imgs[1].contains(&i64::from(c.changed).to_string()), "{imgs:?}");
    let back = |list: &DisplayList| {
        list.items.iter().find_map(|i| match i {
            Item::Op { op: Op::Round { rect, fill, stroke, .. }, .. } => Some((*rect, *fill, *stroke)),
            Item::Op { op: Op::Fill { rect, color }, .. } => Some((*rect, Some(*color), None)),
            _ => None,
        })
    };
    let (rect, fill, _) = back(&list).expect("a highlight");
    assert_eq!(fill, Some(c.changed_back));
    // (from the value on: the name and ` = ` aren't highlighted)
    let name_w = text_size("Lives = ", &f).0;
    assert!(rect.0 >= 16 + GAP + name_w - 4, "{rect:?}");
    let hc = painted(&RAPIDR_HIGH_CONTRAST, 1.0, |p| {
        lives.paint(p, (0, 0, 300, 18), &RowStyle::new(&f));
    });
    assert_eq!(back(&hc).map(|b| (b.1, b.2)), Some((None, Some(ide_theme::values(&RAPIDR_HIGH_CONTRAST).changed))));
    // (selected: the selection says it; no highlight under it)
    let sel = painted(&RAPIDR, 1.0, |p| {
        lives.paint(p, (0, 0, 300, 18), &RowStyle { selected_ink: Some(0xFFFFFF), ..RowStyle::new(&f) });
    });
    assert!(back(&sel).is_none());
    let inc = ValueRow::new(&ValueInfo { name: "$INCLUDE RAPIDQ.INC", kind: "Include", text: "(412 names)", ..Default::default() });
    assert_eq!((inc.text(), inc.accessible_name()), ("$INCLUDE RAPIDQ.INC (412 names)", "$INCLUDE RAPIDQ.INC, 412 names"));
}

/// What `width` says is what `paint` draws; a section's title is bold.
#[test]
fn widths_and_sections() {
    let f = font();
    for r in [
        ValueRow::new(&ValueInfo { name: "FontColor", kind: "Integer", text: "-2147483640", hint: Hint::Color, changed: true, ..Default::default() }),
        row("ColName", "Array", "Array(0 TO 64)"),
        ValueRow::section(ValueKind::Globals, "Globals"),
    ] {
        let mut drawn = 0;
        painted(&RAPIDR, 2.0, |p| drawn = r.paint(p, (0, 0, 600, 18), &RowStyle::new(&f)));
        assert_eq!(drawn, r.width(&f, 16), "{}", r.text());
    }
    let g = ValueRow::section(ValueKind::Globals, "Globals");
    let list = painted(&RAPIDR, 1.0, |p| {
        g.paint(p, (0, 0, 300, 18), &RowStyle::new(&f));
    });
    let bold = list.items.iter().any(|i| matches!(i, Item::Op { op: Op::Text { font, .. }, .. } if font.styles & 1 == 1));
    assert!(bold);
    // (a narrow column: the parts past its right edge aren't drawn)
    let long = row("SomeRatherLongName", "String", "\"and a rather long value too\"");
    let list = painted(&RAPIDR, 1.0, |p| {
        long.paint(p, (0, 0, 60, 18), &RowStyle::new(&f));
    });
    assert!(texts(&list).len() < 3);
}

/// A tree node's value (SetItemValue) as a row: its hint read, a heading,
/// a breakpoint with its state's icon and words.
#[test]
fn rows_from_tree_nodes() {
    use rapidr_value::objects::tree::ItemValue;
    let item = |name: &str, kind: &str, value: &str, hint: &str| ItemValue { name: name.into(), kind: kind.into(), value: value.into(), count: 0, hint: hint.into() };
    let fc = ValueRow::from_item(&item("FontColor", "Integer", "-2147483640", "color"), false);
    assert_eq!((fc.text(), fc.swatch()), ("FontColor = clWindowText", Some(Swatch::System(8))));
    let ws = ValueRow::from_item(&item("WindowState", "Integer", "2", "const:wsMaximized"), true);
    assert_eq!((ws.text(), ws.accessible_name()), ("WindowState = wsMaximized (2)", "WindowState, constant wsMaximized, changed"));
    // (a True / False property: its number as the program reads it, on / off after it)
    let vis = ValueRow::from_item(&item("Visible", "Integer", "1", "bool"), false);
    assert_eq!((vis.text(), vis.accessible_name()), ("Visible = 1 · on", "Visible, 1, on"));
    let off = ValueRow::from_item(&item("KeyPreview", "Integer", "0", "bool"), false);
    assert_eq!(off.text(), "KeyPreview = 0 · off");
    let ws_all = ValueRow::from_item(&item("WindowState", "Integer", "2", "const:wsMaximized,choices:wsNormal|wsMinimized|wsMaximized"), false);
    assert_eq!(ws_all.text(), "WindowState = wsMaximized (2)");
    let h = ValueRow::from_item(&item("Handle", "Integer", "4242", "handle,readonly"), false);
    assert_eq!(h.kind(), ValueKind::Handle);
    let g = ValueRow::from_item(&item("Globals", "Globals", "", ""), false);
    assert_eq!((g.kind(), g.icon().map(|i| i.id)), (ValueKind::Globals, Some("debug/globals")));
    let c = ValueRow::from_item(&item("Components", "Components", "", ""), false);
    assert_eq!((c.kind(), c.text()), (ValueKind::Components, "Components"));
    let form_parts = ValueRow::from_item(&item("Components", "Components", "(3)", ""), false);
    assert_eq!(form_parts.text(), "Components (3)");
    let top = ValueRow::from_item(&item("UpdateScore   Main.rr:42", "CurrentFrame", "", ""), false);
    assert_eq!(top.icon().map(|i| i.id), Some("actions/execution-point"));
    for (state, icon, said) in [
        ("on", "actions/breakpoint", "Main.rr:20, breakpoint"),
        ("off", "actions/breakpoint-disabled", "Main.rr:20, breakpoint, off"),
        ("unverified", "actions/breakpoint-unverified", "Main.rr:20, breakpoint, not bound to code yet"),
        ("conditional", "actions/breakpoint-conditional", "Main.rr:20, conditional breakpoint"),
        ("hitcount", "actions/breakpoint-hit-count", "Main.rr:20, breakpoint with a hit count"),
        ("log", "actions/logpoint", "Main.rr:20, log point"),
    ] {
        let b = ValueRow::from_item(&item("Main.rr:20", "Breakpoint", "", state), false);
        assert_eq!((b.icon().map(|i| i.id), b.accessible_name()), (Some(icon), said));
    }
}

/// The value alone (a data tip's column): no icon, no name.
#[test]
fn the_value_alone() {
    let f = font();
    let r = ValueRow::new(&ValueInfo { name: "Lives", kind: "Integer", text: "2", changed: true, ..Default::default() });
    let mut drawn = 0;
    let list = painted(&RAPIDR, 1.0, |p| drawn = r.paint_value(p, (0, 0, 200, 18), &RowStyle::new(&f)));
    assert_eq!(texts(&list), [("2".into(), ide_theme::values(&RAPIDR).number)]);
    assert_eq!(drawn, r.value_width(&f, 16));
    assert!(images(&list).iter().all(|i| i.contains("glyphs/changed")));
}

/// Thousands of rows: built once; each frame draws only ops — one picture
/// per icon in the list however many rows show it, the same picture frame
/// after frame — and the rows' widths are measured once.
#[test]
fn thousands_of_rows_stay_cheap() {
    let f = font();
    let rows: Vec<ValueRow> = (0..5000)
        .map(|i| match i % 3 {
            0 => ValueRow::new(&ValueInfo { name: "a(1)", kind: "Integer", text: &i.to_string(), ..Default::default() }),
            1 => row("s", "String", "\"x\""),
            _ => ValueRow::new(&ValueInfo { name: "c", kind: "Integer", text: "255", hint: Hint::Color, changed: i % 2 == 0, ..Default::default() }),
        })
        .collect();
    let frame = || {
        painted(&RAPIDR, 2.0, |p| {
            for (k, r) in rows.iter().enumerate() {
                r.paint(p, (0, k as i64 * 18, 300, 18), &RowStyle::new(&f));
            }
        })
    };
    let start = std::time::Instant::now();
    let a = frame();
    let b = frame();
    // (integer, text, the changed mark: three pictures for 5000 rows)
    assert_eq!(a.images.len(), 3, "{:?}", a.images.keys().collect::<Vec<_>>());
    for (k, pic) in &a.images {
        assert!(std::sync::Arc::ptr_eq(pic, &b.images[k]), "{k} made again");
    }
    assert!(rows.iter().all(|r| r.widths.borrow().is_some()));
    // (a generous bound for a debug build on a busy machine: two frames of
    // 5000 rows, ~30,000 ops)
    assert!(start.elapsed() < std::time::Duration::from_secs(10), "{:?}", start.elapsed());
}
