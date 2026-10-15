//! The one introspection / command interface of RapidR programs
//! (docs/ai-architecture.md §3): what RapidR's AI — the IDE's MCP server,
//! the RAI component in users' programs — and tools use to know what a
//! program has and (in later phases) to change it. The same requests and
//! replies at **design time** (the project's source: the CREATE blocks,
//! through RapidR Studio's designer model) and at **run time** (the running
//! program's components), with the same ids (a component's name, as the
//! program writes it) and the same typed values.
//!
//! * [`protocol`] — the requests, replies, error codes and results, as JSON.
//! * [`describe`] — the registry's view of a type or a builtin, every AI
//!   facet and a JSON Schema per value (the same on every side).
//! * [`value`] — typed values (enums, sets, colours …).
//! * [`Introspect`] — what a side implements; [`handle`] / [`handle_json`]
//!   dispatch a request to it.
//! * `design` (cargo feature `design`) — the design-time side, over
//!   `rapidr-designer`'s documents.
//!
//! **Phase 0** (AI-ARCH, 2026-10-15): the read side — `describe`, `list`,
//! `get` — at design time, behind the `design` feature; nothing in RapidR
//! Studio or the default CLI uses it yet. Read-only by construction: no op
//! that changes anything exists in this version (the protocol refuses them
//! as unknown).

pub mod describe;
pub mod protocol;
pub mod value;

#[cfg(feature = "design")]
pub mod design;

pub use protocol::{Error, ErrorCode, EventEntry, GetResult, ListQuery, ListResult, ObjectInfo, Reply, Request, Source, ValueEntry};

/// A side the interface reaches: the design-time model, or a running
/// program. `describe` needs no side (it is the registry's).
pub trait Introspect {
    /// The model's revision: changes exactly when the model does.
    fn rev(&self) -> String;

    /// The components, in creation order, filtered and paged.
    fn list(&self, query: &ListQuery) -> Result<ListResult, Error>;

    /// One component's values: `props` (any case), or all its design-time
    /// properties and bound events.
    fn get(&self, id: &str, props: Option<&[String]>) -> Result<GetResult, Error>;
}

fn to_value<T: serde::Serialize>(r: Result<T, Error>) -> Reply {
    match r.and_then(|v| serde_json::to_value(v).map_err(|e| Error::new(ErrorCode::BadRequest, e.to_string()))) {
        Ok(v) => Reply::Ok(v),
        Err(e) => Reply::Err(e),
    }
}

/// Answers a request with a side (`None`: only `describe` can be answered).
pub fn handle(side: Option<&dyn Introspect>, request: &Request) -> Reply {
    match request {
        Request::Describe { type_name, builtin, what } => match describe::describe(type_name.as_deref(), builtin.as_deref(), what.as_deref()) {
            Ok(v) => Reply::Ok(v),
            Err(e) => Reply::Err(e),
        },
        Request::List { scope, type_name, limit, cursor } => match side {
            Some(s) => to_value(s.list(&ListQuery { scope: scope.clone(), type_name: type_name.clone(), limit: *limit, cursor: cursor.clone() })),
            None => Reply::Err(no_side()),
        },
        Request::Get { id, props } => match side {
            Some(s) => to_value(s.get(id, props.as_deref())),
            None => Reply::Err(no_side()),
        },
    }
}

fn no_side() -> Error {
    Error::new(ErrorCode::Unsupported, "no program is open: only describe can be answered")
}

/// One JSON request line → one JSON reply line.
pub fn handle_json(side: Option<&dyn Introspect>, line: &str) -> String {
    match Request::from_json(line) {
        Ok(r) => handle(side, &r).to_json(),
        Err(e) => Reply::Err(e).to_json(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn describe_needs_no_side_and_the_rest_do() {
        let r = handle_json(None, r#"{"op":"describe","type":"RButton"}"#);
        assert!(r.starts_with(r#"{"ok":true,"result":{"#), "{r}");
        let r = Reply::from_json(&handle_json(None, r#"{"op":"list"}"#)).unwrap();
        assert_eq!(r.error().unwrap().code, ErrorCode::Unsupported);
        let r = Reply::from_json(&handle_json(None, r#"{"op":"destroy","id":"Form1"}"#)).unwrap();
        assert_eq!(r.error().unwrap().code, ErrorCode::BadRequest);
    }
}
