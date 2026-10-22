//! The code's helpers in the code editor's model (docs/code-helpers-plan.md):
//! the language service's helper sites kept and mapped through edits, the
//! settings (a level and a switch per helper), the decorations each line
//! gets — a mark after a value (a colour's swatch, a picture's thumbnail, a
//! font's sample), a chip before an argument (its parameter's name) —, the
//! values a debugger shows at the lines' ends, and a value written back the
//! way it was written (`RGB(…)` stays `RGB(…)`, `&H` stays `&H`, a constant
//! stays a constant). The kernel draws them and opens the value editors;
//! this is what they show and write, on every host (wasm-safe).
//!
//! **No layout shift.** A mark is [`MARK_COLS`] columns, whatever it shows;
//! a chip its name's. While the user types on a line, the sites that come
//! after the pause for that line wait ([`Helpers::pending`]) until the
//! caret leaves it, so nothing on the line being typed jumps (a mark's
//! value — its colour — still follows).

use rapidr_editor::service::{HelperKind, HelperSite};
use rapidr_editor::transaction::{Assoc, ChangeSet};

/// A mark's width, in columns: the same for a swatch, a thumbnail, a font
/// sample, a missing file's mark.
pub const MARK_COLS: usize = 2;

/// Values a line shows at most while the program is paused.
pub const VALUES_PER_LINE: usize = 5;

/// How much the helpers show (`Helpers = "helpful"`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord)]
pub enum Level {
    /// None.
    Off,
    /// The marks only.
    Quiet,
    /// The marks, the caret line's note, the names of literal arguments,
    /// the values while paused (the default).
    #[default]
    Helpful,
    /// All of them, every argument's name, and a mark for the choices and
    /// links too.
    Everything,
}

impl Level {
    pub fn parse(s: &str) -> Level {
        match s.trim().to_ascii_lowercase().as_str() {
            "off" | "none" | "0" => Level::Off,
            "quiet" | "marks" | "1" => Level::Quiet,
            "everything" | "all" | "3" => Level::Everything,
            _ => Level::Helpful,
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Level::Off => "off",
            Level::Quiet => "quiet",
            Level::Helpful => "helpful",
            Level::Everything => "everything",
        }
    }
}

/// Each helper's switch (`HelpersOff = "names,note"` turns those off).
pub const SWITCHES: [&str; 8] = ["color", "font", "file", "choice", "link", "names", "note", "values"];

/// A value the debugger gave for a name (ShowLineValues): shown at the end
/// of each line in view that uses the name.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InlineValue {
    pub name: String,
    /// As the debugger writes it (a text quoted).
    pub value: String,
    /// The debugger's kind (`Integer`, `String`, a component's type …).
    pub kind: String,
}

/// A decoration on a line: a mark after a value or a chip before an
/// argument, `cols` columns wide at document byte `at`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Deco {
    pub at: usize,
    pub cols: usize,
    /// A chip (before the text at `at`); else a mark (after the text
    /// ending at `at`).
    pub chip: bool,
    /// Its site in [`Helpers::sites`].
    pub site: usize,
}

/// The helpers of one code editor.
#[derive(Clone, Debug, Default)]
pub struct Helpers {
    /// The language service's sites, by start; mapped through edits.
    pub sites: Vec<HelperSite>,
    /// Goes up whenever what the lines show could change (the sites, the
    /// level, a switch, Ctrl held): the views' row tables and caches.
    pub version: u64,
    /// The sites that came while the caret stayed on the line being typed
    /// on: given when it leaves.
    pub pending: Option<Vec<HelperSite>>,
    /// A byte on the line being typed on (mapped through edits).
    pub typing_at: Option<usize>,
    /// Ctrl held a moment: every argument's name shows.
    pub held: bool,
    pub level: Level,
    /// The switches turned off.
    pub off: Vec<String>,
    /// Where the project's files are (`ProjectFolder`; "": the code file's
    /// folder): the only place pictures are read from.
    pub project_folder: String,
    /// The project's files (`ProjectFiles`, relative to the folder): what
    /// the file editor offers, and how a file is known to be there.
    pub project_files: Vec<String>,
    /// ShowLineValues': the values, and the lines (0-based, both kept) whose
    /// ends show them.
    pub values: Vec<InlineValue>,
    pub values_lines: Option<(usize, usize)>,
    /// What the open value editor shows, in words (`HelperEditorText`; the
    /// kernel sets it).
    pub editor_text: String,
}

impl Helpers {
    /// Whether switch `name` is on (and the helpers are).
    pub fn on(&self, name: &str) -> bool {
        self.level != Level::Off && !self.off.iter().any(|o| o.eq_ignore_ascii_case(name))
    }

    /// The switch a site's kind belongs to.
    pub fn switch_of(kind: HelperKind) -> &'static str {
        match kind {
            HelperKind::Color => "color",
            HelperKind::Font => "font",
            HelperKind::File => "file",
            HelperKind::Choice | HelperKind::Bool => "choice",
            HelperKind::Component | HelperKind::Handler => "link",
            HelperKind::ArgName => "names",
            HelperKind::Note => "note",
        }
    }

    /// Whether site `h` gets a mark after its value.
    pub fn shows_mark(&self, h: &HelperSite) -> bool {
        if !self.on(Self::switch_of(h.kind)) {
            return false;
        }
        match h.kind {
            HelperKind::Color => h.value.parse::<i64>().is_ok(),
            HelperKind::Font | HelperKind::File => !h.value.is_empty(),
            // (a choice's or a link's ▾: only when everything shows; else
            // Ctrl+. on it)
            HelperKind::Choice | HelperKind::Bool | HelperKind::Component | HelperKind::Handler => self.level == Level::Everything && h.start < h.end,
            HelperKind::ArgName | HelperKind::Note => false,
        }
    }

    /// Whether argument site `h` gets its name's chip.
    pub fn shows_chip(&self, h: &HelperSite) -> bool {
        h.kind == HelperKind::ArgName && self.on("names") && (self.held || self.level == Level::Everything || (self.level >= Level::Helpful && h.flag))
    }

    /// A chip's columns: its name, a colon and a space.
    pub fn chip_cols(h: &HelperSite) -> usize {
        h.label.chars().count() + 2
    }

    /// The sites starting in bytes `a..=b`.
    pub fn in_range(&self, a: usize, b: usize) -> std::ops::Range<usize> {
        let first = self.sites.partition_point(|h| h.start < a);
        let last = self.sites.partition_point(|h| h.start <= b);
        first..last.max(first)
    }

    /// The decorations of the line of bytes `start..end` (its break not
    /// counted), in order.
    pub fn decos(&self, start: usize, end: usize) -> Vec<Deco> {
        if self.level == Level::Off {
            return Vec::new();
        }
        let mut out = Vec::new();
        for i in self.in_range(start, end) {
            let h = &self.sites[i];
            if self.shows_chip(h) {
                out.push(Deco { at: h.start, cols: Self::chip_cols(h), chip: true, site: i });
            } else if h.end <= end && self.shows_mark(h) {
                out.push(Deco { at: h.end, cols: MARK_COLS, chip: false, site: i });
            }
        }
        // (at one byte: the mark of the value ending there first, then the
        // chip of the argument starting there)
        out.sort_by_key(|d| (d.at, d.chip));
        out
    }

    /// The line's decorations as `(byte of the line, columns, chip)`.
    pub fn inserts(&self, start: usize, end: usize) -> Vec<(usize, usize, bool)> {
        self.decos(start, end).into_iter().map(|d| (d.at - start, d.cols, d.chip)).collect()
    }

    /// The note of the line of bytes `start..end`, when it shows.
    pub fn note(&self, start: usize, end: usize) -> Option<&str> {
        if self.level < Level::Helpful || !self.on("note") {
            return None;
        }
        self.in_range(start, end).map(|i| &self.sites[i]).find(|h| h.kind == HelperKind::Note).map(|h| h.label.as_str())
    }

    /// The value site an editor opens for the caret at byte `at` of the
    /// line of bytes `start..end`: the one holding it (the innermost), or
    /// the line's only one; one waiting for its SUB anywhere on the line.
    pub fn editable_at(&self, at: usize, start: usize, end: usize) -> Option<usize> {
        let r = self.in_range(start, end);
        let editable = |h: &HelperSite| !h.form.is_empty() && !matches!(h.kind, HelperKind::ArgName | HelperKind::Note) && self.on(Self::switch_of(h.kind));
        let mut best: Option<usize> = None;
        for i in r.clone() {
            let h = &self.sites[i];
            if editable(h) && h.start <= at && at <= h.end && best.is_none_or(|b| self.sites[b].end - self.sites[b].start > h.end - h.start) {
                best = Some(i);
            }
        }
        best.or_else(|| {
            let all: Vec<usize> = r.filter(|&i| editable(&self.sites[i])).collect();
            (all.len() == 1).then(|| all[0])
        })
    }

    /// Every site's bytes through an edit.
    pub fn map(&mut self, set: &ChangeSet) {
        let changes = set.changes();
        let (Some(first), Some(last)) = (changes.first(), changes.last()) else { return };
        // (a key typed is one small change: the sites before it stay, the
        // ones after it move by the same few bytes — only the ones it
        // touches are mapped one by one; a 4,500-line file's thousands of
        // sites cost a couple of microseconds a key)
        let (from, to) = (first.range.start, last.range.end);
        let delta: isize = changes.iter().map(|c| c.insert.len() as isize - c.range.len() as isize).sum();
        let shift = |p: usize| (p as isize + delta).max(0) as usize;
        let map_all = |v: &mut Vec<HelperSite>| {
            let mut gone = false;
            // (sorted by start: those starting before the edit are left
            // alone but the few just before it, which may reach into it —
            // a line's note, a value over a line)
            let first = v.partition_point(|h| h.start < from);
            let from_i = first.saturating_sub(64);
            for h in v[from_i..].iter_mut() {
                if h.end < from {
                    // (before the edit; a handler's SUB goes before its site)
                    continue;
                }
                if h.start > to {
                    h.start = shift(h.start);
                    h.end = shift(h.end);
                    if h.kind == HelperKind::Handler && h.at > to {
                        h.at = shift(h.at);
                    } else if h.kind == HelperKind::Handler && h.at >= from {
                        h.at = set.map_pos(h.at, Assoc::Before);
                    }
                    continue;
                }
                let (a, b) = (set.map_pos(h.start, Assoc::After), set.map_pos(h.end, Assoc::Before));
                // (a site the edit removed goes; a point stays a point)
                if h.start < h.end && b <= a {
                    h.end = usize::MAX;
                    gone = true;
                    continue;
                }
                h.start = a;
                h.end = b.max(a);
                if h.kind == HelperKind::Handler && h.at >= from {
                    h.at = set.map_pos(h.at, Assoc::Before);
                }
            }
            if gone {
                v.retain(|h| h.end != usize::MAX);
            }
        };
        map_all(&mut self.sites);
        if let Some(p) = &mut self.pending {
            map_all(p);
        }
        if let Some(t) = &mut self.typing_at {
            *t = set.map_pos(*t, Assoc::Before);
        }
    }

    /// The service's new sites. On the line being typed on (`typing`: its
    /// bytes, the caret still there) the sites keep their places: new ones
    /// wait until the caret leaves; a mark's value follows (same width).
    pub fn set_sites(&mut self, mut new: Vec<HelperSite>, typing: Option<(usize, usize)>) {
        new.sort_by_key(|h| (h.start, h.kind != HelperKind::ArgName, h.end));
        match typing {
            Some((a, b)) => {
                let mut kept: Vec<HelperSite> = new.iter().filter(|h| !(a <= h.start && h.start <= b)).cloned().collect();
                for old in self.sites.iter().filter(|h| a <= h.start && h.start <= b) {
                    let mut o = old.clone();
                    if let Some(n) = new.iter().find(|n| n.kind == o.kind && n.start == o.start && n.end == o.end) {
                        o.value = n.value.clone();
                        o.form = n.form.clone();
                    }
                    kept.push(o);
                }
                kept.sort_by_key(|h| (h.start, h.kind != HelperKind::ArgName, h.end));
                self.sites = kept;
                self.pending = Some(new);
            }
            None => {
                self.sites = new;
                self.pending = None;
            }
        }
        self.version += 1;
    }

    /// The caret left the line being typed on: its waiting sites are given.
    pub fn release(&mut self) -> bool {
        self.typing_at = None;
        match self.pending.take() {
            Some(p) => {
                self.sites = p;
                self.version += 1;
                true
            }
            None => false,
        }
    }

    /// ShowLineValues' rows (a line each: name, value, kind, tab-separated).
    pub fn set_values(&mut self, rows: &str, from: usize, to: usize) {
        self.values = rows
            .lines()
            .filter_map(|l| {
                let mut f = l.split('\t');
                let name = f.next()?.trim().to_string();
                let value = f.next()?.to_string();
                let kind = f.next().unwrap_or("").trim().to_string();
                (!name.is_empty()).then_some(InlineValue { name, value, kind })
            })
            .collect();
        self.values_lines = (!self.values.is_empty()).then_some((from.min(to), from.max(to)));
        self.version += 1;
    }

    /// The values line `line` (0-based; its `text`) shows: the names it
    /// uses that have one, in order, at most [`VALUES_PER_LINE`]. Strings and
    /// comments don't count.
    pub fn values_on(&self, line: usize, text: &str) -> Vec<&InlineValue> {
        let Some((a, b)) = self.values_lines else { return Vec::new() };
        if line < a || line > b || !self.on("values") || self.level < Level::Helpful {
            return Vec::new();
        }
        let mut out: Vec<&InlineValue> = Vec::new();
        for w in code_words(text) {
            if out.len() >= VALUES_PER_LINE {
                break;
            }
            if let Some(v) = self.values.iter().find(|v| v.name.eq_ignore_ascii_case(w)) {
                if !out.iter().any(|o| std::ptr::eq(*o, v)) {
                    out.push(v);
                }
            }
        }
        out
    }

    /// A file site's value as a path to read: inside the project's folder
    /// (`dir`, the code file's folder, when ProjectFolder isn't set), never
    /// elsewhere, never a URL.
    pub fn project_path(&self, dir: &str, value: &str) -> Option<String> {
        let folder = if self.project_folder.is_empty() { dir } else { &self.project_folder };
        confined(folder, value)
    }

    /// The folder files are read from: ProjectFolder, else `file`'s (the
    /// code file's) folder.
    pub fn folder(&self, file: &str) -> String {
        if !self.project_folder.is_empty() {
            return self.project_folder.clone();
        }
        std::path::Path::new(file).parent().map(|p| p.to_string_lossy().into_owned()).unwrap_or_default()
    }

    /// Whether the file `value` names is there, inside `folder` (`None`:
    /// it can't be told — a URL, a name reaching outside the project, which
    /// is never looked at). Remembered a while.
    pub fn file_there(&self, folder: &str, value: &str) -> Option<bool> {
        if self.listed(value) == Some(true) {
            return Some(true);
        }
        let path = confined(folder, value)?;
        let key = (folder.to_string(), value.to_string());
        if let Some(v) = THERE.with(|t| t.borrow().get(&key).copied()) {
            return Some(v);
        }
        #[cfg(not(target_arch = "wasm32"))]
        let there = std::path::Path::new(&path).is_file() || crate::objects::read_file(&path).is_ok();
        #[cfg(target_arch = "wasm32")]
        let there = crate::objects::read_file(&path).is_ok();
        THERE.with(|t| {
            let mut t = t.borrow_mut();
            if t.len() > 512 {
                t.clear();
            }
            t.insert(key, there);
        });
        Some(there)
    }

    /// Whether the project's list (when there is one) has `value`.
    pub fn listed(&self, value: &str) -> Option<bool> {
        if self.project_files.is_empty() {
            return None;
        }
        let v = value.replace('\\', "/");
        let v = v.trim_start_matches("./");
        Some(self.project_files.iter().any(|f| f.replace('\\', "/").eq_ignore_ascii_case(v)))
    }
}

impl Helpers {
    /// Line bytes `start..end` (its `text`) read with its helpers, for a
    /// screen reader ("Read line with helpers"): the text, then what each
    /// value is — "orange, 255 130 20", "picture logo.png, missing" —, the
    /// note. `exists` says whether a file site's file is there (`None`:
    /// not known).
    pub fn describe_line(&self, start: usize, end: usize, text: &str, exists: &dyn Fn(&HelperSite) -> Option<bool>) -> String {
        let mut out = text.trim().to_string();
        if self.level == Level::Off {
            return out;
        }
        let mut note: Option<String> = None;
        for i in self.in_range(start, end) {
            let h = &self.sites[i];
            if !self.on(Self::switch_of(h.kind)) {
                continue;
            }
            let said = match h.kind {
                HelperKind::Color => h.value.parse::<i64>().ok().map(color_said),
                HelperKind::Font => Some(format!("font {}", h.value)),
                HelperKind::File => {
                    let what = match h.label.as_str() {
                        "image" => "picture",
                        "sound" => "sound",
                        _ => "file",
                    };
                    Some(match exists(h) {
                        Some(false) => format!("{what} {}, missing", h.value),
                        _ => format!("{what} {}", h.value),
                    })
                }
                HelperKind::Choice | HelperKind::Bool => (!h.value.is_empty()).then(|| format!("{}, one of {} choices", h.value.replace('+', " and "), h.choices.len())),
                HelperKind::Component => Some(format!("link to {}", h.value)),
                HelperKind::Handler if h.value.is_empty() => Some("no handler yet: Ctrl+. offers the SUBs".to_string()),
                HelperKind::Handler => Some(format!("handler {}", h.value)),
                // (the line's help after its values)
                HelperKind::Note if self.level >= Level::Helpful => {
                    note = Some(h.label.clone());
                    None
                }
                _ => None,
            };
            if let Some(s) = said {
                out.push_str("; ");
                out.push_str(&s);
            }
        }
        if let Some(n) = note {
            out.push_str("; ");
            out.push_str(&n);
        }
        out
    }
}

thread_local! {
    /// (folder, a file's name) → whether it is there (Helpers::file_there).
    static THERE: std::cell::RefCell<std::collections::HashMap<(String, String), bool>> = std::cell::RefCell::new(std::collections::HashMap::new());
}

/// Forgets which files were there (a file saved, added or removed).
pub fn forget_files() {
    THERE.with(|t| t.borrow_mut().clear());
}

/// The names a line of code uses (outside strings and `'` comments).
fn code_words(text: &str) -> Vec<&str> {
    let mut out = Vec::new();
    let b = text.as_bytes();
    let mut i = 0;
    let mut in_str = false;
    while i < b.len() {
        let c = b[i];
        if in_str {
            if c == b'"' {
                in_str = false;
            }
            i += 1;
            continue;
        }
        match c {
            b'"' => in_str = true,
            b'\'' => break,
            c if c.is_ascii_alphabetic() || c == b'_' => {
                let s = i;
                while i < b.len() && (b[i].is_ascii_alphanumeric() || b[i] == b'_' || b[i] == b'$' || b[i] == b'%' || b[i] == b'.') {
                    i += 1;
                }
                // (`Form1.Caption`: both the whole and its first name)
                let word = text[s..i].trim_end_matches('.');
                if s == 0 || b[s - 1] != b'.' {
                    if word.contains('.') {
                        out.push(word);
                    }
                    out.push(word.split('.').next().unwrap_or(word));
                }
                continue;
            }
            _ => {}
        }
        i += 1;
    }
    out
}

/// `value` (a file's name as the code writes it) read from inside `folder`:
/// its path, or `None` when it is a URL, or reaches outside the folder.
pub fn confined(folder: &str, value: &str) -> Option<String> {
    let v = value.replace('\\', "/");
    if v.is_empty() || v.contains("://") || v.contains('\0') || folder.is_empty() {
        return None;
    }
    let root = normal(&folder.replace('\\', "/"));
    let absolute = v.starts_with('/') || v.as_bytes().get(1) == Some(&b':');
    let joined = if absolute { normal(&v) } else { normal(&format!("{root}/{v}")) };
    let inside = joined.len() > root.len() && joined.starts_with(&root) && joined.as_bytes()[root.len()] == b'/';
    if !inside {
        return None;
    }
    // (a link out of the folder is outside it too)
    #[cfg(not(target_arch = "wasm32"))]
    if let (Ok(real), Ok(real_root)) = (std::fs::canonicalize(&joined), std::fs::canonicalize(&root)) {
        if !real.starts_with(&real_root) {
            return None;
        }
    }
    Some(joined)
}

/// A path with `.` and `..` resolved (lexically), `/` between parts.
fn normal(p: &str) -> String {
    let lead = if p.starts_with('/') { "/" } else { "" };
    let mut parts: Vec<&str> = Vec::new();
    for part in p.split('/') {
        match part {
            "" | "." => {}
            ".." => {
                parts.pop();
            }
            x => parts.push(x),
        }
    }
    format!("{lead}{}", parts.join("/"))
}

// ---- writing a value back the way it was written ---------------------------

/// A colour (RapidQ's BGR, or a system colour's number) written in `form`,
/// in the style of `old` (its text): `RGB(r, g, b)` (the old spacing and
/// case), `&HBBGGRR` (six digits, the old case), a constant (`clRed` —
/// `RGB(…)` when the colour has no name), a number.
pub fn color_text(form: &str, value: i64, old: &str) -> String {
    let named = crate::panels::inspector::values::color_name(value);
    let system = crate::panels::inspector::values::is_system_color(value);
    let rgb = |old: &str| {
        let (r, g, b) = (value & 0xFF, (value >> 8) & 0xFF, (value >> 16) & 0xFF);
        let name = old.split('(').next().filter(|n| n.trim().eq_ignore_ascii_case("rgb")).map_or("RGB", str::trim);
        let sep = if old.contains(", ") || !old.contains(',') { ", " } else { "," };
        format!("{name}({r}{sep}{g}{sep}{b})")
    };
    match form {
        // (a system colour has no RGB: its constant)
        _ if system => named.map_or_else(|| crate::panels::inspector::values::color_hex(value), str::to_string),
        "hex" => {
            let lower = old.trim().get(2..).is_some_and(|d| d.chars().any(|c| c.is_ascii_lowercase()));
            let prefix = old.trim().get(..2).filter(|p| p.eq_ignore_ascii_case("&h")).unwrap_or("&H");
            let digits = format!("{:06X}", value & 0xFF_FFFF);
            format!("{prefix}{}", if lower { digits.to_ascii_lowercase() } else { digits })
        }
        "const" => named.map_or_else(|| rgb("RGB(0, 0, 0)"), str::to_string),
        "number" => value.to_string(),
        _ => rgb(old),
    }
}

/// A text written as a BASIC string literal (`None`: it can't be one — a
/// `"` or a line break in it).
pub fn string_text(value: &str) -> Option<String> {
    (!value.contains('"') && !value.contains('\n') && !value.contains('\r')).then(|| format!("\"{value}\""))
}

/// A choice written in `form`: a constant, constants added (`a + b`) or
/// OR'd (`a OR b`) — the set's flags; nothing picked is `0` —, a number.
pub fn choice_text(form: &str, picked: &[String]) -> String {
    match form {
        "number" => picked.iter().map(|p| rapidr_lang::eval_constant(p).unwrap_or(0)).sum::<i64>().to_string(),
        _ if picked.is_empty() => "0".into(),
        "flags-or" => picked.join(" OR "),
        _ => picked.join(" + "),
    }
}

/// True / False written as `old` was: a word in its case, or a number.
pub fn bool_text(form: &str, value: bool, old: &str) -> String {
    if form == "number" {
        return if value { if old.trim() == "1" { "1" } else { "-1" } } else { "0" }.into();
    }
    let word = if value { "True" } else { "False" };
    let o = old.trim();
    if !o.is_empty() && o.chars().all(|c| !c.is_ascii_lowercase()) {
        word.to_ascii_uppercase()
    } else if !o.is_empty() && o.chars().all(|c| !c.is_ascii_uppercase()) {
        word.to_ascii_lowercase()
    } else {
        word.into()
    }
}

/// A colour in words, for screen readers and the editor's name line:
/// `orange`, `dark blue`, `light grey`.
pub fn color_words(rgb: u32) -> String {
    let (r, g, b) = (((rgb >> 16) & 0xFF) as f64 / 255.0, ((rgb >> 8) & 0xFF) as f64 / 255.0, (rgb & 0xFF) as f64 / 255.0);
    let max = r.max(g).max(b);
    let min = r.min(g).min(b);
    let l = (max + min) / 2.0;
    let d = max - min;
    let s = if d == 0.0 { 0.0 } else { d / (1.0 - (2.0 * l - 1.0).abs()) };
    if s < 0.15 || d < 0.08 {
        return match l {
            l if l < 0.12 => "black",
            l if l < 0.35 => "dark grey",
            l if l < 0.65 => "grey",
            l if l < 0.9 => "light grey",
            _ => "white",
        }
        .into();
    }
    let h = if d == 0.0 {
        0.0
    } else if max == r {
        60.0 * (((g - b) / d) % 6.0)
    } else if max == g {
        60.0 * ((b - r) / d + 2.0)
    } else {
        60.0 * ((r - g) / d + 4.0)
    };
    let h = if h < 0.0 { h + 360.0 } else { h };
    let name = match h {
        h if !(12.0..345.0).contains(&h) => "red",
        h if h < 42.0 => {
            if l < 0.32 {
                return "brown".into();
            }
            "orange"
        }
        h if h < 68.0 => "yellow",
        h if h < 160.0 => "green",
        h if h < 195.0 => "cyan",
        h if h < 255.0 => "blue",
        h if h < 290.0 => "purple",
        _ => "pink",
    };
    let shade = if l < 0.3 {
        "dark "
    } else if l > 0.75 {
        "light "
    } else {
        ""
    };
    format!("{shade}{name}")
}

/// A colour helper read aloud: "orange, 255 130 20" (a named colour's
/// name first: "clRed, red, 255 0 0").
pub fn color_said(value: i64) -> String {
    use crate::panels::inspector::values::{color_name, is_system_color};
    if is_system_color(value) {
        return format!("{}, the system's colour", color_name(value).unwrap_or("a system colour"));
    }
    let (r, g, b) = (value & 0xFF, (value >> 8) & 0xFF, (value >> 16) & 0xFF);
    let words = color_words(((r << 16) | (g << 8) | b) as u32);
    match color_name(value) {
        Some(n) => format!("{n}, {words}, {r} {g} {b}"),
        None => format!("{words}, {r} {g} {b}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn site(kind: HelperKind, start: usize, end: usize, value: &str, flag: bool) -> HelperSite {
        let mut h = HelperSite::new(kind, start, end);
        h.value = value.into();
        h.flag = flag;
        h.label = "x".into();
        h.form = "rgb".into();
        h
    }

    #[test]
    fn levels_and_switches() {
        let mut h = Helpers::default();
        h.sites = vec![site(HelperKind::ArgName, 2, 3, "", true), site(HelperKind::Color, 5, 9, "255", false), site(HelperKind::ArgName, 11, 12, "", false)];
        let kinds = |h: &Helpers| h.decos(0, 20).iter().map(|d| (d.at, d.chip)).collect::<Vec<_>>();
        assert_eq!(kinds(&h), [(2, true), (9, false)], "helpful: literal chips and marks");
        h.level = Level::Quiet;
        assert_eq!(kinds(&h), [(9, false)]);
        h.held = true;
        assert_eq!(kinds(&h), [(2, true), (9, false), (11, true)], "Ctrl held: every name");
        h.held = false;
        h.level = Level::Everything;
        assert_eq!(kinds(&h).len(), 3);
        h.off = vec!["color".into(), "names".into()];
        assert!(kinds(&h).is_empty());
        h.level = Level::Off;
        h.off.clear();
        assert!(kinds(&h).is_empty());
        assert_eq!(Level::parse("Everything"), Level::Everything);
        assert_eq!(Level::parse("nonsense"), Level::Helpful);
    }

    #[test]
    fn a_line_being_typed_keeps_its_decorations() {
        let mut h = Helpers::default();
        h.set_sites(vec![site(HelperKind::Color, 5, 9, "255", false)], None);
        // (a chip comes for the line being typed: it waits; the colour follows)
        h.set_sites(vec![site(HelperKind::Color, 5, 9, "65280", false), site(HelperKind::ArgName, 6, 7, "", true)], Some((0, 20)));
        assert_eq!(h.sites.len(), 1);
        assert_eq!(h.sites[0].value, "65280");
        assert!(h.release());
        assert_eq!(h.sites.len(), 2);
    }

    #[test]
    fn sites_follow_the_edits() {
        let mut h = Helpers::default();
        h.set_sites(vec![site(HelperKind::Color, 2, 5, "1", false), site(HelperKind::Color, 10, 14, "1", false), site(HelperKind::Color, 20, 22, "1", false)], None);
        // (two bytes typed at 8: the first stays, the others move)
        let set = ChangeSet::new(vec![rapidr_editor::transaction::Change::insert(8, "ab".to_string())], 30).unwrap();
        h.map(&set);
        assert_eq!(h.sites.iter().map(|s| (s.start, s.end)).collect::<Vec<_>>(), [(2, 5), (12, 16), (22, 24)]);
        // (the second's text deleted: it goes)
        let set = ChangeSet::new(vec![rapidr_editor::transaction::Change::new(11..17, String::new())], 32).unwrap();
        h.map(&set);
        assert_eq!(h.sites.iter().map(|s| (s.start, s.end)).collect::<Vec<_>>(), [(2, 5), (16, 18)]);
        // (typed inside one: its end follows)
        let set = ChangeSet::new(vec![rapidr_editor::transaction::Change::insert(3, "x".to_string())], 26).unwrap();
        h.map(&set);
        assert_eq!(h.sites.iter().map(|s| (s.start, s.end)).collect::<Vec<_>>(), [(2, 6), (17, 19)]);
    }

    #[test]
    fn colours_are_written_as_they_were() {
        assert_eq!(color_text("rgb", 0xFF0000, "RGB(255, 130, 20)"), "RGB(0, 0, 255)");
        assert_eq!(color_text("rgb", 0xFF0000, "rgb(1,2,3)"), "rgb(0,0,255)");
        assert_eq!(color_text("hex", 0x14_82FF, "&H0000ff"), "&H1482ff");
        assert_eq!(color_text("hex", 0x14_82FF, "&H0000FF"), "&H1482FF");
        assert_eq!(color_text("const", 255, "clBlue"), "clRed");
        assert_eq!(color_text("const", 0x14_82FF, "clBlue"), "RGB(255, 130, 20)");
        assert_eq!(color_text("number", 255, "16711680"), "255");
        assert_eq!(string_text("logo.png").as_deref(), Some("\"logo.png\""));
        assert_eq!(string_text("a\"b"), None);
        assert_eq!(choice_text("flags+", &["mbYes".into(), "mbNo".into()]), "mbYes + mbNo");
        assert_eq!(choice_text("flags-or", &["akLeft".into(), "akTop".into()]), "akLeft OR akTop");
        assert_eq!(choice_text("number", &["alClient".into()]), "5");
        assert_eq!(bool_text("word", true, "FALSE"), "TRUE");
        assert_eq!(bool_text("word", false, "True"), "False");
        assert_eq!(bool_text("number", true, "0"), "-1");
    }

    #[test]
    fn colours_in_words() {
        assert_eq!(color_words(0xFF8214), "orange");
        assert_eq!(color_words(0x0000FF), "blue");
        assert_eq!(color_words(0x000080), "dark blue");
        assert_eq!(color_words(0xC0C0C0), "light grey");
        assert_eq!(color_words(0xFFFFFF), "white");
        assert_eq!(color_said(0x14_82FF), "orange, 255 130 20");
        assert_eq!(color_said(255), "clRed, red, 255 0 0");
    }

    #[test]
    fn files_are_read_only_from_the_project() {
        assert_eq!(confined("/p/app", "logo.png").as_deref(), Some("/p/app/logo.png"));
        assert_eq!(confined("/p/app", "img\\logo.png").as_deref(), Some("/p/app/img/logo.png"));
        assert_eq!(confined("/p/app", "../secret.png"), None);
        assert_eq!(confined("/p/app", "img/../../secret.png"), None);
        assert_eq!(confined("/p/app", "/etc/passwd"), None);
        assert_eq!(confined("/p/app", "/p/app/x.png").as_deref(), Some("/p/app/x.png"));
        assert_eq!(confined("/p/app", "/p/application/x.png"), None);
        assert_eq!(confined("/p/app", "https://example.com/a.png"), None);
        let mut h = Helpers::default();
        assert_eq!(h.listed("logo.png"), None);
        h.project_files = vec!["img/Logo.png".into()];
        assert_eq!(h.listed("img\\logo.png"), Some(true));
        assert_eq!(h.listed("other.png"), Some(false));
    }

    #[test]
    fn values_follow_the_names_a_line_uses() {
        let mut h = Helpers::default();
        h.set_values("Score\t42\tInteger\nName\t\"Bob\"\tString\nForm1.Caption\t\"Hi\"\tString", 2, 5);
        let names = |line: usize, t: &str| h.values_on(line, t).iter().map(|v| v.name.as_str()).collect::<Vec<_>>();
        assert_eq!(names(3, "PRINT Score; Name ' Score"), ["Score", "Name"]);
        assert_eq!(names(3, "Form1.Caption = \"Score\""), ["Form1.Caption"]);
        assert!(names(9, "PRINT Score").is_empty(), "outside the paused routine's lines");
    }
}
