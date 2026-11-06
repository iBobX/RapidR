//! Literals' whole numbers, for INT64, UINT64 and UINT32.
//!
//! RapidQ reads `&H123456789` as its low 32 bits (591751049) and
//! `&HFFFFFFFF` as -1, and a decimal beyond 64 bits as a DOUBLE; so does
//! RapidR everywhere a classic program could see it: the literal in the
//! AST is that number. The parser also notes each `&H` literal's unsigned
//! value (up to 16 digits) and each decimal's above 9223372036854775807,
//! by the literal's place and the AST value's bits, so `rapidr_ast::numeric`
//! can read one whole beside a 64-bit value (`DIM m AS UINT64 =
//! &HFFFF0000FFFF0000`, `u32 = &HF58ACBFB`, `IF u = 18446744073709551615`).
//!
//! (A table beside the AST, not a new literal kind, so nothing that reads
//! literals changes. Keyed by place and value, a stale entry can only
//! answer for a literal at the same place with the same AST value — the
//! text edited at one spot and parsed again; each parse writes its own.)

use std::collections::HashMap;
use std::sync::Mutex;

use rapidr_diagnostics::TextSpan;

type Key = (usize, usize, i64);

static FULL: Mutex<Option<HashMap<Key, u64>>> = Mutex::new(None);

/// Notes a literal's whole number (`low`: the bits of the value the AST
/// has — an integer's, a DOUBLE's `to_bits`).
pub fn record(span: TextSpan, low: i64, full: u64) {
    if let Ok(mut map) = FULL.lock() {
        let map = map.get_or_insert_with(HashMap::new);
        // (bounded: a long-running host parses many programs)
        if map.len() > 100_000 {
            map.clear();
        }
        map.insert((span.start, span.end, low), full);
    }
}

/// The whole number of the literal at `span` whose AST value's bits are
/// `low`: an `&H` literal's (up to 16 digits), a decimal's beyond
/// 9223372036854775807.
pub fn full(span: TextSpan, low: i64) -> Option<u64> {
    FULL.lock().ok()?.as_ref()?.get(&(span.start, span.end, low)).copied()
}
