//! Hex literals past 8 digits, for INT64 and UINT64.
//!
//! RapidQ reads `&H123456789` as its low 32 bits (591751049), and so does
//! RapidR everywhere a classic program could see it: the literal in the
//! AST is that 32-bit number. The parser also notes all 64 bits here, by
//! the literal's place and low bits, so `rapidr_ast::numeric` can give an
//! INT64 / UINT64 the whole number (`DIM m AS UINT64 = &HFFFF0000FFFF0000`).

use std::collections::HashMap;
use std::sync::Mutex;

use rapidr_diagnostics::TextSpan;

type Key = (usize, usize, i64);

static FULL: Mutex<Option<HashMap<Key, u64>>> = Mutex::new(None);

/// Notes a hex literal's 64 bits (`low`: the 32-bit value the AST has).
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

/// The 64 bits of the hex literal at `span` whose AST value is `low`, when
/// it had more than 8 digits.
pub fn full(span: TextSpan, low: i64) -> Option<u64> {
    FULL.lock().ok()?.as_ref()?.get(&(span.start, span.end, low)).copied()
}
