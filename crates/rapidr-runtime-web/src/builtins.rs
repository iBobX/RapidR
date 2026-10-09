//! BASIC builtin functions — web/WASM implementations.
//!
//! Pure-Rust functions (math, string) are identical to the desktop runtime.
//! Platform-specific functions (PRINT, INPUT, SHELL, SLEEP, BEEP, SOUND, etc.)
//! are replaced with web-compatible equivalents using web-sys / js-sys.

use crate::value::{v_bool, v_dbl, v_int, v_null, v_str, Value};
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;

pub use crate::value::builtins::*;

// ---------------------------------------------------------------------------
// PRINT — outputs to the browser console and to a #rr-console element if present
// ---------------------------------------------------------------------------

/// Hands the exact printed text (partial lines and ANSI sequences
/// included) to `window.__rapidr_print` if the page defines it; exported web
/// bundles use it to show a console on the page (rapidr-webbundle's ansi_screen.js).
fn print_hook(text: &str) {
    let hook = web_sys::window()
        .and_then(|w| js_sys::Reflect::get(&w, &JsValue::from_str("__rapidr_print")).ok())
        .and_then(|f| f.dyn_into::<js_sys::Function>().ok());
    if let Some(f) = hook {
        let _ = f.call1(&JsValue::NULL, &JsValue::from_str(text));
    }
}

/// `__dll_call(lib, alias, spec, args…)` on the web: no DLL can be
/// loaded (docs/windows-dll-calls.md §1).
pub fn rp_dll_call(args: &[Value]) -> Value {
    let s = |i: usize| args.get(i).map(Value::to_string_val).unwrap_or_default();
    crate::value::runtime_error(&crate::value::dll::needs_windows_error(&s(0), &s(1), true))
}

/// `POKE [#page,] address, byte`: a change on the screen page is printed.
pub fn rp_poke(args: &[Value]) -> Value {
    let text = crate::value::memory::rp_poke_text(args);
    if !text.is_empty() {
        rp_print(&[Value::String(text)], false);
    }
    Value::Null
}

/// `PCOPY from, to`: a copy onto the screen page is printed.
pub fn rp_pcopy(from: &Value, to: &Value) -> Value {
    let text = crate::value::memory::rp_pcopy_text(from, to);
    if !text.is_empty() {
        rp_print(&[Value::String(text)], false);
    }
    Value::Null
}

pub fn rp_print(items: &[Value], newline: bool) {
    let mut parts = Vec::new();
    for item in items {
        parts.push(crate::value::format::print_text(item));
    }
    let text = parts.join(" ");
    let msg = if newline {
        format!("{}\n", text)
    } else {
        text
    };
    track_print_column(&msg);
    // (in RapidQ's code page when the program's file is RapidQ's: console::shown)
    let msg = crate::value::console::shown(&msg).into_owned();
    print_hook(&msg);

    // console.log is line-oriented: log complete lines, keep a partial one
    // (`PRINT "a";`) buffered until its newline, or until the current
    // synchronous code finishes (see schedule_output_flush).
    let complete: Vec<String> = LINE_BUF.with(|b| {
        let mut b = b.borrow_mut();
        b.push_str(&msg);
        let mut lines = Vec::new();
        while let Some(i) = b.find('\n') {
            let line: String = b.drain(..=i).collect();
            lines.push(line[..line.len() - 1].to_string());
        }
        lines
    });
    for line in complete {
        web_sys::console::log_1(&JsValue::from_str(&line));
    }
    if LINE_BUF.with(|b| !b.borrow().is_empty()) {
        schedule_output_flush();
    }

    // Also append to #rr-console element if it exists
    if let Some(window) = web_sys::window() {
        if let Some(document) = window.document() {
            if let Some(el) = document.get_element_by_id("rr-console") {
                // Append as text nodes (never markup) so printed data can't
                // inject HTML; spaces/newlines render as before.
                for (i, line) in msg.split('\n').enumerate() {
                    if i > 0 {
                        if let Ok(br) = document.create_element("br") {
                            let _ = el.append_child(&br);
                        }
                    }
                    if !line.is_empty() {
                        let text = document.create_text_node(&line.replace(' ', "\u{00A0}"));
                        let _ = el.append_child(&text);
                    }
                }
            }
        }
    }
}

/// Width of a PRINT zone (`PRINT a, b`), as in QBasic and VB.
pub const PRINT_ZONE_WIDTH: usize = 14;

thread_local! {
    /// Output cursor column (chars since the last newline), for print zones.
    static PRINT_COL: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
    /// Text printed since the last newline, not yet sent to console.log.
    static LINE_BUF: std::cell::RefCell<String> = const { std::cell::RefCell::new(String::new()) };
    static FLUSH_SCHEDULED: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

fn track_print_column(text: &str) {
    // The shared console cursor (CSRLIN/POS) ignores escape sequences.
    crate::value::console::track(text);
    PRINT_COL.with(|c| c.set(crate::value::console::column()));
}

/// `,` in PRINT: pad with spaces to the next print zone.
pub fn rp_print_zone() {
    let pad = PRINT_ZONE_WIDTH - PRINT_COL.with(|c| c.get()) % PRINT_ZONE_WIDTH;
    rp_print(&[Value::String(" ".repeat(pad))], false);
}

/// Send any buffered partial line to console.log.
pub fn rp_flush_output() {
    let pending = LINE_BUF.with(|b| std::mem::take(&mut *b.borrow_mut()));
    if !pending.is_empty() {
        web_sys::console::log_1(&JsValue::from_str(&pending));
    }
}

/// Flush a partial line once the current synchronous work (the program's
/// main, or an event handler) has finished.
///
/// A partial line stays buffered while the program merely yields (the VM's
/// time slices: dialog_web): it isn't over, and the line isn't either; the
/// VM host calls this again once the program stops or waits.
pub fn schedule_output_flush() {
    if LINE_BUF.with(|b| b.borrow().is_empty()) || FLUSH_SCHEDULED.with(|f| f.replace(true)) {
        return;
    }
    let flush = Closure::once_into_js(|| {
        FLUSH_SCHEDULED.with(|f| f.set(false));
        if !crate::dialog_web::is_yielded() {
            rp_flush_output();
        }
    });
    if let Some(window) = web_sys::window() {
        let _ = window.set_timeout_with_callback_and_timeout_and_arguments_0(flush.unchecked_ref(), 0);
    }
}

// ---------------------------------------------------------------------------
// INPUT — uses window.prompt() on web
// ---------------------------------------------------------------------------

pub fn rp_input(prompt: &Value) -> Value {
    let p = prompt.to_string_val();
    if crate::dialog_web::can_wait() {
        open_input(&p, false);
        return Value::String(String::new());
    }
    if let Some(window) = web_sys::window() {
        match window.prompt_with_message(&p) {
            Ok(Some(s)) => Value::String(s),
            _ => Value::String(String::new()),
        }
    } else {
        Value::String(String::new())
    }
}

/// The INPUT statement: an in-page field (the VM waits for it) showing the
/// prompt already printed on the current line; the browser's prompt()
/// elsewhere.
pub fn rp_input_line() -> Value {
    let prompt = LINE_BUF.with(|b| b.borrow().clone());
    if crate::dialog_web::can_wait() {
        open_input(&prompt, true);
        return Value::String(String::new());
    }
    rp_input(&v_str(&prompt))
}

/// INPUT's question in the UI kernel's input box (a wait the VM serves).
fn open_input(prompt: &str, echo: bool) {
    let text = if prompt.trim().is_empty() { "Enter a value:" } else { prompt };
    let _ = crate::kernel_web::input_box(&app_title(), text, echo);
}

// ---------------------------------------------------------------------------
// String functions — identical to desktop (pure Rust)
// ---------------------------------------------------------------------------
















// ---------------------------------------------------------------------------
// Numeric / conversion functions — identical to desktop (pure Rust)
// ---------------------------------------------------------------------------
























// ---------------------------------------------------------------------------
// Additional math / conversion (Phase 3d) — pure Rust, identical to desktop
// ---------------------------------------------------------------------------












// ---------------------------------------------------------------------------
// Date / Time — use js_sys::Date on WASM
// ---------------------------------------------------------------------------

pub fn rp_date() -> Value {
    let d = js_sys::Date::new_0();
    let m = d.get_month() + 1; // JS months are 0-based
    let day = d.get_date();
    let y = d.get_full_year();
    Value::String(format!("{:02}-{:02}-{:04}", m, day, y))
}

pub fn rp_time() -> Value {
    let d = js_sys::Date::new_0();
    let h = d.get_hours();
    let m = d.get_minutes();
    let s = d.get_seconds();
    Value::String(format!("{:02}:{:02}:{:02}", h, m, s))
}

// ---------------------------------------------------------------------------
// Array helpers — identical to desktop
// ---------------------------------------------------------------------------







// ---------------------------------------------------------------------------
// Timer — uses performance.now() on web
// ---------------------------------------------------------------------------

/// TIMER: the seconds since (local) midnight, as RapidQ's on Windows and the
/// desktop runtimes.
pub fn rp_timer() -> Value {
    let d = js_sys::Date::new_0();
    v_dbl(d.get_hours() as f64 * 3600.0 + d.get_minutes() as f64 * 60.0 + d.get_seconds() as f64 + d.get_milliseconds() as f64 / 1000.0)
}

// ---------------------------------------------------------------------------
// INKEY$ — the keys pressed in the page (not typed into a field)
// ---------------------------------------------------------------------------

thread_local! {
    static KEYS_TRACKED: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

/// `INKEY$`: the next key pressed in the page, or "" — keys typed into the
/// program's fields and dialogs are theirs, not INKEY$'s.
pub fn rp_inkey() -> Value {
    track_keys();
    rapidr_value::console::inkey()
}

/// `GET$(n)`: a page has no standard input.
pub fn rp_get_stdin(_n: &Value) -> Value {
    v_str("")
}

/// `SETCONSOLETITLE title`: the page's title.
pub fn rp_set_console_title(title: &Value) -> Value {
    if let Some(doc) = web_sys::window().and_then(|w| w.document()) {
        doc.set_title(&title.to_string_val());
    }
    v_null()
}

/// `CHDRIVE`: no drives in a page.
pub fn rp_chdrive(_drive: &Value) -> Value {
    v_null()
}

/// INPUT$(n)'s wait (the parser's RAPIDR__INPUTCHARS): the program sleeps
/// until a key is pressed in the page. 0 when it can't wait here.
pub fn rp_waitkey() -> Value {
    // (a window shown: the desktop's — a wait the VM serves, the windows'
    // keys INKEY$'s)
    if let Some(v) = crate::kernel_web::wait_key() {
        return v;
    }
    track_keys();
    if rapidr_value::console::key_waiting() {
        return v_int(1);
    }
    v_int(crate::dialog_web::wait_key() as i64)
}

/// The page's keys go to INKEY$'s queue from the first INKEY$ / INPUT$ on.
fn track_keys() {
    if !KEYS_TRACKED.with(|t| t.replace(true)) {
        let cb = Closure::<dyn FnMut(web_sys::KeyboardEvent)>::new(|e: web_sys::KeyboardEvent| {
            let typing = e.target().and_then(|t| t.dyn_into::<web_sys::HtmlElement>().ok()).is_some_and(|t| {
                matches!(t.tag_name().as_str(), "INPUT" | "TEXTAREA" | "SELECT") || t.is_content_editable()
            });
            if typing {
                return;
            }
            let Some(vk) = rapidr_value::input::vk_of_key(&e.key(), &e.code()) else { return };
            let text = if e.key().chars().count() == 1 { e.key() } else { String::new() };
            if let Some(k) = rapidr_value::console::inkey_of(vk, &text) {
                rapidr_value::console::push_key(k);
                crate::dialog_web::key_pressed();
            }
        });
        if let Some(doc) = web_sys::window().and_then(|w| w.document()) {
            let _ = doc.add_event_listener_with_callback("keydown", cb.as_ref().unchecked_ref());
        }
        cb.forget();
    }
}

// ---------------------------------------------------------------------------
// Sleep — NOT SUPPORTED on web (WASM is single-threaded)
// ---------------------------------------------------------------------------

pub fn rp_sleep(_ms: &Value) {
    web_sys::console::warn_1(&JsValue::from_str(
        "[WARN] SLEEP is not supported in WASM — use timers instead",
    ));
}

// ---------------------------------------------------------------------------
// System / Shell — NOT SUPPORTED on web
// ---------------------------------------------------------------------------

/// The page's program and arguments: its path (as Application.ExeName) and
/// its query string's parts (`?a&b%20c`: "a", "b c") — the web's command
/// line (rapidr_value::command_line). A page that runs the program as a
/// part of itself gives its arguments instead, as `window.RAPIDR_ARGS` (an
/// array of strings): the web IDE's preview, whose own query string is
/// the IDE's.
fn page_command_line() -> (String, Vec<String>) {
    let window = web_sys::window();
    let loc = window.as_ref().map(|w| w.location());
    let path = loc.as_ref().and_then(|l| l.pathname().ok()).unwrap_or_default();
    let given = window
        .as_ref()
        .and_then(|w| js_sys::Reflect::get(w, &"RAPIDR_ARGS".into()).ok())
        .filter(js_sys::Array::is_array)
        .map(|a| js_sys::Array::from(&a).iter().map(|v| v.as_string().unwrap_or_default()).collect());
    let args = given.unwrap_or_else(|| {
        let query = loc.as_ref().and_then(|l| l.search().ok()).unwrap_or_default();
        crate::value::command_line::query_args(&query)
    });
    (path, args)
}

/// A bare `COMMAND$` (RapidR's): the page's arguments, joined.
pub fn rp_command() -> Value {
    crate::value::command_line::command_line(&page_command_line().1)
}

/// `COMMAND$(n)`: 0 the page, 1… its arguments.
pub fn rp_command_arg(n: &Value) -> Value {
    let (path, args) = page_command_line();
    crate::value::command_line::command_arg(&path, &args, n)
}

/// `CommandCount`.
pub fn rp_commandcount() -> Value {
    crate::value::command_line::command_count(&page_command_line().1)
}

/// `ENVIRON$(name)`: the page's own table (rapidr_value::environ) — empty
/// until the program's ENVIRON statements set something.
pub fn rp_environ(name: &Value) -> Value {
    let v = rp_environ_get(name);
    // (a test's: the page's RAPIDR_TEST_ENV object — e.g. RAPIDR_TEST_HTTP,
    // the GUI tests' own HTTP server)
    if v.to_string_val().is_empty() {
        let test = web_sys::window().and_then(|w| js_sys::Reflect::get(&w, &"RAPIDR_TEST_ENV".into()).ok()).filter(|o| o.is_object());
        if let Some(s) = test.and_then(|o| js_sys::Reflect::get(&o, &name.to_string_val().into()).ok()).and_then(|v| v.as_string()) {
            return Value::String(s);
        }
    }
    v
}


/// What END throws to leave the compiled program's Rust code (the page
/// ignores it: `object_web::end_program` has already ended the program).
pub const END_UNWIND: &str = "[RapidR] END";

/// END in a compiled web program: ends it (`object_web::end_program`), then
/// leaves the code running — wasm has no process to exit — by throwing
/// [`END_UNWIND`], which the page swallows. (The interpreter halts its VM
/// instead, calling `end_program` itself.)
pub fn rp_end() {
    crate::object_web::end_program();
    swallow_end_unwind();
    wasm_bindgen::throw_str(END_UNWIND);
}

/// Stops the browser reporting [`END_UNWIND`] as an uncaught error.
fn swallow_end_unwind() {
    let Some(window) = web_sys::window() else { return };
    let filter = Closure::<dyn FnMut(web_sys::ErrorEvent)>::new(|e: web_sys::ErrorEvent| {
        if e.message().contains(END_UNWIND) {
            e.prevent_default();
        }
    });
    let _ = window.add_event_listener_with_callback("error", filter.as_ref().unchecked_ref());
    filter.forget();
}

pub fn rp_showmessage(msg: &Value) {
    // (the desktop's: the kernel's box titled Application.Title — under a
    // GUI test without a message hook, printed, the program going on)
    if crate::kernel_web::dialogs_here() {
        let text = msg.to_string_val();
        if rapidr_ui_app::testhooks::under_test() && !rapidr_ui_app::testhooks::message_hook() {
            web_sys::console::log_1(&JsValue::from_str(&format!("[SHOWMESSAGE] {text}")));
            return;
        }
        let title = app_title();
        if crate::kernel_web::choice(&title, &text, &["OK"], None, false, |_| v_null()).is_some() {
            return;
        }
    }
    // (a native web build can't wait for the kernel's box: the browser's)
    if let Some(window) = web_sys::window() {
        let _ = window.alert_with_message(&msg.to_string_val());
    }
}

pub fn rp_msgbox(msg: &Value) -> Value {
    // (the kernel's box with OK, as the desktop's MSGBOX; 0)
    if let Some(v) = crate::kernel_web::choice(&app_title(), &msg.to_string_val(), &["OK"], None, false, |_| v_int(0)) {
        return v;
    }
    rp_showmessage(msg);
    v_int(0)
}

/// RUN: a browser starts no programs.
pub fn rp_run(_command: &Value) -> Value {
    web_sys::console::warn_1(&JsValue::from_str("[WARN] RUN is not supported in the browser"));
    v_int(0)
}

pub fn rp_shell(_command: &Value) -> Value {
    web_sys::console::warn_1(&JsValue::from_str(
        "[WARN] SHELL is not supported in WASM",
    ));
    v_int(-1)
}

pub fn rp_shellwait(_command: &Value) -> Value {
    web_sys::console::warn_1(&JsValue::from_str(
        "[WARN] SHELLWAIT is not supported in WASM",
    ));
    v_int(-1)
}

pub fn rp_beep() {
    play_tone(800.0, 200.0);
}

pub fn rp_sound(freq: &Value, duration: &Value) {
    play_tone(freq.to_f64(), duration.to_f64());
}

thread_local! {
    // Browsers cap the number of live AudioContexts, so share one.
    static AUDIO_CTX: std::cell::RefCell<Option<web_sys::AudioContext>> =
        const { std::cell::RefCell::new(None) };
}

/// The page's AudioContext (made the first time; shared: BEEP, SOUND,
/// QDXSOUND).
pub(crate) fn audio_context() -> Option<web_sys::AudioContext> {
    AUDIO_CTX.with(|cell| {
        let mut slot = cell.borrow_mut();
        if slot.is_none() {
            *slot = web_sys::AudioContext::new().ok();
        }
        slot.clone()
    })
}

/// Plays a sine tone via Web Audio. Failures (no audio support, autoplay
/// policy) are silently ignored, matching desktop BEEP semantics.
fn play_tone(freq_hz: f64, dur_ms: f64) {
    let Some(ctx) = audio_context() else { return };
    let Ok(osc) = ctx.create_oscillator() else { return };
    osc.frequency().set_value(freq_hz as f32);
    if osc.connect_with_audio_node(&ctx.destination()).is_err() {
        return;
    }
    let now = ctx.current_time();
    let _ = osc.start_with_when(now);
    let _ = osc.stop_with_when(now + dur_ms.max(0.0) / 1000.0);
}

pub fn rp_playsound(filename: &Value) -> Value {
    let src = filename.to_string_val();
    if let Ok(audio) = web_sys::HtmlAudioElement::new_with_src(&src) {
        let _ = audio.play();
    }
    v_null()
}

thread_local! {
    /// The sound PLAYWAV is playing (one at a time, as on the desktop).
    static WAV: std::cell::RefCell<Option<web_sys::HtmlAudioElement>> = const { std::cell::RefCell::new(None) };
}

/// PLAYWAV file|resource, options (as on the desktop, crates/rapidr-
/// runtime-core/src/sound.rs): a new sound replaces the one playing, `""`
/// stops it; SND_LOOP (8, or the manual's 3) repeats it. The page can't
/// wait for the end, so SND_SYNC plays in the background too.
pub fn rp_playwav(source: &Value, options: &Value) {
    WAV.with(|w| {
        if let Some(old) = w.borrow_mut().take() {
            let _ = old.pause();
        }
    });
    let options = options.to_i64();
    let src = match source {
        Value::Integer(_) | Value::Double(_) => match rapidr_value::resources::bytes(source.to_i64()) {
            Some(b) => format!("data:audio/wav;base64,{}", rapidr_value::objects::codec::base64_encode(&b)),
            None => return,
        },
        _ => {
            let path = source.to_string_val();
            if path.is_empty() {
                return;
            }
            crate::database_web::get_rapidr_asset(&path).unwrap_or(path)
        }
    };
    if let Ok(audio) = web_sys::HtmlAudioElement::new_with_src(&src) {
        audio.set_loop(options & 8 != 0 || options == 3);
        let _ = audio.play();
        WAV.with(|w| *w.borrow_mut() = Some(audio));
    }
}

// ---------------------------------------------------------------------------
// Directories — none on web
// ---------------------------------------------------------------------------

/// DIREXISTS: no directories on the web — but the program's own, "." (1, as
/// RapidQ's and the desktop's say).
pub fn rp_direxists(path: &Value) -> Value {
    v_int(matches!(path.to_string_val().trim(), "." | "./" | ".\\") as i64)
}

/// FILEEXISTS: a file the program saved this session or one of its
/// project's files (object_web::web_file_exists).
pub fn rp_fileexists(path: &Value) -> Value {
    crate::object_web::install_file_hooks();
    v_int(if crate::object_web::web_file_exists(&path.to_string_val()) { 1 } else { 0 })
}


// ---------------------------------------------------------------------------
// Additional string functions — identical to desktop (pure Rust)
// ---------------------------------------------------------------------------










// ---------------------------------------------------------------------------
// Constants — identical to desktop
// ---------------------------------------------------------------------------

pub fn rp_const_true() -> Value {
    v_bool(true)
}
pub fn rp_const_false() -> Value {
    v_bool(false)
}

pub const CL_BLACK: i64 = 0x000000;
pub const CL_MAROON: i64 = 0x000080;
pub const CL_GREEN: i64 = 0x008000;
pub const CL_OLIVE: i64 = 0x008080;
pub const CL_NAVY: i64 = 0x800000;
pub const CL_PURPLE: i64 = 0x800080;
pub const CL_TEAL: i64 = 0x808000;
pub const CL_GRAY: i64 = 0x808080;
pub const CL_SILVER: i64 = 0xC0C0C0;
pub const CL_RED: i64 = 0x0000FF;
pub const CL_LIME: i64 = 0x00FF00;
pub const CL_YELLOW: i64 = 0x00FFFF;
pub const CL_BLUE: i64 = 0xFF0000;
pub const CL_FUCHSIA: i64 = 0xFF00FF;
pub const CL_AQUA: i64 = 0xFFFF00;
pub const CL_WHITE: i64 = 0xFFFFFF;

pub const VK_LEFT: i64 = 37;
pub const VK_UP: i64 = 38;
pub const VK_RIGHT: i64 = 39;
pub const VK_DOWN: i64 = 40;
pub const VK_RETURN: i64 = 13;
pub const VK_ESCAPE: i64 = 27;
pub const VK_SPACE: i64 = 32;
pub const VK_TAB: i64 = 9;
pub const VK_DELETE: i64 = 46;
pub const VK_BACK: i64 = 8;
pub const VK_F1: i64 = 112;
pub const VK_F2: i64 = 113;
pub const VK_F3: i64 = 114;
pub const VK_F4: i64 = 115;
pub const VK_F5: i64 = 116;
pub const VK_F6: i64 = 117;
pub const VK_F7: i64 = 118;
pub const VK_F8: i64 = 119;
pub const VK_F9: i64 = 120;
pub const VK_F10: i64 = 121;
pub const VK_F11: i64 = 122;
pub const VK_F12: i64 = 123;

pub const MB_OK: i64 = 0;
pub const MB_OKCANCEL: i64 = 1;
pub const MB_YESNOCANCEL: i64 = 3;
pub const MB_YESNO: i64 = 4;
pub const IDOK: i64 = 1;
pub const IDCANCEL: i64 = 2;
pub const IDYES: i64 = 6;
pub const IDNO: i64 = 7;

/// `MESSAGEBOX(text, title, flags)` (RapidQ): a dialog with the buttons the
/// MB_* flags ask for and their MB_ICONxxx icon; returns IDOK/IDYES/… for
/// the one chosen.
pub fn rp_messagebox(text: &Value, title: &Value, flags: &Value) -> Value {
    use crate::value::dialogs as d;
    let buttons = d::message_box_buttons(flags.to_i64());
    show_choice_beep(&text.to_string_val(), &title.to_string_val(), &buttons, d::message_box_icon(flags.to_i64()), true)
}

/// `MESSAGEDLG(text, mtType, mbButtons, helpContext)` (RapidQ): Delphi's
/// MessageDlg — the type's caption ("Warning" …; mtCustom's is
/// Application.Title) and icon; returns mr*.
pub fn rp_messagedlg(text: &Value, msg_type: &Value, buttons: &Value, _help: &Value) -> Value {
    use crate::value::dialogs as d;
    let title = d::message_dlg_caption(msg_type.to_i64(), &app_title());
    let buttons = d::message_dlg_buttons(buttons.to_i64());
    show_choice(&text.to_string_val(), &title, &buttons, d::message_dlg_icon(msg_type.to_i64()))
}

/// Application.Title (a SHOWMESSAGE's caption, mtCustom's).
fn app_title() -> String {
    crate::globals_web::get("application", "title").map(|t| t.to_string_val()).unwrap_or_default()
}

/// Browser dialogs block, which the interpreter needs; they offer OK (alert)
/// or OK/Cancel (confirm), so a third button (Yes/No/Cancel's Cancel,
/// Abort/Retry/Ignore's Ignore) can't be offered on the web.
fn show_choice(text: &str, title: &str, buttons: &[crate::value::dialogs::Button], icon: Option<crate::value::dialogs::MsgIcon>) -> Value {
    show_choice_beep(text, title, buttons, icon, false)
}

/// [`show_choice`], with the icon's sound as it shows (MESSAGEBOX's, as
/// Windows' MessageBox) where the UI kernel draws the box.
fn show_choice_beep(text: &str, title: &str, buttons: &[crate::value::dialogs::Button], icon: Option<crate::value::dialogs::MsgIcon>, beep: bool) -> Value {
    // (the kernel's box — the desktop's — a wait the VM serves, the button
    // chosen mapped to the builtin's result as the desktop maps it)
    let labels: Vec<&'static str> = buttons.iter().map(|b| b.label).collect();
    let owned = buttons.to_vec();
    let then = move |choice: Option<usize>| {
        v_int(match choice.and_then(|i| owned.get(i)) {
            Some(b) => b.result,
            None => crate::value::dialogs::dismissed(&owned),
        })
    };
    if let Some(v) = crate::kernel_web::choice(title, text, &labels, icon, beep, then) {
        return v;
    }
    // (a native web build can't wait for it: the browser's own)
    let Some(window) = web_sys::window() else { return v_int(0) };
    let message = if title.is_empty() { text.to_string() } else { format!("{title}\n\n{text}") };
    match buttons {
        [only] => {
            let _ = window.alert_with_message(&message);
            v_int(only.result)
        }
        [yes, no, ..] => {
            let note = if yes.label == "OK" && no.label == "Cancel" {
                String::new()
            } else {
                format!("\n\n(OK = {}, Cancel = {})", yes.label, no.label)
            };
            let ok = window.confirm_with_message(&format!("{message}{note}")).unwrap_or(false);
            v_int(if ok { yes.result } else { no.result })
        }
        [] => v_int(0),
    }
}

/// `MOUSEX` / `MOUSEY`: the mouse relative to the client area of the form
/// it's over (kernel_web::mouse_in_form).
pub fn rp_mousex() -> Value {
    v_int(crate::kernel_web::mouse_in_form().0)
}

pub fn rp_mousey() -> Value {
    v_int(crate::kernel_web::mouse_in_form().1)
}
