//! What `rapidr i18n` does for translators: a template (POT) from a
//! program's sources, the pseudo-language's catalog made from it, a
//! language's file brought up to date with it, how much of it a language
//! has, and the check that translations keep their markers.

use crate::extract::{self, Found, Keyword};
use crate::markers;
use crate::po::{self, Message};
use crate::pseudo;

/// The header entry of a template, or of a language's file for `language`
/// ("" for a template).
pub fn header(project: &str, language: &str, plural_forms: &str) -> Message {
    let mut text = format!("Project-Id-Version: {project}\nContent-Type: text/plain; charset=UTF-8\nContent-Transfer-Encoding: 8bit\n");
    if !language.is_empty() {
        text.push_str(&format!("Language: {language}\n"));
    }
    if !plural_forms.is_empty() {
        text.push_str(&format!("Plural-Forms: {plural_forms}\n"));
    }
    Message { forms: vec![text], ..Default::default() }
}

/// A template from sources (file name, text), in the order given (each
/// source's keywords: every source's `' i18n-keywords:` lines, and
/// `extra`): the header, then each text once, where it is used (the files,
/// without line numbers, so a template doesn't change with every edit).
pub fn template(project: &str, sources: &[(String, String)], extra: &[Keyword]) -> Vec<Message> {
    let mut keywords: Vec<Keyword> = extra.to_vec();
    for (_, text) in sources {
        for k in extract::keywords_in(text) {
            if !keywords.contains(&k) {
                keywords.push(k);
            }
        }
    }
    let mut out: Vec<Message> = vec![header(project, "", "")];
    let mut index: std::collections::HashMap<(String, String), usize> = std::collections::HashMap::new();
    for (file, text) in sources {
        for Found { context, id, plural, file, .. } in extract::extract(text, file, &keywords) {
            let key = (context.clone(), id.clone());
            match index.get(&key) {
                Some(&i) => {
                    let m = &mut out[i];
                    if !m.refs.contains(&file) {
                        m.refs.push(file);
                    }
                    if m.plural.is_none() {
                        m.plural = plural;
                    }
                }
                None => {
                    index.insert(key, out.len());
                    out.push(Message { context, id, plural, refs: vec![file], ..Default::default() });
                }
            }
        }
    }
    out
}

/// A template of RapidR's own text from Rust sources (file name, text):
/// every `tr_ui` / `tr_ui_fill` ([`extract::extract_rust`]).
pub fn rust_template(project: &str, sources: &[(String, String)]) -> Vec<Message> {
    let mut out: Vec<Message> = vec![header(project, "", "")];
    for (file, text) in sources {
        for f in extract::extract_rust(text, file) {
            match out.iter_mut().find(|m| !m.is_header() && m.id == f.id) {
                Some(m) => {
                    if !m.refs.contains(&f.file) {
                        m.refs.push(f.file);
                    }
                }
                None => out.push(Message { id: f.id, refs: vec![f.file], ..Default::default() }),
            }
        }
    }
    out
}

/// The pseudo-language's catalog (`xx.po`) for a template: every text in
/// it ([`pseudo::pseudo`]).
pub fn pseudo_catalog(template: &[Message], project: &str) -> Vec<Message> {
    let mut out = vec![header(project, pseudo::TAG, "nplurals=2; plural=(n != 1);")];
    for m in template.iter().filter(|m| !m.is_header()) {
        let mut forms = vec![pseudo::pseudo(&m.id)];
        if let Some(p) = &m.plural {
            forms.push(pseudo::pseudo(p));
        }
        out.push(Message { context: m.context.clone(), id: m.id.clone(), plural: m.plural.clone(), forms, notes: vec!["generated: the pseudo-language".into()], ..Default::default() });
    }
    out
}

/// What [`update`] did.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Updated {
    /// Texts new in the template (untranslated in the file now).
    pub added: usize,
    /// Translations kept.
    pub kept: usize,
    /// Translations of texts the template no longer has (left out of the
    /// file; listed so nothing is lost unseen).
    pub dropped: Vec<String>,
}

/// A language's file (its entries) brought up to date with a template:
/// the template's texts in its order, each with the file's translation,
/// flags and translator comments when it has one; its header kept (or
/// made). Code notes and references come from the template.
pub fn update(template: &[Message], file: &[Message], project: &str, language: &str, plural_forms: &str) -> (Vec<Message>, Updated) {
    let mut stats = Updated::default();
    let head = file.iter().find(|m| m.is_header()).cloned().unwrap_or_else(|| header(project, language, plural_forms));
    let mut out = vec![head];
    let find = |c: &str, id: &str| file.iter().find(|m| !m.is_header() && m.context == c && m.id == id);
    for t in template.iter().filter(|m| !m.is_header()) {
        let mut m = Message { context: t.context.clone(), id: t.id.clone(), plural: t.plural.clone(), notes: t.notes.clone(), refs: t.refs.clone(), ..Default::default() };
        match find(&t.context, &t.id) {
            Some(old) if old.translated() => {
                m.forms = old.forms.clone();
                m.flags = old.flags.clone();
                m.comments = old.comments.clone();
                stats.kept += 1;
            }
            _ => stats.added += 1,
        }
        out.push(m);
    }
    for old in file.iter().filter(|m| !m.is_header() && m.translated()) {
        if !template.iter().any(|t| t.context == old.context && t.id == old.id) {
            stats.dropped.push(old.id.clone());
        }
    }
    (out, stats)
}

/// How much of a template a language's file translates.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Coverage {
    /// The template's texts.
    pub total: usize,
    /// Translated and used (not `fuzzy`).
    pub translated: usize,
    /// Of those, marked `review` (a draft no person has checked yet).
    pub review: usize,
    /// Translated but `fuzzy`: not used until someone checks them.
    pub fuzzy: usize,
}

impl Coverage {
    pub fn percent(&self) -> f64 {
        if self.total == 0 {
            100.0
        } else {
            self.translated as f64 * 100.0 / self.total as f64
        }
    }
}

/// [`Coverage`] of `file` against `template`.
pub fn coverage(template: &[Message], file: &[Message]) -> Coverage {
    let mut c = Coverage::default();
    for t in template.iter().filter(|m| !m.is_header()) {
        c.total += 1;
        if let Some(m) = file.iter().find(|m| !m.is_header() && m.context == t.context && m.id == t.id && m.translated()) {
            if m.has_flag("fuzzy") {
                c.fuzzy += 1;
            } else {
                c.translated += 1;
                if m.has_flag("review") {
                    c.review += 1;
                }
            }
        }
    }
    c
}

/// A translation that loses its source's markers: the source text and
/// what is wrong ([`markers::check`]).
pub fn check(language: &str, file: &[Message]) -> Vec<(String, Vec<String>)> {
    let mut out = Vec::new();
    for m in file.iter().filter(|m| !m.is_header() && m.translated()) {
        let mut problems = markers::check_for(language, &m.id, m.forms.first().map(String::as_str).unwrap_or(""));
        if let Some(p) = &m.plural {
            for f in m.forms.iter().skip(1).filter(|f| !f.is_empty()) {
                problems.extend(markers::check_for(language, p, f));
            }
        }
        if !problems.is_empty() {
            out.push((m.id.clone(), problems));
        }
    }
    out
}

/// Two items of one menu with the same mnemonic letter: (the menu, the
/// letter, the captions). `menus` gives each menu's captions as shown.
pub fn mnemonic_clashes(menus: &[(String, Vec<String>)]) -> Vec<(String, char, Vec<String>)> {
    let mut out = Vec::new();
    for (menu, captions) in menus {
        let mut seen: Vec<(char, Vec<String>)> = Vec::new();
        for c in captions {
            if let Some(k) = markers::mnemonic(c) {
                match seen.iter_mut().find(|(l, _)| *l == k) {
                    Some((_, v)) => v.push(c.clone()),
                    None => seen.push((k, vec![c.clone()])),
                }
            }
        }
        for (k, v) in seen.into_iter().filter(|(_, v)| v.len() > 1) {
            out.push((menu.clone(), k, v));
        }
    }
    out
}

/// A catalog's entries as a file's text (a template, a language's file).
pub fn write(messages: &[Message]) -> String {
    po::write_po(messages)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn src() -> Vec<(String, String)> {
        vec![
            ("ide/a.inc".into(), "' i18n-keywords: AddCmd:2\nAddCmd \"x\", \"&Open\"\nl = TR$(\"Ready\")\n".into()),
            ("ide/b.inc".into(), "m = TR$(\"Ready\")\nn = TRN$(\"{n} file\", \"{n} files\", k)\n".into()),
        ]
    }

    #[test]
    fn templates_and_pseudo() {
        let t = template("RapidR Studio", &src(), &[]);
        assert_eq!(t.len(), 4);
        assert_eq!(t[2].refs, ["ide/a.inc", "ide/b.inc"]);
        let text = write(&t);
        assert!(text.contains("msgid \"&Open\"\nmsgstr \"\""), "{text}");
        let back = po::read_messages(&text);
        let key = |m: &Message| (m.context.clone(), m.id.clone(), m.plural.clone(), m.refs.clone(), m.translated());
        assert_eq!(back.iter().map(key).collect::<Vec<_>>(), t.iter().map(key).collect::<Vec<_>>());
        let xx = po::catalog_of(pseudo_catalog(&t, "RapidR Studio"));
        assert_eq!(xx.text("", "Ready"), Some("[Ŕéàðÿ··]"));
        assert_eq!(xx.plural_text("", "{n} file", 3), Some("[{n} fíĺéš····]"));
    }

    #[test]
    fn updates_keep_translations() {
        let t = template("S", &src(), &[]);
        let es = po::read_messages("msgid \"\"\nmsgstr \"Language: es\\n\"\n\nmsgid \"Ready\"\nmsgstr \"Listo\"\n\n#, fuzzy\nmsgid \"&Open\"\nmsgstr \"&Abrir\"\n\nmsgid \"Gone\"\nmsgstr \"Ido\"\n");
        let (new, stats) = update(&t, &es, "S", "es", "");
        assert_eq!((stats.kept, stats.added, stats.dropped.clone()), (2, 1, vec!["Gone".to_string()]));
        let c = coverage(&t, &new);
        assert_eq!((c.total, c.translated, c.fuzzy), (3, 1, 1));
        assert!(check("es", &new).is_empty());
        let bad = po::read_messages("msgid \"&Open {f}\"\nmsgstr \"Abrir\"\n");
        assert_eq!(check("es", &bad)[0].1.len(), 2);
    }

    #[test]
    fn clashes() {
        let menus = vec![("File".to_string(), vec!["&Open".to_string(), "&Save".to_string(), "Sa&ve As".to_string(), "&Salir".to_string()])];
        let c = mnemonic_clashes(&menus);
        assert_eq!(c, vec![("File".to_string(), 's', vec!["&Save".to_string(), "&Salir".to_string()])]);
    }
}
