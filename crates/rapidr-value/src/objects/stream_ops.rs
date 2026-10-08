//! A stream's methods that reach the program's memory (`crate::memory`),
//! as RC.EXE does them (tests/conformance/cases/memstream_memcopy.bas,
//! stream_udt_arrays.bas):
//!
//! * `Mem.MemCopyFrom(address, n)` — n bytes from the address written at
//!   Position (as WriteStr writes: a gap past the end zero-filled, nothing
//!   before the start), Position after them; n ≤ 0 copies nothing.
//! * `Mem.MemCopyTo(address, n)` — n bytes from Position copied to the
//!   address, zeros for the bytes outside the data (RapidQ copied whatever
//!   followed its buffer), Position moved by n — even past the end, and
//!   back for a negative n, which copies nothing.
//! * `Stream.SaveUDTArray(t.field)` / `LoadUDTArray(t.field)` — every
//!   element of a TYPE's array field (its declared type, given by
//!   `rapidr_ast::stream_arrays` as a second argument) as RapidQ lays them
//!   out; a field that isn't an array writes and reads nothing; loading
//!   more bytes than the stream has from Position is RapidQ's "Stream read
//!   error", a run-time error.
//!
//! The address is one of the program's own (VARPTR of a variable, an
//! element, a TYPE, a stream's Pointer): `crate::memory` checks it and the
//! byte count against the block it is in, so nothing outside the
//! program's data is read or written (docs/security-audit.md SEC-19) —
//! the same on every runtime, the web included. A pointer a DLL returned
//! is refused by name, as PEEK's is.

use super::with_stream_mut;
use crate::memory::{self, Kind};
use crate::Value;

/// The largest count a copy takes (a stream's own limit).
const MAX_COPY: i64 = 1 << 31;

pub fn call(id: &str, method: &str, args: &[Value]) -> Result<Value, String> {
    let arg = |i: usize| args.get(i).cloned().unwrap_or(Value::Null);
    match method {
        "memcopyfrom" => {
            let n = arg(1).to_i64();
            if n <= 0 {
                return Ok(Value::Null);
            }
            let n = count(n)?;
            let bytes = memory::read(arg(0).to_i64(), n)?;
            with_stream_mut(id, |m| m.write(&bytes));
        }
        "memcopyto" => {
            let n = arg(1).to_i64();
            if n <= 0 {
                with_stream_mut(id, |m| m.pos = m.pos.saturating_add(n));
                return Ok(Value::Null);
            }
            let addr = arg(0).to_i64();
            let n = count(n)?;
            memory::check_room(addr, n)?;
            let bytes = with_stream_mut(id, |m| {
                let mut out = vec![0u8; n];
                for (i, b) in out.iter_mut().enumerate() {
                    if let Some(&v) = usize::try_from(m.pos.saturating_add(i as i64))
                        .ok()
                        .and_then(|p| m.data.get(p))
                    {
                        *b = v;
                    }
                }
                out
            })
            .unwrap_or_default();
            memory::write(addr, &bytes)?;
            with_stream_mut(id, |m| m.pos = m.pos.saturating_add(n as i64));
        }
        "saveudtarray" => {
            let v = arg(0);
            if !matches!(v, Value::Array(_)) {
                return Ok(Value::Null);
            }
            let mut bytes = Vec::new();
            memory::encode(&v, &element_kind(args), &mut bytes);
            with_stream_mut(id, |m| m.write(&bytes));
        }
        _ => {
            let v = arg(0);
            if !matches!(v, Value::Array(_)) {
                return Ok(Value::Null);
            }
            let kind = element_kind(args);
            let n = memory::size_of(&v, &kind);
            let bytes = with_stream_mut(id, |m| {
                let left = usize::try_from(m.pos).ok().map(|p| m.data.len().saturating_sub(p));
                match left {
                    Some(left) if left >= n && m.pos <= m.data.len() as i64 => Ok(m.read(n)),
                    _ => Err(format!(
                        "stream read error: the array takes {n} bytes, the stream has {} from Position {}",
                        left.unwrap_or(0),
                        m.pos
                    )),
                }
            })
            .unwrap_or_else(|| Ok(Vec::new()))?;
            memory::decode(&bytes, &kind, &v);
        }
    }
    Ok(Value::Null)
}

/// A byte count within a stream's limit.
fn count(n: i64) -> Result<usize, String> {
    if n > MAX_COPY {
        return Err(format!("{n} bytes is more than a stream holds"));
    }
    usize::try_from(n).map_err(|_| format!("{n} bytes"))
}

/// The element type the compilers gave (`"LONG"`, `"STRING*3"`, a TYPE).
fn element_kind(args: &[Value]) -> Kind {
    match args.get(1) {
        Some(Value::String(t)) => Kind::of(t),
        _ => Kind::Variant,
    }
}

#[cfg(test)]
mod tests {
    use super::super::{call as obj_call, create, get, set};
    use crate::{memory, v_int, v_str, BasicArray, Value};
    use std::cell::RefCell;
    use std::rc::Rc;

    fn no_props(_: &str, _: &str) -> Value {
        Value::Null
    }

    fn run(id: &str, method: &str, args: &[Value]) -> Result<Value, String> {
        obj_call(id, method, args, &no_props).expect("handled")
    }

    fn mem(id: &str, text: &str) {
        assert!(create(id, "RMEMORYSTREAM"));
        run(id, "writestr", &[v_str(text), v_int(text.len() as i64)]).unwrap();
    }

    fn text(id: &str) -> String {
        get(id, "text").unwrap().to_string_val()
    }

    fn pos(id: &str) -> i64 {
        get(id, "position").unwrap().to_i64()
    }

    #[test]
    fn memcopy_as_rcexe() {
        mem("so_m", "0123456789");
        // a string's mirror (VARPTR(s$)): written at Position, which moves on
        let s = memory::varptr_var("so_s", &v_str("ABCDEFGHIJ"), "STRING")
            .unwrap()
            .to_i64();
        set("so_m", "position", &v_int(3)).unwrap().unwrap();
        run("so_m", "memcopyfrom", &[v_int(s), v_int(4)]).unwrap();
        assert_eq!((text("so_m"), pos("so_m")), ("012ABCD789".to_string(), 7));
        // past the end: the stream grows
        set("so_m", "position", &v_int(8)).unwrap().unwrap();
        run("so_m", "memcopyfrom", &[v_int(s + 2), v_int(5)]).unwrap();
        assert_eq!(
            (text("so_m"), pos("so_m")),
            ("012ABCD7CDEFG".to_string(), 13)
        );
        // nothing for 0 or fewer bytes
        run("so_m", "memcopyfrom", &[v_int(s), v_int(-3)]).unwrap();
        assert_eq!(pos("so_m"), 13);
        // MemCopyTo: zeros past the data, Position past the end; back by -n
        let t = memory::varptr_var("so_t", &v_str("zzzzzzzzzz"), "STRING")
            .unwrap()
            .to_i64();
        set("so_m", "position", &v_int(11)).unwrap().unwrap();
        run("so_m", "memcopyto", &[v_int(t), v_int(4)]).unwrap();
        assert_eq!(memory::read(t, 5).unwrap(), b"FG\0\0z");
        assert_eq!(pos("so_m"), 15);
        run("so_m", "memcopyto", &[v_int(t), v_int(-5)]).unwrap();
        assert_eq!(pos("so_m"), 10);
        // an address that isn't the program's, a count past the block's end:
        // refused, nothing copied, Position left
        assert!(run("so_m", "memcopyfrom", &[v_int(12345), v_int(4)])
            .unwrap_err()
            .contains("isn't memory of this program"));
        assert!(run("so_m", "memcopyto", &[v_int(t), v_int(20)])
            .unwrap_err()
            .contains("run past the end"));
        assert!(run("so_m", "memcopyto", &[v_int(t), v_int(1 << 40)])
            .unwrap_err()
            .contains("more than a stream holds"));
        assert_eq!(pos("so_m"), 10);
        // a stream's own Pointer
        let p = get("so_m", "pointer").unwrap().to_i64();
        mem("so_m2", "");
        run("so_m2", "memcopyfrom", &[v_int(p + 3), v_int(4)]).unwrap();
        assert_eq!(text("so_m2"), "ABCD");
        // SetSize: Size's other name, read as nothing
        set("so_m", "setsize", &v_int(4)).unwrap().unwrap();
        assert_eq!(
            (
                get("so_m", "size").unwrap().to_i64(),
                get("so_m", "setsize").unwrap().to_string_val()
            ),
            (4, String::new())
        );
    }

    #[test]
    fn udt_arrays_as_rcexe() {
        mem("so_u", "");
        let arr = |vals: &[i64]| {
            Value::Array(Rc::new(RefCell::new(BasicArray {
                bounds: vec![(0, vals.len() as i64 - 1)],
                data: vals.iter().map(|&v| v_int(v)).collect(),
            })))
        };
        let a = arr(&[5, 6, -7, 8]);
        run("so_u", "saveudtarray", &[a, v_str("LONG")]).unwrap();
        assert_eq!(pos("so_u"), 16);
        assert_eq!(get("so_u", "size").unwrap().to_i64(), 16);
        // not an array (a field that isn't one): nothing
        run("so_u", "saveudtarray", &[v_int(3), v_str("SHORT")]).unwrap();
        assert_eq!(pos("so_u"), 16);
        // loaded in place
        let b = arr(&[0, 0, 0, 99]);
        set("so_u", "position", &v_int(0)).unwrap().unwrap();
        run("so_u", "loadudtarray", &[b.clone(), v_str("LONG")]).unwrap();
        let Value::Array(b) = b else { unreachable!() };
        assert_eq!(
            b.borrow()
                .data
                .iter()
                .map(Value::to_i64)
                .collect::<Vec<_>>(),
            [5, 6, -7, 8]
        );
        // more than the stream holds: RapidQ's stream read error, nothing read
        set("so_u", "position", &v_int(8)).unwrap().unwrap();
        let c = arr(&[1, 2, 3, 4]);
        assert!(run("so_u", "loadudtarray", &[c, v_str("LONG")])
            .unwrap_err()
            .contains("stream read error"));
        assert_eq!(pos("so_u"), 8);
    }
}
