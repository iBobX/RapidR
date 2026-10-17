//! RapidR Studio's components on the desktop (feature `studio`: the `rapidr`
//! CLI's runtime, never a program's own runner): RPROJECT,
//! RLANGUAGESERVICE and RPROGRAMSESSION are `rapidr_studio`'s (the same on
//! the web); this is the desktop's host — events through the runtime's
//! queue, the program under development in its own process
//! (`rapidr run --session`, its forms real windows, Stop a kill).

use std::path::PathBuf;

use rapidr_studio::{Host, Transport};

use crate::object::rp_fire_event_args;
use crate::value::Value;

#[derive(Clone, Copy)]
pub struct Desktop;

/// The `rapidr` executable that runs the program: `rapidr run` says which
/// (RAPIDR_RUNTIME), else this one.
fn runtime() -> Result<PathBuf, String> {
    std::env::var_os("RAPIDR_RUNTIME")
        .map(PathBuf::from)
        .filter(|p| p.is_file())
        .or_else(|| std::env::current_exe().ok())
        .ok_or_else(|| "can't find the rapidr executable to run the program".to_string())
}

impl Host for Desktop {
    fn fire(self, name: &str, event: &str, args: &[Value]) {
        rp_fire_event_args(name, event, args);
    }

    fn launch(self, program: &str, args: &[String], theme: &str) -> Result<Box<dyn Transport>, String> {
        if program.is_empty() {
            return Err("no program to run".into());
        }
        let exe = runtime()?;
        // (it runs in its own folder, as `rapidr run` from there would: its
        // data files found beside it)
        let full = std::fs::canonicalize(program).map_err(|e| format!("{program}: {e}"))?;
        let cwd = full.parent().filter(|p| !p.as_os_str().is_empty());
        // (the IDE's own test hooks aren't the program's; under a capture
        // test the program's windows are captured too, as `<prefix>-program`)
        let mut env: Vec<(String, Option<String>)> = std::env::vars()
            .map(|(k, _)| k)
            .filter(|k| k.starts_with("RAPIDR_TEST_") || k == "RAPIDR_CAPTURE" || k == "RAPIDR_CAPTURE_DELAY")
            .map(|k| (k, None))
            .collect();
        if let Ok(prefix) = std::env::var("RAPIDR_CAPTURE") {
            env.push(("RAPIDR_CAPTURE".into(), Some(format!("{prefix}-program"))));
            // (a test that drives the program too — the debugger's event
            // flows: clicks and keys into it, its handlers' breakpoints, its
            // dialogs answered —: RAPIDR_TEST_PROGRAM_<X> is the program's
            // RAPIDR_TEST_<X>, RAPIDR_TEST_PROGRAM_EVENTS its script)
            for (k, v) in std::env::vars() {
                if let Some(hook) = k.strip_prefix("RAPIDR_TEST_PROGRAM_").filter(|h| !h.is_empty()) {
                    env.push((format!("RAPIDR_TEST_{hook}"), Some(v)));
                }
            }
        }
        // (Studio's "Preview in classic": the program's default look)
        if !theme.is_empty() {
            env.push(("RAPIDR_THEME".into(), Some(theme.to_string())));
        }
        // (the front handed back to Studio when the program stops while the
        // user was using it: Studio's process)
        env.push(("RAPIDR_SESSION_IDE".into(), Some(std::process::id().to_string())));
        // (Studio's include folders: the program's $INCLUDEs found as in
        // Studio — Settings > Include folders)
        if let Some(path) = rapidr_studio::project::include_path_env() {
            env.push(("RAPIDR_INCLUDE_PATH".into(), Some(path)));
        }
        let mut transport = rapidr_session::process::ProcessTransport::spawn_with_env(&exe, &full.to_string_lossy(), args, cwd, &env).map_err(|e| format!("{}: {e}", exe.display()))?;
        // (what the program says wakes Studio's loop: a debugger step's
        // stop and its snapshot heard as they come, not every 20 ms)
        if let Some(w) = crate::ui::kernel::waker() {
            transport.set_waker(w);
        }
        // (Show Program Window, Continue: Studio hands the program the front)
        transport.set_activator(Box::new(crate::ui::kernel::activate_process));
        Ok(Box::new(transport))
    }

    fn list_files(self, folder: &str) -> Vec<String> {
        Desktop::files_in(folder)
    }

    fn rapidr(self) -> Option<PathBuf> {
        runtime().ok()
    }

    fn reveal(self, path: &str) -> Result<(), String> {
        let p = std::path::Path::new(path);
        if !p.exists() {
            return Err(format!("{path}: not there"));
        }
        let mut cmd = if cfg!(target_os = "macos") {
            let mut c = std::process::Command::new("open");
            c.arg("-R").arg(p);
            c
        } else if cfg!(windows) {
            let mut c = std::process::Command::new("explorer");
            c.arg(format!("/select,{}", path.replace('/', "\\")));
            c
        } else {
            // (the folder it's in: what every Linux file manager opens)
            let mut c = std::process::Command::new("xdg-open");
            c.arg(p.parent().filter(|d| !d.as_os_str().is_empty()).unwrap_or(std::path::Path::new(".")));
            c
        };
        cmd.spawn().map(|_| ()).map_err(|e| format!("can't show {path}: {e}"))
    }
}

impl Desktop {
    fn files_in(folder: &str) -> Vec<String> {
        std::fs::read_dir(folder)
            .map(|d| d.filter_map(Result::ok).filter(|e| e.path().is_file()).map(|e| e.file_name().to_string_lossy().into_owned()).collect())
            .unwrap_or_default()
    }
}

/// A property of one of Studio's components (`None`: not its own).
pub fn get(type_name: &str, name: &str, prop: &str) -> Option<Value> {
    rapidr_studio::get(type_name, name, prop)
}

/// Sets a property; whether it was the component's own.
pub fn set(type_name: &str, name: &str, prop: &str, v: &Value) -> bool {
    rapidr_studio::set(Desktop, type_name, name, prop, v)
}

/// A method (`None`: not the component's own).
pub fn call(type_name: &str, name: &str, method: &str, args: &[Value]) -> Option<Value> {
    rapidr_studio::call(Desktop, type_name, name, method, args)
}

/// The running sessions' news as their events; whether one runs (the
/// window loop then wakes often to hear it).
pub fn poll() -> bool {
    rapidr_studio::poll(Desktop)
}

/// Whether a program session is running.
pub fn running() -> bool {
    rapidr_studio::session::any_running() || rapidr_studio::build::any()
}

/// How often the window loop must look at Studio's children: a build's
/// or Run in Browser's output every 20 ms; a program session's wakes the
/// loop itself (its transport's reader, also when the program ends), looked
/// at once a second besides; none: nothing to look at.
pub fn poll_every() -> Option<std::time::Duration> {
    if rapidr_studio::build::any() || rapidr_studio::session::starting() {
        Some(std::time::Duration::from_millis(20))
    } else if rapidr_studio::session::any_running() {
        Some(std::time::Duration::from_secs(1))
    } else {
        None
    }
}
