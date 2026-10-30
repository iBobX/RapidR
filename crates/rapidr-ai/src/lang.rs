//! The language a message is written in, so an assistant replies in it
//! (docs/preview4-plan.md R10: two answers came in German to a user who
//! always wrote English, after the conversation was summarised).
//!
//! Small on purpose: the script a text is written in (Chinese, Japanese,
//! Korean, Cyrillic, Arabic, Hebrew, Greek, Devanagari, Thai), else the
//! common short words of the Latin-script languages people write to RapidR
//! in (English, Spanish, Portuguese, French, German, Italian, Dutch). Code
//! and pasted output (fenced blocks, `inline code`, links, paths) are left
//! out first. When it isn't sure ("ok", "do it?", a line of code) it says
//! nothing, and the caller keeps the language it had.
//!
//! RapidR Studio's assistant answers in **Studio's language** (Robert,
//! 2026-10-30), unless the person clearly writes in another: that decision
//! is [`reply_language`], a pure function of Studio's language tag, the
//! setting "RAI answers in:" ([`ReplyIn`]) and the latest message.

/// The words that tell a language apart (lower case; words two languages
/// share — "de", "la", "en", "que", "como", "es" — are in neither list).
const WORDS: &[(&str, &[&str])] = &[
    (
        "English",
        &[
            "the", "and", "is", "are", "you", "it", "this", "that", "with", "for", "not", "but", "what", "how", "can", "please", "add", "make", "why", "does", "my", "when", "where", "there", "would", "should", "could", "have", "has", "of", "to", "at", "be", "an", "if", "then", "hello", "thanks", "thank", "yes", "button", "window", "change", "fix", "now", "also", "just", "i'm", "it's", "don't", "doesn't", "can't",
        ],
    ),
    (
        "Spanish",
        &[
            "el", "los", "las", "por", "pero", "muy", "más", "qué", "cómo", "también", "hay", "yo", "sí", "gracias", "hola", "hacer", "puedes", "quiero", "ahora", "botón", "ventana", "cuando", "donde", "dónde", "sin", "esto", "eso", "ese", "esa", "y", "agrega", "añade", "cambia", "arregla", "errores", "archivo", "explica", "dime", "necesito", "tengo", "puede", "hazme", "dame", "muéstrame", "muestrame", "construye", "crea", "nuevo", "nueva", "botones", "propinas", "ayuda", "ayúdame", "escribe", "calcula",
        ],
    ),
    (
        "Portuguese",
        &[
            "não", "você", "isso", "isto", "um", "uma", "em", "na", "nas", "ao", "é", "olá", "obrigado", "obrigada", "também", "então", "fazer", "botão", "janela", "ele", "ela", "seu", "sua", "muito", "eu", "quero", "agora", "arquivo", "adicione", "mude", "corrija", "preciso", "tenho", "pode",
        ],
    ),
    (
        "French",
        &[
            "le", "des", "et", "est", "pour", "pas", "vous", "elle", "nous", "avec", "dans", "sur", "ce", "cette", "c'est", "oui", "merci", "bonjour", "fenêtre", "bouton", "faire", "peux", "pouvez", "pourquoi", "aussi", "très", "fichier", "ajoute", "ajouter", "erreur", "j'ai", "qu'il", "moi",
        ],
    ),
    (
        "German",
        &[
            "der", "die", "das", "und", "ist", "nicht", "ein", "eine", "ich", "mit", "für", "auf", "zu", "den", "dem", "warum", "bitte", "danke", "hallo", "kannst", "machen", "auch", "noch", "oder", "aber", "sind", "wird", "knopf", "fenster", "nein", "fehler", "waren", "zwei", "datei", "füge", "hinzu", "mach", "jetzt",
        ],
    ),
    (
        "Italian",
        &[
            "gli", "della", "delle", "di", "che", "è", "non", "sono", "questo", "questa", "perché", "anche", "grazie", "ciao", "fare", "puoi", "pulsante", "finestra", "molto", "aggiungi", "errore", "voglio", "adesso",
        ],
    ),
    (
        "Dutch",
        &[
            "het", "een", "van", "niet", "ik", "met", "voor", "op", "zijn", "maar", "wat", "hoe", "waarom", "alsjeblieft", "bedankt", "dank", "knop", "venster", "ook", "nog", "kun", "kunt", "maken", "bestand", "fout", "voeg",
        ],
    ),
];

/// Each language's own name for itself, said with its English name to a
/// model ("Spanish (español)").
const NATIVE: &[(&str, &str)] = &[
    ("Spanish", "español"),
    ("Portuguese", "português"),
    ("French", "français"),
    ("German", "Deutsch"),
    ("Italian", "italiano"),
    ("Dutch", "Nederlands"),
    ("Chinese", "中文"),
    ("Japanese", "日本語"),
    ("Korean", "한국어"),
    ("Russian", "русский"),
    ("Ukrainian", "українська"),
    ("Arabic", "العربية"),
    ("Hebrew", "עברית"),
    ("Greek", "Ελληνικά"),
    ("Hindi", "हिन्दी"),
    ("Thai", "ไทย"),
    ("Polish", "polski"),
    ("Turkish", "Türkçe"),
    ("Swedish", "svenska"),
];

/// The language `text` is written in (its English name), or `None` when
/// it can't tell.
pub fn detect(text: &str) -> Option<&'static str> {
    let prose = prose_of(text);
    // (a script of its own: decided by its letters)
    let mut latin = 0usize;
    let mut scripts: Vec<(&'static str, usize)> = Vec::new();
    let mut ukrainian = false;
    for c in prose.chars() {
        let Some(s) = script(c) else {
            if c.is_alphabetic() {
                latin += 1;
            }
            continue;
        };
        if matches!(c, 'і' | 'ї' | 'є' | 'ґ' | 'І' | 'Ї' | 'Є' | 'Ґ') {
            ukrainian = true;
        }
        match scripts.iter_mut().find(|(n, _)| *n == s) {
            Some(e) => e.1 += 1,
            None => scripts.push((s, 1)),
        }
    }
    let other: usize = scripts.iter().map(|(_, n)| n).sum();
    if other > 0 && other * 3 >= (other + latin) {
        let has = |s: &str| scripts.iter().any(|(n, _)| *n == s);
        // (kana with the Han characters: Japanese)
        if has("Japanese") {
            return Some("Japanese");
        }
        let (best, _) = scripts.iter().max_by_key(|(_, n)| *n)?;
        return Some(match *best {
            "Cyrillic" if ukrainian => "Ukrainian",
            "Cyrillic" => "Russian",
            s => s,
        });
    }
    // (the Latin-script languages: their short words, and a few letters
    // only one of them writes)
    let low = prose.to_lowercase().replace(['\u{2019}', '\u{2018}'], "'");
    let words: Vec<&str> = low.split(|c: char| !(c.is_alphabetic() || c == '\'')).map(|w| w.trim_matches('\'')).filter(|w| !w.is_empty()).collect();
    if words.is_empty() {
        return None;
    }
    let mut scores: Vec<(&'static str, usize)> = WORDS.iter().map(|(lang, list)| (*lang, words.iter().filter(|w| list.contains(w)).count())).collect();
    let mut bump = |lang: &str, chars: &[char]| {
        if low.contains(chars) {
            if let Some(e) = scores.iter_mut().find(|(l, _)| *l == lang) {
                e.1 += 1;
            }
        }
    };
    bump("Spanish", &['¿', '¡', 'ñ']);
    bump("Portuguese", &['ã', 'õ']);
    bump("German", &['ß', 'ä', 'ö', 'ü']);
    scores.sort_by(|a, b| b.1.cmp(&a.1));
    let (lang, best) = scores[0];
    let second = scores[1].1;
    // (sure: two signs or more, at least twice the next language's — or,
    // in a message of a few words, one sign and no other language's)
    let sure = (best >= 2 && best >= 2 * second) || (best == 1 && second == 0 && words.len() <= 4);
    sure.then_some(lang)
}

/// The text without what isn't the writer's own words: fenced code blocks,
/// `inline code`, links and paths.
fn prose_of(text: &str) -> String {
    let mut out = String::new();
    // (what is between ``` fences, wherever they are, is code)
    for segment in text.split("```").step_by(2) {
        for line in segment.lines() {
            for piece in line.split('`').step_by(2) {
                for word in piece.split_whitespace() {
                    if !(word.contains("://") || word.contains('/') || word.contains('\\') || word.contains('=')) {
                        out.push_str(word);
                        out.push(' ');
                    }
                }
            }
            out.push('\n');
        }
    }
    out
}

/// The script a letter belongs to, when it isn't Latin.
fn script(c: char) -> Option<&'static str> {
    Some(match c as u32 {
        0x3040..=0x30FF | 0x31F0..=0x31FF => "Japanese",
        0x4E00..=0x9FFF | 0x3400..=0x4DBF | 0xF900..=0xFAFF => "Chinese",
        0xAC00..=0xD7AF | 0x1100..=0x11FF | 0x3130..=0x318F => "Korean",
        0x0400..=0x04FF => "Cyrillic",
        0x0600..=0x06FF | 0x0750..=0x077F => "Arabic",
        0x0590..=0x05FF => "Hebrew",
        0x0370..=0x03FF => "Greek",
        0x0900..=0x097F => "Hindi",
        0x0E00..=0x0E7F => "Thai",
        _ => return None,
    })
}

/// A language tag's language ("es-AR" → Spanish), when it is one these
/// know.
pub fn name_of_tag(tag: &str) -> Option<&'static str> {
    let primary = tag.trim().split(['-', '_', '.']).next().unwrap_or("").to_ascii_lowercase();
    Some(match primary.as_str() {
        "en" => "English",
        "es" => "Spanish",
        "pt" => "Portuguese",
        "fr" => "French",
        "de" => "German",
        "it" => "Italian",
        "nl" => "Dutch",
        "zh" => "Chinese",
        "ja" => "Japanese",
        "ko" => "Korean",
        "ru" => "Russian",
        "uk" => "Ukrainian",
        "ar" => "Arabic",
        "he" | "iw" => "Hebrew",
        "el" => "Greek",
        "hi" => "Hindi",
        "th" => "Thai",
        "pl" => "Polish",
        "tr" => "Turkish",
        "sv" => "Swedish",
        _ => return None,
    })
}

/// A language as a model is told it: "Spanish (español)", "English".
pub fn for_model(language: &str) -> String {
    match NATIVE.iter().find(|(l, _)| l.eq_ignore_ascii_case(language)) {
        Some((l, n)) => format!("{l} ({n})"),
        None => language.to_string(),
    }
}

/// A language, by its English name ("Spanish").
pub type Language = &'static str;

/// Where the language of the answers comes from (Tools > Settings > AI,
/// "RAI answers in:").
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum ReplyIn {
    /// Studio's language — the default —, unless the person writes clearly
    /// in another one: then that message is answered in it, and the next
    /// message that says too little ("ok", a line of code) goes back to
    /// Studio's. (The RAI component's ReplyLanguage `"app"`.)
    #[default]
    Studio,
    /// The language the person writes in: the latest message's, kept while
    /// a message says too little; Studio's before the first one that says.
    /// (ReplyLanguage `"auto"`, R10's: the same rule, said by the RAI
    /// itself with the language it has seen.)
    Writer,
}

impl ReplyIn {
    /// The setting as it is kept ("studio" or "writer"): anything else
    /// (an old settings file, nothing) is the default, Studio's language.
    pub fn parse(text: &str) -> ReplyIn {
        if text.trim().eq_ignore_ascii_case("writer") { ReplyIn::Writer } else { ReplyIn::Studio }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            ReplyIn::Studio => "studio",
            ReplyIn::Writer => "writer",
        }
    }
}

/// Studio's language as a language: the name of the tag (`es-AR` →
/// Spanish); English for one this doesn't know (a pseudo-language, a tag
/// not yet translated) — RapidR's own words are English there.
pub fn studio_language(tag: &str) -> Language {
    name_of_tag(tag).unwrap_or("English")
}

/// The language an answer is in: Studio's (`studio_tag`, "es", "es-AR",
/// "") unless the person's latest message is clearly written in another
/// language — then theirs. A message that says too little (*ok*, a line of
/// code) is answered in Studio's language.
pub fn reply_language(studio_tag: &str, setting: ReplyIn, latest: &str) -> Language {
    reply_language_keeping(studio_tag, setting, latest, None)
}

/// [`reply_language`] for a chat that goes on: with [`ReplyIn::Writer`], a
/// message that says too little is answered in the language the person was
/// last seen writing in (`kept`), not Studio's.
pub fn reply_language_keeping(studio_tag: &str, setting: ReplyIn, latest: &str, kept: Option<&str>) -> Language {
    if let Some(written) = detect(latest) {
        return written;
    }
    match (setting, kept.and_then(known)) {
        (ReplyIn::Writer, Some(k)) => k,
        _ => studio_language(studio_tag),
    }
}

/// A language's English name, as this module spells it, when it knows it.
fn known(name: &str) -> Option<Language> {
    WORDS.iter().map(|(l, _)| *l).chain(NATIVE.iter().map(|(l, _)| *l)).find(|l| l.eq_ignore_ascii_case(name.trim()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_writers_language() {
        // (the transcript's: English all evening)
        assert_eq!(detect("and add a Clear button"), Some("English"));
        assert_eq!(detect("The server doesn't start when I press Start, can you fix it?"), Some("English"));
        assert_eq!(detect("Das waren die zwei Fehler, die ich gefunden habe."), Some("German"));
        assert_eq!(detect("¿Puedes añadir un botón para borrar la lista?"), Some("Spanish"));
        assert_eq!(detect("¿y el formulario?"), Some("Spanish"));
        assert_eq!(detect("Você pode adicionar um botão para limpar a lista?"), Some("Portuguese"));
        assert_eq!(detect("Peux-tu ajouter un bouton pour vider la liste ?"), Some("French"));
        assert_eq!(detect("Puoi aggiungere un pulsante che svuota la lista?"), Some("Italian"));
        assert_eq!(detect("Kun je een knop toevoegen die de lijst leegmaakt?"), Some("Dutch"));
        assert_eq!(detect("请添加一个清除按钮"), Some("Chinese"));
        assert_eq!(detect("クリアボタンを追加してください"), Some("Japanese"));
        assert_eq!(detect("지우기 버튼을 추가해 주세요"), Some("Korean"));
        assert_eq!(detect("Добавь кнопку очистки"), Some("Russian"));
        assert_eq!(detect("Додай кнопку очищення, і все"), Some("Ukrainian"));
        // (not sure: nothing said — the caller keeps what it had)
        assert_eq!(detect("ok"), None);
        assert_eq!(detect(""), None);
        assert_eq!(detect("```basic\nPRINT \"the and is\"\n```"), None);
        // (code and paths pasted in: the words around them decide)
        assert_eq!(detect("Me sale este error en `server.rr`: ```\nerror server.rr:12: the variable is not declared\n```\n¿qué hago?"), Some("Spanish"));
        assert_eq!(detect("Gracias"), Some("Spanish"));
    }

    #[test]
    fn answers_in_studios_language_unless_the_person_writes_another() {
        use ReplyIn::*;
        // (Studio in Spanish, a message clearly in English: English)
        assert_eq!(reply_language("es", Studio, "Please add a Clear button to the form"), "English");
        // (in Spanish: Spanish)
        assert_eq!(reply_language("es", Studio, "hazme una calculadora"), "Spanish");
        // (says too little: Studio's)
        assert_eq!(reply_language("es", Studio, "ok"), "Spanish");
        assert_eq!(reply_language("es-AR", Studio, ""), "Spanish");
        // (the code alone: Studio's)
        assert_eq!(reply_language("es", Studio, "```basic\nPRINT 1 + 2\n```"), "Spanish");
        assert_eq!(reply_language("en", Studio, "```basic\nPRINT 1 + 2\n```"), "English");
        // (the language the person writes in: English message, English)
        assert_eq!(reply_language("es", Writer, "Please add a Clear button to the form"), "English");
        // (Studio in English, Chinese or Spanish messages)
        assert_eq!(reply_language("en", Studio, "请添加一个清除按钮"), "Chinese");
        assert_eq!(reply_language("en", Studio, "hazme una calculadora con botones"), "Spanish");
        assert_eq!(reply_language("en", Studio, "hazme una calculadora de propinas"), "Spanish");
        assert_eq!(detect("hazme una calculadora de propinas"), Some("Spanish"));
        assert_eq!(reply_language("en", Studio, "ok"), "English");
        // (a language Studio isn't in, or no tag: English)
        assert_eq!(reply_language("xx", Studio, "ok"), "English");
        assert_eq!(reply_language("", Studio, "ok"), "English");
        assert_eq!(reply_language("zh-Hans", Studio, "ok"), "Chinese");
        // (a chat that goes on: Studio's way goes back to Studio's language
        // at "ok"; the writer's way keeps the language it saw)
        assert_eq!(reply_language_keeping("es", Studio, "ok", Some("English")), "Spanish");
        assert_eq!(reply_language_keeping("es", Writer, "ok", Some("English")), "English");
        assert_eq!(reply_language_keeping("es", Writer, "ok", None), "Spanish");
        assert_eq!(reply_language_keeping("es", Writer, "hazme una calculadora", Some("English")), "Spanish");
        // (the setting as it is kept: an old file's nothing is the default)
        assert_eq!(ReplyIn::parse(""), Studio);
        assert_eq!(ReplyIn::parse("studio"), Studio);
        assert_eq!(ReplyIn::parse(" Writer "), Writer);
        assert_eq!(ReplyIn::parse("who knows"), Studio);
        assert_eq!(Writer.as_str(), "writer");
    }

    #[test]
    fn tags_and_names() {
        assert_eq!(name_of_tag("es-AR"), Some("Spanish"));
        assert_eq!(name_of_tag("en_US.UTF-8"), Some("English"));
        assert_eq!(name_of_tag("xx"), None);
        assert_eq!(for_model("Spanish"), "Spanish (español)");
        assert_eq!(for_model("English"), "English");
    }
}
