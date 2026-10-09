//! RapidR Studio's form designer on the source (docs/ide-plan.md I4: the
//! two-way sync's groundwork). **The source is the truth**: a form is its
//! `CREATE Name AS Type … END CREATE` block, found by the parser for tools
//! (`rapidr_parser::tools`), and the designer never owns a separate copy.
//!
//! * [`Document::open`] reads every top-level CREATE block of a file into
//!   the designer model (`rapidr_value::designer`): components, their
//!   assignments in order (the value as source text), nested CREATEs, and
//!   the statements the designer doesn't own (kept as code).
//! * The designer changes the model with commands ([`Designer`]);
//!   [`Document::sync`] turns each one into the **smallest text edit**
//!   ([`TextPatch`]): a property's value span replaced; a new property line
//!   after the block's last one, in its indentation; a removed property's
//!   line (or, on a shared line, its statement and `:`); a new component as
//!   a nested CREATE before its parent's `END CREATE`; a removed one's
//!   lines; z-order and reparenting as the block's own lines moved; a
//!   rename on the CREATE's name. Comments, blank lines, other statements
//!   and everything outside the blocks stay byte for byte. After each edit
//!   the file is read again, so the model is always what the text says.
//! * One history per file: each sync is a text transaction;
//!   [`Document::undo`] / [`Document::redo`] put the exact bytes back.
//!
//! The live sync with the code editor (debounced re-reads, the read-only
//! banner on parse errors, "set in code" rows) is lane L-SYNC's, on this.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use rapidr_ast::{CreateStatement, Expression, Statement};
use rapidr_diagnostics::TextSpan;
use rapidr_parser::{expression_span, parse_source_for_tools, statement_span, ToolsParse};
use rapidr_preprocessor::{decode_source, encode_source, PreprocessOptions, SourceEncoding};
pub use rapidr_value::designer::{self, Command, CommandError, Designer, FormDesign, NodeId};
use rapidr_value::designer::model::{prop_key, Item, Prop, SubItem, Subtree};
use rapidr_value::designer::text::{write_create, Style};

/// A text edit: bytes `start..end` of the file's text replaced by
/// `insert`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TextPatch {
    pub start: usize,
    pub end: usize,
    pub insert: String,
}

/// Where a property assignment, a nested CREATE or other code is.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ItemSpan {
    /// `Left = 10`: the statement and its value.
    Prop { stmt: (usize, usize), value: (usize, usize) },
    Child { stmt: (usize, usize) },
    Code { stmt: (usize, usize) },
}

impl ItemSpan {
    fn stmt(&self) -> (usize, usize) {
        match self {
            ItemSpan::Prop { stmt, .. } | ItemSpan::Child { stmt } | ItemSpan::Code { stmt } => *stmt,
        }
    }
}

/// Where a CREATE block's pieces are in its file (byte offsets).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BlockSpans {
    /// `CREATE` … the end of `END CREATE`.
    pub start: usize,
    pub end: usize,
    /// Its whole lines: from its first line's start to after its last
    /// line's end (the line end included).
    pub lines: (usize, usize),
    /// The component's name in the header.
    pub name: (usize, usize),
    /// After the header line's end.
    pub header_end: usize,
    /// The start of the `END CREATE` line.
    pub end_line: usize,
    /// The header line's indentation.
    pub indent: String,
    /// One per item of the component's body, in order.
    pub items: Vec<ItemSpan>,
}

/// One designed form of the file: its designer and where its blocks are.
#[derive(Clone, Debug)]
pub struct Form {
    pub designer: Designer,
    /// The model as the text last said (the designer's journal is replayed
    /// on it to make the edits).
    synced: FormDesign,
    spans: HashMap<NodeId, BlockSpans>,
}

impl Form {
    pub fn name(&self) -> &str {
        self.synced.node(self.synced.root()).map_or("", |n| n.name.as_str())
    }

    /// Where a component's CREATE block is.
    pub fn spans(&self, id: NodeId) -> Option<&BlockSpans> {
        self.spans.get(&id)
    }
}

/// A source file open in the designer.
#[derive(Clone, Debug)]
pub struct Document {
    text: String,
    encoding: SourceEncoding,
    path: Option<PathBuf>,
    base_dir: PathBuf,
    options: PreprocessOptions,
    style: Style,
    forms: Vec<Form>,
    /// Text transactions: each the patches applied, with the text each
    /// replaced (in the order applied).
    undo: Vec<Vec<(TextPatch, String)>>,
    redo: Vec<Vec<(TextPatch, String)>>,
    diagnostics: Vec<String>,
    /// The first error's line in this file (from 1), when the text doesn't
    /// compile.
    error_line: Option<usize>,
}

/// A component read from the text, with where its pieces are.
struct ReadNode {
    name: String,
    type_written: String,
    body: Vec<ReadItem>,
    spans: BlockSpans,
}

enum ReadItem {
    Prop(Prop, ItemSpan),
    Child(ReadNode),
    Code(String, ItemSpan),
}

impl Document {
    /// Opens a file's bytes (decoded as RapidR reads programs: UTF-8, or
    /// Windows-1252). `path` places its `$INCLUDE`s.
    pub fn open_bytes(bytes: &[u8], path: Option<&Path>, options: PreprocessOptions) -> Document {
        let (text, encoding) = decode_source(bytes);
        let mut d = Document::open(&text, path, options);
        d.encoding = encoding;
        d
    }

    /// Opens a file's text.
    pub fn open(text: &str, path: Option<&Path>, options: PreprocessOptions) -> Document {
        let base_dir = path.and_then(Path::parent).map(Path::to_path_buf).unwrap_or_else(|| PathBuf::from("."));
        let mut d = Document {
            text: text.to_string(),
            encoding: SourceEncoding::Utf8,
            path: path.map(Path::to_path_buf),
            base_dir,
            options,
            style: Style::of(text),
            forms: Vec::new(),
            undo: Vec::new(),
            redo: Vec::new(),
            diagnostics: Vec::new(),
            error_line: None,
        };
        d.reread(&[]);
        d
    }

    pub fn text(&self) -> &str {
        &self.text
    }

    /// The file's bytes, in its encoding.
    pub fn bytes(&self) -> Vec<u8> {
        encode_source(&self.text, self.encoding)
    }

    pub fn forms(&self) -> &[Form] {
        &self.forms
    }

    /// A form's designer, to change it; then [`Document::sync`].
    pub fn designer(&mut self, form: usize) -> Option<&mut Designer> {
        self.forms.get_mut(form).map(|f| &mut f.designer)
    }

    /// The form whose CREATE is named `name` (any case).
    pub fn form_index(&self, name: &str) -> Option<usize> {
        self.forms.iter().position(|f| f.name().eq_ignore_ascii_case(name))
    }

    /// What the parser for tools reported (the text doesn't compile as it
    /// is: L-SYNC shows it, the designer keeps the last good state).
    pub fn diagnostics(&self) -> &[String] {
        &self.diagnostics
    }

    /// Where the text's first error is (a line from 1), when it doesn't
    /// parse: the designer shows the last good state, read-only.
    pub fn error_line(&self) -> Option<usize> {
        self.error_line
    }

    /// The code editor changed the text: the forms are read again (their
    /// components keep their ids by name).
    pub fn set_text(&mut self, text: &str) {
        self.text = text.to_string();
        let prev = self.previous();
        self.reread(&prev);
        for f in &mut self.forms {
            f.designer.history.clear();
            f.designer.take_applied();
        }
    }

    /// Applies a command to form `form` and writes it into the text: one
    /// undo step.
    pub fn apply(&mut self, form: usize, cmd: Command) -> Result<Vec<TextPatch>, CommandError> {
        let Some(f) = self.forms.get_mut(form) else { return Ok(Vec::new()) };
        f.designer.execute(cmd)?;
        Ok(self.sync())
    }

    /// Writes every designer's changes since the last sync into the text,
    /// as the smallest edits: one undo step. Returns the edits, in the
    /// order applied (each in the text as it was after the ones before).
    pub fn sync(&mut self) -> Vec<TextPatch> {
        let mut done: Vec<(TextPatch, String)> = Vec::new();
        for fi in 0..self.forms.len() {
            let journal = self.forms[fi].designer.take_applied();
            for cmd in journal {
                for sub in cmd.flatten() {
                    let patches = {
                        let f = &self.forms[fi];
                        patches_for(sub, &f.synced, &f.spans, &self.text, &self.style)
                    };
                    let f = &mut self.forms[fi];
                    let before = f.synced.clone();
                    let _ = sub.apply(&mut f.synced);
                    let synced = f.synced.clone();
                    if patches.is_empty() {
                        // (nothing to write: the text already says it — or, if
                        // the model did change, the text is the truth)
                        if synced != before {
                            let prev = self.previous();
                            self.reread(&prev);
                        }
                        continue;
                    }
                    for p in sort_patches(patches) {
                        let removed = self.text[p.start..p.end].to_string();
                        self.text.replace_range(p.start..p.end, &p.insert);
                        done.push((p, removed));
                    }
                    let mut prev = self.previous();
                    prev[fi] = synced;
                    self.reread(&prev);
                }
            }
        }
        // (the designers' models are what the text now says)
        for f in &mut self.forms {
            f.designer.design = f.synced.clone();
            f.designer.history.clear();
            f.designer.selection.retain(&f.synced);
        }
        let out: Vec<TextPatch> = done.iter().map(|(p, _)| p.clone()).collect();
        if !done.is_empty() {
            self.undo.push(done);
            self.redo.clear();
        }
        out
    }

    /// A new form for a file (one without a form, or another): `CREATE name
    /// AS RForm` (`QFORM` in a file written with RapidQ's names) with a
    /// Caption and RapidQ's starting size, and the line that shows it
    /// (`name.ShowModal`), at the end of the file in its own style — one
    /// undo step. The edit made.
    pub fn add_form(&mut self, name: &str) -> Vec<TextPatch> {
        let eol = self.style.eol.clone();
        let indent = if self.text.lines().any(|l| l.starts_with([' ', '\t']) && !l.trim().is_empty()) { self.style.indent.clone() } else { "    ".to_string() };
        let mut insert = String::new();
        if !self.text.is_empty() {
            if !self.text.ends_with('\n') {
                insert.push_str(&eol);
            }
            if !self.text.ends_with(&format!("{eol}{eol}")) && self.text.trim() != "" {
                insert.push_str(&eol);
            }
        }
        let path = self.path.as_deref().map(|p| p.to_string_lossy().into_owned()).unwrap_or_default();
        let form_type = if rapidr_value::designer::text::file_uses_rapidq_names(&self.text, &path) { "QFORM" } else { "RForm" };
        let tree = rapidr_value::designer::Subtree::new(name, form_type, &[("Caption", rapidr_value::designer::value::write_str(name)), ("Width", "320".into()), ("Height", "240".into())]);
        insert.push_str(&write_create(&tree, "", &Style { indent, eol: eol.clone() }));
        insert.push_str(&format!("{eol}{name}.ShowModal{eol}"));
        let at = self.text.len();
        let p = TextPatch { start: at, end: at, insert };
        self.text.push_str(&p.insert);
        self.undo.push(vec![(p.clone(), String::new())]);
        self.redo.clear();
        let prev = self.previous();
        self.reread(&prev);
        vec![p]
    }

    /// `line` (a statement) written on its own line right after form
    /// `form`'s `END CREATE`, in the file's line ends — one undo step. The
    /// edit made (none when there is no such form).
    pub fn insert_after_form(&mut self, form: usize, line: &str) -> Vec<TextPatch> {
        let Some(f) = self.forms.get(form) else { return Vec::new() };
        let Some(spans) = f.spans(f.synced.root()) else { return Vec::new() };
        let at = spans.lines.1;
        let eol = self.style.eol.clone();
        let mut insert = String::new();
        if at > 0 && !self.text[..at].ends_with('\n') {
            insert.push_str(&eol);
        }
        insert.push_str(line);
        insert.push_str(&eol);
        let p = TextPatch { start: at, end: at, insert };
        self.text.insert_str(at, &p.insert);
        self.undo.push(vec![(p.clone(), String::new())]);
        self.redo.clear();
        let prev = self.previous();
        self.reread(&prev);
        vec![p]
    }

    /// Undoes the last text transaction: the exact bytes come back.
    pub fn undo(&mut self) -> bool {
        let Some(t) = self.undo.pop() else { return false };
        let inverse = self.revert(&t);
        self.redo.push(inverse);
        true
    }

    pub fn redo(&mut self) -> bool {
        let Some(t) = self.redo.pop() else { return false };
        let inverse = self.revert(&t);
        self.undo.push(inverse);
        true
    }

    pub fn can_undo(&self) -> bool {
        !self.undo.is_empty()
    }

    pub fn can_redo(&self) -> bool {
        !self.redo.is_empty()
    }

    /// Forgets the undo / redo history (the text was changed elsewhere:
    /// its edits' places are gone).
    pub fn clear_history(&mut self) {
        self.undo.clear();
        self.redo.clear();
    }

    /// Applies a transaction's inverse; returns the inverse's own record.
    /// (A record `(patch, removed)`: `start..end` held `removed` and now
    /// holds `insert`.)
    fn revert(&mut self, t: &[(TextPatch, String)]) -> Vec<(TextPatch, String)> {
        let mut record = Vec::new();
        for (p, removed) in t.iter().rev() {
            let end = p.start + p.insert.len();
            self.text.replace_range(p.start..end, removed);
            record.push((TextPatch { start: p.start, end, insert: removed.clone() }, p.insert.clone()));
        }
        let prev = self.previous();
        self.reread(&prev);
        for f in &mut self.forms {
            f.designer.design = f.synced.clone();
            f.designer.history.clear();
            f.designer.take_applied();
            f.designer.selection.retain(&f.synced);
        }
        record
    }

    /// The forms' current models, in order: their ids are kept when the
    /// text is read again.
    fn previous(&self) -> Vec<FormDesign> {
        self.forms.iter().map(|f| f.synced.clone()).collect()
    }

    /// Reads the text's forms again.
    fn reread(&mut self, prev: &[FormDesign]) {
        let parse = parse_source_for_tools(&self.text, &self.base_dir, self.path.clone(), self.options.clone());
        self.diagnostics = parse.diagnostics.iter().map(|d| d.message.clone()).collect();
        let file = self.path.as_deref().and_then(|p| parse.file_id(p)).unwrap_or(0);
        self.error_line = parse.diagnostics.iter().find(|d| d.severity == rapidr_diagnostics::Severity::Error).map(|d| match locate(&parse, file, d.span) {
            Some((start, _)) if start <= self.text.len() => self.text[..start].matches('\n').count() + 1,
            _ => d.location.line.max(1),
        });
        let constants = program_constants(&parse.program.statements);
        // (new components are written in the file's own style: R-NAMES)
        let names = parse.name_counts(file).writing_style();
        let rapidq_file = self.path.as_deref().and_then(|p| p.extension()).and_then(|e| e.to_str()).is_some_and(|e| ["bas", "inc", "rqw", "rqb", "rq"].iter().any(|x| x.eq_ignore_ascii_case(e)));
        let mut forms = Vec::new();
        for s in &parse.program.statements {
            let Statement::Create(c) = s else { continue };
            let Some(node) = read_create(&parse, file, c, &self.text) else { continue };
            // (the same form: the one at the same place with this name — a
            // file may have two forms of one name, in $IFDEF branches)
            let k = forms.len();
            let form_name = node.name.clone();
            let same = |d: &FormDesign| d.node(d.root()).is_some_and(|n| n.name.eq_ignore_ascii_case(&form_name));
            let previous = prev.get(k).filter(|d| same(d)).or_else(|| prev.iter().find(|d| same(d)));
            let mut next = previous.map_or(1, FormDesign::next_id);
            let mut spans = HashMap::new();
            let tree = to_subtree(node, previous, &mut next, &mut spans, &mut Vec::new());
            let mut synced = FormDesign::from_subtree_after(tree, next);
            synced.set_constants(Some(constants.clone()));
            // (a RapidQ program: constants it doesn't define written as numbers)
            synced.set_rapidq(rapidq_file);
            synced.set_names(names);
            let old = self.forms.get(k).filter(|f| same(&f.synced)).or_else(|| self.forms.iter().find(|f| same(&f.synced)));
            let mut designer = match old {
                Some(old) => old.designer.clone(),
                None => Designer::new(synced.clone()),
            };
            designer.design = synced.clone();
            designer.selection.retain(&synced);
            forms.push(Form { designer, synced, spans });
        }
        self.forms = forms;
    }
}

/// Patches applied from the end of the text backwards (a removal before an
/// insertion at the same place).
fn sort_patches(mut p: Vec<TextPatch>) -> Vec<TextPatch> {
    p.sort_by(|a, b| b.start.cmp(&a.start).then(b.end.cmp(&a.end)));
    p
}

/// The located byte range of an AST span in `file` (exact or not: a
/// `$DEFINE`'s use is the name in the file).
fn locate(parse: &ToolsParse, file: usize, span: TextSpan) -> Option<(usize, usize)> {
    let l = parse.locate(span)?;
    (l.file == file).then_some((l.start, l.end))
}

fn is_ident(c: u8) -> bool {
    c.is_ascii_alphanumeric() || c == b'_' || c == b'$' || c == b'%' || c == b'&' || c == b'!' || c == b'#'
}

/// The start of the line holding `at`.
fn line_start(text: &str, at: usize) -> usize {
    text[..at].rfind('\n').map_or(0, |i| i + 1)
}

/// After the end of the line holding `at` (its line end included).
fn line_end(text: &str, at: usize) -> usize {
    text[at..].find('\n').map_or(text.len(), |i| at + i + 1)
}

/// The whitespace a line starts with.
fn indent_at(text: &str, line: usize) -> String {
    text[line..].chars().take_while(|c| *c == ' ' || *c == '\t').collect()
}

/// Whether only blanks are between the line's start and `at`.
fn starts_line(text: &str, at: usize) -> bool {
    text[line_start(text, at)..at].trim().is_empty()
}

/// Whether only blanks or a comment follow `at` on its line.
fn ends_line(text: &str, at: usize) -> bool {
    let rest = &text[at..line_end(text, at)];
    let rest = rest.trim();
    rest.is_empty() || rest.starts_with('\'') || rest.len() >= 3 && rest[..3].eq_ignore_ascii_case("rem") && rest.as_bytes().get(3).is_none_or(|c| c.is_ascii_whitespace())
}

/// A property's name as written, when an assignment's target is one: a
/// bare name (`Left`) or a sub-property (`Font.Name`, `Constraints.MinWidth`).
fn prop_name(target: &Expression, type_name: &str) -> Option<String> {
    match target {
        Expression::Identifier(id) => Some(id.name.clone()),
        Expression::MemberAccess(m) => {
            let Expression::Identifier(root) = &*m.object else { return None };
            let comp = rapidr_value::designer::model::canonical_type(type_name);
            let known = rapidr_lang_component_has(&comp, &root.name) || root.name.eq_ignore_ascii_case("constraints");
            known.then(|| format!("{}.{}", root.name, m.member))
        }
        _ => None,
    }
}

/// Whether the registry gives component `canonical` a property `name`.
fn rapidr_lang_component_has(canonical: &str, name: &str) -> bool {
    rapidr_value::designer::model::Node::new(0, "x", canonical).component().is_some_and(|c| c.property(name).is_some())
}

/// A CREATE block of `file` read from the AST and the text.
fn read_create(parse: &ToolsParse, file: usize, c: &CreateStatement, text: &str) -> Option<ReadNode> {
    let (start, end) = locate(parse, file, c.span)?;
    if end > text.len() || start >= end {
        return None;
    }
    let b = text.as_bytes();
    // The header: CREATE <name> AS <type>
    let mut i = start;
    while i < end && b[i].is_ascii_alphabetic() {
        i += 1;
    }
    while i < end && (b[i] == b' ' || b[i] == b'\t') {
        i += 1;
    }
    let name_start = i;
    while i < end && is_ident(b[i]) {
        i += 1;
    }
    let name = (name_start, i);
    while i < end && (b[i] == b' ' || b[i] == b'\t') {
        i += 1;
    }
    // (AS)
    while i < end && b[i].is_ascii_alphabetic() {
        i += 1;
    }
    while i < end && (b[i] == b' ' || b[i] == b'\t') {
        i += 1;
    }
    let ty_start = i;
    while i < end && is_ident(b[i]) {
        i += 1;
    }
    let type_written = text[ty_start..i].to_string();
    if text[name.0..name.1].is_empty() || type_written.is_empty() {
        return None;
    }
    // The END CREATE line: the last END before the end.
    let upper = text[start..end].to_ascii_uppercase();
    let end_kw = start + upper.rfind("END")?;
    let end_line = line_start(text, end_kw);
    let line_s = if starts_line(text, start) { line_start(text, start) } else { start };
    let line_e = if ends_line(text, end) { line_end(text, end) } else { end };
    let header_end = line_end(text, start);
    let indent = indent_at(text, line_start(text, start));
    let mut items = Vec::new();
    let mut body = Vec::new();
    for s in &c.body {
        let Some(stmt) = locate(parse, file, statement_span(s)) else { continue };
        match s {
            Statement::Comment(_) => {}
            Statement::Create(child) => {
                if let Some(n) = read_create(parse, file, child, text) {
                    let st = (n.spans.start, n.spans.end);
                    body.push(ReadItem::Child(n));
                    items.push(ItemSpan::Child { stmt: st });
                }
            }
            Statement::Assignment(a) => {
                let value = locate(parse, file, expression_span(&a.value));
                let target = locate(parse, file, expression_span(&a.target));
                match (prop_name(&a.target, &type_written), value, target) {
                    (Some(pname), Some(value), Some(target)) if value.1 <= text.len() && value.0 <= value.1 && target.0 < value.0 => {
                        let written = text[target.0..target.1].to_string();
                        let pname = if prop_key(&written) == prop_key(&pname) { written } else { pname };
                        let span = ItemSpan::Prop { stmt: (target.0, value.1), value };
                        body.push(ReadItem::Prop(Prop { name: pname, value: text[value.0..value.1].to_string() }, span.clone()));
                        items.push(span);
                    }
                    _ => {
                        body.push(ReadItem::Code(text[stmt.0..stmt.1.min(text.len())].to_string(), ItemSpan::Code { stmt }));
                        items.push(ItemSpan::Code { stmt });
                    }
                }
            }
            _ => {
                body.push(ReadItem::Code(text[stmt.0..stmt.1.min(text.len())].to_string(), ItemSpan::Code { stmt }));
                items.push(ItemSpan::Code { stmt });
            }
        }
    }
    let _ = items;
    let spans = BlockSpans { start, end, lines: (line_s, line_e), name, header_end, end_line, indent, items: Vec::new() };
    Some(ReadNode { name: text[name.0..name.1].to_string(), type_written, body, spans })
}

/// The read tree as the model's, ids kept by name from `prev` (new ones
/// from `next`), and each block's spans by id.
fn to_subtree(node: ReadNode, prev: Option<&FormDesign>, next: &mut NodeId, spans: &mut HashMap<NodeId, BlockSpans>, used: &mut Vec<NodeId>) -> Subtree {
    let id = match prev.and_then(|p| p.find(&node.name)).filter(|id| !used.contains(id)) {
        Some(id) => id,
        None => {
            let id = *next;
            *next += 1;
            id
        }
    };
    used.push(id);
    let mut block = node.spans;
    let mut body = Vec::new();
    for item in node.body {
        match item {
            ReadItem::Prop(p, s) => {
                block.items.push(s);
                body.push(SubItem::Prop(p));
            }
            ReadItem::Code(c, s) => {
                block.items.push(s);
                body.push(SubItem::Code(c));
            }
            ReadItem::Child(n) => {
                block.items.push(ItemSpan::Child { stmt: (n.spans.start, n.spans.end) });
                body.push(SubItem::Child(to_subtree(n, prev, next, spans, used)));
            }
        }
    }
    let tree = Subtree { id, name: node.name, type_written: node.type_written, body };
    spans.insert(id, block);
    tree
}

/// The patches removing the item at `stmt`: its whole line when it's alone
/// there, else the statement and its `:`.
fn remove_statement(text: &str, stmt: (usize, usize)) -> TextPatch {
    if starts_line(text, stmt.0) && ends_line(text, stmt.1) {
        return TextPatch { start: line_start(text, stmt.0), end: line_end(text, stmt.1), insert: String::new() };
    }
    let b = text.as_bytes();
    // a `:` after it: up to the next statement
    let mut j = stmt.1;
    while j < b.len() && (b[j] == b' ' || b[j] == b'\t') {
        j += 1;
    }
    if j < b.len() && b[j] == b':' {
        j += 1;
        while j < b.len() && (b[j] == b' ' || b[j] == b'\t') {
            j += 1;
        }
        return TextPatch { start: stmt.0, end: j, insert: String::new() };
    }
    // else the `:` before it
    let mut k = stmt.0;
    while k > 0 && (b[k - 1] == b' ' || b[k - 1] == b'\t') {
        k -= 1;
    }
    if k > 0 && b[k - 1] == b':' {
        k -= 1;
        while k > 0 && (b[k - 1] == b' ' || b[k - 1] == b'\t') {
            k -= 1;
        }
    }
    TextPatch { start: k, end: stmt.1, insert: String::new() }
}

/// Where a new item goes before the item at `at`: the start of its line,
/// moved up over the comment lines right above it (they belong to it).
fn before_item(text: &str, at: usize) -> usize {
    let mut pos = line_start(text, at);
    while pos > 0 {
        let prev = line_start(text, pos - 1);
        let line = text[prev..pos].trim();
        let comment = line.starts_with('\'') || line.len() >= 3 && line[..3].eq_ignore_ascii_case("rem") && line.as_bytes().get(3).is_none_or(|c| c.is_ascii_whitespace());
        if !comment {
            break;
        }
        pos = prev;
    }
    pos
}

/// The indentation of a block's items: its first item line's, else one
/// step in from its header.
fn body_indent(text: &str, block: &BlockSpans, style: &Style) -> String {
    block
        .items
        .iter()
        .map(ItemSpan::stmt)
        .find(|s| starts_line(text, s.0))
        .map(|s| indent_at(text, line_start(text, s.0)))
        .unwrap_or_else(|| format!("{}{}", block.indent, style.indent))
}

/// The text edits that make command `cmd` (applied to `m` next) true of
/// the text.
fn patches_for(cmd: &Command, m: &FormDesign, spans: &HashMap<NodeId, BlockSpans>, text: &str, style: &Style) -> Vec<TextPatch> {
    let eol = &style.eol;
    match cmd {
        Command::SetProp { node, name, value } => {
            let (Some(n), Some(block)) = (m.node(*node), spans.get(node)) else { return Vec::new() };
            match value {
                Some(v) => match n.prop_index(name) {
                    Some(i) => match block.items.get(i) {
                        Some(ItemSpan::Prop { value: vs, .. }) if &text[vs.0..vs.1] != v => vec![TextPatch { start: vs.0, end: vs.1, insert: v.clone() }],
                        _ => Vec::new(),
                    },
                    None => {
                        let last = n.body.iter().rposition(|i| matches!(i, Item::Prop(_)));
                        let (at, indent) = match last.and_then(|i| block.items.get(i)) {
                            Some(s) => (line_end(text, s.stmt().1), indent_at(text, line_start(text, s.stmt().0))),
                            None => (block.header_end, body_indent(text, block, style)),
                        };
                        let lead = if at > 0 && !text[..at].ends_with('\n') { eol.as_str() } else { "" };
                        vec![TextPatch { start: at, end: at, insert: format!("{lead}{indent}{name} = {v}{eol}") }]
                    }
                },
                None => {
                    let key = prop_key(name);
                    n.body
                        .iter()
                        .enumerate()
                        .filter(|(_, it)| matches!(it, Item::Prop(p) if prop_key(&p.name) == key))
                        .filter_map(|(i, _)| block.items.get(i))
                        .map(|s| remove_statement(text, s.stmt()))
                        .collect()
                }
            }
        }
        Command::RemoveProp { node, index } => match spans.get(node).and_then(|b| b.items.get(*index)) {
            Some(s) => vec![remove_statement(text, s.stmt())],
            None => Vec::new(),
        },
        Command::InsertProp { node, index, prop } => {
            let Some(block) = spans.get(node) else { return Vec::new() };
            let line = format!("{} = {}", prop.name, prop.value);
            match block.items.get(*index) {
                Some(s) if starts_line(text, s.stmt().0) => {
                    let ls = line_start(text, s.stmt().0);
                    vec![TextPatch { start: ls, end: ls, insert: format!("{}{line}{eol}", indent_at(text, ls)) }]
                }
                Some(s) => vec![TextPatch { start: s.stmt().0, end: s.stmt().0, insert: format!("{line}: ") }],
                None => {
                    let at = match block.items.last() {
                        Some(ItemSpan::Child { .. }) | None => block.end_line,
                        Some(s) => line_end(text, s.stmt().1),
                    };
                    let at = at.max(block.header_end).min(block.end_line.max(block.header_end));
                    vec![TextPatch { start: at, end: at, insert: format!("{}{line}{eol}", body_indent(text, block, style)) }]
                }
            }
        }
        Command::Insert { parent, index, tree } => {
            let (Some(p), Some(block)) = (m.node(*parent), spans.get(parent)) else { return Vec::new() };
            let indent = child_indent(text, m, spans, *parent, style);
            let at = match block.items.get(*index).filter(|_| *index < p.body.len()) {
                Some(s) => before_item(text, s.stmt().0),
                None => block.end_line,
            };
            // (its own lines one step in: the step the parent block uses)
            let step = match body_indent(text, block, style).strip_prefix(block.indent.as_str()) {
                Some(s) if !s.is_empty() => s.to_string(),
                _ => style.indent.clone(),
            };
            let local = Style { indent: step, eol: style.eol.clone() };
            vec![TextPatch { start: at, end: at, insert: write_create(tree, &indent, &local) }]
        }
        Command::Remove { node } => match spans.get(node) {
            Some(b) => vec![TextPatch { start: b.lines.0, end: b.lines.1, insert: String::new() }],
            None => Vec::new(),
        },
        Command::Move { node, parent, index } => {
            let (Some(b), Some(pblock), Some(p)) = (spans.get(node), spans.get(parent), m.node(*parent)) else { return Vec::new() };
            // the target's items without the moved one
            let items: Vec<&ItemSpan> = p.body.iter().zip(&pblock.items).filter(|(it, _)| **it != Item::Child(*node)).map(|(_, s)| s).collect();
            let at = match items.get(*index) {
                Some(s) => before_item(text, s.stmt().0),
                None => pblock.end_line,
            };
            let old_indent = b.indent.clone();
            let new_indent = child_indent(text, m, spans, *parent, style);
            let block = &text[b.lines.0..b.lines.1];
            let moved: String = if old_indent == new_indent {
                block.to_string()
            } else {
                block.split_inclusive('\n').map(|l| match l.strip_prefix(old_indent.as_str()) {
                    Some(rest) => format!("{new_indent}{rest}"),
                    None => l.to_string(),
                }).collect()
            };
            if at >= b.lines.0 && at <= b.lines.1 && old_indent == new_indent {
                // (already there)
                return Vec::new();
            }
            vec![TextPatch { start: b.lines.0, end: b.lines.1, insert: String::new() }, TextPatch { start: at, end: at, insert: moved }]
        }
        Command::Rename { node, name } => match spans.get(node) {
            Some(b) => vec![TextPatch { start: b.name.0, end: b.name.1, insert: name.clone() }],
            None => Vec::new(),
        },
        Command::Batch(_) => Vec::new(),
    }
}

/// The indentation for a CREATE nested in `parent`: its other children's,
/// else its items', else one step in.
fn child_indent(text: &str, m: &FormDesign, spans: &HashMap<NodeId, BlockSpans>, parent: NodeId, style: &Style) -> String {
    if let Some(c) = m.children(parent).iter().filter_map(|c| spans.get(c)).find(|b| starts_line(text, b.start)) {
        return c.indent.clone();
    }
    match spans.get(&parent) {
        Some(b) => body_indent(text, b, style),
        None => style.indent.clone(),
    }
}

/// The program's integer CONSTs (lower case), in order — RAPIDQ.INC's when
/// it includes it: `alClient` means 5 only then (else it's an undeclared
/// variable, 0, in RapidQ and RapidR alike).
fn program_constants(statements: &[Statement]) -> std::collections::BTreeMap<String, i64> {
    let mut out = std::collections::BTreeMap::new();
    for s in statements {
        if let Statement::Const(c) = s {
            if let Some(v) = const_value(&c.value, &out) {
                out.insert(c.name.to_ascii_lowercase(), v);
            }
        }
    }
    out
}

fn const_value(e: &Expression, known: &std::collections::BTreeMap<String, i64>) -> Option<i64> {
    use rapidr_ast::{BinaryOperator as B, LiteralValue, UnaryOperator as U};
    match e {
        Expression::Literal(l) => match &l.value {
            LiteralValue::Integer(n) => Some(*n),
            LiteralValue::Float(f) if f.fract() == 0.0 => Some(*f as i64),
            _ => None,
        },
        Expression::Identifier(id) => known.get(&id.name.to_ascii_lowercase()).copied().or_else(|| rapidr_value::designer::value::builtin_constant(&id.name)),
        Expression::Unary(u) if u.operator == U::Negate => const_value(&u.operand, known).map(|v| -v),
        Expression::Binary(b) => {
            let (l, r) = (const_value(&b.left, known)?, const_value(&b.right, known)?);
            Some(match b.operator {
                B::Add => l + r,
                B::Subtract => l - r,
                B::Multiply => l * r,
                B::Or => l | r,
                B::And => l & r,
                B::Xor => l ^ r,
                _ => return None,
            })
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests;

// (event handlers made from the designer and the inspector: S-PANELS)
mod handlers;
pub use handlers::{default_event, params_text, Handler};
