//! Event handlers made from the designer (docs/ide-plan.md I4, "Events";
//! docs/studio-wow.md DES-6 and INS-3): a double click on a component, or
//! on an event's row in the inspector, gives the event a SUB — the one it
//! already runs, or a new one written into the file — in **one undo step**
//! with the binding (`OnClick = Button1Click` in the CREATE block).
//!
//! Where a new SUB goes, so that the program is still RapidQ (RC.EXE wants
//! a SUB known before the CREATE that names it — checked in
//! `tests/handlers.rs` against RC.EXE's own verdicts):
//! * the file declares its SUBs before the form (`DECLARE SUB …` lines):
//!   a `DECLARE SUB` after the last of them, and the SUB at the end of the
//!   file — the file's own layout kept;
//! * otherwise the SUB just before the form's CREATE block.
//!
//! The SUB's parameters are the registry's for that event
//! (`rapidr_lang`: `OnKeyDown` → `Key AS WORD, Shift AS INTEGER`), its
//! name the component's and the event's without "On" (`Button1Click`, as
//! Delphi names them), made unique when a FUNCTION or another SUB of
//! another meaning has it. A SUB of that name already in the file is bound
//! as it is, never written twice.

use crate::{line_end, Document, TextPatch};

/// What [`Document::create_handler`] did.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Handler {
    /// The SUB the event runs now.
    pub sub: String,
    /// The event's name as the registry spells it (`OnClick`).
    pub event: String,
    /// The line (from 0) where the caret goes: the new SUB's empty body
    /// line, or an existing SUB's header; `None` when the SUB isn't in this
    /// file (an `$INCLUDE`d one).
    pub line: Option<usize>,
    /// The text edits made, in order (each in the text the ones before
    /// left); empty when the handler existed.
    pub patches: Vec<TextPatch>,
    /// A new SUB was written.
    pub created: bool,
}

/// The default event of a component type (any case, Q or R name).
pub fn default_event(type_name: &str) -> Option<&'static str> {
    let c = rapidr_lang::component(&rapidr_value::designer::model::canonical_type(type_name))?;
    c.default_event.or_else(|| c.events.first().map(|e| e.name))
}

/// An event's parameters as a SUB header writes them
/// (`Key AS WORD, Shift AS INTEGER`; `BYREF Action AS INTEGER`).
pub fn params_text(ev: &rapidr_lang::Event) -> String {
    params_text_in(ev, rapidr_lang::NameStyle::RapidR)
}

/// [`params_text`] in a file's style: a component type under RapidR's name
/// (`R AS RRect`), or RapidQ's in a file written with RapidQ's names
/// (`R AS QRECT`) — never mixed (R-NAMES).
pub fn params_text_in(ev: &rapidr_lang::Event, style: rapidr_lang::NameStyle) -> String {
    ev.params
        .iter()
        .map(|p| {
            let mut s = String::new();
            if p.byref {
                s.push_str("BYREF ");
            }
            s.push_str(p.name);
            if !p.ty.is_empty() {
                s.push_str(" AS ");
                match rapidr_lang::component(p.ty) {
                    Some(c) => s.push_str(&c.name_in(style)),
                    None => s.push_str(p.ty),
                }
            }
            s
        })
        .collect::<Vec<_>>()
        .join(", ")
}

/// The header of SUB `name` for `ev` (`SUB Button1Click`,
/// `SUB Edit1KeyDown (Key AS WORD, Shift AS INTEGER)`).
fn header(keyword: &str, name: &str, ev: &rapidr_lang::Event, style: rapidr_lang::NameStyle) -> String {
    let p = params_text_in(ev, style);
    if p.is_empty() {
        format!("{keyword} {name}")
    } else {
        format!("{keyword} {name} ({p})")
    }
}

/// The line (from 0) where routine `name` is defined (`keyword`: SUB or
/// FUNCTION; a DECLARE line isn't one).
fn routine_line(text: &str, keyword: &str, name: &str) -> Option<usize> {
    text.lines().position(|l| {
        let t = l.trim_start();
        let Some(rest) = strip_word(t, keyword) else { return false };
        let rest = rest.trim_start();
        rest.len() >= name.len() && rest[..name.len()].eq_ignore_ascii_case(name) && !rest[name.len()..].starts_with(|c: char| c.is_ascii_alphanumeric() || c == '_')
    })
}

/// `t` after the word `w` (any case) and a blank, if it starts with them.
fn strip_word<'a>(t: &'a str, w: &str) -> Option<&'a str> {
    let head = t.get(..w.len())?;
    let rest = &t[w.len()..];
    (head.eq_ignore_ascii_case(w) && rest.starts_with([' ', '\t'])).then_some(rest)
}

/// Whether `text` has a line declaring a SUB or FUNCTION before byte
/// `before`: the end of the last such line.
fn last_declare_before(text: &str, before: usize) -> Option<usize> {
    let mut at = 0;
    let mut found = None;
    while at < before {
        let end = line_end(text, at);
        let t = text[at..end].trim_start();
        if let Some(rest) = strip_word(t, "DECLARE") {
            let rest = rest.trim_start();
            if strip_word(rest, "SUB").is_some() || strip_word(rest, "FUNCTION").is_some() {
                found = Some(end);
            }
        }
        if end == at {
            break;
        }
        at = end;
    }
    found
}

impl Document {
    /// The SUB event `event` of component `component` (the form itself
    /// too) of form `form` runs: the one bound, else a new one written into
    /// the file and bound, in one undo step. `event` "" is the component's
    /// default event. `Err` says why not (no such component or event).
    pub fn create_handler(&mut self, form: usize, component: &str, event: &str) -> Result<Handler, String> {
        let f = self.forms.get(form).ok_or_else(|| format!("No form {form}"))?;
        let d = &f.designer.design;
        let id = d.find(component).ok_or_else(|| format!("No component {component}"))?;
        let node = d.node(id).ok_or_else(|| format!("No component {component}"))?;
        let comp = node.component().ok_or_else(|| format!("{} isn't a component RapidR knows", node.type_written))?;
        let event = if event.trim().is_empty() { default_event(&node.type_written).unwrap_or("").to_string() } else { event.trim().to_string() };
        let ev = comp.event(&event).ok_or_else(|| format!("{} has no event {event}", node.type_written))?;
        let bound = node.prop(ev.name).map(|v| v.trim().to_string()).filter(|v| !v.is_empty());
        let comp_name = node.name.clone();
        if let Some(sub) = bound {
            let line = routine_line(&self.text, "SUB", &sub).map(|l| l + 1);
            return Ok(Handler { sub, event: ev.name.to_string(), line, patches: Vec::new(), created: false });
        }

        // (the name: the component's and the event's; a SUB of that name
        // already written is bound as it is; a FUNCTION's name is taken)
        let base = format!("{}{}", comp_name, ev.name.strip_prefix("On").unwrap_or(ev.name));
        let mut sub = base.clone();
        let mut k = 2;
        while routine_line(&self.text, "FUNCTION", &sub).is_some() || d.find(&sub).is_some() {
            sub = format!("{base}{k}");
            k += 1;
        }
        let exists = routine_line(&self.text, "SUB", &sub).is_some();

        // (1) the binding, through the designer: the smallest edit
        let saved = self.forms[form].designer.selection.clone();
        {
            let designer = &mut self.forms[form].designer;
            designer.selection.set(id);
            let r = designer.set_property(ev.name, Some(&sub));
            designer.selection = saved.clone();
            r.map_err(|e| format!("{e:?}"))?;
        }
        let undo_before = self.undo.len();
        let mut patches = self.sync();
        if let Some(f) = self.forms.get_mut(form) {
            f.designer.selection = saved;
            f.designer.selection.retain(&f.synced);
        }
        if exists {
            let line = routine_line(&self.text, "SUB", &sub).map(|l| l + 1);
            return Ok(Handler { sub, event: ev.name.to_string(), line, patches, created: false });
        }

        // (2) the SUB itself (and its DECLARE), in the same undo step, its
        // parameters' types in the file's style
        let style = self.forms.get(form).map(|f| f.synced.names()).unwrap_or_default();
        let eol = self.style.eol.clone();
        let indent = self.style.indent.clone();
        let form_start = self.forms.get(form).and_then(|f| f.spans(f.synced.root())).map(|s| s.lines.0).ok_or("The form's CREATE block is gone")?;
        let mut inserts: Vec<TextPatch> = Vec::new();
        match last_declare_before(&self.text, form_start) {
            Some(after_declares) => {
                // (the end of the file first: the DECLARE's place, before
                // it, stays where it is; a blank line before the SUB)
                let end = self.text.len();
                let mut body = String::new();
                if !self.text.ends_with('\n') {
                    body.push_str(&eol);
                }
                if !(self.text.ends_with("\n\n") || self.text.ends_with("\n\r\n")) {
                    body.push_str(&eol);
                }
                body.push_str(&format!("{h}{eol}{indent}{eol}END SUB{eol}", h = header("SUB", &sub, ev, style)));
                inserts.push(TextPatch { start: end, end, insert: body });
                let mut decl = String::new();
                if !self.text[..after_declares].ends_with('\n') {
                    decl.push_str(&eol);
                }
                decl.push_str(&header("DECLARE SUB", &sub, ev, style));
                decl.push_str(&eol);
                inserts.push(TextPatch { start: after_declares, end: after_declares, insert: decl });
            }
            None => {
                let body = format!("{h}{eol}{indent}{eol}END SUB{eol}{eol}", h = header("SUB", &sub, ev, style));
                inserts.push(TextPatch { start: form_start, end: form_start, insert: body });
            }
        }
        let mut record = Vec::new();
        for p in &inserts {
            let removed = self.text[p.start..p.end].to_string();
            self.text.replace_range(p.start..p.end, &p.insert);
            record.push((p.clone(), removed));
        }
        if self.undo.len() > undo_before {
            if let Some(last) = self.undo.last_mut() {
                last.extend(record);
            }
        } else {
            self.undo.push(record);
            self.redo.clear();
        }
        let prev = self.previous();
        self.reread(&prev);
        patches.extend(inserts);
        // (the caret: the new SUB's empty body line)
        let line = routine_line(&self.text, "SUB", &sub).map(|l| l + 1);
        Ok(Handler { sub, event: ev.name.to_string(), line, patches, created: true })
    }
}
