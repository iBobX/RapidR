//! Caption clipping audit for the RAPIDR theme (lane L-THEME): how many
//! captions and texts of real programs would clip if a program's default
//! font (MS Sans Serif 8, today drawn as "RapidR Sans" at 11 px with MS Sans
//! Serif's metrics) were drawn in **Inter** at 11, 12 or 13 px instead, the
//! components keeping RapidQ's sizes.
//!
//! ```text
//! cargo run --release -p rapidr-designer --example caption_audit [-- --list]
//! ```
//!
//! `--list` also prints every text that clips in some setting; `--find
//! TEXT` prints every measured text whose file or component name contains
//! TEXT (to check a number by hand).
//!
//! **Programs.** Every `.bas`, `.rr`, `.inc` and `.rqb` file of the repo's
//! `examples/` and of the RapidQ corpus (`$RAPIDQ_DIR/examples`, default
//! `~/Downloads/Rapidq/examples`, read only; its `include/` on the include
//! path), opened in the designer as `tests/corpus.rs` does. Each component is
//! where the designer's layout puts it (`designer::layout::Layout`: the
//! type's default size, Align, the CREATE block's assignments).
//!
//! **Fonts.** A component's font is its CREATE block's `Font = X`,
//! `Font.Name = …`, `Font.Size = …`, `Font.AddStyles(fsBold)` (in order),
//! then `Comp.Font = X` / `Comp.Font.Name = …` found in the program's code;
//! what it leaves unset is its parent's (ParentFont, as
//! `objects::inherited_font_prop`), up to the form, else MS Sans Serif 8. A
//! QFONT `X` is a `CREATE X AS QFONT` block or a `DIM X AS QFONT` with
//! `X.Name = …`, `X.Size = …`, `X.AddStyles …` anywhere in the program (its
//! includes too). Only literal values (and integer CONSTs) are read: a font
//! the audit can't resolve is counted as such and its texts skipped. A text
//! is measured when its font's name is the default face's
//! ([`is_default_face`]: MS Sans Serif, Microsoft Sans Serif, MS Shell Dlg,
//! Sans Serif, empty).
//!
//! **Texts.** A literal `Caption` (`Text` for QEDIT / QCOMBOBOX), `&`
//! mnemonics stripped from captions (`&&` is one `&`), the widest line
//! measured with `objects::text::text_size`: today in the font as written,
//! under the classic theme (`theme::CLASSIC`: 8 pt → RapidR Sans 11 px,
//! MS Sans Serif's metrics), and in Inter (Inter-SemiBold when bold) at
//! the program's pixel size × 11/11, 12/11 and 13/11, rounded (8 pt: 11, 12,
//! 13 px).
//!
//! **Room** (approximate Windows / RapidQ insets; a text clips when it is
//! wider than its room, or its line height taller than its height room):
//!
//! | component                      | width room   | height room |
//! |--------------------------------|--------------|-------------|
//! | QBUTTON, QCOOLBTN, QOVALBTN    | Width − 8    | Height − 4  |
//! | QLABEL, AutoSize off           | Width        | Height      |
//! | QLABEL, WordWrap (AutoSize off)| longest word ≤ Width; wrapped lines × line height ≤ Height |
//! | QLABEL, AutoSize on            | never clips: grows (its growth is reported) |
//! | QCHECKBOX, QRADIOBUTTON        | Width − 17   | Height      |
//! | QGROUPBOX, QRADIOGROUP caption | Width − 16   | (not checked) |
//! | QPANEL caption                 | Width − 4    | Height − 4  |
//! | QEDIT text                     | Width − 6    | Height − 4  |
//! | QCOMBOBOX text                 | Width − 22   | (not checked: Windows sizes a combo to its font) |
//!
//! A label has no border, so no inset; a check box's text is centred
//! beside its box. Tabs, list items and form captions are not audited.

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use rapidr_ast::{Expression, LiteralValue, Statement};
use rapidr_designer::Document;
use rapidr_preprocessor::PreprocessOptions;
use rapidr_value::designer::layout::Layout;
use rapidr_value::designer::model::{canonical_type, FormDesign, Item, NodeId};
use rapidr_value::designer::value::PropValue;
use rapidr_value::objects::font::Font;
use rapidr_value::objects::text::{is_default_face, text_size};

/// The four settings compared: today's face, then Inter at 11, 12, 13 px
/// for an 11 px (8 pt) program.
const SETTINGS: [&str; 4] = ["today", "Inter 11", "Inter 12", "Inter 13"];
const INTER_PX: [i64; 3] = [11, 12, 13];

// ---------------------------------------------------------------------------
// Fonts

/// A font as far as a program says it: unset fields come from the parent.
#[derive(Clone, Debug, Default)]
struct PartialFont {
    name: Option<String>,
    size: Option<i64>,
    bold: Option<bool>,
    /// A value the audit can't read (a variable, an expression).
    unresolved: bool,
}

impl PartialFont {
    /// `Font = X`: every field becomes X's (a QFONT's unset fields are a new
    /// QFONT's: MS Sans Serif 8, regular).
    fn assign(&mut self, f: &PartialFont) {
        self.name = Some(f.name.clone().unwrap_or_else(|| "MS Sans Serif".into()));
        self.size = Some(f.size.unwrap_or(8));
        self.bold = Some(f.bold.unwrap_or(false));
        self.unresolved |= f.unresolved;
    }

    /// One font property (`name`, `size`, `bold`, `addstyles`, `delstyles`)
    /// set to `v`.
    fn set(&mut self, prop: &str, v: Lit) {
        match (prop, v) {
            ("name", Lit::Str(s)) => self.name = Some(s),
            ("size", Lit::Int(n)) => self.size = Some(n),
            ("bold", Lit::Int(n)) => self.bold = Some(n != 0),
            ("addstyles", Lit::Int(0)) => self.bold = Some(true),
            ("delstyles", Lit::Int(0)) => self.bold = Some(false),
            ("addstyles" | "delstyles", Lit::Int(_)) => {}
            ("name" | "size" | "bold" | "addstyles" | "delstyles", _) => self.unresolved = true,
            _ => {}
        }
    }

    /// Fills unset fields from `parent`.
    fn under(&self, parent: &PartialFont) -> PartialFont {
        PartialFont {
            name: self.name.clone().or_else(|| parent.name.clone()),
            size: self.size.or(parent.size),
            bold: self.bold.or(parent.bold),
            unresolved: self.unresolved || parent.unresolved,
        }
    }

    fn font(&self) -> Font {
        let name = self.name.clone().filter(|n| !n.trim().is_empty()).unwrap_or_else(|| "MS Sans Serif".into());
        let size = self.size.filter(|s| *s != 0).unwrap_or(8);
        Font { name, size, color: 0, styles: u8::from(self.bold.unwrap_or(false)) }
    }
}

/// A value the audit reads: a literal, a CONST, or a name.
#[derive(Clone, Debug)]
enum Lit {
    Str(String),
    Int(i64),
    Name(String),
    Other,
}

/// What a program says about fonts outside the components' CREATE blocks.
#[derive(Default)]
struct ProgramFonts {
    /// QFONTs by lower-case name.
    fonts: HashMap<String, PartialFont>,
    /// `Comp.Font = X` / `Comp.Font.Name = …` in code, by lower-case
    /// component name, in order.
    code_ops: HashMap<String, Vec<FontOp>>,
    consts: HashMap<String, i64>,
}

#[derive(Clone, Debug)]
enum FontOp {
    /// `Font = X`.
    Assign(Lit),
    /// `Font.Size = 10`, `Font.AddStyles fsBold`.
    Set(String, Lit),
}

impl ProgramFonts {
    fn of(doc: &Document, path: &Path, options: &PreprocessOptions) -> ProgramFonts {
        let base = path.parent().unwrap_or(Path::new("."));
        let parse = rapidr_parser::parse_source_for_tools(doc.text(), base, Some(path.to_path_buf()), options.clone());
        let mut p = ProgramFonts::default();
        p.consts.insert("fsbold".into(), 0);
        p.consts.insert("true".into(), -1);
        p.consts.insert("false".into(), 0);
        for s in &parse.program.statements {
            if let Statement::Const(c) = s {
                if let Some(v) = int_of(&c.value, &p.consts) {
                    p.consts.insert(c.name.to_ascii_lowercase(), v);
                }
            }
        }
        // (QFONT names first, so `X.Name = …` knows X is one)
        let mut qfonts = Vec::new();
        collect_qfont_dims(&parse.program.statements, &mut qfonts);
        for q in qfonts {
            p.fonts.entry(q).or_default();
        }
        p.walk(&parse.program.statements);
        p
    }

    fn lit(&self, e: &Expression) -> Lit {
        match e {
            Expression::Literal(l) => match &l.value {
                LiteralValue::String(s) => Lit::Str(s.clone()),
                LiteralValue::Integer(n) => Lit::Int(*n),
                LiteralValue::Float(f) if f.fract() == 0.0 => Lit::Int(*f as i64),
                LiteralValue::Float(_) => Lit::Other,
            },
            Expression::Identifier(id) => match self.consts.get(&id.name.to_ascii_lowercase()) {
                Some(v) => Lit::Int(*v),
                None => Lit::Name(id.name.to_ascii_lowercase()),
            },
            _ => match int_of(e, &self.consts) {
                Some(v) => Lit::Int(v),
                None => Lit::Other,
            },
        }
    }

    fn walk(&mut self, stmts: &[Statement]) {
        for s in stmts {
            match s {
                Statement::Create(c) if canonical_type(&c.type_name) == canonical_type("QFONT") => {
                    let mut f = PartialFont::default();
                    for b in &c.body {
                        match b {
                            Statement::Assignment(a) => {
                                if let Expression::Identifier(id) = &a.target {
                                    f.set(&id.name.to_ascii_lowercase(), self.lit(&a.value));
                                }
                            }
                            Statement::Call(call) => {
                                if let Expression::Identifier(id) = &call.callee {
                                    if let Some(arg) = call.args.first() {
                                        f.set(&id.name.to_ascii_lowercase(), self.lit(arg));
                                    }
                                }
                            }
                            _ => {}
                        }
                    }
                    self.fonts.insert(c.name.to_ascii_lowercase(), f);
                }
                // (a component's own block is read from the designer; only
                // QFONTs nested in it are looked for)
                Statement::Create(c) => self.walk_creates(&c.body),
                Statement::Assignment(a) => {
                    let v = self.lit(&a.value);
                    self.code_assignment(&a.target, v);
                }
                Statement::Call(call) => {
                    // X.AddStyles fsBold / Comp.Font.AddStyles(fsBold)
                    if let (Expression::MemberAccess(_), Some(arg)) = (&call.callee, call.args.first()) {
                        let v = self.lit(arg);
                        self.code_assignment(&call.callee, v);
                    }
                }
                Statement::Subroutine(x) => self.walk(&x.body),
                Statement::Function(x) => self.walk(&x.body),
                Statement::If(x) => {
                    self.walk(&x.then_body);
                    for b in &x.elseif_branches {
                        self.walk(&b.body);
                    }
                    self.walk(&x.else_body);
                }
                Statement::For(x) => self.walk(&x.body),
                Statement::While(x) => self.walk(&x.body),
                Statement::DoLoop(x) => self.walk(&x.body),
                Statement::SelectCase(x) => {
                    for b in &x.cases {
                        self.walk(&b.body);
                    }
                    self.walk(&x.case_else);
                }
                _ => {}
            }
        }
    }

    fn walk_creates(&mut self, stmts: &[Statement]) {
        for s in stmts {
            if let Statement::Create(_) = s {
                self.walk(std::slice::from_ref(s));
            }
        }
    }

    /// `X.Name = v` (X a QFONT), `Comp.Font = v`, `Comp.Font.Name = v`.
    fn code_assignment(&mut self, target: &Expression, v: Lit) {
        let Expression::MemberAccess(m) = target else { return };
        let member = m.member.to_ascii_lowercase();
        match &*m.object {
            Expression::Identifier(id) => {
                let obj = id.name.to_ascii_lowercase();
                if let Some(f) = self.fonts.get_mut(&obj) {
                    f.set(&member, v);
                } else if member == "font" {
                    self.code_ops.entry(obj).or_default().push(FontOp::Assign(v));
                }
            }
            Expression::MemberAccess(inner) if inner.member.eq_ignore_ascii_case("font") => {
                if let Expression::Identifier(id) = &*inner.object {
                    self.code_ops.entry(id.name.to_ascii_lowercase()).or_default().push(FontOp::Set(member, v));
                }
            }
            _ => {}
        }
    }

    /// `Font = v`: the QFONT it names.
    fn apply(&self, f: &mut PartialFont, op: &FontOp) {
        match op {
            FontOp::Assign(Lit::Name(n)) => match self.fonts.get(n) {
                Some(q) => f.assign(q),
                None => f.unresolved = true,
            },
            FontOp::Assign(_) => f.unresolved = true,
            FontOp::Set(p, v) => f.set(p, v.clone()),
        }
    }
}

fn collect_qfont_dims(stmts: &[Statement], out: &mut Vec<String>) {
    for s in stmts {
        match s {
            Statement::Dim(d) if canonical_type(&d.type_name) == canonical_type("QFONT") => {
                out.extend(d.declarators.iter().map(|v| v.name.to_ascii_lowercase()));
            }
            Statement::Subroutine(x) => collect_qfont_dims(&x.body, out),
            Statement::Function(x) => collect_qfont_dims(&x.body, out),
            Statement::If(x) => {
                collect_qfont_dims(&x.then_body, out);
                collect_qfont_dims(&x.else_body, out);
            }
            _ => {}
        }
    }
}

fn int_of(e: &Expression, consts: &HashMap<String, i64>) -> Option<i64> {
    use rapidr_ast::{BinaryOperator as B, UnaryOperator as U};
    match e {
        Expression::Literal(l) => match &l.value {
            LiteralValue::Integer(n) => Some(*n),
            LiteralValue::Float(f) if f.fract() == 0.0 => Some(*f as i64),
            _ => None,
        },
        Expression::Identifier(id) => consts.get(&id.name.to_ascii_lowercase()).copied(),
        Expression::Unary(u) if u.operator == U::Negate => int_of(&u.operand, consts).map(|v| -v),
        Expression::Binary(b) => {
            let (l, r) = (int_of(&b.left, consts)?, int_of(&b.right, consts)?);
            match b.operator {
                B::Add => Some(l + r),
                B::Subtract => Some(l - r),
                B::Multiply => Some(l * r),
                B::Or => Some(l | r),
                _ => None,
            }
        }
        _ => None,
    }
}

/// A component's own font, as its CREATE block (then the code) sets it.
fn own_font(design: &FormDesign, id: NodeId, prog: &ProgramFonts) -> PartialFont {
    let mut f = PartialFont::default();
    let Some(node) = design.node(id) else { return f };
    let lit = |text: &str| match design.read_value(text) {
        PropValue::Str(s) => Lit::Str(s),
        PropValue::Number(n) if n.fract() == 0.0 => Lit::Int(n as i64),
        PropValue::Name(n) => match prog.consts.get(&n.to_ascii_lowercase()) {
            Some(v) => Lit::Int(*v),
            None => Lit::Name(n.to_ascii_lowercase()),
        },
        _ => Lit::Other,
    };
    for item in &node.body {
        match item {
            Item::Prop(p) => {
                let key = p.name.to_ascii_lowercase();
                if key == "font" {
                    prog.apply(&mut f, &FontOp::Assign(lit(&p.value)));
                } else if let Some(sub) = key.strip_prefix("font.").or_else(|| key.strip_prefix("font").filter(|s| ["name", "size", "bold"].contains(s))) {
                    f.set(sub, lit(&p.value));
                }
            }
            Item::Code(c) => {
                // Font.AddStyles(fsBold) / Font.AddStyles fsBold
                let l = c.to_ascii_lowercase().replace(' ', "");
                for op in ["addstyles", "delstyles"] {
                    if let Some(arg) = l.strip_prefix(&format!("font.{op}")) {
                        let arg = arg.trim_matches(|c| c == '(' || c == ')');
                        let v = if arg == "fsbold" || arg == "0" { Lit::Int(0) } else { Lit::Other };
                        f.set(op, v);
                    }
                }
            }
            Item::Child(_) => {}
        }
    }
    if let Some(ops) = prog.code_ops.get(&node.name.to_ascii_lowercase()) {
        for op in ops {
            prog.apply(&mut f, op);
        }
    }
    f
}

/// The font a component draws in: its own, unset fields its parents'.
fn resolved_font(design: &FormDesign, id: NodeId, prog: &ProgramFonts) -> PartialFont {
    let own = own_font(design, id, prog);
    match design.parent(id) {
        Some(p) => own.under(&resolved_font(design, p, prog)),
        None => own,
    }
}

// ---------------------------------------------------------------------------
// Components

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Kind {
    Button,
    Label,
    Check,
    Group,
    Panel,
    Edit,
    Combo,
}

/// The audited components, by the type names RapidQ programs write.
const KINDS: [(&str, Kind); 11] = [
    ("QBUTTON", Kind::Button),
    ("QCOOLBTN", Kind::Button),
    ("QOVALBTN", Kind::Button),
    ("QLABEL", Kind::Label),
    ("QCHECKBOX", Kind::Check),
    ("QRADIOBUTTON", Kind::Check),
    ("QGROUPBOX", Kind::Group),
    ("QRADIOGROUP", Kind::Group),
    ("QPANEL", Kind::Panel),
    ("QEDIT", Kind::Edit),
    ("QCOMBOBOX", Kind::Combo),
];

fn kind_of(canonical: &str) -> Option<(&'static str, Kind)> {
    KINDS.iter().find(|(q, _)| canonical_type(q) == canonical).copied()
}


/// A caption as drawn: `&x` is x (a mnemonic), `&&` is `&`.
fn strip_mnemonics(s: &str) -> String {
    let mut out = String::new();
    let mut chars = s.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '&' {
            if chars.peek() == Some(&'&') {
                chars.next();
                out.push('&');
            }
            continue;
        }
        out.push(c);
    }
    out
}

/// The widest line's width and the line height.
fn measure(text: &str, font: &Font) -> (i64, i64) {
    let lines: Vec<&str> = text.split(['\r', '\n']).collect();
    let w = lines.iter().map(|l| text_size(l, font).0).max().unwrap_or(0);
    (w, text_size("Xg", font).1)
}

/// Word-wrapped to `width` (greedy, at spaces, as DrawText's
/// DT_WORDBREAK): the number of lines and the widest word.
fn wrap(text: &str, font: &Font, width: i64) -> (i64, i64) {
    let mut lines = 0;
    let mut widest_word = 0;
    for para in text.split(['\r', '\n']) {
        lines += 1;
        let mut line = String::new();
        for word in para.split(' ') {
            widest_word = widest_word.max(text_size(word, font).0);
            let candidate = if line.is_empty() { word.to_string() } else { format!("{line} {word}") };
            if !line.is_empty() && text_size(&candidate, font).0 > width {
                lines += 1;
                line = word.to_string();
            } else {
                line = candidate;
            }
        }
    }
    (lines, widest_word)
}

/// The fonts of the four settings for a program font.
fn settings_fonts(today: &Font) -> [Font; 4] {
    let px = today.pixel_size();
    let inter = |n: i64| Font { name: "Inter".into(), size: -((px * n + 5) / 11).max(1), color: 0, styles: today.styles };
    [today.clone(), inter(INTER_PX[0]), inter(INTER_PX[1]), inter(INTER_PX[2])]
}

/// One measured text.
#[derive(Clone, Debug)]
struct Row {
    file: String,
    comp: String,
    ty: &'static str,
    text: String,
    font: String,
    /// The room its width has (None for an AutoSize label: it grows).
    room_w: Option<i64>,
    room_h: Option<i64>,
    width: [i64; 4],
    height: [i64; 4],
    hclip: [bool; 4],
    vclip: [bool; 4],
}

impl Row {
    fn overflow(&self, s: usize) -> i64 {
        self.room_w.map_or(0, |r| self.width[s] - r)
    }
}

/// What a split (examples/ or the corpus) counted.
#[derive(Default)]
struct Split {
    programs: usize,
    with_forms: usize,
    rows: Vec<Row>,
    /// Texts whose font the audit couldn't resolve, by component.
    unresolved_font: Vec<String>,
    /// Texts in another family (Arial, Courier …): not affected.
    other_family: usize,
    /// Captions / texts set by an expression: not measured.
    not_literal: usize,
    /// Font names seen on measured-or-skipped texts, with counts.
    families: HashMap<String, usize>,
}

fn audit_file(path: &Path, label: &str, options: &PreprocessOptions, split: &mut Split) {
    let Ok(bytes) = fs::read(path) else { return };
    split.programs += 1;
    let doc = Document::open_bytes(&bytes, Some(path), options.clone());
    if doc.forms().is_empty() {
        return;
    }
    let has_visual = doc.forms().iter().any(|f| f.designer.design.ids().iter().any(|&id| f.designer.design.node(id).is_some_and(|n| kind_of(&n.canonical).is_some())));
    if !has_visual {
        return;
    }
    split.with_forms += 1;
    let prog = ProgramFonts::of(&doc, path, options);
    for form in doc.forms() {
        let design = &form.designer.design;
        let layout = match std::panic::catch_unwind(|| Layout::of(design)) {
            Ok(l) => l,
            Err(_) => {
                eprintln!("warning: {label}: the layout of {} panicked", form.name());
                continue;
            }
        };
        for id in design.ids() {
            let Some(node) = design.node(id) else { continue };
            let Some((ty, kind)) = kind_of(&node.canonical) else { continue };
            let text_prop = if matches!(kind, Kind::Edit | Kind::Combo) { "Text" } else { "Caption" };
            let Some(raw) = node.prop(text_prop) else { continue };
            let text = match design.read_value(raw) {
                PropValue::Str(s) => s,
                _ => {
                    split.not_literal += 1;
                    continue;
                }
            };
            let text = if matches!(kind, Kind::Edit | Kind::Combo) { text } else { strip_mnemonics(&text) };
            if text.trim().is_empty() {
                continue;
            }
            let pf = resolved_font(design, id, &prog);
            let font = pf.font();
            *split.families.entry(font.name.to_ascii_lowercase()).or_default() += 1;
            if pf.unresolved {
                split.unresolved_font.push(format!("{label}: {}", node.name));
                continue;
            }
            if !is_default_face(&font.name) {
                split.other_family += 1;
                continue;
            }
            let Some(rect) = layout.rect(id) else { continue };
            let flag = |p: &str, default: bool| node.prop(p).and_then(|v| design.read_value(v).int()).map_or(default, |v| v != 0);
            let fonts = settings_fonts(&font);
            let mut row = Row {
                file: label.to_string(),
                comp: node.name.clone(),
                ty,
                text: text.clone(),
                font: format!("{} {}{}", font.name, font.size, if font.styles & 1 != 0 { " bold" } else { "" }),
                room_w: None,
                room_h: None,
                width: [0; 4],
                height: [0; 4],
                hclip: [false; 4],
                vclip: [false; 4],
            };
            let (w, h) = (rect.width, rect.height);
            let (room_w, room_h) = match kind {
                Kind::Button => (Some(w - 8), Some(h - 4)),
                Kind::Label => (Some(w), Some(h)),
                Kind::Check => (Some(w - 17), Some(h)),
                Kind::Group => (Some(w - 16), None),
                Kind::Panel => (Some(w - 4), Some(h - 4)),
                Kind::Edit => (Some(w - 6), Some(h - 4)),
                Kind::Combo => (Some(w - 22), None),
            };
            let autosize = kind == Kind::Label && flag("AutoSize", true);
            let wordwrap = kind == Kind::Label && flag("WordWrap", false);
            for (s, f) in fonts.iter().enumerate() {
                let (tw, lh) = measure(&text, f);
                row.width[s] = tw;
                row.height[s] = lh;
                if autosize {
                    continue;
                }
                if wordwrap {
                    let (lines, word) = wrap(&text, f, w);
                    row.hclip[s] = word > w;
                    row.vclip[s] = lines * lh > h;
                } else {
                    row.hclip[s] = room_w.is_some_and(|r| tw > r);
                    row.vclip[s] = room_h.is_some_and(|r| lh > r);
                }
            }
            if !autosize {
                row.room_w = room_w;
                row.room_h = room_h;
            }
            split.rows.push(row);
        }
    }
}

// ---------------------------------------------------------------------------
// Report

fn percentile(sorted: &[f64], p: f64) -> f64 {
    if sorted.is_empty() {
        return f64::NAN;
    }
    let i = ((sorted.len() - 1) as f64 * p).round() as usize;
    sorted[i]
}

fn short(s: &str, n: usize) -> String {
    if s.chars().count() <= n {
        s.to_string()
    } else {
        format!("{}…", s.chars().take(n - 1).collect::<String>())
    }
}

fn report(name: &str, split: &Split, list: bool, find: Option<&str>) {
    println!("== {name} ==");
    println!("programs read: {}   with audited components: {}", split.programs, split.with_forms);
    let fixed: Vec<&Row> = split.rows.iter().filter(|r| r.room_w.is_some()).collect();
    let auto: Vec<&Row> = split.rows.iter().filter(|r| r.room_w.is_none()).collect();
    println!(
        "texts in the default face measured: {} ({} with a fixed room, {} AutoSize labels); other families skipped: {}; unresolved fonts skipped: {}; non-literal texts skipped: {}",
        split.rows.len(),
        fixed.len(),
        auto.len(),
        split.other_family,
        split.unresolved_font.len(),
        split.not_literal
    );
    let mut fams: Vec<_> = split.families.iter().collect();
    fams.sort_by(|a, b| b.1.cmp(a.1).then(a.0.cmp(b.0)));
    println!("font names on captioned components: {}", fams.iter().take(10).map(|(n, c)| format!("{n} ×{c}")).collect::<Vec<_>>().join(", "));
    println!();
    println!("{:<10} {:>9} {:>9} {:>11} {:>9} {:>9} {:>11} {:>11}", "setting", "h-clips", "of which", "new h-clip", "v-clips", "new v", "ratio med", "ratio p95");
    println!("{:<10} {:>9} {:>9} {:>11} {:>9} {:>9} {:>11} {:>11}", "", "", "today too", "", "", "", "(vs today)", "(vs today)");
    for (s, setting) in SETTINGS.iter().enumerate() {
        let h = fixed.iter().filter(|r| r.hclip[s]).count();
        let h_both = fixed.iter().filter(|r| r.hclip[s] && r.hclip[0]).count();
        let v = fixed.iter().filter(|r| r.vclip[s]).count();
        let v_new = fixed.iter().filter(|r| r.vclip[s] && !r.vclip[0]).count();
        let mut ratios: Vec<f64> = split.rows.iter().filter(|r| r.width[0] > 0).map(|r| r.width[s] as f64 / r.width[0] as f64).collect();
        ratios.sort_by(f64::total_cmp);
        println!("{:<10} {:>9} {:>9} {:>11} {:>9} {:>9} {:>11.3} {:>11.3}", setting, h, h_both, h - h_both, v, v_new, percentile(&ratios, 0.5), percentile(&ratios, 0.95));
    }
    if !auto.is_empty() {
        println!();
        println!("AutoSize labels (never clip; they grow): growth in px vs today");
        for (s, setting) in SETTINGS.iter().enumerate().skip(1) {
            let mut g: Vec<f64> = auto.iter().map(|r| (r.width[s] - r.width[0]) as f64).collect();
            g.sort_by(f64::total_cmp);
            println!("  {:<9} median {:+.0}  p95 {:+.0}  max {:+.0}", setting, percentile(&g, 0.5), percentile(&g, 0.95), g.last().copied().unwrap_or(0.0));
        }
    }
    println!();
    // The worst new clips: fit today, overflow the most in Inter 13.
    let mut worst: Vec<&Row> = fixed.iter().copied().filter(|r| !r.hclip[0] && (r.hclip[2] || r.hclip[3])).collect();
    worst.sort_by(|a, b| b.overflow(3).cmp(&a.overflow(3)).then(a.file.cmp(&b.file)));
    println!("worst {} new horizontal clips (fit today; by Inter-13 overflow):", worst.len().min(15));
    print_rows(worst.iter().take(15).copied());
    let mut vworst: Vec<&Row> = fixed.iter().copied().filter(|r| !r.vclip[0] && (r.vclip[2] || r.vclip[3])).collect();
    vworst.sort_by(|a, b| a.room_h.cmp(&b.room_h).then(a.file.cmp(&b.file)));
    if !vworst.is_empty() {
        println!("tightest {} new vertical clips (fit today; line height vs height room):", vworst.len().min(10));
        for r in vworst.iter().take(10) {
            println!("  {:<44} {:<16} {:<12} room_h {:>3}  line today {:>2}  I12 {:>2}  I13 {:>2}  {:?}", short(&r.file, 44), short(&r.comp, 16), r.ty, r.room_h.unwrap_or(0), r.height[0], r.height[2], r.height[3], short(&r.text, 24));
        }
    }
    if let Some(find) = find {
        println!("measured texts matching {find:?} (room 0: AutoSize):");
        print_rows(split.rows.iter().filter(|r| r.file.contains(find) || r.comp.contains(find)));
        for r in split.rows.iter().filter(|r| r.file.contains(find) || r.comp.contains(find)) {
            println!("    {} {}: line height today {} / I11 {} / I12 {} / I13 {}, height room {:?}, v-clips {:?}", r.file, r.comp, r.height[0], r.height[1], r.height[2], r.height[3], r.room_h, r.vclip);
        }
    }
    if list {
        println!("every text clipping in some setting:");
        print_rows(fixed.iter().copied().filter(|r| r.hclip.iter().any(|c| *c)));
    }
    if !split.unresolved_font.is_empty() {
        println!("unresolved fonts (first 10): {}", split.unresolved_font.iter().take(10).cloned().collect::<Vec<_>>().join("; "));
    }
    println!();
}

fn print_rows<'a>(rows: impl Iterator<Item = &'a Row>) {
    println!("  {:<44} {:<16} {:<12} {:<26} {:>5} {:>5} {:>5} {:>5}  font", "file", "component", "type", "text", "room", "today", "I12", "I13");
    for r in rows {
        println!(
            "  {:<44} {:<16} {:<12} {:<26} {:>5} {:>5} {:>5} {:>5}  {}{}",
            short(&r.file, 44),
            short(&r.comp, 16),
            r.ty,
            format!("{:?}", short(&r.text, 24)),
            r.room_w.unwrap_or(0),
            r.width[0],
            r.width[2],
            r.width[3],
            r.font,
            if r.hclip[0] { "  (clips today)" } else { "" }
        );
    }
}

// ---------------------------------------------------------------------------

fn walk(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = fs::read_dir(dir) else { return };
    let mut entries: Vec<_> = entries.flatten().collect();
    entries.sort_by_key(|e| e.path());
    for entry in entries {
        let path = entry.path();
        if path.is_dir() {
            walk(&path, out);
        } else if path.extension().and_then(|e| e.to_str()).is_some_and(|e| ["bas", "rr", "inc", "rqb"].contains(&e.to_ascii_lowercase().as_str())) {
            out.push(path);
        }
    }
}

fn rapidq_dir() -> Option<PathBuf> {
    let dir = std::env::var_os("RAPIDQ_DIR").map(PathBuf::from).or_else(|| std::env::var_os("HOME").map(|h| Path::new(&h).join("Downloads/Rapidq")))?;
    dir.join("examples").is_dir().then_some(dir)
}

fn main() {
    // "Today" is the classic theme's face (RapidR Sans, MS Sans Serif's
    // metrics), whatever the default theme is.
    rapidr_value::theme::set(&rapidr_value::theme::CLASSIC);
    let args: Vec<String> = std::env::args().collect();
    let list = args.iter().any(|a| a == "--list");
    let find = args.iter().position(|a| a == "--find").and_then(|i| args.get(i + 1)).map(String::as_str);
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let root = root.canonicalize().unwrap_or(root);
    let mut repo = Split::default();
    let mut files = Vec::new();
    walk(&root.join("examples"), &mut files);
    for f in &files {
        let label = f.strip_prefix(&root).unwrap_or(f).display().to_string();
        audit_file(f, &label, &PreprocessOptions::default(), &mut repo);
    }
    let mut corpus = Split::default();
    match rapidq_dir() {
        Some(dir) => {
            let mut files = Vec::new();
            walk(&dir.join("examples"), &mut files);
            let options = PreprocessOptions { include_dirs: vec![dir.join("include")], ..PreprocessOptions::default() };
            for f in &files {
                let label = f.strip_prefix(&dir).unwrap_or(f).display().to_string();
                audit_file(f, &label, &options, &mut corpus);
            }
        }
        None => eprintln!("(no RapidQ corpus: set RAPIDQ_DIR)"),
    }
    report("examples/", &repo, list, find);
    report("RapidQ corpus", &corpus, list, find);
}
