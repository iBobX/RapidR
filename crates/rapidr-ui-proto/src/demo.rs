//! The program the prototype runs — what this RapidQ source would make:
//!
//! ```basic
//! CREATE Form AS QFORM
//!     Caption = "RapidR UI kernel"
//!     CREATE lblName AS QLABEL : Caption = "Name:" : Left = 8 : Top = 12 : END CREATE
//!     CREATE edName AS QEDIT : Text = "Grüße, ñandú ✓" : Left = 56 : Top = 8 : END CREATE
//!     CREATE btnOK AS QBUTTON : Caption = "OK" : Left = 186 : Top = 8 : OnClick = OKClick : END CREATE
//!     CREATE tbLevel AS QTRACKBAR : Left = 8 : Top = 44 : Position = 3 : OnChange = LevelChange : END CREATE
//!     CREATE tcPages AS QTABCONTROL : Left = 164 : Top = 44 : OnChange = PageChange
//!         AddTabs "One", "Two", "Three"
//!         CREATE lblPage AS QLABEL : Caption = "Page One" : END CREATE
//!     END CREATE
//!     CREATE lblStatus AS QLABEL : Left = 8 : Top = 156 : Width = 300 : END CREATE
//! END CREATE
//! ```
//!
//! Its handlers write what happened in lblStatus, reading the shared models
//! as a program would (edName.Text, .SelStart, .SelLength, tbLevel.Position,
//! tcPages.TabIndex).

use rapidr_value::objects::tabcontrol::TabControl;
use rapidr_value::objects::trackbar::TrackBar;
use rapidr_value::{v_int, v_str};

use crate::form::{Clipboard, Component, Edit, Event, Form, Key, Kind, Mods};
use crate::text::TextSystem;

pub fn form() -> Form {
    let mut f = Form::new("RapidR UI kernel");
    f.add(Component::new("lblName", "RLABEL", 8, 12, Kind::Label { caption: "Name:".into() }));
    let mut name = Component::new("edName", "REDIT", 56, 8, Kind::Label { caption: String::new() });
    name.kind = Kind::Edit(Edit::new("Grüße, ñandú ✓", &name.font));
    f.add(name);
    f.add(Component::new("btnOK", "RBUTTON", 186, 8, Kind::Button { caption: "OK".into() }));
    let mut level = TrackBar::default();
    level.set("position", &v_int(3));
    f.add(Component::new("tbLevel", "RTRACKBAR", 8, 44, Kind::TrackBar(level)));
    let mut pages = TabControl::default();
    pages.call("addtabs", &[v_str("One"), v_str("Two"), v_str("Three")]);
    let tabs = Component::new("tcPages", "RTABCONTROL", 164, 44, Kind::TabControl(pages));
    // (the page's label in the area the components fill: TabControl::display)
    let (dx, dy, _, _) = match &tabs.kind {
        Kind::TabControl(t) => t.display(tabs.width, tabs.height, &tabs.font),
        _ => (0, 0, 0, 0),
    };
    let page = Component::new("lblPage", "RLABEL", tabs.left + dx + 4, tabs.top + dy + 4, Kind::Label { caption: "Page One".into() });
    f.add(tabs);
    f.add(page);
    let mut status = Component::new("lblStatus", "RLABEL", 8, 156, Kind::Label { caption: "Ready.".into() });
    status.width = 300;
    status.height = 48;
    f.add(status);
    f.set_focus(f.find("edName"));
    f
}

/// No clipboard (scripted input and tests).
pub struct NoClipboard(pub Option<String>);

impl Clipboard for NoClipboard {
    fn get_text(&mut self) -> Option<String> {
        self.0.clone()
    }
    fn set_text(&mut self, text: &str) {
        self.0 = Some(text.to_string());
    }
}

/// `--script`: what a user would do, through the same input calls the
/// window makes (logical pixels): click OK, drag the track bar's thumb,
/// click the second tab, click into the edit, End, type "!", select the
/// last two characters with Shift+Left. Headless evidence that input
/// reaches the models.
pub fn script(f: &mut Form, ts: &mut TextSystem, clicks: &mut u32) {
    let mut clip = NoClipboard(None);
    let none = Mods::default();
    let mut ev = Vec::new();
    // OK (186, 8, 75, 25)
    ev.extend(f.mouse_down(223.0, 20.0, none, ts));
    ev.extend(f.mouse_up(223.0, 20.0));
    // the thumb (at Position 3) dragged right
    ev.extend(f.mouse_down(58.0, 58.0, none, ts));
    for x in [70.0, 90.0, 108.0] {
        ev.extend(f.mouse_move(x, 58.0, ts).1);
    }
    ev.extend(f.mouse_up(108.0, 58.0));
    // the tab "Two"
    ev.extend(f.mouse_down(220.0, 55.0, none, ts));
    ev.extend(f.mouse_up(220.0, 55.0));
    handle(f, &ev, clicks);
    // into the edit, at its end, "!" typed, two characters selected
    let mut ev = f.mouse_down(150.0, 20.0, none, ts);
    ev.extend(f.mouse_up(150.0, 20.0));
    ev.extend(f.key(Key::End, none, None, ts, &mut clip));
    ev.extend(f.key(Key::Char('!'), none, Some("!"), ts, &mut clip));
    let shift = Mods { shift: true, ..none };
    for _ in 0..2 {
        ev.extend(f.key(Key::Left, shift, None, ts, &mut clip));
    }
    handle(f, &ev, clicks);
}

/// A line at the end of lblStatus (the last three kept).
pub fn log(f: &mut Form, line: &str) {
    let Some(i) = f.find("lblStatus") else { return };
    let mut lines: Vec<String> = match &f.components[i].kind {
        Kind::Label { caption } if caption != "Ready." => caption.lines().map(str::to_string).collect(),
        _ => Vec::new(),
    };
    lines.push(line.to_string());
    let keep = lines.len().saturating_sub(3);
    f.components[i].set_caption(&lines[keep..].join("\n"));
}

fn set_caption(f: &mut Form, name: &str, s: &str) {
    if let Some(i) = f.find(name) {
        f.components[i].set_caption(s);
    }
}

/// The handlers: OKClick, LevelChange, PageChange, and edName's OnChange.
pub fn handle(f: &mut Form, events: &[Event], clicks: &mut u32) {
    for ev in events {
        let (i, what) = match ev {
            Event::Click(i) => (*i, "OnClick"),
            Event::Change(i) => (*i, "OnChange"),
        };
        let name = f.components[i].name.clone();
        let detail = match &f.components[i].kind {
            Kind::Button { .. } => {
                *clicks += 1;
                format!("clicked {} time(s)", clicks)
            }
            Kind::TrackBar(t) => format!("Position = {}", t.get("position").map_or(0, |v| v.to_i64())),
            Kind::TabControl(t) => {
                let tab = t.index;
                let caption = usize::try_from(tab).ok().and_then(|k| t.tabs.get(k)).cloned().unwrap_or_default();
                set_caption(f, "lblPage", &format!("Page {caption}"));
                format!("TabIndex = {tab}")
            }
            Kind::Edit(e) => format!("Text = \"{}\", SelStart = {}", e.model.text(), e.model.sel_start),
            Kind::Label { .. } => String::new(),
        };
        log(f, &format!("{name}.{what}: {detail}"));
    }
}
