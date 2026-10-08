//! RCOMMANDPALETTE's model (docs/ide-components.md §3.8): VS Code's quick
//! open, Xcode's Open Quickly — a box of commands found by typing. Show
//! opens it at the top of its form, over everything, empty, with the
//! commands used last first; the user types a few letters of a command
//! ("category: title" is searched, [`super::fuzzy`]), moves with the
//! arrows and runs one with Enter or a click (OnCommand, after it closed);
//! Escape or the focus leaving it closes it (OnCancel).
//!
//! The kernel (`rapidr-ui-kernel`'s `components/panels/palette.rs`) draws
//! [`Palette::matches`] and keeps the UI state here (the text typed, the
//! highlighted row, the scroll, the hover); what the program hears comes
//! back as a [`User`] action ([`rt_user`]).

use super::fuzzy;
use super::runtime::{basic_bool, truth, Runtime};
use crate::Value;

/// One command.
#[derive(Clone, Debug, PartialEq)]
pub struct Command {
    pub id: String,
    pub title: String,
    /// As shown ("Ctrl+Shift+P"; "Ctrl+K Ctrl+S" for a chord).
    pub shortcut: String,
    /// Put before the title ("File" → "File: Save").
    pub category: String,
    pub icon: String,
    pub enabled: bool,
    /// Made by a prefix ([`Palette::prefixes`]) from what was typed: shown
    /// only while that prefix is typed, never kept among the ones used.
    pub typed: bool,
}

impl Command {
    /// What is searched and shown: "Category: Title".
    pub fn label(&self) -> String {
        if self.category.is_empty() {
            self.title.clone()
        } else {
            format!("{}: {}", self.category, self.title)
        }
    }

    /// Where the title starts in [`Command::label`] (characters).
    pub fn title_at(&self) -> usize {
        if self.category.is_empty() {
            0
        } else {
            self.category.chars().count() + 2
        }
    }
}

/// A command found: its index in [`Palette::commands`] and the matched
/// characters of its label.
#[derive(Clone, Debug, PartialEq)]
pub struct Found {
    pub index: usize,
    pub marks: Vec<usize>,
}

/// RCOMMANDPALETTE's state.
#[derive(Clone, Debug, PartialEq)]
pub struct Palette {
    /// In the order added.
    pub commands: Vec<Command>,
    /// The ids run, the last first.
    pub mru: Vec<String>,
    /// What the user typed.
    pub filter: String,
    pub placeholder: String,
    pub max_rows: usize,
    /// The highlighted row (in [`Palette::matches`]).
    pub active: Option<usize>,
    /// The first row shown.
    pub top: usize,
    /// The row under the mouse, and the row pressed.
    pub hover: Option<usize>,
    pub pressed: Option<usize>,
    /// Shown (between Show and Hide).
    pub shown: bool,
    /// Goes up at every Show: the kernel's search box starts again.
    pub shows: u64,
    /// Typing one of these first (`:` …) gives one command made from the
    /// rest — its title the template with `{}` replaced by it ("Go to line
    /// {}"), its id the prefix and the rest (`:42`) — in place of the list
    /// (VS Code's quick open modes).
    pub prefixes: Vec<(String, String)>,
}

impl Default for Palette {
    fn default() -> Self {
        Palette { commands: Vec::new(), mru: Vec::new(), filter: String::new(), placeholder: "Type a command".into(), max_rows: 10, active: None, top: 0, hover: None, pressed: None, shown: false, shows: 0, prefixes: Vec::new() }
    }
}

crate::panel_models!(Palette);

/// The palette's metrics (logical pixels): the margin around its search
/// box, the box's height, a row's.
pub const PAD: i64 = 8;
pub const BOX: i64 = 30;
pub const ROW: i64 = super::rows::ROW + 4;
/// Its width when the program set none.
pub const WIDTH: i64 = 560;

impl Palette {
    fn index_of(&self, id: &str) -> Option<usize> {
        self.commands.iter().position(|c| c.id.eq_ignore_ascii_case(id))
    }

    /// The commands for what was typed, in order: nothing typed — the ones
    /// used last first (most recent first), then the rest by label; typed —
    /// the matches, best first (ties: used last first, then by label).
    pub fn matches(&self) -> Vec<Found> {
        if self.prefix().is_some() {
            return self.commands.iter().enumerate().filter(|(_, c)| c.typed).map(|(index, _)| Found { index, marks: Vec::new() }).collect();
        }
        let mru_rank = |c: &Command| self.mru.iter().position(|m| m.eq_ignore_ascii_case(&c.id)).unwrap_or(usize::MAX);
        let mut found: Vec<(i32, usize, bool, String, Found)> = Vec::new();
        for (i, c) in self.commands.iter().enumerate().filter(|(_, c)| !c.typed) {
            let label = c.label();
            // (the title alone too, a little better: "s" finds File: Save
            // as soon as Run: Start)
            let whole = fuzzy::score(&self.filter, &label);
            let title = fuzzy::score(&self.filter, &c.title).map(|(s, m)| (s + 5, m.into_iter().map(|k| k + c.title_at()).collect()));
            if let Some((s, marks)) = [whole, title].into_iter().flatten().max_by_key(|(s, _)| *s) {
                found.push((s, mru_rank(c), !c.enabled, label.to_lowercase(), Found { index: i, marks }));
            }
        }
        found.sort_by(|a, b| b.0.cmp(&a.0).then(a.1.cmp(&b.1)).then(a.2.cmp(&b.2)).then(a.3.cmp(&b.3)).then(a.4.index.cmp(&b.4.index)));
        found.into_iter().map(|f| f.4).collect()
    }

    /// The prefix typed first (the longest that fits), with its template.
    pub fn prefix(&self) -> Option<&(String, String)> {
        self.prefixes.iter().filter(|(p, _)| !p.is_empty() && self.filter.starts_with(p.as_str())).max_by_key(|(p, _)| p.len())
    }

    /// The command at row `row` of the matches.
    pub fn command_at(&self, row: usize) -> Option<&Command> {
        self.matches().get(row).map(|f| &self.commands[f.index])
    }

    /// The first row that can be run from `from` going `down` (or up).
    fn enabled_from(&self, found: &[Found], from: usize, down: bool) -> Option<usize> {
        let ok = |i: usize| self.commands[found[i].index].enabled;
        if down {
            (from..found.len()).find(|&i| ok(i))
        } else {
            (0..=from.min(found.len().saturating_sub(1))).rev().find(|&i| ok(i))
        }
    }

    /// The text typed changed: the first command that can run highlighted,
    /// the list at its top.
    pub fn set_filter(&mut self, text: &str) {
        self.filter = text.to_string();
        self.commands.retain(|c| !c.typed);
        if let Some((prefix, template)) = self.prefix().cloned() {
            let rest = self.filter[prefix.len()..].trim().to_string();
            self.commands.push(Command { id: format!("{prefix}{rest}"), title: template.replace("{}", &rest), shortcut: String::new(), category: String::new(), icon: String::new(), enabled: !rest.is_empty(), typed: true });
        }
        let found = self.matches();
        self.active = self.enabled_from(&found, 0, true);
        self.top = 0;
        self.hover = None;
    }

    /// How many rows show (at most MaxRows).
    pub fn rows_shown(&self) -> usize {
        self.matches().len().min(self.max_rows.max(1))
    }

    /// Its height: the search box, then the rows (one for "no match").
    pub fn height(&self) -> i64 {
        PAD + BOX + PAD / 2 + self.rows_shown().max(1) as i64 * ROW + PAD / 2
    }

    /// A key moves the highlight (Up, Down, Page Up, Page Down) over the
    /// commands that can run: whether it was one.
    pub fn nav(&mut self, key: i64) -> bool {
        use super::rows::vk;
        let found = self.matches();
        if found.is_empty() {
            return matches!(key, vk::UP | vk::DOWN | vk::PAGE_UP | vk::PAGE_DOWN);
        }
        let page = self.max_rows.max(1);
        let last = found.len() - 1;
        let at = self.active;
        let next = match (key, at) {
            (vk::DOWN, None) => self.enabled_from(&found, 0, true),
            (vk::UP, None) => self.enabled_from(&found, last, false),
            // (Down at the end wraps to the top; Up at the top to the end)
            (vk::DOWN, Some(i)) => self.enabled_from(&found, i + 1, true).or_else(|| self.enabled_from(&found, 0, true)),
            (vk::UP, Some(i)) => i.checked_sub(1).and_then(|j| self.enabled_from(&found, j, false)).or_else(|| self.enabled_from(&found, last, false)),
            (vk::PAGE_DOWN, i) => self.enabled_from(&found, (i.unwrap_or(0) + page).min(last), false).or_else(|| self.enabled_from(&found, i.unwrap_or(0), true)),
            (vk::PAGE_UP, i) => self.enabled_from(&found, i.unwrap_or(0).saturating_sub(page), true).or(i),
            _ => return false,
        };
        self.active = next;
        self.reveal();
        true
    }

    /// The highlighted row scrolled into view.
    pub fn reveal(&mut self) {
        let Some(a) = self.active else { return };
        let rows = self.max_rows.max(1);
        if a < self.top {
            self.top = a;
        } else if a >= self.top + rows {
            self.top = a + 1 - rows;
        }
    }

    /// The list scrolled by `rows` (the wheel), kept in range.
    pub fn scroll(&mut self, rows: i64) {
        let n = self.matches().len();
        let most = n.saturating_sub(self.max_rows.max(1));
        self.top = (self.top as i64 + rows).clamp(0, most as i64) as usize;
    }

    /// Show: empty, the first command highlighted.
    pub fn open(&mut self) {
        self.shown = true;
        self.shows += 1;
        self.pressed = None;
        self.set_filter("");
    }

    /// A command run: it goes to the front of the ones used last.
    pub fn used(&mut self, id: &str) {
        self.mru.retain(|m| !m.eq_ignore_ascii_case(id));
        self.mru.insert(0, id.to_string());
        self.mru.truncate(50);
    }

    pub fn add(&mut self, c: Command) {
        match self.index_of(&c.id) {
            Some(i) => {
                let enabled = self.commands[i].enabled;
                self.commands[i] = Command { enabled, ..c };
            }
            None => self.commands.push(c),
        }
        self.keep_active();
    }

    pub fn remove(&mut self, id: &str) {
        self.commands.retain(|c| !c.id.eq_ignore_ascii_case(id));
        self.mru.retain(|m| !m.eq_ignore_ascii_case(id));
        self.keep_active();
    }

    /// The highlight still on a row that can run (after a change).
    fn keep_active(&mut self) {
        let found = self.matches();
        match self.active {
            Some(a) if a < found.len() && self.commands[found[a].index].enabled => {}
            _ => self.active = self.enabled_from(&found, 0, true),
        }
        self.top = self.top.min(found.len().saturating_sub(self.max_rows.max(1)));
    }

    /// The highlighted command's id ("" for none).
    pub fn selected(&self) -> String {
        self.active.and_then(|a| self.command_at(a)).map(|c| c.id.clone()).unwrap_or_default()
    }
}

/// What the user did to it (from the kernel).
#[derive(Clone, Debug, PartialEq)]
pub enum User {
    /// The text typed changed (its rows, so its height, may have).
    Filter(String),
    /// A command picked (Enter, a click): it closes, then OnCommand.
    Run(String),
    /// Escape, or the focus left it: it closes, then OnCancel.
    Cancel,
    /// Only its look changed (the row under the mouse): drawn again.
    Redraw,
}

/// Where Show puts palette `name` in its parent: at the top, centred, as
/// wide as Width (560 when none) but 16 pixels from each side at most.
fn place<R: Runtime>(rt: R, name: &str) {
    let parent = rt.get(name, "parent").to_string_val();
    let pw = if parent.is_empty() {
        0
    } else {
        match rt.get(&parent, "clientwidth").to_i64() {
            w if w > 0 => w,
            _ => rt.get(&parent, "width").to_i64(),
        }
    };
    let want = match rt.get(name, "width").to_i64() {
        w if w > 0 => w,
        _ => WIDTH,
    };
    let w = if pw > 0 { want.min(pw - 32).max(120) } else { want };
    let left = if pw > 0 { (pw - w) / 2 } else { rt.get(name, "left").to_i64() };
    let h = with_mut(name, |p| p.height());
    rt.set(name, "left", Value::Integer(left));
    rt.set(name, "top", Value::Integer(8));
    rt.set(name, "width", Value::Integer(w));
    rt.set(name, "height", Value::Integer(h));
}

/// Opens it (Show).
fn show<R: Runtime>(rt: R, name: &str) {
    with_mut(name, Palette::open);
    place(rt, name);
    rt.set(name, "visible", basic_bool(true));
    // (over its neighbours: the kernel's z-order; the keyboard in it)
    rt.restructure();
    rt.focus(name);
}

/// Closes it (Hide, a command run, Escape): whether it was open.
fn hide<R: Runtime>(rt: R, name: &str) -> bool {
    let was = with_mut(name, |p| {
        let was = p.shown;
        p.shown = false;
        p.hover = None;
        p.pressed = None;
        was
    });
    rt.set(name, "visible", basic_bool(false));
    rt.restructure();
    was
}

/// Its methods; `None`: not one of its own.
pub fn rt_method<R: Runtime>(rt: R, name: &str, method: &str, args: &[Value]) -> Option<Value> {
    let s = |i: usize| args.get(i).map(Value::to_string_val).unwrap_or_default();
    match method {
        "addcommand" => {
            if s(0).is_empty() {
                return Some(Value::Null);
            }
            let c = Command { id: s(0), title: s(1), shortcut: s(2), category: s(3), icon: s(4), enabled: true, typed: false };
            with_mut(name, |p| p.add(c));
            resize(rt, name);
            Some(Value::Null)
        }
        "addprefix" => {
            let (prefix, template) = (s(0), s(1));
            with_mut(name, |p| {
                p.prefixes.retain(|(x, _)| *x != prefix);
                if !prefix.is_empty() && !template.is_empty() {
                    p.prefixes.push((prefix, template));
                }
                let f = p.filter.clone();
                p.set_filter(&f);
            });
            Some(Value::Null)
        }
        "removecommand" => {
            with_mut(name, |p| p.remove(&s(0)));
            resize(rt, name);
            Some(Value::Null)
        }
        "clear" => {
            with_mut(name, |p| {
                p.commands.clear();
                p.active = None;
                p.top = 0;
            });
            resize(rt, name);
            Some(Value::Null)
        }
        "commandenabled" => {
            let id = s(0);
            let on = with_mut(name, |p| {
                let i = p.index_of(&id)?;
                if let Some(v) = args.get(1) {
                    p.commands[i].enabled = truth(v);
                    p.keep_active();
                }
                Some(p.commands[i].enabled)
            });
            Some(basic_bool(on.unwrap_or(false)))
        }
        "command" => {
            let i = args.first().map(Value::to_i64).unwrap_or(-1);
            Some(Value::String(with(name, |p| usize::try_from(i).ok().and_then(|i| p.command_at(i)).map(|c| c.id.clone())).flatten().unwrap_or_default()))
        }
        "show" => {
            show(rt, name);
            Some(Value::Null)
        }
        "hide" => {
            hide(rt, name);
            Some(Value::Null)
        }
        _ => None,
    }
}

/// Shown: its height follows its rows.
fn resize<R: Runtime>(rt: R, name: &str) {
    if let Some(h) = with(name, |p| p.shown.then(|| p.height())).flatten() {
        if rt.get(name, "height").to_i64() != h {
            rt.set(name, "height", Value::Integer(h));
        }
    }
}

/// Its properties its model answers.
pub fn rt_get<R: Runtime>(_rt: R, name: &str, prop: &str) -> Option<Value> {
    Some(match prop {
        "filter" => Value::String(with(name, |p| p.filter.clone()).unwrap_or_default()),
        "placeholder" => Value::String(with(name, |p| p.placeholder.clone()).unwrap_or_else(|| Palette::default().placeholder)),
        "count" => Value::Integer(with(name, |p| p.matches().len()).unwrap_or(0) as i64),
        "commandcount" => Value::Integer(with(name, |p| p.commands.len()).unwrap_or(0) as i64),
        "selected" => Value::String(with(name, Palette::selected).unwrap_or_default()),
        "maxrows" => Value::Integer(with(name, |p| p.max_rows).unwrap_or(10) as i64),
        _ => return None,
    })
}

/// Its properties its model keeps: whether `prop` was one.
pub fn rt_set<R: Runtime>(rt: R, name: &str, prop: &str, v: &Value) -> bool {
    match prop {
        "filter" => with_mut(name, |p| p.set_filter(&v.to_string_val())),
        "placeholder" => with_mut(name, |p| p.placeholder = v.to_string_val()),
        "maxrows" => with_mut(name, |p| {
            p.max_rows = v.to_i64().clamp(1, 100) as usize;
            p.reveal();
        }),
        "selected" => {
            let id = v.to_string_val();
            with_mut(name, |p| {
                if let Some(row) = p.matches().iter().position(|f| p.commands[f.index].id.eq_ignore_ascii_case(&id)) {
                    p.active = Some(row);
                    p.reveal();
                }
            });
        }
        _ => return false,
    }
    if matches!(prop, "filter" | "maxrows") {
        resize(rt, name);
    }
    true
}

/// What the user did.
pub fn rt_user<R: Runtime>(rt: R, name: &str, action: User) {
    match action {
        User::Filter(text) => {
            with_mut(name, |p| {
                if p.filter != text {
                    p.set_filter(&text);
                }
            });
            resize(rt, name);
        }
        User::Run(id) => {
            let ok = with_mut(name, |p| {
                let ok = p.index_of(&id).is_some_and(|i| p.commands[i].enabled);
                if ok && !p.commands.iter().any(|c| c.typed && c.id.eq_ignore_ascii_case(&id)) {
                    p.used(&id);
                }
                ok
            });
            if !ok {
                return;
            }
            hide(rt, name);
            rt.fire(name, "OnCommand", &[Value::String(id)]);
        }
        User::Redraw => {}
        User::Cancel => {
            if hide(rt, name) {
                rt.fire(name, "OnCancel", &[]);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::panels::rows::vk;

    fn cmd(id: &str, title: &str, category: &str) -> Command {
        Command { id: id.into(), title: title.into(), shortcut: String::new(), category: category.into(), icon: String::new(), enabled: true, typed: false }
    }

    fn palette() -> Palette {
        let mut p = Palette::default();
        p.add(cmd("save", "Save", "File"));
        p.add(cmd("saveall", "Save All", "File"));
        p.add(cmd("run", "Start", "Run"));
        p.add(cmd("about", "About RapidR", ""));
        p
    }

    fn ids(p: &Palette) -> Vec<String> {
        p.matches().iter().map(|f| p.commands[f.index].id.clone()).collect()
    }

    #[test]
    fn a_prefix_gives_one_command_from_what_was_typed() {
        let mut p = palette();
        p.prefixes.push((":".into(), "Go to line {}".into()));
        p.set_filter(":42");
        assert_eq!(ids(&p), [":42"]);
        assert_eq!(p.command_at(0).unwrap().title, "Go to line 42");
        assert_eq!(p.selected(), ":42");
        p.set_filter(":");
        assert_eq!(p.selected(), "", "nothing to go to yet");
        p.set_filter("sav");
        assert!(!ids(&p).iter().any(|i| i.starts_with(':')));
        assert!(p.commands.iter().all(|c| !c.typed));
    }

    #[test]
    fn nothing_typed_shows_them_by_label() {
        let p = palette();
        assert_eq!(ids(&p), ["about", "save", "saveall", "run"]);
        assert_eq!(p.commands[0].label(), "File: Save");
        assert_eq!(p.commands[0].title_at(), 6);
    }

    #[test]
    fn the_ones_used_last_come_first() {
        let mut p = palette();
        p.used("run");
        p.used("saveall");
        assert_eq!(ids(&p), ["saveall", "run", "about", "save"]);
        p.used("run");
        assert_eq!(ids(&p)[..2], ["run", "saveall"]);
        p.remove("run");
        assert_eq!(p.mru, ["saveall"]);
    }

    #[test]
    fn typing_finds_and_ranks() {
        let mut p = palette();
        p.set_filter("sa");
        // (Run: STArt's letters too, after the closer ones)
        assert_eq!(ids(&p), ["save", "saveall", "run"]);
        assert_eq!(p.matches()[0].marks, vec![6, 7]);
        // (words in any order; "category: title" searched)
        p.set_filter("all file");
        assert_eq!(ids(&p), ["saveall"]);
        p.set_filter("run st");
        assert_eq!(ids(&p), ["run"]);
        p.set_filter("zz");
        assert!(ids(&p).is_empty());
        assert_eq!(p.active, None);
        // (its height: one row for "no match")
        assert_eq!(p.height(), PAD + BOX + PAD / 2 + ROW + PAD / 2);
    }

    #[test]
    fn disabled_ones_are_skipped() {
        let mut p = palette();
        p.commands[0].enabled = false; // (File: Save)
        p.set_filter("");
        assert_eq!(p.active, Some(0)); // (About)
        assert!(p.nav(vk::DOWN));
        assert_eq!(p.selected(), "saveall");
        assert!(p.nav(vk::UP));
        assert_eq!(p.selected(), "about");
        // (Up at the top wraps to the end)
        assert!(p.nav(vk::UP));
        assert_eq!(p.selected(), "run");
        p.set_filter("save");
        assert_eq!(p.selected(), "saveall");
    }

    #[test]
    fn the_highlight_stays_in_view() {
        let mut p = Palette { max_rows: 2, ..Palette::default() };
        for i in 0..6 {
            p.add(cmd(&format!("c{i}"), &format!("Command {i}"), ""));
        }
        p.open();
        assert_eq!(p.rows_shown(), 2);
        for _ in 0..3 {
            p.nav(vk::DOWN);
        }
        assert_eq!((p.active, p.top), (Some(3), 2));
        p.nav(vk::PAGE_UP);
        assert_eq!((p.active, p.top), (Some(1), 1));
        p.scroll(10);
        assert_eq!(p.top, 4);
    }

    #[test]
    fn adding_again_changes_it() {
        let mut p = palette();
        p.commands[1].enabled = false;
        p.add(cmd("saveall", "Save Everything", "File"));
        assert_eq!(p.commands.len(), 4);
        assert_eq!(p.commands[1].title, "Save Everything");
        assert!(!p.commands[1].enabled);
    }
}
