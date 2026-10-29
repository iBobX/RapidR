//! Recipes: short working programs the assistant starts from
//! (docs/small-models-plan.md §1). A small model is bad at inventing a
//! program and good at adapting one, so when the user asks for a new
//! program Studio finds the closest recipe itself — BM25 over each
//! recipe's header (its title and keywords, English, Spanish and
//! Portuguese; what it *is*) with its code only a tie-breaker (what it
//! uses), no model, no download — and sends it with the question, as
//! data. A request no recipe's title or keywords share a word with, or only
//! weakly, gets none: no example beats a wrong one. `find_example` finds
//! one later in a conversation.
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
    // (the systems recipes: one tiny machine told in ten small programs —
    // docs/manual/recipes-systems.md)
    ("memory_bytes.rr", include_str!("../../../examples/recipes/memory_bytes.rr")),
    ("flags_bits.rr", include_str!("../../../examples/recipes/flags_bits.rr")),
    ("cpu_step.rr", include_str!("../../../examples/recipes/cpu_step.rr")),
    ("cpu_stack.rr", include_str!("../../../examples/recipes/cpu_stack.rr")),
    ("hex_dump.rr", include_str!("../../../examples/recipes/hex_dump.rr")),
    ("disassembler.rr", include_str!("../../../examples/recipes/disassembler.rr")),
    ("load_binary.rr", include_str!("../../../examples/recipes/load_binary.rr")),
    ("video_memory.rr", include_str!("../../../examples/recipes/video_memory.rr")),
    ("keyboard_matrix.rr", include_str!("../../../examples/recipes/keyboard_matrix.rr")),
    ("cpu_timing.rr", include_str!("../../../examples/recipes/cpu_timing.rr")),
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

/// A text with its accents folded ([`fold`]).
pub fn fold_text(text: &str) -> String {
    text.chars().map(fold).collect()
}

const STOP: &[&str] = &[
    "a", "an", "the", "me", "my", "i", "you", "it", "is", "are", "be", "to", "of", "and", "or", "for", "in", "on", "with", "that", "this", "please", "can", "could", "would", "want", "need", "like", "some", "build", "make", "create", "write", "app", "application", "program", "simple", "small", "basic", "gui", "window", "rapidr", "un", "una", "el", "la", "los", "las", "de", "del", "con", "para", "por", "que", "y", "o", "en", "mi", "hazme", "haz", "crea", "creame", "quiero", "programa", "aplicacion", "um", "uma", "o", "os", "as", "do", "da", "com", "e", "faz", "faca", "cria", "crie", "quero", "aplicativo", "aplicacao",
    // (generic request words: they say nothing about which program)
    "just", "also", "using", "use", "show", "add", "function", "feature", "thing", "something", "stuff", "any", "all", "more", "new", "good", "nice", "really", "very", "etc", "tool",
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

/// A recipe's header words for the search: its title, keywords and what it
/// shows — what it *is*.
fn header_words(r: &Recipe) -> Vec<String> {
    words(&r.text.lines().take_while(|l| l.starts_with('\'')).collect::<Vec<_>>().join(" "))
}

/// BM25 of each document in `docs` for the terms `q`.
fn bm25(docs: &[Vec<String>], q: &[String]) -> Vec<f64> {
    let n = docs.len() as f64;
    let avg = docs.iter().map(Vec::len).sum::<usize>() as f64 / n.max(1.0);
    let (k1, b) = (1.2, 0.75);
    docs.iter()
        .map(|d| {
            let mut tf: HashMap<&str, f64> = HashMap::new();
            for w in d {
                *tf.entry(w.as_str()).or_default() += 1.0;
            }
            q.iter()
                .map(|t| {
                    let df = docs.iter().filter(|d| d.iter().any(|w| w == t)).count() as f64;
                    let idf = ((n - df + 0.5) / (df + 0.5) + 1.0).ln();
                    let f = tf.get(t.as_str()).copied().unwrap_or(0.0);
                    idf * f * (k1 + 1.0) / (f + k1 * (1.0 - b + b * d.len() as f64 / avg))
                })
                .sum()
        })
        .collect()
}

/// What the code's words count for next to the header's: a tie-breaker (a
/// "generate" in some recipe's code says little of what it is).
const CODE_WEIGHT: f64 = 0.15;

/// The least a recipe's header must score for a request to count as asking
/// for it (one keyword of a recipe among ten, about 1.5; a lone generic
/// word shared with a long header, under 1).
const LEAST_SCORE: f64 = 1.2;

/// The recipes for `query`, best first, with their scores: BM25 over the
/// headers (title, keywords) plus a little of the code's. Only those whose
/// header shares a word with the request, and clearly: a request that
/// matches no title or keyword (a poem, a word only code has) gets none.
pub fn search(query: &str) -> Vec<(f64, Recipe)> {
    let recipes = all();
    let heads: Vec<Vec<String>> = recipes.iter().map(header_words).collect();
    let codes: Vec<Vec<String>> = recipes.iter().map(|r| words(r.text)).collect();
    // (each word once: a word said again and again weighs no more)
    let mut q: Vec<String> = Vec::new();
    for w in words(query) {
        if !q.contains(&w) {
            q.push(w);
        }
    }
    let (head, code) = (bm25(&heads, &q), bm25(&codes, &q));
    let mut scored: Vec<(f64, Recipe)> = recipes.into_iter().zip(head.into_iter().zip(code)).filter(|(_, (h, _))| *h >= LEAST_SCORE).map(|(r, (h, c))| (h + CODE_WEIGHT * c, r)).collect();
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

/// How the recipe sent with a question is labelled: Studio's, never the
/// user's — a small model thanked the user for "the recipe you provided"
/// (Robert's walk, 2026-10-18).
pub const RECIPE_LABEL: &str = "(An example RapidR Studio found — not from the user. Don't mention it unless asked.)";

/// What Studio sends with a request for a new program: the closest
/// recipe (two when both are short and the second is nearly as close).
pub fn context_for(question: &str) -> Option<String> {
    context_and_title(question).map(|(c, _)| c)
}

/// [`context_for`]'s text, and the title of the recipe it starts from
/// (the chat's card says which).
pub fn context_and_title(question: &str) -> Option<(String, String)> {
    let found = search(question);
    let (best, first) = found.first()?;
    let mut out = format!("{RECIPE_LABEL} A working RapidR program close to what the user wants: start from it and adapt it; keep what works — its layout, its DECLARE SUBs, its handlers, Form.ShowModal at the end.\n\nExample \"{}\" (its file in the new project: main.rr):\n```basic\n{}\n```", first.title, first.code());
    if let Some((second, r)) = found.get(1) {
        if *second >= best * 0.75 && first.text.len() + r.text.len() < 6000 {
            out.push_str(&format!("\n\nAnother close one, example \"{}\":\n```basic\n{}\n```", r.title, r.code()));
        }
    }
    Some((out, first.title.clone()))
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
        assert!(all.len() >= 20);
        for r in &all {
            assert!(!r.title.is_empty() && r.keywords.split(',').count() >= 6, "{}: its header", r.file);
            // (130 lines for an app; the systems recipes — a CPU, a screen
            // — are the longest, and a small model's window holds 150)
            assert!(r.text.lines().count() <= 150, "{}: {} lines", r.file, r.text.lines().count());
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

    /// The systems recipes (docs/manual/recipes-systems.md): what an agent
    /// building a machine asks, in English, Spanish and Portuguese, finds
    /// the one technique it needs.
    #[test]
    fn the_systems_recipes_are_found_by_what_they_do() {
        for (q, file) in [
            // (the CPU)
            ("emulador cpu registros", "cpu_step.rr"),
            ("write a tiny CPU emulator with registers and a program counter", "cpu_step.rr"),
            ("fetch decode execute loop with SELECT CASE opcodes", "cpu_step.rr"),
            ("emulador de CPU con contador de programa", "cpu_step.rr"),
            ("processador registradores instrução", "cpu_step.rr"),
            // (an agent's first words for a machine: the CPU leads, the
            // technique it then needs is found on its own words)
            ("8-bit CPU emulator", "cpu_step.rr"),
            ("how do I emulate an 8 bit computer", "cpu_step.rr"),
            ("emulador de ordenador de 8 bits con juegos", "cpu_step.rr"),
            ("instruction decoder", "cpu_step.rr"),
            ("debugger showing registers and memory", "cpu_step.rr"),
            ("build a debugger for my CPU", "disassembler.rr"),
            ("keyboard for the emulated computer", "keyboard_matrix.rr"),
            ("show the screen of the emulated machine", "video_memory.rr"),
            ("load a snapshot file into the emulator memory", "load_binary.rr"),
            ("ROM loader", "load_binary.rr"),
            ("game loop at 50 frames per second", "cpu_timing.rr"),
            // (the stack)
            ("stack push pop", "cpu_stack.rr"),
            ("how do I do CALL and RET with a stack pointer", "cpu_stack.rr"),
            ("pila puntero de pila apilar desapilar", "cpu_stack.rr"),
            ("pilha empilhar desempilhar sub-rotina", "cpu_stack.rr"),
            // (memory)
            ("memory as a byte array with peek and poke", "memory_bytes.rr"),
            ("leer y escribir una palabra de 16 bits en memoria", "memory_bytes.rr"),
            ("endereço e memória little-endian", "memory_bytes.rr"),
            // (bits and flags)
            ("flags register carry zero overflow after add", "flags_bits.rr"),
            ("banderas acarreo desbordamiento rotar bits", "flags_bits.rr"),
            ("signed and unsigned bytes, rotate left through carry", "flags_bits.rr"),
            // (a viewer)
            ("memory hex dump", "hex_dump.rr"),
            ("visor de memoria volcado hexadecimal", "hex_dump.rr"),
            ("hex editor with an ASCII column", "hex_dump.rr"),
            // (a disassembler)
            ("disassembler opcode table", "disassembler.rr"),
            ("desensamblador tabla de opcodes mnemonico", "disassembler.rr"),
            // (a file)
            ("read binary file header", "load_binary.rr"),
            ("load a binary file into the memory array and parse its header", "load_binary.rr"),
            ("cargar archivo binario cabecera", "load_binary.rr"),
            // (the screen)
            ("draw video memory on canvas", "video_memory.rr"),
            ("framebuffer bitmap with colour attributes on an RCanvas", "video_memory.rr"),
            ("memoria de video pantalla atributos de color", "video_memory.rr"),
            // (the keyboard)
            ("keyboard matrix key down key up", "keyboard_matrix.rr"),
            ("teclado matriz de teclado puerto de entrada", "keyboard_matrix.rr"),
            // (time)
            ("run cycles per frame from a timer", "cpu_timing.rr"),
            ("emulation speed 50 Hz frame budget pause and step", "cpu_timing.rr"),
        ] {
            assert_eq!(top(q), file, "{q}");
        }
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
        let (c, title) = context_and_title("Build me a scientific calculator app").unwrap();
        assert_eq!(title, "Calculator (basic and scientific)");
        // (labelled as Studio's, never the user's: a small model thanked
        // the user for "the recipe you provided")
        assert!(c.starts_with("(An example RapidR Studio found — not from the user. Don't mention it unless asked.) A working RapidR program close to what the user wants"), "{c}");
        assert!(!c.to_lowercase().contains("recipe") && !c.contains("you provided"), "{c}");
        assert!(c.contains("Example \"Calculator (basic and scientific)\" (its file in the new project: main.rr)") && !c.contains("examples/recipes") && c.contains("SUB KeyPress (Sender AS RButton)"), "{c}");
        // (about 2K tokens at most: a small model's window is 8K and more)
        assert!(c.len() < 9000, "{}", c.len());
    }

    #[test]
    fn titles_and_keywords_decide_not_the_words_a_request_always_has() {
        // (Robert's walk, 2026-10-18: "functions" and "graphs" sent the
        // Password generator, whose code says "generate" and "functions")
        assert_eq!(top("Just make a scientific calculator with functions and graphs"), "calculator.rr");
        assert_eq!(top("scientific calculator with graphs"), "calculator.rr");
        assert_eq!(top("password"), "password_generator.rr");
        assert_eq!(top("a web server manager"), "server_manager.rr");
        assert_eq!(top("generate random passwords with symbols"), "password_generator.rr");
        // (nothing close: no example, rather than a wrong one — a code-only
        // word, a request about no recipe)
        assert!(search("a poem").is_empty());
        // (a long request: a word said again and again counts once)
        let long = format!("Build a scientific calculator. {}", "alpha bravo charlie delta echo foxtrot golf hotel india juliet kilo lima mike november oscar papa ".repeat(14));
        assert_eq!(top(&long), "calculator.rr");
        assert!(context_and_title("a poem").is_none());
        assert!(search("just make a thing with functions and graphs").is_empty(), "only words every request has");
    }
}
