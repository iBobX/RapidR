//! Declarative language definitions (docs/ide-components.md §2).
//!
//! A language is a TOML file: comments, brackets and auto-closing pairs,
//! indentation and folding rules, keyword groups, snippets, and a small
//! state-machine tokenizer — each state an ordered list of rules (a regex,
//! the token kind, an optional push / pop / next of a state, or an embedded
//! language until an end pattern). The state at a line's end is carried to
//! the next line ([`crate::highlight`]), so an edit re-colours only until a
//! line's end state is unchanged. No tree-sitter: BASIC's real structure
//! comes from RapidR's own parser (the language service, stage I3).
//!
//! The format:
//!
//! ```toml
//! [language]
//! id = "rapidq-basic"            # required
//! name = "RapidQ / RapidR BASIC" # required
//! extensions = ["bas", "rr"]     # file extensions, without the dot
//! filenames = []                 # whole file names (`Cargo.lock`)
//! case_insensitive = true        # regexes and keywords ignore case
//! line_comment = ["'"]           # the first one is what Toggle Comment writes
//! block_comment = ["/*", "*/"]
//! word = '[A-Za-z_][A-Za-z0-9_]*'  # what a word is (double-click, whole word)
//! keyword_suffixes = "$%&!#"     # `LEFT$` is the keyword LEFT
//!
//! [brackets]
//! pairs = [["(", ")"]]           # matched and folded (single characters)
//! auto_close = [["(", ")"], ['"', '"']]   # default: `pairs`
//! auto_close_before = " \t)]};,"  # close only before these (or the line's end)
//!
//! [indent]
//! increase = 'regex'             # a line like this indents the next one
//! decrease = 'regex'             # a line like this goes one level back
//!
//! [folding]
//! markers = [['start regex', 'end regex']]
//! brackets = true                # multi-line bracket pairs fold
//! offside = true                 # fold by indentation (the default when
//!                                # neither markers nor brackets are given)
//!
//! [keywords]                     # group = words; the group is a token kind
//! "keyword.control" = ["IF", "THEN"]   # (`control` is short for it)
//!
//! [defaults]                     # the token of text no rule matches, per state
//! string = "string"
//!
//! [[states.root]]                # `root` is where every line of the file starts
//! match = "'.*$"                 # a regex (always searched in one line)
//! token = "comment"
//! [[states.root]]
//! match = '"'
//! token = "string"
//! push = "string"                # also: pop = true, next = "state"
//! [[states.root]]
//! match = '[A-Za-z_]\w*'
//! token = "variable"
//! keywords = true                # a word in a [keywords] group takes its kind
//! [[states.root]]
//! match = '(SUB)(\s+)(\w+)'
//! groups = ["keyword", "", "function"]  # a kind per capture group ("" = token)
//! [[states.root]]
//! match = '^RUSTSTART.*$'
//! token = "keyword"
//! embed = "rust"                 # another language from here …
//! end = '^RUSTEND.*$'            # … until this (searched first on each line)
//! end_token = "keyword"
//! [[states.root]]
//! include = "common"             # another state's rules, here
//!
//! [[snippets]]
//! prefix = "sub"
//! body = "SUB ${1:Name}(${2})\n\t$0\nEND SUB"
//! description = "A SUB"
//! ```

pub mod basic;
mod kind;
mod tokenizer;

use std::collections::{BTreeMap, HashMap};
use std::sync::{Arc, OnceLock};

use regex::Regex;
use regex_automata::meta;
use regex_automata::util::syntax;
use serde::Deserialize;

pub use kind::{Token, TokenKind, FIXED_KINDS};
pub use tokenizer::{EMBED_MARK, MAX_LINE_BYTES, MAX_STACK};

/// A language definition that could not be loaded.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LangError(pub String);

impl std::fmt::Display for LangError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for LangError {}

fn err<T>(msg: impl Into<String>) -> Result<T, LangError> {
    Err(LangError(msg.into()))
}

// ---- the TOML format ----

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Def {
    language: LangDef,
    #[serde(default)]
    brackets: BracketsDef,
    #[serde(default)]
    indent: IndentDef,
    #[serde(default)]
    folding: FoldingDef,
    #[serde(default)]
    keywords: BTreeMap<String, Vec<String>>,
    #[serde(default)]
    defaults: BTreeMap<String, String>,
    #[serde(default)]
    states: BTreeMap<String, Vec<RuleDef>>,
    #[serde(default)]
    snippets: Vec<Snippet>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct LangDef {
    id: String,
    name: String,
    #[serde(default)]
    extensions: Vec<String>,
    #[serde(default)]
    filenames: Vec<String>,
    #[serde(default)]
    case_insensitive: bool,
    #[serde(default)]
    line_comment: Vec<String>,
    #[serde(default)]
    block_comment: Option<[String; 2]>,
    #[serde(default)]
    word: Option<String>,
    #[serde(default)]
    keyword_suffixes: String,
}

#[derive(Deserialize, Default)]
#[serde(deny_unknown_fields)]
struct BracketsDef {
    #[serde(default)]
    pairs: Vec<[String; 2]>,
    #[serde(default)]
    auto_close: Option<Vec<[String; 2]>>,
    #[serde(default)]
    auto_close_before: Option<String>,
}

#[derive(Deserialize, Default)]
#[serde(deny_unknown_fields)]
struct IndentDef {
    increase: Option<String>,
    decrease: Option<String>,
}

#[derive(Deserialize, Default)]
#[serde(deny_unknown_fields)]
struct FoldingDef {
    #[serde(default)]
    markers: Vec<[String; 2]>,
    #[serde(default)]
    brackets: bool,
    offside: Option<bool>,
}

#[derive(Deserialize, Clone, Default)]
#[serde(deny_unknown_fields)]
struct RuleDef {
    #[serde(rename = "match")]
    pattern: Option<String>,
    token: Option<String>,
    #[serde(default)]
    keywords: bool,
    #[serde(default)]
    groups: Vec<String>,
    push: Option<String>,
    #[serde(default)]
    pop: bool,
    next: Option<String>,
    embed: Option<String>,
    end: Option<String>,
    end_token: Option<String>,
    include: Option<String>,
}

/// A snippet: what to type (`prefix`) and what it becomes (`body`, with
/// `$1`, `${1:default}` and `$0` tab stops).
#[derive(Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Snippet {
    pub prefix: String,
    pub body: String,
    #[serde(default)]
    pub description: String,
}

// ---- the compiled form ----

#[derive(Debug)]
pub(crate) struct Rule {
    pub token: TokenKind,
    pub keywords: bool,
    /// A kind per capture group (`None`: the rule's token).
    pub groups: Vec<Option<TokenKind>>,
    /// The rule alone, for its capture groups.
    pub single: Option<meta::Regex>,
    pub pop: bool,
    pub push: Option<u16>,
    pub next: Option<u16>,
    pub embed: Option<u16>,
}

#[derive(Debug)]
pub(crate) struct State {
    /// Every rule's pattern, in order (`None`: no rules).
    pub regex: Option<meta::Regex>,
    pub rules: Vec<Rule>,
    pub default: TokenKind,
}

#[derive(Debug)]
pub(crate) struct Embed {
    pub lang: Arc<Language>,
    pub end: meta::Regex,
    pub end_token: TokenKind,
}

/// A loaded language definition.
#[derive(Debug)]
pub struct Language {
    pub id: String,
    pub name: String,
    pub extensions: Vec<String>,
    pub filenames: Vec<String>,
    pub case_insensitive: bool,
    pub line_comment: Vec<String>,
    pub block_comment: Option<(String, String)>,
    /// Bracket pairs (single characters).
    pub brackets: Vec<(char, char)>,
    pub auto_close: Vec<(String, String)>,
    /// Auto-close only before one of these characters or the line's end.
    pub auto_close_before: String,
    pub indent_increase: Option<Regex>,
    pub indent_decrease: Option<Regex>,
    pub fold_markers: Vec<(Regex, Regex)>,
    /// Both ends of every fold marker, start patterns first.
    pub(crate) fold_marker_set: Option<regex::RegexSet>,
    pub fold_brackets: bool,
    pub fold_offside: bool,
    pub word: Regex,
    pub keyword_suffixes: String,
    pub snippets: Vec<Snippet>,
    pub(crate) keywords: HashMap<Box<str>, TokenKind>,
    pub(crate) states: Vec<State>,
    pub(crate) state_names: Vec<String>,
    pub(crate) embeds: Vec<Embed>,
}

fn kind(name: &str, ctx: &str) -> Result<TokenKind, LangError> {
    let name = match name {
        "control" => "keyword.control",
        "builtin" => "function",
        n => n,
    };
    TokenKind::named(name).ok_or_else(|| LangError(format!("{ctx}: unknown token kind {name:?}")))
}

fn compile(pattern: &str, ci: bool, ctx: &str) -> Result<Regex, LangError> {
    regex::RegexBuilder::new(pattern).case_insensitive(ci).build().map_err(|e| LangError(format!("{ctx}: {e}")))
}

fn meta_builder(ci: bool) -> meta::Builder {
    let mut b = meta::Regex::builder();
    b.syntax(syntax::Config::new().case_insensitive(ci));
    b
}

fn compile_meta(patterns: &[&str], ci: bool, ctx: &str) -> Result<meta::Regex, LangError> {
    meta_builder(ci).build_many(patterns).map_err(|e| LangError(format!("{ctx}: {e}")))
}

impl Language {
    /// Loads a definition. `resolve` finds the languages it embeds by id.
    pub fn from_toml(src: &str, resolve: &dyn Fn(&str) -> Option<Arc<Language>>) -> Result<Language, LangError> {
        Language::from_toml_with(src, resolve, &[])
    }

    /// Loads a definition with extra keyword groups (BASIC's come from the
    /// compiler's lists, later from the language registry).
    pub fn from_toml_with(src: &str, resolve: &dyn Fn(&str) -> Option<Arc<Language>>, extra_keywords: &[(&str, &[&str])]) -> Result<Language, LangError> {
        let def: Def = toml::from_str(src).map_err(|e| LangError(format!("language definition: {e}")))?;
        let ci = def.language.case_insensitive;
        let id = def.language.id.clone();
        let at = |what: &str| format!("{id}: {what}");

        // keywords
        let mut keywords = HashMap::new();
        let mut add = |group: &str, words: &mut dyn Iterator<Item = &str>| -> Result<(), LangError> {
            let k = kind(group, &at(&format!("[keywords] {group}")))?;
            for w in words {
                let w = if ci { w.to_uppercase() } else { w.to_string() };
                keywords.entry(w.into_boxed_str()).or_insert(k);
            }
            Ok(())
        };
        for (group, words) in &def.keywords {
            add(group, &mut words.iter().map(String::as_str))?;
        }
        for (group, words) in extra_keywords {
            add(group, &mut words.iter().copied())?;
        }

        // states: root first, then the others by name
        if !def.states.is_empty() && !def.states.contains_key("root") {
            return err(at("no [[states.root]]"));
        }
        let mut state_names: Vec<String> = Vec::new();
        if def.states.contains_key("root") {
            state_names.push("root".into());
        }
        state_names.extend(def.states.keys().filter(|k| *k != "root").cloned());
        if state_names.len() >= EMBED_MARK as usize {
            return err(at("too many states"));
        }
        let state_index = |name: &str, ctx: &str| -> Result<u16, LangError> {
            state_names.iter().position(|s| s == name).map(|i| i as u16).ok_or_else(|| LangError(format!("{ctx}: no state {name:?}")))
        };

        // includes, spliced (a state can't include itself through others)
        fn expand(states: &BTreeMap<String, Vec<RuleDef>>, name: &str, depth: usize, out: &mut Vec<RuleDef>, id: &str) -> Result<(), LangError> {
            if depth > 16 {
                return err(format!("{id}: includes nest too deep (a cycle?) at {name:?}"));
            }
            let Some(rules) = states.get(name) else {
                return err(format!("{id}: include of unknown state {name:?}"));
            };
            for r in rules {
                match &r.include {
                    Some(inc) => expand(states, inc, depth + 1, out, id)?,
                    None => out.push(r.clone()),
                }
            }
            Ok(())
        }

        let mut embeds: Vec<Embed> = Vec::new();
        let mut states = Vec::with_capacity(state_names.len());
        for name in &state_names {
            let mut defs = Vec::new();
            expand(&def.states, name, 0, &mut defs, &id)?;
            let mut patterns: Vec<String> = Vec::new();
            let mut rules = Vec::new();
            for (i, r) in defs.iter().enumerate() {
                let ctx = at(&format!("states.{name} rule {}", i + 1));
                let Some(pattern) = &r.pattern else {
                    return err(format!("{ctx}: no `match`"));
                };
                let single = compile_meta(&[pattern.as_str()], ci, &ctx)?;
                let hir = syntax::parse_with(pattern, &syntax::Config::new().case_insensitive(ci)).map_err(|e| LangError(format!("{ctx}: {e}")))?;
                let may_be_empty = hir.properties().minimum_len() == Some(0);
                let transition = r.pop || r.push.is_some() || r.next.is_some() || r.embed.is_some();
                if may_be_empty && !transition {
                    return err(format!("{ctx}: {pattern:?} can match nothing (only a rule that changes state may)"));
                }
                let token = match &r.token {
                    Some(t) => kind(t, &ctx)?,
                    None if !r.groups.is_empty() || r.keywords => TokenKind::TEXT,
                    None if transition => TokenKind::TEXT,
                    None => return err(format!("{ctx}: no `token`")),
                };
                let groups_in_pattern = single.group_info().group_len(regex_automata::PatternID::ZERO).saturating_sub(1);
                if r.groups.len() > groups_in_pattern {
                    return err(format!("{ctx}: {} group kinds for {groups_in_pattern} groups", r.groups.len()));
                }
                let groups = r.groups.iter().map(|g| if g.is_empty() { Ok(None) } else { kind(g, &ctx).map(Some) }).collect::<Result<Vec<_>, _>>()?;
                let embed = match &r.embed {
                    None => None,
                    Some(lang_id) => {
                        let Some(lang) = resolve(lang_id) else {
                            return err(format!("{ctx}: unknown language {lang_id:?} to embed"));
                        };
                        let Some(end) = &r.end else {
                            return err(format!("{ctx}: an embedded language needs an `end` pattern"));
                        };
                        let end_re = compile_meta(&[end.as_str()], ci, &format!("{ctx} end"))?;
                        let end_token = match &r.end_token {
                            Some(t) => kind(t, &ctx)?,
                            None => TokenKind::TEXT,
                        };
                        if embeds.len() >= EMBED_MARK as usize - 1 {
                            return err(format!("{ctx}: too many embedded regions"));
                        }
                        embeds.push(Embed { lang, end: end_re, end_token });
                        Some((embeds.len() - 1) as u16)
                    }
                };
                if r.end.is_some() && r.embed.is_none() {
                    return err(format!("{ctx}: `end` only goes with `embed`"));
                }
                rules.push(Rule {
                    token,
                    keywords: r.keywords,
                    single: if groups.is_empty() { None } else { Some(single) },
                    groups,
                    pop: r.pop,
                    push: r.push.as_deref().map(|s| state_index(s, &ctx)).transpose()?,
                    next: r.next.as_deref().map(|s| state_index(s, &ctx)).transpose()?,
                    embed,
                });
                patterns.push(pattern.clone());
            }
            let regex = if patterns.is_empty() {
                None
            } else {
                let refs: Vec<&str> = patterns.iter().map(String::as_str).collect();
                Some(compile_meta(&refs, ci, &at(&format!("states.{name}")))?)
            };
            let default = match def.defaults.get(name) {
                Some(t) => kind(t, &at(&format!("[defaults] {name}")))?,
                None => TokenKind::TEXT,
            };
            states.push(State { regex, rules, default });
        }
        for name in def.defaults.keys() {
            if !state_names.contains(name) {
                return err(at(&format!("[defaults] names no state {name:?}")));
            }
        }

        // brackets
        let mut brackets = Vec::new();
        for [o, c] in &def.brackets.pairs {
            let (mut oc, mut cc) = (o.chars(), c.chars());
            match (oc.next(), oc.next(), cc.next(), cc.next()) {
                (Some(o), None, Some(c), None) => brackets.push((o, c)),
                _ => return err(at(&format!("[brackets] pairs: {o:?} / {c:?} must be single characters"))),
            }
        }
        let auto_close: Vec<(String, String)> = match &def.brackets.auto_close {
            Some(pairs) => pairs.iter().map(|[o, c]| (o.clone(), c.clone())).collect(),
            None => def.brackets.pairs.iter().map(|[o, c]| (o.clone(), c.clone())).collect(),
        };
        for (o, c) in &auto_close {
            if o.is_empty() || c.is_empty() {
                return err(at("[brackets] auto_close: empty pair"));
            }
        }

        let indent_increase = def.indent.increase.as_deref().map(|p| compile(p, ci, &at("[indent] increase"))).transpose()?;
        let indent_decrease = def.indent.decrease.as_deref().map(|p| compile(p, ci, &at("[indent] decrease"))).transpose()?;
        let mut fold_markers = Vec::new();
        for [s, e] in &def.folding.markers {
            fold_markers.push((compile(s, ci, &at("[folding] markers"))?, compile(e, ci, &at("[folding] markers"))?));
        }
        let fold_marker_set = if fold_markers.is_empty() {
            None
        } else {
            let all: Vec<&str> = def.folding.markers.iter().map(|m| m[0].as_str()).chain(def.folding.markers.iter().map(|m| m[1].as_str())).collect();
            Some(regex::RegexSetBuilder::new(all).case_insensitive(ci).build().map_err(|e| LangError(at(&format!("[folding] markers: {e}"))))?)
        };
        let fold_offside = def.folding.offside.unwrap_or(fold_markers.is_empty() && !def.folding.brackets);
        let word = compile(def.language.word.as_deref().unwrap_or(r"[\p{Alphabetic}\p{Nd}_]+"), ci, &at("word"))?;

        Ok(Language {
            id: def.language.id,
            name: def.language.name,
            extensions: def.language.extensions.iter().map(|e| e.trim_start_matches('.').to_ascii_lowercase()).collect(),
            filenames: def.language.filenames,
            case_insensitive: ci,
            line_comment: def.language.line_comment,
            block_comment: def.language.block_comment.map(|[a, b]| (a, b)),
            brackets,
            auto_close,
            auto_close_before: def.brackets.auto_close_before.unwrap_or_else(|| " \t)]};,".to_string()),
            indent_increase,
            indent_decrease,
            fold_markers,
            fold_marker_set,
            fold_brackets: def.folding.brackets,
            fold_offside,
            word,
            keyword_suffixes: def.language.keyword_suffixes,
            snippets: def.snippets,
            keywords,
            states,
            state_names,
            embeds,
        })
    }

    /// The kind of keyword `word` is, if it is one.
    pub fn keyword_kind(&self, word: &str) -> Option<TokenKind> {
        let lookup = |w: &str| -> Option<TokenKind> {
            if self.case_insensitive {
                if w.bytes().all(|b| !b.is_ascii_lowercase()) {
                    self.keywords.get(w).copied()
                } else {
                    self.keywords.get(w.to_uppercase().as_str()).copied()
                }
            } else {
                self.keywords.get(w).copied()
            }
        };
        if let Some(k) = lookup(word) {
            return Some(k);
        }
        let last = word.chars().last()?;
        if word.len() > last.len_utf8() && self.keyword_suffixes.contains(last) {
            return lookup(&word[..word.len() - last.len_utf8()]);
        }
        None
    }

    /// Every keyword and its kind (for completion lists), sorted.
    pub fn keywords(&self) -> Vec<(&str, TokenKind)> {
        let mut v: Vec<(&str, TokenKind)> = self.keywords.iter().map(|(w, k)| (&**w, *k)).collect();
        v.sort();
        v
    }

    /// The states' names (index = the state number in a stack).
    pub fn state_names(&self) -> &[String] {
        &self.state_names
    }

    /// The languages this one embeds.
    pub fn embedded(&self) -> impl Iterator<Item = &Arc<Language>> {
        self.embeds.iter().map(|e| &e.lang)
    }

    /// Whether `c` is part of a word (by the `word` pattern).
    pub fn is_word_char(&self, c: char) -> bool {
        let mut buf = [0u8; 4];
        self.word.is_match(c.encode_utf8(&mut buf))
    }
}

/// A set of languages: the built-in ones plus any loaded, looked up by id,
/// file extension or file name.
#[derive(Clone, Debug, Default)]
pub struct Languages {
    langs: Vec<Arc<Language>>,
}

/// The built-in definitions, in load order (embedded languages first).
pub const BUILTIN_SOURCES: &[(&str, &str)] = &[
    ("plaintext", include_str!("../../languages/plaintext.toml")),
    ("json", include_str!("../../languages/json.toml")),
    ("toml", include_str!("../../languages/toml.toml")),
    ("csv", include_str!("../../languages/csv.toml")),
    ("sql", include_str!("../../languages/sql.toml")),
    ("css", include_str!("../../languages/css.toml")),
    ("javascript", include_str!("../../languages/javascript.toml")),
    ("rust", include_str!("../../languages/rust.toml")),
    ("markdown", include_str!("../../languages/markdown.toml")),
    ("html", include_str!("../../languages/html.toml")),
    ("rapidq-basic", include_str!("../../languages/rapidq-basic.toml")),
];

impl Languages {
    /// An empty set.
    pub fn new() -> Self {
        Languages { langs: Vec::new() }
    }

    /// The built-in languages (loaded once, shared).
    pub fn builtin() -> &'static Languages {
        static B: OnceLock<Languages> = OnceLock::new();
        B.get_or_init(|| {
            let mut set = Languages::new();
            for (id, src) in BUILTIN_SOURCES {
                let extra: &[(&str, &[&str])] = if *id == "rapidq-basic" { basic::KEYWORD_GROUPS } else { &[] };
                match set.load_with(src, extra) {
                    Ok(_) => {}
                    Err(e) => panic!("built-in language {id}: {e}"),
                }
            }
            set
        })
    }

    /// The built-in languages, in a set that more can be added to.
    pub fn with_builtins() -> Self {
        Languages::builtin().clone()
    }

    /// Loads a definition (replacing a language with the same id).
    pub fn load(&mut self, src: &str) -> Result<Arc<Language>, LangError> {
        self.load_with(src, &[])
    }

    fn load_with(&mut self, src: &str, extra: &[(&str, &[&str])]) -> Result<Arc<Language>, LangError> {
        let lang = {
            let resolve = |id: &str| self.get(id);
            Arc::new(Language::from_toml_with(src, &resolve, extra)?)
        };
        self.add(lang.clone());
        Ok(lang)
    }

    pub fn add(&mut self, lang: Arc<Language>) {
        self.langs.retain(|l| l.id != lang.id);
        self.langs.push(lang);
    }

    pub fn get(&self, id: &str) -> Option<Arc<Language>> {
        self.langs.iter().find(|l| l.id.eq_ignore_ascii_case(id)).cloned()
    }

    /// The language for a file path (by name, then extension); plain text
    /// when none claims it.
    pub fn for_path(&self, path: &str) -> Arc<Language> {
        let name = path.rsplit(['/', '\\']).next().unwrap_or(path);
        if let Some(l) = self.langs.iter().rev().find(|l| l.filenames.iter().any(|f| f.eq_ignore_ascii_case(name))) {
            return l.clone();
        }
        if let Some((_, ext)) = name.rsplit_once('.') {
            let ext = ext.to_ascii_lowercase();
            if let Some(l) = self.langs.iter().rev().find(|l| l.extensions.contains(&ext)) {
                return l.clone();
            }
        }
        self.plain_text()
    }

    /// Plain text (always there: a set without it makes an empty one).
    pub fn plain_text(&self) -> Arc<Language> {
        self.get("plaintext").unwrap_or_else(|| Languages::builtin().get("plaintext").expect("plain text is built in"))
    }

    pub fn iter(&self) -> impl Iterator<Item = &Arc<Language>> {
        self.langs.iter()
    }
}
