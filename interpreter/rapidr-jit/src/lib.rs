//! RapidR's JIT (docs/jit-design.md): typed bytecode that runs hot —
//! a loop turning thousands of times, a routine called a thousand times —
//! compiled to machine code while the program runs, with exactly the
//! interpreter's results, errors and debugger.
//!
//! Tier 1: functions and loops whose instructions are numbers, booleans,
//! one-index arrays, the pure maths builtins and calls between such
//! routines; everything else stays interpreted (and compiled code leaves
//! for the interpreter at the instruction it doesn't run).
//!
//! Only where a JIT may run: 64-bit x86 and ARM desktops. On the web,
//! iOS and 32-bit x86 [`install`] does nothing (the crate has no code
//! generator there). `RAPIDR_JIT=off` turns it off, `always` compiles
//! everything it can at once (the tests' way: every conformance case
//! through compiled code).

#[cfg(all(any(target_arch = "x86_64", target_arch = "aarch64"), not(target_os = "ios"), not(target_family = "wasm")))]
mod compile;
#[cfg(all(any(target_arch = "x86_64", target_arch = "aarch64"), not(target_os = "ios"), not(target_family = "wasm")))]
mod engine;
pub mod kinds;
#[cfg(all(any(target_arch = "x86_64", target_arch = "aarch64"), not(target_os = "ios"), not(target_family = "wasm")))]
mod native;

use rapidr_vm::{Host, Vm};

/// `RAPIDR_JIT`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    /// The interpreter only.
    Off,
    /// Hot code compiled (the default).
    On,
    /// Everything it can, at once.
    Always,
}

impl Mode {
    /// From `RAPIDR_JIT` (`off`, `on`, `always`; anything else: on).
    pub fn from_env() -> Mode {
        match std::env::var("RAPIDR_JIT").map(|v| v.trim().to_ascii_lowercase()) {
            Ok(v) if v == "off" || v == "0" || v == "no" => Mode::Off,
            Ok(v) if v == "always" => Mode::Always,
            _ => Mode::On,
        }
    }
}

/// Whether this build has a JIT for this machine at all.
pub const SUPPORTED: bool = cfg!(all(any(target_arch = "x86_64", target_arch = "aarch64"), not(target_os = "ios"), not(target_family = "wasm")));

/// Installs the JIT on `vm` (`RAPIDR_JIT`'s mode): `Ok` with what was
/// done, `Err` with why the interpreter runs alone — for
/// `rapidr run --verbose`.
pub fn install<H: Host + ?Sized>(vm: &mut Vm<'_, H>) -> Result<String, String> {
    install_mode(vm, Mode::from_env())
}

/// [`install`] with a mode.
pub fn install_mode<H: Host + ?Sized>(vm: &mut Vm<'_, H>, mode: Mode) -> Result<String, String> {
    if mode == Mode::Off {
        return Err("JIT: off (RAPIDR_JIT=off)".into());
    }
    #[cfg(all(any(target_arch = "x86_64", target_arch = "aarch64"), not(target_os = "ios"), not(target_family = "wasm")))]
    {
        native::probe().map_err(|why| format!("JIT: off, {why}; the interpreter runs everything"))?;
        vm.set_jit(Box::new(engine::Engine::new(mode)));
        Ok(format!("JIT: on{}", if mode == Mode::Always { " (always)" } else { "" }))
    }
    #[cfg(not(all(any(target_arch = "x86_64", target_arch = "aarch64"), not(target_os = "ios"), not(target_family = "wasm"))))]
    {
        let _ = vm;
        Err("JIT: not on this kind of computer; the interpreter runs everything".into())
    }
}

/// What the JIT did in `vm`'s run (`rapidr run --verbose`).
pub fn report<H: Host + ?Sized>(vm: &Vm<'_, H>) -> Option<String> {
    vm.jit().map(|j| j.report())
}
