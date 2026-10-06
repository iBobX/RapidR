//! The Debug Adapter Protocol's wire: `Content-Length` framed JSON, and the
//! messages and arguments `rapidr dap` uses (our own types from the
//! protocol's JSON schema: the `dap` crate is an alpha, docs/ide-plan.md
//! §6.6). Bodies the adapter sends are built as JSON values.

use std::io::{BufRead, Write};

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// A message from the client: only requests come that way.
#[derive(Debug, Clone, Deserialize)]
pub struct Request {
    pub seq: i64,
    #[serde(rename = "type")]
    pub kind: String,
    #[serde(default)]
    pub command: String,
    #[serde(default)]
    pub arguments: Value,
}

/// The adapter's answer to a request.
#[derive(Debug, Clone, Serialize)]
pub struct Response {
    pub seq: i64,
    #[serde(rename = "type")]
    pub kind: &'static str,
    pub request_seq: i64,
    pub success: bool,
    pub command: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub body: Option<Value>,
}

/// Something that happened.
#[derive(Debug, Clone, Serialize)]
pub struct Event {
    pub seq: i64,
    #[serde(rename = "type")]
    pub kind: &'static str,
    pub event: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub body: Option<Value>,
}

/// `launch`'s arguments (the launch.json configuration).
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LaunchArguments {
    #[serde(default)]
    pub program: Option<String>,
    #[serde(default)]
    pub args: Vec<String>,
    #[serde(default)]
    pub cwd: Option<String>,
    #[serde(default)]
    pub stop_on_entry: bool,
    #[serde(default)]
    pub no_debug: bool,
    #[serde(default)]
    pub env: std::collections::BTreeMap<String, Option<String>>,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Source {
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub path: Option<String>,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SourceBreakpoint {
    pub line: u32,
    #[serde(default)]
    pub condition: Option<String>,
    #[serde(default)]
    pub hit_condition: Option<String>,
    #[serde(default)]
    pub log_message: Option<String>,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SetBreakpointsArguments {
    #[serde(default)]
    pub source: Source,
    #[serde(default)]
    pub breakpoints: Option<Vec<SourceBreakpoint>>,
    /// The deprecated form: just lines.
    #[serde(default)]
    pub lines: Option<Vec<u32>>,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SetExceptionBreakpointsArguments {
    #[serde(default)]
    pub filters: Vec<String>,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StackTraceArguments {
    #[serde(default)]
    pub start_frame: Option<usize>,
    #[serde(default)]
    pub levels: Option<usize>,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScopesArguments {
    pub frame_id: i64,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VariablesArguments {
    pub variables_reference: i64,
    #[serde(default)]
    pub start: Option<u32>,
    #[serde(default)]
    pub count: Option<u32>,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EvaluateArguments {
    pub expression: String,
    #[serde(default)]
    pub frame_id: Option<i64>,
    #[serde(default)]
    pub context: Option<String>,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SetVariableArguments {
    pub variables_reference: i64,
    pub name: String,
    pub value: String,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DisconnectArguments {
    #[serde(default)]
    pub terminate_debuggee: Option<bool>,
}

/// Reads one message: `None` at the end of the stream (the client is gone).
/// Headers other than `Content-Length` are skipped, as the protocol says.
pub fn read_message(input: &mut impl BufRead) -> std::io::Result<Option<Value>> {
    let mut length: Option<usize> = None;
    loop {
        let mut line = String::new();
        if input.read_line(&mut line)? == 0 {
            return Ok(None);
        }
        let line = line.trim_end_matches(['\r', '\n']);
        if line.is_empty() {
            if length.is_some() {
                break;
            }
            // (stray blank lines between messages)
            continue;
        }
        if let Some((name, value)) = line.split_once(':') {
            if name.trim().eq_ignore_ascii_case("content-length") {
                length = value.trim().parse().ok();
            }
        }
    }
    let mut body = vec![0u8; length.unwrap_or(0)];
    input.read_exact(&mut body)?;
    serde_json::from_slice(&body).map(Some).map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))
}

/// Writes one message with its `Content-Length` header.
pub fn write_message(out: &mut impl Write, message: &impl Serialize) -> std::io::Result<()> {
    let body = serde_json::to_vec(message).map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
    write!(out, "Content-Length: {}\r\n\r\n", body.len())?;
    out.write_all(&body)?;
    out.flush()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn messages_are_framed_with_their_byte_length() {
        let mut out = Vec::new();
        let event = Event { seq: 1, kind: "event", event: "output".into(), body: Some(serde_json::json!({"output": "héllo\n"})) };
        write_message(&mut out, &event).unwrap();
        let text = String::from_utf8(out.clone()).unwrap();
        let (header, body) = text.split_once("\r\n\r\n").unwrap();
        assert_eq!(header, format!("Content-Length: {}", body.len()));
        // (bytes, not characters: é is two)
        assert_eq!(body.len(), body.chars().count() + 1);
        // two messages back to back, with an extra header, read back
        let mut stream = out.clone();
        stream.extend_from_slice(b"Content-Type: application/vscode-jsonrpc\r\nContent-Length: 2\r\n\r\n{}");
        let mut reader = std::io::BufReader::new(&stream[..]);
        let first = read_message(&mut reader).unwrap().unwrap();
        assert_eq!(first["event"], "output");
        assert_eq!(first["body"]["output"], "héllo\n");
        assert_eq!(read_message(&mut reader).unwrap().unwrap(), serde_json::json!({}));
        assert!(read_message(&mut reader).unwrap().is_none());
    }

    #[test]
    fn requests_parse_with_their_arguments() {
        let r: Request = serde_json::from_str(r#"{"seq":3,"type":"request","command":"setBreakpoints","arguments":{"source":{"path":"/p/util.inc"},"breakpoints":[{"line":4}]}}"#).unwrap();
        let args: SetBreakpointsArguments = serde_json::from_value(r.arguments).unwrap();
        assert_eq!(args.source.path.as_deref(), Some("/p/util.inc"));
        assert_eq!(args.breakpoints.unwrap()[0].line, 4);
        let r: Request = serde_json::from_str(r#"{"seq":1,"type":"request","command":"configurationDone"}"#).unwrap();
        assert!(r.arguments.is_null());
        let l: LaunchArguments = serde_json::from_value(serde_json::json!({"program":"a.bas","stopOnEntry":true,"env":{"A":"1","B":null}})).unwrap();
        assert!(l.stop_on_entry && !l.no_debug);
        assert_eq!(l.env.get("B"), Some(&None));
    }
}
