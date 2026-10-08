//! Browser [`Host`] implementation for the RapidR bytecode VM.
//!
//! Routes builtins / component ops to `rapidr-runtime-web`. The
//! program lives in a `RefCell` session; the page's events queue their bytecode
//! handlers, which run when the VM is idle or at its next safe point —
//! never by re-entering a running VM. The VM runs in time slices: a program
//! that never waits gives the page a turn every few milliseconds and then
//! goes on (see `continue_slice`).
//!
//! Designed to be wrapped in a `wasm-bindgen` shim by a thin
//! application crate (`rapidrintr.wasm`) that loads a `.rrbc` module
//! at runtime.

#![forbid(unsafe_code)]

#![allow(clippy::too_many_lines)]

use std::cell::{Cell, RefCell};
use std::collections::VecDeque;

use rapidr_bytecode::Module;
use rapidr_runtime_web::object_web as obj;
use rapidr_runtime_web::prelude::*;
use rapidr_value::{v_dbl, v_int, v_null, v_str, Value};
use rapidr_session::program::{Control, ProgramEnd};
use rapidr_session::protocol::{Command, Event as Message, EventBody, Request, PROTOCOL_VERSION};
use rapidr_vm::{Host, Vm, VmError};
use wasm_bindgen::prelude::*;

/// Browser host: routes the [`Host`] surface to `rapidr-runtime-web`.
#[derive(Default)]
pub struct WebHost {
    /// Set to true once any DOM/GUI component has been created.
    pub has_components: bool,
}

impl Host for WebHost {
    const YIELDS: bool = true;

    fn yield_now(&mut self) -> bool {
        rapidr_runtime_web::dialog_web::should_yield()
    }

    fn call_builtin(&mut self, name: &str, args: &[Value]) -> Result<Value, String> {
        // Unknown names are an error, never a silent no-op. The compiler
        // rejects them up front; this guards bytecode from other sources.
        if !rapidr_bytecode::builtins::is_builtin(name) {
            return Err(format!("Unknown builtin function '{name}'"));
        }
        // LBOUND(arr [, dim]) / UBOUND(arr [, dim]) read the array's own
        // bounds (dimension 1 by default).
        let key = rapidr_bytecode::builtins::builtin_key(name);
        if let Some(result) = rapidr_value::shared_builtin(&key, args) {
            return result;
        }
        if key == "__component_array" || key == "__objcreate" {
            self.has_components = true;
            HAS_COMPONENTS.with(|h| h.set(true));
        }
        if key == "lbound" || key == "ubound" {
            let arr = args.first().cloned().unwrap_or_else(v_null);
            let dim = args.get(1).map(|v| v.to_i64()).unwrap_or(1);
            return rapidr_value::array_bound(&arr, dim, key == "ubound")
                .map(v_int)
                .ok_or_else(|| format!("{}: argument is not an array, or it has no dimension {dim}", key.to_uppercase()));
        }
        let key = rapidr_bytecode::builtins::builtin_key(name);
        // PRINT # / WRITE # take any number of items after the file number.
        if key == "print_hash" || key == "write_hash" {
            let rest = args.get(1..).unwrap_or(&[]);
            let file = args.first().cloned().unwrap_or_else(v_null);
            if key == "print_hash" {
                rp_print_hash(&file, rest);
            } else {
                rp_write_hash(&file, rest);
            }
            return Ok(v_null());
        }
        Ok(call_builtin_web(name, args))
    }

    fn create_comp(&mut self, kind: &str, id: &str) -> Result<Value, String> {
        rp_create_component(id, kind);
        self.has_components = true;
        HAS_COMPONENTS.with(|h| h.set(true));
        Ok(v_str(id))
    }

    fn set_prop(&mut self, id: &str, name: &str, value: Value) -> Result<(), String> {
        rp_comp_set(id, name, value);
        Ok(())
    }

    fn get_prop(&mut self, id: &str, name: &str) -> Result<Value, String> {
        if let Some(v) = module_constant(id, name) {
            return Ok(v);
        }
        // (`WHILE DB.FetchRow`: a method of the object's type is called)
        Ok(rp_comp_value(id, name))
    }

    fn call_method(&mut self, id: &str, method: &str, args: &[Value]) -> Result<Value, String> {
        // (the IDE's session hears about the program's forms)
        if SINK.with(|s| s.borrow().is_some()) {
            let shown = match method.to_ascii_lowercase().as_str() {
                "show" | "showmodal" => Some(true),
                "close" | "hide" => Some(false),
                _ => None,
            };
            if let Some(shown) = shown.filter(|_| obj::rp_comp_type(id).to_ascii_uppercase().contains("FORM")) {
                let id = id.to_string();
                emit(Message::new(if shown { EventBody::FormShown { id, caption: None } } else { EventBody::FormClosed { id } }));
            }
        }
        let v = rp_comp_call(id, method, args);
        // (an object's RapidQ exception: a run-time error — rapidr_value::raise)
        match rapidr_value::take_raised() {
            Some(m) => Err(rapidr_value::exception(&m)),
            None => Ok(v),
        }
    }

    fn component_properties(&mut self, id: &str) -> Option<(String, Vec<(String, Value)>)> {
        obj::rp_comp_get_all_properties(id).map(|(kind, props)| (kind, props.into_iter().collect()))
    }

    fn register_event(&mut self, id: &str, event: &str, handler_fn_index: u32) -> Result<(), String> {
        obj::rp_bind_event_indirect(id, event, handler_fn_index);
        Ok(())
    }

    fn print(&mut self, s: &str) -> Result<(), String> {
        // Mirror codegen-web behaviour: `rp_print` writes to `console.log`.
        // Use it for the trailing-newline form so we get one log call per
        // line; non-newline writes go to console.log too.
        rp_print(&[v_str(s)], false);
        Ok(())
    }

    fn input(&mut self) -> Result<String, String> {
        // An in-page field; the VM waits for it (dialog_web). Where it can't
        // wait, the browser's prompt().
        Ok(rapidr_runtime_web::builtins_input_line().to_string_val())
    }

    fn suspend_requested(&mut self) -> bool {
        rapidr_runtime_web::dialog_web::take_suspend()
    }

    fn take_events(&mut self) -> Vec<Event> {
        take_queued_events()
    }

    fn defer_events(&mut self, events: Vec<Event>) {
        DEFERRED.with(|q| q.borrow_mut().extend(events));
    }
}

fn module_constant(module_id: &str, member: &str) -> Option<Value> {
    match (module_id.to_lowercase().as_str(), member.to_lowercase().as_str()) {
        ("math", "pi") => Some(v_dbl(std::f64::consts::PI)),
        ("math", "e") => Some(v_dbl(std::f64::consts::E)),
        ("math", "tau") => Some(v_dbl(std::f64::consts::TAU)),
        _ => None,
    }
}

fn call_builtin_web(name: &str, args: &[Value]) -> Value {
    let mut lower = name.to_lowercase();
    if matches!(lower.chars().last(), Some('$' | '%' | '#' | '&' | '!')) {
        lower.pop();
    }
    let a0 = args.first().cloned().unwrap_or_else(v_null);
    let a1 = args.get(1).cloned().unwrap_or_else(v_null);
    let a2 = args.get(2).cloned().unwrap_or_else(v_null);

    match lower.as_str() {
        // Output / input
        "print" | "println" => { rp_print(args, lower == "println"); v_null() }
        "input" | "input_func" => rp_input(&a0),

        // String
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
            if args.len() >= 3 { rp_instr(&a0, &a1, &a2) }
            else { rp_instr(&v_int(1), &a0, &a1) }
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
        "rinstr" => rp_rinstr(&a0, &a1),
        "format" => rp_format(&a0, args.get(1..).unwrap_or(&[])),
        "strf" => rp_strf(&a0, &a1, &a2, &args.get(3).cloned().unwrap_or_else(v_null)),

        // Numeric / math
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

        // Time / system
        "date" | "date_func" | "date$" => rp_date(),
        "time" | "time_func" | "time$" => rp_time(),
        "timer" => rp_timer(),
        // (none yet, and the time slice is over: the browser gets a turn,
        // so a key can come — a `DO: LOOP UNTIL INKEY$ <> ""` doesn't
        // freeze the page)
        "rapidr__waitkey" => rp_waitkey(),
        "inkey" => {
            let k = rp_inkey();
            if k.to_string_val().is_empty() && rapidr_runtime_web::dialog_web::slice_over() {
                rapidr_runtime_web::dialog_web::pause(0.0);
            }
            k
        }
        // (the program pauses, the browser goes on: dialog_web::sleep)
        "sleep" => {
            if !rapidr_runtime_web::dialog_web::sleep(a0.to_f64().max(0.0) * 1000.0) {
                rp_sleep(&a0);
            }
            v_null()
        }
        "command" => if args.is_empty() { rp_command() } else { rp_command_arg(&a0) },
        "commandcount" => rp_commandcount(),
        "environ" => rp_environ(&a0),
        // The events waiting for the program run (right after this:
        // Host::take_events). Once its time slice is over, the program
        // pauses too, and the browser goes on (painting, new events).
        "doevents" => {
            // (with windows, the desktop's: the timers due now fire before it
            // returns, a wait the VM serves)
            if rapidr_runtime_web::kernel_web::doevents() {
                return v_null();
            }
            if rapidr_runtime_web::dialog_web::slice_over() {
                if !rapidr_runtime_web::dialog_web::pause(0.0) {
                    rp_doevents();
                }
            } else {
                let waiting: Vec<Event> = DEFERRED.with(|q| q.borrow_mut().drain(..).collect());
                EVENTS.with(|q| waiting.into_iter().rev().for_each(|e| q.borrow_mut().push_front(e)));
            }
            v_null()
        }
        // END: the VM halts after this (bcgen); the program's forms,
        // timers and events end here.
        "end" => { rapidr_runtime_web::object_web::end_program(); v_null() }
        "showmessage" => { rp_showmessage(&a0); v_null() }
        "msgbox" => rp_msgbox(&a0),
        "messagebox" => rp_messagebox(&a0, &a1, &a2),
        "messagedlg" => rp_messagedlg(&a0, &a1, &a2, &args.get(3).cloned().unwrap_or_else(v_null)),
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

        // Array
        "lbound" => rp_lbound(&[a0]),
        "ubound" => rp_ubound(&[a0]),

        // File / dir (browser stubs)
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
        "rename" => { rp_rename(&a0, &a1); v_null() }
        "curdir" => rp_curdir(),
        "chdir" => { rp_chdir(&a0); v_null() }
        "open" => { rp_open(&a0, &a1, &a2); v_null() }
        "close" => { rp_close(&a0); v_null() }
        "seek" => { rp_seek(&a0, &a1); v_null() }

        // Module-style constants
        "math.pi" | "pi" => v_dbl(std::f64::consts::PI),
        "math.e" | "e" => v_dbl(std::f64::consts::E),

        // `DIM lbl(1 TO 3) AS QLABEL` and `lbl(i).OnClick = Handler`
        "__component_array" => {
            let (kind, name, bounds) = rapidr_value::objects::component_array_args(args);
            rapidr_runtime_web::object_web::rp_component_array(&kind, &name, &bounds)
        }
        // Components reached through objects (rapidr_ast::objects): by id.
        "__objget" => rp_comp_read(&a0.to_string_val(), &a1.to_string_val()),
        "__objset" => {
            rp_comp_set(&a0.to_string_val(), &a1.to_string_val(), args.get(2).cloned().unwrap_or_else(v_null));
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
            if let Value::String(s) = &a0 { rapidr_runtime_web::object_web::gui_register_timer(s); }
            v_null()
        }
        // `$THEME name`: the browser has its own look; Application.Theme
        // reads the name back (as native web builds' set_theme).
        "__set_theme" => {
            obj::set_theme(&a0.to_string_val());
            v_null()
        }

        _ => v_null(),
    }
}

// ---------- Sessions and events ----------
//
// The running program (module + VM) lives in `SESSION`, a `RefCell`: every
// entry from JavaScript (starting the program, a DOM event, a dialog's
// answer, the debugger) borrows it for the duration, so the VM can never be
// entered twice. The runtime never calls into the VM: it queues each event
// handler in `EVENTS`. The VM runs those itself right after the host
// operation that fired them (see `Host::take_events`); events that arrive
// while it's idle run from `run_idle_events`, which every entry point calls
// once it has released the session. No raw pointers, no aliasing.
//
// When the VM yields (its time slice is over: `VmError::Yielded`), the
// session keeps what it was doing (`Slice`) and a message to the page's own
// MessageChannel continues it (`continue_slice`) after the browser's turn.
// Until then nothing else runs the VM: DOM events wait in `DEFERRED` (they
// run when the program waits — DoEvents, a dialog, ShowModal, the end of
// main — as on the desktop), and a dialog's answer waits in dialog_web.

/// One program run in the page — under the IDE's session protocol when
/// `end` is there ([`session_open`]).
struct Session {
    module: Module,
    vm: Vm<'static, WebHost>,
    /// `__main` stopped for a dialog: when it finishes, show the forms.
    main_waiting: bool,
    /// Tells a continuation whether the session is still the page's.
    generation: u64,
    /// The VM yielded while doing this; it goes on with `continue_slice`.
    slice: Option<Slice>,
    /// The program's end of the IDE's session (rapidr-session).
    end: Option<ProgramEnd>,
    /// Stopped for the debugger (a breakpoint, a step, a pause, an error).
    stopped: bool,
    /// `__main` has run to its end.
    main_finished: bool,
    /// `exited` was sent.
    exited: bool,
}

/// What the VM was running when it yielded: what to do once it stops.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Slice {
    /// `__main`, from its start (`rapidr_run_bc`).
    Main,
    /// Code continued after a dialog (or ShowModal, SLEEP, DOEVENTS).
    Resumed,
    /// Event handlers run while the VM was idle (`run_idle_events`).
    Idle,
    /// The debugger let the program go on (`session_request`).
    Debug,
}

impl Session {
    fn new(module: Module, debug: bool) -> Self {
        // The host lives as long as the page's wasm instance; one small
        // struct per run.
        let host: &'static mut WebHost = Box::leak(Box::default());
        let mut vm = Vm::new(host);
        vm.debug_mode = debug;
        let generation = NEXT_GENERATION.with(|g| {
            let n = g.get() + 1;
            g.set(n);
            n
        });
        Session { module, vm, main_waiting: false, generation, slice: None, end: None, stopped: false, main_finished: false, exited: false }
    }
}

type Event = rapidr_value::events::QueuedEvent;

use rapidr_runtime_web::dialog_web as dialog;

thread_local! {
    static SESSION: RefCell<Option<Session>> = const { RefCell::new(None) };
    /// Event handlers the runtime fired, not yet taken by the VM.
    static EVENTS: RefCell<VecDeque<Event>> = const { RefCell::new(VecDeque::new()) };
    /// Events the VM took but couldn't run because a handler before them
    /// waits for a dialog: they run from `run_idle_events`, first.
    static DEFERRED: RefCell<VecDeque<Event>> = const { RefCell::new(VecDeque::new()) };
    static HAS_COMPONENTS: Cell<bool> = const { Cell::new(false) };
    /// The forms were shown already (see [`finalize_forms`]).
    static FINALIZED: Cell<bool> = const { Cell::new(false) };
    static NEXT_GENERATION: Cell<u64> = const { Cell::new(0) };
    /// The page's channel for continuing a yielded VM (receiving and
    /// sending port): a message is a task of its own, without
    /// `setTimeout`'s minimum delay.
    static WAKE: RefCell<Option<(web_sys::MessagePort, web_sys::MessagePort)>> = const { RefCell::new(None) };
    /// Where the session's events go (the preview frame's function, which
    /// posts them to the IDE): `session_open`'s.
    static SINK: RefCell<Option<js_sys::Function>> = const { RefCell::new(None) };
    /// Events waiting to be sent once the session is released.
    static OUTBOX: RefCell<Vec<Message>> = const { RefCell::new(Vec::new()) };
}

/// Queues an event for the IDE (sent by [`flush_events`]).
fn emit(event: Message) {
    if SINK.with(|s| s.borrow().is_some()) {
        OUTBOX.with(|o| o.borrow_mut().push(event));
    }
}

/// Sends the queued events (as JSON) — never while the session is
/// borrowed, so the page may answer at once.
fn flush_events() {
    let events: Vec<Message> = OUTBOX.with(|o| std::mem::take(&mut *o.borrow_mut()));
    if events.is_empty() {
        return;
    }
    let Some(sink) = SINK.with(|s| s.borrow().clone()) else { return };
    for event in events {
        let _ = sink.call1(&JsValue::NULL, &JsValue::from_str(&event.to_json()));
    }
}

/// Shows the program's forms, once (a program that waits in ShowModal shows
/// them then; finishing later must not reopen the forms the user closed).
fn finalize_forms() {
    if FINALIZED.with(|f| f.replace(true)) {
        return;
    }
    rapidr_runtime_web::kernel_web::finalize();
}

fn take_queued_events() -> Vec<Event> {
    EVENTS.with(|q| q.borrow_mut().drain(..).collect())
}

/// Replaces the page's program: the old session and its queued events go.
fn start_session(session: Session) {
    dialog::clear_modals();
    // (the interpreter serves the UI kernel host's waits itself: ShowModal,
    // the dialogs, INPUT$, DOEVENTS — rapidr_ui_app::waits)
    rapidr_runtime_web::kernel_web::set_interpreter(true);
    EVENTS.with(|q| q.borrow_mut().clear());
    DEFERRED.with(|q| q.borrow_mut().clear());
    HAS_COMPONENTS.with(|h| h.set(false));
    FINALIZED.with(|f| f.set(false));
    SESSION.with(|s| {
        if let Ok(mut slot) = s.try_borrow_mut() {
            *slot = Some(session);
        }
    });
    let _ = obj::rp_set_event_dispatcher(Box::new(|fn_index, args| {
        // (with the continuation of an event fired with rp_fire_event_then)
        let event = Event { then: rapidr_value::events::take_armed(), ..Event::new(fn_index, args.to_vec()) };
        // Between two time slices the program is busy: the event waits
        // until it waits.
        if dialog::is_yielded() {
            DEFERRED.with(|q| q.borrow_mut().push_back(event));
            return;
        }
        EVENTS.with(|q| q.borrow_mut().push_back(event));
        run_idle_events();
    }));
    install_resume_handler();
}

/// What happened in the VM, reported once the session is released (a
/// stop was sent as it happened: [`run_step`]).
fn report(result: Result<(), VmError>, what: &str) {
    match result {
        Ok(()) | Err(VmError::Suspended | VmError::Yielded | VmError::Paused) => {}
        Err(e) => report_error(&format!("[rapidr] {what}: {e}")),
    }
}

/// An error, on the console (which the page running a session passes on
/// as the program's error output, as it does what the program prints).
fn report_error(text: &str) {
    web_sys::console::error_1(&JsValue::from_str(text));
}

/// Under a session, the program's end once it has one: END ran, or the
/// main program finished with no form to wait for (or failed).
fn check_exit(error: bool) {
    let exit = SESSION.with(|s| {
        let Ok(mut guard) = s.try_borrow_mut() else { return None };
        let session = guard.as_mut()?;
        if session.end.is_none() || session.exited || session.stopped {
            return None;
        }
        // (an error of the main program ends it; an event handler's doesn't)
        let failed = error && !session.main_finished && !session.main_waiting;
        let over = failed
            || rapidr_runtime_web::object_web::program_ended()
            || (session.main_finished && !HAS_COMPONENTS.with(Cell::get) && session.vm.frames.is_empty() && session.slice.is_none());
        over.then(|| {
            session.exited = true;
            i32::from(failed)
        })
    });
    if let Some(code) = exit {
        emit(Message::new(EventBody::Exited { code }));
    }
}

/// Runs `step` on the session's VM as one entry (a time slice). If it
/// yields, the session keeps `kind` and continues later; main stopping for
/// a dialog is noted. Returns the result and whether the main program has
/// now finished after waiting (then the forms' turn: [`settle`]).
fn run_step(
    session: &mut Session,
    kind: Slice,
    step: impl FnOnce(&mut Vm<'static, WebHost>, &Module) -> Result<(), VmError>,
) -> (Result<(), VmError>, bool) {
    dialog::enter_vm();
    let result = step(&mut session.vm, &session.module);
    dialog::leave_vm();
    match &result {
        Err(VmError::Yielded) => {
            session.slice = Some(kind);
            dialog::set_yielded(true);
            schedule_continue(session.generation);
        }
        Err(VmError::Suspended) if kind == Slice::Main => session.main_waiting = true,
        // Stopped for the debugger: the IDE hears where, now.
        Err(VmError::Paused) => {
            session.stopped = true;
            if let Some(end) = session.end.as_mut() {
                emit(end.stopped_event(&session.vm, &session.module));
            }
        }
        _ => {}
    }
    let main_done = kind == Slice::Resumed && result.is_ok() && session.main_waiting && session.vm.frames.is_empty();
    if main_done {
        session.main_waiting = false;
    }
    // (the main program ran to its end: from its start, or after a wait or
    // a stop of the debugger)
    if result.is_ok() && session.vm.frames.is_empty() && (kind == Slice::Main || main_done || (kind == Slice::Debug && !session.main_waiting)) {
        session.main_finished = true;
    }
    (result, main_done)
}

/// Once the session is released, what follows a VM entry that didn't
/// yield (see [`Slice`]): main's forms, the debugger's status, errors to
/// the console; then a partial line printed, and the events that waited.
fn settle(kind: Slice, result: Result<(), VmError>, main_done: bool) {
    if matches!(result, Err(VmError::Yielded)) {
        return;
    }
    let error = matches!(&result, Err(e) if !matches!(e, VmError::Paused | VmError::Suspended | VmError::Yielded));
    // The main program ran to its end: the program's end, as in RapidQ
    // (RC.EXE: a program whose main code ends after `Form.Show` exits
    // there — its form goes, its timers never tick) and on the desktop.
    // Its forms are shown first, and what showing them fired (OnShow) runs.
    let mut main_ended = false;
    match kind {
        Slice::Main => match result {
            Ok(()) => {
                if HAS_COMPONENTS.with(Cell::get) {
                    finalize_forms();
                    main_ended = true;
                }
            }
            // Waiting for a dialog: the forms appear when `__main` finishes;
            // waiting in ShowModal, now (the form is what it waits for).
            Err(VmError::Suspended) => {
                if dialog::modal_waiting() && HAS_COMPONENTS.with(Cell::get) {
                    finalize_forms();
                }
            }
            // (stopped for the debugger: said already)
            Err(VmError::Paused) => {}
            // (as the IDE's preview reports an error `rapidr_run_bc` returns)
            Err(e) => report_error(&format!("[run error] vm error: {e}")),
        },
        Slice::Resumed => {
            if main_done && HAS_COMPONENTS.with(Cell::get) {
                finalize_forms();
                // (the main program went on after its ShowModal and finished)
                main_ended = result.is_ok();
            }
            report(result, "vm error");
        }
        Slice::Idle => report(result, "event handler failed"),
        Slice::Debug => {
            match &result {
                // (the main program ran to its end: its forms, as after Main)
                Ok(()) if HAS_COMPONENTS.with(Cell::get) => {
                    finalize_forms();
                    main_ended = SESSION.with(|s| s.try_borrow().ok().is_some_and(|g| g.as_ref().is_some_and(|session| session.main_finished)));
                }
                Err(VmError::Suspended) if dialog::modal_waiting() && HAS_COMPONENTS.with(Cell::get) => finalize_forms(),
                _ => {}
            }
            report(result, "vm error");
        }
    }
    check_exit(error);
    schedule_output_flush();
    run_idle_events();
    // (unless what showing the forms fired waits: a ShowModal, a dialog)
    if main_ended && !dialog::modal_waiting() && !dialog::is_yielded() {
        rapidr_runtime_web::object_web::end_program();
        check_exit(false);
    }
    flush_events();
}

/// Continues the VM that yielded, in a task of its own (the browser has had
/// its turn), if the session is still the one that yielded.
fn schedule_continue(generation: u64) {
    let posted = WAKE.with(|wake| {
        let mut wake = wake.borrow_mut();
        if wake.is_none() {
            let channel = web_sys::MessageChannel::new().ok()?;
            let on_message = Closure::<dyn FnMut(web_sys::MessageEvent)>::new(|e: web_sys::MessageEvent| {
                if let Some(generation) = e.data().as_f64() {
                    continue_slice(generation as u64);
                }
            });
            channel.port1().set_onmessage(Some(on_message.as_ref().unchecked_ref()));
            on_message.forget();
            *wake = Some((channel.port1(), channel.port2()));
        }
        wake.as_ref()?.1.post_message(&JsValue::from_f64(generation as f64)).ok()
    });
    if posted.is_none() {
        let later = Closure::once_into_js(move || continue_slice(generation));
        if let Some(window) = web_sys::window() {
            let _ = window.set_timeout_with_callback_and_timeout_and_arguments_0(later.unchecked_ref(), 0);
        }
    }
}

/// The next time slice of the VM that yielded. A program replaced meanwhile
/// (a new run, the debugger gone) isn't continued.
fn continue_slice(generation: u64) {
    let outcome = SESSION.with(|s| {
        let Ok(mut guard) = s.try_borrow_mut() else { return Err(()) };
        let Some(session) = guard.as_mut().filter(|s| s.generation == generation) else { return Ok(None) };
        let Some(kind) = session.slice.take() else { return Ok(None) };
        dialog::set_yielded(false);
        let (result, main_done) = run_step(session, kind, |vm, module| vm.resume(module));
        Ok(Some((kind, result, main_done)))
    });
    match outcome {
        // (the VM is busy — it never is between tasks — so: a bit later)
        Err(()) => schedule_continue(generation),
        Ok(Some((kind, result, main_done))) => settle(kind, result, main_done),
        Ok(None) => {}
    }
}

/// Runs the queued event handlers while the VM is idle, each to
/// completion before the next. While the VM runs (the session is
/// borrowed) this does nothing: the VM runs them at its next safe point.
fn run_idle_events() {
    run_idle_events_inner();
    // (what the handlers did: a stop, an error, the program's end)
    check_exit(false);
    flush_events();
}

fn run_idle_events_inner() {
    loop {
        // The answers that came during a yield: it goes on.
        while dialog::resume_pending() {}
        // (busy between two time slices: later)
        if dialog::is_yielded() {
            return;
        }
        let outcome = SESSION.with(|s| {
            let Ok(mut guard) = s.try_borrow_mut() else { return None };
            let session = guard.as_mut()?;
            let mut batch: Vec<Event> = DEFERRED.with(|q| q.borrow_mut().drain(..).collect());
            batch.extend(take_queued_events());
            if batch.is_empty() {
                return None;
            }
            let mut batch = batch.into_iter();
            let (result, _) = run_step(session, Slice::Idle, |vm, module| {
                batch.by_ref().try_for_each(|event| vm.invoke_event(module, event).map(drop))
            });
            // A handler waits for a dialog, yielded (or failed): the rest
            // run later.
            DEFERRED.with(|q| q.borrow_mut().extend(batch));
            Some(result)
        });
        match outcome {
            None | Some(Err(VmError::Yielded)) => return,
            Some(Err(e @ (VmError::Suspended | VmError::Paused))) => {
                report(Err(e), "event handler");
                return;
            }
            Some(result) => report(result, "event handler failed"),
        }
    }
}

/// Continues the program after an in-page dialog (dialog_web): the answer
/// becomes the result of the MESSAGEBOX/INPUT that opened it.
fn install_resume_handler() {
    dialog::set_resume_handler(std::rc::Rc::new(|value, echo| {
        let outcome = SESSION.with(|s| {
            let Ok(mut guard) = s.try_borrow_mut() else { return None };
            let session = guard.as_mut()?;
            if let Some(line) = echo {
                // What was typed, as a terminal shows it.
                let _ = session.vm.host_mut().print(&format!("{line}\n"));
                session.vm.print_col = 0;
            }
            Some(run_step(session, Slice::Resumed, |vm, module| vm.resume_with(module, value)))
        });
        let Some((result, main_done)) = outcome else { return };
        settle(Slice::Resumed, result, main_done);
    }));
}

// ---------- wasm-bindgen entry point ----------

/// Decode and execute a `.rrbc` module from a byte slice.
///
/// Runs `__main` (it may stop for a dialog; the forms appear when it
/// finishes), shows the forms it created, and leaves the program in the
/// page's session: DOM events then run its handlers.
#[wasm_bindgen]
pub fn rapidr_run_bc(bytes: &[u8]) -> Result<(), JsValue> {
    let module = Module::from_bytes(bytes)
        .map_err(|e| JsValue::from_str(&format!("rrbc decode error: {e}")))?;
    rapidr_runtime_web::value::resources::set_all(&module.resources);
    rapidr_runtime_web::object_web::install_object_hooks();
    // (RapidR Studio's RPROGRAMSESSION compiles the program it runs here)
    rapidr_runtime_web::studio_web::set_compiler(compile_for_studio);
    // Installed before `__main` runs, so events fired during setup (an
    // RSqlite OnConnect, a synchronous RHTTP OnLoad, …) reach their handlers.
    start_session(Session::new(module, false));
    // (a plain run: no IDE session listens)
    SINK.with(|s| *s.borrow_mut() = None);

    let result = SESSION.with(|s| {
        let mut guard = s.try_borrow_mut().map_err(|_| JsValue::from_str("the VM is busy"))?;
        let session = guard.as_mut().ok_or_else(|| JsValue::from_str("no program"))?;
        Ok::<_, JsValue>(run_step(session, Slice::Main, |vm, module| vm.run(module)).0)
    })?;
    if let Err(e) = &result {
        if !matches!(e, VmError::Suspended | VmError::Yielded | VmError::Paused) {
            return Err(JsValue::from_str(&format!("vm error: {e}")));
        }
    }
    // (yielded: `__main` goes on in a moment, and this follows when it stops)
    settle(Slice::Main, result, false);
    Ok(())
}

// ---------- In-browser compile pipeline ----------
//
// The IDE bundles a single `rapidrintr.wasm` that handles BOTH compiling
// `.rr` source to `.rrbc` AND running the resulting bytecode. Exposing
// `compile()` here (instead of in a separate cdylib) means the IDE only
// needs one `init()` call and one wasm download.

/// A component's property as text (what the program reads), for tests and
/// tools that compare the browser with the desktop.
#[wasm_bindgen]
pub fn rapidr_get_prop(name: &str, prop: &str) -> String {
    // `__shown`: whether the component has an element the user can see.
    if prop == "__shown" {
        return i64::from(rapidr_runtime_web::kernel_web::element_shown(name)).to_string();
    }
    rapidr_runtime_web::object_web::rp_comp_get(name, prop).to_string_val()
}

/// Whether the program's main code has run to its end (or END ran): what a
/// desktop console program's exit is, for tests and tools that compare the
/// browser with the desktop. (Waiting in a dialog or a ShowModal isn't, nor
/// running between two time slices.)
#[wasm_bindgen]
pub fn rapidr_main_done() -> bool {
    rapidr_runtime_web::object_web::program_ended()
        || SESSION.with(|s| {
            s.try_borrow().ok().is_some_and(|g| {
                g.as_ref().is_some_and(|session| !session.main_waiting && session.slice != Some(Slice::Main))
            })
        })
}

/// Which GUI host draws the program's forms: the UI kernel (the page's
/// only one since the DOM host went; tests read it).
#[wasm_bindgen]
pub fn rapidr_host() -> String {
    "kernel".into()
}

/// For GUI tests on the kernel host (as the desktop's test hooks read the
/// process's environment): `RAPIDR_CAPTURE`, `RAPIDR_TEST_EVENTS`,
/// `RAPIDR_TEST_DUMP`, `RAPIDR_TEST_RESIZE`, `RAPIDR_TEST_SPLIT` and the
/// dialogs' answers, as an object of strings — set before the program runs.
#[wasm_bindgen]
pub fn rapidr_set_test_env(vars: JsValue) {
    rapidr_runtime_web::kernel_web::set_test_env(&vars);
}

/// A GUI test's results on the kernel host once its script ended (JSON:
/// `dump` — `RAPIDR_TEST_DUMP`'s lines —, `a11y` — each shown window's
/// accessibility tree, bottom to top — and `captures` — each window as the
/// desktop's `RAPIDR_CAPTURE` BMP, base64), `undefined` before.
#[wasm_bindgen]
pub fn rapidr_test_results() -> Option<String> {
    rapidr_runtime_web::kernel_web::test_results()
}

/// Compile a single RapidR source string to `.rrbc` bytecode bytes.
///
/// Mirrors `rapidr-compiler-wasm::compile` so any tool depending on the
/// older crate can switch over without behaviour change. On error,
/// returns the human-readable message as a JS exception.
///
/// `assets` (optional) maps the project's asset names (`name` and
/// `assets/name`) to data URLs: the files a `$RESOURCE` line names are
/// built into the program from there, as the desktop compiler reads them
/// from disk.
///
/// The program's lines are named `project_name` in its source map (what a
/// run-time error and the debugger's breakpoints and stops call it).
#[wasm_bindgen]
pub fn compile(source: &str, project_name: &str, assets: JsValue) -> Result<Vec<u8>, JsValue> {
    compile_inner(project_name, source, Vec::new(), &assets).map_err(|e| JsValue::from_str(&e))
}

/// Compiles a project's main file `main` from `files` (an object: each
/// file's name → its text) — what `$INCLUDE` finds — and `assets` as
/// [`compile`]. The source map names each line's file, as the desktop's
/// does: a breakpoint in an included file stops there.
#[wasm_bindgen]
pub fn compile_files(main: &str, files: JsValue, assets: JsValue) -> Result<Vec<u8>, JsValue> {
    let mut all = Vec::new();
    if let Some(object) = files.dyn_ref::<js_sys::Object>() {
        for entry in js_sys::Object::entries(object).iter() {
            let pair = js_sys::Array::from(&entry);
            if let (Some(name), Some(text)) = (pair.get(0).as_string(), pair.get(1).as_string()) {
                all.push((name, text));
            }
        }
    }
    let source = all
        .iter()
        .find(|(name, _)| name == main)
        .map(|(_, text)| text.clone())
        .ok_or_else(|| JsValue::from_str(&format!("{main}: not among the project's files")))?;
    compile_inner(main, &source, all, &assets).map_err(|e| JsValue::from_str(&e))
}

/// A `$RESOURCE` file's bytes from the project's assets: the name as
/// written, or its last part (`resource_files\two.bin` → `two.bin`), or under
/// `assets/`.
fn resource_bytes(assets: &JsValue, file: &str) -> Option<Vec<u8>> {
    if assets.is_undefined() || assets.is_null() {
        return None;
    }
    let written = file.replace('\\', "/");
    let base = written.rsplit('/').next().unwrap_or(&written).to_string();
    for key in [written.clone(), format!("assets/{written}"), base.clone(), format!("assets/{base}")] {
        if let Some(url) = js_sys::Reflect::get(assets, &JsValue::from_str(&key)).ok().and_then(|v| v.as_string()) {
            return rapidr_runtime_web::database_web::decode_base64(&url);
        }
    }
    // Names in any case, as RapidQ on Windows finds them (`BACK1.BMP` for
    // back1.bmp).
    let names = js_sys::Object::keys(assets.dyn_ref::<js_sys::Object>()?);
    let found = names.iter().filter_map(|n| n.as_string()).find(|n| {
        let n = n.strip_prefix("assets/").unwrap_or(n);
        n.eq_ignore_ascii_case(&written) || n.rsplit('/').next().is_some_and(|b| b.eq_ignore_ascii_case(&base))
    })?;
    let url = js_sys::Reflect::get(assets, &JsValue::from_str(&found)).ok()?.as_string()?;
    rapidr_runtime_web::database_web::decode_base64(&url)
}

/// (RapidR Studio's) A program from its files, as RPROGRAMSESSION runs it.
fn compile_for_studio(main: &str, files: Vec<(String, String)>) -> Result<Vec<u8>, String> {
    let source = files.iter().find(|(n, _)| n == main).map(|(_, t)| t.clone()).ok_or_else(|| format!("{main}: not among the program's files"))?;
    compile_inner(main, &source, files, &JsValue::UNDEFINED)
}

/// (RapidR Studio's page) A file into the page's store before (or while)
/// the program runs: the files the page keeps in the browser's private
/// file system, restored; `handle`: a FileSystemFileHandle its writes go
/// back to (else undefined).
#[wasm_bindgen]
pub fn rapidr_store_file(path: &str, bytes: &[u8], handle: JsValue) {
    rapidr_runtime_web::file_picker_web::keep_file(path, bytes.to_vec(), Some(handle));
}

/// (RapidR Studio's page) The program under development's frame sent an
/// event (session protocol JSON): the session's events fire.
#[wasm_bindgen]
pub fn studio_session_incoming(json: &str) {
    rapidr_runtime_web::rapidr_studio::channel::incoming(json);
    rapidr_runtime_web::studio_web::poll();
    run_idle_events();
}

/// (RapidR Studio's page) The program's frame ended (closed, or it failed
/// to start).
#[wasm_bindgen]
pub fn studio_session_ended(code: i32) {
    rapidr_runtime_web::rapidr_studio::channel::ended(code);
    rapidr_runtime_web::studio_web::poll();
    run_idle_events();
}

fn compile_inner(main: &str, source: &str, files: Vec<(String, String)>, assets: &JsValue) -> Result<Vec<u8>, String> {
    let options = rapidr_preprocessor::PreprocessOptions { virtual_files: files, ..Default::default() };
    let main_path = std::path::PathBuf::from(main);
    let pre = rapidr_preprocessor::preprocess_source(source, ".", Some(main_path.clone()), options)
        .map_err(|e| format!("preprocess error: {e}"))?;

    let tokens = rapidr_lexer::Lexer::new(&pre.source, None)
        .tokenize()
        .map_err(|e| format!("lex error: {e}"))?;

    let program = rapidr_parser::parse_tokens(&tokens)
        .map_err(|e| e.to_string())?;

    // (as the desktop compiler: lines from $INCLUDEd files are a library's)
    let library_lines: Vec<bool> = pre.line_map.iter().map(|(file, _)| file.as_ref().is_some_and(|f| *f != main_path)).collect();
    let mut compiled = rapidr_bcgen::compile_program_with_libraries(&program, Some(&pre.source), &library_lines)
        .map_err(|e| format!("bcgen error: {e}"))?;
    // Run-time errors and the debugger name each line's file and line.
    let origins = pre.line_map.iter().map(|(file, line)| (file.as_deref().and_then(|f| f.to_str()), *line as u32));
    compiled.module.source_map = rapidr_bytecode::SourceMap::from_origins(main, origins);
    compiled.module.apply_app_type_directive(pre.app_type.as_deref());

    // `$RESOURCE` files are built into the module.
    for r in &pre.resources {
        let bytes = match resource_bytes(assets, &r.file) {
            Some(b) => b,
            // (`$OPTION ICON` without its icon: the default one)
            None if r.optional => Vec::new(),
            None => return Err(format!("$RESOURCE {}: file not found in the project's assets: '{}' (add it under Assets)", r.name, r.file)),
        };
        compiled.module.resources.push((r.name.clone(), bytes));
    }

    Ok(compiled.module.to_bytes())
}

// ---------- The IDE's session (rapidr-session's protocol) ----------
//
// The preview frame's end of RapidR Studio's program session: the page
// opens the program with `session_open` (its events go to `sink`, as JSON),
// passes every request from the IDE to `session_request`. The program waits
// for `start` (after the breakpoints). A stop leaves the VM where it is
// (`VmError::Paused`): the page stays responsive, and `continue` / the
// steps run it on as a slice of its own (`Slice::Debug`).

/// Opens `bytes` (a `.rrbc`; `program`: its name for `ready`) as the page's
/// program under the IDE's session; events go to `sink` (a function taking
/// the event's JSON).
#[wasm_bindgen]
pub fn session_open(bytes: &[u8], program: &str, sink: js_sys::Function) -> Result<(), JsValue> {
    let module = Module::from_bytes(bytes).map_err(|e| JsValue::from_str(&format!("rrbc decode error: {e}")))?;
    rapidr_runtime_web::value::resources::set_all(&module.resources);
    rapidr_runtime_web::object_web::install_object_hooks();
    let mut session = Session::new(module, false);
    session.end = Some(ProgramEnd::new());
    start_session(session);
    SINK.with(|s| *s.borrow_mut() = Some(sink));
    OUTBOX.with(|o| o.borrow_mut().clear());
    emit(Message::new(EventBody::Ready { protocol: PROTOCOL_VERSION, runtime: format!("RapidR {}", env!("CARGO_PKG_VERSION")), program: program.to_string() }));
    flush_events();
    Ok(())
}

/// One request from the IDE (JSON, rapidr-session's protocol).
#[wasm_bindgen]
pub fn session_request(json: &str) {
    match Request::from_json(json) {
        Ok(request) => serve_request(request),
        Err(e) => emit(Message::error(0, e)),
    }
    flush_events();
}

/// What a request leaves to do once the session is released.
enum After {
    Nothing,
    /// Run the main program from its start.
    Run,
    /// Go on from a stop.
    Go(fn(&mut Vm<'static, WebHost>, &Module) -> Result<(), VmError>),
    /// End the program.
    End,
}

fn serve_request(request: Request) {
    let seq = request.seq;
    let outcome = SESSION.with(|s| {
        let Ok(mut guard) = s.try_borrow_mut() else { return Err("the program is busy") };
        let Some(session) = guard.as_mut().filter(|s| s.end.is_some()) else { return Err("no program under a session") };
        if matches!(request.command, Command::Stop) {
            return Ok(After::End);
        }
        // (a pause while the program runs between two time slices: the
        // debugger watches from now on, and the next slice stops at once)
        if matches!(request.command, Command::Pause) && !session.stopped {
            session.vm.debug_mode = true;
        }
        let paused = session.stopped && session.slice.is_none();
        let mut end = session.end.take().unwrap_or_default();
        let (reply, control) = end.handle(&mut session.vm, &session.module, request, paused);
        let after = match control {
            Control::Start { .. } => After::Run,
            Control::Continue => After::Go(|vm, m| vm.resume(m)),
            Control::StepIn => After::Go(|vm, m| vm.step_into(m)),
            Control::StepOver => After::Go(|vm, m| vm.step_over(m)),
            Control::StepOut => After::Go(|vm, m| vm.step_out(m)),
            Control::Stop => After::End,
            // (INPUT on the web is answered in the program's page)
            Control::None | Control::Pause | Control::Input(_) => After::Nothing,
        };
        if matches!(after, After::Go(_)) {
            end.resumed();
            session.stopped = false;
        }
        session.end = Some(end);
        if let Some(reply) = reply {
            emit(reply);
        }
        Ok(after)
    });
    let after = match outcome {
        Ok(after) => after,
        Err(e) => {
            emit(Message::error(seq, e));
            return;
        }
    };
    match after {
        After::Nothing => {}
        After::Run => {
            let result = SESSION.with(|s| {
                let mut guard = s.try_borrow_mut().ok()?;
                let session = guard.as_mut()?;
                Some(run_step(session, Slice::Main, |vm, module| vm.run(module)).0)
            });
            if let Some(result) = result {
                settle(Slice::Main, result, false);
            }
        }
        After::Go(step) => {
            emit(Message::new(EventBody::Continued));
            let result = SESSION.with(|s| {
                let mut guard = s.try_borrow_mut().ok()?;
                let session = guard.as_mut()?;
                Some(run_step(session, Slice::Debug, step))
            });
            if let Some((result, main_done)) = result {
                settle(Slice::Debug, result, main_done);
            }
        }
        After::End => {
            if seq != 0 {
                emit(Message::reply(seq, EventBody::Ok));
            }
            rapidr_runtime_web::object_web::end_program();
            dialog::clear_modals();
            SESSION.with(|s| {
                if let Ok(mut slot) = s.try_borrow_mut() {
                    *slot = None;
                }
            });
            dialog::set_yielded(false);
            emit(Message::new(EventBody::Exited { code: 0 }));
        }
    }
}
