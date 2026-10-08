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
        }
        // (Studio's "Preview in classic": the program's default look)
        if !theme.is_empty() {
            env.push(("RAPIDR_THEME".into(), Some(theme.to_string())));
        }
        rapidr_session::process::ProcessTransport::spawn_with_env(&exe, &full.to_string_lossy(), args, cwd, &env)
            .map(|t| Box::new(t) as Box<dyn Transport>)
            .map_err(|e| format!("{}: {e}", exe.display()))
    }

    fn list_files(self, folder: &str) -> Vec<String> {
        Desktop::files_in(folder)
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
    rapidr_studio::session::any_running()
}
