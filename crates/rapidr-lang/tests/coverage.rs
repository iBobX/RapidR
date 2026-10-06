//! The registry covers what the compilers and the runtimes know, and agrees
//! with them (docs/ide-plan.md, stage I0's acceptance): every component the
//! compilers create and every name rule of theirs, every builtin of the
//! builtins table, every statement keyword the lexer and parser know, every
//! directive the preprocessor handles, every constant it supplies, the
//! value-method rule — and every entry has its docs.

use std::collections::BTreeSet;

use rapidr_lang::*;

fn repo(rel: &str) -> String {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..").join(rel);
    std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()))
}

/// The string patterns of the arms of `match … {` in `fn name` of `src`.
fn quoted_in_fn(src: &str, name: &str) -> BTreeSet<String> {
    let start = src.find(&format!("fn {name}")).unwrap_or_else(|| panic!("fn {name}"));
    let body = &src[start..];
    let end = body.find("\n}\n").unwrap_or(body.len());
    let mut out = BTreeSet::new();
    let mut rest = &body[..end];
    while let Some(i) = rest.find('"') {
        let tail = &rest[i + 1..];
        let Some(j) = tail.find('"') else { break };
        out.insert(tail[..j].to_string());
        rest = &tail[j + 1..];
    }
    out
}

#[test]
fn component_types_are_the_compilers() {
    assert_eq!(rapidr_ast::COMPONENT_TYPES, COMPONENT_TYPES, "rapidr-ast's list is the registry's");
    let created: Vec<&str> = COMPONENTS.iter().filter(|c| c.kind == Kind::Component).map(|c| c.name).collect();
    assert_eq!(created, COMPONENT_TYPES);
    for t in COMPONENT_TYPES {
        assert!(rapidr_ast::is_component_type_name(t), "{t}");
    }
}

#[test]
fn rapidq_names_follow_the_compilers_rules() {
    let include_lib: Vec<&str> = rapidr_ast::INCLUDE_LIBRARY_COMPONENTS.iter().map(|(q, _)| *q).collect();
    for c in COMPONENTS.iter().filter(|c| c.kind == Kind::Component) {
        for q in c.rapidq.iter().chain(c.aliases.iter()) {
            if include_lib.contains(q) {
                continue; // (the parser decides: the program's own TYPE or the built-in)
            }
            assert_eq!(rapidr_ast::canonical_type_name(q), c.name, "{q} means {}", c.name);
        }
    }
    for (q, r) in rapidr_ast::INCLUDE_LIBRARY_COMPONENTS.iter().chain(rapidr_ast::library::LIBRARY_TYPES) {
        assert_eq!(component(q).map(|c| c.name), Some(*r), "{q}");
    }
    // (every Q name canonical_type_name maps is the registry's)
    for c in COMPONENTS.iter().filter(|c| c.kind == Kind::Component) {
        let q = format!("Q{}", &c.name[1..]);
        if rapidr_ast::canonical_type_name(&q) == c.name && c.rapidq.is_none() {
            // RapidQ has no such name: the prefix rule accepts it (QPLOT), the
            // registry doesn't call it RapidQ's (docs/q-and-r-components.md §2)
            assert!(component(&q).is_none(), "{q}");
        }
    }
    for q in rapidr_ast::RAPIDQ_OBJECTS_NOT_YET_IMPLEMENTED {
        assert_eq!(component(q).map(|c| c.kind), Some(Kind::Planned), "{q}");
    }
    for (name, file, _) in rapidr_preprocessor::RAPIDR_LIBRARIES {
        let c = component(name).unwrap_or_else(|| panic!("{name}"));
        assert_eq!((c.kind, c.from), (Kind::Library, Some(*file)), "{name}");
    }
}

#[test]
fn every_builtin() {
    let table: BTreeSet<&str> = rapidr_bytecode::builtins::BUILTINS.iter().copied().collect();
    for key in &table {
        assert!(builtin(key).is_some() || INTERNAL_BUILTINS.contains(key), "BUILTINS' `{key}` isn't in the registry (builtins.toml)");
    }
    for key in INTERNAL_BUILTINS {
        assert!(table.contains(key), "internal `{key}` isn't in BUILTINS");
    }
    for b in BUILTINS {
        assert_eq!(b.bare, rapidr_bytecode::builtins::BARE_BUILTINS.contains(&b.key), "{}: bare", b.name);
        if b.missing {
            assert!(!table.contains(b.key), "{} is marked missing but the runtimes have it", b.name);
        } else if !table.contains(b.key) {
            // (the compilers lower it themselves)
            assert!(compiles(&format!("DIM v AS INTEGER\nx = {}(v)\n", b.name)) || compiles(&format!("{} 1\n", b.name)) || b.key.starts_with("param"),
                "{} isn't in BUILTINS and doesn't compile", b.name);
        }
    }
    // (bcgen's list of RapidQ's builtins: every one is the registry's, RapidQ's)
    let bcgen = repo("interpreter/rapidr-bcgen/src/lib.rs");
    let start = bcgen.find("const RAPIDQ_BUILTINS").expect("RAPIDQ_BUILTINS");
    let list = &bcgen[start..start + bcgen[start..].find("];").unwrap()];
    for key in list.split('"').skip(1).step_by(2) {
        let b = builtin(key).unwrap_or_else(|| panic!("bcgen's RapidQ builtin `{key}` isn't in the registry"));
        assert_eq!(b.origin, Origin::RapidQ, "{key}");
    }
}

fn compiles(src: &str) -> bool {
    let Ok(tokens) = rapidr_lexer::Lexer::new(src, None).tokenize() else { return false };
    let Ok(program) = rapidr_parser::parse_tokens(&tokens) else { return false };
    rapidr_bcgen::compile_program_with_source(&program, Some(src)).is_ok()
}

#[test]
fn statements_and_keywords() {
    let names: BTreeSet<String> = STATEMENTS
        .iter()
        .flat_map(|s| s.name.split([' ', '#']))
        .chain(KEYWORDS.iter().map(|k| k.name))
        .chain(TYPE_NAMES.iter().map(|t| t.name))
        // (builtins written as statements: KILL "file", CLOSE #1)
        .chain(BUILTINS.iter().map(|b| b.name.trim_end_matches('$')))
        .filter(|w| !w.is_empty())
        .map(str::to_ascii_uppercase)
        .collect();
    // The lexer's keywords.
    for k in quoted_in_fn(&repo("crates/rapidr-lexer/src/lib.rs"), "keyword_token") {
        assert!(names.contains(&k), "the lexer's keyword {k} isn't in the registry (language.toml)");
    }
    // The words the parser recognizes as statements.
    let parser = repo("crates/rapidr-parser/src/lib.rs");
    for part in parser.split("peek_identifier_eq(\"").skip(1) {
        let word = &part[..part.find('"').unwrap()];
        assert!(names.contains(word), "the parser's {word} isn't in the registry (language.toml)");
    }
    // Every statement the registry lists starts with a word the lexer, the
    // parser or a lowering knows (`"DIM"`, `"swap"`, `TokenType::Data`).
    let sources = [repo("crates/rapidr-lexer/src/lib.rs"), parser.clone(), repo("crates/rapidr-ast/src/lib.rs"), repo("interpreter/rapidr-bcgen/src/lib.rs")];
    for s in STATEMENTS {
        let first = s.name.split(' ').next().unwrap();
        let camel = format!("TokenType::{}{}", &first[..1], first[1..].to_ascii_lowercase());
        let known = sources.iter().any(|src| {
            src.contains(&format!("\"{first}\"")) || src.contains(&format!("\"{}\"", first.to_ascii_lowercase())) || src.contains(&camel)
        });
        assert!(known, "statement {} isn't one the parser knows", s.name);
    }
}

#[test]
fn directives() {
    let registry: BTreeSet<&str> = DIRECTIVES.iter().map(|d| d.name).collect();
    let mut found = BTreeSet::new();
    for src in [repo("crates/rapidr-preprocessor/src/lib.rs"), repo("crates/rapidr-parser/src/lib.rs"), repo("crates/rapidr-lexer/src/lib.rs")] {
        for part in src.split("\"$").skip(1) {
            let word: String = part.chars().take_while(|c| c.is_ascii_uppercase()).collect();
            if !word.is_empty() && part[word.len()..].starts_with('"') {
                found.insert(format!("${word}"));
            }
        }
    }
    for d in &found {
        assert!(registry.contains(d.as_str()), "{d} (the preprocessor's) isn't in the registry");
    }
    for d in &registry {
        assert!(found.contains(*d), "{d} is in the registry but nothing handles it");
    }
}

#[test]
fn constants_are_the_preprocessors() {
    let rapidq_inc: Vec<(String, i64)> =
        CONSTANT_GROUPS.iter().filter(|g| g.source == "RAPIDQ.INC").flat_map(|g| g.constants.iter().map(|(n, v)| (n.to_string(), *v))).collect();
    let pre: Vec<(String, i64)> = rapidr_preprocessor::RAPIDQ_INC_CONSTANTS.iter().map(|(n, v)| (n.to_string(), *v)).collect();
    assert_eq!(rapidq_inc, pre, "RAPIDQ.INC's constants, in order");
    let rapidr: Vec<(String, i64)> =
        CONSTANT_GROUPS.iter().filter(|g| g.origin == Origin::RapidR).flat_map(|g| g.constants.iter().map(|(n, v)| (n.to_ascii_lowercase(), *v))).collect();
    let ast: Vec<(String, i64)> = rapidr_ast::RAPIDR_CONSTANTS.iter().map(|(n, v)| (n.to_string(), *v)).collect();
    assert_eq!(rapidr, ast, "RapidR's own constants");
    // (the library includes': rapidr_preprocessor's LIBRARY_INCLUDES)
    let src = repo("crates/rapidr-preprocessor/src/lib.rs");
    let start = src.find("const LIBRARY_INCLUDES").unwrap();
    let table = &src[start..start + src[start..].find("\n];").unwrap()];
    for line in table.lines().filter(|l| l.contains(".inc\"")) {
        let file = line.split('"').nth(1).unwrap();
        let consts: Vec<&str> = line.split('"').skip(3).step_by(2).collect();
        for c in consts {
            let (_, g) = constant(c).unwrap_or_else(|| panic!("{file}'s {c}"));
            assert!(g.source.eq_ignore_ascii_case(file), "{c}: {} not {file}", g.source);
        }
    }
}

#[test]
fn value_methods_follow_the_runtimes_rule() {
    for c in COMPONENTS.iter().filter(|c| c.kind == Kind::Component) {
        for m in c.methods.iter().filter(|m| !m.missing) {
            assert_eq!(
                m.value,
                rapidr_value::members::is_value_method(c.name, &m.name.to_ascii_lowercase()),
                "{}.{}: `value` must be rapidr_value::members' rule",
                c.name,
                m.name
            );
        }
    }
}

#[test]
fn everything_has_docs() {
    let mut missing = Vec::new();
    for c in COMPONENTS.iter().chain(GLOBALS) {
        if c.doc.is_empty() {
            missing.push(c.name.to_string());
        }
        for p in c.properties {
            if p.doc.is_empty() {
                missing.push(format!("{}.{}", c.name, p.name));
            }
        }
        for m in c.methods {
            if m.doc.is_empty() {
                missing.push(format!("{}.{}()", c.name, m.name));
            }
        }
        for e in c.events {
            if e.doc.is_empty() {
                missing.push(format!("{}.{}", c.name, e.name));
            }
        }
    }
    missing.extend(BUILTINS.iter().filter(|b| b.doc.is_empty()).map(|b| b.name.to_string()));
    missing.extend(STATEMENTS.iter().filter(|s| s.doc.is_empty()).map(|s| s.name.to_string()));
    missing.extend(DIRECTIVES.iter().filter(|s| s.doc.is_empty()).map(|s| s.name.to_string()));
    missing.extend(KEYWORDS.iter().filter(|s| s.doc.is_empty()).map(|s| s.name.to_string()));
    missing.extend(TYPE_NAMES.iter().filter(|s| s.doc.is_empty()).map(|s| s.name.to_string()));
    missing.extend(CONSTANT_GROUPS.iter().filter(|s| s.doc.is_empty()).map(|s| s.name.to_string()));
    assert!(missing.is_empty(), "{} entries without docs: {}", missing.len(), missing.join(", "));
}

#[test]
fn categories_are_known() {
    const CATEGORIES: &[&str] = &["Layout", "Appearance", "Behavior", "Data", "Font", "Window", "Drawing", "Media", "Network", "Help", "Accessibility", "Misc", ""];
    for c in COMPONENTS.iter().chain(GLOBALS) {
        for p in c.properties {
            assert!(CATEGORIES.contains(&p.category), "{}.{}: category {}", c.name, p.name, p.category);
        }
    }
}
