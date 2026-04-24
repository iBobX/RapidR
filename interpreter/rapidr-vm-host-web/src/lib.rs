//! Browser [`Host`] implementation for the RapidR bytecode VM.
//!
//! Routes builtins / component / DOM ops to `rapidr-runtime-web`, and
//! installs a thread-local indirect dispatcher so DOM events can
//! re-enter the [`Vm`] to invoke bytecode handler functions.
//!
//! Designed to be wrapped in a `wasm-bindgen` shim by a thin
//! application crate (`rapidrintr.wasm`) that loads a `.rrbc` module
//! at runtime.

#![allow(clippy::too_many_lines)]

use std::cell::Cell;

use rapidr_bytecode::Module;
use rapidr_runtime_web::object_web as obj;
use rapidr_runtime_web::prelude::*;
use rapidr_value::{v_dbl, v_int, v_null, v_str, Value};
use rapidr_vm::{Host, Vm};
use wasm_bindgen::prelude::*;

/// Browser host: routes the [`Host`] surface to `rapidr-runtime-web`.
#[derive(Default)]
pub struct WebHost {
    /// Set to true once any DOM/GUI component has been created.
    pub has_components: bool,
}

impl Host for WebHost {
    fn call_builtin(&mut self, name: &str, args: &[Value]) -> Result<Value, String> {
        Ok(call_builtin_web(name, args))
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
        if let Some(v) = module_constant(id, name) {
            return Ok(v);
        }
        Ok(rp_comp_get(id, name))
    }

    fn call_method(&mut self, id: &str, method: &str, args: &[Value]) -> Result<Value, String> {
        Ok(rp_comp_method(id, method, args))
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
        // Browser has no synchronous stdin; return empty string.
        Ok(String::new())
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
        "rinstr" => rp_rinstr(&a0, &a1),
        "format" => rp_format(&a0, &a1),
        "strf" => rp_strf(&a0),

        // Numeric / math
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

        // Time / system
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
        "direxists" => rp_direxists(&a0),
        "fileexists" => rp_fileexists(&a0),
        "shell" => rp_shell(&a0),
        "shellwait" => rp_shellwait(&a0),
        "beep" => { rp_beep(); v_null() }
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

        _ => v_null(),
    }
}

// ---------- Indirect-dispatch event loop ----------

thread_local! {
    static ACTIVE_VM: Cell<*mut ()> = const { Cell::new(std::ptr::null_mut()) };
}

struct VmCtx<'a, 'h, H: Host + ?Sized> {
    vm: &'a mut Vm<'h, H>,
    module: &'a Module,
}

/// Set up the indirect dispatcher so DOM events re-enter the supplied
/// [`Vm`]. Unlike the native version, this does NOT block — the
/// browser's event loop drives subsequent callbacks. The caller is
/// expected to keep `vm`, `module`, and `host` alive (e.g. by storing
/// them in a `'static` cell) for as long as the page lives.
///
/// # Safety
/// The pointers stored in the thread-local must remain valid for the
/// lifetime of the page. The wrapper `BcRuntime::start` below takes
/// care of this by leaking everything intentionally into a
/// `Box::leak`'d `'static` slot.
pub fn install_dispatcher<H: Host + ?Sized + 'static>(module: *const Module, vm: *mut Vm<'_, H>) {
    // Stash a leaked `VmCtx` so the dispatcher closure can find it.
    let ctx = Box::leak(Box::new(VmCtx::<'static, 'static, H> {
        // SAFETY: caller guarantees `vm` and `module` live for 'static.
        vm: unsafe { &mut *(vm as *mut Vm<'static, H>) },
        module: unsafe { &*(module as *const Module) },
    }));
    let ctx_ptr = (ctx as *mut VmCtx<'_, '_, H>) as *mut ();
    ACTIVE_VM.with(|c| c.set(ctx_ptr));

    let _ = obj::rp_set_event_dispatcher(Box::new(|fn_index, args| {
        ACTIVE_VM.with(|c| {
            let p = c.get() as *mut VmCtx<'_, '_, H>;
            if p.is_null() { return; }
            let ctx = unsafe { &mut *p };
            if let Err(e) = ctx.vm.invoke_function(ctx.module, fn_index, args.to_vec()) {
                web_sys::console::error_1(
                    &JsValue::from_str(&format!("[rapidr] event handler #{fn_index} failed: {e}")),
                );
            }
        });
    }));
}

// ---------- wasm-bindgen entry point ----------

/// Decode and execute a `.rrbc` module from a byte slice.
///
/// Steps performed:
/// 1. Decode the `.rrbc` bytes into a [`Module`] (errors → JS error).
/// 2. Run `__main` to set up globals and create components.
/// 3. If components were created, install the indirect dispatcher so
///    DOM events fire bytecode handlers; the browser event loop then
///    drives the program. Otherwise it's a "compute-only" run.
///
/// All of `host`, `vm`, and `module` are leaked into `'static` storage
/// so DOM callbacks can reach them; this is safe because a browser page
/// keeps its WASM instance alive for its full lifetime.
#[wasm_bindgen]
pub fn rapidr_run_bc(bytes: &[u8]) -> Result<(), JsValue> {
    let module = Module::from_bytes(bytes)
        .map_err(|e| JsValue::from_str(&format!("rrbc decode error: {e}")))?;
    let module: &'static Module = Box::leak(Box::new(module));

    let host: &'static mut WebHost = Box::leak(Box::new(WebHost::default()));
    let vm: &'static mut Vm<'static, WebHost> = Box::leak(Box::new(Vm::new(host)));

    vm.run(module).map_err(|e| JsValue::from_str(&format!("vm error: {e}")))?;

    if vm.host_mut().has_components {
        install_dispatcher::<WebHost>(module as *const _, vm as *mut _);
    }
    Ok(())
}
