//! The language definitions: every built-in loads, and each colours what it
//! should, across lines and inside embedded languages.

use std::sync::Arc;

use rapidr_editor::{Language, Languages, TokenKind};

fn lang(id: &str) -> Arc<Language> {
    Languages::builtin().get(id).unwrap_or_else(|| panic!("{id} is built in"))
}

#[test]
fn basics_old_id_is_an_alias() {
    // (`rapidq-basic`, the id before it was RapidR's: saved settings and
    // programs setting RCODEEDITOR's Language keep working)
    assert_eq!(lang("rapidq-basic").id, "rapidr-basic");
    assert_eq!(lang("RapidR-Basic").name, "RapidR BASIC");
}

/// Tokenizes `src` line by line from the root; each line's (text, kind)
/// pairs, kinds by name.
fn colour(lang: &Language, src: &str) -> Vec<Vec<(String, String)>> {
    let mut stack = Vec::new();
    let mut out = Vec::new();
    let mut tokens = Vec::new();
    for line in src.split('\n') {
        lang.tokenize_line(line, &mut stack, &mut tokens);
        out.push(tokens.iter().map(|t| (line[t.start as usize..t.end as usize].to_string(), t.kind.name().to_string())).collect());
    }
    out
}

fn has(line: &[(String, String)], text: &str, kind: &str) -> bool {
    line.iter().any(|(t, k)| t == text && k == kind)
}

#[test]
fn every_builtin_loads() {
    let ids: Vec<&str> = Languages::builtin().iter().map(|l| l.id.as_str()).collect();
    for id in ["rapidr-basic", "plaintext", "json", "sql", "csv", "markdown", "html", "css", "javascript", "toml", "rust"] {
        assert!(ids.contains(&id), "{id} missing");
    }
    let set = Languages::builtin();
    assert_eq!(set.for_path("C:\\x\\Prog.BAS").id, "rapidr-basic");
    assert_eq!(set.for_path("a/b/main.rs").id, "rust");
    assert_eq!(set.for_path("Cargo.lock").id, "toml");
    assert_eq!(set.for_path("notes").id, "plaintext");
    assert_eq!(set.for_path("x.unknown").id, "plaintext");
}

#[test]
fn basic() {
    let l = lang("rapidr-basic");
    let c = colour(&l, "SUB Foo(x AS INTEGER) ' hi\n  if X1 = &HFF then PRINT \"a\" + LEFT$(s$, 2)\nREM old\n$INCLUDE \"rapidq.inc\"\nDIM f AS QFORM, g AS RBUTTON\nEND SUB");
    assert!(has(&c[0], "SUB", "keyword"));
    assert!(has(&c[0], "Foo", "function"));
    assert!(has(&c[0], "INTEGER", "type"));
    assert!(has(&c[0], "' hi", "comment"));
    assert!(has(&c[1], "if", "keyword.control"));
    assert!(has(&c[1], "&HFF", "number"));
    assert!(has(&c[1], "\"a\"", "string"));
    assert!(has(&c[1], "LEFT$", "function"));
    assert!(!c[1].iter().any(|(t, _)| t == "X1" || t == "s$"));
    assert!(has(&c[2], "REM old", "comment"));
    assert!(has(&c[3], "$INCLUDE", "directive"));
    assert!(has(&c[4], "QFORM", "type.component"));
    assert!(has(&c[4], "RBUTTON", "type.component"));
    assert!(has(&c[5], "END", "keyword.control"));
    // a string left open ends with its line; the next line is code again
    let c = colour(&l, "s = \"open\nPRINT 1");
    assert!(has(&c[0], "\"open", "string"));
    assert!(has(&c[1], "PRINT", "keyword"));
}

#[test]
fn basic_escapes_and_rust_blocks() {
    let l = lang("rapidr-basic");
    let c = colour(&l, "PRINT \"a\\nb\"\n$ESCAPECHARS ON\nPRINT \"a\\nb\"\n$ESCAPECHARS OFF\nPRINT \"a\\nb\"");
    assert!(!c[0].iter().any(|(_, k)| k == "string.escape"));
    assert!(has(&c[2], "\\n", "string.escape"));
    assert!(!c[4].iter().any(|(_, k)| k == "string.escape"));

    let c = colour(&l, "RUSTSTART\nfn main() { let x = 1; } // rust\n/* nested /* comment */\nstill */\nrustend\nPRINT 1");
    assert!(has(&c[0], "RUSTSTART", "keyword"));
    assert!(has(&c[1], "fn", "keyword"));
    assert!(has(&c[1], "// rust", "comment"));
    assert!(c[3].iter().any(|(t, k)| t.contains("still") && k == "comment"));
    assert!(has(&c[4], "rustend", "keyword"));
    assert!(has(&c[5], "PRINT", "keyword"));
}

#[test]
fn rust_and_javascript() {
    let r = lang("rust");
    let c = colour(&r, "let s = r#\"raw \" still\"#; 'a' 'life Vec<u8> println!(\"x{}\\n\")");
    assert!(has(&c[0], "r#\"raw \" still\"#", "string"));
    assert!(has(&c[0], "'a'", "string"));
    assert!(has(&c[0], "'life", "label"));
    assert!(has(&c[0], "Vec", "type"));
    assert!(has(&c[0], "u8", "type"));
    assert!(has(&c[0], "println!", "function.macro"));
    assert!(has(&c[0], "\\n", "string.escape"));

    let j = lang("javascript");
    let c = colour(&j, "const t = `a ${ {x: `b`}.x } c\nd` + 'e'; // f");
    assert!(has(&c[0], "const", "keyword"));
    assert!(c[0].iter().any(|(t, k)| t == "`a " && k == "string"));
    assert!(has(&c[0], "${", "punctuation"));
    assert!(c[1].iter().any(|(t, k)| t == "d`" && k == "string"));
    assert!(has(&c[1], "'e'", "string"));
    assert!(has(&c[1], "// f", "comment"));
}

#[test]
fn html_embeds_css_and_javascript() {
    let h = lang("html");
    let c = colour(&h, "<!DOCTYPE html>\n<style>\nbody { color: red; }\n</style>\n<script type=\"module\">\nlet x = 1;\n</script>\n<p class='a'>Hi &amp; bye</p>");
    assert!(has(&c[0], "<!DOCTYPE html>", "directive"));
    assert!(has(&c[1], "style", "keyword.tag"));
    assert!(has(&c[2], "body", "keyword.tag"));
    assert!(has(&c[2], "color", "variable.property"));
    assert!(has(&c[3], "</style>", "keyword.tag"));
    assert!(has(&c[4], "type", "variable.attribute"));
    assert!(has(&c[4], "\"module\"", "string"));
    assert!(has(&c[5], "let", "keyword"));
    assert!(has(&c[6], "</script>", "keyword.tag"));
    assert!(has(&c[7], "p", "keyword.tag"));
    assert!(has(&c[7], "&amp;", "constant.entity"));
}

#[test]
fn data_languages() {
    let c = colour(&lang("json"), "{\"a\": [1, -2.5e3, true, null, \"x\\u0041\\q\"], bad}");
    assert!(has(&c[0], "\"a\"", "variable.property"));
    assert!(has(&c[0], "-2.5e3", "number"));
    assert!(has(&c[0], "true", "constant"));
    assert!(has(&c[0], "\\u0041", "string.escape"));
    assert!(has(&c[0], "\\q", "invalid"));
    assert!(has(&c[0], "bad", "invalid"));

    let c = colour(&lang("toml"), "[package]\nname = \"x\" # c\nwhen = 1979-05-27T07:32:00Z\ns = \"\"\"\nmulti\n\"\"\"");
    assert!(has(&c[0], "[package]", "type.table"));
    assert!(has(&c[1], "name", "variable.key"));
    assert!(has(&c[1], "# c", "comment"));
    assert!(has(&c[2], "1979-05-27T07:32:00Z", "constant.date"));
    assert!(has(&c[4], "multi", "string"));

    let c = colour(&lang("sql"), "select a, count(*) from t where b = 'it''s\nok' -- c");
    assert!(has(&c[0], "select", "keyword"));
    assert!(has(&c[0], "count", "function"));
    assert!(has(&c[0], "''", "string.escape"));
    assert!(has(&c[1], "ok'", "string"));
    assert!(has(&c[1], "-- c", "comment"));

    let c = colour(&lang("csv"), "name,age,\"multi\nline\",3.5\nx1,12");
    assert!(has(&c[0], "\"multi", "string"));
    assert!(has(&c[1], "line\"", "string"));
    assert!(has(&c[1], "3.5", "number"));
    assert!(has(&c[2], "12", "number"));
    assert!(!c[2].iter().any(|(t, k)| t == "1" && k == "number"));

    let c = colour(&lang("markdown"), "# Title\nSome **bold** and `code` and [a link](http://x).\n```rust\nlet x;\n```\n> quote");
    assert!(has(&c[0], "# Title", "keyword.heading"));
    assert!(has(&c[1], "**bold**", "variable.strong"));
    assert!(has(&c[1], "`code`", "string.code"));
    assert!(has(&c[1], "[a link](http://x)", "function.link"));
    assert!(has(&c[3], "let x;", "string.code"));
    assert!(has(&c[5], "> quote", "comment.quote"));

    assert!(colour(&lang("plaintext"), "IF x THEN").iter().all(|l| l.is_empty()));
}

#[test]
fn user_definitions_and_their_errors() {
    let mut set = Languages::with_builtins();
    let ini = r#"
[language]
id = "ini"
name = "INI"
extensions = ["ini"]
line_comment = [";"]
[[states.root]]
match = ';.*$'
token = "comment"
[[states.root]]
match = '^\[[^\]]*\]'
token = "type.section"
"#;
    let l = set.load(ini).expect("loads");
    assert_eq!(set.for_path("a.INI").id, "ini");
    let c = colour(&l, "[core]\n; x");
    assert!(has(&c[0], "[core]", "type.section"));
    assert_eq!(TokenKind::named("type.section").unwrap().base(), TokenKind::TYPE);

    let bad = |body: &str| set.clone().load(&format!("[language]\nid = \"x\"\nname = \"X\"\n{body}")).unwrap_err().0;
    assert!(bad("[[states.root]]\nmatch = '('\ntoken = \"comment\"").contains("states.root rule 1"));
    assert!(bad("[[states.root]]\nmatch = 'a*'\ntoken = \"comment\"").contains("can match nothing"));
    assert!(bad("[[states.root]]\nmatch = 'a'\ntoken = \"heading\"").contains("unknown token kind"));
    assert!(bad("[[states.root]]\nmatch = 'a'\ntoken = \"comment\"\npush = \"nowhere\"").contains("no state"));
    assert!(bad("[[states.root]]\nmatch = 'a'\nembed = \"cobol\"\nend = 'b'").contains("unknown language"));
    assert!(bad("[[states.other]]\nmatch = 'a'\ntoken = \"comment\"").contains("no [[states.root]]"));
    assert!(bad("[[states.root]]\ninclude = \"root\"").contains("too deep"));
    assert!(bad("[colours]\nx = 1").contains("unknown field"));
}

#[test]
fn hostile_lines_stay_bounded() {
    // a pushing rule on every character can't grow the stack without bound,
    // and a 1 MB line is tokenized only so far
    let mut set = Languages::new();
    let l = set
        .load("[language]\nid = \"nest\"\nname = \"N\"\n[defaults]\nb = \"comment\"\n[[states.root]]\nmatch = '\\('\ntoken = \"comment\"\npush = \"b\"\n[[states.b]]\nmatch = '\\('\ntoken = \"comment\"\npush = \"b\"\n[[states.b]]\nmatch = '$'\npush = \"b\"")
        .unwrap();
    let mut stack = Vec::new();
    let mut tokens = Vec::new();
    l.tokenize_line(&"(".repeat(10_000), &mut stack, &mut tokens);
    assert!(stack.len() <= rapidr_editor::lang::MAX_STACK);
    let basic = lang("rapidr-basic");
    let long = "PRINT 1 ".repeat(200_000);
    let mut stack = Vec::new();
    basic.tokenize_line(&long, &mut stack, &mut tokens);
    assert!(tokens.last().unwrap().end as usize <= rapidr_editor::lang::MAX_LINE_BYTES);
}
