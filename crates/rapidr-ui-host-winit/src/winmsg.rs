//! A form's WndProc on Windows (`rapidr_value::window_messages`): every
//! message Windows sends or posts to the form's window, kept here as it
//! arrives and handed to the program after the host's turn ([`take`]:
//! runtime-core fires the form's OnWndProc with each). The window is
//! subclassed with comctl32's `SetWindowSubclass`, which sees the sent
//! messages (WM_SIZE, WM_ACTIVATE …) and the posted ones (WM_USER + n,
//! winmm's MM_MIM_DATA, the mouse and keys) alike, and stacks with the
//! other subclasses of the window (AccessKit's); the message then goes on
//! to winit as before — the handler's answer isn't Windows' (RapidQ ignores
//! it too). Only the windows of forms whose WndProc is bound are
//! subclassed. The host never calls program code (the crate's rule), so
//! the handler runs once the pump returns: in the order the messages came.
//!
//! Other systems have no Windows messages: nothing is ever kept.

/// One message as the window got it: (hWnd, uMsg, wParam, lParam).
pub type Message = (i64, u32, u64, i64);

/// The messages kept since the last call, oldest first.
pub fn take() -> Vec<Message> {
    #[cfg(target_os = "windows")]
    {
        win::take()
    }
    #[cfg(not(target_os = "windows"))]
    {
        Vec::new()
    }
}

/// Whether messages are kept — some arrive outside the host's turn (the
/// program's own `SendMessage` to its form runs the window procedure at
/// once): the next turn doesn't wait then.
pub fn pending() -> bool {
    #[cfg(target_os = "windows")]
    {
        win::pending()
    }
    #[cfg(not(target_os = "windows"))]
    {
        false
    }
}

#[cfg(target_os = "windows")]
pub(crate) use win::{follow, hook};

#[cfg(target_os = "windows")]
mod win {
    use std::cell::{Cell, RefCell};

    use windows_sys::Win32::Foundation::{HWND, LPARAM, LRESULT, WPARAM};
    use windows_sys::Win32::Graphics::Gdi::GetUpdateRect;
    use windows_sys::Win32::UI::Shell::{DefSubclassProc, SetWindowSubclass};
    use winit::window::Window;

    use super::Message;

    /// The subclass's id among the window's ("RRWM").
    const ID: usize = 0x5252_574D;
    /// At most this many kept between two turns (ten times Windows' own
    /// limit for a thread's posted messages; the program's own SendMessage
    /// in a loop adds one each without a turn between): a program stuck in a
    /// long computation doesn't grow the queue for ever — newer messages are
    /// dropped then, as PostMessage fails on a full queue.
    const LIMIT: usize = 100_000;

    thread_local! {
        static QUEUE: RefCell<Vec<Message>> = const { RefCell::new(Vec::new()) };
        /// The listening forms' revision the windows follow.
        static SEEN: Cell<u64> = const { Cell::new(0) };
    }

    pub fn take() -> Vec<Message> {
        QUEUE.try_with(|q| q.try_borrow_mut().map(|mut q| std::mem::take(&mut *q)).unwrap_or_default()).unwrap_or_default()
    }

    pub fn pending() -> bool {
        QUEUE.try_with(|q| q.try_borrow().is_ok_and(|q| !q.is_empty())).unwrap_or(false)
    }

    unsafe extern "system" fn subclass(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM, _id: usize, _data: usize) -> LRESULT {
        // (a WM_PAINT with no invalid part is RapidR's own redraw:
        // rapidr_value::window_messages::heard)
        let update = msg == rapidr_value::window_messages::WM_PAINT && GetUpdateRect(hwnd, std::ptr::null_mut(), 0) != 0;
        if rapidr_value::window_messages::heard(msg, update) {
            // (never a panic here: it can't unwind through Windows)
            let _ = QUEUE.try_with(|q| {
                if let Ok(mut q) = q.try_borrow_mut() {
                    if q.len() < LIMIT {
                        q.push((hwnd as i64, msg, wparam as u64, lparam as i64));
                    }
                }
            });
        }
        DefSubclassProc(hwnd, msg, wparam, lparam)
    }

    fn hwnd_of(window: &Window) -> Option<HWND> {
        use winit::raw_window_handle::{HasWindowHandle, RawWindowHandle};
        match window.window_handle().ok()?.as_raw() {
            RawWindowHandle::Win32(w) => Some(w.hwnd.get() as HWND),
            _ => None,
        }
    }

    /// Form `f`'s new window, before it shows: subclassed if its WndProc is
    /// bound (so the messages of its showing are heard: WM_SHOWWINDOW,
    /// WM_ACTIVATE, WM_SIZE …).
    pub fn hook(f: &str, window: &Window) {
        if rapidr_value::window_messages::listens(f) {
            if let Some(hwnd) = hwnd_of(window) {
                // (the same subclass again only updates its data: harmless)
                unsafe { SetWindowSubclass(hwnd, Some(subclass), ID, 0) };
            }
        }
    }

    /// The windows of forms whose WndProc was bound after they were made:
    /// subclassed now (looked at only when another form starts listening).
    pub fn follow<'a>(wins: impl Iterator<Item = (&'a str, &'a Window)>) {
        let now = rapidr_value::window_messages::revision();
        if SEEN.with(|s| s.replace(now)) == now {
            return;
        }
        for (f, window) in wins {
            hook(f, window);
        }
    }
}
