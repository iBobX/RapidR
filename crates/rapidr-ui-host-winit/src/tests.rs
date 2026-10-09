//! The host without a window: forms in a `MemStore`, pumped headless,
//! captured by the CPU renderer, their accessibility trees diffed.

use std::time::Duration;

use rapidr_ui_kernel::{KernelEvent, MemClipboard, MemStore, Mods};
use rapidr_value::input::Button;
use rapidr_value::{v_int, v_str};

use crate::headless::HeadlessHost;
use crate::{a11y, capture, Desktop, Host, HostCmd, HostEvent, Source, WindowSpec};

fn store() -> MemStore {
    let mut s = MemStore::new();
    s.add("frm", "RFORM", None).set("frm", "caption", v_str("Host")).set("frm", "clientwidth", v_int(200)).set("frm", "clientheight", v_int(100));
    s.set("frm", "color", v_int(0x00FF00));
    s.add("btn", "RBUTTON", Some("frm")).set("btn", "caption", v_str("OK")).set("btn", "left", v_int(10)).set("btn", "top", v_int(10));
    s.add("tb", "RTRACKBAR", Some("frm")).set("tb", "left", v_int(10)).set("tb", "top", v_int(50)).set("tb", "width", v_int(150));
    s.add("dlg", "RFORM", None).set("dlg", "clientwidth", v_int(80)).set("dlg", "clientheight", v_int(40));
    s
}

fn desk(s: &MemStore) -> Desktop {
    let mut d = Desktop::new(Box::new(MemClipboard::default()));
    for f in ["frm", "dlg"] {
        d.ensure_form(s, f, false, WindowSpec { title: f.into(), size: (200, 100), position: None, border: true, icon: None, ..Default::default() });
    }
    d
}

/// (the input lane's) A status bar's size grip dragged: `Desktop` makes the
/// kernel's request a window command, and the headless host resizes the
/// window at once, as the user's drag of its border (OnResize after the pump).
#[test]
fn a_size_grips_drag_resizes_the_window_as_the_user() {
    let mut s = store();
    s.add("bar", "RSTATUSBAR", Some("frm")).set("bar", "align", v_int(2)).set("bar", "top", v_int(76)).set("bar", "width", v_int(200)).set("bar", "height", v_int(24));
    let mut d = Desktop::new(Box::new(MemClipboard::default()));
    d.ensure_form(&s, "frm", false, WindowSpec { title: "frm".into(), size: (200, 100), ..Default::default() });
    d.show("frm");
    let mut h = HeadlessHost::new(1.0);
    h.pump(Some(Duration::ZERO), &mut d, &s);
    // (over it, the sizing arrow; elsewhere on the bar, its own)
    d.mouse_move(&s, "frm", 195.0, 96.0, Mods::NONE, Source::Script);
    assert_eq!(crate::platform::cursor_at(&mut d, &s, "frm", (195.0, 96.0)), rapidr_value::input::Cursor::SizeNWSE);
    d.mouse_move(&s, "frm", 100.0, 90.0, Mods::NONE, Source::Script);
    assert_eq!(crate::platform::cursor_at(&mut d, &s, "frm", (100.0, 90.0)), rapidr_value::input::Cursor::Default);
    d.events.clear();
    d.mouse_down(&s, "frm", (195.0, 96.0), Button::Left, Mods::NONE, Source::Script);
    d.mouse_move(&s, "frm", 225.0, 116.0, Mods::NONE, Source::Script);
    assert_eq!(d.cmds, vec![HostCmd::Resize { form: "frm".into(), w: 230, h: 120 }]);
    assert!(d.events.is_empty(), "the press and drag are the grip's");
    h.pump(Some(Duration::ZERO), &mut d, &s);
    assert_eq!(d.form("frm").map(|f| f.spec.size), Some((230, 120)));
    assert_eq!(d.events, vec![HostEvent::Kernel("frm".into(), KernelEvent::Resized("frm".into(), 230, 120))]);
}

#[test]
fn a_headless_pump_runs_the_commands_and_sleeps() {
    let s = store();
    let mut d = desk(&s);
    let mut h = HeadlessHost::new(2.0);
    d.show("frm");
    assert_eq!(d.cmds, vec![HostCmd::Show("frm".into())]);
    let t = std::time::Instant::now();
    h.pump(Some(Duration::from_millis(20)), &mut d, &s);
    assert!(t.elapsed() >= Duration::from_millis(20));
    assert!(d.cmds.is_empty());
    assert_eq!(d.forms["frm"].scale, 2.0);
    assert_eq!(h.screen(), crate::HEADLESS_SCREEN);
    assert_eq!(d.stacking(), ["frm"]);
}

#[test]
fn window_states() {
    let s = store();
    let mut d = desk(&s);
    let mut h = HeadlessHost::new(1.0);
    // the program's state: the headless host takes it as it is told
    d.form("frm").unwrap().spec.state = 2;
    d.show("frm");
    h.pump(Some(Duration::ZERO), &mut d, &s);
    assert_eq!(d.forms["frm"].state, 2);
    d.form("frm").unwrap().spec.state = 1;
    d.cmds.push(HostCmd::State("frm".into()));
    h.pump(Some(Duration::ZERO), &mut d, &s);
    assert_eq!(d.forms["frm"].state, 1);
    assert!(d.events.is_empty());
    // the system's word (the user restored it): the program hears it as
    // its WindowState set, once
    d.window_state("frm", 0);
    d.window_state("frm", 0);
    assert_eq!(d.events, vec![HostEvent::Kernel("frm".into(), KernelEvent::Set { id: "frm".into(), prop: "windowstate".into(), value: 0 })]);
    assert_eq!(d.forms["frm"].spec.state, 0);
}

#[test]
fn captures_are_device_pixels_on_the_cpu() {
    // (RapidQ's look: the button's Windows face; RapidR's own look is the
    // default since L-THEME)
    rapidr_value::theme::set(&rapidr_value::theme::CLASSIC);
    let s = store();
    let mut d = desk(&s);
    for (scale, w, h) in [(1.0, 200, 100), (2.0, 400, 200)] {
        d.form("frm").unwrap().scale = scale;
        let px = capture(&mut d, &s, "frm").unwrap();
        assert_eq!((px.width, px.height), (w, h));
        // the form's Color (&H00FF00 is green in RapidQ's BGR)
        assert_eq!(px.pixels[(px.height - 1) * px.width + px.width - 1], 0x00FF00);
        // the button's face (Windows' 3D face, &HF0F0F0) inside it
        let (x, y) = ((20.0 * scale) as usize, (20.0 * scale) as usize);
        assert_eq!(px.pixels[y * px.width + x], 0xF0F0F0);
    }
    assert!(capture(&mut d, &s, "nothing").is_none());
}

#[test]
fn input_reaches_the_kernel_and_modality_drops_the_users() {
    let s = store();
    let mut d = desk(&s);
    d.show("frm");
    d.show("dlg");
    // a click on the button
    d.mouse_down(&s, "frm", (20.0, 20.0), Button::Left, Mods::NONE, Source::User);
    d.mouse_up(&s, "frm", (20.0, 20.0), Button::Left, Mods::NONE, Source::User);
    assert!(d.events.contains(&HostEvent::Kernel("frm".into(), KernelEvent::Click("btn".into()))));
    d.events.clear();
    // the dialog modal: the user's clicks on the form go nowhere, a test's do
    d.modal.push("dlg".into());
    d.mouse_down(&s, "frm", (20.0, 20.0), Button::Left, Mods::NONE, Source::User);
    assert!(d.events.is_empty());
    d.close_box("frm", Source::User);
    assert!(d.events.is_empty());
    d.key_down(&s, "frm", 39, "", Mods::NONE, Source::Script);
    assert!(matches!(&d.events[0], HostEvent::Kernel(f, KernelEvent::KeyDown { vk: 39, .. }) if f == "frm"));
    // under a test, the user's input is dropped everywhere
    d.events.clear();
    d.modal.clear();
    d.ignore_user = true;
    d.close_box("frm", Source::User);
    assert!(d.events.is_empty());
    d.close_box("frm", Source::Script);
    assert_eq!(d.events, vec![HostEvent::Kernel("frm".into(), KernelEvent::Close("frm".into()))]);
}

#[test]
fn a_resize_is_laid_out_and_told_once() {
    let s = store();
    let mut d = desk(&s);
    d.resized("frm", 300, 150);
    assert_eq!(d.events, vec![HostEvent::Kernel("frm".into(), KernelEvent::Resized("frm".into(), 300, 150))]);
    assert_eq!(d.forms["frm"].spec.size, (300, 150));
    d.events.clear();
    d.resized("frm", 300, 150);
    assert!(d.events.is_empty());
    d.moved("frm", 40, 50);
    d.moved("frm", 40, 50);
    assert_eq!(d.events, vec![HostEvent::Kernel("frm".into(), KernelEvent::Moved("frm".into(), 40, 50))]);
}

#[test]
fn accessibility_updates_send_what_changed() {
    let s = store();
    let mut d = desk(&s);
    let Desktop { forms, text, .. } = &mut d;
    let f = forms.get_mut("frm").unwrap();
    let mut sent = a11y::Sent::default();
    let tree = f.ui.access_tree(&s, text);
    let first = a11y::update(&tree, 2.0, &mut sent, false).unwrap();
    // the window, the button, the track bar
    assert_eq!(first.nodes.len(), 3);
    assert!(first.tree.is_some());
    assert!(a11y::update(&tree, 2.0, &mut sent, false).is_none());
    // the track bar moved: only its node
    rapidr_value::objects::with_trackbar_mut("tb", |t| t.key(39));
    let tree = f.ui.access_tree(&s, text);
    let next = a11y::update(&tree, 2.0, &mut sent, false).unwrap();
    assert_eq!(next.nodes.len(), 1);
    assert_eq!(next.nodes[0].0 .0, rapidr_value::objects::a11y::node_id("tb"));
    assert!(next.tree.is_none());
    // asked again for all of it
    assert_eq!(a11y::update(&tree, 2.0, &mut sent, true).unwrap().nodes.len(), 3);
}

#[test]
fn a_synthetic_italic_leans_forward_on_the_cpu() {
    // (Liberation has no italic faces: parley skews the upright one)
    let mut s = MemStore::new();
    s.add("fi", "RFORM", None).set("fi", "clientwidth", v_int(60)).set("fi", "clientheight", v_int(40));
    s.add("li", "RLABEL", Some("fi")).set("li", "caption", v_str("I")).set("li", "left", v_int(4)).set("li", "top", v_int(4)).set("li", "fontsize", v_int(20)).set("li", "fontitalic", v_int(1));
    let mut d = Desktop::new(Box::new(MemClipboard::default()));
    d.ensure_form(&s, "fi", false, WindowSpec { title: "fi".into(), size: (60, 40), position: None, border: true, icon: None, ..Default::default() });
    let px = capture(&mut d, &s, "fi").unwrap();
    let ink_rows: Vec<usize> = (0..px.height).filter(|y| (0..px.width).any(|x| px.pixels[y * px.width + x] & 0xFF < 0x40)).collect();
    let left = |y: usize| (0..px.width).find(|x| px.pixels[y * px.width + x] & 0xFF < 0x40).unwrap();
    let (top, bottom) = (ink_rows[1], ink_rows[ink_rows.len() - 2]);
    assert!(left(top) > left(bottom), "the top of the I right of its foot: {} vs {}", left(top), left(bottom));
}
