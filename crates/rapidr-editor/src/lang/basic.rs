//! RapidQ / RapidR BASIC's keyword groups, added to `rapidq-basic.toml`'s
//! rules when it loads: the language registry's words
//! (`rapidr_lang::words`) — its statements, keywords, types, operators,
//! components under both names and the builtins the runtimes implement —
//! so the editor colours exactly the language the compilers accept.

use std::sync::OnceLock;

/// The groups, as `Language::from_toml_with` takes them (the first group
/// that has a word decides its kind).
pub fn keyword_groups() -> &'static [(&'static str, &'static [&'static str])] {
    static GROUPS: OnceLock<Vec<(&'static str, &'static [&'static str])>> = OnceLock::new();
    GROUPS.get_or_init(|| {
        let group = |kind: &'static str, words: &str| -> (&'static str, &'static [&'static str]) {
            (kind, Box::leak(rapidr_lang::words(words).into_boxed_slice()))
        };
        vec![
            group("keyword.control", "control"),
            group("keyword", "keyword"),
            group("type", "type"),
            group("keyword.operator", "operator"),
            group("constant", "constant"),
            group("type.component", "component"),
            group("type.component", "component_q"),
            group("function", "builtin"),
        ]
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeSet;

    fn group(kind: &str) -> BTreeSet<&'static str> {
        keyword_groups().iter().filter(|(k, _)| *k == kind).flat_map(|(_, w)| w.iter().copied()).collect()
    }

    fn lexes_as_keyword(word: &str) -> bool {
        let tokens = rapidr_lexer::Lexer::new(&format!("{word} "), None).tokenize().expect("lexes");
        !matches!(tokens.first().map(|t| t.kind), Some(rapidr_lexer::TokenType::Identifier) | None)
    }

    #[test]
    fn the_lexers_keywords_are_coloured() {
        let all: BTreeSet<&str> = keyword_groups().iter().flat_map(|(_, w)| w.iter().copied()).collect();
        for w in ["IF", "THEN", "NEXT", "LOOP", "DIM", "AS", "CREATE", "DEFLNG", "DEFLONG", "RUSTSTART", "RUSTEND", "INTEGER", "INT64", "AND", "MOD"] {
            assert!(lexes_as_keyword(w), "{w} is a keyword of rapidr-lexer");
            assert!(all.contains(w), "{w} isn't coloured");
        }
        assert!(group("keyword.control").contains("WEND") && group("keyword.operator").contains("XOR"));
        // (statements the parser knows by name: coloured too)
        assert!(group("keyword").contains("SWAP") && group("keyword").contains("REDIM"));
    }

    #[test]
    fn the_components() {
        let ours = group("type.component");
        for r in rapidr_ast::COMPONENT_TYPES {
            assert!(ours.contains(r), "{r}");
        }
        for q in ["QFORM", "QBUTTON", "QGAUGE", "QOUTLINE", "COMPORT"] {
            assert!(ours.contains(q), "{q}");
        }
        assert!(!ours.contains("QPLOT"), "RapidQ has no QPLOT");
    }

    #[test]
    fn the_builtins() {
        let ours = group("function");
        let words: BTreeSet<&str> = keyword_groups().iter().flat_map(|(_, w)| w.iter().copied()).collect();
        let internal = |b: &str| b.starts_with("__") || b.contains('.') || b.ends_with("_func") || b.ends_with("_hash") || b.ends_with("_field") || b == "line_input" || b == "rapidr__waitkey";
        for b in rapidr_bytecode::builtins::BUILTINS.iter().filter(|b| !internal(b)) {
            let upper = b.to_ascii_uppercase();
            assert!(words.contains(upper.as_str()) || matches!(*b, "e" | "get" | "println"), "the builtin {upper} isn't coloured");
        }
        assert!(ours.contains("MID") && ours.contains("COMMANDCOUNT"));
    }
}
