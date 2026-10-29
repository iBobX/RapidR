//! Native [`Host`] implementation for the RapidR bytecode VM.
//!
//! Routes all side effects to `rapidr-runtime-core` so bytecode programs
//! get the same builtin / GUI / file-IO behaviour as Rust-codegen
//! programs. Event registration goes through
//! [`rapidr_runtime_core::object::EventHandler::Indirect`]; the runtime
//! only *queues* the handlers it fires, and the VM runs them itself at safe
//! points (after the host operation that fired them, or while it serves
//! `ShowModal` and the program's windows via `Host::pump`). Nothing calls
//! back into a running VM, so there are no raw pointers and no aliasing.

#![forbid(unsafe_code)]

use std::cell::RefCell;
use std::collections::VecDeque;
use std::io::{self, BufRead, Write};

use rapidr_bytecode::Module;
use rapidr_runtime_core::object as obj;
use rapidr_runtime_core::prelude::*;
use rapidr_value::{v_dbl, v_int, v_null, v_str, Value};
use rapidr_value::events::QueuedEvent;
use rapidr_vm::{Host, Vm};

#[cfg(feature = "session")]
pub mod session;

/// Native host: routes the [`Host`] surface to `rapidr-runtime-core`.
#[derive(Default)]
pub struct NativeHost {
    /// Registered (component, event) → bytecode fn index. Kept here for
    /// debugging — the authoritative copy lives in
    /// `rapidr-runtime-core`'s `EVENT_HANDLERS`.
    pub events: Vec<(String, String, u32)>,
    /// Where PRINT goes instead of standard output (a session: the IDE).
    pub output: Option<Box<dyn FnMut(&str)>>,
    /// Where INPUT reads a line instead of standard input (a session);
    /// `None` from it: no more input.
    pub input_source: Option<Box<dyn FnMut() -> Option<String>>>,
    /// Told when a form is shown (`true`) or closed / hidden (a session).
    pub on_form: Option<Box<dyn FnMut(&str, bool)>>,
    /// Where a handler's run-time error goes instead of standard error, the
    /// program going on (a session: the IDE's Output).
    pub errors: Option<Box<dyn FnMut(&str)>>,
    /// Each builtin name the program called, as the program wrote it, and
    /// its key (`rapidr_bytecode::builtins::builtin_key`; `None`: not a
    /// builtin): worked out once, not at every call.
    keys: std::collections::HashMap<String, Option<std::rc::Rc<str>>>,
}

impl NativeHost {
    /// A builtin call ([`Host::call_builtin`] runs it inside
    /// `rapidr_value::host_call`).
    fn builtin(&mut self, name: &str, args: &[Value]) -> Result<Value, String> {
        let key = match self.keys.get(name) {
            Some(k) => k.clone(),
            None => {
                let key = rapidr_bytecode::builtins::builtin_key(name);
                let k = rapidr_bytecode::builtins::is_builtin(name).then(|| std::rc::Rc::<str>::from(key));
                self.keys.insert(name.to_string(), k.clone());
                k
            }
        };
        // Unknown names are an error, never a silent no-op. The compiler
        // rejects them up front; this guards bytecode from other sources.
        let Some(key) = key else {
            return Err(format!("Unknown builtin function '{name}'"));
        };
        let key: &str = &key;
        // LBOUND(arr [, dim]) / UBOUND(arr [, dim]) read the array's own
        // bounds (dimension 1 by default).
        if let Some(result) = rapidr_value::shared_builtin(key, args) {
            return result;
        }
        // A DLL's routine (rapidr_runtime_core::ffi): the real call on
        // Windows, the error elsewhere.
        if key == "__dll_call" {
            let s = |i: usize| args.get(i).map(Value::to_string_val).unwrap_or_default();
            return rapidr_runtime_core::ffi::dll_call(&s(0), &s(1), &s(2), args.get(3..).unwrap_or(&[]));
        }
        // POKE / PCOPY: a change on the screen page is printed.
        if key == "poke" || key == "pcopy" {
            let text = if key == "poke" {
                rapidr_value::memory::poke_args(args)?
            } else {
                let a = |i: usize| args.get(i).cloned().unwrap_or_else(v_null);
                rapidr_value::console::pcopy(&a(0), &a(1))?
            };
            if !text.is_empty() {
                self.print(&text)?;
            }
            return Ok(v_null());
        }
        if key == "lbound" || key == "ubound" {
            let arr = args.first().cloned().unwrap_or_else(v_null);
            let dim = args.get(1).map(|v| v.to_i64()).unwrap_or(1);
            return rapidr_value::array_bound(&arr, dim, key == "ubound")
                .map(v_int)
                .ok_or_else(|| format!("{}: argument is not an array, or it has no dimension {dim}", key.to_uppercase()));
        }
        Ok(call_builtin_key(key, args))
    }
}

impl Host for NativeHost {
    fn call_builtin(&mut self, name: &str, args: &[Value]) -> Result<Value, String> {
        // (a run-time error the runtime raises in a call of the program's:
        // the program's, at its line — a ShowModal's loop goes on after it
        // — never the process' end there: rapidr_value::host_call)
        rapidr_value::host_call(|| self.builtin(name, args)).and_then(|r| r)
    }

    fn create_comp(&mut self, kind: &str, id: &str) -> Result<Value, String> {
        rapidr_value::host_call(|| {
            rp_create_component(id, kind);
            v_str(id)
        })
    }

    fn set_prop(&mut self, id: &str, name: &str, value: Value) -> Result<(), String> {
        // (the program's set: RapidQ's events for it fire — an edit's OnChange …)
        rapidr_value::host_call(|| rp_prop_set(id, name, value))
    }

    fn get_prop(&mut self, id: &str, name: &str) -> Result<Value, String> {
        // Module-style constants (e.g. `math.pi`) — IMPORT is a no-op in
        // bcgen so these never become real components. Resolve a few
        // common ones here for demo parity.
        if let Some(v) = module_constant(id, name) {
            return Ok(v);
        }
        // (`WHILE DB.FetchRow`: a method of the object's type is called)
        rapidr_value::host_call(|| rp_comp_value(id, name))
    }

    fn component_type(&mut self, id: &str) -> Option<String> {
        Some(obj::rp_comp_type(id)).filter(|t| !t.is_empty())
    }

    fn component_children(&mut self, id: &str) -> Vec<String> {
        obj::rp_comp_children(id)
    }

    fn call_method(&mut self, id: &str, method: &str, args: &[Value]) -> Result<Value, String> {
        if let Some(on_form) = self.on_form.as_mut() {
            let shown = match method.to_ascii_lowercase().as_str() {
                "show" | "showmodal" => Some(true),
                "close" | "hide" => Some(false),
                _ => None,
            };
            if let Some(shown) = shown.filter(|_| obj::rp_comp_type(id).to_ascii_uppercase().contains("FORM")) {
                on_form(id, shown);
            }
        }
        // (an object's RapidQ exception — "Cannot create file x." —: the
        // program's run-time error, rapidr_value::host_call)
        rapidr_value::host_call(|| rp_comp_call(id, method, args)).map_err(|m| rapidr_value::exception(&m))
    }

    fn take_events(&mut self) -> Vec<QueuedEvent> {
        EVENTS.with(|q| q.borrow_mut().drain(..).collect())
    }

    fn defer_events(&mut self, events: Vec<QueuedEvent>) {
        // Before anything queued since.
        EVENTS.with(|q| {
            let mut q = q.borrow_mut();
            for e in events.into_iter().rev() {
                q.push_front(e);
            }
        });
    }

    fn wait_started(&mut self) -> bool {
        obj::rp_take_wait_started()
    }

    fn wait_catches_errors(&mut self) -> bool {
        obj::rp_wait_catches_errors()
    }

    fn handler_failed(&mut self, failure: &rapidr_vm::HandlerFailure) {
        match self.errors.as_mut() {
            Some(errors) => errors(&format!("{}\n", failure.text)),
            None => eprintln!("{}", failure.text),
        }
        // (RapidR's Application.OnError: the program hears it — not of an
        // error in its OnError handler itself, which would go round)
        let bound = self.events.iter().rev().find(|(id, event, _)| id.eq_ignore_ascii_case("application") && event.eq_ignore_ascii_case("onerror")).map(|(_, _, h)| *h);
        if bound.is_some_and(|h| failure.handler != Some(h)) {
            obj::rp_fire_event_args("application", "onerror", &failure.event_args());
        }
    }

    fn pump(&mut self) -> Option<Value> {
        obj::rp_pump_wait()
    }

    // (timers during native menu tracking: while a native menu holds the
    // window system, the runtime's tracking ticks fire the due timers and
    // the VM, lent to the wait, runs their handlers)
    fn pump_serving(&self) -> Option<fn(&mut dyn FnMut()) -> Option<Value>> {
        Some(obj::rp_pump_wait_serving)
    }

    fn serve_paused(&mut self) -> bool {
        obj::rp_serve_paused()
    }

    fn closed_while_paused(&mut self) -> bool {
        obj::rp_closed_while_paused()
    }

    fn register_event(&mut self, id: &str, event: &str, handler_fn_index: u32) -> Result<(), String> {
        obj::rp_bind_event_indirect(id, event, handler_fn_index);
        self.events.push((id.to_string(), event.to_string(), handler_fn_index));
        Ok(())
    }

    fn print(&mut self, s: &str) -> Result<(), String> {
        // Keep the shared console cursor current (CSRLIN, POS, LOCATE).
        rapidr_value::console::track(s);
        // (in RapidQ's code page when the program's file is RapidQ's)
        let shown = rapidr_value::console::shown(s);
        let s: &str = &shown;
        if let Some(output) = self.output.as_mut() {
            output(s);
            return Ok(());
        }
        let stdout = io::stdout();
        let mut h = stdout.lock();
        h.write_all(s.as_bytes()).map_err(|e| e.to_string())?;
        h.flush().map_err(|e| e.to_string())?;
        Ok(())
    }

    fn input(&mut self) -> Result<String, String> {
        if let Some(source) = self.input_source.as_mut() {
            return source().ok_or_else(|| "INPUT: no more input".to_string());
        }
        let mut line = String::new();
        // (INKEY$ may have left the terminal reading a key at a time)
        rapidr_runtime_core::terminal::line_mode();
        io::stdin().lock().read_line(&mut line).map_err(|e| e.to_string())?;
        if line.ends_with('\n') { line.pop(); }
        if line.ends_with('\r') { line.pop(); }
        Ok(line)
    }
}

/// Map a (case-insensitive) BASIC builtin name + args to a [`Value`]
/// using the `rp_*` runtime functions. Mirrors
/// `rapidr_codegen_rust::builtin_function_call`. Unknown names return
/// [`v_null`].
/// A builtin by its key (`rapidr_bytecode::builtins::builtin_key`: lower
/// case, no type suffix — `MID$` is `mid`), its arguments read in place.
fn call_builtin_key(lower: &str, args: &[Value]) -> Value {
    let null = v_null();
    let a0 = args.first().unwrap_or(&null);
    let a1 = args.get(1).unwrap_or(&null);
    let a2 = args.get(2).unwrap_or(&null);

    match lower {
        // --- Output / input ---
        "print" | "println" => {
            rp_print(args, lower == "println");
            v_null()
        }
        "input" | "input_func" => rp_input(&a0),

        // --- String ---
        "len" => rp_len(&a0),
        "mid" => rp_mid(&a0, &a1, &a2),
        "left" => rp_left(&a0, &a1),
        "right" => rp_right(&a0, &a1),
        "ucase" => rp_ucase(&a0),
        "lcase" => rp_lcase(&a0),
        "ltrim" => rp_ltrim(&a0),
        "rtrim" => rp_rtrim(&a0),
        "trim" => rp_trim(&a0),
        "instr" => {
            if args.len() >= 3 {
                rp_instr(&a0, &a1, &a2)
            } else {
                rp_instr(&v_int(1), &a0, &a1)
            }
        }
        "space" => rp_space(&a0),
        "string" => rp_string_func(&a0, &a1),
        "chr" => rp_chr(&a0),
        "asc" => rp_asc(&a0),
        "replace" => rp_replace(&a0, &a1, &a2),
        "str" => rp_str(&a0),
        "val" => rp_val(&a0),
        "insert" => rp_insert(&a0, &a1, &a2),
        "delete" => rp_delete(&a0, &a1, &a2),
        "reverse" => rp_reverse(&a0),
        "field" => rp_field(&a0, &a1, &a2),
        "tally" => rp_tally(&a0, &a1),
        "inv" => rapidr_value::rp_inv(&a0, &a1),
        // Console (RapidQ appendix C), as ANSI sequences
        "cls" => { rp_print(&[v_str(&rapidr_value::console::cls())], false); v_null() }
        "color" => { rp_print(&[v_str(&rapidr_value::console::color(&a0, &a1))], false); v_null() }
        "locate" => { rp_print(&[v_str(&rapidr_value::console::locate(&a0, &a1))], false); v_null() }
        "csrlin" => rapidr_value::console::csrlin(),
        "pos" => rapidr_value::console::pos(),
        "shl" => rapidr_value::rp_shl(&a0, &a1),
        "shr" => rapidr_value::rp_shr(&a0, &a1),
        // SUBI / FUNCTIONI arguments
        "__pack" => rapidr_value::variadic::pack(args),
        "__paramstr" => rapidr_value::variadic::param_str(&a0, &a1),
        "__paramval" => rapidr_value::variadic::param_val(&a0, &a1),
        "__paramstrcount" => rapidr_value::variadic::param_str_count(&a0),
        "__paramvalcount" => rapidr_value::variadic::param_val_count(&a0),
        "rinstr" => {
            if args.len() >= 3 {
                rp_rinstr_from(&a0, &a1, &a2)
            } else {
                rp_rinstr(&a0, &a1)
            }
        }
        "format" => rp_format(&a0, args.get(1..).unwrap_or(&[])),
        "strf" => rp_strf(&a0, &a1, &a2, &args.get(3).cloned().unwrap_or_else(v_null)),

        // --- Numeric / math ---
        "int" => rp_int(&a0),
        "abs" => rp_abs(&a0),
        "sgn" => rp_sgn(&a0),
        "sqr" => rp_sqr(&a0),
        "sin" => rp_sin(&a0),
        "cos" => rp_cos(&a0),
        "tan" => rp_tan(&a0),
        "atn" | "atan" => rp_atn(&a0),
        "tab" => rp_tab(&a0),
        "get" => rp_get_stdin(&a0),
        "setconsoletitle" => rp_set_console_title(&a0),
        "chdrive" => rp_chdrive(&a0),
        "acos" => rp_acos(&a0),
        "asin" => rp_asin(&a0),
        "log" => rp_log(&a0),
        "exp" => rp_exp(&a0),
        "ceil" => rp_ceil(&a0),
        "floor" => rp_floor(&a0),
        "round" => rp_round(&a0),
        "hex" => rp_hex(&a0),
        "oct" => rp_oct(&a0),
        "bin" => rp_bin(&a0),
        "rnd" => rp_rnd(&a0),
        "fix" => rp_fix(&a0),
        "frac" => rp_frac(&a0),
        "cbool" => rp_cbool(&a0),
        "cint" => rp_cint(&a0),
        "clng" => rp_clng(&a0),
        "cdbl" => rp_cdbl(&a0),
        "csng" => rp_csng(&a0),
        "iif" => rp_iif(&a0, &a1, &a2),
        "hextodec" => rp_hextodec(&a0),
        "convbase" => rp_convbase(&a0, &a1, &a2),
        "rgb" => rp_rgb(&a0, &a1, &a2),
        "randomize" => { rp_randomize(&a0); v_null() }
        "vartype" => rp_vartype(&a0),

        // --- Time / system ---
        "date" | "date_func" | "date$" => rp_date(),
        "time" | "time_func" | "time$" => rp_time(),
        "timer" => rp_timer(),
        "inkey" => rp_inkey(),
        "rapidr__waitkey" => rp_waitkey(),
        "sleep" => { rp_sleep(&a0); v_null() }
        "command" => if args.is_empty() { rp_command() } else { rp_command_arg(&a0) },
        "commandcount" => rp_commandcount(),
        "environ" => rp_environ(&a0),
        "doevents" => { rp_doevents(); v_null() }
        "end" => { if args.is_empty() { rp_end() } else { rp_end_code(&a0) }; v_null() }
        "showmessage" => { rp_showmessage(&a0); v_null() }
        "msgbox" => rp_msgbox(&a0),
        "messagebox" => rp_messagebox(&a0, &a1, &a2),
        "messagedlg" => rp_messagedlg(&a0, &a1, &a2, &args.get(3).cloned().unwrap_or_else(v_null), &args.get(4).cloned().unwrap_or_else(v_null)),
        "direxists" => rp_direxists(&a0),
        "fileexists" => rp_fileexists(&a0),
        "resource" => rp_resource(&a0),
        "resourcecount" => rp_resourcecount(),
        "playwav" => { rp_playwav(&a0, &a1); v_null() }
        "mousex" => rp_mousex(),
        "mousey" => rp_mousey(),
        "extractresource" => { rp_extractresource(&a0, &a1); v_null() }
        "shell" => rp_shell(&a0),
        "run" => rp_run(&a0),
        "shellwait" => rp_shellwait(&a0),
        "beep" => { rp_beep(); v_null() }
        "replacesubstr" => rp_replacesubstr(&a0, &a1, &a2),
        "sound" => { rp_sound(&a0, &a1); v_null() }
        "playsound" => rp_playsound(&a0),
        "isnumeric" => rp_isnumeric(&a0),

        // --- Array ---
        "lbound" => rp_lbound(std::slice::from_ref(a0)),
        "ubound" => rp_ubound(std::slice::from_ref(a0)),
        // (POKE / PCOPY / a DLL call are answered in `call_builtin`, where
        // an error can be reported; listed for the registry)
        "poke" | "pcopy" | "__dll_call" => v_null(),

        // --- File / directory ---
        "freefile" => rp_freefile(),
        "eof" => rp_eof(&a0),
        "lof" => rp_lof(&a0),
        "filelen" => rp_filelen(&a0),
        "line_input" => rp_line_input(&a0),
        "input_field" => rp_input_field(&a0),
        "dir" => rp_dir(&a0, &a1),
        "mkdir" => { rp_mkdir(&a0); v_null() }
        "rmdir" => { rp_rmdir(&a0); v_null() }
        "kill" => { rp_kill(&a0); v_null() }
        "unloadlibrary" => { rapidr_runtime_core::prelude::rp_unloadlibrary(&a0); v_null() }
        "rename" => { rp_rename(&a0, &a1); v_null() }
        "curdir" => rp_curdir(),
        "chdir" => { rp_chdir(&a0); v_null() }
        "open" => { rp_open(&a0, &a1, &a2); v_null() }
        "close" => { rp_close(&a0); v_null() }
        "seek" => { rp_seek(&a0, &a1); v_null() }
        "print_hash" | "print#" => { rp_print_hash(args.first().unwrap_or(&v_null()), &args.iter().skip(1).cloned().collect::<Vec<_>>()); v_null() }
        "write_hash" | "write#" => { rp_write_hash(args.first().unwrap_or(&v_null()), &args.iter().skip(1).cloned().collect::<Vec<_>>()); v_null() }

        // --- Math constants pulled from common patterns ---
        "math.pi" | "pi" => v_dbl(std::f64::consts::PI),
        "math.e" | "e" => v_dbl(std::f64::consts::E),

        // `DIM lbl(1 TO 3) AS QLABEL` and `lbl(i).OnClick = Handler`
        "__component_array" => {
            let (kind, name, bounds) = rapidr_value::objects::component_array_args(args);
            obj::rp_component_array(&kind, &name, &bounds)
        }
        // Components reached through objects (rapidr_ast::objects): by id.
        "__objget" => rp_comp_read(&a0.to_string_val(), &a1.to_string_val()),
        "__objset" => {
            rp_prop_set(&a0.to_string_val(), &a1.to_string_val(), args.get(2).cloned().unwrap_or_else(v_null));
            v_null()
        }
        "__objcall" => rp_comp_call(&a0.to_string_val(), &a1.to_string_val(), args.get(2..).unwrap_or(&[])),
        "__objcreate" => {
            rp_create_component(&a0.to_string_val(), &a1.to_string_val());
            v_null()
        }
        "__bind_event_this" => {
            // (component id, event, handler pointer = index + 1, instance)
            let ptr = args.get(2).map_or(0, |v| v.to_i64());
            if ptr > 0 {
                obj::rp_bind_event_indirect_this(&a0.to_string_val(), &a1.to_string_val(), (ptr - 1) as u32, args.get(3).cloned().unwrap_or_else(v_null));
            }
            v_null()
        }
        "__bind_event" => {
            // (object id, event, function pointer = index + 1)
            let ptr = args.get(2).map_or(0, |v| v.to_i64());
            if ptr > 0 {
                obj::rp_bind_event_indirect(&a0.to_string_val(), &a1.to_string_val(), (ptr - 1) as u32);
            }
            v_null()
        }

        // GUI plumbing emitted by bcgen
        "__gui_register_timer" => {
            if let Value::String(s) = &a0 { gui_register_timer(s); }
            v_null()
        }
        // `$THEME name`
        "__set_theme" => {
            set_theme(&a0.to_string_val());
            v_null()
        }

        _ => v_null(),
    }
}

// ---------- Indirect-dispatch event loop ----------

/// Resolve module-style constants like `math.pi`. Returns `None` for
/// unknown (module, member) pairs.
fn module_constant(module_id: &str, member: &str) -> Option<Value> {
    match (module_id.to_lowercase().as_str(), member.to_lowercase().as_str()) {
        ("math", "pi") => Some(v_dbl(std::f64::consts::PI)),
        ("math", "e") => Some(v_dbl(std::f64::consts::E)),
        ("math", "tau") => Some(v_dbl(std::f64::consts::TAU)),
        _ => None,
    }
}

thread_local! {
    /// Event handlers the runtime fired, waiting for the VM to run them
    /// ([`Host::take_event`]). The runtime never calls into the VM: it
    /// only queues here, so an event can't re-enter the VM mid-operation.
    static EVENTS: RefCell<VecDeque<QueuedEvent>> = const { RefCell::new(VecDeque::new()) };
}

/// Routes the runtime's events for bytecode handlers into [`EVENTS`].
fn install_event_queue() -> Option<obj::IndirectDispatcher> {
    obj::rp_set_cooperative_waits(true);
    obj::rp_set_event_dispatcher(Box::new(|fn_index, args| {
        // (with the continuation of an event fired with rp_fire_event_then)
        let event = QueuedEvent::fired(fn_index, args);
        EVENTS.with(|q| q.borrow_mut().push_back(event));
    }))
}

fn remove_event_queue(prev: Option<obj::IndirectDispatcher>) {
    match prev {
        Some(p) => {
            let _ = obj::rp_set_event_dispatcher(p);
        }
        None => {
            let _ = obj::rp_clear_event_dispatcher();
        }
    }
    obj::rp_set_cooperative_waits(false);
    EVENTS.with(|q| q.borrow_mut().clear());
}

/// The program being run is the file `path` with `args`, not this
/// executable (the RapidR Runtime running a program file:
/// `rapidr_runtime_core::program`). Call before [`run_bytes`].
pub fn set_program(path: &str, args: Vec<String>) {
    rapidr_runtime_core::program::set(path, args);
}

/// Decode an in-memory `.rrbc` byte slice and execute it on a fresh
/// [`NativeHost`].
///
/// Events go through a queue the VM drains itself (see [`EVENTS`]):
/// `Form.ShowModal` in `MAIN` becomes a wait the VM serves, running the
/// form's handlers between UI events. The program ends with `MAIN`, its
/// windows with it (RapidQ's).
///
/// Used by both the CLI's `run-bc` subcommand and the
/// `rapidrintr-runner` stub binary (Phase 8: bytecode → single exe).
pub fn run_bytes(bytes: &[u8]) -> Result<(), String> {
    let module = Module::from_bytes(bytes).map_err(|e| format!("decode error: {e}"))?;
    rapidr_runtime_core::value::resources::set_all(&module.resources);
    let mut host = NativeHost::default();
    let mut vm = Vm::new(&mut host);
    run_module(&module, &mut vm)
}

/// Runs `module`'s main program on `vm`; then the program's end (timers
/// stopped, files closed).
pub fn run_module(module: &Module, vm: &mut Vm<'_, NativeHost>) -> Result<(), String> {
    rapidr_runtime_core::terminal::start();
    let prev = install_event_queue();

    let main_result = vm.run(module).map_err(|e| format!("vm error: {e}"));
    // The main program's end is the program's, as in RapidQ (RC.EXE: a
    // program whose main code ends after `Form.Show` exits there — its
    // form goes, its timers never tick): windows left open close with it.
    // (A program keeps its windows by waiting: ShowModal, a DOEVENTS loop.)
    // Timers stop first, so no tick fires into a finished program.
    obj::rp_stop_all_timers();
    obj::rp_mark_shutting_down();
    remove_event_queue(prev);
    // Files the program never closed keep what was written.
    rapidr_runtime_core::value::basic_files::close_all();
    main_result
}
