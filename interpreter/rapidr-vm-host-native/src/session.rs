//! The program's end of a session on the desktop: `rapidr run --session`
//! (rapidr-session's protocol; docs/ide-plan.md §3.3).
//!
//! Requests arrive on standard input, one JSON object per line, read by a
//! thread of their own; events leave on standard output, framed
//! (`rapidr_session::protocol::frame`) among what the program prints.
//! The program runs on the main thread (the window system's); a stop blocks
//! it there and serves the IDE's requests ([`BlockingDebugger`]) until one
//! lets it go on. What arrives while it runs reaches it at its next
//! statement (the VM's interrupt); while it waits for the window system,
//! when it next runs code (an event, a timer). Stop is answered by the
//! reader itself: the process ends.

use std::collections::VecDeque;
use std::io::{BufRead, Write};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc;
use std::sync::{Arc, Condvar, Mutex};

use rapidr_bytecode::Module;
use rapidr_session::program::BlockingDebugger;
use rapidr_session::protocol::{frame, Command, Event, EventBody, Request, PROTOCOL_VERSION};
use rapidr_vm::Vm;

use crate::NativeHost;
use rapidr_runtime_core::object as obj;

/// Sends an event to the IDE (standard output, framed).
pub fn send(event: &Event) {
    let mut out = std::io::stdout().lock();
    let _ = out.write_all(frame(event).as_bytes());
    let _ = out.flush();
}

/// Lines for INPUT, from `input` requests.
#[derive(Default)]
struct InputQueue {
    lines: Mutex<(VecDeque<String>, bool)>,
    ready: Condvar,
}

impl InputQueue {
    fn push(&self, line: String) {
        self.lines.lock().unwrap().0.push_back(line);
        self.ready.notify_all();
    }

    fn close(&self) {
        self.lines.lock().unwrap().1 = true;
        self.ready.notify_all();
    }

    fn pop(&self) -> Option<String> {
        let mut guard = self.lines.lock().unwrap();
        loop {
            if let Some(line) = guard.0.pop_front() {
                return Some(line);
            }
            if guard.1 {
                return None;
            }
            guard = self.ready.wait(guard).unwrap();
        }
    }
}

/// Runs the program in `bytes` (`program`: its path, as the IDE named it)
/// as the program's end of a session; returns its exit code.
pub fn run_session(bytes: &[u8], program: &str) -> Result<i32, String> {
    let module = Module::from_bytes(bytes).map_err(|e| format!("decode error: {e}"))?;
    rapidr_runtime_core::value::resources::set_all(&module.resources);

    let input = Arc::new(InputQueue::default());
    let mut host = NativeHost {
        output: Some(Box::new(|text: &str| send(&Event::new(EventBody::Output { stream: "stdout".into(), text: text.to_string() })))),
        input_source: Some(Box::new({
            let input = input.clone();
            move || input.pop()
        })),
        on_form: Some(Box::new(|id: &str, shown: bool| {
            let body = if shown { EventBody::FormShown { id: id.to_string(), caption: None } } else { EventBody::FormClosed { id: id.to_string() } };
            send(&Event::new(body));
        })),
        ..NativeHost::default()
    };
    let mut vm = Vm::new(&mut host);
    let interrupt = vm.interrupt.clone();
    let (tx, rx) = mpsc::channel::<Request>();
    spawn_reader(tx, interrupt, input);

    send(&Event::new(EventBody::Ready { protocol: PROTOCOL_VERSION, runtime: format!("RapidR {}", env!("CARGO_PKG_VERSION")), program: program.to_string() }));
    let mut debugger = BlockingDebugger::new(rx, Box::new(send));
    if debugger.until_start(&mut vm, &module).is_none() {
        return Ok(0);
    }
    vm.debugger = Some(Box::new(debugger));
    // (its waits turn now and then: a pause reaches a program that waits
    // for its events)
    obj::rp_set_debug_poll(true);
    let result = crate::run_module(&module, &mut vm);
    vm.debugger = None;
    let code = match result {
        Ok(()) => 0,
        Err(e) => {
            send(&Event::new(EventBody::Output { stream: "stderr".into(), text: format!("{e}\n") }));
            1
        }
    };
    send(&Event::new(EventBody::Exited { code }));
    Ok(code)
}

/// Reads requests from standard input: `stop` ends the process, `input`
/// feeds INPUT, the rest go to the program (and raise its interrupt). The
/// IDE gone (the end of input): the program ends.
fn spawn_reader(tx: mpsc::Sender<Request>, interrupt: Arc<AtomicBool>, input: Arc<InputQueue>) {
    std::thread::spawn(move || {
        let stdin = std::io::stdin();
        for line in stdin.lock().lines() {
            let Ok(line) = line else { break };
            if line.trim().is_empty() {
                continue;
            }
            let request = match Request::from_json(&line) {
                Ok(r) => r,
                Err(e) => {
                    send(&Event::error(0, e));
                    continue;
                }
            };
            match &request.command {
                Command::Stop => {
                    if request.seq != 0 {
                        send(&Event::reply(request.seq, EventBody::Ok));
                    }
                    send(&Event::new(EventBody::Exited { code: 0 }));
                    std::process::exit(0);
                }
                Command::Input { text: Some(text), .. } => {
                    input.push(text.clone());
                    if request.seq != 0 {
                        send(&Event::reply(request.seq, EventBody::Ok));
                    }
                }
                _ => {
                    if tx.send(request).is_err() {
                        break;
                    }
                    interrupt.store(true, Ordering::Relaxed);
                }
            }
        }
        // (the IDE is gone)
        input.close();
        std::process::exit(0);
    });
}
