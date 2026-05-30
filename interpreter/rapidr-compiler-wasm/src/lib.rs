//! Browser-callable wrapper around the full RapidR compile pipeline:
//! source text -> preprocessor -> lexer -> parser -> bcgen -> `.rrbc` bytes.
//!
//! The IDE loads this `.wasm`, compiles the user's program in-browser, and
//! hands the resulting bytes to `rapidrintr.wasm` (rapidr-vm-host-web) for
//! execution.

use rapidr_lexer::Lexer;
use rapidr_parser::parse_tokens;
use rapidr_preprocessor::{preprocess_source, PreprocessOptions};
use wasm_bindgen::prelude::*;

/// Compile a single RapidR source string to `.rrbc` bytecode bytes.
///
/// `project_name` is currently informational only — it is not embedded in
/// the bytecode. Pass the file stem (e.g. `"hello_web"`) so future tooling
/// can use it for diagnostics.
///
/// On error, returns the human-readable message as a JS exception.
#[wasm_bindgen]
pub fn compile(source: &str, _project_name: &str) -> Result<Vec<u8>, JsValue> {
    compile_inner(source).map_err(|e| JsValue::from_str(&e))
}

/// Pure-Rust entry point so the same pipeline can be unit-tested without
/// `wasm-bindgen`. Mirrors `compile()` exactly.
pub fn compile_inner(source: &str) -> Result<Vec<u8>, String> {
    let pre = preprocess_source(source, ".", None, PreprocessOptions::default())
        .map_err(|e| format!("preprocess error: {e}"))?;

    let tokens = Lexer::new(&pre.source, None)
        .tokenize()
        .map_err(|e| format!("lex error: {e}"))?;

    let program = parse_tokens(&tokens);

    let compiled = rapidr_bcgen::compile_program(&program)
        .map_err(|e| format!("bcgen error: {e}"))?;

    Ok(compiled.module.to_bytes())
}

/// Compile-and-decode round trip: useful as a quick wasm smoke test.
/// Returns the number of functions in the resulting module so JS callers
/// can verify the artifact is well-formed without re-parsing it.
#[wasm_bindgen]
pub fn compile_and_count_fns(source: &str) -> Result<u32, JsValue> {
    let bytes = compile(source, "smoke")?;
    let module = rapidr_bytecode::Module::from_bytes(&bytes)
        .map_err(|e| JsValue::from_str(&format!("decode error: {e}")))?;
    Ok(module.functions.len() as u32)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hello_world_round_trips() {
        let src = "PRINT \"hello\"\n";
        let bytes = compile_inner(src).expect("compile");
        let module = rapidr_bytecode::Module::from_bytes(&bytes).expect("decode");
        assert!(!module.functions.is_empty());
    }
}
