//! RapidQ's `RAPIDQ.INC` in an import.
//!
//! RapidR supplies RAPIDQ.INC's constants itself: `$INCLUDE "RAPIDQ.INC"`
//! needs no file (`rapidr_preprocessor::RAPIDQ_INC_CONSTANTS`). They come
//! with the `$INCLUDE` line, as in RapidQ — a program without the line
//! reads `clRed` as an undeclared name (0) — so the importer keeps the line
//! while that is so ([`constants_built_in`]): removing it would change the
//! program.
//!
//! The file itself is RapidQ's, and the copy doesn't carry it — unless the
//! program uses something of it that RapidR's constants don't give the
//! same way (RapidQ's file also has a `QBColor` array, `$DEFINE`s, maybe a
//! user's own additions): then it is copied, and the report says why
//! ([`must_carry`]).

use std::collections::HashMap;

use rapidr_lexer::TokenType;
use rapidr_parser::ToolsParse;

use crate::convert::{is_rapidq_inc_name, Change, Edit, FilePlan, ProgramPlan};

/// Whether every RAPIDQ.INC constant is there without the `$INCLUDE` line
/// (as RapidR's own constants are: `rapidr_ast::rapidr_constant`). Not
/// today: RapidQ programs read those names as undeclared without it.
pub fn constants_built_in() -> bool {
    rapidr_preprocessor::RAPIDQ_INC_CONSTANTS.iter().all(|(n, _)| rapidr_ast::rapidr_constant(n).is_some())
}

/// A number token's value (`123`, `0x800000` — the lexer's `&H800000`).
fn number(lexeme: &str) -> Option<i64> {
    let l = lexeme.trim_end_matches(['&', '%', '!', '#']);
    if let Some(h) = l.strip_prefix("0x") {
        return i64::from_str_radix(h, 16).ok();
    }
    if let Some(o) = l.strip_prefix("0o") {
        return i64::from_str_radix(o, 8).ok();
    }
    if let Some(b) = l.strip_prefix("0b") {
        return i64::from_str_radix(b, 2).ok();
    }
    l.parse().ok()
}

/// Why the program needs RapidQ's RAPIDQ.INC file in its copy, when it
/// reads one and uses something of it RapidR's built-in constants don't
/// give the same way.
pub(crate) fn must_carry(tp: &ToolsParse, _files: &[FilePlan]) -> Option<String> {
    let origins = &tp.preprocessed.origins.files;
    let fid = origins.iter().position(|f| f.path.as_ref().is_some_and(|p| is_rapidq_inc_name(&p.to_string_lossy())))?;
    let builtin: HashMap<String, i64> = rapidr_preprocessor::RAPIDQ_INC_CONSTANTS.iter().map(|(n, v)| (n.to_ascii_uppercase(), *v)).collect();
    let lf = tp.files.get(fid)?;
    let toks: Vec<_> = lf.tokens.iter().filter(|t| !matches!(t.kind, TokenType::Newline | TokenType::Eof)).collect();
    // What the file defines that RapidR's constants don't (or not alike).
    let mut extra: Vec<String> = Vec::new();
    for (i, t) in toks.iter().enumerate() {
        let name = |k: usize| toks.get(k).filter(|n| n.kind == TokenType::Identifier).map(|n| n.lexeme.to_ascii_uppercase());
        match t.kind {
            TokenType::Const => {
                let Some(n) = name(i + 1) else { continue };
                let neg = toks.get(i + 3).is_some_and(|m| m.kind == TokenType::Minus);
                let value = toks.get(i + if neg { 4 } else { 3 }).filter(|v| v.kind == TokenType::Number).and_then(|v| number(&v.lexeme)).map(|v| if neg { -v } else { v });
                let same = match (value, builtin.get(&n)) {
                    (Some(v), Some(b)) => *b == v || *b == v as u32 as i32 as i64,
                    _ => false,
                };
                if !same {
                    extra.push(n);
                }
            }
            TokenType::Dim | TokenType::Type | TokenType::Sub | TokenType::Function => extra.extend(name(i + 1)),
            _ => {}
        }
    }
    for line in lf.text.split('\n') {
        let t = line.trim_start();
        if t.get(..7).is_some_and(|d| d.eq_ignore_ascii_case("$DEFINE")) {
            extra.extend(t[7..].split_whitespace().next().map(str::to_ascii_uppercase));
        }
    }
    extra.sort();
    extra.dedup();
    if extra.is_empty() {
        return None;
    }
    // Used anywhere else in the program?
    let mut used: Vec<String> = Vec::new();
    for (i, f) in tp.files.iter().enumerate() {
        if i == fid {
            continue;
        }
        for t in &f.tokens {
            if t.kind == TokenType::Identifier {
                let u = rapidr_ast::strip_type_suffix(&t.lexeme).to_ascii_uppercase();
                if extra.contains(&u) && !used.contains(&u) {
                    used.push(u);
                }
            }
        }
        for line in f.text.split('\n') {
            let t = line.trim_start();
            let word = t.split_whitespace().next().unwrap_or("");
            if ["$IFDEF", "$IFNDEF", "$UNDEF"].iter().any(|d| d.eq_ignore_ascii_case(word)) {
                if let Some(s) = t.split_whitespace().nth(1).map(str::to_ascii_uppercase) {
                    if extra.contains(&s) && !used.contains(&s) {
                        used.push(s);
                    }
                }
            }
        }
    }
    if used.is_empty() {
        return None;
    }
    Some(format!(
        "the program uses {} of RapidQ's RAPIDQ.INC, which RapidR's built-in RAPIDQ.INC constants don't give the same way: the file is copied",
        used.join(", ")
    ))
}

/// The `$INCLUDE "RAPIDQ.INC"` lines: removed only when RapidR has the
/// constants without them ([`constants_built_in`]); kept otherwise.
pub(crate) fn note_include_lines(plan: &mut ProgramPlan) {
    if !constants_built_in() {
        return;
    }
    for f in plan.files.iter_mut().filter(|f| !f.is_rapidq_inc) {
        let mut start = 0;
        for (i, line) in f.text.split('\n').enumerate() {
            let body = line.trim_end_matches('\r');
            let t = body.trim_start();
            let is_include = t.get(..8).is_some_and(|d| d.eq_ignore_ascii_case("$INCLUDE"))
                && is_rapidq_inc_name(t[8..].trim().trim_matches(['"', '<', '>', '\'']).trim());
            if is_include {
                f.edits.push(Edit { start, end: start + body.len(), text: String::new() });
                f.changes.push(Change { line: i + 1, column: 1, from: body.to_string(), to: String::new() });
            }
            start += line.len() + 1;
        }
        f.edits.sort_by_key(|e| e.start);
        f.changes.sort_by_key(|c| (c.line, c.column));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn numbers() {
        assert_eq!(number("0x800000"), Some(0x80_0000));
        assert_eq!(number("15"), Some(15));
        assert_eq!(number("0xFF&"), Some(255));
    }

    #[test]
    fn rapidq_inc_constants_come_with_the_include_line() {
        // (RapidQ programs without the line read clRed as an undeclared 0)
        assert!(!constants_built_in());
    }
}
