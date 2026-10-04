//! The GUI tests' environment, parsed once for the desktop host:
//!
//! - `RAPIDR_CAPTURE=<prefix>`: after `RAPIDR_CAPTURE_DELAY` seconds (default
//!   1.5) the host fires the test's events, prints the dump, saves every
//!   shown window as `<prefix>-<n>.bmp` and exits. User input is ignored.
//! - `RAPIDR_TEST_EVENTS=b1.onclick,edit.__key_65,…`: the events, one per
//!   turn of the loop ([`Action`] lists the synthetic ones).
//! - `RAPIDR_TEST_DUMP=lbl.caption,frm.__shown`: `component.property`s to
//!   print as `lbl.caption=…` (`__shown`: 1 when the component is visible
//!   up to its window, else 0).
//! - `RAPIDR_TEST_RESIZE=w,h`: the frontmost form resized (Width, Height)
//!   as a user dragging its border would, before the events.
//! - `RAPIDR_TEST_SPLIT=splitter:delta`: a QSPLITTER dragged by `delta`.
//! - `RAPIDR_TEST_FILE_DIALOG=a;b`: what Open/Save dialogs pick (empty:
//!   Cancel).
//! - `RAPIDR_TEST_COLOR_DIALOG=255;` / `RAPIDR_TEST_FONT_DIALOG=…`: what
//!   each colour / font dialog answers in turn (empty: Cancel).

use rapidr_value::input::Mouse;

/// A GUI test drives the program (`RAPIDR_CAPTURE` or `RAPIDR_TEST_EVENTS`):
/// it makes no sound (a message box's beep) and asks nothing of the system
/// it doesn't need.
pub fn under_test() -> bool {
    std::env::var_os("RAPIDR_CAPTURE").is_some() || std::env::var_os("RAPIDR_TEST_EVENTS").is_some()
}

/// A GUI test's run (`RAPIDR_CAPTURE` set).
#[derive(Clone, Debug, PartialEq)]
pub struct Capture {
    /// The BMPs' path prefix.
    pub prefix: String,
    /// Seconds before the script starts.
    pub delay: f64,
    pub events: Vec<TestEvent>,
    /// `RAPIDR_TEST_RESIZE`: the frontmost form's new Width and Height.
    pub resize: Option<(i32, i32)>,
    /// `RAPIDR_TEST_SPLIT`: the splitter and how far it's dragged.
    pub split: Option<(String, i64)>,
}

impl Capture {
    pub fn from_env() -> Option<Capture> {
        Capture::parse(|k| std::env::var(k).ok())
    }

    /// The run `var` (an environment) asks for, if any.
    pub fn parse(var: impl Fn(&str) -> Option<String>) -> Option<Capture> {
        let prefix = var("RAPIDR_CAPTURE")?;
        let delay = var("RAPIDR_CAPTURE_DELAY").and_then(|d| d.parse().ok()).unwrap_or(1.5);
        let events = parse_events(&var("RAPIDR_TEST_EVENTS").unwrap_or_default());
        let resize = var("RAPIDR_TEST_RESIZE").and_then(|r| {
            let (w, h) = r.split_once(',')?;
            Some((w.trim().parse::<i32>().ok()?, h.trim().parse::<i32>().ok()?))
        });
        let split = var("RAPIDR_TEST_SPLIT").and_then(|r| {
            let (name, delta) = r.rsplit_once(':')?;
            Some((name.trim().to_string(), delta.trim().parse::<i64>().ok()?))
        });
        Some(Capture { prefix, delay, events, resize, split })
    }
}

/// One of `RAPIDR_TEST_EVENTS`: `component.event`.
#[derive(Clone, Debug, PartialEq)]
pub struct TestEvent {
    /// The component as the test wrote it.
    pub comp: String,
    pub action: Action,
}

impl TestEvent {
    /// `comp` lowercased (the store's and the widgets' key).
    pub fn comp_lower(&self) -> String {
        self.comp.to_lowercase()
    }
}

/// What a test event does.
#[derive(Clone, Debug, PartialEq)]
pub enum Action {
    /// Any other event, lowercased: its handler fired as the user's would
    /// (`onclick` on a toggle button presses it through its group first).
    Fire(String),
    /// `__key_N`: virtual key N pressed and released with the component
    /// focused.
    Key(i64),
    /// `__mousedown_x_y`, `__mouseup_x_y`, `__mousemove_x_y`: the mouse at
    /// (x, y) in the component (a press is a single click).
    Mouse(Mouse, i64, i64),
    /// (the input lane's) `__dblclick_x_y`: a double click at (x, y) in the
    /// component — press, release, press (the double), release.
    DblClick(i64, i64),
    /// `__item_i`: a list's (or combo box's) item i picked, then OnClick.
    Item(i64),
    /// `__node_i`: a tree's node i picked, then OnClick.
    Node(i64),
    /// `__toggle_i`: OnClick, then a tree's node i expanded or collapsed.
    Toggle(i64),
    /// `__cell_c_r`: a grid's cell (c, r) clicked.
    Cell(i64, i64),
    /// `__edit`: F2, the selected node (list view: item) edited.
    Edit,
    /// `__enter`: "Renamed" typed into that editor, then Enter.
    Enter,
    /// `__escape`: the edit dropped.
    Escape,
    /// `__close`: the window's close button.
    Close,
    /// A synthetic event malformed (`__key_`, `__mouse…`, `__dblclick…`, `__item_`,
    /// `__node_`, `__toggle_`, `__edit…`, `__enter…`, `__escape…` without
    /// the right numbers): nothing happens.
    Ignored,
}

/// `RAPIDR_TEST_EVENTS`'s list; items without a `.` are dropped.
pub fn parse_events(list: &str) -> Vec<TestEvent> {
    list.split(',').map(str::trim).filter(|e| !e.is_empty()).filter_map(parse_event).collect()
}

/// `component.event` (split at the last `.`).
pub fn parse_event(item: &str) -> Option<TestEvent> {
    let (comp, event) = item.rsplit_once('.')?;
    let event = event.to_ascii_lowercase();
    // The numbers after a prefix, `_`-separated; what isn't one is skipped.
    let nums = |prefix: &str| -> Option<Vec<i64>> { event.strip_prefix(prefix).map(|r| r.split('_').filter_map(|n| n.parse().ok()).collect()) };
    let one = |prefix: &str| nums(prefix).and_then(|n| <[i64; 1]>::try_from(n).ok()).map(|[i]| i);
    let mouse = [("__mousedown_", Mouse::Down), ("__mouseup_", Mouse::Up), ("__mousemove_", Mouse::Move)]
        .into_iter()
        .find_map(|(p, kind)| nums(p).and_then(|n| <[i64; 2]>::try_from(n).ok()).map(|[x, y]| (kind, x, y)));
    let double = nums("__dblclick_").and_then(|n| <[i64; 2]>::try_from(n).ok());
    let action = if let Some(i) = one("__node_") {
        Action::Node(i)
    } else if event == "__edit" {
        Action::Edit
    } else if event == "__enter" {
        Action::Enter
    } else if event == "__escape" {
        Action::Escape
    } else if let Some(i) = one("__toggle_") {
        Action::Toggle(i)
    } else if let Some(i) = one("__item_") {
        Action::Item(i)
    } else if let Some(vk) = one("__key_") {
        Action::Key(vk)
    } else if let Some((kind, x, y)) = mouse {
        Action::Mouse(kind, x, y)
    } else if let Some([x, y]) = double {
        Action::DblClick(x, y)
    } else if ["__key_", "__mouse", "__dblclick", "__item_", "__node_", "__toggle_", "__edit", "__enter", "__escape"].iter().any(|p| event.starts_with(p)) {
        Action::Ignored
    } else if event == "__close" {
        Action::Close
    } else if let Some((c, r)) = event.strip_prefix("__cell_").and_then(|rc| {
        let (c, r) = rc.split_once('_')?;
        Some((c.parse::<i64>().ok()?, r.parse::<i64>().ok()?))
    }) {
        Action::Cell(c, r)
    } else {
        Action::Fire(event)
    };
    Some(TestEvent { comp: comp.to_string(), action })
}

/// One of `RAPIDR_TEST_DUMP`: `component.property`.
#[derive(Clone, Debug, PartialEq)]
pub struct DumpItem {
    /// As the test wrote it (trimmed): the printed line's key.
    pub label: String,
    pub comp: String,
    /// The property; `__shown` asks the host.
    pub prop: String,
}

impl DumpItem {
    pub fn is_shown(&self) -> bool {
        self.prop == "__shown"
    }
}

/// `RAPIDR_TEST_DUMP`'s list; items without a `.` are dropped.
pub fn parse_dump(list: &str) -> Vec<DumpItem> {
    list.split(',')
        .map(str::trim)
        .filter(|p| !p.is_empty())
        .filter_map(|p| {
            let (comp, prop) = p.rsplit_once('.')?;
            Some(DumpItem { label: p.to_string(), comp: comp.to_string(), prop: prop.to_string() })
        })
        .collect()
}

/// The dump's lines, `label=value`: `shown` answers `__shown`, `get` the
/// properties (as the program would read them).
pub fn dump_lines(items: &[DumpItem], shown: impl Fn(&str) -> bool, get: impl Fn(&str, &str) -> String) -> Vec<String> {
    items
        .iter()
        .map(|d| {
            let value = if d.is_shown() { i64::from(shown(&d.comp)).to_string() } else { get(&d.comp, &d.prop) };
            format!("{}={value}", d.label)
        })
        .collect()
}

/// Prints `RAPIDR_TEST_DUMP`'s lines.
pub fn print_dump(shown: impl Fn(&str) -> bool, get: impl Fn(&str, &str) -> String) {
    for line in dump_lines(&parse_dump(&std::env::var("RAPIDR_TEST_DUMP").unwrap_or_default()), shown, get) {
        println!("{line}");
    }
}

/// `RAPIDR_TEST_FILE_DIALOG`: the paths a test's Open/Save dialog picks
/// (at most one unless `multi`; none: Cancel), or `None` to ask the user.
pub fn file_dialog_answer(multi: bool) -> Option<Vec<String>> {
    std::env::var("RAPIDR_TEST_FILE_DIALOG").ok().map(|answer| file_dialog_paths(&answer, multi))
}

pub fn file_dialog_paths(answer: &str, multi: bool) -> Vec<String> {
    answer.split(';').filter(|p| !p.is_empty()).take(if multi { usize::MAX } else { 1 }).map(str::to_string).collect()
}

// (the dialogs lane's)
thread_local! {
    /// How many answers each answering hook has given.
    static ANSWERED: std::cell::RefCell<std::collections::HashMap<&'static str, usize>> = std::cell::RefCell::new(std::collections::HashMap::new());
}

/// Hook `var`'s next answer, `;`-separated, one per dialog in turn
/// (`None`: the hook isn't set; past its last: an empty answer, Cancel).
fn next_answer(var: &'static str) -> Option<String> {
    let list = std::env::var(var).ok()?;
    let n = ANSWERED.with(|a| {
        let mut a = a.borrow_mut();
        let n = a.entry(var).or_insert(0);
        *n += 1;
        *n - 1
    });
    Some(list.split(';').nth(n).unwrap_or("").trim().to_string())
}

/// `RAPIDR_TEST_COLOR_DIALOG=255;&HFF00;`: what each QCOLORDIALOG.Execute
/// answers in turn — a colour for OK, empty for Cancel (`None`: ask the
/// user). The custom colours stay.
pub fn color_dialog_answer() -> Option<Option<i64>> {
    next_answer("RAPIDR_TEST_COLOR_DIALOG").map(|a| rapidr_value::color_dialog::parse_color(&a))
}

/// `RAPIDR_TEST_FONT_DIALOG=Courier New,14,bu,255;`: what each
/// QFONTDIALOG.Execute answers in turn — name, size, styles (b i u s),
/// colour for OK, empty for Cancel (`None`: ask the user).
pub fn font_dialog_answer() -> Option<Option<rapidr_value::objects::font::Font>> {
    next_answer("RAPIDR_TEST_FONT_DIALOG").map(|a| rapidr_value::font_dialog::parse_answer(&a))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn act(item: &str) -> Action {
        parse_event(item).unwrap().action
    }

    #[test]
    fn events_are_component_dot_event_split_at_the_last_dot() {
        let e = parse_events(" b1.OnClick , ,nodot, lbl(1).onchange,frm.sub.onkeydown");
        assert_eq!(e.len(), 3);
        assert_eq!(e[0], TestEvent { comp: "b1".into(), action: Action::Fire("onclick".into()) });
        assert_eq!(e[1].comp, "lbl(1)");
        assert_eq!(e[2].comp, "frm.sub");
        assert_eq!(TestEvent { comp: "Btn".into(), action: Action::Close }.comp_lower(), "btn");
    }

    #[test]
    fn synthetic_input() {
        assert_eq!(act("e.__key_65"), Action::Key(65));
        assert_eq!(act("e.__KEY_39"), Action::Key(39));
        assert_eq!(act("c.__mousedown_190_10"), Action::Mouse(Mouse::Down, 190, 10));
        assert_eq!(act("c.__mouseup_1_2"), Action::Mouse(Mouse::Up, 1, 2));
        assert_eq!(act("c.__mousemove_-3_4"), Action::Mouse(Mouse::Move, -3, 4));
        assert_eq!(act("p.__dblclick_3_4"), Action::DblClick(3, 4));
        assert_eq!(act("l.__item_2"), Action::Item(2));
        assert_eq!(act("t.__node_0"), Action::Node(0));
        assert_eq!(act("t.__toggle_1"), Action::Toggle(1));
        assert_eq!(act("g.__cell_2_1"), Action::Cell(2, 1));
        assert_eq!(act("t.__edit"), Action::Edit);
        assert_eq!(act("t.__enter"), Action::Enter);
        assert_eq!(act("t.__escape"), Action::Escape);
        assert_eq!(act("frm.__close"), Action::Close);
    }

    #[test]
    fn numbers_that_dont_parse_are_skipped() {
        assert_eq!(act("e.__key_65_x"), Action::Key(65));
        assert_eq!(act("e.__key_1_2"), Action::Ignored);
        assert_eq!(act("e.__key_"), Action::Ignored);
        assert_eq!(act("c.__mousedown_5"), Action::Ignored);
        assert_eq!(act("c.__mousewheel_1_2"), Action::Ignored);
        assert_eq!(act("p.__dblclick_3"), Action::Ignored);
        assert_eq!(act("t.__editing"), Action::Ignored);
        assert_eq!(act("t.__node_a"), Action::Ignored);
        // A cell needs both numbers; otherwise it's an event of that name.
        assert_eq!(act("g.__cell_2_x"), Action::Fire("__cell_2_x".into()));
        assert_eq!(act("g.__cell_2_1_3"), Action::Fire("__cell_2_1_3".into()));
    }

    #[test]
    fn capture_reads_its_environment() {
        let env = |pairs: &'static [(&'static str, &'static str)]| move |k: &str| pairs.iter().find(|(n, _)| *n == k).map(|(_, v)| v.to_string());
        assert_eq!(Capture::parse(env(&[("RAPIDR_TEST_EVENTS", "b.onclick")])), None);
        let c = Capture::parse(env(&[("RAPIDR_CAPTURE", "/tmp/x")])).unwrap();
        assert_eq!((c.prefix.as_str(), c.delay, c.events.len(), c.resize, c.split), ("/tmp/x", 1.5, 0, None, None));
        let c = Capture::parse(env(&[
            ("RAPIDR_CAPTURE", "p"),
            ("RAPIDR_CAPTURE_DELAY", "0.25"),
            ("RAPIDR_TEST_EVENTS", "b.onclick,tb.__key_39"),
            ("RAPIDR_TEST_RESIZE", " 300 , 200 "),
            ("RAPIDR_TEST_SPLIT", "a:b:-40"),
        ]))
        .unwrap();
        assert_eq!(c.delay, 0.25);
        assert_eq!(c.events[1].action, Action::Key(39));
        assert_eq!(c.resize, Some((300, 200)));
        assert_eq!(c.split, Some(("a:b".into(), -40)));
        let c = Capture::parse(env(&[("RAPIDR_CAPTURE", "p"), ("RAPIDR_CAPTURE_DELAY", "soon"), ("RAPIDR_TEST_RESIZE", "300"), ("RAPIDR_TEST_SPLIT", "s:far")])).unwrap();
        assert_eq!((c.delay, c.resize, c.split), (1.5, None, None));
    }

    #[test]
    fn dump_formats_label_equals_value() {
        let items = parse_dump(" lbl.caption ,,frm.__shown,nodot,t.enabled");
        assert_eq!(items.len(), 3);
        let lines = dump_lines(&items, |c| c == "frm", |c, p| format!("{c}:{p}"));
        assert_eq!(lines, ["lbl.caption=lbl:caption", "frm.__shown=1", "t.enabled=t:enabled"]);
        let lines = dump_lines(&parse_dump("x.__shown"), |_| false, |_, _| unreachable!());
        assert_eq!(lines, ["x.__shown=0"]);
    }

    #[test]
    fn file_dialog_answers() {
        assert_eq!(file_dialog_paths("a;b;", false), ["a"]);
        assert_eq!(file_dialog_paths("a;;b", true), ["a", "b"]);
        assert!(file_dialog_paths("", true).is_empty());
    }
}
