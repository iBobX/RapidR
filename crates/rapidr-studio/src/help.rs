//! RapidR Studio's Help pane (docs/studio-wow.md HLP-1): the language
//! registry's entry for a word, as a page the pane shows. The entries and the
//! pages — `entry`, `lookup`, `page`, `suggest`, `Ask` — are
//! `rapidr_langsvc::help` (one implementation for this pane and for
//! `rapidr lsp`'s Help request, so every editor shows the same pages), re-exported
//! here; what stays here is the chat's hover, `quick`, whose type belongs to
//! the UI's Markdown view.

pub use rapidr_langsvc::help::*;

/// The components a member name is looked up in first (a member after a
/// dot, in a chat's answer, whose variable's type isn't known): the usual
/// ones, then the registry's order.
const USUAL: [&str; 12] = ["RForm", "RButton", "RLabel", "REdit", "RListBox", "RComboBox", "RMemo", "RPanel", "RTimer", "RCheckBox", "RRadioButton", "RImage"];

/// A member named `name` (a method, a property, an event) of some
/// component, when the component isn't known: the entry, and the
/// component's name.
fn member_anywhere(name: &str) -> Option<(Vec<String>, &'static str)> {
    let usual = USUAL.iter().filter_map(|n| rapidr_lang::component(n));
    let rest = rapidr_lang::COMPONENTS.iter().filter(|c| c.kind == rapidr_lang::Kind::Component);
    usual.chain(rest).find_map(|c| member(c, name).map(|m| (m, c.name)))
}

/// A text cut at its first sentence or clause ("…; …" — what follows is for
/// the page), at most `max` characters, cut at a word.
fn first_sentence(t: &str, max: usize) -> String {
    let one: String = t.split_whitespace().collect::<Vec<_>>().join(" ");
    let end = [". ", "; ", " — ", " (", ": "].iter().filter_map(|m| one.find(m).map(|i| if *m == ". " { i + 1 } else { i })).min().unwrap_or(one.len());
    let clause = &one[..end];
    if clause.chars().count() <= max {
        return clause.trim_end_matches([',', ';', ':']).to_string();
    }
    let cut: String = clause.chars().take(max).collect();
    let at = cut.rfind(' ').unwrap_or(cut.len());
    format!("{}…", cut[..at].trim_end_matches([',', ';', ':', ' ']))
}

/// A word of code as one line, for a tooltip: its name or syntax, what it
/// is, the first sentence of what it does ("PRINT — Statement: Writes
/// text to the console."). `member`: a dot comes before the word, so it is
/// a member of a component (looked up in the usual ones first); the
/// component's name goes with it. `None` when the registry doesn't know
/// the word.
pub fn quick(word: &str, member: bool) -> Option<rapidr_value::panels::markdown::WordHelp> {
    let (parts, of) = if member {
        let (parts, of) = member_anywhere(word)?;
        (parts, of.to_string())
    } else {
        (entry(word, "")?, String::new())
    };
    let head = match parts.first() {
        Some(t) if member => t.clone(),
        Some(t) => parts.get(1).map(|syn| syn.lines().next().unwrap_or(t)).filter(|syn| !syn.is_empty() && syn.chars().count() <= 40 && !syn.contains("CREATE")).unwrap_or(t).to_string(),
        None => return None,
    };
    let what = parts.get(2).map(|w| w.split(" · ").next().unwrap_or(w)).unwrap_or("");
    let what = what.split(" = ").next().unwrap_or(what);
    let doc = parts.get(3).map(|d| first_sentence(d, 72)).unwrap_or_default();
    let tip = match (what.is_empty(), doc.is_empty()) {
        (true, true) => head,
        (false, true) => format!("{head} — {what}"),
        (true, false) => format!("{head} — {doc}"),
        (false, false) => format!("{head} — {what}: {doc}"),
    };
    Some(rapidr_value::panels::markdown::WordHelp { tip, of })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The chat's hover: a word of code in one line, a member after a dot
    /// with the component it was found in.
    #[test]
    fn a_word_of_code_is_one_line_of_help() {
        let t = |w: &str, m: bool| quick(w, m).map(|h| (h.tip, h.of));
        let (tip, of) = t("PRINT", false).expect("a statement");
        assert!(tip.starts_with("PRINT") && tip.contains(" — "), "{tip}");
        assert!(of.is_empty() && !tip.contains('\n') && tip.chars().count() < 200, "{tip}");
        let (tip, _) = t("RListBox", false).expect("a component");
        assert!(tip.starts_with("RListBox — Component"), "{tip}");
        let (tip, of) = t("AddItems", true).expect("a member");
        assert!(tip.contains("AddItems") && !of.is_empty(), "{tip} / {of}");
        assert!(t("Total", false).is_none() && t("NoSuchMember", true).is_none(), "the program's own names and unknown words have none");
    }
}
