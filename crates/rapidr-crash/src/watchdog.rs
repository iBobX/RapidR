//! RapidR Studio's hang watchdog.
//!
//! The window's thread says when it is busy and when it waits for the
//! system ([`busy`], [`idle`]: the UI hosts call them around their wait
//! for events — a nested loop's wait, a dialog's, a native menu's tracking
//! count as waiting). A thread of its own looks four times a second: busy
//! for longer than the limit (5 s), Studio isn't answering —
//!
//! 1. a hang report is written at once: what Studio was doing (its last
//!    command), the code of Studio's own it ran last and the event it was
//!    handling, the last AI and MCP calls, the last log lines, the
//!    process's threads by name (no backtrace: only the thread itself could
//!    take its own);
//! 2. the person is asked, in a small window of its own — a separate
//!    RapidR process (`rapidr __dialog`), so it draws even though Studio's
//!    window can't —: "RapidR Studio isn't responding. [Wait] [Save Copies
//!    and Quit]". Wait asks again 30 s later if it still doesn't answer.
//!    Save Copies and Quit ends Studio: the copies of unsaved work it keeps
//!    every few seconds (safety.inc) stay, offered at the next start;
//! 3. Studio answering again closes the question; the report says how long
//!    it took ("Answered again after 7 s") and is never offered as a crash.
//!
//! Under the GUI tests' hooks nothing is shown: the report is written and
//! said on standard error.

use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};

use crate::report::Report;

static BASE: OnceLock<Instant> = OnceLock::new();
/// Since when (ms after [`BASE`], 1 at least) the window's thread is busy;
/// 0: it waits for the system.
static BUSY_SINCE: AtomicU64 = AtomicU64::new(0);
static STARTED: AtomicBool = AtomicBool::new(false);
static TEXTS: Mutex<Option<Texts>> = Mutex::new(None);

fn now_ms() -> u64 {
    BASE.get_or_init(Instant::now).elapsed().as_millis() as u64 + 1
}

/// The window's thread has work (an event, a turn of the program).
#[inline]
pub fn busy() {
    if BUSY_SINCE.load(Ordering::Relaxed) == 0 {
        BUSY_SINCE.store(now_ms(), Ordering::Relaxed);
    }
}

/// The window's thread waits for the system (nothing to do, a dialog or a
/// menu the person is using).
#[inline]
pub fn idle() {
    BUSY_SINCE.store(0, Ordering::Relaxed);
}

/// The question's words, in Studio's language, and its look.
#[derive(Clone, Debug, Default)]
pub struct Texts {
    /// "RapidR Studio isn't responding"
    pub title: String,
    /// What waiting and quitting do.
    pub body: String,
    /// "&Wait"
    pub wait: String,
    /// "&Save Copies and Quit"
    pub quit: String,
    /// The window's caption.
    pub caption: String,
    /// Studio's look (`RAPIDR_THEME` for the question's window).
    pub theme: String,
    /// Studio's language (`RAPIDR_LANG`).
    pub lang: String,
}

/// Starts the watchdog (once; again: new words): Studio is said hung once
/// its window's thread has been busy for `after`.
pub fn start(after: Duration, texts: Texts) {
    *TEXTS.lock().unwrap_or_else(|e| e.into_inner()) = Some(texts);
    if STARTED.swap(true, Ordering::AcqRel) {
        return;
    }
    let _ = BASE.get_or_init(Instant::now);
    let _ = std::thread::Builder::new().name("rapidr-watchdog".into()).spawn(move || watch(after));
}

/// Whether the watchdog runs.
pub fn started() -> bool {
    STARTED.load(Ordering::Acquire) && !STOPPED.load(Ordering::Acquire)
}

static STOPPED: AtomicBool = AtomicBool::new(false);

/// The watchdog stops looking (a test's capture and end: the work from
/// here is the test's).
pub fn stop() {
    STOPPED.store(true, Ordering::Release);
}

fn watch(after: Duration) {
    let limit = after.as_millis() as u64;
    // (the episode reported: its busy-since stamp)
    let mut reported = 0u64;
    let mut report: Option<(Report, std::path::PathBuf)> = None;
    let mut next_ask = 0u64;
    loop {
        std::thread::sleep(Duration::from_millis(250));
        if STOPPED.load(Ordering::Acquire) {
            return;
        }
        let since = BUSY_SINCE.load(Ordering::Relaxed);
        let now = now_ms();
        // (an episode that ended: its report says how long it lasted)
        if reported != 0 && since != reported {
            if let Some((mut r, path)) = report.take() {
                r.outcome = format!("Answered again after {} s", (now.saturating_sub(reported) + 500) / 1000);
                let _ = std::fs::write(&path, r.render());
                if crate::testing() {
                    eprintln!("[rapidr] (test) Studio answered again: {}", r.outcome);
                }
            }
            reported = 0;
        }
        if since == 0 || now.saturating_sub(since) < limit {
            continue;
        }
        if reported != since {
            reported = since;
            let secs = now.saturating_sub(since) / 1000;
            let r = hang_report(secs);
            report = crate::write(&r).map(|p| (r, p));
            if let Some((_, p)) = &report {
                eprintln!("[rapidr] RapidR Studio isn't responding ({secs} s): a hang report was written: {}", crate::scrub::home(&p.to_string_lossy()));
            }
            next_ask = now;
        }
        if crate::testing() || now < next_ask {
            continue;
        }
        // Asked in a window of its own.
        let texts = TEXTS.lock().unwrap_or_else(|e| e.into_inner()).clone().unwrap_or_default();
        match ask(&texts, since) {
            Answer::Quit => {
                if let Some((mut r, path)) = report.take() {
                    r.outcome = format!("Ended by the person (Save Copies and Quit) after {} s", now_ms().saturating_sub(since) / 1000);
                    let _ = std::fs::write(&path, r.render());
                }
                quit_now();
            }
            Answer::Wait => next_ask = now_ms() + 30_000,
            // (Studio answered while it asked: the next turn says so)
            Answer::Answered => next_ask = u64::MAX,
        }
    }
}

/// The hang report: what Studio was doing.
fn hang_report(secs: u64) -> Report {
    let headline = format!("RapidR Studio stopped responding: its window didn't answer for {secs} seconds.");
    let mut r = crate::report_now("hang", &headline, &format!("The window's thread was busy for {secs} s without waiting for the system."));
    r.app = crate::App::Studio.name().into();
    r.thread = "main (the window's)".into();
    r.threads = thread_names();
    r
}

enum Answer {
    Wait,
    Quit,
    Answered,
}

/// Asks in a separate RapidR process (`rapidr __dialog`): the window draws
/// though Studio's can't. Studio answering again closes it.
fn ask(t: &Texts, since: u64) -> Answer {
    let Ok(exe) = std::env::current_exe() else { return Answer::Wait };
    let file = std::env::temp_dir().join(format!("rapidr-hang-{}.rr", std::process::id()));
    if std::fs::write(&file, dialog_source(t)).is_err() {
        return Answer::Wait;
    }
    let mut cmd = std::process::Command::new(exe);
    cmd.arg("__dialog").arg(&file).stdin(std::process::Stdio::null()).stdout(std::process::Stdio::piped()).stderr(std::process::Stdio::null());
    if !t.theme.is_empty() {
        cmd.env("RAPIDR_THEME", &t.theme);
    }
    if !t.lang.is_empty() {
        cmd.env("RAPIDR_LANG", &t.lang);
    }
    let Ok(mut child) = cmd.spawn() else {
        let _ = std::fs::remove_file(&file);
        return Answer::Wait;
    };
    let answer = loop {
        std::thread::sleep(Duration::from_millis(200));
        if BUSY_SINCE.load(Ordering::Relaxed) != since {
            // (our own child, by its handle)
            let _ = child.kill();
            let _ = child.wait();
            break Answer::Answered;
        }
        match child.try_wait() {
            Ok(Some(_)) => {
                let mut out = String::new();
                if let Some(mut so) = child.stdout.take() {
                    use std::io::Read;
                    let _ = so.read_to_string(&mut out);
                }
                break if out.trim() == "quit" { Answer::Quit } else { Answer::Wait };
            }
            Ok(None) => {}
            Err(_) => break Answer::Wait,
        }
    };
    let _ = std::fs::remove_file(&file);
    answer
}

/// A BASIC string literal (quotes as CHR$(34), lines as CHR$(10)).
fn basic(text: &str) -> String {
    let parts: Vec<String> = text.split('\n').map(|l| format!("\"{}\"", l.replace('"', "\" + CHR$(34) + \""))).collect();
    parts.join(" + CHR$(10) + ")
}

/// A dialog button's width for its caption (12 a side and more, 96 at
/// least: the design guide's).
fn button_width(caption: &str) -> u32 {
    let chars = caption.chars().filter(|c| *c != '&').count() as u32;
    (chars * 7 + 32).max(96)
}

/// The question's window, a small RapidR program.
pub fn dialog_source(t: &Texts) -> String {
    let or = |s: &str, d: &str| if s.is_empty() { basic(d) } else { basic(s) };
    format!(
        r#"' RapidR Studio isn't responding: the watchdog's question (rapidr-crash).
$APPTYPE GUI
DECLARE SUB HangPick(Sender AS RButton)
DECLARE SUB HangLayOut
CREATE HangForm AS RForm
  Caption = {caption}
  BorderStyle = 3
  CREATE HangTitle AS RLabel
    WordWrap = 1: AutoSize = 0
  END CREATE
  CREATE HangBody AS RLabel
    WordWrap = 1: AutoSize = 0
  END CREATE
  CREATE HangWait AS RButton
    Caption = {wait}: Tag = 1
    Default = 1: Cancel = 1
    OnClick = HangPick
  END CREATE
  CREATE HangQuit AS RButton
    Caption = {quit}: Tag = 2
    OnClick = HangPick
  END CREATE
END CREATE

SUB HangPick(Sender AS RButton)
  IF Sender.Tag = 2 THEN PRINT "quit" ELSE PRINT "wait"
  HangForm.Close
END SUB

SUB HangLayOut
  DIM w AS INTEGER, bw AS INTEGER, qw AS INTEGER
  w = 440
  HangTitle.Font.Size = 11: HangTitle.Font.Bold = 1
  HangTitle.Left = 20: HangTitle.Top = 20: HangTitle.Width = w: HangTitle.Height = 22
  HangBody.Left = 20: HangBody.Top = 50: HangBody.Width = w: HangBody.Height = 72
  HangTitle.Caption = {title}
  HangBody.Caption = {body}
  HangForm.AccessibleName = {title}
  bw = {wait_w}
  qw = {quit_w}
  HangWait.Height = 32: HangQuit.Height = 32
  HangWait.Top = 136: HangQuit.Top = 136
  HangWait.Width = bw: HangQuit.Width = qw
  HangWait.Left = 20 + w - bw
  HangQuit.Left = HangWait.Left - 8 - qw
  HangForm.ClientWidth = w + 40
  HangForm.ClientHeight = 188
END SUB

HangLayOut
HangForm.Center
HangForm.ShowModal
"#,
        caption = or(&t.caption, "RapidR Studio"),
        title = or(&t.title, "RapidR Studio isn't responding"),
        body = or(&t.body, "Its window hasn't answered for a few seconds. Wait gives it more time. Save Copies and Quit closes it: the copies of your unsaved work it keeps as you type (every 20 seconds) are offered when it starts again. A report was written on this computer."),
        wait = or(&t.wait, "&Wait"),
        quit = or(&t.quit, "&Save Copies and Quit"),
        wait_w = button_width(if t.wait.is_empty() { "Wait" } else { &t.wait }),
        quit_w = button_width(if t.quit.is_empty() { "Save Copies and Quit" } else { &t.quit }),
    )
}

/// Ends Studio now, from this thread (the window's is stuck): no clean-up
/// that could wait for it — the copies of unsaved work stay for the next
/// start.
fn quit_now() -> ! {
    #[cfg(unix)]
    // SAFETY: _exit ends the process at once; nothing runs after it.
    unsafe {
        libc::_exit(3)
    }
    #[cfg(not(unix))]
    std::process::exit(3)
}

/// The process's threads by name (Linux: /proc; macOS: the kernel's list;
/// elsewhere none).
pub fn thread_names() -> Vec<String> {
    #[cfg(target_os = "linux")]
    {
        let mut out: Vec<String> = std::fs::read_dir("/proc/self/task")
            .map(|rd| rd.flatten().filter_map(|e| std::fs::read_to_string(e.path().join("comm")).ok().map(|n| n.trim().to_string())).collect())
            .unwrap_or_default();
        out.sort();
        out
    }
    #[cfg(target_os = "macos")]
    {
        macos_threads()
    }
    #[cfg(not(any(target_os = "linux", target_os = "macos")))]
    {
        Vec::new()
    }
}

#[cfg(target_os = "macos")]
extern "C" {
    // (libSystem's; not in the libc crate)
    fn mach_port_deallocate(task: libc::mach_port_t, name: libc::mach_port_t) -> libc::kern_return_t;
}

#[cfg(target_os = "macos")]
#[allow(unsafe_code, deprecated)]
fn macos_threads() -> Vec<String> {
    let mut out = Vec::new();
    // SAFETY: the kernel's list of this task's threads, read and given
    // back; each name read into a buffer of its size.
    unsafe {
        let task = libc::mach_task_self();
        let mut list: libc::thread_act_array_t = std::ptr::null_mut();
        let mut count: libc::mach_msg_type_number_t = 0;
        if libc::task_threads(task, &mut list, &mut count) != 0 || list.is_null() {
            return out;
        }
        for i in 0..count as usize {
            let port = *list.add(i);
            let pt = libc::pthread_from_mach_thread_np(port);
            let mut name = [0 as libc::c_char; 64];
            let n = if pt != 0 && libc::pthread_getname_np(pt, name.as_mut_ptr(), name.len()) == 0 {
                std::ffi::CStr::from_ptr(name.as_ptr()).to_string_lossy().into_owned()
            } else {
                String::new()
            };
            out.push(if n.is_empty() { "(unnamed)".to_string() } else { n });
            mach_port_deallocate(task, port);
        }
        libc::vm_deallocate(task, list as libc::vm_address_t, (count as usize * std::mem::size_of::<libc::thread_act_t>()) as libc::vm_size_t);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_question_is_a_program_with_its_words() {
        let t = Texts { title: "RapidR Studio no responde".into(), wait: "&Esperar".into(), quit: "&Guardar copias y salir".into(), body: "Dijo \"no\"".into(), ..Texts::default() };
        let src = dialog_source(&t);
        assert!(src.contains("Caption = \"&Esperar\""));
        assert!(src.contains("HangBody.Caption = \"Dijo \" + CHR$(34) + \"no\" + CHR$(34) + \"\""));
        assert!(src.contains("Caption = \"RapidR Studio\""));
    }

    #[test]
    fn the_threads_have_names() {
        let names = thread_names();
        if cfg!(any(target_os = "linux", target_os = "macos")) {
            assert!(!names.is_empty());
        }
    }
}
