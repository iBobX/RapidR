//! `search_manual` (docs/preview4-plan.md K3): RAI's search of RapidR's own
//! user manual — the guide pages of docs/manual, ours (MIT), built in so
//! the web Studio has them too. Each page is cut into its sections (one per
//! heading); a question finds the best three by BM25 over their words (a
//! heading's words count twice), no model and no download, the same way
//! the recipes are found (`recipes`). Each section goes back trimmed, with
//! its page and heading, so the model can say where it read it.
//!
//! The reference pages (docs/manual/reference, generated from the
//! registry) aren't here: `rapidr_describe` answers from the registry
//! itself.

use std::collections::HashMap;
use std::sync::OnceLock;

/// The manual's guide pages (a test keeps the list whole).
pub const PAGES: &[(&str, &str)] = &[
    ("README.md", include_str!("../../../docs/manual/README.md")),
    ("ai-assistant.md", include_str!("../../../docs/manual/ai-assistant.md")),
    ("ai-apps.md", include_str!("../../../docs/manual/ai-apps.md")),
    ("app-eyes-and-hands.md", include_str!("../../../docs/manual/app-eyes-and-hands.md")),
    ("archives.md", include_str!("../../../docs/manual/archives.md")),
    ("building-apps.md", include_str!("../../../docs/manual/building-apps.md")),
    ("cli-and-runtime.md", include_str!("../../../docs/manual/cli-and-runtime.md")),
    ("compatibility.md", include_str!("../../../docs/manual/compatibility.md")),
    ("code-editor.md", include_str!("../../../docs/manual/code-editor.md")),
    ("components.md", include_str!("../../../docs/manual/components.md")),
    ("data-science-with-rapidr.md", include_str!("../../../docs/manual/data-science-with-rapidr.md")),
    ("data-science.md", include_str!("../../../docs/manual/data-science.md")),
    ("databases.md", include_str!("../../../docs/manual/databases.md")),
    ("debugging.md", include_str!("../../../docs/manual/debugging.md")),
    ("designing-forms.md", include_str!("../../../docs/manual/designing-forms.md")),
    ("directx-and-media.md", include_str!("../../../docs/manual/directx-and-media.md")),
    ("for-teachers.md", include_str!("../../../docs/manual/for-teachers.md")),
    ("getting-started.md", include_str!("../../../docs/manual/getting-started.md")),
    ("graphics.md", include_str!("../../../docs/manual/graphics.md")),
    ("importing-rapidq.md", include_str!("../../../docs/manual/importing-rapidq.md")),
    ("language.md", include_str!("../../../docs/manual/language.md")),
    ("local-ai.md", include_str!("../../../docs/manual/local-ai.md")),
    ("markdown.md", include_str!("../../../docs/manual/markdown.md")),
    ("network.md", include_str!("../../../docs/manual/network.md")),
    ("other-editors.md", include_str!("../../../docs/manual/other-editors.md")),
    ("recipes-systems.md", include_str!("../../../docs/manual/recipes-systems.md")),
    ("rich-text.md", include_str!("../../../docs/manual/rich-text.md")),
    ("secrets.md", include_str!("../../../docs/manual/secrets.md")),
    ("serial-ports.md", include_str!("../../../docs/manual/serial-ports.md")),
    ("showcase.md", include_str!("../../../docs/manual/showcase.md")),
    ("studio-editor.md", include_str!("../../../docs/manual/studio-editor.md")),
    ("studio-languages.md", include_str!("../../../docs/manual/studio-languages.md")),
    ("studio-projects.md", include_str!("../../../docs/manual/studio-projects.md")),
    ("svg-icons.md", include_str!("../../../docs/manual/svg-icons.md")),
    ("translations.md", include_str!("../../../docs/manual/translations.md")),
    ("troubleshooting.md", include_str!("../../../docs/manual/troubleshooting.md")),
    ("versions.md", include_str!("../../../docs/manual/versions.md")),
    ("web.md", include_str!("../../../docs/manual/web.md")),
    ("why-rapidr.md", include_str!("../../../docs/manual/why-rapidr.md")),
];

/// The most of a section a model gets (characters).
pub const SECTION_CHARS: usize = 1_400;

/// A section of a page: what one heading says, up to the next heading.
#[derive(Debug)]
pub struct Section {
    pub page: &'static str,
    /// The page's title and the section's heading: `The language › FOR`.
    pub heading: String,
    /// Its text: Markdown without pictures or comments.
    pub text: String,
    words: Vec<String>,
}

const STOP: &[&str] = &[
    "a", "an", "the", "i", "me", "my", "you", "your", "it", "its", "is", "are", "was", "be", "been", "to", "of", "and", "or", "for", "in", "on", "at", "by", "with", "that", "this", "these", "those", "as", "from", "do", "does", "how", "what", "when", "where", "which", "who", "why", "can", "could", "should", "would", "will", "into", "if", "then", "than", "there", "here", "so", "not", "no", "but", "all", "any", "some", "one", "has", "have", "get", "gets", "use", "uses", "using", "used",
];

/// A text's words for the search: lower case, accents folded (the
/// recipes' rule), the common ones gone, a plural's -s dropped.
pub fn words(text: &str) -> Vec<String> {
    crate::recipes::fold_text(&text.to_lowercase())
        .split(|c: char| !c.is_alphanumeric())
        .filter(|w| !w.is_empty() && !STOP.contains(w))
        .map(|w| if w.len() > 3 && w.ends_with('s') && !w.ends_with("ss") { w[..w.len() - 1].to_string() } else { w.to_string() })
        .collect()
}

/// A page cut into its sections.
fn split(page: &'static str, text: &str) -> Vec<Section> {
    let mut out = Vec::new();
    let mut title = String::new();
    let mut heading = String::new();
    let mut body: Vec<&str> = Vec::new();
    let mut fenced = false;
    let mut comment = false;
    let flush = |heading: &str, title: &str, body: &mut Vec<&str>, out: &mut Vec<Section>| {
        let text = body.join("\n").trim().to_string();
        body.clear();
        if text.is_empty() && heading.is_empty() {
            return;
        }
        let full = if title.is_empty() || title == heading { heading.to_string() } else if heading.is_empty() { title.to_string() } else { format!("{title} › {heading}") };
        let mut w = words(&full);
        w.extend(words(&full));
        w.extend(words(&text));
        out.push(Section { page, heading: full, text, words: w });
    };
    for line in text.lines() {
        let t = line.trim_start();
        if t.starts_with("```") {
            fenced = !fenced;
        }
        if !fenced {
            if t.starts_with("<!--") {
                comment = !t.contains("-->");
                continue;
            }
            if comment {
                comment = !t.contains("-->");
                continue;
            }
            if let Some(h) = t.strip_prefix('#') {
                let level = 1 + h.chars().take_while(|c| *c == '#').count();
                let h = h.trim_start_matches('#').trim().to_string();
                flush(&heading, &title, &mut body, &mut out);
                if level == 1 {
                    title = h.clone();
                }
                heading = h;
                continue;
            }
            // (pictures: their words are their alt text only)
            if t.starts_with("![") || t.starts_with("<img") {
                continue;
            }
        }
        body.push(line);
    }
    flush(&heading, &title, &mut body, &mut out);
    out
}

/// Every section of the manual (cut once).
pub fn sections() -> &'static [Section] {
    static ALL: OnceLock<Vec<Section>> = OnceLock::new();
    ALL.get_or_init(|| PAGES.iter().flat_map(|(page, text)| split(page, text)).filter(|s| !s.text.is_empty()).collect())
}

/// The manual's pages translated (docs/manual/<language>/<page>.md, a
/// test keeps the list whole): (language, page, text). A page with no
/// translation is read in English.
pub const TRANSLATED: &[(&str, &str, &str)] = &[
    ("es", "README.md", include_str!("../../../docs/manual/es/README.md")),
    ("es", "ai-apps.md", include_str!("../../../docs/manual/es/ai-apps.md")),
    ("es", "ai-assistant.md", include_str!("../../../docs/manual/es/ai-assistant.md")),
    ("es", "app-eyes-and-hands.md", include_str!("../../../docs/manual/es/app-eyes-and-hands.md")),
    ("es", "archives.md", include_str!("../../../docs/manual/es/archives.md")),
    ("es", "building-apps.md", include_str!("../../../docs/manual/es/building-apps.md")),
    ("es", "cli-and-runtime.md", include_str!("../../../docs/manual/es/cli-and-runtime.md")),
    ("es", "code-editor.md", include_str!("../../../docs/manual/es/code-editor.md")),
    ("es", "compatibility.md", include_str!("../../../docs/manual/es/compatibility.md")),
    ("es", "components.md", include_str!("../../../docs/manual/es/components.md")),
    ("es", "data-science-with-rapidr.md", include_str!("../../../docs/manual/es/data-science-with-rapidr.md")),
    ("es", "data-science.md", include_str!("../../../docs/manual/es/data-science.md")),
    ("es", "databases.md", include_str!("../../../docs/manual/es/databases.md")),
    ("es", "debugging.md", include_str!("../../../docs/manual/es/debugging.md")),
    ("es", "designing-forms.md", include_str!("../../../docs/manual/es/designing-forms.md")),
    ("es", "directx-and-media.md", include_str!("../../../docs/manual/es/directx-and-media.md")),
    ("es", "for-teachers.md", include_str!("../../../docs/manual/es/for-teachers.md")),
    ("es", "getting-started.md", include_str!("../../../docs/manual/es/getting-started.md")),
    ("es", "graphics.md", include_str!("../../../docs/manual/es/graphics.md")),
    ("es", "importing-rapidq.md", include_str!("../../../docs/manual/es/importing-rapidq.md")),
    ("es", "language.md", include_str!("../../../docs/manual/es/language.md")),
    ("es", "local-ai.md", include_str!("../../../docs/manual/es/local-ai.md")),
    ("es", "markdown.md", include_str!("../../../docs/manual/es/markdown.md")),
    ("es", "network.md", include_str!("../../../docs/manual/es/network.md")),
    ("es", "other-editors.md", include_str!("../../../docs/manual/es/other-editors.md")),
    ("es", "recipes-systems.md", include_str!("../../../docs/manual/es/recipes-systems.md")),
    ("es", "rich-text.md", include_str!("../../../docs/manual/es/rich-text.md")),
    ("es", "secrets.md", include_str!("../../../docs/manual/es/secrets.md")),
    ("es", "serial-ports.md", include_str!("../../../docs/manual/es/serial-ports.md")),
    ("es", "showcase.md", include_str!("../../../docs/manual/es/showcase.md")),
    ("es", "studio-editor.md", include_str!("../../../docs/manual/es/studio-editor.md")),
    ("es", "studio-languages.md", include_str!("../../../docs/manual/es/studio-languages.md")),
    ("es", "studio-projects.md", include_str!("../../../docs/manual/es/studio-projects.md")),
    ("es", "svg-icons.md", include_str!("../../../docs/manual/es/svg-icons.md")),
    ("es", "translations.md", include_str!("../../../docs/manual/es/translations.md")),
    ("es", "troubleshooting.md", include_str!("../../../docs/manual/es/troubleshooting.md")),
    ("es", "versions.md", include_str!("../../../docs/manual/es/versions.md")),
    ("es", "web.md", include_str!("../../../docs/manual/es/web.md")),
    ("es", "why-rapidr.md", include_str!("../../../docs/manual/es/why-rapidr.md")),
];

/// The translated pages' sections, as `es/getting-started.md`.
fn translated_sections() -> &'static [(&'static str, Vec<Section>)] {
    static ALL: OnceLock<Vec<(&'static str, Vec<Section>)>> = OnceLock::new();
    ALL.get_or_init(|| {
        let mut out: Vec<(&'static str, Vec<Section>)> = Vec::new();
        for (lang, page, text) in TRANSLATED {
            let name: &'static str = Box::leak(format!("{lang}/{page}").into_boxed_str());
            let secs: Vec<Section> = split(name, text).into_iter().filter(|s| !s.text.is_empty()).collect();
            match out.iter_mut().find(|(l, _)| l == lang) {
                Some((_, v)) => v.extend(secs),
                None => out.push((lang, secs)),
            }
        }
        out
    })
}

/// Whether the manual has pages translated into `tag`'s language (`es-UY`
/// → `es`): a page that isn't is read in English, and said so.
pub fn has_pages_in(tag: &str) -> bool {
    rapidr_value::i18n::fallback_chain(tag).iter().any(|t| TRANSLATED.iter().any(|(l, _, _)| l == t))
}

/// A page of the manual in language `tag` (`es-UY` → `es` → English):
/// its text, and whether it is the translation.
pub fn page_in(page: &str, tag: &str) -> Option<(&'static str, bool)> {
    let page = page.trim().trim_start_matches("./");
    for t in rapidr_value::i18n::fallback_chain(tag) {
        if let Some((_, _, text)) = TRANSLATED.iter().find(|(l, p, _)| *l == t && *p == page) {
            return Some((text, true));
        }
    }
    PAGES.iter().find(|(p, _)| *p == page).map(|(_, text)| (*text, false))
}

/// [`search`] for a person who speaks `tag`: the sections of the pages in
/// their language first, then the English manual's, `n` in all.
pub fn search_in(query: &str, n: usize, tag: &str) -> Vec<&'static Section> {
    let mut out: Vec<&'static Section> = Vec::new();
    for t in rapidr_value::i18n::fallback_chain(tag) {
        if let Some((_, secs)) = translated_sections().iter().find(|(l, _)| *l == t) {
            out.extend(rank(secs, query, n));
        }
    }
    for s in search(query, n) {
        if out.len() >= n {
            break;
        }
        out.push(s);
    }
    out.truncate(n);
    out
}

/// The `n` sections that answer `query` best (none when no word of it is
/// in the manual).
pub fn search(query: &str, n: usize) -> Vec<&'static Section> {
    rank(sections(), query, n)
}

fn rank<'a>(all: &'a [Section], query: &str, n: usize) -> Vec<&'a Section> {
    let mut q = words(query);
    q.sort();
    q.dedup();
    if q.is_empty() {
        return Vec::new();
    }
    // (BM25, k1 1.2, b 0.75; each term's document frequency counted once)
    let count = all.len() as f64;
    let avg = all.iter().map(|s| s.words.len()).sum::<usize>() as f64 / count.max(1.0);
    let df: HashMap<&str, f64> = q.iter().map(|t| (t.as_str(), all.iter().filter(|s| s.words.iter().any(|w| w == t)).count() as f64)).collect();
    let mut scored: Vec<(f64, usize)> = all
        .iter()
        .enumerate()
        .map(|(i, s)| {
            let mut tf: HashMap<&str, f64> = HashMap::new();
            for w in &s.words {
                if df.contains_key(w.as_str()) {
                    *tf.entry(w.as_str()).or_default() += 1.0;
                }
            }
            let score: f64 = q
                .iter()
                .map(|t| {
                    let d = df[t.as_str()];
                    let idf = ((count - d + 0.5) / (d + 0.5) + 1.0).ln();
                    let f = tf.get(t.as_str()).copied().unwrap_or(0.0);
                    idf * f * 2.2 / (f + 1.2 * (0.25 + 0.75 * s.words.len() as f64 / avg))
                })
                .sum();
            (score, i)
        })
        .filter(|(score, _)| *score > 0.0)
        .collect();
    scored.sort_by(|a, b| b.0.total_cmp(&a.0).then(a.1.cmp(&b.1)));
    scored.into_iter().take(n).map(|(_, i)| &all[i]).collect()
}

/// A section as a model gets it: where it is, then its text, trimmed at a
/// line to [`SECTION_CHARS`].
pub fn excerpt(s: &Section) -> String {
    let mut text = String::new();
    for line in s.text.lines() {
        if text.chars().count() + line.chars().count() > SECTION_CHARS {
            text.push_str("…");
            break;
        }
        text.push_str(line);
        text.push('\n');
    }
    // (the section's place in words, not a file path: the manual is built
    // into Studio, an AI app has no file to open — it's all here)
    format!("Manual › {}\n{}", s.heading, text.trim_end())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_guide_page_is_searched() {
        // (a page added to the manual is added here too)
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../docs/manual");
        let mut on_disk: Vec<String> = std::fs::read_dir(&dir).unwrap().flatten().map(|e| e.file_name().to_string_lossy().to_string()).filter(|n| n.ends_with(".md")).collect();
        on_disk.sort();
        let mut listed: Vec<String> = PAGES.iter().map(|(n, _)| n.to_string()).collect();
        listed.sort();
        assert_eq!(listed, on_disk);
    }

    #[test]
    fn every_translated_page_is_built_in() {
        // (docs/manual/<language>/<page>.md: each one in TRANSLATED, each
        // translating a page the manual has)
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../docs/manual");
        let mut on_disk: Vec<String> = Vec::new();
        for lang in std::fs::read_dir(&dir).unwrap().flatten().filter(|e| e.path().is_dir()) {
            let l = lang.file_name().to_string_lossy().to_string();
            if l == "reference" || l == "images" || l.len() > 7 {
                continue;
            }
            for page in std::fs::read_dir(lang.path()).unwrap().flatten() {
                let n = page.file_name().to_string_lossy().to_string();
                if n.ends_with(".md") {
                    on_disk.push(format!("{l}/{n}"));
                }
            }
        }
        on_disk.sort();
        let mut listed: Vec<String> = TRANSLATED.iter().map(|(l, p, _)| format!("{l}/{p}")).collect();
        listed.sort();
        assert_eq!(listed, on_disk);
        for (_, page, _) in TRANSLATED {
            assert!(PAGES.iter().any(|(p, _)| p == page), "{page} translates no page of the manual");
        }
    }

    #[test]
    fn the_user_s_language_comes_first() {
        let (text, translated) = page_in("getting-started.md", "es-UY").unwrap();
        assert!(translated && text.contains("# Primeros pasos"));
        // (every guide page is in Spanish now; a language with none reads the English)
        let (text, translated) = page_in("language.md", "es").unwrap();
        assert!(translated && text.contains("# El lenguaje"));
        let (text, translated) = page_in("language.md", "pt").unwrap();
        assert!(!translated && text.starts_with("# "));
        assert!(page_in("getting-started.md", "en").is_some_and(|(_, t)| !t));
        let top = search_in("instalar el SDK", 3, "es");
        assert_eq!(top[0].page, "es/getting-started.md", "{:?}", top.iter().map(|s| &s.page).collect::<Vec<_>>());
    }

    #[test]
    fn questions_find_their_sections() {
        assert!(sections().len() > 100, "{}", sections().len());
        let top = search("how do I debug with breakpoints", 3);
        assert!(top.iter().any(|s| s.page == "debugging.md"), "{:?}", top.iter().map(|s| (&s.page, &s.heading)).collect::<Vec<_>>());
        let top = search("serial port baud rate", 3);
        assert_eq!(top[0].page, "serial-ports.md");
        assert!(search("zzqx", 3).is_empty());
        let e = excerpt(top[0]);
        assert!(e.starts_with("Manual › ") && e.chars().count() <= SECTION_CHARS + 200, "{e}");
    }
}
