//! RapidR Studio's components in the browser: RPROJECT, RLANGUAGESERVICE
//! and RPROGRAMSESSION are `rapidr_studio`'s (the same as on the desktop);
//! this is the web's host. The program under development is compiled here
//! (the page's compiler, which the interpreter's entry registers) and run by
//! the page's `RAPIDR_STUDIO_HOST` in a sandboxed frame of its own (an
//! opaque origin: it can't reach the IDE), speaking the session protocol
//! over a MessagePort; what it sends comes back through
//! `rapidr_studio::channel` (the interpreter's `studio_session_incoming`).

use std::cell::Cell;

use rapidr_studio::channel::ChannelTransport;
use rapidr_studio::{Host, Transport};
use wasm_bindgen::{JsCast, JsValue};

use crate::object_web::rp_fire_event_args;
use crate::value::Value;

/// Compiles a program from its files (main's path, (path, text) of each).
pub type Compiler = fn(&str, Vec<(String, String)>) -> Result<Vec<u8>, String>;

thread_local! {
    static COMPILER: Cell<Option<Compiler>> = const { Cell::new(None) };
}

/// The page's compiler (the interpreter's entry gives it).
pub fn set_compiler(c: Compiler) {
    COMPILER.with(|k| k.set(Some(c)));
}

#[derive(Clone, Copy)]
pub struct Web;

/// The page's `RAPIDR_STUDIO_HOST` (ide/web/studio.js): `run(bytes,
/// program, args)`, `send(json)`, `stop()`.
fn studio_host() -> Option<JsValue> {
    let w = web_sys::window()?;
    js_sys::Reflect::get(&w, &"RAPIDR_STUDIO_HOST".into()).ok().filter(|h| h.is_object())
}

fn host_call(name: &str, args: &[JsValue]) -> Result<JsValue, String> {
    let host = studio_host().ok_or("this page can't run programs (no RAPIDR_STUDIO_HOST)")?;
    let f = js_sys::Reflect::get(&host, &name.into()).ok().and_then(|f| f.dyn_into::<js_sys::Function>().ok()).ok_or_else(|| format!("RAPIDR_STUDIO_HOST.{name} is missing"))?;
    let list = js_sys::Array::new();
    for a in args {
        list.push(a);
    }
    f.apply(&host, &list).map_err(|e| e.as_string().unwrap_or_else(|| format!("{e:?}")))
}

impl Host for Web {
    fn fire(self, name: &str, event: &str, args: &[Value]) {
        rp_fire_event_args(name, event, args);
    }

    fn launch(self, program: &str, args: &[String], theme: &str) -> Result<Box<dyn Transport>, String> {
        if program.is_empty() {
            return Err("no program to run".into());
        }
        let compile = COMPILER.with(Cell::get).ok_or("no compiler on this page")?;
        let (main, files) = rapidr_studio::project::program_files(program)?;
        let bytes = compile(&main, files)?;
        let js_args = js_sys::Array::new();
        for a in args {
            js_args.push(&JsValue::from_str(a));
        }
        host_call("run", &[js_sys::Uint8Array::from(bytes.as_slice()).into(), JsValue::from_str(program), js_args.into(), JsValue::from_str(theme)])?;
        Ok(Box::new(ChannelTransport::new(
            |json| host_call("send", &[JsValue::from_str(json)]).map(drop),
            || {
                let _ = host_call("stop", &[]);
            },
        )))
    }

    fn list_files(self, folder: &str) -> Vec<String> {
        crate::object_web::stored_names_in(folder)
    }
}

/// A property of one of Studio's components (`None`: not its own).
pub fn get(type_name: &str, name: &str, prop: &str) -> Option<Value> {
    rapidr_studio::get(type_name, name, prop)
}

/// Sets a property; whether it was the component's own.
pub fn set(type_name: &str, name: &str, prop: &str, v: &Value) -> bool {
    rapidr_studio::set(Web, type_name, name, prop, v)
}

/// A method (`None`: not the component's own).
pub fn call(type_name: &str, name: &str, method: &str, args: &[Value]) -> Option<Value> {
    rapidr_studio::call(Web, type_name, name, method, args)
}

/// The program's frame sent something: the sessions' news as events.
pub fn poll() -> bool {
    rapidr_studio::poll(Web)
}
