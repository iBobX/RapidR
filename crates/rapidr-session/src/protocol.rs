//! The messages between the IDE and the program it runs, as JSON.
//!
//! The IDE sends [`Request`]s; the program sends [`Event`]s — the reply to a
//! request (with `re` = its `seq`) or something that happened (`re` absent:
//! output, a stop, a form shown, the end). Every request with a `seq` gets
//! exactly one reply: its own kind ([`EventBody::StackTrace`] …),
//! [`EventBody::Ok`] or [`EventBody::Error`].
//!
//! ```json
//! {"seq":3,"type":"setBreakpoints","file":"util.inc","breakpoints":[{"line":12}]}
//! {"re":3,"type":"breakpoints","file":"util.inc","breakpoints":[{"line":12,"verified":true,"actualLine":12}]}
//! {"type":"stopped","reason":"breakpoint","file":"util.inc","line":12}
//! ```
//!
//! Files are named as the program's source map names them: the file's name
//! (`util.inc`), never its path — the program's own file included.
//!
//! On the desktop (`rapidr run --session`) requests arrive on the program's
//! standard input, one JSON object per line, and events leave on its
//! standard output framed as an OSC escape sequence ([`frame`]) so they can
//! share it with the program's own output (what a program prints outside
//! the protocol, e.g. by the runtime directly, is output too: [`Deframer`]).
//! On the web the same JSON travels over the preview frame's MessagePort.

use serde::{Deserialize, Serialize};

/// The protocol's version ([`EventBody::Ready`]).
pub const PROTOCOL_VERSION: u32 = 1;

/// A breakpoint the IDE asks for in a file.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct SourceBreakpoint {
    pub line: u32,
    /// Stop only when this BASIC expression is true (I6; carried, not yet
    /// evaluated).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub condition: Option<String>,
    /// Hit count condition (I6; carried).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hit: Option<String>,
    /// A logpoint's message (I6; carried).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub log: Option<String>,
}

/// What the IDE asks.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase", rename_all_fields = "camelCase")]
pub enum Command {
    /// Start the program (after the breakpoints are set). `program` and
    /// `args` are the ones it was opened with when absent.
    Start {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        program: Option<String>,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        args: Vec<String>,
        #[serde(default = "yes")]
        debug: bool,
        #[serde(default, skip_serializing_if = "is_false")]
        stop_on_entry: bool,
        #[serde(default, skip_serializing_if = "is_false")]
        break_on_error: bool,
    },
    /// End the program (a kill: no handler runs).
    Stop,
    /// Stop at the next statement the program runs.
    Pause,
    /// Replaces the breakpoints of `file`.
    SetBreakpoints {
        file: String,
        #[serde(default)]
        breakpoints: Vec<SourceBreakpoint>,
    },
    /// Stop at a run-time error's statement (before it unwinds).
    SetBreakOnError { enabled: bool },
    Continue,
    StepIn,
    StepOver,
    StepOut,
    /// The stopped program's frames, innermost first.
    StackTrace,
    /// A frame's scopes (locals, globals).
    Scopes { frame: u32 },
    /// The variables of a scope or the children of a value (`ref`), `count`
    /// of them from `start` (arrays page).
    Variables {
        #[serde(rename = "ref")]
        reference: u32,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        start: Option<u32>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        count: Option<u32>,
    },
    /// Evaluates a BASIC expression in a frame (the innermost when absent).
    /// `context` `"repl"` (the Immediate window): `? expr` is evaluated,
    /// anything else runs as statements in the frame.
    Evaluate {
        expr: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        frame: Option<u32>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        context: Option<String>,
    },
    /// Sets a variable of a frame (a local, else a global) to the value of
    /// the BASIC expression `value`.
    SetVariable {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        frame: Option<u32>,
        name: String,
        value: String,
    },
    /// Sets a component's property to the value of the BASIC expression
    /// `value` (live property preview).
    SetProperty { object: String, prop: String, value: String },
    /// A component's properties as the runtime holds them.
    Properties { object: String },
    /// Input for the program: a line for INPUT (`text`), or an event for a
    /// form drawn in the IDE (`form`, `event`: remote forms, I5).
    Input {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        text: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        form: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        event: Option<serde_json::Value>,
    },
}

fn yes() -> bool {
    true
}

fn is_false(b: &bool) -> bool {
    !*b
}

/// A request: a command and the number its reply carries (`0`: no reply
/// wanted).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Request {
    #[serde(default, skip_serializing_if = "is_zero")]
    pub seq: u64,
    #[serde(flatten)]
    pub command: Command,
}

fn is_zero(n: &u64) -> bool {
    *n == 0
}

impl Request {
    pub fn new(seq: u64, command: Command) -> Self {
        Request { seq, command }
    }

    pub fn to_json(&self) -> String {
        serde_json::to_string(self).unwrap_or_default()
    }

    pub fn from_json(text: &str) -> Result<Request, String> {
        serde_json::from_str(text).map_err(|e| format!("bad request: {e}"))
    }
}

/// Where a breakpoint landed.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PlacedBreakpoint {
    /// The line asked for.
    pub line: u32,
    /// Whether it stops anywhere.
    pub verified: bool,
    /// The line it stops at (the next with code).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub actual_line: Option<u32>,
}

/// One frame of the stopped program.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StackFrame {
    /// The frame's number (the outermost is 0): what `scopes`, `evaluate`
    /// and `setVariable` name.
    pub id: u32,
    /// The SUB or FUNCTION (`__main` for the program's top level).
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub file: Option<String>,
    pub line: u32,
}

/// A frame's scope: its variables are `variables { ref }`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScopeInfo {
    pub name: String,
    #[serde(rename = "ref")]
    pub reference: u32,
}

/// A variable (or an element, a field) and its value as text.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Variable {
    pub name: String,
    /// The value as the debugger shows it (strings quoted).
    pub value: String,
    /// `Integer`, `Double`, `String`, `Array`, the TYPE's name …
    pub kind: String,
    /// Non-zero when it has children (an array's elements, an object's
    /// fields): `variables { ref }`.
    #[serde(rename = "ref", default, skip_serializing_if = "is_zero32")]
    pub reference: u32,
    /// How many children (an array's elements).
    #[serde(default, skip_serializing_if = "is_zero32")]
    pub count: u32,
}

fn is_zero32(n: &u32) -> bool {
    *n == 0
}

/// What the program says.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase", rename_all_fields = "camelCase")]
pub enum EventBody {
    /// The program's end of the session is up (before `start`).
    Ready { protocol: u32, runtime: String, program: String },
    /// Stopped: `reason` is `breakpoint`, `step`, `pause`, `entry` or
    /// `exception` (with the error as `description`).
    Stopped {
        reason: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        file: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        line: Option<u32>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        description: Option<String>,
    },
    /// Running again after a stop.
    Continued,
    /// What the program printed: `stream` is `stdout` or `stderr`.
    Output { stream: String, text: String },
    /// A form was shown.
    FormShown {
        id: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        caption: Option<String>,
    },
    /// A form's picture: the kernel's display list, its accessibility tree
    /// and caret (remote forms, I5: not sent yet).
    FormFrame {
        id: String,
        ops: serde_json::Value,
        #[serde(default)]
        a11y: serde_json::Value,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        caret: Option<serde_json::Value>,
    },
    /// A form was closed (or hidden).
    FormClosed { id: String },
    /// The program ended.
    Exited { code: i32 },
    /// A request was done.
    Ok,
    /// A request failed.
    Error { message: String },
    /// `setBreakpoints`' reply.
    Breakpoints { file: String, breakpoints: Vec<PlacedBreakpoint> },
    /// `stackTrace`'s reply.
    StackTrace { frames: Vec<StackFrame> },
    /// `scopes`' reply.
    Scopes { scopes: Vec<ScopeInfo> },
    /// `variables`' reply.
    Variables { variables: Vec<Variable> },
    /// `evaluate`'s and `setVariable`'s reply.
    Evaluate {
        result: String,
        kind: String,
        #[serde(rename = "ref", default, skip_serializing_if = "is_zero32")]
        reference: u32,
    },
    /// `properties`' reply: the component's type and properties.
    Properties { kind: String, properties: Vec<Variable> },
}

/// A message from the program: a reply (`re`) or an event.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Event {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub re: Option<u64>,
    #[serde(flatten)]
    pub body: EventBody,
}

impl Event {
    /// Something that happened (no request asked).
    pub fn new(body: EventBody) -> Self {
        Event { re: None, body }
    }

    /// The reply to request `seq` (none wanted when it's 0).
    pub fn reply(seq: u64, body: EventBody) -> Self {
        Event { re: (seq != 0).then_some(seq), body }
    }

    pub fn error(seq: u64, message: impl Into<String>) -> Self {
        Event::reply(seq, EventBody::Error { message: message.into() })
    }

    pub fn to_json(&self) -> String {
        serde_json::to_string(self).unwrap_or_default()
    }

    pub fn from_json(text: &str) -> Result<Event, String> {
        serde_json::from_str(text).map_err(|e| format!("bad event: {e}"))
    }
}

/// The OSC sequence that frames an event on a stream it shares with the
/// program's output: `ESC ] 7767 ; <json> BEL`. JSON escapes every control
/// character, so the BEL can't occur inside; a terminal ignores the
/// sequence.
pub const FRAME_START: &str = "\x1b]7767;";
pub const FRAME_END: char = '\x07';

/// `event` framed for a shared stream.
pub fn frame(event: &Event) -> String {
    format!("{FRAME_START}{}{FRAME_END}", event.to_json())
}

/// What a shared stream carries: the program's own text, or an event.
#[derive(Debug, Clone, PartialEq)]
pub enum Incoming {
    Text(String),
    Event(Event),
    /// A framed message that didn't parse.
    Garbled(String),
}

/// Splits a stream of bytes (the program's standard output) into its text
/// and its framed events, chunk by chunk: a frame or a UTF-8 character cut
/// between two chunks waits for the rest.
#[derive(Debug, Default)]
pub struct Deframer {
    pending: Vec<u8>,
}

impl Deframer {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn push(&mut self, bytes: &[u8]) -> Vec<Incoming> {
        self.pending.extend_from_slice(bytes);
        let mut out = Vec::new();
        let start = FRAME_START.as_bytes();
        loop {
            match find(&self.pending, start) {
                Some(at) => {
                    let Some(len) = self.pending[at..].iter().position(|&b| b == FRAME_END as u8) else {
                        // (the frame's end hasn't come yet: the text before it has)
                        self.take_text(at, &mut out);
                        return out;
                    };
                    self.take_text(at, &mut out);
                    let json = String::from_utf8_lossy(&self.pending[start.len()..len]).into_owned();
                    self.pending.drain(..=len);
                    out.push(match Event::from_json(&json) {
                        Ok(e) => Incoming::Event(e),
                        Err(_) => Incoming::Garbled(json),
                    });
                }
                None => {
                    // Keep what could be the start of a frame, or of a
                    // UTF-8 character, for the next chunk.
                    let keep = partial_suffix(&self.pending, start);
                    let mut cut = self.pending.len() - keep;
                    cut = utf8_boundary(&self.pending[..cut]);
                    self.take_text(cut, &mut out);
                    return out;
                }
            }
        }
    }

    /// What's left at the stream's end, as text.
    pub fn finish(&mut self) -> Option<Incoming> {
        if self.pending.is_empty() {
            return None;
        }
        let text = String::from_utf8_lossy(&std::mem::take(&mut self.pending)).into_owned();
        Some(Incoming::Text(text))
    }

    fn take_text(&mut self, upto: usize, out: &mut Vec<Incoming>) {
        if upto == 0 {
            return;
        }
        let text = String::from_utf8_lossy(&self.pending[..upto]).into_owned();
        self.pending.drain(..upto);
        match out.last_mut() {
            Some(Incoming::Text(t)) => t.push_str(&text),
            _ => out.push(Incoming::Text(text)),
        }
    }
}

fn find(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    haystack.windows(needle.len()).position(|w| w == needle)
}

/// How many bytes at the end of `bytes` are the start of `prefix`.
fn partial_suffix(bytes: &[u8], prefix: &[u8]) -> usize {
    (1..prefix.len().min(bytes.len() + 1)).rev().find(|&n| bytes.ends_with(&prefix[..n])).unwrap_or(0)
}

/// The longest prefix of `bytes` that doesn't end inside a UTF-8 character.
fn utf8_boundary(bytes: &[u8]) -> usize {
    let n = bytes.len();
    for back in 1..=3.min(n) {
        let b = bytes[n - back];
        if b & 0xC0 != 0x80 {
            // a lead byte: complete if the character fits
            let need = if b >= 0xF0 { 4 } else if b >= 0xE0 { 3 } else if b >= 0xC0 { 2 } else { 1 };
            return if need > back { n - back } else { n };
        }
    }
    n
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn requests_round_trip_as_json() {
        let r = Request::new(3, Command::SetBreakpoints { file: "util.inc".into(), breakpoints: vec![SourceBreakpoint { line: 12, ..Default::default() }] });
        let json = r.to_json();
        assert_eq!(json, r#"{"seq":3,"type":"setBreakpoints","file":"util.inc","breakpoints":[{"line":12}]}"#);
        assert_eq!(Request::from_json(&json).unwrap(), r);
        // defaults: debug on, no seq
        let r = Request::from_json(r#"{"type":"start"}"#).unwrap();
        assert_eq!(r.seq, 0);
        assert!(matches!(r.command, Command::Start { debug: true, stop_on_entry: false, .. }));
        let r = Request::from_json(r#"{"seq":9,"type":"variables","ref":2}"#).unwrap();
        assert_eq!(r.command, Command::Variables { reference: 2, start: None, count: None });
        assert!(Request::from_json(r#"{"type":"fly"}"#).is_err());
    }

    #[test]
    fn events_round_trip_as_json() {
        let e = Event::new(EventBody::Stopped { reason: "breakpoint".into(), file: Some("a.bas".into()), line: Some(4), description: None });
        assert_eq!(e.to_json(), r#"{"type":"stopped","reason":"breakpoint","file":"a.bas","line":4}"#);
        let e = Event::reply(7, EventBody::Evaluate { result: "42".into(), kind: "Integer".into(), reference: 0 });
        assert_eq!(e.to_json(), r#"{"re":7,"type":"evaluate","result":"42","kind":"Integer"}"#);
        assert_eq!(Event::from_json(&e.to_json()).unwrap(), e);
        assert_eq!(Event::reply(0, EventBody::Ok).re, None);
    }

    #[test]
    fn the_deframer_separates_output_and_events_across_chunks() {
        let ev = Event::new(EventBody::Exited { code: 0 });
        let stream = format!("héllo {}wörld\n{}", frame(&ev), frame(&Event::new(EventBody::Continued)));
        let bytes = stream.as_bytes();
        // every split point gives the same result
        for split in 0..bytes.len() {
            let mut d = Deframer::new();
            let mut got = d.push(&bytes[..split]);
            got.extend(d.push(&bytes[split..]));
            got.extend(d.finish());
            let mut text = String::new();
            let mut events = Vec::new();
            for i in got {
                match i {
                    Incoming::Text(t) => text.push_str(&t),
                    Incoming::Event(e) => events.push(e),
                    Incoming::Garbled(g) => panic!("garbled {g}"),
                }
            }
            assert_eq!(text, "héllo wörld\n", "split {split}");
            assert_eq!(events, vec![ev.clone(), Event::new(EventBody::Continued)], "split {split}");
        }
    }
}
