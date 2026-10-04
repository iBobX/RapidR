//! The system's Open / Save dialogs (rfd's *async* dialogs; plan §1.5 and
//! Stage 0's findings): **never block on a native dialog.** A dialog is a
//! [`HostCmd::FileDialog`](crate::HostCmd::FileDialog) the program queues;
//! the winit host makes it *inside* a pump (where NSApp is running, so
//! macOS shows a sheet on the parent window instead of falling back to a
//! blocking `runModal`), polls it once with the pump's waker, and keeps the
//! future. runtime-core steps (windows paint, timers tick) and asks
//! [`Host::file_dialog`](crate::Host::file_dialog) after each step until
//! the answer is there; rfd's completion wakes the pump
//! (`HostEvent::Wake`).
//!
//! The headless host shows nothing: its dialogs are cancelled (tests answer
//! through `RAPIDR_TEST_FILE_DIALOG` before a dialog is asked for).

use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll, Waker};

/// What an Open / Save dialog shows (runtime-core's `ui::file_dialog`
/// request, in the host's terms).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct FileRequest {
    pub save: bool,
    pub multi: bool,
    pub title: Option<String>,
    /// (description, patterns: `*.txt`, `*.*`) in the program's order.
    pub filters: Vec<(String, Vec<String>)>,
    /// The one shown first (FilterIndex, from 0).
    pub filter_index: usize,
    pub dir: Option<String>,
    pub file_name: Option<String>,
}

/// A pattern's extension for rfd: `*.txt` → `txt`; `*` / `*.*` → `*`
/// (any file); anything else (an exact name) has none.
fn extension(pattern: &str) -> Option<String> {
    let p = pattern.trim();
    if p == "*" || p == "*.*" {
        return Some("*".into());
    }
    let ext = p.strip_prefix("*.")?;
    (!ext.is_empty() && !ext.contains(['*', '?'])).then(|| ext.to_string())
}

/// The filters rfd gets, the program's FilterIndex one first (Windows and
/// the portal show the first). macOS has no filter menu in rfd and allows
/// every filter's extensions at once, so it gets only the chosen filter —
/// and none when that one is "any file".
pub fn rfd_filters(req: &FileRequest, macos: bool) -> Vec<(String, Vec<String>)> {
    let mut order: Vec<usize> = (0..req.filters.len()).collect();
    if req.filter_index < order.len() {
        order.remove(req.filter_index);
        order.insert(0, req.filter_index);
    }
    let mut out: Vec<(String, Vec<String>)> = order
        .into_iter()
        .map(|i| {
            let (name, pats) = &req.filters[i];
            (name.clone(), pats.iter().filter_map(|p| extension(p)).collect::<Vec<_>>())
        })
        .filter(|(_, exts)| !exts.is_empty())
        .collect();
    if macos {
        out.truncate(1);
        if out.first().is_some_and(|(_, exts)| exts.iter().any(|e| e == "*")) {
            out.clear();
        }
    }
    out
}

type Answer = Pin<Box<dyn Future<Output = Vec<String>>>>;

enum Slot {
    /// rfd's future, polled when the pump is woken.
    Pending(Answer),
    Done(Vec<String>),
}

/// The open dialogs, by the program's id.
#[derive(Default)]
pub struct Dialogs {
    slots: HashMap<u64, Slot>,
}

impl Dialogs {
    /// Dialog `id` shown (inside a pump), on `parent`'s window when there
    /// is one (a sheet on macOS), and polled once so its completion wakes
    /// the pump.
    pub fn open(&mut self, id: u64, req: &FileRequest, parent: Option<&winit::window::Window>, waker: &Waker) {
        let mut d = rfd::AsyncFileDialog::new();
        if let Some(t) = &req.title {
            d = d.set_title(t);
        }
        for (name, exts) in rfd_filters(req, cfg!(target_os = "macos")) {
            d = d.add_filter(name, &exts);
        }
        if let Some(dir) = &req.dir {
            d = d.set_directory(dir);
        }
        if let Some(f) = &req.file_name {
            d = d.set_file_name(f);
        }
        if let Some(w) = parent {
            d = d.set_parent(w);
        }
        let path = |h: rfd::FileHandle| h.path().to_string_lossy().into_owned();
        let fut: Answer = if req.save {
            let f = d.save_file();
            Box::pin(async move { f.await.map(path).into_iter().collect() })
        } else if req.multi {
            let f = d.pick_files();
            Box::pin(async move { f.await.map(|v| v.into_iter().map(path).collect()).unwrap_or_default() })
        } else {
            let f = d.pick_file();
            Box::pin(async move { f.await.map(path).into_iter().collect() })
        };
        self.slots.insert(id, Slot::Pending(fut));
        self.poll(id, waker);
    }

    /// Dialog `id` answered with `paths` (none: cancelled) — a host without
    /// dialogs.
    pub fn answer(&mut self, id: u64, paths: Vec<String>) {
        self.slots.insert(id, Slot::Done(paths));
    }

    fn poll(&mut self, id: u64, waker: &Waker) {
        if let Some(Slot::Pending(fut)) = self.slots.get_mut(&id) {
            if let Poll::Ready(paths) = fut.as_mut().poll(&mut Context::from_waker(waker)) {
                self.slots.insert(id, Slot::Done(paths));
            }
        }
    }

    /// Dialog `id`'s answer once it's closed (the paths picked; none:
    /// cancelled), else `None` (still open, or not made yet).
    pub fn take(&mut self, id: u64, waker: &Waker) -> Option<Vec<String>> {
        self.poll(id, waker);
        match self.slots.remove(&id)? {
            Slot::Done(paths) => Some(paths),
            pending => {
                self.slots.insert(id, pending);
                None
            }
        }
    }

    /// Whether a dialog is open now.
    pub fn any_open(&self) -> bool {
        self.slots.values().any(|s| matches!(s, Slot::Pending(_)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn req(filters: &[(&str, &[&str])], index: usize) -> FileRequest {
        FileRequest { filters: filters.iter().map(|(n, p)| (n.to_string(), p.iter().map(|s| s.to_string()).collect())).collect(), filter_index: index, ..Default::default() }
    }

    #[test]
    fn filters_for_rfd() {
        let r = req(&[("Text files", &["*.txt", "*.TXT"]), ("All Files", &["*.*"])], 1);
        let s = |v: &[(String, Vec<String>)]| v.iter().map(|(n, e)| format!("{n}:{}", e.join(","))).collect::<Vec<_>>().join("|");
        assert_eq!(s(&rfd_filters(&r, false)), "All Files:*|Text files:txt,TXT");
        // (macOS: only the chosen one, none for "any file")
        assert!(rfd_filters(&r, true).is_empty());
        let r = req(&[("Text files", &["*.txt"]), ("Pictures", &["*.bmp", "pic?.ico", "*.ico"])], 1);
        assert_eq!(s(&rfd_filters(&r, true)), "Pictures:bmp,ico");
        assert_eq!(s(&rfd_filters(&req(&[], 0), false)), "");
    }

    #[test]
    fn a_kernel_dialogs_window_goes_for_good() {
        use crate::{Desktop, HostCmd, WindowSpec};
        let d = rapidr_ui_kernel::dialogs::Dialog::message(1, "Title", "Text", &["OK"], None);
        let mut desk = Desktop::new(Box::new(rapidr_ui_kernel::MemClipboard::default()));
        desk.ensure_form(&d.store, &d.id, false, WindowSpec::default());
        desk.show(&d.id);
        assert_eq!(desk.stacking(), [d.id.clone()]);
        desk.forget(&d.id);
        assert!(desk.forms.is_empty());
        assert_eq!(desk.cmds.last(), Some(&HostCmd::Forget(d.id.clone())));
    }

    #[test]
    fn answers_are_taken_once() {
        let mut d = Dialogs::default();
        let w = Waker::noop();
        assert_eq!(d.take(1, w), None);
        d.answer(1, vec!["a".into()]);
        assert!(!d.any_open());
        assert_eq!(d.take(1, w), Some(vec!["a".to_string()]));
        assert_eq!(d.take(1, w), None);
        let fut: Answer = Box::pin(async { vec!["b".to_string()] });
        d.slots.insert(2, Slot::Pending(fut));
        assert_eq!(d.take(2, w), Some(vec!["b".to_string()]));
    }
}
