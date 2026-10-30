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

// (the hints themselves — their ids and words — are the language service's:
// `rapidr_langsvc::foreign`, the one table `rapidr check`, Studio's Problems
// pane and these tools share)
pub use rapidr_langsvc::foreign::{closest, code_of, component_for, distance, for_message};

/// The other-language habits on one source line, each with its fix (id,
/// hint). Meant for a line the compiler reported an error on.
pub fn for_line(line: &str) -> Vec<(&'static str, String)> {
    rapidr_langsvc::foreign::habits(line).into_iter().map(|h| (h.id, h.text().to_string())).collect()
}

fn words_upper(code: &str) -> Vec<String> {
    code.split(|c: char| !(c.is_alphanumeric() || c == '_' || c == '$')).filter(|w| !w.is_empty()).map(str::to_ascii_uppercase).collect()
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
        assert!(ids("CONST TOTAL = 0").is_empty());
        // (RC.EXE has no LET: `let x = 1` is an unknown SUB there too)
        assert_eq!(ids("LET total = 0"), ["let"]);
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
        let (id, t) = for_message("Type mismatch, expecting type DOUBLE, but got STRING").unwrap();
        assert!(id == "undeclared" && t.contains("DIM it AS STRING"), "{t}");
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
