//! RapidQ's global objects — `Screen`, `Application`, `Clipboard`, `Mouse`
//! — the same on every runtime: what they answer comes from the runtime's
//! [`Platform`] (the UI kernel's host and the system clipboard, arboard, on
//! the desktop; the browser on the web); what the program sets and reads
//! back is kept here.

use crate::{v_bool, v_int, v_null, v_str, Value};
use std::cell::RefCell;
use std::collections::HashMap;

/// Clipboard format of plain text (Windows' CF_TEXT).
pub const CF_TEXT: i64 = 1;

/// What a runtime's platform answers.
pub trait Platform {
    /// The screen's size (Screen.Width / Height).
    fn screen_size(&self) -> (i64, i64);
    /// The screen less task bars and menu bars (Screen.ClientWidth / …).
    fn work_area(&self) -> (i64, i64) {
        self.screen_size()
    }
    /// The mouse on the screen (Screen.MouseX / MouseY).
    fn mouse(&self) -> (i64, i64);
    fn monitors(&self) -> i64 {
        1
    }
    fn clipboard_text(&self) -> String;
    fn set_clipboard_text(&self, text: &str);
    /// The program's executable path ("" when it has none).
    fn exe_path(&self) -> String;
    /// Application.Terminate: the program ends.
    fn terminate(&self);
    /// Application.Minimize: its windows minimized.
    fn minimize(&self) {}
    /// Screen.Cursor: the mouse pointer over every form (crDefault = 0).
    fn set_cursor(&self, _cursor: i64) {}
    /// Application.Title: the program's name (task bar / tab).
    fn set_title(&self, _title: &str) {}
    /// Screen.Scale: device pixels per pixel (2 on a Retina screen, 1.5 at
    /// 150 %) — programs keep RapidQ's pixels; this tells them how fine the
    /// screen is.
    fn scale(&self) -> f64 {
        crate::objects::bitmap::exact_scale()
    }
    /// `Application.Icon` / `IcoHandle` changed: forms without their own
    /// icon take it ([`application_icon`]).
    fn set_icon(&self) {}
    /// `Application.Theme` (RapidR's): the look the desktop draws with
    /// (`crate::theme`; the web draws its own).
    fn theme(&self) -> String {
        crate::theme::current().name.to_string()
    }
    /// `Application.Theme = name`: draw with that theme from now on.
    fn set_theme(&self, _name: &str) {}
}

thread_local! {
    /// What the program set: (object, property) → value.
    static STORED: RefCell<HashMap<(String, String), Value>> = RefCell::new(HashMap::new());
}

/// `name`'s global object: "screen", "application", "clipboard", "mouse",
/// "filerec".
pub fn global(name: &str) -> Option<&'static str> {
    ["screen", "application", "clipboard", "mouse", "filerec"].into_iter().find(|g| g.eq_ignore_ascii_case(name))
}

/// `FileRec` (RapidQ's manual, DIR$): the file DIR$ found last — RC.EXE's
/// FileName, ShortName ("" when the name is short already), Date
/// (`10-5-2026`: month-day-year), Time (`10:07`), Size (bytes), FileTime
/// (newer files greater).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct FileRec {
    pub file_name: String,
    pub short_name: String,
    pub date: String,
    pub time: String,
    pub size: i64,
    pub file_time: i64,
}

thread_local! {
    static FILE_REC: RefCell<FileRec> = RefCell::new(FileRec::default());
}

/// DIR$ found a file: FileRec says it.
pub fn set_file_rec(rec: FileRec) {
    FILE_REC.with(|f| *f.borrow_mut() = rec);
}

fn file_rec(prop: &str) -> Option<Value> {
    FILE_REC.with(|f| {
        let f = f.borrow();
        Some(match prop {
            "filename" => v_str(&f.file_name),
            "shortname" => v_str(&f.short_name),
            "date" => v_str(&f.date),
            "time" => v_str(&f.time),
            "size" => v_int(f.size),
            "filetime" => v_int(f.file_time),
            _ => return None,
        })
    })
}

fn stored(object: &str, prop: &str) -> Option<Value> {
    STORED.with(|s| s.borrow().get(&(object.to_string(), prop.to_string())).cloned())
}

fn store(object: &str, prop: &str, value: Value) {
    STORED.with(|s| s.borrow_mut().insert((object.to_string(), prop.to_string()), value));
}

/// Application.ExeName: the executable's file name.
pub fn exe_name(path: &str) -> String {
    path.rsplit(['/', '\\']).next().unwrap_or("").to_string()
}

/// Application.Path: the executable's folder.
pub fn exe_folder(path: &str) -> String {
    match path.rfind(['/', '\\']) {
        Some(i) => path[..i].to_string(),
        None => String::new(),
    }
}

/// The application's icon (`Application.IcoHandle`, else `.Icon`), for
/// forms without their own.
pub fn application_icon() -> Option<Value> {
    ["icohandle", "icon"].into_iter().filter_map(|p| stored("application", p)).find(|v| crate::objects::has_icon(v))
}

/// Application's hint settings (lowercase `prop`: ShowHint, HintPause,
/// HintHidePause, HintShortPause — milliseconds — and HintColor), as the
/// program set them or the VCL's defaults: what the UI kernel's tooltips
/// follow.
pub fn hint_setting(prop: &str) -> Value {
    stored("application", prop).unwrap_or_else(|| match prop {
        "showhint" => v_bool(true),
        "hintpause" => v_int(500),
        "hinthidepause" => v_int(2500),
        "hintshortpause" => v_int(50),
        "hintcolor" => v_int(0x00E1_FFFF),
        _ => Value::Null,
    })
}

/// A property of global object `name` (lowercase `prop`); `None` for one it
/// doesn't have (the runtime's own lookup goes on).
pub fn get(p: &dyn Platform, name: &str, prop: &str) -> Option<Value> {
    let object = global(name)?;
    if object == "filerec" {
        return file_rec(prop);
    }
    let v = match (object, prop) {
        ("screen", "width") => v_int(p.screen_size().0),
        ("screen", "height") => v_int(p.screen_size().1),
        ("screen", "clientwidth") => v_int(p.work_area().0),
        ("screen", "clientheight") => v_int(p.work_area().1),
        ("screen", "mousex") | ("mouse", "x") => v_int(p.mouse().0),
        ("screen", "mousey") | ("mouse", "y") => v_int(p.mouse().1),
        ("screen", "monitors") => v_int(p.monitors()),
        // (RapidR's: RapidQ's pixels stay 96 to the inch — a program that
        // scales by PixelsPerInch / 96 would otherwise scale twice)
        ("screen", "scale") => Value::Double(p.scale()),
        ("screen", "pixelsperinch") => v_int(96),
        ("screen", "consolex") => stored(object, prop).unwrap_or(v_int(80)),
        ("screen", "consoley") => stored(object, prop).unwrap_or(v_int(25)),
        ("screen", "cursor") => stored(object, prop).unwrap_or(v_int(0)),
        ("clipboard", "text") => v_str(&p.clipboard_text()),
        ("clipboard", "formatcount") => v_int(i64::from(!p.clipboard_text().is_empty())),
        ("application", "exename") => stored(object, prop).unwrap_or_else(|| v_str(&exe_name(&p.exe_path()))),
        ("application", "path") => v_str(&exe_folder(&p.exe_path())),
        ("application", "title") => stored(object, prop).unwrap_or_else(|| {
            let name = exe_name(&p.exe_path());
            v_str(name.rsplit_once('.').map_or(name.as_str(), |(stem, _)| stem))
        }),
        ("application", "showhint" | "hintpause" | "hinthidepause" | "hintshortpause" | "hintcolor") => hint_setting(prop),
        // (RapidR's: the theme drawn now — `auto` reads as what it chose)
        ("application", "theme") => v_str(&p.theme()),
        // (RC.EXE: Application.Icon reads as the icon's handle — a number,
        // never 0 — which a QNOTIFYICONDATA's hIcon takes; crate::tray
        // shows the application's icon for it)
        ("application", "icon") => v_int(crate::handles::icon_handle("")),
        _ => stored(object, prop)?,
    };
    Some(v)
}

/// Sets a property of global object `name`; false when `name` isn't one.
pub fn set(p: &dyn Platform, name: &str, prop: &str, value: &Value) -> bool {
    let Some(object) = global(name) else { return false };
    match (object, prop) {
        ("clipboard", "text") => p.set_clipboard_text(&value.to_string_val()),
        ("screen", "cursor") => {
            p.set_cursor(value.to_i64());
            store(object, prop, value.clone());
        }
        ("application", "title") => {
            p.set_title(&value.to_string_val());
            store(object, prop, value.clone());
        }
        ("application", "icon" | "icohandle") => {
            store(object, prop, value.clone());
            p.set_icon();
        }
        ("application", "theme") => p.set_theme(&value.to_string_val()),
        // (read-only: what the platform answers)
        ("screen", "width" | "height" | "clientwidth" | "clientheight" | "mousex" | "mousey" | "monitors" | "scale" | "pixelsperinch") => {}
        _ => store(object, prop, value.clone()),
    }
    true
}

/// A method of global object `name`; `None` for one it doesn't have.
pub fn call(p: &dyn Platform, name: &str, method: &str, args: &[Value]) -> Option<Value> {
    let object = global(name)?;
    let arg = |i: usize| args.get(i).cloned().unwrap_or_else(v_null);
    Some(match (object, method) {
        ("clipboard", "setastext") => {
            p.set_clipboard_text(&arg(0).to_string_val());
            v_null()
        }
        // GetAsText(n): n is the buffer's size, its closing NUL included —
        // n - 1 characters (RC.EXE: "abcdef" with 3 gives "ab").
        ("clipboard", "getastext") => {
            let text = p.clipboard_text();
            match args.first().map(Value::to_i64) {
                Some(n) if n >= 0 => v_str(&text.chars().take((n - 1).max(0) as usize).collect::<String>()),
                _ => v_str(&text),
            }
        }
        ("clipboard", "clear") => {
            p.set_clipboard_text("");
            v_null()
        }
        ("clipboard", "hasformat") => v_bool(arg(0).to_i64() == CF_TEXT && !p.clipboard_text().is_empty()),
        // Format(i): the formats there are — text only.
        ("clipboard", "format") => v_int(if arg(0).to_i64() == 0 && !p.clipboard_text().is_empty() { CF_TEXT } else { 0 }),
        // (the clipboard is never held by one program here)
        ("clipboard", "open" | "close") => v_null(),
        ("application", "terminate") => {
            p.terminate();
            v_null()
        }
        ("application", "minimize") => {
            p.minimize();
            v_null()
        }
        ("screen", "getpixeldepth") => v_int(32),
        ("screen", "monitors") => v_int(p.monitors()),
        ("screen", "mousebuttons") => v_int(3),
        ("screen", "mousepresent") => v_bool(true),
        ("screen", "mouseswap") => v_bool(false),
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;

    #[derive(Default)]
    struct Fake {
        clip: RefCell<String>,
        ended: RefCell<bool>,
    }

    impl Platform for Fake {
        fn screen_size(&self) -> (i64, i64) {
            (1920, 1080)
        }
        fn mouse(&self) -> (i64, i64) {
            (5, 6)
        }
        fn clipboard_text(&self) -> String {
            self.clip.borrow().clone()
        }
        fn set_clipboard_text(&self, text: &str) {
            *self.clip.borrow_mut() = text.to_string();
        }
        fn exe_path(&self) -> String {
            "/home/me/apps/demo.exe".into()
        }
        fn terminate(&self) {
            *self.ended.borrow_mut() = true;
        }
    }

    #[test]
    fn screen_and_mouse() {
        let p = Fake::default();
        assert_eq!(get(&p, "Screen", "width").unwrap().to_i64(), 1920);
        assert_eq!(get(&p, "SCREEN", "clientheight").unwrap().to_i64(), 1080);
        assert_eq!(get(&p, "screen", "mousey").unwrap().to_i64(), 6);
        assert_eq!(get(&p, "mouse", "x").unwrap().to_i64(), 5);
        assert!(get(&p, "screen", "x").is_none());
        assert!(get(&p, "form1", "width").is_none());
        assert!(set(&p, "screen", "width", &v_int(3)));
        assert_eq!(get(&p, "screen", "width").unwrap().to_i64(), 1920);
    }

    #[test]
    fn clipboard_text() {
        let p = Fake::default();
        assert!(!call(&p, "clipboard", "hasformat", &[v_int(CF_TEXT)]).unwrap().to_bool());
        set(&p, "Clipboard", "text", &v_str("hello"));
        assert_eq!(get(&p, "clipboard", "text").unwrap().to_string_val(), "hello");
        assert_eq!(call(&p, "clipboard", "getastext", &[v_int(3)]).unwrap().to_string_val(), "he");
        assert!(call(&p, "clipboard", "hasformat", &[v_int(CF_TEXT)]).unwrap().to_bool());
        assert_eq!(get(&p, "clipboard", "formatcount").unwrap().to_i64(), 1);
        call(&p, "clipboard", "clear", &[]);
        assert_eq!(get(&p, "clipboard", "text").unwrap().to_string_val(), "");
    }

    #[test]
    fn application() {
        let p = Fake::default();
        assert_eq!(get(&p, "application", "exename").unwrap().to_string_val(), "demo.exe");
        assert_eq!(get(&p, "application", "path").unwrap().to_string_val(), "/home/me/apps");
        assert_eq!(get(&p, "application", "title").unwrap().to_string_val(), "demo");
        set(&p, "application", "title", &v_str("Mine"));
        assert_eq!(get(&p, "application", "title").unwrap().to_string_val(), "Mine");
        set(&p, "application", "helpfile", &v_str("x.hlp"));
        assert_eq!(get(&p, "application", "helpfile").unwrap().to_string_val(), "x.hlp");
        call(&p, "application", "terminate", &[]);
        assert!(*p.ended.borrow());
        // (RapidR's Theme: the platform's theme drawn now — the classic look
        // until one is named; setting it is the platform's to apply)
        assert_eq!(get(&p, "application", "theme").unwrap().to_string_val(), "classic");
        assert!(set(&p, "application", "theme", &v_str("dark")));
        assert_eq!(get(&p, "application", "theme").unwrap().to_string_val(), "classic");
    }
}
