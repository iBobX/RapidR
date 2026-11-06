//! The constructs a CPU emulator's core is made of (docs/jit-design.md
//! §12.8): `IIF`, a `DIM` inside one branch of a `SELECT CASE` or an `IF`,
//! a comparison counted in arithmetic (`n * (a < b)`). Compiled in the
//! default mode (hot after a thousand calls), with the interpreter's
//! answers — the arms' side effects, the kinds and the values left behind
//! at every way back to the interpreter included.

use rapidr_jit::Mode;
use rapidr_value::Value;
use rapidr_vm::{Host, StubHost, Vm};

/// The test host, calling the pure maths builtins (and IIF) itself as the
/// desktop's does.
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
    let lines: Vec<&str> = src.lines().collect();
    let report = rapidr_jit::report_with(&vm, &|line| lines.get(line as usize - 1).map(|l| l.to_string())).unwrap_or_default();
    drop(vm);
    let mut out = h.0.output;
    if let Err(e) = r {
        out.push_str(&format!("ERROR: {e}"));
    }
    (out, report)
}

/// The same output with the JIT compiling everything at once (`always`)
/// and in its default mode as without it; the default mode's report.
fn same(src: &str) -> String {
    let (off, _) = run(src, Mode::Off);
    let (always, report) = run(src, Mode::Always);
    assert_eq!(always, off, "always: {report}");
    let (on, report) = run(src, Mode::On);
    assert_eq!(on, off, "on: {report}");
    report
}

/// RapidR8Bit's JIT probe (one SUB per construct its Z80 core uses), a
/// tenth of its turns: every SUB compiled in the default mode — `P2_Iif`
/// and `P3_DimInCase` stayed interpreted before.
const PROBE: &str = "DIM mem(65535) AS INTEGER
DIM regA AS INTEGER, regPC AS INTEGER, cyc AS LONG, flagC AS INTEGER

SUB P1_SelectList(op AS INTEGER)
    SELECT CASE op
        CASE &HC7, &HCF, &HD7: regA = op AND &H38
        CASE ELSE: regA = 1
    END SELECT
END SUB

SUB P2_Iif(v AS INTEGER)
    flagC = IIF(v > 255, 1, 0)
END SUB

SUB P3_DimInCase(op AS INTEGER)
    SELECT CASE op
        CASE 1
            DIM t AS INTEGER
            t = op * 2: regA = t
        CASE ELSE: regA = 0
    END SELECT
END SUB

SUB P4_GlobalArray(a AS INTEGER)
    mem(a) = (mem((a + 1) AND 65535) + 1) AND 255
END SUB

SUB P5_Callee(x AS INTEGER, y AS LONG)
    cyc = cyc + y + x
END SUB
SUB P5_Call(a AS INTEGER)
    P5_Callee a, 4
END SUB

SUB P6_Bits(v AS INTEGER)
    regA = ((v \\ 16) AND 1) OR ((v * 2) AND 254) XOR (v AND 128)
END SUB

SUB P7_LongMix(v AS INTEGER)
    cyc = cyc + v
    regPC = (regPC + 1) AND 65535
END SUB

SUB P8_DimInit(v AS INTEGER)
    DIM k AS INTEGER = v AND 7
    regA = k
END SUB

DIM i AS LONG
FOR i = 1 TO 30000
    P1_SelectList i AND 255
    P2_Iif i AND 511
    P3_DimInCase i AND 1
    P4_GlobalArray i AND 65535
    P5_Call i AND 255
    P6_Bits i AND 255
    P7_LongMix i AND 255
    P8_DimInit i AND 255
NEXT
PRINT \"done \"; regA; \" \"; cyc; \" \"; flagC
";

#[test]
fn the_probe_compiles_every_routine() {
    let r = same(PROBE);
    assert!(!r.contains("stays interpreted"), "{r}");
    for name in ["P2_Iif", "P3_DimInCase"] {
        assert!(!r.contains(name), "{name}: {r}");
    }
    assert!(r.contains("8 routine(s) compiled") || r.contains("9 routine(s) compiled"), "{r}");
}

#[test]
fn both_arms_of_iif_are_worked_out_first() {
    // (IIF is a function: its arms are its arguments, each worked out —
    // Bump's count is two a call, whichever arm is picked)
    let r = same(
        "DIM n AS INTEGER, s AS INTEGER\nFUNCTION Bump(v AS INTEGER) AS INTEGER\n  n = n + 1\n  Result = v\nEND FUNCTION\nSUB Step1(i AS INTEGER)\n  s = s + IIF(i MOD 3 = 0, Bump(10), Bump(1))\nEND SUB\nDIM i AS INTEGER\nFOR i = 1 TO 5000\n  Step1 i\nNEXT\nPRINT n; \" \"; s\n",
    );
    assert!(!r.contains("stays interpreted"), "{r}");
}

#[test]
fn iif_of_every_kind() {
    // (arms of one kind; of two kinds stored into typed variables, each
    // converted as the store converts; into an untyped one, the
    // interpreter's; text arms, the interpreter's; conditions of every kind)
    same(
        "DIM gi AS INTEGER, gd AS DOUBLE, gb AS BYTE, gs AS SHORT, gf AS SINGLE, g64 AS INT64, gu AS UINT64, gw AS WORD
DIM t AS STRING, v
SUB Kinds(i AS INTEGER)
  DIM d AS DOUBLE, u, c AS DOUBLE
  d = i / 7
  c = i MOD 3
  gi = IIF(i AND 1, i * 3, -i)
  gd = IIF(d > 100, d, 0.25)
  gb = IIF(i > 50, 300 + i, -1)
  gs = IIF(c, 40000, 2.5)
  gf = IIF(i < 10, 1, d)
  g64 = IIF(i > 2, 9223372036854775807, i)
  gu = IIF(i > 2, 18446744073709551615, i)
  gw = IIF(c = 1, -1, 65536 + i)
  v = IIF(c = 2, i, 0.5)
  u = IIF(c, u, 1)
  t = IIF(c = 0, \"zero\", \"other\")
END SUB
DIM i AS INTEGER
FOR i = 1 TO 3000
  Kinds i
NEXT
PRINT gi; \" \"; gd; \" \"; gb; \" \"; gs; \" \"; gf; \" \"; g64; \" \"; gu; \" \"; gw; \" \"; v; \" \"; t; \" \"; VARTYPE(v)
",
    );
}

#[test]
fn iif_conditions_of_every_kind() {
    same(
        "DIM r AS INTEGER, z AS DOUBLE, nn
SUB C(i AS INTEGER)
  DIM d AS DOUBLE, b AS INTEGER
  d = (i MOD 5) - 2.5
  b = (i MOD 4 = 0)
  r = r + IIF(d, 1, 2) + IIF(i > 7, 4, 8) + IIF(b, 16, 32) + IIF(nn, 64, 128) + IIF(z / z, 256, 512)
END SUB
DIM i AS INTEGER
FOR i = 1 TO 3000
  C i
NEXT
PRINT r
",
    );
}

#[test]
fn a_dim_in_one_branch() {
    // (SELECT CASE and IF; in a SUB called hot, and in a hot loop of the
    // main program — the local of two kinds where the ways meet, its value
    // as it is wherever compiled code goes back to the interpreter)
    let r = same(
        "DIM regA AS INTEGER, cnt AS LONG
SUB Op(op AS INTEGER)
  SELECT CASE op AND 3
    CASE 1
      DIM t AS INTEGER
      t = op * 2: regA = t
    CASE 2
      DIM w AS DOUBLE
      w = op / 4: regA = INT(w)
    CASE ELSE: regA = 0
  END SELECT
  IF op > 100 THEN
    DIM extra AS LONG
    extra = op - 100
    cnt = cnt + extra
  END IF
END SUB
DIM i AS INTEGER
FOR i = 1 TO 5000
  Op i
  SELECT CASE i MOD 3
    CASE 0
      DIM k AS INTEGER
      k = i \\ 3
      cnt = cnt + k
    CASE ELSE
      cnt = cnt + 1
  END SELECT
NEXT
PRINT regA; \" \"; cnt; \" \"; k
",
    );
    assert!(!r.contains("stays interpreted"), "{r}");
}

#[test]
fn for_loops_in_both_branches_of_an_if() {
    // (an audio engine's voice loop: a FOR in the THEN and another in the
    // ELSE, inside an outer loop — each FOR's hidden end and step locals
    // are Null on the way that didn't run it, an Integer on the other)
    let r = same(
        "DIM buf(255) AS DOUBLE, total AS DOUBLE
SUB Render(n AS INTEGER, mode AS INTEGER)
  DIM v AS INTEGER, i AS INTEGER
  FOR v = 0 TO 7
    IF (v + mode) MOD 2 = 0 THEN
      FOR i = 0 TO n - 1
        buf(i) = buf(i) * 0.5 + v * 0.25
      NEXT
    ELSE
      FOR i = 0 TO n - 1 STEP 2
        buf(i) = buf(i) - 0.125
      NEXT
    END IF
  NEXT
END SUB
DIM blk AS INTEGER, j AS INTEGER
FOR blk = 1 TO 3000
  Render 64, blk AND 1
NEXT
FOR j = 0 TO 63
  total = total + buf(j)
NEXT
PRINT total
",
    );
    assert!(!r.contains("stays interpreted") && r.contains("1 routine(s) compiled"), "{r}");
    // (and in the main program's own loop, entered compiled at its head)
    let r = same(
        "DIM buf(255) AS DOUBLE, total AS DOUBLE, v AS INTEGER, i AS INTEGER, blk AS LONG
FOR blk = 1 TO 4000
  FOR v = 0 TO 3
    IF (v + blk) MOD 3 = 0 THEN
      FOR i = 0 TO 15
        buf(i) = buf(i) * 0.5 + v
      NEXT
    ELSE
      FOR i = 0 TO 15 STEP 3
        buf(i) = buf(i) - 0.5
      NEXT
    END IF
  NEXT
NEXT
FOR i = 0 TO 15
  total = total + buf(i)
NEXT
PRINT total
",
    );
    assert!(!r.contains("stays interpreted"), "{r}");
}

#[test]
fn a_local_of_two_kinds_read_after_the_ways_meet() {
    // (read where it may be either: the interpreter reads it — compiled
    // code goes back there with the very value and kind it holds)
    same(
        "DIM s AS DOUBLE
SUB Mix(i AS INTEGER)
  DIM x
  IF i MOD 2 = 0 THEN
    x = i
  ELSE
    x = i + 0.5
  END IF
  s = s + x
END SUB
DIM i AS INTEGER
FOR i = 1 TO 4000
  Mix i
NEXT
PRINT s
",
    );
}

#[test]
fn a_local_of_two_kinds_left_behind_in_a_loop() {
    // (the loop leaves for the interpreter with t an Integer on some turns
    // and Null on others: the interpreter, after the loop, prints exactly
    // what the last turn left)
    for last in [3000, 3001] {
        same(&format!(
            "DIM i AS INTEGER, s AS LONG
FOR i = 1 TO {last}
  IF i MOD 2 = 0 THEN
    DIM t AS INTEGER
    t = i
    s = s + t
  END IF
NEXT
PRINT s; \" [\"; t; \"] \"; VARTYPE(t)
"
        ));
    }
}

#[test]
fn comparisons_counted_in_arithmetic() {
    // (`n * (a < b)`: a true is -1, as floats — a Double; `-(a < b)`; a
    // Null in arithmetic)
    let r = same(
        "DIM mem(65535) AS INTEGER, regIX AS LONG, ofs AS LONG, rom AS LONG, f AS DOUBLE, nn, z AS DOUBLE
rom = 16384
SUB Fetch(addr AS LONG)
  ofs = -rom * (addr < 16384)
  regIX = mem(addr - rom * (addr < 16384) + ofs) + 1
  f = f + (addr > 30000) + (addr = 7) * 0.5 - (addr < 2) / 4 + nn + nn * 3
  z = -(addr > 5) + -(addr < 5) + -nn
END SUB
DIM i AS LONG
FOR i = 1 TO 40000
  mem(i AND 32767) = i AND 255
  Fetch i AND 32767
NEXT
PRINT regIX; \" \"; ofs; \" \"; f; \" \"; z; \" \"; STR$(-(1 > 2))
",
    );
    assert!(!r.contains("stays interpreted"), "{r}");
}

#[test]
fn a_global_array_read_after_its_index() {
    // (`mem(a + IIF(…))`: the array is read first, its index worked out
    // after — the index's call may go back to the interpreter, which may
    // REDIM the array: the element read is the old array's, as the
    // interpreter reads it)
    let r = same(
        "DIM mem(4095) AS INTEGER, rom AS INTEGER, s AS LONG, said AS INTEGER
FUNCTION Pick(i AS INTEGER) AS INTEGER
  IF i = 2500 THEN
    PRINT \"redim at \"; i
    REDIM mem(10) AS INTEGER
  END IF
  Result = i AND 7
END FUNCTION
SUB Fetch(addr AS INTEGER)
  DIM hi AS INTEGER = mem(((addr + 1) AND 1023) + IIF(((addr + 1) AND 1023) < 512, rom, 0))
  s = s + hi
  mem(addr AND 1023 + IIF(addr > 2000, 0, 8)) = hi + 1
END SUB
SUB Fetch2(addr AS INTEGER)
  s = s + mem(Pick(addr) + IIF(addr > 9, 1, 2))
END SUB
rom = 2048
DIM i AS INTEGER
FOR i = 0 TO 4095
  mem(i) = i AND 255
NEXT
FOR i = 1 TO 3000
  Fetch i
NEXT
FOR i = 1 TO 3000
  Fetch2 i
NEXT
PRINT s; \" \"; UBOUND(mem)
",
    );
    assert!(!r.contains("Fetch (line"), "{r}");
}

#[test]
fn a_routine_with_a_rare_way_back_is_compiled() {
    // (an emulator's step that prints once in a thousand runs: compiled,
    // and leaves on that way only)
    let r = same(
        "DIM s AS LONG, t AS STRING
SUB Step1(i AS INTEGER)
  s = s + (i AND 15)
  IF i MOD 1000 = 0 THEN
    PRINT \"at \"; i
    t = t + \"x\"
  END IF
END SUB
DIM i AS INTEGER
FOR i = 1 TO 5000
  Step1 i
NEXT
PRINT s; \" \"; t
",
    );
    assert!(!r.contains("stays interpreted") && !r.contains("used no more"), "{r}");
    assert!(r.contains("back to the interpreter most at"), "{r}");
}

#[test]
fn a_routine_that_leaves_on_every_run_stays_interpreted() {
    // (its first statement is text: every run would go back at once)
    let r = same("DIM t AS STRING, s AS LONG\nSUB Add1(i AS INTEGER)\n  t = LEFT$(t + \"ab\", 8)\n  s = s + i\nEND SUB\nDIM i AS INTEGER\nFOR i = 1 TO 3000\n  Add1 i\nNEXT\nPRINT s; \" \"; t\n");
    assert!(r.contains("Add1 (line 2) stays interpreted"), "{r}");
}

#[test]
fn a_way_back_taken_on_most_runs_is_given_up() {
    // (half the calls leave at the text, having done little: after 64
    // runs the compiled code is used no more — the same answers)
    let r = same("DIM t AS STRING, s AS LONG\nSUB Add1(i AS INTEGER)\n  s = s + i\n  IF i MOD 3 <> 0 THEN t = LEFT$(t + \"ab\", 8)\nEND SUB\nDIM i AS INTEGER\nFOR i = 1 TO 5000\n  Add1 i\nNEXT\nPRINT s; \" \"; t\n");
    assert!(r.contains("Add1 (line 2): compiled code used no more"), "{r}");
}

#[test]
fn verbose_names_the_line_and_the_statement() {
    // (what stays interpreted, and where compiled code goes back most:
    // file and line, and the statement)
    let src = "DIM s AS STRING\nSUB Talk(i AS INTEGER)\n  s = s + \"x\"\nEND SUB\nDIM i AS INTEGER\nFOR i = 1 TO 3000\n  Talk i\nNEXT\nPRINT LEN(s)\n";
    let (_, report) = run(src, Mode::On);
    assert!(report.contains("Talk (line 2) stays interpreted"), "{report}");
    assert!(report.contains("line 3: s = s + \"x\""), "{report}");
    let (_, report) = run(src, Mode::Always);
    assert!(report.contains("back to the interpreter most at"), "{report}");
    assert!(report.contains("line 3 in Talk"), "{report}");
}

/// `cargo test --release -p rapidr-jit --test iif_and_branches show -- --ignored --nocapture`
#[test]
#[ignore]
fn show_reports() {
    for mode in [Mode::On, Mode::Always] {
        let (out, report) = run(PROBE, mode);
        println!("== probe {mode:?}\n{out}\n{report}\n");
    }
}
