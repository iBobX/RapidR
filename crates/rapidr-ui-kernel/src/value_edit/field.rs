//! A value editor's one-line field (a colour's Red, its Hex, a font's
//! size, a list's filter): typed into while it has the editor's focus —
//! everything it holds selected when it gets the focus, so typing replaces
//! it —, Backspace, Delete, Home / End, Left / Right; what it may hold is
//! its kind's (digits, hex digits, any text).

use rapidr_value::objects::font::Font;
use rapidr_value::objects::ops::{Place, Rect};
use rapidr_value::objects::text::text_size;

use super::Look;
use crate::paint::Painter;

/// What a field takes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Accept {
    /// 0–9, at most `n`.
    Digits(usize),
    /// A colour's `#RRGGBB` (the `#` kept).
    Hex,
    Text,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Field {
    pub label: &'static str,
    pub text: String,
    /// The caret (a byte of `text`).
    pub caret: usize,
    /// All of it selected (it just got the focus): typing replaces it.
    pub all: bool,
    pub accept: Accept,
}

impl Field {
    pub fn new(label: &'static str, text: &str, accept: Accept) -> Field {
        Field { label, text: text.to_string(), caret: text.len(), all: false, accept }
    }

    /// Its text set from outside (the value changed elsewhere).
    pub fn set(&mut self, text: &str) {
        if self.text != text {
            self.text = text.to_string();
            self.caret = self.text.len();
        }
    }

    pub fn focus(&mut self) {
        self.all = true;
        self.caret = self.text.len();
    }

    fn takes(&self, c: char) -> bool {
        match self.accept {
            Accept::Digits(n) => c.is_ascii_digit() && (self.all || self.text.len() < n),
            Accept::Hex => (c.is_ascii_hexdigit() || c == '#') && (self.all || self.text.trim_start_matches('#').len() < 6),
            Accept::Text => !c.is_control(),
        }
    }

    /// A key (`vk`, the text it types): whether the text changed (`Some`)
    /// or the key was the field's at all.
    pub fn key(&mut self, vk: i64, text: &str) -> Option<bool> {
        match vk {
            8 => {
                if self.all {
                    self.text.clear();
                    self.caret = 0;
                } else if self.caret > 0 {
                    let prev = self.text[..self.caret].chars().last().map_or(0, char::len_utf8);
                    self.caret -= prev;
                    self.text.remove(self.caret);
                }
                self.all = false;
                Some(true)
            }
            46 => {
                if self.all {
                    self.text.clear();
                    self.caret = 0;
                } else if self.caret < self.text.len() {
                    self.text.remove(self.caret);
                }
                self.all = false;
                Some(true)
            }
            36 => {
                self.caret = 0;
                self.all = false;
                Some(false)
            }
            35 => {
                self.caret = self.text.len();
                self.all = false;
                Some(false)
            }
            37 => {
                self.caret = if self.all { 0 } else { self.text[..self.caret].char_indices().last().map_or(0, |(i, _)| i) };
                self.all = false;
                Some(false)
            }
            39 => {
                self.caret = if self.all { self.text.len() } else { self.text[self.caret..].chars().next().map_or(self.caret, |c| self.caret + c.len_utf8()) };
                self.all = false;
                Some(false)
            }
            _ => {
                let typed: String = text.chars().filter(|c| !c.is_control()).collect();
                if typed.is_empty() || !typed.chars().all(|c| self.takes(c)) {
                    return None;
                }
                if self.all {
                    self.text.clear();
                    self.caret = 0;
                    self.all = false;
                }
                self.text.insert_str(self.caret, &typed);
                self.caret += typed.len();
                Some(true)
            }
        }
    }

    /// Draws it in `r`: its label left (when `label_w` > 0), the box, the
    /// text, the caret or the selection when `focused`.
    pub fn paint(&self, p: &mut Painter, r: Rect, label_w: i64, focused: bool, look: &Look, font: &Font) {
        let (x, y, w, h) = r;
        if label_w > 0 {
            p.text((x, y, label_w, h), self.label, font, look.text, Place::Left);
        }
        let b = (x + label_w, y + 2, w - label_w, h - 4);
        p.round(b, 3.0, Some(look.field), Some(if focused { look.focus } else { look.border }), if focused { 2.0 } else { 1.0 });
        let th = text_size("Ag", font).1;
        let ty = b.1 + (b.3 - th) / 2;
        let tw = text_size(&self.text, font).0;
        if focused && self.all && !self.text.is_empty() {
            p.fill((b.0 + 5, ty, tw + 2, th), look.selected);
            p.text((b.0 + 6, ty, b.2 - 10, th), &self.text, font, look.selected_text, Place::TopLeft);
        } else {
            p.text((b.0 + 6, ty, b.2 - 10, th), &self.text, font, look.text, Place::TopLeft);
            if focused {
                let cx = b.0 + 6 + text_size(&self.text[..self.caret.min(self.text.len())], font).0;
                p.fill((cx, ty, 1, th), look.text);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn typing_replaces_what_was_selected() {
        let mut f = Field::new("Red", "255", Accept::Digits(3));
        f.focus();
        assert_eq!(f.key(0, "1"), Some(true));
        assert_eq!(f.text, "1");
        assert_eq!(f.key(0, "x"), None, "digits only");
        f.key(0, "2");
        f.key(0, "8");
        assert_eq!(f.key(0, "9"), None, "three at most");
        f.key(8, "");
        assert_eq!(f.text, "12");
        let mut h = Field::new("Hex", "#FF8214", Accept::Hex);
        h.focus();
        h.key(0, "#00f");
        assert_eq!(h.text, "#00f");
    }
}
