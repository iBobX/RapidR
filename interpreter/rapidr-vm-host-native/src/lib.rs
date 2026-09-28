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
use rapidr_vm::{Host, Vm};

/// Native host: routes the [`Host`] surface to `rapidr-runtime-core`.
#[derive(Default)]
pub struct NativeHost {
    /// Registered (component, event) → bytecode fn index. Kept here for
    /// debugging — the authoritative copy lives in
    /// `rapidr-runtime-core`'s `EVENT_HANDLERS`.
    pub events: Vec<(String, String, u32)>,
    /// Set to true once any GUI component has been created — signals
    /// the CLI driver that it should call [`run_event_loop`].
    pub has_components: bool,
}

impl Host for NativeHost {
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
        }
        if key == "lbound" || key == "ubound" {
            let arr = args.first().cloned().unwrap_or_else(v_null);
            let dim = args.get(1).map(|v| v.to_i64()).unwrap_or(1);
            return rapidr_value::array_bound(&arr, dim, key == "ubound")
                .map(v_int)
                .ok_or_else(|| format!("{}: argument is not an array, or it has no dimension {dim}", key.to_uppercase()));
        }
        Ok(call_builtin_native(name, args))
    }

    fn create_comp(&mut self, kind: &str, id: &str) -> Result<Value, String> {
        rp_create_component(id, kind);
        self.has_components = true;
        Ok(v_str(id))
    }

    fn set_prop(&mut self, id: &str, name: &str, value: Value) -> Result<(), String> {
        rp_comp_set(id, name, value);
        Ok(())
    }

    fn get_prop(&mut self, id: &str, name: &str) -> Result<Value, String> {
        // Module-style constants (e.g. `math.pi`) — IMPORT is a no-op in
        // bcgen so these never become real components. Resolve a few
        // common ones here for demo parity.
        if let Some(v) = module_constant(id, name) {
            return Ok(v);
        }
        Ok(rp_comp_get(id, name))
    }

    fn call_method(&mut self, id: &str, method: &str, args: &[Value]) -> Result<Value, String> {
        Ok(rp_comp_method(id, method, args))
    }

    fn take_events(&mut self) -> Vec<(u32, Vec<Value>)> {
        EVENTS.with(|q| q.borrow_mut().drain(..).collect())
    }

    fn defer_events(&mut self, events: Vec<(u32, Vec<Value>)>) {
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

    fn pump(&mut self) -> Option<Value> {
        obj::rp_pump_wait()
    }

    fn register_event(&mut self, id: &str, event: &str, handler_fn_index: u32) -> Result<(), String> {
        obj::rp_bind_event_indirect(id, event, handler_fn_index);
        self.events.push((id.to_string(), event.to_string(), handler_fn_index));
        Ok(())
    }

    fn print(&mut self, s: &str) -> Result<(), String> {
        // Keep the shared console cursor current (CSRLIN, POS, LOCATE).
        rapidr_value::console::track(s);
        let stdout = io::stdout();
        let mut h = stdout.lock();
        h.write_all(s.as_bytes()).map_err(|e| e.to_string())?;
        h.flush().map_err(|e| e.to_string())?;
        Ok(())
    }

    fn input(&mut self) -> Result<String, String> {
        let mut line = String::new();
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
fn call_builtin_native(name: &str, args: &[Value]) -> Value {
    // Strip BASIC type-suffix (e.g. MID$ → mid, INT% → int) and lowercase.
    let mut lower = name.to_lowercase();
    if matches!(lower.chars().last(), Some('$' | '%' | '#' | '&' | '!')) {
        lower.pop();
    }
    let a0 = args.first().cloned().unwrap_or_else(v_null);
    let a1 = args.get(1).cloned().unwrap_or_else(v_null);
    let a2 = args.get(2).cloned().unwrap_or_else(v_null);

    match lower.as_str() {
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
        "rinstr" => rp_rinstr(&a0, &a1),
        "format" => rp_format(&a0, &a1),
        "strf" => rp_strf(&a0),

        // --- Numeric / math ---
        "int" => rp_int(&a0),
        "abs" => rp_abs(&a0),
        "sgn" => rp_sgn(&a0),
        "sqr" => rp_sqr(&a0),
        "sin" => rp_sin(&a0),
        "cos" => rp_cos(&a0),
        "tan" => rp_tan(&a0),
        "atn" => rp_atn(&a0),
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
        "sizeof" => rp_sizeof(&a0),

        // --- Time / system ---
        "date" | "date_func" | "date$" => rp_date(),
        "time" | "time_func" | "time$" => rp_time(),
        "timer" => rp_timer(),
        "sleep" => { rp_sleep(&a0); v_null() }
        "command" => rp_command(),
        "environ" => rp_environ(&a0),
        "doevents" => { rp_doevents(); v_null() }
        "end" => { rp_end(); v_null() }
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
        "shellwait" => rp_shellwait(&a0),
        "beep" => { rp_beep(); v_null() }
        "replacesubstr" => rp_replacesubstr(&a0, &a1, &a2),
        "sound" => { rp_sound(&a0, &a1); v_null() }
        "playsound" => rp_playsound(&a0),
        "isnumeric" => rp_isnumeric(&a0),

        // --- Array ---
        "lbound" => rp_lbound(&[a0]),
        "ubound" => rp_ubound(&[a0]),

        // --- File / directory ---
        "freefile" => rp_freefile(),
        "eof" => rp_eof(&a0),
        "lof" => rp_lof(&a0),
        "filelen" => rp_filelen(&a0),
        "line_input" => rp_line_input(&a0),
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
        "__objget" => rp_comp_get(&a0.to_string_val(), &a1.to_string_val()),
        "__objset" => {
            rp_comp_set(&a0.to_string_val(), &a1.to_string_val(), args.get(2).cloned().unwrap_or_else(v_null));
            v_null()
        }
        "__objcall" => rp_comp_method(&a0.to_string_val(), &a1.to_string_val(), args.get(2..).unwrap_or(&[])),
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
    static EVENTS: RefCell<VecDeque<(u32, Vec<Value>)>> = const { RefCell::new(VecDeque::new()) };
}

/// Routes the runtime's events for bytecode handlers into [`EVENTS`].
fn install_event_queue() -> Option<obj::IndirectDispatcher> {
    obj::rp_set_cooperative_waits(true);
    obj::rp_set_event_dispatcher(Box::new(|fn_index, args| {
        EVENTS.with(|q| q.borrow_mut().push_back((fn_index, args.to_vec())));
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

/// Runs the program's windows until none is left: pumps UI events and
/// runs the handlers they queue. Call it after `vm.run(&module)` when
/// `host.has_components` is true.
pub fn run_event_loop<H: Host + ?Sized>(module: &Module, vm: &mut Vm<'_, H>) {
    let prev = install_event_queue();
    serve_app(module, vm);
    remove_event_queue(prev);
}

fn serve_app<H: Host + ?Sized>(module: &Module, vm: &mut Vm<'_, H>) {
    obj::rp_begin_app_wait();
    loop {
        // Each handler runs to completion before the next UI event.
        for (fn_index, args) in vm.host_mut().take_events() {
            if let Err(e) = vm.invoke_function(module, fn_index, args) {
                eprintln!("[rapidr] event handler #{fn_index} failed: {e}");
            }
        }
        if obj::rp_pump_wait().is_some() {
            return;
        }
    }
}

/// Decode an in-memory `.rrbc` byte slice and execute it on a fresh
/// [`NativeHost`].
///
/// Events go through a queue the VM drains itself (see [`EVENTS`]):
/// `Form.ShowModal` in `MAIN` becomes a wait the VM serves, running the
/// form's handlers between UI events. After `MAIN`, if components exist,
/// the program's windows run until they're all closed.
///
/// Used by both the CLI's `run-bc` subcommand and the
/// `rapidrintr-runner` stub binary (Phase 8: bytecode → single exe).
pub fn run_bytes(bytes: &[u8]) -> Result<(), String> {
    let module = Module::from_bytes(bytes).map_err(|e| format!("decode error: {e}"))?;
    rapidr_runtime_core::value::resources::set_all(&module.resources);
    let mut host = NativeHost::default();
    let mut vm = Vm::new(&mut host);
    let prev = install_event_queue();

    let main_result = vm.run(&module).map_err(|e| format!("vm error: {e}"));
    if main_result.is_ok() && vm.host_mut().has_components {
        serve_app(&module, &mut vm);
    }

    // Stop timers first so FLTK timeouts queued before the loop ended
    // don't fire into a finished program.
    obj::rp_stop_all_timers();
    obj::rp_mark_shutting_down();
    remove_event_queue(prev);
    main_result
}
