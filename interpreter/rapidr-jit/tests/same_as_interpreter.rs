//! Programs run with the JIT (`always`: everything it can, at once) and
//! without: the same output, the same run-time error at the same line.

use rapidr_jit::Mode;
use rapidr_value::Value;
use rapidr_vm::{Host, StubHost, Vm};

/// The test host, calling the pure maths builtins itself as the desktop's
/// does.
#[derive(Default)]
struct MathHost(StubHost);

impl Host for MathHost {
    const DIRECT_MATHS: bool = true;
    fn call_builtin(&mut self, name: &str, args: &[Value]) -> Result<Value, String> {
        match rapidr_vm::direct_builtin(name) {
            Some(f) if args.len() == 1 => Ok(f(&args[0])),
            _ => self.0.call_builtin(name, args),
        }
    }
    fn create_comp(&mut self, k: &str, id: &str) -> Result<Value, String> {
        self.0.create_comp(k, id)
    }
    fn set_prop(&mut self, id: &str, n: &str, v: Value) -> Result<(), String> {
        self.0.set_prop(id, n, v)
    }
    fn get_prop(&mut self, id: &str, n: &str) -> Result<Value, String> {
        self.0.get_prop(id, n)
    }
    fn call_method(&mut self, id: &str, m: &str, a: &[Value]) -> Result<Value, String> {
        self.0.call_method(id, m, a)
    }
    fn register_event(&mut self, id: &str, e: &str, f: u32) -> Result<(), String> {
        self.0.register_event(id, e, f)
    }
    fn print(&mut self, s: &str) -> Result<(), String> {
        self.0.print(s)
    }
    fn input(&mut self) -> Result<String, String> {
        self.0.input()
    }
}

fn compile(src: &str) -> rapidr_bytecode::Module {
    let toks = rapidr_lexer::Lexer::new(src, Some("test".into())).tokenize().expect("lex");
    let prog = rapidr_parser::parse_tokens(&toks).expect("parse");
    rapidr_bcgen::compile_program_with_source(&prog, Some(src)).expect("compile").module
}

/// The output (and the error, if one) of `src` run in `mode`, and the
/// JIT's report.
fn run(src: &str, mode: Mode) -> (String, String) {
    let m = compile(src);
    let mut h = MathHost::default();
    let mut vm = Vm::new(&mut h);
    let _ = rapidr_jit::install_mode(&mut vm, mode);
    let r = vm.run(&m);
    let report = rapidr_jit::report(&vm).unwrap_or_default();
    drop(vm);
    let mut out = h.0.output;
    if let Err(e) = r {
        out.push_str(&format!("ERROR: {e}"));
    }
    (out, report)
}

/// Both ways the same; returns the JIT's report.
fn same(src: &str) -> String {
    let (off, _) = run(src, Mode::Off);
    let (on, report) = run(src, Mode::Always);
    assert_eq!(on, off, "with the JIT ({report})");
    report
}

#[test]
fn a_global_loop() {
    let r = same("DIM i AS INTEGER\nDIM s AS INTEGER\ns = 0\nFOR i = 1 TO 100000\n  s = s + i\nNEXT\nPRINT s\n");
    assert!(r.contains("1 loop(s)"), "{r}");
}

#[test]
fn a_loop_in_a_sub() {
    // (`always`: the SUB compiled from its start, PRINT left to the
    // interpreter)
    let r = same("SUB Count\n  DIM i AS INTEGER\n  DIM s AS INTEGER\n  s = 0\n  FOR i = 1 TO 100000\n    s = s + i\n  NEXT\n  PRINT s\nEND SUB\nCount\n");
    assert!(r.contains("1 routine(s)") && r.contains("ran 1"), "{r}");
}

#[test]
fn doubles_at_the_top() {
    same("DIM i AS INTEGER\nDIM x AS DOUBLE, y AS DOUBLE\nx = 0.0\ny = 1.0\nFOR i = 1 TO 5000\n  x = x * 0.999 + y\n  y = y + 0.000001\nNEXT\nPRINT INT(x)\nPRINT x\n");
}

#[test]
fn calls_with_doubles() {
    let r = same(
        "FUNCTION Clamp(x AS DOUBLE, lo AS DOUBLE, hi AS DOUBLE) AS DOUBLE\n  IF x < lo THEN\n    Result = lo\n  ELSEIF x > hi THEN\n    Result = hi\n  ELSE\n    Result = x\n  END IF\nEND FUNCTION\nFUNCTION MyRun AS DOUBLE\n  DIM i AS INTEGER, s AS DOUBLE\n  s = 0.0\n  FOR i = 1 TO 20000\n    s = s + Clamp((i MOD 300) - 100.0, 0.0, 150.0)\n  NEXT\n  Result = s\nEND FUNCTION\nPRINT MyRun\n",
    );
    assert!(r.contains("called from compiled code"), "{r}");
}

#[test]
fn recursion() {
    same("FUNCTION Fib(n AS INTEGER) AS INTEGER\n  IF n < 2 THEN\n    Result = n\n  ELSE\n    Result = Fib(n - 1) + Fib(n - 2)\n  END IF\nEND FUNCTION\nPRINT Fib(20)\n");
}

#[test]
fn dsp_scalars() {
    same(
        "CONST VOICES = 4\nCONST BLOCK = 41\nCONST BLOCKS = 10\nFUNCTION MyRun AS DOUBLE\n  DIM blk AS INTEGER, i AS INTEGER, v AS INTEGER\n  DIM ph AS DOUBLE, inc AS DOUBLE, osc AS DOUBLE, env AS DOUBLE, lp AS DOUBLE, bp AS DOUBLE, hp AS DOUBLE\n  DIM total AS DOUBLE\n  total = 0.0\n  FOR v = 0 TO VOICES - 1\n    ph = (v * 0.17) - INT(v * 0.17)\n    inc = (110.0 * (2.0 ^ ((v MOD 24) / 12.0))) / 44100.0\n    env = 0.01\n    lp = 0.0\n    bp = 0.0\n    FOR blk = 1 TO BLOCKS\n      FOR i = 0 TO BLOCK - 1\n        ph = ph + inc\n        IF ph >= 1.0 THEN ph = ph - 1.0\n        osc = 2.0 * ph - 1.0\n        env = env + 0.0005 * (0.80 - env)\n        lp = lp + 0.28 * bp\n        hp = (osc * env * 0.15) - lp - 0.55 * bp\n        bp = 0.28 * hp + bp\n      NEXT\n    NEXT\n    total = total + ABS(lp)\n  NEXT\n  Result = total\nEND FUNCTION\nPRINT INT(MyRun * 1000000)\nPRINT MyRun\n",
    );
}

#[test]
fn global_arrays() {
    same("DIM a(99) AS DOUBLE\nDIM i AS INTEGER\nDIM s AS DOUBLE\nFOR i = 0 TO 99\n  a(i) = i * 0.5\nNEXT\nFOR i = 0 TO 99\n  s = s + a(i)\nNEXT\nPRINT s\n");
}

#[test]
fn a_run_time_error_inside_a_hot_loop_is_the_interpreters() {
    let src = "DIM i AS INTEGER\nDIM s AS INTEGER\nDIM d AS INTEGER\nd = 5\nFOR i = 1 TO 100\n  d = d - 1\n  s = s + 100 MOD d\nNEXT\nPRINT s\n";
    let (off, _) = run(src, Mode::Off);
    assert!(off.contains("Division by zero") && off.contains("line 7"), "{off}");
    same(src);
}

#[test]
fn an_index_out_of_range_in_a_hot_loop() {
    same("DIM a(9) AS INTEGER\nDIM i AS INTEGER\nFOR i = 0 TO 20\n  a(i) = i\nNEXT\nPRINT a(9)\n");
}

// ---- going back to the interpreter at each kind of place ----

#[test]
fn deep_recursion_goes_on_in_the_interpreter_beyond_the_stacks_room() {
    // (60,000 deep: more compiled frames than a test thread's stack has
    // room for — compiled calls as deep as it has, the interpreter makes
    // the next call each time, every frame put back on the way; SEC-50)
    let r = same("FUNCTION Depth(n AS INTEGER) AS INTEGER\n  IF n = 0 THEN\n    Result = 0\n  ELSE\n    Result = Depth(n - 1) + 1\n  END IF\nEND FUNCTION\nPRINT Depth(60000)\n");
    assert!(r.contains("at the machine stack's room") || r.contains("deep calls"), "{r}");
}

#[test]
fn runaway_recursion_is_the_interpreters_stack_overflow() {
    let src = "FUNCTION Down(n AS INTEGER) AS INTEGER\n  Result = Down(n + 1)\nEND FUNCTION\nPRINT Down(1)\n";
    let (off, _) = run(src, Mode::Off);
    assert!(off.contains("stack overflow"), "{off}");
    same(src);
}

#[test]
fn a_variable_that_changes_kind_in_a_hot_loop() {
    // (x untyped: an Integer, then a Double from the 50,000th turn on)
    same("DIM i AS INTEGER\nx = 0\nFOR i = 1 TO 100000\n  IF i = 50000 THEN x = x + 0.5\n  x = x + 1\nNEXT\nPRINT x\n");
}

#[test]
fn an_array_element_of_another_kind() {
    same("DIM a(9)\nDIM i AS INTEGER\nDIM s AS DOUBLE\nFOR i = 0 TO 9\n  a(i) = i\nNEXT\na(3) = \"7\"\nFOR i = 1 TO 30000\n  s = s + a(i MOD 5)\nNEXT\nPRINT s\n");
}

#[test]
fn byref_write_back_in_a_hot_loop() {
    same("SUB Bump(BYREF n AS INTEGER)\n  n = n + 1\nEND SUB\nDIM i AS INTEGER, c AS INTEGER\nFOR i = 1 TO 20000\n  Bump c\nNEXT\nPRINT c\n");
}

#[test]
fn gosub_and_text_in_a_hot_loop() {
    same("DIM i AS INTEGER, t AS STRING\nFOR i = 1 TO 3000\n  GOSUB addone\nNEXT\nPRINT LEN(t)\nEND\naddone:\n  t = t + \"ab\"\n  RETURN\n");
}

#[test]
fn a_call_to_a_routine_whose_result_is_never_worked_out() {
    // (Y returns only after a PRINT — left to the interpreter — or through
    // itself: its result's kind is never known, so X, which calls it,
    // can't be either; X's caller must not call X compiled. Found by
    // RapidR Studio run with RAPIDR_JIT=always: "can't resolve symbol")
    same("FUNCTION Y(n AS INTEGER) AS INTEGER\n  IF n = 0 THEN\n    PRINT \"bottom\"\n    Result = 0\n  ELSE\n    Result = Y(n - 1) + 1\n  END IF\nEND FUNCTION\nFUNCTION X(n AS INTEGER) AS INTEGER\n  IF n < 0 THEN\n    Result = 0\n  ELSE\n    Result = Y(n)\n  END IF\nEND FUNCTION\nDIM i AS INTEGER, s AS INTEGER\nFOR i = 1 TO 50\n  s = s + X(3)\nNEXT\nPRINT s\n");
}

#[test]
fn a_compiled_call_into_a_routine_that_prints() {
    // (always: the callee compiled though it PRINTs — it leaves at its
    // PRINT, its caller at the call, both frames put back)
    same("SUB Say(n AS INTEGER)\n  IF n MOD 1000 = 0 THEN PRINT \"at \"; n\nEND SUB\nDIM i AS INTEGER\nFOR i = 1 TO 5000\n  Say i\nNEXT\n");
}

/// `cargo test -p rapidr-jit --test same_as_interpreter show -- --ignored --nocapture`
#[test]
#[ignore]
fn show_reports() {
    for src in [
        "DIM i AS INTEGER\nDIM s AS INTEGER\ns = 0\nFOR i = 1 TO 100000\n  s = s + i\nNEXT\nPRINT s\n",
        "FUNCTION Fib(n AS INTEGER) AS INTEGER\n  IF n < 2 THEN\n    Result = n\n  ELSE\n    Result = Fib(n - 1) + Fib(n - 2)\n  END IF\nEND FUNCTION\nPRINT Fib(20)\n",
        "DIM a(9) AS INTEGER\nDIM i AS INTEGER\nFOR i = 0 TO 20\n  a(i) = i\nNEXT\nPRINT a(9)\n",
    ] {
        let (out, report) = run(src, Mode::Always);
        println!("{out}\n{report}\n---");
    }
}
