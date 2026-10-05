//! RapidQ's two data types that are objects: QRECT (`Left`, `Top`,
//! `Right`, `Bottom`) and QNOTIFYICONDATA (Windows' NOTIFYICONDATA:
//! `cbSize`, `hWnd`, `uID`, `uFlags`, `uCallbackMessage`, `hIcon`,
//! `szTip`) — fixed fields, stored as RapidQ's compiler (RC.EXE) stores them
//! (docs/rapidq-ground-truth.md):
//!
//! - every number field is a 32-bit integer: a fractional value is cut
//!   toward zero (`3.7` → 3, `-2.5` → -2), one outside 32 bits becomes
//!   -2147483648 (the x87's "integer indefinite"), a string 0 (`"12"` too);
//! - QNOTIFYICONDATA's `cbSize` is 88 and read-only (the compiler refuses
//!   the store: `N.CBSIZE is a read-only value.`), `uID` starts as the
//!   program's instance handle (4194304, `&H400000`), the rest 0;
//! - `szTip` keeps at most 64 characters, up to the first `CHR$(0)`; a
//!   number stored into it is "".
//!
//! Members they don't have, assigning one to another, and either inside a
//! TYPE without EXTENDS are the compiler's errors (`rapidr_ast::rapidq_checks`).

use crate::{v_int, v_str, Value};

/// The instance handle RapidQ programs see (a 32-bit Windows program's
/// image base): QNOTIFYICONDATA's `uID` until set.
pub const HINSTANCE: i64 = 0x0040_0000;
/// QNOTIFYICONDATA's `cbSize` (NOTIFYICONDATAA's size before Windows 2000).
pub const NOTIFYICONDATA_SIZE: i64 = 88;
/// The most characters `szTip` keeps.
pub const TIP_LENGTH: usize = 64;

/// Which data type.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Rect,
    NotifyIconData,
}

impl Kind {
    /// The kind of RapidR type `type_name` (RRECT, RNOTIFYICONDATA).
    pub fn of(type_name: &str) -> Option<Kind> {
        match type_name.to_ascii_uppercase().as_str() {
            "RRECT" | "QRECT" => Some(Kind::Rect),
            "RNOTIFYICONDATA" | "QNOTIFYICONDATA" => Some(Kind::NotifyIconData),
            _ => None,
        }
    }

    /// Its number fields (lowercase), in order.
    pub fn numbers(self) -> &'static [&'static str] {
        match self {
            Kind::Rect => &["left", "top", "right", "bottom"],
            Kind::NotifyIconData => &["cbsize", "hwnd", "uid", "uflags", "ucallbackmessage", "hicon"],
        }
    }

    /// Its fields as RapidQ's manual writes them.
    pub fn members(self) -> &'static [&'static str] {
        match self {
            Kind::Rect => &["Left", "Top", "Right", "Bottom"],
            Kind::NotifyIconData => &["cbSize", "hWnd", "uID", "uFlags", "uCallbackMessage", "hIcon", "szTip"],
        }
    }

    /// What `SIZEOF` answers for one (RC.EXE: 16 and 24 — QNOTIFYICONDATA's
    /// six numbers, its tip not counted).
    pub fn size(self) -> i64 {
        match self {
            Kind::Rect => 16,
            Kind::NotifyIconData => 24,
        }
    }
}

/// A QRECT or a QNOTIFYICONDATA.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Record {
    pub kind: Kind,
    numbers: Vec<i64>,
    pub tip: String,
}

/// A value stored into a 32-bit field, as RapidQ converts it.
pub fn to_int32(v: &Value) -> i64 {
    let f = match v {
        Value::Integer(n) => *n as f64,
        Value::Double(d) => *d,
        Value::Boolean(b) => {
            if *b {
                -1.0
            } else {
                0.0
            }
        }
        // (a string, an object …: 0)
        _ => return 0,
    };
    let t = f.trunc();
    if t.is_nan() || t < i32::MIN as f64 || t > i32::MAX as f64 {
        i32::MIN as i64
    } else {
        t as i64
    }
}

/// A value stored into `szTip`: text up to its first NUL, at most
/// [`TIP_LENGTH`] characters; a number is "".
pub fn to_tip(v: &Value) -> String {
    match v {
        Value::String(_) => v.to_string_val().split('\0').next().unwrap_or("").chars().take(TIP_LENGTH).collect(),
        _ => String::new(),
    }
}

impl Record {
    pub fn new(kind: Kind) -> Record {
        let mut numbers = vec![0; kind.numbers().len()];
        if kind == Kind::NotifyIconData {
            numbers[0] = NOTIFYICONDATA_SIZE;
            numbers[2] = HINSTANCE;
        }
        Record { kind, numbers, tip: String::new() }
    }

    fn slot(&self, prop: &str) -> Option<usize> {
        self.kind.numbers().iter().position(|f| f.eq_ignore_ascii_case(prop))
    }

    /// Number field `prop` (lowercase), 0 when it has none of that name.
    pub fn number(&self, prop: &str) -> i64 {
        self.slot(prop).map_or(0, |i| self.numbers[i])
    }

    pub fn get(&self, prop: &str) -> Option<Value> {
        if self.kind == Kind::NotifyIconData && prop.eq_ignore_ascii_case("sztip") {
            return Some(v_str(&self.tip));
        }
        self.slot(prop).map(|i| v_int(self.numbers[i]))
    }

    /// Stores `prop`; `false` for a field it doesn't have. `cbSize` keeps
    /// its value (the compiler refuses such a store).
    pub fn set(&mut self, prop: &str, v: &Value) -> bool {
        if self.kind == Kind::NotifyIconData && prop.eq_ignore_ascii_case("sztip") {
            self.tip = to_tip(v);
            return true;
        }
        let Some(i) = self.slot(prop) else { return false };
        if !(self.kind == Kind::NotifyIconData && i == 0) {
            self.numbers[i] = to_int32(v);
        }
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn numbers_are_stored_as_rapidq_stores_them() {
        let mut r = Record::new(Kind::Rect);
        assert_eq!(r.get("left"), Some(v_int(0)));
        for (v, want) in [
            (Value::Double(3.7), 3),
            (Value::Double(-2.5), -2),
            (Value::Double(-3.9), -3),
            (Value::Double(2147483648.0), -2147483648),
            (Value::Double(4294967297.0), -2147483648),
            (v_str("12"), 0),
            (v_str("abc"), 0),
            (v_int(-5), -5),
        ] {
            assert!(r.set("Left", &v));
            assert_eq!(r.number("left"), want, "{v:?}");
        }
        assert!(!r.set("width", &v_int(1)));
        assert_eq!(r.get("width"), None);
    }

    #[test]
    fn notify_icon_data_starts_as_rapidq_s() {
        let mut n = Record::new(Kind::NotifyIconData);
        assert_eq!(n.number("cbsize"), 88);
        assert_eq!(n.number("uid"), 4194304);
        assert_eq!(n.number("hwnd"), 0);
        assert_eq!(n.get("sztip"), Some(v_str("")));
        n.set("cbsize", &v_int(5));
        assert_eq!(n.number("cbsize"), 88);
        n.set("szTip", &v_str(&"x".repeat(100)));
        assert_eq!(n.tip.len(), 64);
        n.set("szTip", &v_str("a\0b"));
        assert_eq!(n.tip, "a");
        n.set("szTip", &v_int(42));
        assert_eq!(n.tip, "");
        n.set("hIcon", &v_str("77"));
        assert_eq!(n.number("hicon"), 0);
        assert_eq!(Kind::NotifyIconData.size(), 24);
        assert_eq!(Kind::Rect.size(), 16);
    }
}
