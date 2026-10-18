//! Friendly hints: what someone coming from another language — or a small
//! AI model — writes in RapidR BASIC, and the exact fix (docs/small-models-plan.md
//! §4, docs/learn-with-rai-plan.md "friendly errors"). Pure functions over
//! text: the assistant's check_code adds them under the compiler's own
//! messages (`ai_tools`), and Learn with RAI can show the same table to a
//! person. The compiler and its messages don't change.
//!
//! * [`for_line`]: the habits on a line the compiler stopped at (`==`,
//!   `!=`, `&&`, `AS T AS T`, `+=`, braces, `[…]` lists, `return x` …);
//! * [`for_message`]: a compiler message read for what it means (`Unknown
//!   data type BOOLEAN` → INTEGER; `Unknown type MOUSEEVENT` → an event's
//!   own parameters);
//! * [`program`]: what only the whole file shows — the main form never
//!   shown (no `ShowModal`), a handler that does nothing yet or only says
//!   what "a real implementation would" do;
//! * [`closest`]: the known name nearest an unknown one (`ButtonPressed` →
//!   `ButtonPress`).

/// A finding of [`program`]: its line (1-based), whether it is an error
/// (the program doesn't work as written) or a warning, and the hint.
#[derive(Clone, Debug, PartialEq)]
pub struct Finding {
    pub line: usize,
    pub error: bool,
    pub id: &'static str,
    pub text: String,
}

/// A line's comment: what follows its first `'` outside a string ("" when
/// it has none).
pub fn comment_of(line: &str) -> &str {
    let mut quoted = false;
    for (i, c) in line.char_indices() {
        match c {
            '"' => quoted = !quoted,
            '\'' if !quoted => return &line[i + 1..],
            _ => {}
        }
    }
    ""
}

/// The habits that never compile, wherever they are (outside strings and
/// comments): [`for_line`]'s ids a whole file is searched for, since the
/// compiler stops at its first syntax error and never reaches the others.
pub const NEVER_BASIC: &[&str] = &["ne", "or", "as-as", "braces", "brackets", "comment"];

/// A line's code: the comment gone, each string literal's text blanked
/// (its quotes kept), so a habit inside a string or a comment isn't one.
pub fn code_of(line: &str) -> String {
    let mut out = String::new();
    let mut quoted = false;
    for c in line.chars() {
        match c {
            '"' => {
                quoted = !quoted;
                out.push('"');
            }
            '\'' if !quoted => break,
            _ if quoted => out.push(' '),
            c => out.push(c),
        }
    }
    // (REM starts a comment too)
    let upper = out.to_ascii_uppercase();
    if let Some(i) = upper.find("REM ").filter(|&i| i == 0 || upper[..i].ends_with(':') || upper[..i].trim().is_empty()) {
        out.truncate(i);
    }
    out
}

fn words_upper(code: &str) -> Vec<String> {
    code.split(|c: char| !(c.is_alphanumeric() || c == '_' || c == '$')).filter(|w| !w.is_empty()).map(str::to_ascii_uppercase).collect()
}

/// The other-language habits on one source line, each with its fix (id,
/// hint). Meant for a line the compiler reported an error on.
pub fn for_line(line: &str) -> Vec<(&'static str, String)> {
    let code = code_of(line);
    let t = code.trim();
    let lower = t.to_ascii_lowercase();
    let w = words_upper(t);
    let mut out: Vec<(&'static str, String)> = Vec::new();
    let mut add = |id: &'static str, text: &str| {
        if !out.iter().any(|(i, _)| *i == id) {
            out.push((id, text.to_string()));
        }
    };
    // (`==`, `&&`, `+=` and `++` are RapidQ's too: they compile and work,
    // so they're no habit to correct)
    if t.contains("!=") {
        add("ne", "`!=` isn't BASIC: \"not equal\" is `<>` — IF a <> b THEN");
    }
    if t.contains("||") {
        add("or", "`||` isn't BASIC: use OR — IF a > 0 OR b > 0 THEN");
    }
    if let Some(k) = w.iter().position(|x| x == "AS") {
        if w[k + 1..].iter().skip(1).any(|x| x == "AS") && w.first().is_some_and(|f| f == "DIM" || f == "CONST" || f == "STATIC") && !t.contains(',') {
            add("as-as", "a name takes one `AS type`: DIM ready AS INTEGER (not AS INTEGER AS INTEGER)");
        }
    }
    if t.contains('{') || t.contains('}') {
        add("braces", "BASIC has no braces: a block ends with its own words — END SUB, END FUNCTION, END IF, NEXT, WEND, LOOP");
    }
    if t.ends_with(';') && !w.first().is_some_and(|f| f == "PRINT" || f == "LPRINT" || f == "WRITE") {
        add("semicolon", "statements end at the end of the line: no `;`");
    }
    if t.contains('[') || t.contains(']') {
        add("brackets", "BASIC has no `[ ]`: an array is DIM names(3) AS STRING, filled one item at a time — names(1) = \"Ana\"; its items are names(i)");
    }
    if lower.starts_with("function ") && (t.contains('{') || t.contains("=>")) || lower.starts_with("def ") || lower.starts_with("void ") || lower.starts_with("public ") || lower.starts_with("private sub") {
        add("function", "a routine is SUB Name(a AS INTEGER) … END SUB, or FUNCTION Name(a AS INTEGER) AS INTEGER … Result = … END FUNCTION");
    }
    if w.first().is_some_and(|f| f == "RETURN") && w.len() > 1 {
        add("return", "a FUNCTION gives its value with `Result = …` (or `FunctionName = …`); RETURN only goes back from a GOSUB");
    }
    if lower.starts_with("print(") || lower.starts_with("console.") || lower.starts_with("printf") || lower.starts_with("echo ") {
        add("print", "to write a line: PRINT \"Total: \"; total (no brackets around the list); in a window, set a label's Caption instead");
    }
    if lower.starts_with("var ") || (t.starts_with("const ") && t.contains('=')) {
        add("var", "a variable is declared with DIM name AS type (DIM total AS DOUBLE); a constant with CONST NAME = value");
    }
    if lower.contains("else if") || lower.starts_with("elif") || w.iter().any(|x| x == "ELSIF") {
        add("elseif", "it's ELSEIF, one word: IF … THEN … ELSEIF … THEN … ELSE … END IF");
    }
    if w.iter().any(|x| x == "ENDIF") {
        add("endif", "it's END IF, two words");
    }
    if lower.contains(".length") || lower.contains(".size()") || lower.contains(".count()") {
        add("length", "a string's length is LEN(s); a list's items are .ItemCount");
    }
    if w.iter().any(|x| x == "NULL" || x == "NONE" || x == "NIL" || x == "UNDEFINED") {
        add("null", "BASIC has no null: use \"\" for an empty string, 0 for a number");
    }
    if w.first().is_some_and(|f| f == "NEW") || lower.contains("= new ") {
        add("new", "components are made with CREATE Name AS RButton … END CREATE (or DIM Name AS RButton), not `new`");
    }
    if lower.contains("//") && !lower.contains("://") {
        add("comment", "a comment starts with ' (an apostrophe), not //");
    }
    // (an event bound to nothing: OnChange = (), OnClick = "", OnClick =)
    let compact: String = t.chars().filter(|c| !c.is_whitespace()).collect::<String>().to_ascii_uppercase();
    if compact.starts_with("ON") && (compact.ends_with("=()") || compact.ends_with("=\"\"") || compact.ends_with('=') || compact.contains("=SUB") || compact.contains("=FUNCTION")) {
        add("event", "an event takes a SUB's name: OnChange = ValueChanged — with DECLARE SUB ValueChanged at the top and SUB ValueChanged … END SUB below; leave the line out if nothing should happen");
    }
    if w.iter().any(|x| x == "TRUE" || x == "FALSE") {
        add("bool", "truth values are numbers: 1 is true, 0 is false — DIM done AS INTEGER: done = 1 (True is 1 only with $INCLUDE \"RAPIDQ.INC\"; there is no False)");
    }
    out
}

/// What a compiler message means, said as a fix (id, hint): data types
/// from other languages, the type of an event's argument.
pub fn for_message(message: &str) -> Option<(&'static str, String)> {
    let after = |prefix: &str| message.strip_prefix(prefix).map(|r| r.trim().trim_matches(['\'', '`', '"']).to_string());
    if let Some(ty) = after("Unknown data type").or_else(|| after("Unknown type")) {
        let up = ty.to_ascii_uppercase();
        let fix = match up.as_str() {
            "BOOLEAN" | "BOOL" => "RapidR has no BOOLEAN: use INTEGER (0 is false, 1 or -1 true) — DIM running AS INTEGER".to_string(),
            "COLOR" | "COLOUR" | "TCOLOR" => "a colour is an INTEGER: DIM pen AS INTEGER: pen = RGB(255, 0, 0)".to_string(),
            "INT" | "INT32" | "INT64" | "NUMBER" => "use INTEGER (or LONG) for whole numbers, DOUBLE for decimals".to_string(),
            "FLOAT" | "REAL" | "DECIMAL" | "FLOAT64" => "use DOUBLE (or SINGLE) for numbers with decimals".to_string(),
            "STR" | "CHAR" | "TEXT" => "use STRING for text".to_string(),
            "VOID" => "a routine that returns nothing is a SUB: SUB Name … END SUB".to_string(),
            "DATE" | "DATETIME" | "TIME" => "dates are strings in RapidR: DATE$ and TIME$ give today's".to_string(),
            "ARRAY" | "LIST" => "an array is DIM items(10) AS STRING (round brackets, a size)".to_string(),
            _ if up.contains("EVENT") || up.contains("ARGS") || up.ends_with("SENDER") => format!(
                "{ty} isn't a RapidR type: an event's handler takes the event's own parameters — rapidr_describe the component; e.g. OnClick: SUB Name (no parameters, or Sender AS RButton), OnMouseDown: SUB Name(Button AS INTEGER, X AS INTEGER, Y AS INTEGER, Shift AS INTEGER)"
            ),
            _ => match component_for(&ty) {
                Some(r) => format!("{ty} isn't a RapidR type: the component is {r} (rapidr_describe {r})"),
                None => return None,
            },
        };
        return Some(("type", fix));
    }
    None
}

/// RapidR's component for another toolkit's name (TextBox → REdit), or for
/// a RapidR name that isn't spelled quite right.
pub fn component_for(name: &str) -> Option<String> {
    let up = name.to_ascii_uppercase();
    let map = [
        ("TEXTBOX", "REdit"), ("TEXTFIELD", "REdit"), ("ENTRY", "REdit"), ("INPUT", "REdit"), ("LINEEDIT", "REdit"),
        ("TEXTAREA", "RRichEdit"), ("MULTILINETEXT", "RRichEdit"),
        ("BUTTON", "RButton"), ("COMMANDBUTTON", "RButton"),
        ("LABEL", "RLabel"), ("TEXTBLOCK", "RLabel"), ("STATICTEXT", "RLabel"),
        ("LISTBOX", "RListBox"), ("LIST", "RListBox"), ("LISTVIEW", "RListView"),
        ("COMBOBOX", "RComboBox"), ("DROPDOWN", "RComboBox"), ("SELECT", "RComboBox"),
        ("CHECKBOX", "RCheckBox"), ("RADIOBUTTON", "RRadioButton"), ("OPTIONBUTTON", "RRadioButton"),
        ("PICTUREBOX", "RCanvas"), ("CANVAS", "RCanvas"), ("PAINTBOX", "RCanvas"), ("DRAWINGAREA", "RCanvas"),
        ("TIMER", "RTimer"), ("PANEL", "RPanel"), ("FRAME", "RPanel"), ("GROUPBOX", "RGroupBox"),
        ("WINDOW", "RForm"), ("FORM", "RForm"), ("DIALOG", "RForm"),
        ("DATAGRID", "RStringGrid"), ("GRID", "RStringGrid"), ("TABLE", "RStringGrid"), ("DATAGRIDVIEW", "RStringGrid"),
        ("IMAGE", "RImage"), ("PROGRESSBAR", "RGauge"), ("SLIDER", "RTrackBar"), ("TRACKBAR", "RTrackBar"),
        ("MENU", "RMainMenu"), ("MENUITEM", "RMenuItem"), ("STATUSBAR", "RStatusBar"),
    ];
    let bare = up.trim_start_matches(['Q', 'R', 'T']).to_string();
    for key in [up.as_str(), bare.as_str()] {
        if let Some((_, r)) = map.iter().find(|(k, _)| *k == key) {
            if rapidr_lang::resolve_component(r).is_some() {
                return Some((*r).to_string());
            }
        }
    }
    let types: Vec<String> = rapidr_lang::COMPONENTS.iter().filter(|c| c.kind == rapidr_lang::Kind::Component).map(|c| c.spelling()).collect();
    closest(name, types.iter().map(String::as_str))
}

/// The candidate nearest `name` (any case), when one is close: a few
/// letters different for its length.
pub fn closest<'a>(name: &str, candidates: impl IntoIterator<Item = &'a str>) -> Option<String> {
    let want = name.to_ascii_lowercase();
    let mut best: Option<(usize, &str)> = None;
    for c in candidates {
        let d = distance(&want, &c.to_ascii_lowercase());
        if d == 0 {
            continue;
        }
        if best.is_none_or(|(b, _)| d < b) {
            best = Some((d, c));
        }
    }
    let (d, c) = best?;
    let room = (want.chars().count().max(c.chars().count()) / 3).max(1);
    (d <= room).then(|| c.to_string())
}

/// Levenshtein distance (characters).
pub fn distance(a: &str, b: &str) -> usize {
    let b: Vec<char> = b.chars().collect();
    let mut row: Vec<usize> = (0..=b.len()).collect();
    for (i, ca) in a.chars().enumerate() {
        let mut prev = row[0];
        row[0] = i + 1;
        for j in 0..b.len() {
            let cur = row[j + 1];
            row[j + 1] = (prev + usize::from(ca != b[j])).min(row[j] + 1).min(cur + 1);
            prev = cur;
        }
    }
    row[b.len()]
}

/// A comment that says the code around it isn't the real thing.
fn placeholder(comment: &str) -> bool {
    let c = comment.to_ascii_lowercase();
    ["a real implementation", "in a real ", "for a real ", "placeholder", "not implemented", "not yet implemented", "implement this", "implement the logic", "your code here", "logic here", "code goes here", "simplified approach", "for simplicity", "we simulate", "simulate this", "would go here", "would be here"]
        .iter()
        .any(|p| c.contains(p))
}

/// What the whole file shows: the main form never shown, handlers that do
/// nothing yet or only say what real code would do, TODOs. `main`: the
/// program's main file (where Form.ShowModal belongs).
pub fn program(text: &str, main: bool) -> Vec<Finding> {
    let mut out = Vec::new();
    let lines: Vec<&str> = text.lines().collect();
    let codes: Vec<String> = lines.iter().map(|l| code_of(l)).collect();
    let upper: Vec<String> = codes.iter().map(|c| c.trim().to_ascii_uppercase()).collect();
    // (the handlers the file binds: OnClick = Name)
    let mut bound: Vec<String> = Vec::new();
    for u in &upper {
        for st in u.split(':') {
            let st = st.trim();
            if let Some((lhs, rhs)) = st.split_once('=') {
                let lhs = lhs.trim();
                let short = lhs.rsplit('.').next().unwrap_or(lhs);
                if short.starts_with("ON") && short.len() > 2 && !rhs.trim().is_empty() {
                    bound.push(rhs.trim().to_string());
                }
            }
        }
    }
    // (each SUB / FUNCTION: its statements and its comments)
    let mut k = 0;
    while k < lines.len() {
        let u = &upper[k];
        let head = u.strip_prefix("SUB ").or_else(|| u.strip_prefix("FUNCTION ")).or_else(|| u.strip_prefix("SUB\t"));
        let Some(head) = head else {
            k += 1;
            continue;
        };
        // (its name as the file spells it)
        let spelled = codes[k].trim();
        let name: String = spelled[spelled.len() - head.len()..].trim().chars().take_while(|c| c.is_alphanumeric() || *c == '_' || *c == '$').collect();
        let is_fn = u.starts_with("FUNCTION");
        let start = k;
        let mut statements = 0;
        let mut said: Option<(usize, String)> = None;
        let mut todo: Option<usize> = None;
        k += 1;
        while k < lines.len() && !(upper[k].starts_with("END SUB") || upper[k].starts_with("END FUNCTION")) {
            if !upper[k].is_empty() {
                statements += 1;
            }
            let comment = comment_of(lines[k]);
            if said.is_none() && placeholder(comment) {
                said = Some((k + 1, comment.trim().chars().take(70).collect()));
            }
            if todo.is_none() && (comment.contains("TODO") || comment.contains("FIXME")) {
                todo = Some(k + 1);
            }
            k += 1;
        }
        let what = if is_fn { "FUNCTION" } else { "SUB" };
        if statements == 0 && bound.iter().any(|b| b.eq_ignore_ascii_case(&name)) {
            out.push(Finding { line: start + 1, error: true, id: "empty-handler", text: format!("{what} {name} does nothing yet (it has no statements), but an event calls it: write what it should do") });
        } else if let Some((at, c)) = said {
            out.push(Finding { line: at, error: true, id: "placeholder", text: format!("{what} {name} isn't finished: its comment says \"{c}\". Write the real code now (the user runs this program): no placeholders") });
        } else if let Some(at) = todo {
            out.push(Finding { line: at, error: false, id: "todo", text: format!("{what} {name} has a TODO: finish it, or say so to the user") });
        }
        k += 1;
    }
    // (a GUI program's main file shows its form; without ShowModal it
    // starts and ends at once, nothing shown)
    if main {
        let form = codes.iter().zip(&upper).find_map(|(c, u)| {
            let w = words_upper(u);
            let is_form = w.first().is_some_and(|f| f == "CREATE" || f == "DIM") && w.get(2).is_some_and(|a| a == "AS") && w.get(3).is_some_and(|t| t == "RFORM" || t == "QFORM");
            is_form.then(|| c.split_whitespace().nth(1).unwrap_or("Form").trim_end_matches(',').to_string())
        });
        let shows = upper.iter().any(|u| u.contains(".SHOWMODAL") || u.starts_with("SHOWMODAL")) || (upper.iter().any(|u| u.contains(".SHOW")) && upper.iter().any(|u| u.contains("DOEVENTS")));
        if let Some(form) = form.filter(|_| !shows) {
            out.push(Finding { line: lines.len().max(1), error: true, id: "showmodal", text: format!("the program never shows its window: end the main file with {form}.ShowModal (it shows the form and runs until it is closed)") });
        }
    }
    out
}

/// One component's members in one short paragraph (names only): what to
/// put beside an error about a member it doesn't have.
pub fn members(c: &rapidr_lang::Component) -> String {
    let props: Vec<&str> = c.properties.iter().map(|p| p.name).collect();
    let methods: Vec<&str> = c.methods.iter().map(|m| m.name).collect();
    let events: Vec<String> = c.events.iter().map(|e| format!("{}({})", e.name, e.params.iter().map(|p| format!("{} AS {}", p.name, p.ty)).collect::<Vec<_>>().join(", "))).collect();
    format!("{} has — properties: {}; methods: {}; events: {}", c.spelling(), props.join(", "), if methods.is_empty() { "none".into() } else { methods.join(", ") }, if events.is_empty() { "none".into() } else { events.join(", ") })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ids(line: &str) -> Vec<&'static str> {
        for_line(line).into_iter().map(|(i, _)| i).collect()
    }

    #[test]
    fn habits_on_a_line() {
        // (RapidQ's own: `==`, `&&`, `+=`, `++` compile and work)
        assert!(ids("IF ButtonPressed == \"7\" THEN").is_empty());
        assert!(ids("IF a > 0 && b > 0 THEN").is_empty());
        assert!(ids("count += 1").is_empty() && ids("i++").is_empty());
        assert_eq!(ids("IF a != b THEN"), ["ne"]);
        assert_eq!(ids("IF a || b THEN"), ["or"]);
        assert_eq!(ids("DIM waitingForOperand AS BOOLEAN AS BOOLEAN"), ["as-as"]);
        assert_eq!(ids("SUB Go() {"), ["braces"]);
        assert_eq!(ids("x = 1;"), ["semicolon"]);
        assert!(ids("PRINT a;").is_empty(), "PRINT's ; is BASIC");
        assert_eq!(ids("Q1_Options = [\"Berlin\", \"Madrid\", \"Paris\"]"), ["brackets"]);
        assert!(ids("function add(a, b) {").contains(&"function"));
        assert_eq!(ids("return total"), ["return"]);
        assert_eq!(ids("print(\"hello\")"), ["print"]);
        assert_eq!(ids("var total = 0"), ["var"]);
        assert_eq!(ids("const total = 0"), ["var"]);
        assert!(ids("CONST TOTAL = 0").is_empty() && ids("LET total = 0").is_empty());
        assert_eq!(ids("} else if (x > 1) {"), ["braces", "elseif"]);
        assert_eq!(ids("ENDIF"), ["endif"]);
        assert_eq!(ids("n = name.length"), ["length"]);
        assert_eq!(ids("x = null"), ["null"]);
        assert_eq!(ids("b = new Button()"), ["new"]);
        assert_eq!(ids("x = 1 // one"), ["comment"]);
        assert_eq!(ids("OnChange = () ' Optional: handle change if needed"), ["event"]);
        assert_eq!(ids("OnClick = \"\""), ["event"]);
        assert_eq!(ids("running = true"), ["bool"]);
        // (inside strings and comments: nothing)
        assert!(ids("PRINT \"a == b && c\" ' x != y").is_empty());
        assert!(ids("Caption = \"[ok]\"").is_empty());
        assert!(ids("DIM a AS INTEGER, b AS STRING").is_empty());
        assert!(ids("URL = \"http://x\"").is_empty());
    }

    #[test]
    fn messages_read_for_what_they_mean() {
        let (_, t) = for_message("Unknown data type BOOLEAN").unwrap();
        assert!(t.contains("INTEGER"), "{t}");
        let (_, t) = for_message("Unknown data type COLOR").unwrap();
        assert!(t.contains("RGB("), "{t}");
        let (_, t) = for_message("Unknown type MOUSEEVENT").unwrap();
        assert!(t.contains("OnMouseDown: SUB Name(Button AS INTEGER, X AS INTEGER"), "{t}");
        let (_, t) = for_message("Unknown type TextBox").unwrap();
        assert!(t.contains("REdit"), "{t}");
        let (_, t) = for_message("Unknown type RButon").unwrap();
        assert!(t.contains("RButton"), "{t}");
        assert!(for_message("Syntax error near 'x'").is_none());
        assert!(for_message("Unknown type Zzyzx").is_none());
    }

    #[test]
    fn the_closest_name() {
        assert_eq!(closest("ButtonPressed", ["ButtonPress", "Display", "Total"]).as_deref(), Some("ButtonPress"));
        assert_eq!(closest("dispaly", ["Display", "Total"]).as_deref(), Some("Display"));
        assert_eq!(closest("x", ["Display"]), None);
        assert_eq!(closest("Display", ["display"]), None, "the same name isn't a suggestion");
    }

    #[test]
    fn what_the_whole_file_shows() {
        let src = "DECLARE SUB Go\nDECLARE SUB Calc\nCREATE Form AS RForm\n  CREATE B AS RButton\n    OnClick = Go\n  END CREATE\n  CREATE C AS RButton: OnClick = Calc: END CREATE\nEND CREATE\nSUB Go\n  ' nothing yet\nEND SUB\nSUB Calc\n  ' In a real implementation, this would parse the expression.\n  ShowMessage \"Calculating\"\nEND SUB\nSUB Helper\nEND SUB\n";
        let f = program(src, true);
        let ids: Vec<&str> = f.iter().map(|x| x.id).collect();
        assert_eq!(ids, ["empty-handler", "placeholder", "showmodal"], "{f:#?}");
        assert_eq!(f[0].line, 9);
        assert!(f[1].text.contains("In a real implementation") && f[1].line == 13, "{f:#?}");
        assert!(f[2].text.contains("Form.ShowModal"));
        // (shown: no finding; an include file isn't the main one)
        let ok = "CREATE Main AS RForm\nEND CREATE\nMain.ShowModal\n";
        assert!(program(ok, true).is_empty());
        assert!(program("CREATE F AS RForm\nEND CREATE\n", false).is_empty());
        // (a console program has no form to show)
        assert!(program("PRINT 1\n", true).is_empty());
        // (a TODO: a warning)
        let todo = program("SUB A\n  x = 1 ' TODO: more\nEND SUB\n", false);
        assert!(todo.len() == 1 && !todo[0].error && todo[0].id == "todo", "{todo:#?}");
    }

    #[test]
    fn a_components_members_in_short() {
        let m = members(rapidr_lang::resolve_component("RButton").unwrap());
        assert!(m.starts_with("RButton has — properties: ") && m.contains("Caption") && m.contains("events: ") && m.contains("OnClick"), "{m}");
    }
}
