//! A demo script (docs/video-pipeline.md): JSON with `//` comment lines,
//! its texts in one language or several (`{"en": "…", "es": "…"}`
//! anywhere a text goes), steps of narration and actions.
//!
//! ```text
//! { "id": "first-app", "title": {"en": "Your first app", "es": "Tu primera app"},
//!   "screen": [1280, 720], "scale": 1.5, "fps": 30,
//!   "steps": [
//!     { "card": { "title": "Your first app" }, "say": ["Let's make an app."] },
//!     { "say": ["Drag a button onto the form."],
//!       "do": [ { "drag": "Toolbox > RButton", "to": "designdoc(0)", "cue": "drop" } ] } ] }
//! ```

use serde_json::{Map, Value as Json};

/// A whole script, in one language.
#[derive(Clone, Debug, PartialEq)]
pub struct Script {
    pub id: String,
    pub title: String,
    /// The frame (logical pixels) and its scale: 1280 × 720 at 1.5 is a
    /// 1920 × 1080 video.
    pub screen: (i64, i64),
    pub scale: f64,
    pub fps: f64,
    /// The program's main window, the frame's base (`studio`).
    pub window: String,
    /// How fast the pointer glides (1 normal, 2 twice as fast) and how big
    /// it is.
    pub speed: f64,
    pub pointer_size: f64,
    /// How fast typing goes (1 natural, about 11 characters a second).
    pub typing: f64,
    /// The captions' text height (logical pixels; 0: none drawn).
    pub caption_px: i64,
    pub steps: Vec<Step>,
}

/// Narration lines and the actions played meanwhile; the step ends when
/// both are done, then `gap_ms` more.
#[derive(Clone, Debug, PartialEq, Default)]
pub struct Step {
    pub say: Vec<Line>,
    pub actions: Vec<Action>,
    pub gap_ms: u64,
    pub card: Option<CardSpec>,
}

/// One sentence: what the caption shows, how long it is said for, and the
/// cue it waits for.
#[derive(Clone, Debug, PartialEq)]
pub struct Line {
    /// The caption's words (`{shown|spoken}` resolved to "shown").
    pub text: String,
    /// What the voice says (the "spoken" side).
    pub spoken: String,
    /// How long it is said: the narration clip's length (the video
    /// pipeline measures it), else an estimate from its words.
    pub ms: u64,
    /// It starts only once an action fired this cue.
    pub cue: Option<String>,
    /// The caption shown (false: said only).
    pub captions: bool,
    /// The narration clip (passed through to the timeline).
    pub clip: Option<String>,
}

/// A title or end card.
#[derive(Clone, Debug, PartialEq, Default)]
pub struct CardSpec {
    pub title: String,
    pub lines: Vec<String>,
    pub small: Vec<String>,
    pub background: Option<String>,
    pub logo: Option<String>,
    /// The logo's height (logical pixels).
    pub logo_height: i64,
    pub light_text: bool,
}

/// What an action acts on.
#[derive(Clone, Debug, PartialEq, Default)]
pub struct Target {
    /// In the program RapidR Studio runs (`app:`), not in Studio.
    pub app: bool,
    /// The component: RAppHands' words — a name, a caption, a label
    /// (`caption:Run`, `name:Button1`, `label:Name`).
    pub what: String,
    /// An item inside it, by what it shows (`Toolbox > RButton`: the
    /// accessibility tree's names — a list's rows, a tree's items).
    pub item: Option<String>,
    /// Where in it (0 to 1 across and down; the middle by default).
    pub fx: f64,
    pub fy: f64,
    /// A point of a window instead (`{"window": "studio", "at": [x, y]}`).
    pub window: Option<String>,
    pub at: Option<(f64, f64)>,
}

/// When `waitFor` is over.
#[derive(Clone, Debug, PartialEq)]
pub enum Cond {
    /// A window shows (a form's name or title).
    Window(String),
    /// It's gone.
    Gone(String),
    /// A target can be found (shown, in a window that takes input).
    Shows(Target),
    /// `component.property` reads `value` (`exact`) or has it in it.
    Prop { path: String, value: String, exact: bool },
    /// The program run from Studio shows a window, or none any more.
    App(bool),
}

#[derive(Clone, Debug, PartialEq)]
pub enum Act {
    Move(Target),
    Click { target: Target, double: bool, right: bool },
    Drag { from: Target, to: Target },
    Type { into: Option<Target>, text: String },
    Key { into: Option<Target>, keys: String },
    Scroll { target: Target, by: f64 },
    Wait(u64),
    WaitFor { cond: Cond, timeout_ms: u64 },
    /// A ring (or a spotlight) around it; `None` clears it; `for_ms`: for
    /// that long only.
    Highlight { target: Option<Target>, spotlight: bool, for_ms: Option<u64> },
    /// RapidR Studio's own commands (`file.saveAll`, `open:main.rr` …).
    Do(String),
    /// A named moment (`poster`: the video's poster frame).
    Mark(String),
    /// The pointer shown or hidden.
    Pointer(bool),
}

/// An action and when it plays.
#[derive(Clone, Debug, PartialEq)]
pub struct Action {
    pub act: Act,
    /// Fired when the pointer arrives (or when the action starts, for one
    /// without a pointer): the narration line waiting for it starts.
    pub cue: Option<String>,
    /// It waits until the step's line n (1 is the first) has been said.
    pub after_line: Option<usize>,
    /// A pause after it (ms).
    pub pause_ms: u64,
    /// How long the pointer takes to get there (else from the distance).
    pub glide_ms: Option<u64>,
}

/// The words a line takes to say when no clip measured it: about 2.6
/// words a second, at least 1.2 s.
pub fn estimate_ms(spoken: &str) -> u64 {
    let words = spoken.split_whitespace().count().max(1) as f64;
    let chars = spoken.chars().count() as f64;
    // (a long word takes longer: the characters count too)
    ((words * 330.0 + chars * 12.0).max(1200.0)).round() as u64
}

/// `{shown|spoken}` resolved: what the caption shows, what the voice says.
pub fn markup(text: &str) -> (String, String) {
    let mut shown = String::new();
    let mut spoken = String::new();
    let mut rest = text;
    while let Some(open) = rest.find('{') {
        let Some(close) = rest[open..].find('}').map(|c| open + c) else { break };
        let inner = &rest[open + 1..close];
        let Some(bar) = inner.find('|') else {
            shown.push_str(&rest[..=close]);
            spoken.push_str(&rest[..=close]);
            rest = &rest[close + 1..];
            continue;
        };
        shown.push_str(&rest[..open]);
        spoken.push_str(&rest[..open]);
        shown.push_str(&inner[..bar]);
        spoken.push_str(&inner[bar + 1..]);
        rest = &rest[close + 1..];
    }
    shown.push_str(rest);
    spoken.push_str(rest);
    (shown, spoken)
}

/// Lines starting with `//` (after spaces) dropped: comments.
pub fn strip_comments(text: &str) -> String {
    text.lines().map(|l| if l.trim_start().starts_with("//") { "" } else { l }).collect::<Vec<_>>().join("\n")
}

/// The languages a script's texts may be in (never a key an action uses:
/// `do`, `to`, `at`, `by` aren't here).
const LANGS: [&str; 24] = ["en", "es", "pt", "zh", "fr", "de", "it", "ja", "ko", "nl", "ru", "ar", "hi", "ca", "gl", "eu", "pl", "tr", "uk", "sv", "da", "fi", "nb", "xx"];

/// Whether `k` is a language's tag (`en`, `es`, `pt-BR`, `zh`).
fn is_lang(k: &str) -> bool {
    let (a, b) = k.split_once('-').unwrap_or((k, ""));
    LANGS.contains(&a) && (b.is_empty() || (b.len() == 2 && b.chars().all(|c| c.is_ascii_uppercase())) || b.len() == 4)
}

/// Every `{"en": …, "es": …}` in `v` replaced by its `lang` text (else
/// English's, else the first): the script in one language.
pub fn resolve_lang(v: &Json, lang: &str) -> Json {
    match v {
        Json::Object(m) if !m.is_empty() && m.keys().all(|k| is_lang(k)) => {
            let base = lang.split('-').next().unwrap_or(lang);
            let pick = m.get(lang).or_else(|| m.get(base)).or_else(|| m.iter().find(|(k, _)| k.split('-').next() == Some(base)).map(|(_, v)| v)).or_else(|| m.get("en")).or_else(|| m.values().next());
            pick.map(|p| resolve_lang(p, lang)).unwrap_or(Json::Null)
        }
        Json::Object(m) => Json::Object(m.iter().map(|(k, v)| (k.clone(), resolve_lang(v, lang))).collect()),
        Json::Array(a) => Json::Array(a.iter().map(|v| resolve_lang(v, lang)).collect()),
        other => other.clone(),
    }
}

/// A script from its text, in `lang`.
pub fn parse(text: &str, lang: &str) -> Result<Script, String> {
    let raw: Json = serde_json::from_str(&strip_comments(text)).map_err(|e| format!("The demo script isn't valid JSON: {e}"))?;
    let v = resolve_lang(&raw, lang);
    let o = v.as_object().ok_or("The demo script is a JSON object: { \"steps\": [ … ] }")?;
    let num = |k: &str, d: f64| o.get(k).and_then(Json::as_f64).unwrap_or(d);
    let screen = match o.get("screen") {
        Some(Json::Array(a)) if a.len() == 2 => (a[0].as_i64().unwrap_or(1280), a[1].as_i64().unwrap_or(720)),
        Some(_) => return Err("\"screen\" is [width, height] in logical pixels (1280, 720)".into()),
        None => (1280, 720),
    };
    if screen.0 < 320 || screen.1 < 240 || screen.0 > 7680 || screen.1 > 4320 {
        return Err(format!("\"screen\" {} × {} is out of range (320 × 240 to 7680 × 4320)", screen.0, screen.1));
    }
    let pointer = o.get("pointer").and_then(Json::as_object);
    let pnum = |k: &str, d: f64| pointer.and_then(|p| p.get(k)).and_then(Json::as_f64).unwrap_or(d);
    let caption_px = o.get("captions").and_then(|c| c.get("size")).and_then(Json::as_i64).unwrap_or((screen.1 as f64 * 0.03).round() as i64);
    let steps = o.get("steps").and_then(Json::as_array).ok_or("The demo script needs \"steps\": [ … ]")?;
    let mut out = Vec::new();
    for (i, s) in steps.iter().enumerate() {
        out.push(parse_step(s).map_err(|e| format!("step {}: {e}", i + 1))?);
    }
    Ok(Script {
        id: o.get("id").and_then(Json::as_str).unwrap_or("demo").to_string(),
        title: text_of(o.get("title")).unwrap_or_default(),
        screen,
        scale: num("scale", 1.0).clamp(0.5, 4.0),
        fps: num("fps", 30.0).clamp(1.0, 120.0),
        window: o.get("window").and_then(Json::as_str).unwrap_or("studio").to_lowercase(),
        speed: pnum("speed", 1.0).clamp(0.1, 10.0),
        pointer_size: pnum("size", 1.3).clamp(0.5, 4.0),
        typing: num("typing", 1.0).clamp(0.1, 10.0),
        caption_px: caption_px.clamp(0, 200),
        steps: out,
    })
}

/// A text: a string, or `{ "text": … }`.
fn text_of(v: Option<&Json>) -> Option<String> {
    match v? {
        Json::String(s) => Some(s.clone()),
        Json::Object(m) => m.get("text").and_then(Json::as_str).map(str::to_string),
        _ => None,
    }
}

const VERBS: [&str; 18] = ["move", "click", "double", "right", "drag", "type", "key", "scroll", "wait", "waitFor", "highlight", "spotlight", "do", "open", "mark", "pointer", "hover", "press"];

fn parse_step(v: &Json) -> Result<Step, String> {
    let o = v.as_object().ok_or("a step is an object: { \"say\": [ … ], \"do\": [ … ] }")?;
    let mut step = Step { gap_ms: secs(o.get("gap")).unwrap_or(450), ..Step::default() };
    match o.get("say") {
        None => {}
        Some(Json::Array(lines)) => {
            for (i, l) in lines.iter().enumerate() {
                step.say.push(parse_line(l).map_err(|e| format!("line {}: {e}", i + 1))?);
            }
        }
        Some(l) => step.say.push(parse_line(l)?),
    }
    if let Some(c) = o.get("card") {
        step.card = Some(parse_card(c)?);
    }
    match o.get("do") {
        Some(Json::Array(acts)) => {
            for (i, a) in acts.iter().enumerate() {
                step.actions.push(parse_action(a).map_err(|e| format!("action {}: {e}", i + 1))?);
            }
        }
        // ("do": "file.saveAll" — a Studio command, as an action's own)
        Some(Json::String(_)) => step.actions.push(parse_action(v)?),
        Some(_) => return Err("\"do\" is a list of actions".into()),
        None => {
            // (the shorthand: one action written in the step itself)
            if VERBS.iter().any(|k| o.contains_key(*k)) {
                step.actions.push(parse_action(v)?);
            }
        }
    }
    Ok(step)
}

fn parse_line(v: &Json) -> Result<Line, String> {
    let (text, o) = match v {
        Json::String(s) => (s.clone(), None),
        Json::Object(m) => (m.get("text").and_then(Json::as_str).ok_or("a line is a text or { \"text\": … }")?.to_string(), Some(m)),
        _ => return Err("a line is a text or { \"text\": … }".into()),
    };
    let (shown, spoken) = markup(&text);
    let get = |k: &str| o.and_then(|m| m.get(k));
    let ms = get("ms").and_then(Json::as_u64).unwrap_or_else(|| estimate_ms(&spoken));
    Ok(Line {
        text: shown,
        spoken,
        ms,
        cue: get("cue").and_then(Json::as_str).map(str::to_string),
        captions: get("captions").and_then(Json::as_bool).unwrap_or(true),
        clip: get("clip").and_then(Json::as_str).map(str::to_string),
    })
}

fn parse_card(v: &Json) -> Result<CardSpec, String> {
    let o = v.as_object().ok_or("a card is { \"title\": …, \"lines\": [ … ] }")?;
    let list = |k: &str| o.get(k).and_then(Json::as_array).map(|a| a.iter().filter_map(|s| s.as_str().map(str::to_string)).collect()).unwrap_or_default();
    Ok(CardSpec {
        title: o.get("title").and_then(Json::as_str).unwrap_or_default().to_string(),
        lines: list("lines"),
        small: list("small"),
        background: o.get("background").and_then(Json::as_str).map(str::to_string),
        logo: o.get("logo").and_then(Json::as_str).map(str::to_string),
        logo_height: o.get("logoHeight").and_then(Json::as_i64).unwrap_or(72),
        light_text: o.get("lightText").and_then(Json::as_bool).unwrap_or(o.contains_key("background")),
    })
}

fn secs(v: Option<&Json>) -> Option<u64> {
    v.and_then(Json::as_f64).map(|s| (s.max(0.0) * 1000.0).round() as u64)
}

/// A target as written: a string, or `{ "target": …, "x": 0.8 }`, or
/// `{ "window": …, "at": [x, y] }`.
pub fn parse_target(v: &Json) -> Result<Target, String> {
    match v {
        Json::String(s) => Ok(target_of(s)),
        Json::Object(m) => {
            let mut t = match m.get("target") {
                Some(Json::String(s)) => target_of(s),
                Some(_) => return Err("\"target\" is a text".into()),
                None => Target { fx: 0.5, fy: 0.5, ..Target::default() },
            };
            if let Some(x) = m.get("x").and_then(Json::as_f64) {
                t.fx = x;
            }
            if let Some(y) = m.get("y").and_then(Json::as_f64) {
                t.fy = y;
            }
            if let Some(w) = m.get("window").and_then(Json::as_str) {
                t.window = Some(w.to_string());
            }
            if let Some(Json::Array(a)) = m.get("at") {
                if a.len() == 2 {
                    t.at = Some((a[0].as_f64().unwrap_or(0.0), a[1].as_f64().unwrap_or(0.0)));
                }
            }
            if m.get("app").and_then(Json::as_bool) == Some(true) {
                t.app = true;
            }
            if t.what.is_empty() && t.at.is_none() {
                return Err("a target names a component (\"target\": …) or a point (\"window\": …, \"at\": [x, y])".into());
            }
            Ok(t)
        }
        _ => Err("a target is a text (a name, \"caption:Run\", \"Toolbox > RButton\", \"app:Button1\") or an object".into()),
    }
}

/// `app:Button1`, `Toolbox > RButton`, `caption:Run`.
pub fn target_of(s: &str) -> Target {
    let mut t = s.trim();
    let mut app = false;
    if t.len() >= 4 && t[..4].eq_ignore_ascii_case("app:") {
        app = true;
        t = t[4..].trim();
    }
    let (what, item) = match t.split_once(" > ") {
        Some((a, b)) => (a.trim().to_string(), Some(b.trim().to_string())),
        None => (t.to_string(), None),
    };
    Target { app, what, item, fx: 0.5, fy: 0.5, window: None, at: None }
}

fn cond_of(v: &Json) -> Result<Cond, String> {
    match v {
        Json::String(s) => {
            let (k, rest) = s.split_once(':').ok_or("a condition is \"window:NAME\", \"gone:NAME\", \"shows:TARGET\" or an object")?;
            match k.trim() {
                "window" => Ok(Cond::Window(rest.trim().into())),
                "gone" => Ok(Cond::Gone(rest.trim().into())),
                "shows" => Ok(Cond::Shows(target_of(rest))),
                "app" => Ok(Cond::App(rest.trim() != "gone")),
                other => Err(format!("\"{other}\" isn't a condition: window, gone, shows, app")),
            }
        }
        Json::Object(m) => {
            if let Some(w) = m.get("window").and_then(Json::as_str) {
                return Ok(Cond::Window(w.into()));
            }
            if let Some(w) = m.get("gone").and_then(Json::as_str) {
                return Ok(Cond::Gone(w.into()));
            }
            if let Some(t) = m.get("shows") {
                return Ok(Cond::Shows(parse_target(t)?));
            }
            if let Some(a) = m.get("app").and_then(Json::as_bool) {
                return Ok(Cond::App(a));
            }
            if let Some(p) = m.get("prop").and_then(Json::as_str) {
                if !p.contains('.') {
                    return Err("\"prop\" is component.property (studio.caption)".into());
                }
                let val = |k: &str| m.get(k).map(|v| v.as_str().map_or_else(|| v.to_string(), str::to_string));
                if let Some(v) = val("is") {
                    return Ok(Cond::Prop { path: p.into(), value: v, exact: true });
                }
                if let Some(v) = val("has") {
                    return Ok(Cond::Prop { path: p.into(), value: v, exact: false });
                }
                return Err("\"prop\" needs \"is\" or \"has\"".into());
            }
            Err("a condition is { \"window\" }, { \"gone\" }, { \"shows\" }, { \"prop\", \"is\" | \"has\" } or { \"app\" }".into())
        }
        _ => Err("a condition is a text or an object".into()),
    }
}

fn parse_action(v: &Json) -> Result<Action, String> {
    let o: &Map<String, Json> = v.as_object().ok_or("an action is an object: { \"click\": \"caption:Run\" }")?;
    let verb = VERBS.iter().find(|k| o.contains_key(**k)).ok_or_else(|| format!("no action in it: one of {}", VERBS.join(", ")))?;
    let val = &o[*verb];
    let opt_target = |k: &str| o.get(k).map(parse_target).transpose();
    let act = match *verb {
        "move" | "hover" => Act::Move(parse_target(val)?),
        "click" | "press" => Act::Click { target: parse_target(val)?, double: false, right: false },
        "double" => Act::Click { target: parse_target(val)?, double: true, right: false },
        "right" => Act::Click { target: parse_target(val)?, double: false, right: true },
        "drag" => Act::Drag { from: parse_target(val)?, to: parse_target(o.get("to").ok_or("a drag needs \"to\"")?)? },
        "type" => Act::Type { into: opt_target("into")?, text: val.as_str().ok_or("\"type\" is the text typed")?.to_string() },
        "key" => Act::Key { into: opt_target("into")?, keys: val.as_str().ok_or("\"key\" is the keys (Enter, Ctrl+S, Tab Tab)")?.to_string() },
        "scroll" => Act::Scroll { target: parse_target(val)?, by: o.get("by").and_then(Json::as_f64).unwrap_or(3.0) },
        "wait" => Act::Wait(secs(Some(val)).ok_or("\"wait\" is seconds")?),
        "waitFor" => Act::WaitFor { cond: cond_of(val)?, timeout_ms: secs(o.get("timeout")).unwrap_or(30_000) },
        "highlight" | "spotlight" => Act::Highlight {
            target: if val.is_null() || val == &Json::Bool(false) { None } else { Some(parse_target(val)?) },
            spotlight: *verb == "spotlight" || o.get("spotlight").and_then(Json::as_bool) == Some(true),
            for_ms: secs(o.get("for")),
        },
        "do" => Act::Do(val.as_str().ok_or("\"do\" is Studio's commands, separated by commas")?.to_string()),
        "open" => Act::Do(format!("open:{}", val.as_str().ok_or("\"open\" is a file")?)),
        "mark" => Act::Mark(val.as_str().ok_or("\"mark\" is a name")?.to_string()),
        "pointer" => Act::Pointer(match val {
            Json::Bool(b) => *b,
            Json::String(s) => s != "hide" && s != "off",
            _ => return Err("\"pointer\" is \"show\" or \"hide\"".into()),
        }),
        _ => unreachable!(),
    };
    Ok(Action {
        act,
        cue: o.get("cue").and_then(Json::as_str).map(str::to_string),
        after_line: o.get("afterLine").and_then(Json::as_u64).map(|n| n as usize),
        pause_ms: secs(o.get("pause")).unwrap_or(250),
        glide_ms: secs(o.get("glide")),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = r#"
    // a comment line
    { "id": "t", "title": {"en": "First app", "es": "Primera app"}, "scale": 1.5,
      "steps": [
        { "card": { "title": "Hi", "lines": ["RapidR Studio"] }, "say": ["{2K|two K} pixels."] },
        { "say": [ {"en": "Drag a button.", "es": "Arrastra un botón."}, { "text": "Done.", "cue": "drop", "ms": 900 } ],
          "do": [ { "drag": "Toolbox > RButton", "to": "designdoc(0)", "cue": "drop" },
                  { "type": {"en": "Hello", "es": "Hola"}, "into": "app:Edit1", "afterLine": 1 },
                  { "waitFor": "window:NewDialog", "timeout": 5 },
                  { "waitFor": { "prop": "studio.caption", "has": "MyApp" } },
                  { "spotlight": { "target": "Inspector > Caption", "x": 0.75 }, "for": 2 },
                  { "highlight": null } ] },
        { "click": "caption:Run", "gap": 1 },
        { "do": "file.saveAll" } ] }"#;

    #[test]
    fn a_script_parses_in_each_language() {
        let en = parse(SAMPLE, "en").unwrap();
        assert_eq!((en.title.as_str(), en.screen, en.scale), ("First app", (1280, 720), 1.5));
        assert_eq!(en.steps.len(), 4);
        assert_eq!(en.steps[0].card.as_ref().unwrap().lines, vec!["RapidR Studio"]);
        // ({shown|spoken}: the caption and the voice apart)
        assert_eq!((en.steps[0].say[0].text.as_str(), en.steps[0].say[0].spoken.as_str()), ("2K pixels.", "two K pixels."));
        let s = &en.steps[1];
        assert_eq!(s.say[0].text, "Drag a button.");
        assert_eq!((s.say[1].cue.as_deref(), s.say[1].ms), (Some("drop"), 900));
        assert_eq!(s.actions[0].act, Act::Drag { from: target_of("Toolbox > RButton"), to: target_of("designdoc(0)") });
        assert_eq!(s.actions[0].cue.as_deref(), Some("drop"));
        match &s.actions[1].act {
            Act::Type { into: Some(t), text } => assert_eq!((t.app, t.what.as_str(), text.as_str()), (true, "Edit1", "Hello")),
            other => panic!("{other:?}"),
        }
        assert_eq!(s.actions[1].after_line, Some(1));
        assert_eq!(s.actions[2].act, Act::WaitFor { cond: Cond::Window("NewDialog".into()), timeout_ms: 5000 });
        assert_eq!(s.actions[3].act, Act::WaitFor { cond: Cond::Prop { path: "studio.caption".into(), value: "MyApp".into(), exact: false }, timeout_ms: 30_000 });
        match &s.actions[4].act {
            Act::Highlight { target: Some(t), spotlight: true, for_ms: Some(2000) } => assert_eq!((t.what.as_str(), t.item.as_deref(), t.fx), ("Inspector", Some("Caption"), 0.75)),
            other => panic!("{other:?}"),
        }
        assert_eq!(s.actions[5].act, Act::Highlight { target: None, spotlight: false, for_ms: None });
        // (the shorthands: one action in the step; "do" as a text)
        assert_eq!(en.steps[2].actions[0].act, Act::Click { target: target_of("caption:Run"), double: false, right: false });
        assert_eq!(en.steps[2].gap_ms, 1000);
        assert_eq!(en.steps[3].actions[0].act, Act::Do("file.saveAll".into()));
        let es = parse(SAMPLE, "es").unwrap();
        assert_eq!(es.title, "Primera app");
        assert_eq!(es.steps[1].say[0].text, "Arrastra un botón.");
        assert!(matches!(&es.steps[1].actions[1].act, Act::Type { text, .. } if text == "Hola"));
        // (a language the script lacks: English)
        assert_eq!(parse(SAMPLE, "pt-BR").unwrap().title, "First app");
    }

    #[test]
    fn mistakes_say_where_they_are() {
        assert!(parse("{", "en").unwrap_err().contains("isn't valid JSON"));
        assert!(parse(r#"{"steps": [ {"do": [ {"jump": 1} ]} ]}"#, "en").unwrap_err().starts_with("step 1: action 1: no action in it"));
        assert!(parse(r#"{"steps": [ {"do": [ {"drag": "a"} ]} ]}"#, "en").unwrap_err().contains("a drag needs \"to\""));
        assert!(parse(r#"{"screen": [10, 10], "steps": []}"#, "en").unwrap_err().contains("out of range"));
        assert!(parse(r#"{"steps": [ {"waitFor": "soon:1"} ]}"#, "en").unwrap_err().contains("isn't a condition"));
    }

    #[test]
    fn markup_and_estimates() {
        assert_eq!(markup("Press {F5|F five} now {not markup}"), ("Press F5 now {not markup}".into(), "Press F five now {not markup}".into()));
        assert_eq!(estimate_ms("Hi"), 1200);
        let long = estimate_ms("Drag a button from the Toolbox onto the form, then let it go.");
        assert!((3000..6000).contains(&long), "{long}");
        assert!(is_lang("en") && is_lang("pt-BR") && !is_lang("text") && !is_lang("EN") && !is_lang("do") && !is_lang("to"));
    }
}
