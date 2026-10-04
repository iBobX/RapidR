//! BASIC builtin functions — implementations that back the generated Rust code.

use crate::value::{v_bool, v_dbl, v_int, v_null, v_str, Value};
use std::io::{self, Write};

pub use crate::value::builtins::*;

#[cfg(feature = "audio")]
use rodio::Source;

// ---------------------------------------------------------------------------
// PRINT
// ---------------------------------------------------------------------------

/// BASIC `PRINT` — items are space-separated; optional trailing newline.
pub fn rp_print(items: &[Value], newline: bool) {
    let mut text = items.iter().map(|i| i.to_string_val()).collect::<Vec<_>>().join(" ");
    if newline {
        text.push('\n');
    }
    print!("{text}");
    let _ = io::stdout().flush();
    track_print_column(&text);
}

/// Width of a PRINT zone (`PRINT a, b`), as in QBasic and VB.
pub const PRINT_ZONE_WIDTH: usize = 14;

thread_local! {
    /// Output cursor column (chars since the last newline), for print zones.
    static PRINT_COL: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
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

// ---------------------------------------------------------------------------
// INPUT
// ---------------------------------------------------------------------------

pub fn rp_input(prompt: &Value) -> Value {
    let p = prompt.to_string_val();
    if !p.is_empty() {
        print!("{}", p);
        let _ = io::stdout().flush();
    }
    let mut buf = String::new();
    crate::terminal::line_mode();
    match io::stdin().read_line(&mut buf) {
        Ok(_) => Value::String(buf.trim_end_matches('\n').trim_end_matches('\r').to_string()),
        Err(_) => Value::String(String::new()),
    }
}

// ---------------------------------------------------------------------------
// String functions
// ---------------------------------------------------------------------------
















// ---------------------------------------------------------------------------
// Numeric / conversion functions
// ---------------------------------------------------------------------------
























// ---------------------------------------------------------------------------
// Additional math / conversion (Phase 3d)
// ---------------------------------------------------------------------------












/// DATE$ — current date as MM-DD-YYYY
pub fn rp_date() -> Value {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    // Simple date calculation — days since epoch
    let days = now / 86400;
    let (y, m, d) = days_to_ymd(days as i64 + 719468); // days from year 0 to unix epoch
    Value::String(format!("{:02}-{:02}-{:04}", m, d, y))
}

/// TIME$ — current time as HH:MM:SS
pub fn rp_time() -> Value {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let time_of_day = now % 86400;
    let h = time_of_day / 3600;
    let m = (time_of_day % 3600) / 60;
    let s = time_of_day % 60;
    Value::String(format!("{:02}:{:02}:{:02}", h, m, s))
}

/// Civil date from day count (algorithm from Howard Hinnant).
fn days_to_ymd(z: i64) -> (i64, u32, u32) {
    let era = if z >= 0 { z } else { z - 146096 } / 146097;
    let doe = (z - era * 146097) as u32;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    (y, m, d)
}






// ---------------------------------------------------------------------------
// Type checking
// ---------------------------------------------------------------------------


// ---------------------------------------------------------------------------
// Misc
// ---------------------------------------------------------------------------

pub fn rp_timer() -> Value {
    use std::time::{SystemTime, UNIX_EPOCH};
    let dur = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default();
    v_dbl(dur.as_secs_f64())
}

/// `INKEY$`: the next key pressed, or "" (a console program's terminal;
/// it doesn't wait).
pub fn rp_inkey() -> Value {
    crate::terminal::read_keys();
    rapidr_value::console::inkey()
}

/// INPUT$(n)'s wait (the parser's RAPIDR__INPUTCHARS): sleeps until a key
/// is pressed — in the program's windows when it has any, else in the
/// terminal — and wakes with it, no polling. 0 when no key can come.
pub fn rp_waitkey() -> Value {
    if rapidr_value::console::key_waiting() {
        return v_int(1);
    }
    #[cfg(feature = "desktop-ui")]
    if let Some(waited) = crate::ui::gui_wait_key() {
        return v_int(waited as i64);
    }
    v_int(crate::terminal::wait_key() as i64)
}

/// `GET$(n)`: up to n bytes read from standard input as they are (a CGI
/// program's request body; "" at its end). RapidQ's manual: "a general
/// purpose read function. It will read the STDIN."
pub fn rp_get_stdin(n: &Value) -> Value {
    use std::io::Read;
    let n = n.to_i64().clamp(0, 1 << 24) as usize;
    let mut buf = vec![0u8; n];
    let mut got = 0;
    let mut stdin = io::stdin().lock();
    while got < n {
        match stdin.read(&mut buf[got..]) {
            Ok(0) | Err(_) => break,
            Ok(k) => got += k,
        }
    }
    buf.truncate(got);
    Value::String(buf.iter().map(|&b| char::from(b)).collect())
}

/// `SETCONSOLETITLE title`: the terminal window's title (the standard
/// title escape, which Windows' console understands too).
pub fn rp_set_console_title(title: &Value) -> Value {
    use std::io::IsTerminal;
    let t: String = title.to_string_val().chars().filter(|c| !c.is_control()).collect();
    if io::stdout().is_terminal() {
        print!("\x1b]0;{t}\x07");
        let _ = io::stdout().flush();
    }
    v_null()
}

/// `CHDRIVE "d:"`: the current drive (Windows; other systems have none).
pub fn rp_chdrive(drive: &Value) -> Value {
    #[cfg(windows)]
    {
        let d = drive.to_string_val();
        if let Some(letter) = d.chars().next().filter(char::is_ascii_alphabetic) {
            let _ = std::env::set_current_dir(format!("{letter}:"));
        }
    }
    #[cfg(not(windows))]
    let _ = drive;
    v_null()
}

/// `DOEVENTS`: pending UI events, timers and redraws get their turn
/// (console programs: nothing to do).
pub fn rp_doevents() {
    #[cfg(feature = "desktop-ui")]
    crate::ui::gui_doevents();
}

/// `SLEEP seconds` (RapidQ's manual: `SLEEP 1.5` pauses 1.5 seconds).
pub fn rp_sleep(seconds: &Value) {
    let s = seconds.to_f64();
    if s.is_finite() && s > 0.0 {
        std::thread::sleep(std::time::Duration::from_secs_f64(s.min(86_400.0)));
    }
}

pub fn rp_command() -> Value {
    Value::String(std::env::args().skip(1).collect::<Vec<_>>().join(" "))
}

pub fn rp_environ(name: &Value) -> Value {
    match std::env::var(name.to_string_val()) {
        Ok(v) => Value::String(v),
        Err(_) => v_str(""),
    }
}


pub fn rp_end() {
    // What the program wrote to files it never closed is kept.
    crate::file_io::rp_close_all();
    std::process::exit(0);
}

/// `SHOWMESSAGE text`: RapidQ's modal message box with an OK button,
/// titled with Application.Title. Under a test's hooks (RAPIDR_CAPTURE,
/// RAPIDR_TEST_EVENTS), and without a GUI, the message is printed instead and
/// the program goes on, as if OK was pressed.
pub fn rp_showmessage(msg: &Value) {
    let text = msg.to_string_val();
    #[cfg(feature = "desktop-ui")]
    if std::env::var_os("RAPIDR_CAPTURE").is_none() && std::env::var_os("RAPIDR_TEST_EVENTS").is_none() {
        let title = crate::globals::get("application", "title").map(|t| t.to_string_val()).unwrap_or_default();
        crate::ui::gui_choice(&title, &text, &["OK"]);
        return;
    }
    println!("[SHOWMESSAGE] {text}");
}

pub fn rp_msgbox(msg: &Value) -> Value {
    let text = msg.to_string_val();
    #[cfg(feature = "desktop-ui")]
    crate::ui::message_box(&text);
    #[cfg(not(feature = "desktop-ui"))]
    {
        println!("[MSGBOX] {}", text);
    }
    v_int(0)
}

// Filesystem functions — now in file_io.rs, but keep these thin wrappers
// for backward compatibility with existing codegen output.
pub fn rp_direxists(path: &Value) -> Value {
    v_int(if std::path::Path::new(&path.to_string_val()).is_dir() { -1 } else { 0 })
}

pub fn rp_fileexists(path: &Value) -> Value {
    v_int(if std::path::Path::new(&path.to_string_val()).is_file() { -1 } else { 0 })
}

// Default value for types

// ---------------------------------------------------------------------------
// Additional string functions (Phase 3c)
// ---------------------------------------------------------------------------










// ---------------------------------------------------------------------------
// System / Shell (Phase 3b partial — no crossterm yet)
// ---------------------------------------------------------------------------

/// SHELL — execute a command asynchronously
pub fn rp_shell(command: &Value) -> Value {
    match shell_command(&command.to_string_val()).status() {
        Ok(status) => v_int(status.code().unwrap_or(-1) as i64),
        Err(_) => v_int(-1),
    }
}

/// The system's shell running `cmd` (`cmd /C` on Windows, `sh -c` elsewhere).
fn shell_command(cmd: &str) -> std::process::Command {
    #[cfg(windows)]
    {
        let mut c = std::process::Command::new("cmd");
        c.arg("/C").arg(cmd);
        c
    }
    #[cfg(not(windows))]
    {
        let mut c = std::process::Command::new("sh");
        c.arg("-c").arg(cmd);
        c
    }
}

/// RUN — starts a program without waiting for it; its process ID (0 when
/// it couldn't start).
pub fn rp_run(command: &Value) -> Value {
    match shell_command(&command.to_string_val()).spawn() {
        Ok(child) => v_int(child.id() as i64),
        Err(_) => v_int(0),
    }
}

/// SHELLWAIT — execute a command and wait, return exit code
pub fn rp_shellwait(command: &Value) -> Value {
    match shell_command(&command.to_string_val()).status() {
        Ok(status) => v_int(status.code().unwrap_or(-1) as i64),
        Err(_) => v_int(-1),
    }
}

/// BEEP — play a short beep sound
pub fn rp_beep() {
    #[cfg(feature = "audio")]
    {
        use std::time::Duration;
        std::thread::spawn(|| {
            if let Ok((_stream, handle)) = rodio::OutputStream::try_default() {
                let source = rodio::source::SineWave::new(800.0)
                    .take_duration(Duration::from_millis(200));
                let _ = handle.play_raw(rodio::source::Source::convert_samples(source));
                std::thread::sleep(Duration::from_millis(220));
            }
        });
    }
    #[cfg(not(feature = "audio"))]
    {
        print!("\x07");
        let _ = io::stdout().flush();
    }
}

/// SOUND(freq, duration_ms) — play a tone at the given frequency for the given duration
pub fn rp_sound(freq: &Value, duration: &Value) {
    let freq_hz = freq.to_f64();
    let dur_ms = duration.to_i64() as u64;
    #[cfg(feature = "audio")]
    {
        use std::time::Duration;
        std::thread::spawn(move || {
            if let Ok((_stream, handle)) = rodio::OutputStream::try_default() {
                let source = rodio::source::SineWave::new(freq_hz as f32)
                    .take_duration(Duration::from_millis(dur_ms));
                let _ = handle.play_raw(rodio::source::Source::convert_samples(source));
                std::thread::sleep(Duration::from_millis(dur_ms + 20));
            }
        });
    }
    #[cfg(not(feature = "audio"))]
    {
        let _ = (freq_hz, dur_ms); // suppress unused warnings
    }
}

/// PLAYSOUND(filename) — play a WAV file
pub fn rp_playsound(filename: &Value) -> Value {
    let path = filename.to_string_val();
    #[cfg(feature = "audio")]
    {
        std::thread::spawn(move || {
            if let Ok((_stream, handle)) = rodio::OutputStream::try_default() {
                if let Ok(file) = std::fs::File::open(&path) {
                    let buf = std::io::BufReader::new(file);
                    if let Ok(source) = rodio::Decoder::new(buf) {
                        let sink = rodio::Sink::try_new(&handle).unwrap();
                        sink.append(source);
                        sink.sleep_until_end();
                    } else {
                        eprintln!("[ERROR] PlaySound: unsupported audio format: {}", path);
                    }
                } else {
                    eprintln!("[ERROR] PlaySound: file not found: {}", path);
                }
            }
        });
    }
    #[cfg(not(feature = "audio"))]
    {
        let _ = path;
        eprintln!("[WARN] PlaySound: audio feature not enabled");
    }
    v_null()
}

// ---------------------------------------------------------------------------
// Constants — exposed as functions for codegen convenience
// ---------------------------------------------------------------------------

pub fn rp_const_true() -> Value { v_bool(true) }
pub fn rp_const_false() -> Value { v_bool(false) }

// Color constants (BGR byte order matching BASIC convention)
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

// Virtual key constants
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

// MessageBox constants
pub const MB_OK: i64 = 0;
pub const MB_OKCANCEL: i64 = 1;
pub const MB_YESNOCANCEL: i64 = 3;
pub const MB_YESNO: i64 = 4;
pub const IDOK: i64 = 1;
pub const IDCANCEL: i64 = 2;
pub const IDYES: i64 = 6;
pub const IDNO: i64 = 7;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_len() {
        assert_eq!(rp_len(&v_str("hello")), v_int(5));
    }

    #[test]
    fn test_mid() {
        assert_eq!(rp_mid(&v_str("Hello World"), &v_int(7), &v_int(5)), v_str("World"));
    }

    #[test]
    fn test_hex() {
        assert_eq!(rp_hex(&v_int(255)), v_str("FF"));
    }

    #[test]
    fn test_ucase_lcase() {
        assert_eq!(rp_ucase(&v_str("hello")), v_str("HELLO"));
        assert_eq!(rp_lcase(&v_str("HELLO")), v_str("hello"));
    }

    #[test]
    fn test_math() {
        assert_eq!(rp_abs(&v_int(-5)), v_int(5));
        assert_eq!(rp_sgn(&v_int(-3)), v_int(-1));
        assert_eq!(rp_ceil(&v_dbl(2.3)).to_f64(), 3.0);
    }

    #[test]
    fn test_direxists() {
        assert_eq!(rp_direxists(&v_str(".")), v_int(-1));
        assert_eq!(rp_direxists(&v_str("NONEXISTENT_DIR_12345")), v_int(0));
    }

    #[test]
    fn test_fix_frac() {
        assert_eq!(rp_fix(&v_dbl(-3.7)), v_int(-3));
        assert_eq!(rp_fix(&v_dbl(3.7)), v_int(3));
        let frac = rp_frac(&v_dbl(3.75)).to_f64();
        assert!((frac - 0.75).abs() < 1e-10);
    }

    #[test]
    fn test_cint_clng() {
        assert_eq!(rp_cint(&v_dbl(3.6)), v_int(4));
        assert_eq!(rp_cint(&v_dbl(3.4)), v_int(3));
        assert_eq!(rp_clng(&v_dbl(-2.7)), v_int(-3));
    }

    #[test]
    fn test_iif() {
        assert_eq!(rp_iif(&v_bool(true), &v_str("yes"), &v_str("no")), v_str("yes"));
        assert_eq!(rp_iif(&v_bool(false), &v_str("yes"), &v_str("no")), v_str("no"));
    }

    #[test]
    fn test_hextodec() {
        assert_eq!(rp_hextodec(&v_str("FF")), v_int(255));
        assert_eq!(rp_hextodec(&v_str("&HFF")), v_int(255));
        assert_eq!(rp_hextodec(&v_str("0xFF")), v_int(255));
    }

    #[test]
    fn test_convbase() {
        assert_eq!(rp_convbase(&v_str("255"), &v_int(10), &v_int(16)), v_str("FF"));
        assert_eq!(rp_convbase(&v_str("FF"), &v_int(16), &v_int(10)), v_str("255"));
        assert_eq!(rp_convbase(&v_str("10"), &v_int(10), &v_int(2)), v_str("1010"));
    }

    #[test]
    fn test_insert_delete() {
        // RapidQ order: INSERT$(insert, source, index).
        assert_eq!(rp_insert(&v_str(" Beautiful"), &v_str("Hello World"), &v_int(6)), v_str("Hello Beautiful World"));
        assert_eq!(rp_insert(&v_str("hi"), &v_str("Hello"), &v_int(3)), v_str("Hehillo"));
        assert_eq!(rp_delete(&v_str("Hello World"), &v_int(6), &v_int(1)), v_str("HelloWorld"));
    }

    #[test]
    fn test_reverse() {
        assert_eq!(rp_reverse(&v_str("Hello")), v_str("olleH"));
    }

    #[test]
    fn test_field() {
        assert_eq!(rp_field(&v_str("one,two,three"), &v_str(","), &v_int(2)), v_str("two"));
        assert_eq!(rp_field(&v_str("one,two,three"), &v_str(","), &v_int(4)), v_str(""));
    }

    #[test]
    fn test_tally() {
        assert_eq!(rp_tally(&v_str("hello world hello"), &v_str("hello")), v_int(2));
    }

    #[test]
    fn test_rinstr() {
        assert_eq!(rp_rinstr(&v_str("hello world hello"), &v_str("hello")), v_int(13));
        assert_eq!(rp_rinstr(&v_str("hello"), &v_str("xyz")), v_int(0));
    }

    #[test]
    fn test_format() {
        assert_eq!(rp_format(&v_str("%.2f|%.5d"), &[v_dbl(3.14159), v_int(42)]), v_str("3.14|00042"));
    }

    #[test]
    fn test_rnd() {
        let r = rp_rnd(&v_int(0));
        let f = r.to_f64();
        assert!(f >= 0.0 && f < 1.0);

        let r2 = rp_rnd(&v_int(10));
        let n = r2.to_i64();
        assert!(n >= 0 && n < 10);
    }

    #[test]
    fn test_rgb() {
        assert_eq!(rp_rgb(&v_int(255), &v_int(0), &v_int(0)), v_int(255)); // red
        assert_eq!(rp_rgb(&v_int(0), &v_int(255), &v_int(0)), v_int(0xFF00)); // green
    }

    #[test]
    fn test_vartype() {
        assert_eq!(rp_vartype(&v_int(0)), v_int(2));
        assert_eq!(rp_vartype(&v_dbl(0.0)), v_int(5));
        assert_eq!(rp_vartype(&v_str("")), v_int(8));
    }

    #[test]
    fn test_date_time() {
        let d = rp_date().to_string_val();
        assert_eq!(d.len(), 10); // MM-DD-YYYY
        let t = rp_time().to_string_val();
        assert_eq!(t.len(), 8); // HH:MM:SS
    }
}

/// `MESSAGEBOX(text, title, flags)` (RapidQ): a dialog with the buttons the
/// MB_* flags ask for; returns IDOK/IDYES/… for the one chosen.
pub fn rp_messagebox(text: &Value, title: &Value, flags: &Value) -> Value {
    let buttons = crate::value::dialogs::message_box_buttons(flags.to_i64());
    show_choice(&text.to_string_val(), &title.to_string_val(), &buttons)
}

/// `MESSAGEDLG(text, mtType, mbButtons, helpContext)` (RapidQ): returns mr*.
pub fn rp_messagedlg(text: &Value, msg_type: &Value, buttons: &Value, _help: &Value) -> Value {
    let title = crate::value::dialogs::message_dlg_title(msg_type.to_i64());
    let buttons = crate::value::dialogs::message_dlg_buttons(buttons.to_i64());
    show_choice(&text.to_string_val(), title, &buttons)
}

fn show_choice(text: &str, title: &str, buttons: &[crate::value::dialogs::Button]) -> Value {
    #[cfg(feature = "desktop-ui")]
    {
        let labels: Vec<&str> = buttons.iter().map(|b| b.label).collect();
        let result = match crate::ui::gui_choice(title, text, &labels) {
            Some(i) => buttons[i].result,
            None => crate::value::dialogs::dismissed(buttons),
        };
        v_int(result)
    }
    #[cfg(not(feature = "desktop-ui"))]
    {
        // No GUI: show the message and take the first (affirmative) button.
        println!("[{title}] {text}");
        v_int(buttons.first().map_or(1, |b| b.result))
    }
}

/// `MOUSEX` / `MOUSEY`: the mouse pointer relative to the active form's
/// client area.
pub fn rp_mousex() -> Value {
    #[cfg(feature = "desktop-ui")]
    return v_int(crate::ui::mouse_in_form().0);
    #[cfg(not(feature = "desktop-ui"))]
    v_int(0)
}

pub fn rp_mousey() -> Value {
    #[cfg(feature = "desktop-ui")]
    return v_int(crate::ui::mouse_in_form().1);
    #[cfg(not(feature = "desktop-ui"))]
    v_int(0)
}
