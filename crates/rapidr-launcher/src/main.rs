//! `rapidrw`: how the desktop starts RapidR programs.
//!
//! - **Windows** (`rapidrw.exe`, a windowed executable, next to the console
//!   `rapidr.exe` — as `pythonw.exe` beside `python.exe`): the file types'
//!   launcher. It asks `rapidr info` the program's `$APPTYPE` and runs
//!   `rapidr open` in a new console for a console program, with no console
//!   window for a windowed one. `rapidrw --ide [file]` starts the IDE.
//! - **macOS** (the executable of `RapidR.app` and `RapidR Runtime.app`):
//!   Finder hands an app the files it opens as an Apple event, not as
//!   arguments; this receives them and hands them on — a source file to the
//!   IDE (`rapidr ide`, in RapidR.app), anything else to `rapidr open` —
//!   replacing itself with the last one (`exec`), so the app in the Dock is
//!   the program.
//! - Elsewhere (Linux' `.desktop` files run `rapidr open %f` directly) it
//!   runs `rapidr open` with its arguments.
#![cfg_attr(windows, windows_subsystem = "windows")]

use std::path::PathBuf;
use std::process::ExitCode;

/// An executable beside this one.
fn beside(name: &str) -> PathBuf {
    let exe = std::env::current_exe().and_then(|e| e.canonicalize()).unwrap_or_default();
    exe.parent().map(|d| d.join(format!("{name}{}", std::env::consts::EXE_SUFFIX))).unwrap_or_else(|| PathBuf::from(name))
}

#[cfg(windows)]
fn main() -> ExitCode {
    use std::os::windows::process::CommandExt;
    use std::process::{Command, Stdio};
    const CREATE_NEW_CONSOLE: u32 = 0x0000_0010;
    const CREATE_NO_WINDOW: u32 = 0x0800_0000;

    let args: Vec<std::ffi::OsString> = std::env::args_os().skip(1).collect();
    let rapidr = beside("rapidr");
    // `rapidrw --ide [file]`: the IDE (the Start menu, a source file's Open)
    if args.first().is_some_and(|a| a == "--ide") {
        let status = Command::new(&rapidr).arg("ide").args(&args[1..]).creation_flags(CREATE_NO_WINDOW).stdin(Stdio::null()).status();
        return ExitCode::from(u8::from(!status.is_ok_and(|s| s.success())));
    }
    let Some(file) = args.first() else {
        let _ = Command::new(&rapidr).arg("about").creation_flags(CREATE_NO_WINDOW).status();
        return ExitCode::SUCCESS;
    };
    // (a program that doesn't compile: `rapidr open` shows why in a window)
    let console = Command::new(&rapidr)
        .arg("info")
        .arg(file)
        .creation_flags(CREATE_NO_WINDOW)
        .stdin(Stdio::null())
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).lines().any(|l| matches!(l.trim(), "apptype: console" | "apptype: cgi")))
        .unwrap_or(false);
    let mut open = Command::new(&rapidr);
    open.arg("open").args(&args);
    if console {
        open.creation_flags(CREATE_NEW_CONSOLE);
    } else {
        open.creation_flags(CREATE_NO_WINDOW).stdin(Stdio::null()).stdout(Stdio::null()).stderr(Stdio::null());
    }
    match open.status() {
        Ok(s) => ExitCode::from(s.code().unwrap_or(1).clamp(0, 255) as u8),
        Err(_) => ExitCode::from(1),
    }
}

#[cfg(target_os = "macos")]
mod mac {
    use std::cell::RefCell;
    use std::os::unix::process::CommandExt;
    use std::path::PathBuf;
    use std::process::{Command, Stdio};

    use objc2::rc::Retained;
    use objc2::runtime::ProtocolObject;
    use objc2::{define_class, msg_send, DefinedClass, MainThreadOnly};
    use objc2_app_kit::{NSApplication, NSApplicationDelegate};
    use objc2_foundation::{MainThreadMarker, NSArray, NSDate, NSNotification, NSObject, NSObjectProtocol, NSRunLoop, NSURL};

    #[derive(Default)]
    struct Ivars {
        files: RefCell<Vec<PathBuf>>,
    }

    define_class!(
        // SAFETY: NSObject has no subclassing requirements; no Drop.
        #[unsafe(super = NSObject)]
        #[thread_kind = MainThreadOnly]
        #[ivars = Ivars]
        struct Delegate;

        // SAFETY: no requirements.
        unsafe impl NSObjectProtocol for Delegate {}

        // SAFETY: the signatures are AppKit's.
        unsafe impl NSApplicationDelegate for Delegate {
            #[unsafe(method(application:openURLs:))]
            fn open_urls(&self, _app: &NSApplication, urls: &NSArray<NSURL>) {
                for url in urls.iter() {
                    if let Some(path) = url.path() {
                        self.ivars().files.borrow_mut().push(PathBuf::from(path.to_string()));
                    }
                }
            }

            #[unsafe(method(applicationDidFinishLaunching:))]
            fn did_finish_launching(&self, _notification: &NSNotification) {
                // (files are handed over before this when the app is started
                // to open them; a moment more for a late Apple event)
                if self.ivars().files.borrow().is_empty() {
                    NSRunLoop::currentRunLoop().runUntilDate(&NSDate::dateWithTimeIntervalSinceNow(0.3));
                }
                let files = self.ivars().files.borrow().clone();
                hand_on(files);
            }
        }
    );

    impl Delegate {
        fn new(mtm: MainThreadMarker) -> Retained<Self> {
            let this = Self::alloc(mtm).set_ivars(Ivars::default());
            // SAFETY: NSObject's init.
            unsafe { msg_send![super(this), init] }
        }
    }

    /// What opens `file` (none: the app itself was opened): in RapidR.app
    /// (its home has the IDE) a source file opens in the IDE; anything else
    /// runs (`rapidr open`).
    pub(crate) fn command(file: Option<&PathBuf>) -> Command {
        let rapidr = super::beside("rapidr");
        // (an install's IDE, or a checkout's: tools/studio_app.sh's dev app
        // says where in RAPIDR_HOME, through its Info.plist's LSEnvironment)
        let has_ide = rapidr.parent().is_some_and(|d| d.join("../lib/rapidr/ide/rapidr-ide.rrbc").is_file())
            || std::env::var_os("RAPIDR_HOME").is_some_and(|h| PathBuf::from(h).join("ide/studio.rr").is_file());
        let is_source = file.is_some_and(|f| f.extension().is_some_and(|e| e.eq_ignore_ascii_case("rr") || e.eq_ignore_ascii_case("bas")));
        let mut cmd = Command::new(&rapidr);
        match file {
            Some(f) if has_ide && is_source => cmd.arg("ide").arg(f),
            Some(f) => cmd.arg("open").arg(f),
            None if has_ide => cmd.arg("ide"),
            None => cmd.arg("about"),
        };
        cmd.stdin(Stdio::null());
        // (a program finds its files beside it; the IDE starts at home)
        match file.and_then(|f| f.parent()) {
            Some(dir) => {
                cmd.current_dir(dir);
            }
            None => {
                if let Some(home) = std::env::var_os("HOME") {
                    cmd.current_dir(home);
                }
            }
        }
        cmd
    }

    /// Each file to its program; the last one replaces this process.
    fn hand_on(files: Vec<PathBuf>) {
        let (last, rest) = match files.split_last() {
            Some((last, rest)) => (Some(last), rest),
            None => (None, &[][..]),
        };
        for f in rest {
            let _ = command(Some(f)).spawn();
        }
        let err = command(last).exec();
        eprintln!("rapidrw: {err}");
        std::process::exit(1);
    }

    pub fn main() {
        let mtm = MainThreadMarker::new().expect("the main thread");
        let app = NSApplication::sharedApplication(mtm);
        let delegate = Delegate::new(mtm);
        app.setDelegate(Some(ProtocolObject::from_ref(&*delegate)));
        app.run();
    }
}

#[cfg(target_os = "macos")]
fn main() -> ExitCode {
    mac::main();
    ExitCode::SUCCESS
}

#[cfg(not(any(windows, target_os = "macos")))]
fn main() -> ExitCode {
    use std::os::unix::process::CommandExt;
    let err = std::process::Command::new(beside("rapidr")).arg("open").args(std::env::args_os().skip(1)).exec();
    eprintln!("rapidrw: {err}");
    ExitCode::from(1)
}
