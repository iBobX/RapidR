//! RapidR's language registry: the one source of truth for what the
//! language has (docs/ide-plan.md, stage I0; decision D5).
//!
//! Every component (RapidQ's name and RapidR's), property, method, event,
//! builtin, statement, directive, constant and keyword, with its origin
//! (RapidQ's, or a RapidR extension), types, defaults, read-only and
//! design-time flags, categories, parameter lists, the default event and
//! short docs. From it come the compilers' component list
//! ([`COMPONENT_TYPES`]), the runtimes' name tests ([`is_component_type`],
//! [`is_component_method`]), and every generated description of the
//! language ([`export`]): the IDE's completion data, the VS Code
//! extension's data, the user manual's reference pages, the AI system
//! prompt.
//!
//! # The data
//!
//! `data/*.toml`, compiled into static tables by `build.rs` (which fails
//! the build on a mistake, naming the file and entry):
//!
//! - `components/<family>.toml`: `[[component]]`s — `name` (RapidR's,
//!   upper case), `rapidq` (RapidQ's name, when RapidQ has it), `from` (the
//!   include file RapidQ's name comes from, when it isn't RC.EXE's
//!   built-in), `aliases`, `kind` (`component` — the compilers create it;
//!   `library` — a BASIC library RapidR supplies; `planned` — RapidQ has
//!   it, RapidR not yet), `visual`, `container`, `size`, `default_event`,
//!   `only` (`desktop` / `web`), `sets` (members from sets.toml), `doc`, and
//!   its `properties`, `methods` and `events`, one inline table each:
//!   - property: `name`, `type` (int, float, string, bool, color, enum, set,
//!     font, component, picture, resource, item, any), `values` (an enum's or
//!     set's constants), `kinds` (a component property's types), `default`
//!     (a number, Boolean, text — or for other types a constant
//!     expression: `"alNone"`, `"akLeft + akTop"`), `access` (`read` /
//!     `write`; else both), `design` (shown at design time: default for
//!     read-write, non-indexed ones), `indexed` (1 or 2: `Item(i)`),
//!     `category`, `origin`, `from`, `only`, `missing`, `editor` (the
//!     inspector's editor beyond the type's: `strings`, `columns`, `file`,
//!     `picture`, `multiline`, `sql`, `expression`), `doc`;
//!   - method: `name`, `params`, `returns`, `value` (read without
//!     parentheses calls it: `IF Dlg.Execute THEN`), `origin`, `from`,
//!     `only`, `missing`, `test` (`skip: reason` or `args: …` for the
//!     conformance programs), `doc`;
//!   - event: `name`, `params`, `origin`, `from`, `only`, `missing`, `doc`.
//!
//!   `params`: `[BYREF] Name [AS Type]`, comma-separated, optional ones in
//!   `[ ]`, a last `...` for any number more (SUBI).
//!   `origin`: a member's family is its component's (RapidQ's components'
//!   members are RapidQ's) unless it says `rapidr`. `missing`: RapidQ has it,
//!   RapidR doesn't answer it yet on any runtime. `only`: one runtime
//!   answers it.
//! - `globals.toml`: the global objects (`[[object]]`, same fields).
//! - `items.toml`: what an indexed property gives (`Tree.Item(i)` is a
//!   TreeNode): `[[object]]`s that a property of `type = "item"` names in
//!   its `kinds`.
//! - `sets.toml`: members RapidR adds to every visual component.
//! - `glossary.toml`: docs and categories by member name, for members that
//!   don't have their own `doc`.
//! - `builtins.toml`, `language.toml` (statements, directives, keywords,
//!   type names), `constants.toml`.
//!
//! Every description is written in RapidR's own words (never RapidQ's
//! help); facts (names, types, defaults, parameter lists, which are
//! RapidQ's) come from RapidQ's manual, its KEYWORD.LST and RC.EXE itself
//! (tools/lang_seed.py says how the data was first made).
//!
//! The registry is tied to the runtimes by tests: tests/coverage.rs (every
//! name the compilers and the builtins table know is here), tests/reverse.rs
//! (every name the runtimes' dispatch answers is here), and the conformance
//! programs ([`conformance`]) every runtime runs (tests/lang_conformance.mjs).

pub mod conformance;
pub mod export;

/// Where a name comes from: RapidQ (its manual, its include files or its
/// compiler RC.EXE have it) or RapidR (an extension: RC.EXE refuses it).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Origin {
    RapidQ,
    RapidR,
}

/// What a registry component is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    /// The compilers create it ([`COMPONENT_TYPES`]).
    Component,
    /// A RapidQ library written in BASIC that RapidR supplies (QDOCKFORM).
    Library,
    /// RapidQ has it, RapidR doesn't yet (QOLEOBJECT).
    Planned,
    /// A global object, never created (Screen, Application).
    Global,
    /// The object an indexed property gives (Tree.Item(i): a TreeNode).
    Item,
}

/// Which runtimes answer a name (native builds and the interpreter are the
/// desktop).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Runtimes {
    All,
    Desktop,
    Web,
}

/// A property's type, as the IDE edits it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Type {
    Int,
    Float,
    String,
    Bool,
    Color,
    Enum,
    Set,
    Font,
    Component,
    Picture,
    Resource,
    /// An object of items.toml (`Item(i)` of a tree: a TreeNode).
    Item,
    Any,
}

/// A property's default.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum DefaultValue {
    Int(i64),
    Float(f64),
    Bool(bool),
    Str(&'static str),
    /// A constant expression: `alNone`, `akLeft + akTop`.
    Expr(&'static str),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Access {
    ReadWrite,
    Read,
    Write,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Property {
    pub name: &'static str,
    pub ty: Type,
    /// An enum's or a set's constants.
    pub values: &'static [&'static str],
    /// A component property's types (`QFORM`, `QPANEL` …).
    pub kinds: &'static [&'static str],
    pub default: Option<DefaultValue>,
    pub access: Access,
    /// Set at design time (the inspector shows it).
    pub design: bool,
    /// 1: `Item(i)`; 2: `Cell(c, r)`.
    pub indexed: u8,
    pub category: &'static str,
    pub origin: Origin,
    /// The include file a RapidQ member comes from (RAPIDQ2.INC).
    pub from: Option<&'static str>,
    pub runtimes: Runtimes,
    /// RapidQ has it; RapidR doesn't answer it yet.
    pub missing: bool,
    /// The extension set it came from (sets.toml).
    pub set: Option<&'static str>,
    /// The inspector's editor when its type's isn't enough: `strings` (a
    /// list of lines), `columns`, `file`, `picture`, `multiline`, `sql`,
    /// `expression` (RPropertyInspector).
    pub editor: Option<&'static str>,
    pub doc: &'static str,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Param {
    pub name: &'static str,
    /// BASIC's type (`INTEGER`, `STRING`, `QRECT` …), "" when any.
    pub ty: &'static str,
    pub optional: bool,
    pub byref: bool,
    /// Any number more of these (SUBI).
    pub variadic: bool,
    pub default: Option<&'static str>,
}

/// How the conformance programs call a method.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Test {
    /// With arguments made from its parameters.
    Call,
    /// Not called (why: it waits for the user, needs a device …).
    Skip(&'static str),
    /// With these arguments.
    Args(&'static str),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Method {
    pub name: &'static str,
    pub params: &'static [Param],
    pub returns: Option<&'static str>,
    /// Reading it without parentheses calls it (`WHILE DB.FetchRow`).
    pub value: bool,
    pub origin: Origin,
    pub from: Option<&'static str>,
    pub runtimes: Runtimes,
    pub missing: bool,
    pub test: Test,
    pub set: Option<&'static str>,
    pub doc: &'static str,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Event {
    pub name: &'static str,
    pub params: &'static [Param],
    pub origin: Origin,
    pub from: Option<&'static str>,
    pub runtimes: Runtimes,
    pub missing: bool,
    pub set: Option<&'static str>,
    pub doc: &'static str,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Component {
    /// RapidR's name (upper case: `RBUTTON`); a global object's as written (`Screen`).
    pub name: &'static str,
    /// Its name in mixed case without the Q / R, as RapidQ's documentation
    /// spells its classes (`CheckBox`, `StringGrid`, `DXScreen`): what a
    /// designer names new ones after (CheckBox1) and how tools show it
    /// ([`Component::pretty`]); a global object's name.
    pub display: &'static str,
    /// RapidQ's name, when RapidQ has the component.
    pub rapidq: Option<&'static str>,
    /// The include file RapidQ's name comes from, when it isn't RC.EXE's own.
    pub from: Option<&'static str>,
    /// Other names that mean it (QOUTLINE, COMPORT …).
    pub aliases: &'static [&'static str],
    pub kind: Kind,
    /// What it's for (the manual's and the toolbox's grouping).
    pub group: &'static str,
    /// RapidQ's (it has a RapidQ name) or RapidR's own.
    pub origin: Origin,
    pub visual: bool,
    pub container: bool,
    /// Width and height of a new one.
    pub size: Option<(u32, u32)>,
    pub default_event: Option<&'static str>,
    pub runtimes: Runtimes,
    /// Where it works, when it's more than one runtime and less than all.
    pub where_note: Option<&'static str>,
    /// A global object that is an instance of a component (Printer).
    pub instance_of: Option<&'static str>,
    pub doc: &'static str,
    pub properties: &'static [Property],
    pub methods: &'static [Method],
    pub events: &'static [Event],
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Builtin {
    /// As programs write it (`MID$`).
    pub name: &'static str,
    /// The runtimes' dispatch key (`mid`: rapidr_bytecode::builtins::builtin_key).
    pub key: &'static str,
    /// How it's called (`MID$(text, start [, count])`).
    pub syntax: &'static str,
    pub params: &'static [Param],
    pub returns: Option<&'static str>,
    pub group: &'static str,
    /// May be written without parentheses or arguments (`t = TIMER`).
    pub bare: bool,
    pub origin: Origin,
    pub runtimes: Runtimes,
    pub missing: bool,
    pub doc: &'static str,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Statement {
    pub name: &'static str,
    pub syntax: &'static str,
    pub group: &'static str,
    pub origin: Origin,
    pub doc: &'static str,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Directive {
    pub name: &'static str,
    pub syntax: &'static str,
    pub origin: Origin,
    pub doc: &'static str,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Keyword {
    pub name: &'static str,
    /// `operator`, `control` (THEN, NEXT, LOOP …: part of a flow statement)
    /// or `keyword`.
    pub kind: &'static str,
    pub origin: Origin,
    pub doc: &'static str,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TypeName {
    pub name: &'static str,
    pub origin: Origin,
    pub doc: &'static str,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ConstantGroup {
    pub name: &'static str,
    /// Where a program gets them: `RAPIDQ.INC`, a library include, `RapidR`.
    pub source: &'static str,
    pub origin: Origin,
    pub doc: &'static str,
    pub constants: &'static [(&'static str, i64)],
}

include!(concat!(env!("OUT_DIR"), "/registry.rs"));

/// The component one of these names means: RapidR's (`RBUTTON`), RapidQ's
/// (`QBUTTON`) or an alias (`QGAUGE`), any case. Global objects aren't
/// components ([`global`]).
pub fn component(name: &str) -> Option<&'static Component> {
    let upper = name.to_ascii_uppercase();
    BY_NAME.binary_search_by(|(n, _)| (*n).cmp(upper.as_str())).ok().map(|i| &COMPONENTS[BY_NAME[i].1])
}

/// The component the compilers read a type name as: [`component`], or a
/// Q-prefixed name RapidQ doesn't have read as RapidR's component of the
/// same name (`QPLOT` is RPLOT), as `rapidr_ast::canonical_type_name` does.
pub fn resolve_component(name: &str) -> Option<&'static Component> {
    component(name).or_else(|| {
        let rest = name.get(1..).filter(|_| name.starts_with(['Q', 'q']))?;
        component(&format!("R{rest}")).filter(|c| c.kind == Kind::Component)
    })
}

/// A global object by name (`Screen`, any case).
pub fn global(name: &str) -> Option<&'static Component> {
    GLOBALS.iter().find(|g| g.name.eq_ignore_ascii_case(name))
}

/// An item object by name (`TreeNode`, any case): what an indexed property
/// of type `item` gives.
pub fn item(name: &str) -> Option<&'static Component> {
    ITEMS.iter().find(|g| g.name.eq_ignore_ascii_case(name))
}

/// Whether `type_name` (any case) is a component the compilers create:
/// RapidR's name, as [`COMPONENT_TYPES`] lists it.
pub fn is_component_type(type_name: &str) -> bool {
    COMPONENT_TYPES.iter().any(|t| t.eq_ignore_ascii_case(type_name))
}

/// Whether `member` (any case) is a method of some component the compilers
/// create.
pub fn is_component_method(member: &str) -> bool {
    COMPONENTS.iter().filter(|c| c.kind == Kind::Component).any(|c| c.method(member).is_some())
}

/// A builtin by name or key (`MID$`, `mid`, `Mid`).
pub fn builtin(name: &str) -> Option<&'static Builtin> {
    let key = builtin_key(name);
    BUILTINS.iter().find(|b| b.key == key)
}

/// The runtimes' dispatch key of a builtin name: lower case, one BASIC type
/// suffix (`$ % # & !`) dropped.
pub fn builtin_key(name: &str) -> String {
    let mut key = name.to_ascii_lowercase();
    if key.len() > 1 && matches!(key.chars().last(), Some('$' | '%' | '#' | '&' | '!')) {
        key.pop();
    }
    key
}

/// A statement by its name (any case): `PRINT`, `SELECT CASE`, `PRINT #`.
pub fn statement(name: &str) -> Option<&'static Statement> {
    let name = name.split_whitespace().collect::<Vec<_>>().join(" ");
    STATEMENTS.iter().find(|s| s.name.eq_ignore_ascii_case(&name))
}

/// The statement a word starts (any case): its own (`PRINT`), else the one
/// whose name starts with it (`SELECT` → SELECT CASE, `LINE` → LINE INPUT).
pub fn statement_starting(word: &str) -> Option<&'static Statement> {
    statement(word).or_else(|| STATEMENTS.iter().find(|s| s.name.split(' ').next().is_some_and(|w| w.eq_ignore_ascii_case(word))))
}

/// A keyword by name (`THEN`, `AND`, `BYREF`; any case).
pub fn keyword(name: &str) -> Option<&'static Keyword> {
    KEYWORDS.iter().find(|k| k.name.eq_ignore_ascii_case(name))
}

/// A directive by name, with or without its `$` (`$INCLUDE`, `include`).
pub fn directive(name: &str) -> Option<&'static Directive> {
    let n = name.trim_start_matches('$');
    DIRECTIVES.iter().find(|d| d.name.trim_start_matches('$').eq_ignore_ascii_case(n))
}

/// A built-in type by name (`INTEGER`, `int64`).
pub fn type_name(name: &str) -> Option<&'static TypeName> {
    TYPE_NAMES.iter().find(|t| t.name.eq_ignore_ascii_case(name))
}

/// The words of the language a code editor colours, by kind (upper case,
/// without type suffixes; each list sorted, words in one list only):
/// `control` (the flow statements' words and THEN, NEXT, LOOP …),
/// `keyword` (the other statements' words and keywords), `type`,
/// `operator`, `constant` (TRUE, FALSE), `component` (RapidR's names),
/// `component_q` (RapidQ's names and aliases), `builtin` (what the runtimes
/// implement).
pub fn words(kind: &str) -> Vec<&'static str> {
    let statement_words = |flow: bool| -> Vec<&'static str> {
        STATEMENTS.iter().filter(|s| (s.group == "Flow") == flow).flat_map(|s| s.name.split(' ')).filter(|w| w.chars().all(|c| c.is_ascii_uppercase())).collect()
    };
    let mut w: Vec<&'static str> = match kind {
        "control" => {
            let mut v = statement_words(true);
            v.extend(KEYWORDS.iter().filter(|k| k.kind == "control").map(|k| k.name));
            v
        }
        "keyword" => {
            let mut v = statement_words(false);
            v.extend(KEYWORDS.iter().filter(|k| k.kind == "keyword").map(|k| k.name));
            v
        }
        "type" => TYPE_NAMES.iter().map(|t| t.name).collect(),
        "operator" => KEYWORDS.iter().filter(|k| k.kind == "operator").map(|k| k.name).collect(),
        "constant" => vec!["TRUE", "FALSE"],
        "component" => COMPONENT_TYPES.to_vec(),
        "component_q" => COMPONENTS.iter().filter(|c| c.kind == Kind::Component).flat_map(|c| c.rapidq.into_iter().chain(c.aliases.iter().copied())).collect(),
        "builtin" => BUILTINS.iter().filter(|b| !b.missing).map(|b| b.name.trim_end_matches(['$', '%', '#', '&', '!'])).collect(),
        _ => Vec::new(),
    };
    w.sort_unstable();
    w.dedup();
    // (a word in two lists belongs to the first: the flow's NEXT isn't a keyword too)
    let earlier: Vec<&'static str> = match kind {
        "keyword" => words("control"),
        "type" => [words("control"), words("keyword")].concat(),
        "operator" => [words("control"), words("keyword"), words("type")].concat(),
        "builtin" => [words("control"), words("keyword"), words("type"), words("operator")].concat(),
        _ => Vec::new(),
    };
    w.retain(|x| !earlier.contains(x));
    w
}

/// A constant's value by name (any case), and its group.
pub fn constant(name: &str) -> Option<(i64, &'static ConstantGroup)> {
    CONSTANT_GROUPS.iter().find_map(|g| g.constants.iter().find(|(n, _)| n.eq_ignore_ascii_case(name)).map(|(_, v)| (*v, g)))
}

/// A constant expression's value (`akLeft + akTop`, `alNone`, `5`).
pub fn eval_constant(expr: &str) -> Option<i64> {
    let mut total = 0i64;
    for part in expr.split(['+', '|']).map(str::trim) {
        total += match part.parse::<i64>() {
            Ok(n) => n,
            Err(_) => constant(part)?.0,
        };
    }
    Some(total)
}

impl Component {
    /// A member property by name (any case).
    pub fn property(&self, name: &str) -> Option<&'static Property> {
        self.properties.iter().find(|p| p.name.eq_ignore_ascii_case(name))
    }

    pub fn method(&self, name: &str) -> Option<&'static Method> {
        self.methods.iter().find(|m| m.name.eq_ignore_ascii_case(name))
    }

    pub fn event(&self, name: &str) -> Option<&'static Event> {
        self.events.iter().find(|e| e.name.eq_ignore_ascii_case(name))
    }

    /// Whether `name` is any of its members.
    pub fn has_member(&self, name: &str) -> bool {
        self.property(name).is_some() || self.method(name).is_some() || self.event(name).is_some()
    }

    /// How RapidR writes it: its R name in mixed case (`RButton`,
    /// `RStringGrid`, `RDXScreen`), the preferred spelling everywhere —
    /// docs, completion, hovers, the RapidQ importer
    /// (docs/q-and-r-components.md §1). A component RapidR has no R name for
    /// (an include library's TYPE, a planned one: `QDockForm`) and a global
    /// object (`Screen`) as they are named.
    pub fn spelling(&self) -> String {
        self.pretty(self.name)
    }

    /// Its RapidQ name in mixed case (`QButton`), when RapidQ has it.
    pub fn rapidq_spelling(&self) -> Option<String> {
        self.rapidq.map(|q| self.pretty(q))
    }

    /// The name a RapidQ-style file is written with: RapidQ's for RapidQ's
    /// components (upper case, as RapidQ's own programs write them), RapidR's
    /// for its own. Only code added to a file written with RapidQ's names
    /// uses it ([`Component::name_in`]); everything else writes
    /// [`Component::spelling`].
    pub fn written_name(&self) -> &'static str {
        self.rapidq.unwrap_or(self.name)
    }

    /// The name code added to a file is written with, in the file's own
    /// style (docs/ide-plan.md, R-NAMES): RapidR's (`RButton`), or, in a
    /// file written with RapidQ's names, RapidQ's (`QBUTTON`) — so a file
    /// never mixes the two.
    pub fn name_in(&self, style: NameStyle) -> String {
        match style {
            NameStyle::RapidQ => self.written_name().to_string(),
            _ => self.spelling(),
        }
    }

    /// One of its names (`QCHECKBOX`, `rcheckbox`) in mixed case
    /// (`QCheckBox`, `RCheckBox`); a name that isn't Q / R + its stem (an
    /// alias) as it is.
    pub fn pretty(&self, written: &str) -> String {
        if self.kind == Kind::Global {
            return self.display.to_string();
        }
        match written.split_at_checked(1) {
            Some((first, rest)) if rest.eq_ignore_ascii_case(self.display) => format!("{}{}", first.to_ascii_uppercase(), self.display),
            _ => written.to_string(),
        }
    }

    /// Its global object's component, for an instance (Printer → RPRINTER).
    pub fn instance_component(&self) -> Option<&'static Component> {
        self.instance_of.and_then(component)
    }
}

/// How a file writes the names of the components RapidQ has too: what
/// RapidR Studio's designer and completion follow, so a file never mixes
/// the two (docs/ide-plan.md, R-NAMES). Counted over the file's type names
/// ([`NameCounts`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum NameStyle {
    /// RapidQ's names (`QBUTTON`): a RapidQ program — or a file using
    /// more of RapidQ's names than RapidR's.
    RapidQ,
    /// RapidR's names (`RButton`), or none yet: RapidR's default.
    #[default]
    RapidR,
    /// Both (as many of each, or counted without deciding).
    Mixed,
}

/// How many of a file's type names name one of RapidQ's components by
/// RapidQ's name (`QBUTTON`) and by RapidR's (`RButton`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct NameCounts {
    pub rapidq: usize,
    pub rapidr: usize,
}

impl NameCounts {
    /// Counts one name as written (`QBUTTON`, `RButton`, `QGauge`): a name
    /// of a RapidQ component, one way or the other; anything else is left.
    pub fn count(&mut self, written: &str) {
        let Some(c) = component(written).filter(|c| c.rapidq.is_some() && c.kind == Kind::Component) else { return };
        if written.eq_ignore_ascii_case(c.name) {
            self.rapidr += 1;
        } else {
            self.rapidq += 1;
        }
    }

    /// Strictly what the file has: RapidQ's names only, RapidR's only (or
    /// none), or both.
    pub fn style(&self) -> NameStyle {
        match (self.rapidq, self.rapidr) {
            (0, _) => NameStyle::RapidR,
            (_, 0) => NameStyle::RapidQ,
            _ => NameStyle::Mixed,
        }
    }

    /// The style new code is written in: the names the file uses most,
    /// RapidR's on a tie (and in a new or empty file).
    pub fn writing_style(&self) -> NameStyle {
        if self.rapidq > self.rapidr { NameStyle::RapidQ } else { NameStyle::RapidR }
    }
}

impl Param {
    /// `[BYREF] Name [AS Type]`, in brackets when optional.
    pub fn text(&self) -> String {
        let mut s = String::new();
        if self.byref {
            s.push_str("BYREF ");
        }
        s.push_str(self.name);
        if !self.ty.is_empty() {
            s.push_str(" AS ");
            s.push_str(self.ty);
        }
        if let Some(d) = self.default {
            s.push_str(" = ");
            s.push_str(d);
        }
        if self.variadic {
            s.push_str(", …");
        }
        if self.optional {
            s = format!("[{s}]");
        }
        s
    }
}

/// `(a AS INTEGER, [b AS STRING])`, or "" for none.
pub fn params_text(params: &[Param]) -> String {
    if params.is_empty() {
        return String::new();
    }
    format!("({})", params.iter().map(Param::text).collect::<Vec<_>>().join(", "))
}

impl Method {
    /// `Name(params) [AS type]`.
    pub fn signature(&self) -> String {
        let mut s = format!("{}{}", self.name, params_text(self.params));
        if let Some(r) = self.returns {
            s.push_str(" AS ");
            s.push_str(r);
        }
        s
    }
}

impl Event {
    /// `Name(params)`: the handler's parameters.
    pub fn signature(&self) -> String {
        format!("{}{}", self.name, params_text(self.params))
    }
}

impl Origin {
    pub fn as_str(self) -> &'static str {
        match self {
            Origin::RapidQ => "rapidq",
            Origin::RapidR => "rapidr",
        }
    }
}

impl Runtimes {
    pub fn as_str(self) -> &'static str {
        match self {
            Runtimes::All => "all",
            Runtimes::Desktop => "desktop",
            Runtimes::Web => "web",
        }
    }
}

impl Type {
    pub fn as_str(self) -> &'static str {
        match self {
            Type::Int => "int",
            Type::Float => "float",
            Type::String => "string",
            Type::Bool => "bool",
            Type::Color => "color",
            Type::Enum => "enum",
            Type::Set => "set",
            Type::Font => "font",
            Type::Component => "component",
            Type::Picture => "picture",
            Type::Resource => "resource",
            Type::Item => "item",
            Type::Any => "any",
        }
    }
}

impl Kind {
    pub fn as_str(self) -> &'static str {
        match self {
            Kind::Component => "component",
            Kind::Library => "library",
            Kind::Planned => "planned",
            Kind::Global => "global",
            Kind::Item => "item",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_find_components() {
        let b = component("QBUTTON").unwrap();
        assert_eq!(b.name, "RBUTTON");
        assert_eq!(component("rbutton").unwrap().name, "RBUTTON");
        assert_eq!(component("QGAUGE").unwrap().name, "RPROGRESSBAR");
        assert_eq!(component("qoutline").unwrap().name, "RTREEVIEW");
        assert_eq!(component("COMPORT").unwrap().name, "RCOMPORT");
        assert!(component("QNOTHING").is_none());
        assert!(component("QPLOT").is_none(), "not one of its names");
        assert_eq!(resolve_component("QPlot").unwrap().name, "RPLOT");
        assert_eq!(resolve_component("QBUTTON").unwrap().name, "RBUTTON");
        assert!(resolve_component("QNOTHING").is_none());
        assert_eq!(b.written_name(), "QBUTTON");
        assert_eq!(component("RPLOT").unwrap().written_name(), "RPLOT");
        assert_eq!(b.spelling(), "RButton");
        assert_eq!(b.rapidq_spelling().as_deref(), Some("QButton"));
        assert_eq!(component("QSTRINGGRID").unwrap().spelling(), "RStringGrid");
        assert_eq!(component("QGAUGE").unwrap().spelling(), "RProgressBar");
        assert_eq!(component("RPLOT").unwrap().spelling(), "RPlot");
        assert_eq!(component("RPLOT").unwrap().rapidq_spelling(), None);
        assert_eq!(component("QDOCKFORM").unwrap().spelling(), "QDockForm");
        assert_eq!(global("screen").unwrap().spelling(), "Screen");
        assert_eq!(b.name_in(NameStyle::RapidR), "RButton");
        assert_eq!(b.name_in(NameStyle::Mixed), "RButton");
        assert_eq!(b.name_in(NameStyle::RapidQ), "QBUTTON");
        assert_eq!(component("RPLOT").unwrap().name_in(NameStyle::RapidQ), "RPLOT");
    }

    #[test]
    fn name_styles_count_rapidq_components_only() {
        let mut n = NameCounts::default();
        assert_eq!((n.style(), n.writing_style()), (NameStyle::RapidR, NameStyle::RapidR));
        for w in ["QFORM", "QButton", "qgauge", "RPLOT", "INTEGER", "QNOTHING"] {
            n.count(w);
        }
        assert_eq!(n, NameCounts { rapidq: 3, rapidr: 0 }, "RPLOT is RapidR's own: no style");
        assert_eq!((n.style(), n.writing_style()), (NameStyle::RapidQ, NameStyle::RapidQ));
        for w in ["RButton", "RFORM", "rlabel"] {
            n.count(w);
        }
        assert_eq!((n.style(), n.writing_style()), (NameStyle::Mixed, NameStyle::RapidR), "a tie: RapidR's");
    }

    #[test]
    fn component_types_are_the_created_ones() {
        assert!(is_component_type("RFORM") && is_component_type("rbutton"));
        assert!(!is_component_type("QFORM"), "RapidR's names only");
        assert!(!is_component_type("QDOCKFORM") && !is_component_type("QOLEOBJECT"));
        for t in COMPONENT_TYPES {
            assert_eq!(component(t).unwrap().kind, Kind::Component);
        }
    }

    #[test]
    fn members_and_origins() {
        let form = component("QFORM").unwrap();
        assert_eq!(form.property("caption").unwrap().origin, Origin::RapidQ);
        assert_eq!(form.property("Anchors").unwrap().origin, Origin::RapidR);
        assert_eq!(form.property("Anchors").unwrap().set, Some("layout"));
        assert!(is_component_method("ShowModal"));
        assert!(!is_component_method("Frobnicate"));
        assert_eq!(component("RPLOT").unwrap().origin, Origin::RapidR);
    }

    #[test]
    fn constants() {
        assert_eq!(constant("alClient").unwrap().0, 5);
        assert_eq!(eval_constant("akLeft + akTop"), Some(3));
        assert_eq!(constant("clBtnFace").unwrap().1.source, "RAPIDQ.INC");
    }

    #[test]
    fn builtins() {
        assert_eq!(builtin("MID$").unwrap().key, "mid");
        assert_eq!(builtin("mid").unwrap().name, "MID$");
        assert!(builtin("TIMER").unwrap().bare);
    }

    #[test]
    fn language_words() {
        assert_eq!(statement("select  case").unwrap().name, "SELECT CASE");
        assert!(statement("SELECT").is_none());
        assert_eq!(statement_starting("select").unwrap().name, "SELECT CASE");
        assert_eq!(statement_starting("Print").unwrap().name, "PRINT");
        assert_eq!(statement("print #").unwrap().origin, Origin::RapidR);
        assert_eq!(keyword("then").unwrap().kind, "control");
        assert_eq!(directive("include").unwrap().name, "$INCLUDE");
        assert_eq!(directive("$Theme").unwrap().origin, Origin::RapidR);
        assert_eq!(type_name("int64").unwrap().origin, Origin::RapidR);
        assert_eq!(type_name("INTEGER").unwrap().origin, Origin::RapidQ);
    }
}
