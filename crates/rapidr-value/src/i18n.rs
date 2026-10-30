//! Programs in every language (RapidR's): `TR$`, `TRN$` and
//! `Application.Language`, one implementation for every runtime
//! (docs/program-languages-plan.md §1–2, lane T1).
//!
//! - **Catalogs** are PO files, one per language, in a `lang` folder beside
//!   the program (`lang/es.po`, `lang/pt-BR.po`, `lang/zh-Hans.po`): the
//!   folder of the program file the RapidR Runtime runs (`rapidr run
//!   prog.bas`), of a built executable, or the web page's — or the folder
//!   the program names (`Application.LanguageFolder`). RapidR reads them
//!   with its own small parser (`rapidr_i18n`); no gettext code.
//! - **The language** is the system's until the program sets
//!   `Application.Language`; `es-AR` looks in `es-AR.po`, then `es.po`,
//!   then gives the source text ([`fallback_chain`]).
//! - **Plurals** follow the catalog's `Plural-Forms` header ([`Plural`]),
//!   English's rule (`n != 1`) when it has none or for the source text.
//! - **Placeholders** `{name}` are filled by name ([`fill`]); `{{` and `}}`
//!   are literal braces; a name nobody gave stays as written.
//! - **A change of language** re-translates the text the program's
//!   components show that came from its catalogs ([`take_retranslation`]:
//!   a caption shown in the old language is read back to its source, then
//!   translated again), before `OnLanguageChange` runs for the rest.
//!
//! A missing folder, file or entry is never an error: the text is the
//! program's own. A bad line in a file is skipped; the good entries load.

use crate::{v_str, Value};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

pub use rapidr_i18n::po::{parse_po, Catalog, Plural};
pub use rapidr_i18n::{fallback_chain, fill, normalize_tag};

/// The value of placeholder `name` among `pairs` (name, value, name,
/// value …; names in any case).
fn pair_value(pairs: &[Value], name: &str) -> Option<String> {
    pairs.chunks(2).find(|p| p[0].to_string_val().trim().eq_ignore_ascii_case(name)).map(|p| p.get(1).map(Value::to_string_val).unwrap_or_default())
}

/// What reads a catalog file by its name in the program's `lang` folder
/// (`lang/es.po`): its bytes, None when there is none.
pub type Reader = fn(&str) -> Option<Vec<u8>>;

#[derive(Default)]
struct State {
    /// What the program set (normalized); None: the system's.
    chosen: Option<String>,
    /// The system's language, once asked.
    system: Option<String>,
    /// The current language's catalogs, most specific first; None until
    /// the first translation after the language was set.
    chain: Option<Rc<Vec<Catalog>>>,
    /// The language the program was last told about (OnLanguageChange):
    /// None until it listens.
    told: Option<String>,
    /// The folder the program's catalogs are in, when it named one
    /// (`Application.LanguageFolder`); None: `lang` beside the program.
    folder: Option<String>,
    /// The catalogs of the language before the last change, while the
    /// components' text is still to be translated again
    /// ([`take_retranslation`]).
    before: Option<Rc<Vec<Catalog>>>,
}

thread_local! {
    static STATE: RefCell<State> = RefCell::new(State::default());
    /// How the runtime asks the system's language (the web's: the
    /// browser's), and reads the program's files (the web's: the page's).
    static SYSTEM_PROBE: Cell<Option<fn() -> String>> = const { Cell::new(None) };
    static READER: Cell<Option<Reader>> = const { Cell::new(None) };
}

/// A runtime without a process environment or file system (the web's)
/// says how to ask the system's language and how to read the program's
/// files (`lang/es.po`), before the program runs.
pub fn set_platform(system_language: fn() -> String, reader: Reader) {
    SYSTEM_PROBE.with(|p| p.set(Some(system_language)));
    READER.with(|r| r.set(Some(reader)));
}

/// The language the program speaks now: what it set as
/// `Application.Language`, else the system's (normalized: `es-AR`).
pub fn language() -> String {
    STATE.with(|s| {
        let mut s = s.borrow_mut();
        if let Some(l) = &s.chosen {
            return l.clone();
        }
        s.system.get_or_insert_with(system_language).clone()
    })
}

/// `Application.Language = tag`: the program speaks that language from
/// now on (its catalogs read again); "" goes back to the system's. When
/// the language changes, the text the components show is to be translated
/// again ([`take_retranslation`]).
pub fn set_language(tag: &str) {
    let tag = normalize_tag(tag);
    let old = language();
    let old_chain = chain();
    STATE.with(|s| {
        let mut s = s.borrow_mut();
        s.chosen = (!tag.is_empty()).then_some(tag);
        s.chain = None;
    });
    if language() != old {
        STATE.with(|s| {
            let mut s = s.borrow_mut();
            // (two changes before the hosts look: the text still shows the
            // first language's)
            if s.before.is_none() {
                s.before = Some(old_chain);
            }
        });
    }
}

/// The folder the program's catalogs are in (`Application.LanguageFolder`):
/// "" for `lang` beside the program.
pub fn folder() -> String {
    STATE.with(|s| s.borrow().folder.clone().unwrap_or_default())
}

/// `Application.LanguageFolder = folder`: the catalogs are read from there
/// from now on ("" goes back to `lang` beside the program); a language's
/// text shown so far is translated again as for a change of language.
pub fn set_folder(folder: &str) {
    let folder = folder.trim().replace('\\', "/");
    let folder = folder.trim_end_matches('/').to_string();
    let old_chain = chain();
    STATE.with(|s| {
        let mut s = s.borrow_mut();
        if s.folder.clone().unwrap_or_default() == folder {
            return;
        }
        s.folder = (!folder.is_empty()).then_some(folder);
        s.chain = None;
        if s.before.is_none() {
            s.before = Some(old_chain);
        }
    });
}

/// The program listens for the language's changes (its
/// Application.OnLanguageChange handler was bound): from the language it
/// has now.
pub fn listen() {
    let now = language();
    STATE.with(|s| s.borrow_mut().told = Some(now));
}

/// Whether the language changed since the program was last told (since
/// its OnLanguageChange handler was bound, or this last said so). Never
/// before it listens; a change undone before it's asked is none.
pub fn take_change() -> bool {
    let told = STATE.with(|s| s.borrow().told.clone());
    let Some(told) = told else { return false };
    let now = language();
    if told == now {
        return false;
    }
    STATE.with(|s| s.borrow_mut().told = Some(now));
    true
}

/// How text shown in the language before a change reads in the language
/// now ([`take_retranslation`]).
pub struct Retranslation {
    before: Rc<Vec<Catalog>>,
    now: Rc<Vec<Catalog>>,
}

impl Retranslation {
    /// `shown`'s text in the language now, when it is a catalog's text (a
    /// translation in the language before, or a source text either
    /// language's catalogs have) and reads differently now. A text with a
    /// tab (a menu item's caption and its shortcut) is translated before
    /// its tab. Anything else — the program's own data, text made with
    /// placeholders filled — None: the program's OnLanguageChange sets it.
    pub fn text(&self, shown: &str) -> Option<String> {
        if let Some((head, tail)) = shown.split_once('\t') {
            return self.text(head).map(|h| format!("{h}\t{tail}"));
        }
        if shown.trim().is_empty() {
            return None;
        }
        let (context, source) = match self.before.iter().find_map(|c| c.source_of(shown)) {
            Some((c, s)) => (c.to_string(), s.to_string()),
            None => {
                let known = |chain: &[Catalog]| chain.iter().find_map(|c| c.context_of(shown)).map(|c| (c.to_string(), shown.to_string()));
                known(&self.now).or_else(|| known(&self.before))?
            }
        };
        let new = self.now.iter().find_map(|c| c.text(&context, &source)).unwrap_or(&source);
        (new != shown).then(|| new.to_string())
    }
}

/// After a change of language (or of the catalogs' folder), once: how the
/// components' text reads now — the hosts translate it again
/// ([`Retranslation::text`]) before OnLanguageChange. None when nothing
/// changed since they last asked.
pub fn take_retranslation() -> Option<Retranslation> {
    let before = STATE.with(|s| s.borrow_mut().before.take())?;
    let now = chain();
    if before.is_empty() && now.is_empty() {
        return None;
    }
    Some(Retranslation { before, now })
}

/// The system's language: the runtime's probe (the web's: the browser's),
/// else the computer's (`rapidr_i18n::tags::system_language`: `LANGUAGE`,
/// `LC_ALL`, `LC_MESSAGES`, the system's setting, `LANG`; English when
/// none says).
fn system_language() -> String {
    if let Some(probe) = SYSTEM_PROBE.with(Cell::get) {
        let t = normalize_tag(&probe());
        return if t.is_empty() { "en".to_string() } else { t };
    }
    rapidr_i18n::tags::system_language()
}

/// A catalog file's bytes, by name in the program's catalog folder: the
/// runtime's reader (the web's), else `lang` beside the program file or
/// executable (`rapidr_value::command_line::files_folder`), or the folder
/// the program named.
fn read(name: &str) -> Option<Vec<u8>> {
    let folder = folder();
    if let Some(reader) = READER.with(Cell::get) {
        let dir = if folder.is_empty() { "lang".to_string() } else { folder };
        return reader(&format!("{dir}/{name}"));
    }
    let dir = if folder.is_empty() {
        crate::command_line::files_folder()?.join("lang")
    } else {
        let f = std::path::PathBuf::from(&folder);
        if f.is_absolute() {
            f
        } else {
            crate::command_line::files_folder().map(|p| p.join(&f)).unwrap_or(f)
        }
    };
    std::fs::read(dir.join(name)).ok()
}

/// The catalog for exactly language `tag`, when the program has its file
/// (`es-AR.po`, or `es_AR.po`, in either case).
fn load(tag: &str) -> Option<Catalog> {
    let lower = tag.to_ascii_lowercase();
    let mut names: Vec<String> = Vec::with_capacity(4);
    for name in [tag.to_string(), tag.replace('-', "_"), lower.replace('-', "_"), lower] {
        if !names.contains(&name) {
            names.push(name);
        }
    }
    names.iter().find_map(|name| read(&format!("{name}.po"))).map(|bytes| parse_po(&String::from_utf8_lossy(&bytes)))
}

/// The current language's catalogs (read once per language).
fn chain() -> Rc<Vec<Catalog>> {
    if let Some(c) = STATE.with(|s| s.borrow().chain.clone()) {
        return c;
    }
    let chain: Rc<Vec<Catalog>> = Rc::new(fallback_chain(&language()).iter().filter_map(|t| load(t)).collect());
    STATE.with(|s| s.borrow_mut().chain = Some(chain.clone()));
    chain
}

/// `text`'s translation in `context` in the current language, else `text`.
pub fn translate(context: &str, text: &str) -> String {
    let chain = chain();
    chain.iter().find_map(|c| c.text(context, text)).unwrap_or(text).to_string()
}

/// The translation of `singular` / `plural` for count `n` in `context`:
/// the first catalog that has it, by its language's plural rule; else the
/// source text by English's (`singular` for 1).
pub fn translate_plural(context: &str, singular: &str, plural: &str, n: u64) -> String {
    let chain = chain();
    match chain.iter().find_map(|c| c.plural_text(context, singular, n)) {
        Some(t) => t.to_string(),
        None if n == 1 => singular.to_string(),
        None => plural.to_string(),
    }
}

/// `TR$(text$ [, context$] [, name$, value …])`: after the text, an odd
/// number of arguments starts with the context; the rest are placeholder
/// pairs, name then value.
pub fn rp_tr(args: &[Value]) -> Value {
    let text = args.first().map(Value::to_string_val).unwrap_or_default();
    let rest = args.get(1..).unwrap_or(&[]);
    let (context, pairs) = split_context(rest);
    let translated = translate(&context, &text);
    v_str(&fill(&translated, |name| pair_value(pairs, name)))
}

/// `TRN$(singular$, plural$, count [, context$] [, name$, value …])`: the
/// form for `count` in the current language, `{n}` the count.
pub fn rp_trn(args: &[Value]) -> Value {
    let arg = |i: usize| args.get(i).map(Value::to_string_val).unwrap_or_default();
    let count = args.get(2).cloned().unwrap_or(Value::Integer(0));
    let n = count.to_f64().abs();
    let n = if n.is_finite() { n.trunc().min(u64::MAX as f64) as u64 } else { 0 };
    let (context, pairs) = split_context(args.get(3..).unwrap_or(&[]));
    let translated = translate_plural(&context, &arg(0), &arg(1), n);
    v_str(&fill(&translated, |name| if name.eq_ignore_ascii_case("n") { Some(count.to_string_val()) } else { pair_value(pairs, name) }))
}

fn split_context(rest: &[Value]) -> (String, &[Value]) {
    if rest.len() % 2 == 1 {
        (rest[0].to_string_val(), &rest[1..])
    } else {
        (String::new(), rest)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{v_dbl, v_int};
    use std::collections::HashMap;

    const ES: &str = "msgid \"\"\nmsgstr \"Plural-Forms: nplurals=2; plural=(n != 1);\\n\"\n\n\
                      msgid \"Save file\"\nmsgstr \"Guardar archivo\"\n\n\
                      msgctxt \"menu\"\nmsgid \"Open\"\nmsgstr \"Abrir\"\n\n\
                      msgid \"Open\"\nmsgstr \"Abierto\"\n\n\
                      msgid \"{n} file\"\nmsgid_plural \"{n} files\"\nmsgstr[0] \"{n} archivo\"\nmsgstr[1] \"{n} archivos\"\n";

    thread_local!(static FILES: RefCell<HashMap<String, String>> = RefCell::new(HashMap::new()));

    fn test_reader(name: &str) -> Option<Vec<u8>> {
        FILES.with(|f| f.borrow().get(name).map(|s| s.as_bytes().to_vec()))
    }

    fn with_files(files: &[(&str, &str)]) {
        FILES.with(|f| *f.borrow_mut() = files.iter().map(|(k, v)| (k.to_string(), v.to_string())).collect());
        set_platform(|| "en-US".to_string(), test_reader);
        STATE.with(|s| *s.borrow_mut() = State::default());
    }

    #[test]
    fn translates_in_the_language_set() {
        with_files(&[
            ("lang/es.po", ES),
            ("lang/es_AR.po", "msgid \"Open\"\nmsgstr \"Abrí\"\n"),
            ("lang/zh-Hans.po", "msgid \"\"\nmsgstr \"Plural-Forms: nplurals=1; plural=0;\\n\"\nmsgid \"{n} file\"\nmsgid_plural \"{n} files\"\nmsgstr[0] \"{n} 个文件\"\nmsgid \"Save file\"\nmsgstr \"保存文件\"\n"),
        ]);
        let tr = |args: &[Value]| rp_tr(args).to_string_val();
        let trn = |args: &[Value]| rp_trn(args).to_string_val();
        assert_eq!(language(), "en-US");
        assert_eq!(tr(&[v_str("Save file")]), "Save file");
        set_language("es");
        assert_eq!(language(), "es");
        assert_eq!(tr(&[v_str("Save file")]), "Guardar archivo");
        assert_eq!(tr(&[v_str("Open"), v_str("menu")]), "Abrir");
        assert_eq!(tr(&[v_str("Open")]), "Abierto");
        assert_eq!(trn(&[v_str("{n} file"), v_str("{n} files"), v_int(1)]), "1 archivo");
        assert_eq!(trn(&[v_str("{n} file"), v_str("{n} files"), v_int(5)]), "5 archivos");
        assert_eq!(tr(&[v_str("Hello, {name}!"), v_str("name"), v_str("Ana")]), "Hello, Ana!");
        // es-AR: its own file first, then es
        set_language("es_ar");
        assert_eq!(language(), "es-AR");
        assert_eq!(tr(&[v_str("Open")]), "Abrí");
        assert_eq!(tr(&[v_str("Save file")]), "Guardar archivo");
        // pt: no file, the source text; English's plural rule
        set_language("pt");
        assert_eq!(tr(&[v_str("Save file")]), "Save file");
        assert_eq!(trn(&[v_str("{n} file"), v_str("{n} files"), v_int(1)]), "1 file");
        assert_eq!(trn(&[v_str("{n} file"), v_str("{n} files"), v_dbl(2.5)]), "2.5 files");
        // zh-Hans: one form for every count
        set_language("zh-Hans");
        assert_eq!(trn(&[v_str("{n} file"), v_str("{n} files"), v_int(1)]), "1 个文件");
        assert_eq!(trn(&[v_str("{n} file"), v_str("{n} files"), v_int(9)]), "9 个文件");
        set_language("zh_CN");
        assert_eq!(tr(&[v_str("Save file")]), "保存文件");
        // context and placeholders together; a count as a placeholder too
        set_language("es");
        assert_eq!(tr(&[v_str("Open"), v_str("menu"), v_str("x"), v_int(1)]), "Abrir");
        assert_eq!(trn(&[v_str("{n} file"), v_str("{n} files"), v_int(-1), v_str("who"), v_str("Ana")]), "-1 archivo");
        // back to the system's
        set_language("");
        assert_eq!(language(), "en-US");
    }

    #[test]
    fn language_changes_are_told_once() {
        with_files(&[]);
        assert!(!take_change(), "never before the program listens");
        set_language("es");
        assert!(!take_change());
        listen();
        assert!(!take_change());
        set_language("pt-br");
        assert!(take_change());
        assert!(!take_change());
        set_language("es");
        set_language("pt-BR");
        assert!(!take_change(), "a change undone is none");
    }

    #[test]
    fn a_program_without_catalogs_gets_its_own_text() {
        with_files(&[]);
        set_language("de");
        assert_eq!(rp_tr(&[v_str("Hi {who}"), v_str("who"), v_int(3)]).to_string_val(), "Hi 3");
        assert_eq!(rp_tr(&[]).to_string_val(), "");
        assert_eq!(rp_trn(&[v_str("one"), v_str("many")]).to_string_val(), "many");
        assert!(take_retranslation().is_none(), "no catalogs: nothing to translate again");
    }

    #[test]
    fn shown_text_is_translated_again() {
        with_files(&[("lang/es.po", ES), ("lang/xx.po", "msgid \"Save file\"\nmsgstr \"[Šàvé ƒíĺé]\"\n")]);
        // (English, the system's: the sources shown)
        assert!(take_retranslation().is_none());
        set_language("es");
        let r = take_retranslation().expect("a change");
        assert_eq!(r.text("Save file").as_deref(), Some("Guardar archivo"));
        assert_eq!(r.text("Save file\tCtrl+S").as_deref(), Some("Guardar archivo\tCtrl+S"));
        assert_eq!(r.text("my data"), None);
        assert!(take_retranslation().is_none(), "once");
        set_language("xx");
        let r = take_retranslation().unwrap();
        assert_eq!(r.text("Guardar archivo").as_deref(), Some("[Šàvé ƒíĺé]"));
        // (no xx for it: back to the source)
        assert_eq!(r.text("Abierto").as_deref(), Some("Open"));
        set_language("en");
        let r = take_retranslation().unwrap();
        assert_eq!(r.text("[Šàvé ƒíĺé]").as_deref(), Some("Save file"));
        assert_eq!(r.text("Save file"), None);
    }

    #[test]
    fn a_named_folder_holds_the_catalogs() {
        with_files(&[("studio/es.po", ES)]);
        set_language("es");
        assert_eq!(translate("", "Save file"), "Save file");
        set_folder("studio/");
        assert_eq!(folder(), "studio");
        assert_eq!(translate("", "Save file"), "Guardar archivo");
        assert_eq!(take_retranslation().unwrap().text("Save file").as_deref(), Some("Guardar archivo"));
        set_folder("");
        assert_eq!(translate("", "Save file"), "Save file");
    }
}
