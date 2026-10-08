//! The program's windows as the host keeps them: what the glue asks of the
//! desktop or web host ([`Windows`]), and the two things it leaves for the
//! host's next turn — window commands ([`WindowOp`], [`take_ops`]) and
//! "something changed" ([`invalidate`], [`restructure`], [`take_notify`]).
//!
//! The host may be busy when the program asks (a store hook inside one of
//! its callbacks), so nothing here reaches into it: the commands wait in a
//! queue its next turn takes (the desktop's `sync_desk`, before each pump).

use std::cell::{Cell, RefCell};

use rapidr_value::input::Mouse;

/// A window command for the host's next turn.
#[derive(Clone, Debug, PartialEq)]
pub enum WindowOp {
    /// Made (the first time) and shown, on top.
    Show(String),
    Hide(String),
    Title(String, String),
    /// Its inside (client area and an in-window menu bar), logical pixels.
    Size(String, (i64, i64)),
    /// Its frame's top left on the screen.
    Position(String, (i64, i64)),
    /// A frame and title bar (BorderStyle <> bsNone).
    Border(String, bool),
    Icon(String, Option<Icon>),
    Minimize(String),
    /// A pop-up menu shown by the host (form, menu, x, y in its inside).
    Popup(String, String, i64, i64),
    // (the WindowState lane's)
    /// The window maximized, minimized or restored (wsNormal …).
    State(String, i64),
    // (the DirectX lane's)
    /// The window covers the screen without a frame (a QDXSCREEN's
    /// FullScreen); the system's new size comes back as a resize.
    Fullscreen(String),
}

/// A window's picture (RGBA, straight): a form's IcoHandle / Icon, else
/// the application's.
#[derive(Clone, Debug, PartialEq)]
pub struct Icon {
    pub width: u32,
    pub height: u32,
    pub rgba: Vec<u8>,
}

/// A GUI test's input (`RAPIDR_TEST_EVENTS`, `RAPIDR_TEST_RESIZE`; the
/// script, `script.rs`), which the host routes through the kernel as the
/// user's would be.
#[derive(Clone, Debug, PartialEq)]
pub enum ScriptInput {
    /// `comp.__key_N`: the component focused, the key pressed and released
    /// (`__key_N_S`: with RapidQ's Shift state S held — ssShift 256, ssCtrl
    /// 16, ssAlt 1).
    Key { comp: String, vk: i64, state: i64 },
    /// `comp.__mousedown_x_y` …: the mouse at (x, y) in the component (a
    /// press is a single click, however soon after another).
    Mouse { comp: String, kind: Mouse, x: i64, y: i64 },
    /// (the input lane's) `comp.__dblclick_x_y`: a double click at (x, y)
    /// in the component.
    DblClick { comp: String, x: i64, y: i64 },
    /// (the lists lane's) The component's own step (`__item_i`, `__node_i`,
    /// `__toggle_i`, `__cell_c_r`, `__edit`, `__enter`, `__escape`:
    /// `FormUi::test_action`) on form `form`.
    Step { form: String, comp: String, step: String },
    /// `RAPIDR_TEST_RESIZE=w,h`: the frontmost form resized (Width, Height)
    /// as a user dragging its border would.
    Resize { w: i64, h: i64 },
    /// (timers during native menu tracking) `__hold_ms`: the next pump held,
    /// as a native menu the user keeps open would hold it.
    Hold(i64),
}

/// What the glue asks of the host its windows are on. runtime-core
/// implements it over the desktop host (winit or headless); the web runtime
/// will over the browser's (docs/web-host-plan.md, "W2 results"). As
/// [`Program`](crate::Program), a unit struct passed by value. **Nothing
/// here waits for the user**: the waits that do (a native build's ShowModal
/// loop, INPUT$, the dialogs) are the runtime's, around these.
pub trait Windows: Copy + 'static {
    // ---- the host ----

    /// The host started, the first time a window is needed (a console
    /// program never opens the system's windowing).
    fn start(self);
    fn started(self) -> bool;
    /// The window commands queued so far ([`take_ops`]) carried out now —
    /// the desktop pumps once without waiting, so a window exists before
    /// its OnShow. Nothing is dispatched.
    fn flush(self);
    /// The events the host queued, dispatched now, each to completion (a
    /// program form's through [`crate::dispatch::dispatch`]).
    fn dispatch_pending(self);

    // ---- what only the host knows ----

    /// No window system to ask (the desktop's headless host of the GUI
    /// tests): a WindowState change is simulated
    /// ([`crate::forms::simulate_state`]).
    fn headless(self) -> bool;
    /// The scale tests force (`RAPIDR_SCALE`): every window's, whatever
    /// its screen says.
    fn forced_scale(self) -> Option<f64>;
    /// The scale form `form`'s window shows at (the host's default for one
    /// it hasn't made); `None` when it can't say now.
    fn window_scale(self, form: &str) -> Option<f64>;
    /// The screen's size (logical pixels).
    fn screen(self) -> (i64, i64);
    /// The screen less the task bar / menu bar.
    fn work_area(self) -> (i64, i64);
    /// A QMAINMENU is a bar inside its form's window (everywhere but on
    /// macOS' menu bar).
    fn menu_in_window(self) -> bool;
    /// The windows shown, bottom to top (the frontmost last).
    fn stacking(self) -> Vec<String>;
    /// The form component `comp` is on, and where it is in that window's
    /// inside (logical; an in-window menu bar included), from the kernel's
    /// tree.
    fn place_of(self, comp: &str) -> Option<(String, (i64, i64))>;

    // ---- what the glue has the host do ----

    /// Form `form`'s window resized (its inside) and moved by the system, as
    /// a user's drag would be — the kernel lays out; the program hears it
    /// when the host's events are dispatched (the headless host's maximize).
    fn system_resized(self, form: &str, inside: (i64, i64), at: (i64, i64));
    /// Pop-up menu `menu` open at (x, y) of `form`'s window's inside
    /// (`program`: the program's Popup, not an AutoPopup) — the host's own
    /// menu or the kernel's; its pick comes back as a `MenuPick`.
    fn open_popup(self, form: &str, menu: &str, x: i64, y: i64, program: bool);
    /// Whether a kernel-drawn pop-up menu is open on `form` (a wait for one
    /// lasts that long: `waits::Wait::Menu`). A host whose pop-up menus
    /// never hold the program has none to say.
    fn popup_open(self, _form: &str) -> bool {
        false
    }

    // ---- the dialogs (`crate::dialogs`) ----

    /// A kernel-drawn dialog's window (`id`, the dialog's form: its parts
    /// are in `crate::dialogs::with_store`'s store) made and shown: titled
    /// `title`, its inside `size`, a dialog's frame (not resizable, a close
    /// box), centred on the screen. The glue puts it on the modal list.
    fn open_dialog(self, id: &str, title: &str, size: (i64, i64));
    /// Dialog `id`'s window gone for good (it answered).
    fn close_dialog(self, id: &str);
    /// Dialog `id` grew (a colour dialog's editor opened): its window too.
    fn dialog_resized(self, _id: &str, _size: (i64, i64)) {}
    /// A message box's sound as it shows (Windows' MessageBox beeps its
    /// icon's); never under a test (`crate::testhooks::under_test`).
    fn beep(self, _icon: Option<rapidr_value::dialogs::MsgIcon>) {}
    /// The system's font families, for a font dialog's list (besides the
    /// shared ones, `choose_dialogs::font_names`).
    fn font_families(self) -> Vec<String> {
        Vec::new()
    }
    /// The host's own Open / Save dialog for `req` (request `id`), over
    /// window `form` (`None`: the program has none shown); its answer comes
    /// later ([`Windows::files_answer`]) — nothing waits here.
    fn ask_files(self, id: u64, form: Option<&str>, req: &crate::file_dialog::Request);
    /// Request `id`'s paths once its dialog closed (none: cancelled),
    /// `None` while it's open.
    fn files_answer(self, id: u64) -> Option<Vec<String>>;

    // ---- a GUI test (`script.rs`) ----

    /// The script's input, through the kernel's routing.
    fn script_input(self, input: ScriptInput);
    /// The test's end: every shown window captured, the accessibility trees
    /// written (`RAPIDR_TEST_A11Y`), the program ended — the desktop's
    /// process exits; a page can't, so the web's keeps the results and ends
    /// the program, and the script is over.
    fn capture_and_end(self, prefix: &str);
}

thread_local! {
    static OPS: RefCell<Vec<WindowOp>> = const { RefCell::new(Vec::new()) };
    /// Something to draw again: (paint, the component tree changed).
    static NOTIFY: Cell<(bool, bool)> = const { Cell::new((false, false)) };
}

/// A window command for the host's next turn.
pub fn push_op(op: WindowOp) {
    OPS.with(|o| o.borrow_mut().push(op));
}

/// The window commands queued, oldest first (the host's turn takes them).
pub fn take_ops() -> Vec<WindowOp> {
    OPS.with(|o| std::mem::take(&mut *o.borrow_mut()))
}

/// Something drawn changed: painted again at the host's next turn.
pub fn invalidate() {
    NOTIFY.with(|n| n.set((true, n.get().1)));
}

/// Components added, removed or moved between parents.
pub fn restructure() {
    NOTIFY.with(|n| n.set((n.get().0, true)));
}

/// What changed since the host's last turn: (paint, the component tree).
pub fn take_notify() -> (bool, bool) {
    NOTIFY.with(|n| n.replace((false, false)))
}
