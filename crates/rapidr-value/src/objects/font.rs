//! QFONT (manual, Appendix A): a font description assigned to components
//! with `Label.Font = Font`. Styles are RAPIDQ.INC's fsBold = 0,
//! fsItalic = 1, fsUnderline = 2, fsStrikeOut = 3.

use crate::{v_int, v_str, Value};

#[derive(Debug, Clone, PartialEq)]
pub struct Font {
    pub name: String,
    pub size: i64,
    pub color: i64,
    /// Bit n set = style n (fsBold …) on.
    pub styles: u8,
}

impl Font {
    /// Its height in pixels, as Windows makes it (RapidQ's fonts are GDI's):
    /// points at 96 dpi rounded as MulDiv rounds (10 pt → 13, 12 → 16); a
    /// negative size is pixels already. What text is drawn and measured at
    /// (TextWidth) on every runtime.
    pub fn pixel_size(&self) -> i64 {
        if self.size < 0 {
            (-self.size).min(1_000)
        } else {
            // (MS Sans Serif is a bitmap font: 9, 11 and 13 points show its
            // 8, 10 and 12, RapidQ's capture)
            let points = match self.size {
                9 | 11 | 13 if super::text::family_name(&self.name) == "RapidR Sans" => self.size - 1,
                p => p,
            };
            (points.clamp(1, 1_000) * 96 + 36) / 72
        }
    }
}

impl Default for Font {
    fn default() -> Self {
        Self { name: super::DEFAULT_FONT_NAME.into(), size: super::DEFAULT_FONT_SIZE, color: 0, styles: 0 }
    }
}

/// Font names every RapidR runtime can show (the desktop and web runtimes
/// map them to the platform's own fonts): what `FontName(i)` lists.
pub const FONT_NAMES: &[&str] = &["Arial", "Courier New", "Times New Roman", "MS Sans Serif", "Symbol", "Inter", "JetBrains Mono"];

const STYLE_NAMES: [&str; 4] = ["bold", "italic", "underline", "strikeout"];

/// A component's font styles as flat properties, by style number (fsBold 0 …
/// fsStrikeOut 3), and as the program writes them (`Label.Font.Bold`).
const COMPONENT_STYLES: [(&str, &str); 4] = [("fontbold", "font.bold"), ("fontitalic", "font.italic"), ("fontunderline", "font.underline"), ("fontstrikeout", "font.strikeout")];

/// A component's font style set (`Label.Font.Underline = 5`, `FontBold =
/// -1`): its flat and dotted property names and the value RapidQ keeps —
/// 1 for anything but 0 (RC.EXE reads `Font.Bold` back as 1 after
/// `= -1`). `None`: not a style.
pub fn component_style(prop: &str, val: &Value) -> Option<(&'static str, &'static str, Value)> {
    let prop = prop.to_ascii_lowercase();
    let (flat, dotted) = COMPONENT_STYLES.iter().find(|(f, d)| *f == prop || *d == prop)?;
    Some((flat, dotted, v_int(i64::from(val.to_bool()))))
}

/// `Label.Font.AddStyles(fsBold, …)` / `DelStyles(…)` on a component (the
/// runtimes get `font.addstyles`): the styles' flat properties and their
/// new value, 1 or 0; numbers outside fsBold … fsStrikeOut change nothing
/// (RC.EXE). `None`: not one of them.
pub fn component_style_call(method: &str, args: &[Value]) -> Option<Vec<(&'static str, Value)>> {
    let on = match method {
        "font.addstyles" => 1,
        "font.delstyles" => 0,
        _ => return None,
    };
    Some(args.iter().filter_map(|a| usize::try_from(a.to_i64()).ok().and_then(|i| COMPONENT_STYLES.get(i)).map(|(flat, _)| (*flat, v_int(on)))).collect())
}

impl Font {
    /// A style as RapidQ reads it: 1 on, 0 off (RC.EXE).
    fn flag(&self, style: usize) -> Value {
        v_int(i64::from(self.styles & 1 << style != 0))
    }

    pub fn get(&self, prop: &str) -> Option<Value> {
        if let Some(style) = STYLE_NAMES.iter().position(|s| *s == prop) {
            return Some(self.flag(style));
        }
        Some(match prop {
            "name" => v_str(&self.name),
            "size" => v_int(self.size),
            "color" => v_int(self.color),
            "fontcount" => v_int(FONT_NAMES.len() as i64),
            _ => return None,
        })
    }

    pub fn set(&mut self, prop: &str, val: &Value) -> bool {
        if let Some(style) = STYLE_NAMES.iter().position(|s| *s == prop) {
            self.set_style(style as i64, val.to_bool());
            return true;
        }
        match prop {
            "name" => self.name = val.to_string_val(),
            "size" => self.size = val.to_i64(),
            "color" => self.color = val.to_i64(),
            _ => return false,
        }
        true
    }

    fn set_style(&mut self, style: i64, on: bool) {
        if (0..4).contains(&style) {
            if on {
                self.styles |= 1 << style;
            } else {
                self.styles &= !(1 << style);
            }
        }
    }

    pub fn call(&mut self, method: &str, args: &[Value]) -> Option<Value> {
        match method {
            "addstyles" | "delstyles" => {
                for a in args {
                    self.set_style(a.to_i64(), method == "addstyles");
                }
                Some(Value::Null)
            }
            // `Font.FontName(i)`: the i-th available font name.
            "fontname" => Some(v_str(args.first().and_then(|i| FONT_NAMES.get(i.to_i64() as usize)).unwrap_or(&""))),
            _ => None,
        }
    }

    /// The component properties `X.Font = ThisFont` sets (the flat names the
    /// runtimes' widgets read).
    pub fn component_properties(&self) -> Vec<(&'static str, Value)> {
        vec![
            ("fontname", v_str(&self.name)),
            ("fontsize", v_int(self.size)),
            ("fontcolor", v_int(self.color)),
            ("fontbold", self.flag(0)),
            ("fontitalic", self.flag(1)),
            ("fontunderline", self.flag(2)),
            ("fontstrikeout", self.flag(3)),
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn styles() {
        let mut f = Font::default();
        f.call("addstyles", &[v_int(0), v_int(1)]);
        assert_eq!((f.get("bold").unwrap().to_i64(), f.get("italic").unwrap().to_i64()), (1, 1));
        f.call("delstyles", &[v_int(0)]);
        assert_eq!(f.get("bold").unwrap().to_i64(), 0);
        f.set("underline", &v_int(-1));
        assert_eq!(f.styles, 0b110);
        assert_eq!(f.call("fontname", &[v_int(0)]).unwrap().to_string_val(), "Arial");
        assert_eq!(component_style("Font.Underline", &v_int(5)), Some(("fontunderline", "font.underline", v_int(1))));
        assert_eq!(component_style_call("font.addstyles", &[v_int(0), v_int(7), v_int(3)]), Some(vec![("fontbold", v_int(1)), ("fontstrikeout", v_int(1))]));
    }
}
