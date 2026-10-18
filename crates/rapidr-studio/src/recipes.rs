//! Recipes: short working programs the assistant starts from
//! (docs/small-models-plan.md §1). A small model is bad at inventing a
//! program and good at adapting one, so when the user asks for a new
//! program Studio finds the closest recipe itself — BM25 over each
//! recipe's header (its title and keywords, English, Spanish and
//! Portuguese) and code, no model, no download — and sends it with the
//! question, as data. `find_example` finds one later in a conversation.
//!
//! The recipes are `examples/recipes/*.rr`: idiomatic RapidR (RapidR's
//! names, DECLARE SUB, Align, ShowModal), each clicked by the examples
//! suite (tests/examples_run.mjs). Built in, so the web Studio has them too.

use std::collections::HashMap;

/// A recipe: its file, title and keywords (its header's `' Recipe:` and
/// `' Keywords:` lines) and its text.
#[derive(Debug)]
pub struct Recipe {
    pub file: &'static str,
    pub title: String,
    pub keywords: String,
    pub text: &'static str,
}

const SOURCES: &[(&str, &str)] = &[
    ("calculator.rr", include_str!("../../../examples/recipes/calculator.rr")),
    ("tip_calculator.rr", include_str!("../../../examples/recipes/tip_calculator.rr")),
    ("stopwatch.rr", include_str!("../../../examples/recipes/stopwatch.rr")),
    ("todo_list.rr", include_str!("../../../examples/recipes/todo_list.rr")),
    ("unit_converter.rr", include_str!("../../../examples/recipes/unit_converter.rr")),
    ("quiz_game.rr", include_str!("../../../examples/recipes/quiz_game.rr")),
    ("drawing_pad.rr", include_str!("../../../examples/recipes/drawing_pad.rr")),
    ("password_generator.rr", include_str!("../../../examples/recipes/password_generator.rr")),
    ("csv_viewer.rr", include_str!("../../../examples/recipes/csv_viewer.rr")),
    ("server_manager.rr", include_str!("../../../examples/recipes/server_manager.rr")),
];

impl Recipe {
    /// Its text as a model gets it: without the search's lines (`' Recipe:`,
    /// `' Keywords:`), which a model would copy into the user's program.
    pub fn code(&self) -> String {
        self.text.lines().filter(|l| !l.starts_with("' Recipe:") && !l.starts_with("' Keywords:")).collect::<Vec<_>>().join("\n").trim().to_string()
    }
}

/// Every recipe.
pub fn all() -> Vec<Recipe> {
    SOURCES
        .iter()
        .map(|(file, text)| {
            let field = |key: &str| text.lines().find_map(|l| l.strip_prefix(&format!("' {key}:")).map(|v| v.trim().to_string())).unwrap_or_default();
            Recipe { file, title: field("Recipe"), keywords: field("Keywords"), text }
        })
        .collect()
}

/// Lower case, accents gone (cronómetro → cronometro), so English, Spanish
/// and Portuguese words meet.
fn fold(c: char) -> char {
    match c {
        'á' | 'à' | 'â' | 'ã' | 'ä' => 'a',
        'é' | 'è' | 'ê' | 'ë' => 'e',
        'í' | 'ì' | 'î' | 'ï' => 'i',
        'ó' | 'ò' | 'ô' | 'õ' | 'ö' => 'o',
        'ú' | 'ù' | 'û' | 'ü' => 'u',
        'ç' => 'c',
        'ñ' => 'n',
        c => c,
    }
}

const STOP: &[&str] = &[
    "a", "an", "the", "me", "my", "i", "you", "it", "is", "are", "be", "to", "of", "and", "or", "for", "in", "on", "with", "that", "this", "please", "can", "could", "would", "want", "need", "like", "some", "build", "make", "create", "write", "app", "application", "program", "simple", "small", "basic", "gui", "window", "rapidr", "un", "una", "el", "la", "los", "las", "de", "del", "con", "para", "por", "que", "y", "o", "en", "mi", "hazme", "haz", "crea", "creame", "quiero", "programa", "aplicacion", "um", "uma", "o", "os", "as", "do", "da", "com", "e", "faz", "faca", "cria", "crie", "quero", "aplicativo", "aplicacao",
];

/// A text's words, folded, the common ones gone, a plural's -s dropped.
pub fn words(text: &str) -> Vec<String> {
    text.to_lowercase()
        .chars()
        .map(fold)
        .collect::<String>()
        .split(|c: char| !c.is_alphanumeric())
        .filter(|w| !w.is_empty() && !STOP.contains(w))
        .map(|w| if w.len() > 3 && w.ends_with('s') && !w.ends_with("ss") { w[..w.len() - 1].to_string() } else { w.to_string() })
        .collect()
}

/// A recipe's words for the search: its header (title, keywords, what it
/// shows) three times over — what it *is* weighs more than what it uses —
/// and its code once.
fn doc_words(r: &Recipe) -> Vec<String> {
    let header: String = r.text.lines().take_while(|l| l.starts_with('\'')).collect::<Vec<_>>().join(" ");
    let head = words(&header);
    let mut out = Vec::new();
    for _ in 0..3 {
        out.extend(head.iter().cloned());
    }
    out.extend(words(r.text));
    out
}

/// The recipes for `query`, best first, with their BM25 scores (only those
/// that share a word with it).
pub fn search(query: &str) -> Vec<(f64, Recipe)> {
    let recipes = all();
    let docs: Vec<Vec<String>> = recipes.iter().map(doc_words).collect();
    let n = docs.len() as f64;
    let avg = docs.iter().map(Vec::len).sum::<usize>() as f64 / n.max(1.0);
    let mut q = words(query);
    q.dedup();
    let (k1, b) = (1.2, 0.75);
    let mut scored: Vec<(f64, Recipe)> = recipes
        .into_iter()
        .zip(&docs)
        .map(|(r, d)| {
            let mut tf: HashMap<&str, f64> = HashMap::new();
            for w in d {
                *tf.entry(w.as_str()).or_default() += 1.0;
            }
            let score: f64 = q
                .iter()
                .map(|t| {
                    let df = docs.iter().filter(|d| d.iter().any(|w| w == t)).count() as f64;
                    let idf = ((n - df + 0.5) / (df + 0.5) + 1.0).ln();
                    let f = tf.get(t.as_str()).copied().unwrap_or(0.0);
                    idf * f * (k1 + 1.0) / (f + k1 * (1.0 - b + b * d.len() as f64 / avg))
                })
                .sum();
            (score, r)
        })
        .filter(|(s, _)| *s > 0.0)
        .collect();
    scored.sort_by(|a, b| b.0.total_cmp(&a.0));
    scored
}

/// Whether the user asks for a new program ("Build me a stopwatch", "Make
/// a quiz game", "Hazme una calculadora", "Crie um app de tarefas") — not a
/// change to the open one ("Make the total bigger"). With a project open,
/// only when it says app / program / game / new.
pub fn asks_for_new_program(question: &str, project_open: bool) -> bool {
    let q: String = question.to_lowercase().chars().map(fold).collect();
    let w: Vec<&str> = q.split(|c: char| !c.is_alphanumeric()).filter(|w| !w.is_empty()).collect();
    const VERBS: &[&str] = &[
        "build", "make", "create", "write", "code", "develop", "generate", "design", "crea", "crear", "creame", "haz", "hazme", "hacer", "construye", "construir", "construyeme", "programa", "programame", "desarrolla", "escribe", "escribeme", "genera", "cria", "crie", "criar", "faca", "faz", "fazer", "construa", "desenvolva", "escreva", "gere",
    ];
    const WANT: &[&str] = &["want", "need", "quiero", "necesito", "quero", "preciso"];
    const ARTICLES: &[&str] = &["me", "a", "an", "un", "una", "um", "uma", "my", "mi", "meu", "minha", "new", "nuevo", "nueva", "novo", "nova"];
    const NOUNS: &[&str] = &["app", "apps", "application", "program", "programa", "aplicacion", "aplicativo", "aplicacao", "tool", "herramienta", "ferramenta", "game", "juego", "jogo", "utility", "calculator", "calculadora"];
    let early = &w[..w.len().min(6)];
    let verb = early.iter().position(|x| VERBS.contains(x) || WANT.contains(x));
    let Some(at) = verb else { return false };
    let after = &w[at + 1..];
    let noun = w.iter().any(|x| NOUNS.contains(x));
    let new = w.iter().any(|x| ["new", "nuevo", "nueva", "novo", "nova", "separate", "another"].contains(x));
    if project_open {
        return noun && (new || after.first().is_some_and(|x| ARTICLES.contains(x)));
    }
    after.first().is_some_and(|x| ARTICLES.contains(x)) || noun
}

/// What Studio sends with a request for a new program: the closest
/// recipe (two when both are short and the second is nearly as close).
pub fn context_for(question: &str) -> Option<String> {
    let found = search(question);
    let (best, first) = found.first()?;
    let mut out = format!("(RapidR Studio, not the user: a working RapidR program close to what the user wants — start from it and adapt it; keep what works: its layout, its DECLARE SUBs, its handlers, Form.ShowModal at the end.)\n\nRecipe \"{}\" (its file in the new project: main.rr):\n```basic\n{}\n```", first.title, first.code());
    if let Some((second, r)) = found.get(1) {
        if *second >= best * 0.75 && first.text.len() + r.text.len() < 6000 {
            out.push_str(&format!("\n\nAnother close one, recipe \"{}\":\n```basic\n{}\n```", r.title, r.code()));
        }
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn top(q: &str) -> &'static str {
        search(q).first().map_or("", |(_, r)| r.file)
    }

    #[test]
    fn every_recipe_has_a_header_and_fits() {
        let all = all();
        assert!(all.len() >= 10);
        for r in &all {
            assert!(!r.title.is_empty() && r.keywords.split(',').count() >= 6, "{}: its header", r.file);
            assert!(r.text.lines().count() <= 130, "{}: {} lines", r.file, r.text.lines().count());
            assert!(r.text.contains("ShowModal") && r.text.contains("$THEME rapidr"), "{}", r.file);
        }
    }

    #[test]
    fn the_closest_recipe() {
        assert_eq!(top("Build me a scientific calculator app"), "calculator.rr");
        assert_eq!(top("Regular scientific calculator. GUI."), "calculator.rr");
        assert_eq!(top("Build me a tip calculator"), "tip_calculator.rr");
        assert_eq!(top("Build me a stopwatch"), "stopwatch.rr");
        assert_eq!(top("Build me a to-do list app where I can add and remove tasks"), "todo_list.rr");
        assert_eq!(top("Build me a unit converter from kilometers to miles"), "unit_converter.rr");
        assert_eq!(top("Build me a quiz game: three questions about world capitals, each with answer buttons, and show the score"), "quiz_game.rr");
        assert_eq!(top("Build me a drawing pad: draw with the mouse, pick a colour, and a Clear button"), "drawing_pad.rr");
        assert_eq!(top("Build me a password generator with a length setting"), "password_generator.rr");
        assert_eq!(top("Build me a CSV viewer: open a CSV file and show it in a grid"), "csv_viewer.rr");
        assert_eq!(top("Let's build a web server manager with a GUI. It needs Start, Restart, Stop, Settings and Sites buttons"), "server_manager.rr");
        // (Spanish, Portuguese)
        assert_eq!(top("Hazme una calculadora científica"), "calculator.rr");
        assert_eq!(top("Crie um cronômetro"), "stopwatch.rr");
        assert_eq!(top("Quiero una lista de tareas"), "todo_list.rr");
        // (nothing alike: nothing)
        assert!(search("zzz qqq").is_empty());
    }

    #[test]
    fn only_requests_for_a_new_program() {
        for q in ["Build me a scientific calculator app", "Build me a stopwatch", "Make a quiz game", "Create an app that converts units", "I want a to-do list app", "Hazme una calculadora", "Crie um app de tarefas", "Write a program that generates passwords"] {
            assert!(asks_for_new_program(q, false), "{q}");
        }
        for q in ["Make the total bigger and bold", "Add a split-between-people field", "What does main.rr do?", "Regular scientific calculator. GUI.", "Fix the errors", "continue", "Make it blue"] {
            assert!(!asks_for_new_program(q, false), "{q}");
        }
        // (with a project open: a new app, said so)
        assert!(asks_for_new_program("Build me a new app: a stopwatch", true));
        assert!(asks_for_new_program("Make a separate program for the reports", true));
        assert!(!asks_for_new_program("Make a button that clears the list", true));
        assert!(!asks_for_new_program("Create a settings dialog", true));
    }

    #[test]
    fn the_context_is_the_recipe_as_data() {
        let c = context_for("Build me a scientific calculator app").unwrap();
        assert!(c.starts_with("(RapidR Studio, not the user: a working RapidR program close to what the user wants"), "{c}");
        assert!(c.contains("Recipe \"Calculator (basic and scientific)\" (its file in the new project: main.rr)") && !c.contains("examples/recipes") && c.contains("SUB KeyPress (Sender AS RButton)"), "{c}");
        // (about 2K tokens at most: a small model's window is 8K and more)
        assert!(c.len() < 9000, "{}", c.len());
    }
}
