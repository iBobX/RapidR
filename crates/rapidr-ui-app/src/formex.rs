//! RFormEx's members on both runtimes (`rapidr_value::formex` is the
//! model; docs/rformex-design.md): its properties as the program sets and
//! reads them, its methods, its tray icon (in `rapidr_value::tray` as
//! RapidR's own: the form's Handle, uID `TRAY_UID`, message `WM_TRAYICON`)
//! and its clicks as events, the drops on it as OnDrag, the window's look
//! (Transparent, TransparentClient, fsStayOnTop, RemoveTaskBar) as window
//! commands, OnMinimise, OnGetFocus / OnLostFocus, the wheel the classic
//! way and the messages OnMessage hears off Windows (WM_SIZE, WM_MOVE).
//! The runtimes create the form's TRAYICO (an RIMAGELIST `<form>.trayico`)
//! and its animation's timer (an RTIMER `<form>.__trayanim`) and route
//! here.

use rapidr_value::formex as fx;
use rapidr_value::{tray, v_int, v_str, Value};

use crate::windows::{push_op, WindowOp};
use crate::Program;

fn lower(s: &str) -> String {
    s.to_ascii_lowercase()
}

/// The form's TRAYICO (its id).
pub fn tray_list(form: &str) -> String {
    format!("{}.trayico", lower(form))
}

/// The timer that paces its tray animation (its id).
pub fn anim_timer(form: &str) -> String {
    format!("{}.__trayanim", lower(form))
}

/// The form a sub-component id of an RFormEx is on (`main.trayico` →
/// `main`), when it is one of [`tray_list`] / [`anim_timer`].
fn owner(id: &str, suffix: &str) -> Option<String> {
    let id = lower(id);
    let form = id.strip_suffix(suffix)?.to_string();
    fx::is(&form).then_some(form)
}

/// Whether `id` is an RFormEx's animation timer: one tick of it shows the
/// next picture (the runtimes' `timer_firing` asks: it fires no OnTimer).
pub fn tray_tick<P: Program>(p: P, id: &str) -> bool {
    let Some(form) = owner(id, ".__trayanim") else { return false };
    let count = count(p, &form);
    if fx::tray_shown(&form) && count > 1 {
        let frame = fx::next_frame(&form, count);
        show_tray(p, &form, Some(frame));
    }
    true
}

/// How many pictures its TRAYICO has.
fn count<P: Program>(p: P, form: &str) -> usize {
    usize::try_from(p.get(&tray_list(form), "count").to_i64()).unwrap_or(0)
}

/// TrayIconIndex as the tray uses it (0-based): the program's, from 1 (0:
/// the first).
fn index<P: Program>(p: P, form: &str) -> usize {
    usize::try_from(p.get(form, "trayiconindex").to_i64() - 1).unwrap_or(0)
}

/// The tooltip: TrayIconHint (the application's title until set: [`get`]).
fn hint<P: Program>(p: P, form: &str) -> String {
    p.get(form, "trayiconhint").to_string_val()
}

/// The form's icon in the tray as it is now: picture `frame` (else
/// TrayIconIndex's) of TRAYICO, TrayIconHint.
fn show_tray<P: Program>(p: P, form: &str, frame: Option<usize>) {
    let list = tray_list(form);
    let i = frame.unwrap_or_else(|| index(p, form));
    let reader = |id: &str, prop: &str| p.get(id, prop);
    let picture = match rapidr_value::objects::call(&list, "getico", &[v_int(i as i64)], &reader) {
        Some(Ok(v)) => v.to_string_val(),
        _ => String::new(),
    };
    let icon = (!picture.is_empty()).then(|| rapidr_value::handles::icon_handle(&picture));
    tray::put(tray::TrayIcon { hwnd: rapidr_value::handles::handle_of(form), uid: fx::TRAY_UID, message: Some(fx::WM_TRAYICON), icon, tip: Some(hint(p, form)) });
}

/// The icon's key in the tray.
fn tray_key(form: &str) -> (i64, i64) {
    (rapidr_value::handles::handle_of(form), fx::TRAY_UID)
}

/// The animation timer's pace: TrayIconUpdate (2000 ms until set), on while
/// the icon shows with two pictures or more.
fn animate<P: Program>(p: P, form: &str) {
    let timer = anim_timer(form);
    let ms = p.get(form, "trayiconupdate").to_i64();
    let on = fx::tray_shown(form) && count(p, form) > 1 && ms > 0;
    p.set(&timer, "interval", v_int(ms.max(1)));
    p.set(&timer, "enabled", v_int(i64::from(on)));
}

/// The window's look as the program set it: (opacity — `None` while
/// nothing was asked; its background a hole; on top; no taskbar button).
pub fn look<P: Program>(p: P, form: &str) -> (Option<u8>, bool, bool, bool) {
    if !fx::is(form) {
        return (None, false, false, false);
    }
    let alpha = match p.get(form, "transparent") {
        _ if !fx::alpha_active(form) => None,
        Value::Null => None,
        v => u8::try_from(v.to_i64()).ok().filter(|a| *a < 255),
    };
    (alpha, p.get(form, "transparentclient").to_bool(), fx::on_top(p.get(form, "formstyle").to_i64()), p.get(form, "__skiptaskbar").to_bool())
}

fn look_changed<P: Program>(p: P, form: &str) {
    push_op(WindowOp::Look(lower(form), look(p, form)));
}

/// A property of RFormEx `form` set by the program: `true` when it was one
/// of RFormEx's (stored here, and what follows done); `false`: the form's
/// usual set goes on.
pub fn set<P: Program>(p: P, form: &str, prop: &str, v: &Value) -> bool {
    if !fx::is(form) {
        return false;
    }
    match prop {
        fx::CLASSIC => fx::set_classic(form),
        "transparent" => {
            // (0 to 255; another value leaves it as it was, as the include's)
            if !fx::alpha_valid(v.to_i64()) {
                return true;
            }
            p.store(form, prop, v_int(v.to_i64()));
            fx::note_alpha_set(form);
            look_changed(p, form);
        }
        "transparentclient" | "formstyle" => {
            p.store(form, prop, v_int(v.to_i64()));
            look_changed(p, form);
        }
        "trayiconhint" => {
            p.store(form, prop, v_str(&v.to_string_val()));
            if fx::tray_shown(form) {
                show_tray(p, form, None);
            }
        }
        "trayiconindex" => set_index(p, form, v.to_i64()),
        "trayiconupdate" => {
            p.store(form, prop, v_int(v.to_i64()));
            animate(p, form);
        }
        "droppoint.x" | "droppoint.y" => {
            let (x, y) = fx::drop_point(form);
            fx::set_drop_point(form, if prop.ends_with('x') { (v.to_i64(), y) } else { (x, v.to_i64()) });
        }
        // (Color: the hole follows the colour the window is painted — the
        // host keys what it paints)
        _ => return false,
    }
    true
}

/// TrayIconIndex (and SetTrayIconIndex): one of TRAYICO's pictures, from 1;
/// past the last it is ignored, as the include's.
fn set_index<P: Program>(p: P, form: &str, i: i64) {
    if i > count(p, form) as i64 {
        return;
    }
    p.store(form, "trayiconindex", v_int(i));
    if fx::tray_shown(form) {
        show_tray(p, form, None);
    }
}

/// RFormEx's own properties (lower case): the ones [`get`] answers.
pub const PROPERTIES: &[&str] = &[
    "transparent",
    "trayiconhint",
    "trayiconupdate",
    "trayiconshown",
    "droppoint.x",
    "droppoint.y",
    "dragenable",
    "transparentclient",
    "formstyle",
    "trayiconindex",
    "dragzone.left",
    "dragzone.top",
    "dragzone.right",
    "dragzone.bottom",
];

/// Whether `prop` (lower case) is one of [`PROPERTIES`] (the quick test a
/// form's every read makes before the form is looked up).
pub fn reads(prop: &str) -> bool {
    PROPERTIES.contains(&prop)
}

/// An RFormEx property as the program reads it, when it is one of
/// RFormEx's (its value before the program set it included). `stored`
/// reads what the store has (never this again).
pub fn get<P: Program>(p: P, form: &str, prop: &str, stored: impl Fn(&str) -> Option<Value>) -> Option<Value> {
    if !reads(prop) || !fx::is(form) {
        return None;
    }
    Some(match prop {
        "transparent" => stored(prop).unwrap_or_else(|| v_int(fx::transparent_default(form))),
        "trayiconhint" => stored(prop).unwrap_or_else(|| v_str(&p.get("application", "title").to_string_val())),
        "trayiconupdate" => stored(prop).unwrap_or(v_int(2000)),
        "trayiconshown" => v_int(i64::from(fx::tray_shown(form))),
        "droppoint.x" => v_int(fx::drop_point(form).0),
        "droppoint.y" => v_int(fx::drop_point(form).1),
        "dragenable" | "transparentclient" | "formstyle" | "trayiconindex" | "dragzone.left" | "dragzone.top" | "dragzone.right" | "dragzone.bottom" => {
            stored(prop).unwrap_or(v_int(0))
        }
        _ => return None,
    })
}

/// An RFormEx method, when it is one of RFormEx's (its result).
pub fn call<P: Program>(p: P, form: &str, method: &str, args: &[Value]) -> Option<Value> {
    if !fx::is(form) {
        return None;
    }
    match method {
        "addtrayicon" => {
            if !fx::tray_shown(form) {
                fx::set_tray_shown(form, true);
                show_tray(p, form, None);
                animate(p, form);
            }
        }
        "deltrayicon" => remove_tray(p, form),
        "modifytrayicon" => {
            if fx::tray_shown(form) {
                show_tray(p, form, None);
            }
        }
        "settrayiconindex" => set_index(p, form, args.first().map_or(0, Value::to_i64)),
        "removetaskbar" => {
            p.store(form, "__skiptaskbar", v_int(1));
            look_changed(p, form);
        }
        // (the include's own OnShow / OnClose work is RFormEx's own)
        "inheritonshow" | "inheritonclose" => {}
        _ => return None,
    }
    Some(Value::Null)
}

/// Its tray icon taken away (DelTrayIcon, the form closed for good).
pub fn remove_tray<P: Program>(p: P, form: &str) {
    if fx::is(form) && fx::tray_shown(form) {
        fx::set_tray_shown(form, false);
        tray::remove(tray_key(form));
        animate(p, form);
    }
}

/// A sub-component member of an RFormEx: `TrayICO.AddICOFile …` is its
/// RIMAGELIST's — (that one's id, the member).
pub fn sub_component(form: &str, member: &str) -> Option<(String, String)> {
    if !fx::is(form) {
        return None;
    }
    let (sub, rest) = member.split_once('.')?;
    sub.eq_ignore_ascii_case("trayico").then(|| (tray_list(form), rest.to_string()))
}

/// The form's own tray icon's mouse message `mouse` (lParam): its events
/// (OnTrayClick, OnTrayDblClick, OnTrayMouseUp). Whether it was its own.
pub fn tray_message<P: Program>(p: P, form: &str, msg: i64, mouse: i64) -> bool {
    if !fx::is(form) || msg != fx::WM_TRAYICON {
        return false;
    }
    for (event, args) in fx::tray_events(mouse) {
        p.fire_args(form, event, &args);
    }
    true
}

/// Files dropped on RFormEx `form` at `at` (client pixels): with
/// DragEnable, inside DragZone, one OnDrag each, DropPoint set first.
pub fn dropped<P: Program>(p: P, form: &str, files: &[&str], at: (i64, i64)) {
    if !fx::is(form) || !p.get(form, "dragenable").to_bool() || !p.has_handler(form, "ondrag") {
        return;
    }
    let z = |s: &str| p.get(form, &format!("dragzone.{s}")).to_i64();
    if !fx::in_zone((z("left"), z("top"), z("right"), z("bottom")), fx::is_classic(form), at) {
        return;
    }
    fx::set_drop_point(form, at);
    for f in files {
        p.fire_args(form, "ondrag", &[v_str(f)]);
    }
}

/// The client area's top left on the screen (from Left / Top and the frame
/// RapidR accounts).
fn client_origin<P: Program>(p: P, form: &str) -> (i64, i64) {
    let style = rapidr_value::layout::frame_style(form, p.get(form, "borderstyle").to_i64());
    let (fw, fh) = rapidr_value::layout::form_frame(style);
    (p.get(form, "left").to_i64() + fw / 2, p.get(form, "top").to_i64() + fh - fw / 2)
}

/// OnMouseWheel's arguments for form `form` hearing a turn of `delta`
/// notches at (x, y) of its client area with Shift `shift`: RapidR's as they
/// are, a classic form's as the include gave them (`fx::classic_wheel`).
pub fn wheel_args<P: Program>(p: P, form: &str, delta: i64, (x, y): (i64, i64), shift: i64) -> [i64; 4] {
    if !fx::is_classic(form) {
        return [delta, x, y, shift];
    }
    let (ox, oy) = client_origin(p, form);
    fx::classic_wheel(delta, (ox + x, oy + y), shift)
}

/// The window became (`true`) or stopped being the active one, as the
/// system says (`Desktop::focused`'s, for a GUI test's script).
pub fn activated<P: Program>(p: P, form: &str, active: bool) {
    if fx::is(form) && fx::set_active(form, active) {
        p.fire(form, if active { "ongetfocus" } else { "onlostfocus" });
    }
}

/// (a GUI test's `__minimise`) The person minimised the window, as the
/// system says it: a classic form's on Windows hears OnMinimise first
/// (its SC_MINIMIZE held), every other once the window is minimised.
pub fn user_minimised<R: Program + crate::Windows>(rt: R, form: &str) {
    if cfg!(target_os = "windows") && fx::holds_minimise(form) {
        minimise_asked(rt, form);
        return;
    }
    rt.set(form, "windowstate", v_int(rapidr_value::window_state::WS_MINIMIZED));
    if fx::is(form) {
        rt.fire(form, "onminimise");
    }
}

/// (Windows, a classic form) The window's SC_MINIMIZE, held by the host:
/// OnMinimise first, then the window minimises (as the include did). An
/// RFormEx's — and a classic one's elsewhere — comes once the system says
/// the window is minimised (`Desktop::window_state`).
pub fn minimise_asked<P: Program>(p: P, form: &str) {
    p.fire(form, "onminimise");
    push_op(WindowOp::Minimize(lower(form)));
}

/// (off Windows) What OnMessage hears when the window was resized or moved
/// there: WM_SIZE / WM_MOVE as Windows numbers them.
pub fn resized<P: Program>(p: P, form: &str) {
    if cfg!(target_os = "windows") || !fx::is(form) || !p.has_handler(form, "onmessage") {
        return;
    }
    let state = p.get(form, "windowstate").to_i64();
    let [msg, w, l] = fx::wm_size(state, p.get(form, "clientwidth").to_i64(), p.get(form, "clientheight").to_i64());
    let hwnd = rapidr_value::handles::handle_of(form);
    p.fire_args(form, "onmessage", &[v_int(hwnd), v_int(msg), v_int(w), v_int(l)]);
}

/// (off Windows) WM_MOVE for OnMessage, the window moved.
pub fn moved<P: Program>(p: P, form: &str) {
    if cfg!(target_os = "windows") || !fx::is(form) || !p.has_handler(form, "onmessage") {
        return;
    }
    let (x, y) = client_origin(p, form);
    let [msg, w, l] = fx::wm_move(x, y);
    let hwnd = rapidr_value::handles::handle_of(form);
    p.fire_args(form, "onmessage", &[v_int(hwnd), v_int(msg), v_int(w), v_int(l)]);
}

/// A window message for an RFormEx's OnMessage (on Windows: the window's
/// own; elsewhere the program's own tray icons'): fired when it is one it
/// hears.
pub fn message<P: Program>(p: P, form: &str, m: [i64; 4]) {
    if fx::is(form) && fx::message_heard(m[1]) && p.has_handler(form, "onmessage") {
        p.fire_args(form, "onmessage", &m.map(v_int));
    }
}
