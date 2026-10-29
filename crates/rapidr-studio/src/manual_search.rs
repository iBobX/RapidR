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
    ("archives.md", include_str!("../../../docs/manual/archives.md")),
    ("building-apps.md", include_str!("../../../docs/manual/building-apps.md")),
    ("cli-and-runtime.md", include_str!("../../../docs/manual/cli-and-runtime.md")),
    ("code-editor.md", include_str!("../../../docs/manual/code-editor.md")),
    ("components.md", include_str!("../../../docs/manual/components.md")),
    ("data-science-with-rapidr.md", include_str!("../../../docs/manual/data-science-with-rapidr.md")),
    ("data-science.md", include_str!("../../../docs/manual/data-science.md")),
    ("databases.md", include_str!("../../../docs/manual/databases.md")),
    ("debugging.md", include_str!("../../../docs/manual/debugging.md")),
    ("designing-forms.md", include_str!("../../../docs/manual/designing-forms.md")),
    ("differences.md", include_str!("../../../docs/manual/differences.md")),
    ("directx-and-media.md", include_str!("../../../docs/manual/directx-and-media.md")),
    ("getting-started.md", include_str!("../../../docs/manual/getting-started.md")),
    ("importing-rapidq.md", include_str!("../../../docs/manual/importing-rapidq.md")),
    ("language.md", include_str!("../../../docs/manual/language.md")),
    ("markdown.md", include_str!("../../../docs/manual/markdown.md")),
    ("network.md", include_str!("../../../docs/manual/network.md")),
    ("other-editors.md", include_str!("../../../docs/manual/other-editors.md")),
    ("recipes-systems.md", include_str!("../../../docs/manual/recipes-systems.md")),
    ("rich-text.md", include_str!("../../../docs/manual/rich-text.md")),
    ("secrets.md", include_str!("../../../docs/manual/secrets.md")),
    ("serial-ports.md", include_str!("../../../docs/manual/serial-ports.md")),
    ("showcase.md", include_str!("../../../docs/manual/showcase.md")),
    ("studio-editor.md", include_str!("../../../docs/manual/studio-editor.md")),
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

/// The `n` sections that answer `query` best (none when no word of it is
/// in the manual).
pub fn search(query: &str, n: usize) -> Vec<&'static Section> {
    let all = sections();
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
    format!("docs/manual/{} — {}\n{}", s.page, s.heading, text.trim_end())
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
    fn questions_find_their_sections() {
        assert!(sections().len() > 100, "{}", sections().len());
        let top = search("how do I debug with breakpoints", 3);
        assert!(top.iter().any(|s| s.page == "debugging.md"), "{:?}", top.iter().map(|s| (&s.page, &s.heading)).collect::<Vec<_>>());
        let top = search("serial port baud rate", 3);
        assert_eq!(top[0].page, "serial-ports.md");
        assert!(search("zzqx", 3).is_empty());
        let e = excerpt(top[0]);
        assert!(e.starts_with("docs/manual/serial-ports.md — ") && e.chars().count() <= SECTION_CHARS + 200, "{e}");
    }
}
