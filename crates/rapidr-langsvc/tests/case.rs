//! Automatic case (`case_edits`): the language's words in the case asked,
//! as the user types and when formatting; never a string, a comment, a
//! directive's argument or the program's own names — and never what a
//! program does (`formatting_keeps_every_program_the_same`).

use std::fs;
use std::path::{Path, PathBuf};

use rapidr_langsvc::{Analysis, CaseOptions, CaseScope, IdentifierCase, KeywordCase, Options, TextEdit};

fn apply(text: &str, edits: &[TextEdit]) -> String {
    let mut out = text.to_string();
    let mut sorted = edits.to_vec();
    sorted.sort_by_key(|e| e.start);
    for w in sorted.windows(2) {
        assert!(w[0].end <= w[1].start, "overlapping edits {:?} {:?}", w[0], w[1]);
    }
    for e in sorted.iter().rev() {
        out.replace_range(e.start..e.end, &e.text);
    }
    out
}

fn analysis(keywords: KeywordCase, identifiers: IdentifierCase) -> Analysis {
    Analysis::new(Options { case: CaseOptions { keywords, identifiers }, ..Default::default() })
}

fn file() -> PathBuf {
    std::env::temp_dir().join(format!("rapidr-case-{}", std::process::id())).join("main.bas")
}

/// `text` with every word cased (formatting's case pass alone).
fn cased(text: &str, keywords: KeywordCase, identifiers: IdentifierCase) -> String {
    let mut a = analysis(keywords, identifiers);
    let f = file();
    a.update(f.clone(), text.to_string());
    apply(text, &a.case_edits(&f, CaseScope::Range { start: 0, end: text.len() }))
}

/// Types `typed` at the end of `before`, one character at a time, as an
/// editor with format-on-type does: after each trigger, the edits the
/// service answers are applied.
fn type_text(a: &mut Analysis, before: &str, typed: &str) -> String {
    let f = file();
    let mut text = before.to_string();
    for ch in typed.chars() {
        text.push(ch);
        a.update(f.clone(), text.clone());
        if rapidr_langsvc::case::TRIGGERS.contains(&ch) {
            let edits = a.case_edits(&f, CaseScope::Typed { offset: text.len(), ch });
            text = apply(&text, &edits);
        }
    }
    text
}

#[test]
fn typing_a_declaration() {
    let mut a = analysis(KeywordCase::Upper, IdentifierCase::Preserve);
    assert_eq!(type_text(&mut a, "", "dim x as integer\n"), "DIM x AS INTEGER\n");
    // mid-line, after a colon
    let mut a = analysis(KeywordCase::Upper, IdentifierCase::Preserve);
    assert_eq!(type_text(&mut a, "x = 1: ", "dim y as string\nprint mid$(\"dim as\", 1, 2) ' dim as\n"), "x = 1: DIM y AS STRING\nPRINT MID$(\"dim as\", 1, 2) ' dim as\n");
}

#[test]
fn only_the_word_just_finished() {
    let text = "dim a as integer\ndim ";
    let mut a = analysis(KeywordCase::Upper, IdentifierCase::Preserve);
    let f = file();
    a.update(f.clone(), text.to_string());
    let edits = a.case_edits(&f, CaseScope::Typed { offset: text.len(), ch: ' ' });
    assert_eq!(apply(text, &edits), "dim a as integer\nDIM ");
}

#[test]
fn words_inside_longer_names_and_blocks() {
    let src = "dim Dimension as integer, EndPoint as long\nif Dimension > 0 then\nEndPoint = 1\nelseif EndPoint then\nend if\nsub Go(byref n as double)\nend sub\n";
    let want = "DIM Dimension AS INTEGER, EndPoint AS LONG\nIF Dimension > 0 THEN\nEndPoint = 1\nELSEIF EndPoint THEN\nEND IF\nSUB Go(BYREF n AS DOUBLE)\nEND SUB\n";
    assert_eq!(cased(src, KeywordCase::Upper, IdentifierCase::Preserve), want);
    let proper = cased(src, KeywordCase::Proper, IdentifierCase::Preserve);
    assert!(proper.starts_with("Dim Dimension As Integer, EndPoint As Long\nIf Dimension > 0 Then\n"), "{proper}");
    assert!(proper.contains("End If\nSub Go(Byref n As Double)\nEnd Sub"), "{proper}");
    let lower = cased(want, KeywordCase::Lower, IdentifierCase::Preserve);
    assert_eq!(lower, "dim Dimension as integer, EndPoint as long\nif Dimension > 0 then\nEndPoint = 1\nelseif EndPoint then\nend if\nsub Go(byref n as double)\nend sub\n");
    assert_eq!(cased(src, KeywordCase::Preserve, IdentifierCase::Preserve), src);
}

#[test]
fn strings_comments_and_directive_arguments_stay() {
    let src = "$include \"Lib/Dim As.inc\"\n$typecheck on\nrem dim as integer\nprint \"dim ünïcødé 😀 as\" ' print\n? \"x\"\n";
    let got = cased(src, KeywordCase::Upper, IdentifierCase::Preserve);
    assert_eq!(got, "$INCLUDE \"Lib/Dim As.inc\"\n$TYPECHECK on\nrem dim as integer\nPRINT \"dim ünïcødé 😀 as\" ' print\n? \"x\"\n");
}

#[test]
fn crlf_files() {
    let src = "dim n as integer\r\nfor n = 1 to 3\r\nprint n\r\nnext\r\n";
    assert_eq!(cased(src, KeywordCase::Upper, IdentifierCase::Preserve), "DIM n AS INTEGER\r\nFOR n = 1 TO 3\r\nPRINT n\r\nNEXT\r\n");
    let mut a = analysis(KeywordCase::Upper, IdentifierCase::Preserve);
    assert_eq!(type_text(&mut a, "x = 1\r\n", "print x\r\n"), "x = 1\r\nPRINT x\r\n");
}

#[test]
fn the_programs_own_names_stay() {
    // (a variable named as a builtin, a property set in a CREATE body, members)
    let src = "dim Left as integer\nLeft = len(\"ab\")\ncreate form as qform\nCaption = \"x\"\nend create\nForm.Caption = \"y\"\nform.show\n";
    let got = cased(src, KeywordCase::Upper, IdentifierCase::Preserve);
    assert_eq!(got, "DIM Left AS INTEGER\nLeft = LEN(\"ab\")\nCREATE form AS qform\nCaption = \"x\"\nEND CREATE\nForm.Caption = \"y\"\nform.show\n");
}

#[test]
fn names_as_declared() {
    let src = "DIM TotalCount AS INTEGER\nCONST MaxItems = 3\nSUB AddOne\n  totalcount = TOTALCOUNT + maxitems\nEND SUB\naddone\nCREATE Form AS QFORM\nEND CREATE\nform.caption = \"x\"\nFORM.showmodal\n";
    let got = cased(src, KeywordCase::Upper, IdentifierCase::Declaration);
    assert_eq!(got, "DIM TotalCount AS INTEGER\nCONST MaxItems = 3\nSUB AddOne\n  TotalCount = TotalCount + MaxItems\nEND SUB\nAddOne\nCREATE Form AS QFORM\nEND CREATE\nForm.Caption = \"x\"\nForm.ShowModal\n");
    // (off: as typed)
    assert_eq!(cased(src, KeywordCase::Upper, IdentifierCase::Preserve), src);
}

#[test]
fn formatting_indents_and_cases_together() {
    let src = "sub Go\nprint 1   \nend sub\n";
    let mut a = analysis(KeywordCase::Upper, IdentifierCase::Preserve);
    let f = file();
    a.update(f.clone(), src.to_string());
    assert_eq!(apply(src, &a.format(&f, "    ")), "SUB Go\n    PRINT 1\nEND SUB\n");
    // a range: only its lines
    let edits = a.format_range(&f, 7, 12, "    ");
    assert_eq!(apply(src, &edits), "sub Go\n    PRINT 1\nend sub\n");
}

// ---------------------------------------------------------------------------

/// The programs of the conformance suite and the examples.
fn corpus() -> Vec<PathBuf> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let mut out = Vec::new();
    fn walk(dir: &Path, out: &mut Vec<PathBuf>) {
        let Ok(rd) = fs::read_dir(dir) else { return };
        for e in rd.flatten() {
            let p = e.path();
            if p.is_dir() {
                walk(&p, out);
            } else if p.extension().is_some_and(|x| x.eq_ignore_ascii_case("bas") || x.eq_ignore_ascii_case("rr")) {
                out.push(p);
            }
        }
    }
    walk(&root.join("tests/conformance/cases"), &mut out);
    walk(&root.join("examples"), &mut out);
    out.sort();
    out
}

/// The program compiled as `rapidr build-bc` compiles it: its bytecode, or
/// its errors.
fn compile(path: &Path, text: &str) -> Result<Vec<u8>, Vec<String>> {
    let base = path.parent().unwrap();
    let tools = rapidr_parser::tools::parse_source_for_tools(text, base, Some(path.to_path_buf()), Default::default());
    if !tools.diagnostics.is_empty() {
        return Err(tools.diagnostics.iter().map(|d| d.message.to_ascii_lowercase()).collect());
    }
    let library_lines: Vec<bool> = tools.preprocessed.line_map.iter().map(|(f, _)| f.as_deref().is_some_and(|f| f != path)).collect();
    match rapidr_bcgen::compile_program_diagnostics(&tools.program, Some(&tools.preprocessed.source), &library_lines) {
        Ok(c) => Ok(canonical(&c.module)),
        Err(e) => Err(e.iter().map(|d| d.message.to_ascii_lowercase()).collect()),
    }
}

/// A module's bytecode with every name it refers to written out in lower
/// case: BASIC's names ignore case, and the compiler keeps the spelling it
/// met first in its name pool (for the debugger), so the same program
/// written in another case has the same code but another pool. Everything
/// else — constants (the string literals among them), code, jumps, lines,
/// resources — is compared byte for byte.
fn canonical(m: &rapidr_bytecode::Module) -> Vec<u8> {
    use rapidr_bytecode::Op;
    let name = |id: u32| m.strings.get(id as usize).map_or(format!("<{id}>"), |s| s.to_ascii_lowercase());
    // (a component in an expression is its name as written, a string
    // constant — `x = Timer`; the source's string literals are compared
    // byte for byte by `strings_and_comments`)
    let constant = |id: u32| match m.consts.get(id as usize) {
        Some(rapidr_bytecode::Const::Str(s)) => format!("Str({:?})", s.to_ascii_lowercase()),
        Some(c) => format!("{c:?}"),
        None => format!("<{id}>"),
    };
    let mut out = format!("{}\n{:?}\n{:?}\n", m.entry, m.app_type, m.resources).into_bytes();
    for f in &m.functions {
        out.extend(format!("fn {} {:?} {} {:?} {:?}\n", f.name.to_ascii_lowercase(), f.params.iter().map(|p| p.by_ref).collect::<Vec<_>>(), f.n_locals, f.line_info, f.local_names.iter().map(|n| n.to_ascii_lowercase()).collect::<Vec<_>>()).bytes());
        let code = &f.code;
        let mut ip = 0;
        while ip < code.len() {
            let op = Op::from_u8(code[ip]).unwrap_or_else(|| panic!("bad op {:#x} in {}", code[ip], f.name));
            out.push(code[ip]);
            ip += 1;
            // (each operand: S a u32 name, 4 / 2 / 1 bytes otherwise)
            if op == Op::LoadConst {
                let id = u32::from_le_bytes(code[ip..ip + 4].try_into().unwrap());
                out.extend(format!("<{}>", constant(id)).bytes());
                ip += 4;
                continue;
            }
            let layout: &[u8] = match op {
                Op::Jump | Op::JumpIf | Op::JumpIfNot | Op::Gosub => &[4],
                Op::LoadLocal | Op::StoreLocal | Op::GetField | Op::SetField => &[2],
                Op::LoadGlobal | Op::StoreGlobal | Op::GetPropDyn | Op::SetPropDyn => b"S",
                Op::CallSub | Op::CallFunc => &[4, 1],
                Op::CallBuiltin | Op::CallMethodDyn => &[b'S', 1],
                Op::LoadArgOut | Op::CallIndirect | Op::ToNum | Op::NewArray | Op::AGet | Op::ASet => &[1],
                Op::CreateComp | Op::SetProp | Op::GetProp => b"SS",
                Op::CallMethod => &[b'S', b'S', 1],
                Op::RegisterEvent => &[b'S', b'S', 4],
                Op::Redim => &[2, 4],
                _ => &[],
            };
            for &w in layout {
                if w == b'S' {
                    let id = u32::from_le_bytes(code[ip..ip + 4].try_into().unwrap());
                    out.extend(format!("<{}>", name(id)).bytes());
                    ip += 4;
                } else {
                    out.extend_from_slice(&code[ip..ip + w as usize]);
                    ip += w as usize;
                }
            }
        }
        out.push(b'\n');
    }
    out
}

/// Every string literal and comment of a text, in order.
fn strings_and_comments(text: &str) -> Vec<String> {
    let lf = rapidr_lexer::LosslessFile::lex(text, None);
    let mut out: Vec<String> = lf.tokens.iter().filter(|t| t.kind == rapidr_lexer::TokenType::StringLit).map(|t| text[t.span.start..t.span.end].to_string()).collect();
    out.extend(lf.trivia.iter().filter(|t| t.kind == rapidr_lexer::TriviaKind::Comment).map(|t| text[t.span.start..t.span.end].to_string()));
    out
}

/// BASIC ignores case: formatting (indentation and every case option) leaves
/// every program's bytecode as it was, its strings and comments byte for
/// byte.
#[test]
fn formatting_keeps_every_program_the_same() {
    let files = corpus();
    assert!(files.len() > 150, "{} programs", files.len());
    let mut failures = Vec::new();
    let mut changed = 0;
    for path in &files {
        let text = rapidr_preprocessor::read_source(path).unwrap();
        let before = compile(path, &text);
        for (keywords, identifiers) in [
            (KeywordCase::Upper, IdentifierCase::Declaration),
            (KeywordCase::Lower, IdentifierCase::Preserve),
            (KeywordCase::Proper, IdentifierCase::Declaration),
        ] {
            let mut a = analysis(keywords, identifiers);
            a.update(path.clone(), text.clone());
            let formatted = apply(&text, &a.format(path, "    "));
            if formatted != text {
                changed += 1;
            }
            let after = compile(path, &formatted);
            let changes: Vec<String> = text.lines().zip(formatted.lines()).filter(|(a, b)| a.trim() != b.trim()).take(4).map(|(a, b)| format!("\n      {a:?}\n   -> {b:?}")).collect();
            let name = format!("{} ({keywords:?}, {identifiers:?}){}", path.display(), changes.concat());
            match (&before, &after) {
                (Ok(x), Ok(y)) if x != y => {
                    // (CASE_DIFF=dir: both canonical forms, to compare)
                    if let Ok(dir) = std::env::var("CASE_DIFF") {
                        let stem = path.file_stem().unwrap().to_string_lossy();
                        let _ = fs::write(Path::new(&dir).join(format!("{stem}.before")), x);
                        let _ = fs::write(Path::new(&dir).join(format!("{stem}.{keywords:?}{identifiers:?}.after")), y);
                    }
                    failures.push(format!("{name}: the bytecode changed"))
                }
                (Ok(_), Err(e)) => failures.push(format!("{name}: no longer compiles: {e:?}")),
                (Err(_), Ok(_)) => failures.push(format!("{name}: compiles now")),
                _ => {}
            }
            if strings_and_comments(&text) != strings_and_comments(&formatted) {
                failures.push(format!("{name}: a string or a comment changed"));
            }
        }
    }
    assert!(changed > 0, "nothing was formatted");
    assert!(failures.is_empty(), "{} failure(s):\n{}", failures.len(), failures.join("\n"));
}
