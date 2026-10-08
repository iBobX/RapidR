//! How the designer writes CREATE blocks it makes (a component added from
//! the toolbox, pasted, duplicated): `CREATE Name AS Type`, one property per
//! line indented one step in, nested CREATEs, `END CREATE` — in the file's
//! own indentation and line ends. Names follow docs/q-and-r-components.md:
//! a new RapidQ component under its Q name (QBUTTON), a RapidR-only one
//! under its R name (RPLOT).

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

/// Whether a form written with type `form_type` uses RapidQ's names
/// (`QFORM`, `QForm`): its new components are then written as RapidQ
/// writes them; a form written with RapidR's (`RForm`) gets RapidR's.
pub fn uses_rapidq_names(form_type: &str) -> bool {
    form_type.trim_start().starts_with(['Q', 'q'])
}

/// Whether a new component of this type takes its caption's size
/// (`crate::autosize`: a label, AutoSize on by default).
fn autosizes(comp: Option<&rapidr_lang::Component>) -> bool {
    comp.is_some_and(|c| c.name == "RLABEL")
}

/// Whether a source file is written with RapidQ's names, so a form added to
/// it (or a new file of its program) is too (`rapidr_project::forms`).
pub fn file_uses_rapidq_names(text: &str, path: &str) -> bool {
    rapidr_project::forms::uses_rapidq_names(text, path)
}

/// How a component of `type_name` (any of its names) is written in a form
/// of `form_type`, in the file's own style (no mixing): RapidR's name
/// (`RButton`, `RPlot`: the registry's spelling) in a RapidR form;
/// RapidQ's (`QBUTTON`) in a RapidQ form for a component RapidQ has, and
/// RapidR's own name for one it hasn't (`RPLOT`).
pub fn type_written_for(form_type: &str, type_name: &str) -> String {
    let Some(c) = rapidr_lang::component(type_name) else { return type_name.to_ascii_uppercase() };
    if uses_rapidq_names(form_type) {
        c.written_name().to_string()
    } else {
        c.pretty(c.name)
    }
}

/// A new component of `type_name` (any of its names) at `rect`: named as
/// Delphi names them (`Button1`), its type written in the form's style
/// ([`type_written_for`]), with its Caption (its name, where it has one)
/// and its Left / Top / Width / Height.
pub fn new_component(design: &FormDesign, type_name: &str, rect: Rect) -> Subtree {
    let comp = rapidr_lang::component(type_name);
    let form_type = design.node(design.root()).map_or("", |n| n.type_written.as_str());
    let written = type_written_for(form_type, type_name);
    let name = design.new_name(&written);
    let mut props: Vec<(&str, String)> = Vec::new();
    if comp.is_some_and(|c| c.property("Caption").is_some()) {
        props.push(("Caption", super::value::write_str(&name)));
    }
    let visual = comp.is_none_or(|c| c.visual);
    if visual {
        props.push(("Left", rect.left.to_string()));
        props.push(("Top", rect.top.to_string()));
        // (a label sizes itself to its caption — RapidQ's AutoSize, on until
        // the program turns it off: a Width written after its Caption would
        // stick and clip a longer one, so it gets none until it is resized)
        if !(autosizes(comp) && rect.width == crate::layout::default_size("RLABEL").map_or(rect.width, |s| s.0)) {
            props.push(("Width", rect.width.to_string()));
            props.push(("Height", rect.height.to_string()));
        }
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
        assert_eq!(t.type_written, "QBUTTON", "RapidQ's name for a RapidQ component");
        let text = write_create(&t, "  ", &Style { indent: "    ".into(), eol: "\r\n".into() });
        assert_eq!(text, "  CREATE Button1 AS QBUTTON\r\n      Caption = \"Button1\"\r\n      Left = 8\r\n      Top = 16\r\n      Width = 75\r\n      Height = 25\r\n  END CREATE\r\n");
        let plot = new_component(&d, "RPLOT", Rect::new(0, 0, 600, 400));
        assert_eq!((plot.name.as_str(), plot.type_written.as_str()), ("Plot1", "RPLOT"));
        // a RapidR form: RapidR's names, as the registry spells them
        let r = FormDesign::new("Form1", "RForm");
        for (ty, written) in [("QBUTTON", "RButton"), ("RLABEL", "RLabel"), ("qedit", "REdit"), ("RPLOT", "RPlot"), ("QSTRINGGRID", "RStringGrid")] {
            assert_eq!(new_component(&r, ty, Rect::new(0, 0, 10, 10)).type_written, written, "{ty}");
        }
        assert_eq!(new_component(&r, "QBUTTON", Rect::new(0, 0, 10, 10)).name, "Button1");
        assert!(file_uses_rapidq_names("' notes\nCREATE Form AS QFORM\nEND CREATE\n", "a.rr"));
        assert!(!file_uses_rapidq_names("CREATE Form1 AS RForm\nEND CREATE\n", "a.bas"));
        assert!(file_uses_rapidq_names("PRINT 1\n", "old.bas") && !file_uses_rapidq_names("PRINT 1\n", "new.rr"));
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
