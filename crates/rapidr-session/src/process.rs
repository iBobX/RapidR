//! The desktop transport: the program in a child process,
//! `rapidr run --session <program> [args]`, requests on its standard input
//! (one JSON object per line), events framed on its standard output among
//! what it prints ([`crate::protocol::Deframer`]), its standard error as
//! output too. The program can't reach the IDE's memory, and Stop is a
//! kill.

use std::io::{Read, Write};
use std::path::Path;
use std::process::{Child, ChildStdin, Command as Process, Stdio};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError};
use std::time::Duration;

use crate::client::Transport;
use crate::protocol::{Deframer, Event, EventBody, Incoming, Request};

/// A program run as a child process.
pub struct ProcessTransport {
    child: Child,
    stdin: Option<ChildStdin>,
    incoming: Receiver<Incoming>,
    exit: Option<i32>,
}

impl ProcessTransport {
    /// Runs `runtime run --session program args…` (`runtime`: the `rapidr`
    /// executable), in `cwd` if given.
    pub fn spawn(runtime: &Path, program: &str, args: &[String], cwd: Option<&Path>) -> std::io::Result<Self> {
        let mut command = Process::new(runtime);
        command.arg("run").arg("--session").arg(program).args(args);
        command.stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::piped());
        if let Some(dir) = cwd {
            command.current_dir(dir);
        }
        let mut child = command.spawn()?;
        let (tx, incoming) = mpsc::channel();
        let mut stdout = child.stdout.take().expect("piped stdout");
        let out_tx = tx.clone();
        std::thread::spawn(move || {
            let mut deframer = Deframer::new();
            let mut buf = [0u8; 8192];
            loop {
                match stdout.read(&mut buf) {
                    Ok(0) | Err(_) => break,
                    Ok(n) => {
                        for item in deframer.push(&buf[..n]) {
                            if out_tx.send(item).is_err() {
                                return;
                            }
                        }
                    }
                }
            }
            if let Some(rest) = deframer.finish() {
                let _ = out_tx.send(rest);
            }
        });
        let mut stderr = child.stderr.take().expect("piped stderr");
        std::thread::spawn(move || {
            let mut buf = [0u8; 8192];
            let mut pending: Vec<u8> = Vec::new();
            loop {
                match stderr.read(&mut buf) {
                    Ok(0) | Err(_) => break,
                    Ok(n) => {
                        pending.extend_from_slice(&buf[..n]);
                        // (whole characters only)
                        let cut = match std::str::from_utf8(&pending) {
                            Ok(_) => pending.len(),
                            Err(e) if e.error_len().is_none() => e.valid_up_to(),
                            Err(_) => pending.len(),
                        };
                        let text = String::from_utf8_lossy(&pending[..cut]).into_owned();
                        pending.drain(..cut);
                        let event = Event::new(EventBody::Output { stream: "stderr".into(), text });
                        if tx.send(Incoming::Event(event)).is_err() {
                            return;
                        }
                    }
                }
            }
        });
        let stdin = child.stdin.take();
        Ok(ProcessTransport { child, stdin, incoming, exit: None })
    }
}

impl Transport for ProcessTransport {
    fn send(&mut self, request: &Request) -> Result<(), String> {
        let stdin = self.stdin.as_mut().ok_or("the program has ended")?;
        let mut line = request.to_json();
        line.push('\n');
        stdin.write_all(line.as_bytes()).and_then(|_| stdin.flush()).map_err(|e| format!("the program has ended ({e})"))
    }

    fn receive(&mut self, timeout: Option<Duration>) -> Vec<Incoming> {
        let mut out = Vec::new();
        if let Some(t) = timeout {
            match self.incoming.recv_timeout(t) {
                Ok(item) => out.push(item),
                Err(RecvTimeoutError::Timeout | RecvTimeoutError::Disconnected) => return out,
            }
        }
        while let Ok(item) = self.incoming.try_recv() {
            out.push(item);
        }
        out
    }

    fn kill(&mut self) {
        self.stdin = None;
        let _ = self.child.kill();
        if let Ok(status) = self.child.wait() {
            self.exit.get_or_insert(status.code().unwrap_or(-1));
        }
    }

    fn ended(&mut self) -> Option<i32> {
        if self.exit.is_none() {
            if let Ok(Some(status)) = self.child.try_wait() {
                self.exit = Some(status.code().unwrap_or(-1));
            }
        }
        self.exit
    }
}

impl Drop for ProcessTransport {
    fn drop(&mut self) {
        if self.exit.is_none() {
            self.kill();
        }
    }
}
