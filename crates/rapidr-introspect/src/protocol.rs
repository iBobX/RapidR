//! The interface's messages, as JSON (docs/ai-architecture.md §3.2).
//!
//! A request is an object with an `op`; a reply is
//! `{"ok":true,"result":{…}}` or `{"ok":false,"error":{"code":…,"message":…}}`.
//!
//! ```json
//! {"op":"describe","type":"RButton"}
//! {"op":"list","type":"RButton","limit":50}
//! {"op":"get","id":"Button1","props":["Caption","Left"]}
//! ```
//!
//! Phase 0 has the read side: `describe`, `list`, `get`. The other ops of
//! the design (`set`, `call`, `create`, `connect`, `destroy`, `subscribe`,
//! transactions, `preview`, `undo`) are refused as unknown until their
//! phase lands — never half-done.

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// The protocol's version (a reply to `describe` says it).
pub const PROTOCOL_VERSION: u32 = 1;

/// The longest request line accepted (bytes).
pub const MAX_REQUEST: usize = 64 * 1024;

/// What a client asks.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "op", rename_all = "camelCase", rename_all_fields = "camelCase", deny_unknown_fields)]
pub enum Request {
    /// The registry's view of a component type (`type`, RapidR's or
    /// RapidQ's name), of a builtin (`builtin`), or the list of every type
    /// or builtin (`what`: `types`, `builtins`). Exactly one of the three.
    Describe {
        #[serde(default, rename = "type", skip_serializing_if = "Option::is_none")]
        type_name: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        builtin: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        what: Option<String>,
    },
    /// The program's components in creation order (depth first): all, or
    /// those inside `scope` (an id), or those of one `type`; `limit` at a
    /// time (default 200, at most 1,000) from `cursor` (a previous reply's
    /// `next`).
    List {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        scope: Option<String>,
        #[serde(default, rename = "type", skip_serializing_if = "Option::is_none")]
        type_name: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        limit: Option<u32>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        cursor: Option<String>,
    },
    /// One component's values: `props` (properties or events, any case), or
    /// all its design-time properties and its bound events.
    Get {
        id: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        props: Option<Vec<String>>,
    },
}

impl Request {
    pub fn from_json(text: &str) -> Result<Request, Error> {
        if text.len() > MAX_REQUEST {
            return Err(Error::new(ErrorCode::BadRequest, format!("a request is at most {MAX_REQUEST} bytes")));
        }
        serde_json::from_str(text).map_err(|e| Error::new(ErrorCode::BadRequest, format!("bad request: {e} (ops: describe, list, get)")))
    }

    pub fn to_json(&self) -> String {
        serde_json::to_string(self).unwrap_or_default()
    }
}

/// Why a request failed (docs/ai-architecture.md §3.2).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ErrorCode {
    /// Not a request this side understands (malformed, an unknown op or field).
    BadRequest,
    /// No component type or builtin of that name (or one RapidR doesn't have yet).
    UnknownType,
    /// No component, property or event of that name.
    NotFound,
    /// A name two components have: the message lists their qualified ids.
    Ambiguous,
    /// The model changed since the revision or cursor the request carried.
    Stale,
    /// The permission tier or the exposure refuses it.
    Denied,
    /// The user is being asked.
    NeedsConfirmation,
    /// This side can't do it.
    Unsupported,
    /// A value of the wrong type, range or kind.
    InvalidValue,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Error {
    pub code: ErrorCode,
    pub message: String,
}

impl Error {
    pub fn new(code: ErrorCode, message: impl Into<String>) -> Error {
        Error { code, message: message.into() }
    }
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", serde_json::to_value(self.code).ok().and_then(|v| v.as_str().map(str::to_string)).unwrap_or_default(), self.message)
    }
}

/// A reply.
#[derive(Debug, Clone, PartialEq)]
pub enum Reply {
    Ok(Value),
    Err(Error),
}

impl Reply {
    /// One line of JSON, `ok` first.
    pub fn to_json(&self) -> String {
        match self {
            Reply::Ok(result) => format!("{{\"ok\":true,\"result\":{result}}}"),
            Reply::Err(e) => format!("{{\"ok\":false,\"error\":{}}}", serde_json::to_string(e).unwrap_or_default()),
        }
    }

    pub fn from_json(text: &str) -> Result<Reply, String> {
        let v: Value = serde_json::from_str(text).map_err(|e| e.to_string())?;
        match v.get("ok").and_then(Value::as_bool) {
            Some(true) => Ok(Reply::Ok(v.get("result").cloned().unwrap_or(Value::Null))),
            Some(false) => serde_json::from_value(v.get("error").cloned().unwrap_or(Value::Null)).map(Reply::Err).map_err(|e| e.to_string()),
            None => Err("a reply has `ok`".into()),
        }
    }

    pub fn result(&self) -> Option<&Value> {
        match self {
            Reply::Ok(v) => Some(v),
            Reply::Err(_) => None,
        }
    }

    pub fn error(&self) -> Option<&Error> {
        match self {
            Reply::Ok(_) => None,
            Reply::Err(e) => Some(e),
        }
    }
}

/// `list`'s filters and page.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ListQuery {
    pub scope: Option<String>,
    pub type_name: Option<String>,
    pub limit: Option<u32>,
    pub cursor: Option<String>,
}

/// The default and the largest page of `list`.
pub const LIST_LIMIT: u32 = 200;
pub const LIST_MAX: u32 = 1000;

/// One component in `list`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ObjectInfo {
    /// What every other request names it by: its name, qualified only when
    /// another component has the same one (docs/ai-architecture.md §3.3).
    pub id: String,
    /// Its name as the program writes it.
    pub name: String,
    /// Its type under RapidR's name (`RButton`).
    #[serde(rename = "type")]
    pub type_name: String,
    /// Its type as the source writes it (`QBUTTON`), when that differs.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub type_written: Option<String>,
    /// The id of the component it's created in (none for a form).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parent: Option<String>,
    /// The id of its form.
    pub form: String,
    /// The file its CREATE is in (design time).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub file: Option<String>,
    /// The CREATE's line, from 1 (design time).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub line: Option<u32>,
    pub visual: bool,
    pub container: bool,
    /// How many components are created directly inside it.
    pub children: u32,
}

/// `list`'s result.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListResult {
    pub rev: String,
    /// How many match (all pages).
    pub total: u32,
    pub objects: Vec<ObjectInfo>,
    /// The cursor of the next page, when there is one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub next: Option<String>,
}

/// Where a value in `get` comes from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Source {
    /// Written in the CREATE block (design time) / set by the program.
    Set,
    /// Not written: the registry's default applies.
    Default,
    /// Not written, and no default is known.
    Unset,
}

/// One property's value in `get`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ValueEntry {
    /// The registry's spelling (`Caption`), or as written for a part
    /// (`Font.Name`) or a name the registry doesn't know.
    pub name: String,
    /// The value, typed (docs/ai-architecture.md §3.4).
    pub value: Value,
    /// Its source text (design time; never for a secret).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub written: Option<String>,
    pub source: Source,
    /// The assignment's line, from 1 (design time).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub line: Option<u32>,
    /// The registry doesn't know this property of this type.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub unknown: bool,
}

/// An event bound to a handler, in `get`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EventEntry {
    pub name: String,
    /// The SUB it runs (`Button1Click`); none when `get` asked for an
    /// unbound event.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub handler: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub line: Option<u32>,
}

/// `get`'s result.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetResult {
    pub rev: String,
    pub id: String,
    pub name: String,
    #[serde(rename = "type")]
    pub type_name: String,
    pub values: Vec<ValueEntry>,
    pub events: Vec<EventEntry>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn requests_round_trip_and_unknown_things_are_refused() {
        let r = Request::from_json(r#"{"op":"get","id":"Button1","props":["Caption"]}"#).unwrap();
        assert_eq!(r, Request::Get { id: "Button1".into(), props: Some(vec!["Caption".into()]) });
        assert_eq!(Request::from_json(&r.to_json()).unwrap(), r);
        let d = Request::from_json(r#"{"op":"describe","type":"QBUTTON"}"#).unwrap();
        assert!(matches!(d, Request::Describe { type_name: Some(ref t), .. } if t == "QBUTTON"));
        assert_eq!(Request::from_json(r#"{"op":"list"}"#).unwrap(), Request::List { scope: None, type_name: None, limit: None, cursor: None });
        // not phase 0's (or not anything's): refused, never half-done
        for bad in [r#"{"op":"set","id":"Button1","values":{"Caption":"x"}}"#, r#"{"op":"get","id":"B","values":{}}"#, r#"{"id":"B"}"#, "not json"] {
            let e = Request::from_json(bad).unwrap_err();
            assert_eq!(e.code, ErrorCode::BadRequest, "{bad}");
        }
        let huge = format!(r#"{{"op":"get","id":"{}"}}"#, "x".repeat(MAX_REQUEST));
        assert_eq!(Request::from_json(&huge).unwrap_err().code, ErrorCode::BadRequest);
    }

    #[test]
    fn replies_as_json() {
        let ok = Reply::Ok(serde_json::json!({"n": 1}));
        assert_eq!(ok.to_json(), r#"{"ok":true,"result":{"n":1}}"#);
        let err = Reply::Err(Error::new(ErrorCode::NotFound, "no Button9"));
        assert_eq!(err.to_json(), r#"{"ok":false,"error":{"code":"not_found","message":"no Button9"}}"#);
        assert_eq!(Reply::from_json(&err.to_json()).unwrap(), err);
        assert_eq!(Reply::from_json(&ok.to_json()).unwrap(), ok);
        assert_eq!(err.error().unwrap().to_string(), "not_found: no Button9");
    }
}
