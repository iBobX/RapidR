//! How the designer writes CREATE blocks it makes (a component added from
//! the toolbox, pasted, duplicated): `CREATE Name AS Type`, one property per
//! line indented one step in, nested CREATEs, `END CREATE` — in the file's
//! own indentation and line ends. Names follow docs/q-and-r-components.md
//! and the file's own style (R-NAMES): RapidR's names (`RButton`), or, in a
//! file written with RapidQ's names, RapidQ's (`QBUTTON`) — never mixed.

use super::model::{FormDesign, SubItem, Subtree};
use crate::layout::Rect;

/// The file's way of writing: its indentation step and line end.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Style {
    pub indent: String,
    pub eol: String,
}

impl Default for Style {
    fn default() -> Self {
        Style { indent: "  ".into(), eol: "\n".into() }
    }
}

impl Style {
    /// Guesses a file's style: its line end (the first one), its smallest
    /// indentation step (two spaces when nothing is indented).
    pub fn of(text: &str) -> Style {
        let eol = if text.contains("\r\n") { "\r\n" } else { "\n" };
        let step = text
            .lines()
            .filter(|l| !l.trim().is_empty())
            .map(|l| l.chars().take_while(|c| *c == ' ' || *c == '\t').collect::<String>())
            .filter(|w| !w.is_empty())
            .min_by_key(|w| w.len())
            .unwrap_or_else(|| "  ".into());
        Style { indent: step, eol: eol.into() }
    }
}

/// `tree` as CREATE block text, each line starting with `base` (the
/// indentation of where it goes).
pub fn write_create(tree: &Subtree, base: &str, style: &Style) -> String {
    let mut out = String::new();
    write_into(&mut out, tree, base, style);
    out
}

fn write_into(out: &mut String, tree: &Subtree, base: &str, style: &Style) {
    let inner = format!("{base}{}", style.indent);
    out.push_str(&format!("{base}CREATE {} AS {}{}", tree.name, tree.type_written, style.eol));
    for item in &tree.body {
        match item {
            SubItem::Prop(p) => out.push_str(&format!("{inner}{} = {}{}", p.name, p.value, style.eol)),
            SubItem::Child(c) => write_into(out, c, &inner, style),
            SubItem::Code(code) => {
                for line in code.lines() {
                    out.push_str(&format!("{inner}{}{}", line.trim_start(), style.eol));
                }
            }
        }
    }
    out.push_str(&format!("{base}END CREATE{}", style.eol));
}

/// A new component of `type_name` (any of its names) at `rect`: named as
/// Delphi names them (`Button1`), written in the file's style
/// ([`FormDesign::names`]: `RButton`, or `QBUTTON` in a RapidQ-style
/// file), with its Caption (its name, where it has one) and its Left / Top
/// / Width / Height.
pub fn new_component(design: &FormDesign, type_name: &str, rect: Rect) -> Subtree {
    let comp = rapidr_lang::component(type_name);
    let written = comp.map_or_else(|| type_name.to_ascii_uppercase(), |c| c.name_in(design.names()));
    let name = design.new_name(&written);
    let mut props: Vec<(&str, String)> = Vec::new();
    if comp.is_some_and(|c| c.property("Caption").is_some()) {
        props.push(("Caption", super::value::write_str(&name)));
    }
    let visual = comp.is_none_or(|c| c.visual);
    if visual {
        props.push(("Left", rect.left.to_string()));
        props.push(("Top", rect.top.to_string()));
        props.push(("Width", rect.width.to_string()));
        props.push(("Height", rect.height.to_string()));
    }
    Subtree::new(&name, &written, &props)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn blocks_in_the_files_style() {
        let d = FormDesign::new("Form1", "QFORM");
        let t = new_component(&d, "RBUTTON", Rect::new(8, 16, 75, 25));
        assert_eq!(t.type_written, "QBUTTON", "RapidQ's name in a form written with RapidQ's names");
        let r = FormDesign::new("Form1", "RForm");
        assert_eq!(new_component(&r, "QBUTTON", Rect::new(8, 16, 75, 25)).type_written, "RButton", "RapidR's names otherwise");
        let mut said = FormDesign::new("Form1", "QFORM");
        said.set_names(rapidr_lang::NameStyle::RapidR);
        assert_eq!(new_component(&said, "QLABEL", Rect::new(0, 0, 10, 10)).type_written, "RLabel", "the file's style first");
        let text = write_create(&t, "  ", &Style { indent: "    ".into(), eol: "\r\n".into() });
        assert_eq!(text, "  CREATE Button1 AS QBUTTON\r\n      Caption = \"Button1\"\r\n      Left = 8\r\n      Top = 16\r\n      Width = 75\r\n      Height = 25\r\n  END CREATE\r\n");
        let plot = new_component(&d, "RPLOT", Rect::new(0, 0, 600, 400));
        assert_eq!((plot.name.as_str(), plot.type_written.as_str()), ("Plot1", "RPLOT"));
        // (named after the registry's mixed-case spelling, as Delphi and VB)
        for (ty, name) in [("QCHECKBOX", "CheckBox1"), ("RSTRINGGRID", "StringGrid1"), ("QCOMBOBOX", "ComboBox1"), ("QRICHEDIT", "RichEdit1"), ("QDXSCREEN", "DXScreen1")] {
            assert_eq!(new_component(&d, ty, Rect::new(0, 0, 10, 10)).name, name);
        }
        let timer = new_component(&d, "QTIMER", Rect::new(0, 0, 0, 0));
        assert!(timer.prop("Left").is_none(), "not visual: no geometry");
        assert_eq!(Style::of("a\r\n\tb\r\n").indent, "\t");
        assert_eq!(Style::of("x\n").indent, "  ");
    }
}
