//! The interim program end: `rapidr __debuggee <file> [args]`.
//!
//! Replaced by `rapidr run --session` when lane L-SESS lands
//! (rapidr-session and the VM's debugger primitives): then
//! `crate::program_end_command` runs that instead, and this folder is
//! deleted. It speaks the same protocol (`crate::session_wire`) on today's
//! VM, with what that VM can do: breakpoints by file and line, stepping,
//! pause (through the host's yields), the stack and variables, evaluating
//! variables / elements / fields / properties, setting them to a literal,
//! post-mortem break on a run-time error.

mod compile;
mod end;
mod host;
mod inspect;

use std::process::ExitCode;

use crate::session_wire::{Event, EventBody};

/// `rapidr __debuggee <file> [args]` (`args`: the file, then its
/// arguments).
pub fn debuggee_main(args: &[String]) -> ExitCode {
    let Some((program, rest)) = args.split_first() else {
        eprintln!("rapidr __debuggee <file> [args]");
        return ExitCode::from(2);
    };
    let compiled = std::fs::read(program)
        .map_err(|e| format!("{program}: {e}"))
        .and_then(|bytes| {
            if bytes.starts_with(rapidr_bytecode::MAGIC) {
                Err(format!("{program}: a compiled program can't be debugged: debug its source file"))
            } else {
                compile::compile_to_bytecode(program)
            }
        });
    let compiled = match compiled {
        Ok(c) => c,
        Err(e) => {
            // (the compiler's own text, as `rapidr run` prints it)
            end::send(&Event::new(EventBody::Output { stream: "stderr".into(), text: format!("{e}\n") }));
            end::send(&Event::new(EventBody::Exited { code: 1 }));
            return ExitCode::from(1);
        }
    };
    for w in &compiled.warnings {
        end::send(&Event::new(EventBody::Output { stream: "stderr".into(), text: format!("warning: {w}\n") }));
    }
    let code = end::run(compiled.module, program, rest.to_vec());
    ExitCode::from(code.clamp(0, 255) as u8)
}
