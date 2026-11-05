//! What Studio tells an AI app that connects to its MCP server (and what
//! the first `studio_status` adds): the recipe-first workflow and a short
//! language card, so even a small model — which the eval of 2026-10-29
//! (dist/ai-eval-2026-10-29/report.md) showed never looks at an example,
//! writes other languages' syntax and repeats its error — starts from a
//! working program and fixes what the compiler says.
//!
//! One text, kept under [`BUDGET_TOKENS`] (a test measures it): the server's
//! `instructions` (ide/mcp.inc asks for them with `RAIProjectTools.
//! ServerInstructions`), nothing else per connection. The language card is
//! the 15 rules small models broke most, in the words of the compiler's
//! hints (`rapidr_langsvc::foreign`).

/// The instructions' budget, in tokens (`rapidr_lang::card::estimate_tokens`).
pub const BUDGET_TOKENS: usize = 1_200;

/// The core rules' own budget (tokens): short enough for a small model to
/// keep in mind (docs/ai-tools.md "Small models").
pub const CORE_BUDGET_TOKENS: usize = 420;

/// The core rules: what every AI working in RapidR Studio is told first —
/// RAI and AI apps alike —, numbered, one line each, plain words, the exact
/// tools. The rest comes when asked (how_do_i, the tools' own words).
pub const CORE_RULES: &str = "\
RapidR Studio, the IDE of RapidR (a modern BASIC). The user watches every call. The open project IS the user's program: work in its files.
1. studio_status first: what is open, the errors, what you may do.
2. Learn with rapidr_describe (the truth about a component or builtin), find_example (working code) and search_manual. Never read RapidR's own source code, never invent a member.
3. Something new: find_example first, then adapt its code.
4. To change code: save_version, read_file, propose_edit (only the lines that change), check_code, run_program, app_screenshot or app_layout, save_version.
5. A new file: write_file. create_project only makes a separate new project.
6. check_code after every change. Problems must be 0 before you run, measure or report.
7. Each error has a hint line with the RapidR form: apply it. The same error twice: stop rewriting, find_example.
8. See the running program with app_layout or app_screenshot; use it with app_click, app_type, app_key; read_output for what it printed.
9. A crash or a wrong value: run_program with debug true, then debug (breakpoint, state, step_over).
10. Never use git or a shell for the user's project: save_version, list_versions, compare_versions, restore_version.
11. Not sure which tool comes next: how_do_i. A tool or an answer is missing: report_gap, then go on.
12. Report only what you saw working: the output, a picture, numbers.
";

/// The ready-made instructions Studio's MCP server offers an AI app's
/// person (MCP prompts): (name, title, what it is for, the text).
pub const PROMPTS: &[(&str, &str, &str, &str)] = &[
    (
        "rapidr-way",
        "Work on this project the RapidR way",
        "The steps and tools for changing the open project safely: versions, small changes, checks, runs, seeing it work.",
        "Work on the project open in RapidR Studio, step by step, the RapidR way:
1. studio_status — what is open, and what you may do.
2. list_versions — start_versions if it keeps none; then save_version {\"message\": \"Before: <the task>\"}.
3. Learn what you need: rapidr_describe for a component or builtin, find_example for working code, search_manual for how-to. Never read RapidR's own source, never invent a member.
4. read_file, then propose_edit — only the lines that change. A new file: write_file.
5. check_code until Problems = 0 (apply each error's hint line).
6. run_program, then app_layout or app_screenshot to see it; app_click and app_type to use it; read_output.
7. When a step works: save_version {\"message\": \"<what works now>\"}. Something broke: compare_versions, or restore_version.
8. Never use git or a shell for the project. A tool or an answer is missing: report_gap.
9. At the end, tell the user what you changed and what you saw working.",
    ),
    (
        "new-app",
        "Start a new RapidR app",
        "From a sentence to a working program: an example first, then the code, checked, run and seen.",
        "Make the new program the user describes, in RapidR, in RapidR Studio:
1. studio_status — an empty project open: build it there (write_file main.rr); nothing open: create_project.
2. find_example {\"query\": \"<what the program does, in a few words>\"} — start from the working code it gives.
3. Adapt it: names, captions, logic. A window is CREATE Form AS RForm … END CREATE, ending with Form.ShowModal.
4. check_code until Problems = 0.
5. run_program, then app_screenshot or app_layout to see it; click through it with app_click.
6. save_version {\"message\": \"First version\"} (start_versions first if needed).
7. Tell the user what it does and how to use it.",
    ),
];

/// The resources Studio's MCP server offers (uri, title, what it is): the
/// rules, the workflows, the manual's guide to forms.
pub const RESOURCES: &[(&str, &str, &str)] = &[
    ("rapidr://guide/rules", "The core rules", "How to work in RapidR Studio: the core rules and the language card"),
    ("rapidr://guide/workflows", "Workflows", "Each task's steps with the exact tools (what how_do_i answers)"),
    ("rapidr://guide/forms", "Designing forms", "RapidR's manual on designing forms: the designer, layout, Align"),
];

/// A resource's text (`None`: not one of them).
pub fn resource(uri: &str) -> Option<String> {
    match uri {
        "rapidr://guide/rules" => Some(server_instructions()),
        "rapidr://guide/workflows" => Some(crate::tools_core::all_workflows()),
        "rapidr://guide/forms" => crate::manual_search::PAGES.iter().find(|(p, _)| *p == "designing-forms.md").map(|(_, t)| t.to_string()),
        _ => None,
    }
}

/// The language card: the rules small models break most (a line each).
const CARD: &str = "\
RapidR in short (names are not case-sensitive):
- x = 1 assigns. No SET, LET, var or :=.
- Compare with = and <>, not == or !=. AND, OR, NOT, not && || !. Join text with +.
- Remainder is MOD, not %. Power is ^. A comment starts with '. Text is \"double quoted\" only.
- A line is a statement: no ; and no { } .
- DIM before use: DIM total AS DOUBLE, DIM name AS STRING, DIM n AS INTEGER. An undeclared name is a DOUBLE, so s = \"a\" without DIM fails.
- No BOOLEAN: use an INTEGER (0 false, 1 true). No null, new or classes.
- IF a = 1 THEN, ELSEIF a = 2 THEN, ELSE, END IF (ELSEIF one word, END IF two, no : after THEN). One line: IF a THEN PRINT \"x\" ELSE PRINT \"y\".
- FOR i = 1 TO 10 STEP 2 ... NEXT i (no in, no range). WHILE c ... WEND. DO ... LOOP UNTIL c.
- EXIT FOR or EXIT DO leaves a loop (no break). A list's items: FOR i = 0 TO List.ItemCount - 1.
- SELECT CASE x, CASE 1, CASE 2, 3, CASE ELSE, END SELECT (no switch).
- SUB Name(a AS INTEGER) ... END SUB. Call it without brackets: Name 5.
- FUNCTION Name(a AS INTEGER) AS INTEGER ... Result = a * 2 ... END FUNCTION. Call it in an expression: Name(5). No def, no return.
- DIM a(10) AS STRING is an array (items 0 to 10): a(1) = \"x\". No [ ] and no .Add.
- LEN(s), MID$(s, start, n), LEFT$, RIGHT$, UCASE$, LCASE$, TRIM$, INSTR(s, find).
- STR$(n) is a number as text, VAL(s) text as a number. No str(), int(), parseInt().
- PRINT \"Total: \"; total writes a line. No print(), console.log.
- Another file: $INCLUDE \"file.rr\" at the top. No import or #include.
- A window: CREATE Form AS RForm, Caption = \"Title\", then each control CREATE Go AS RButton with Caption = \"Go\" and OnClick = GoClick, END CREATE, and a last END CREATE.
- A handler is DECLARE SUB GoClick at the top and SUB GoClick ... END SUB below. The program ends with Form.ShowModal.
- RButton, RLabel, REdit (.Text), RListBox (.AddItems, .Clear, .ItemCount), RCheckBox, RTimer (.Interval, OnTimer), RCanvas, RPanel.
- A check box's .Checked is 1 or 0, not True. ShowMessage \"text\" opens a message box.
- Not sure of a component, property or builtin? rapidr_describe it. Never invent one.
";

/// The server's instructions: the core rules, then the card.
pub fn server_instructions() -> String {
    format!("{CORE_RULES}\n{CARD}")
}

/// What `studio_status` adds at its end: the next step for this project.
/// `files` are the project's source files (relative path, text); `open`:
/// whether a project is open at all.
pub fn status_next(open: bool, files: &[(String, String)]) -> String {
    if !open {
        return "No project is open: call find_example for what the user wants, then create_project with that recipe (or open_project for a program that exists).".to_string();
    }
    if is_empty(files) {
        return "This project is empty: call find_example first (what the user wants, in a few words), then write the recipe into main.rr with write_file and check_code it.".to_string();
    }
    "Next: read_file what you change, change it with propose_edit, then check_code. For something new, find_example first.".to_string()
}

/// Whether the project's sources hold no code: nothing but comments and blank lines.
pub fn is_empty(files: &[(String, String)]) -> bool {
    files.iter().all(|(_, text)| text.lines().all(|l| l.trim().is_empty() || l.trim_start().starts_with('\'')))
}

#[cfg(test)]
mod tests {
    use super::*;
    use rapidr_langsvc::{Analysis, Options, Severity};
    use std::path::Path;

    #[test]
    fn the_instructions_hold_the_workflow_and_stay_inside_the_budget() {
        let text = server_instructions();
        for must in ["find_example", "check_code", "hint", "The open project IS the user's program", "create_project only makes a separate new project", "stop rewriting", "propose_edit", "studio_status", "Never use git or a shell", "save_version", "Problems must be 0", "how_do_i", "report_gap"] {
            assert!(text.contains(must), "missing {must:?}");
        }
        let tokens = rapidr_lang::card::estimate_tokens(&text);
        println!("MCP instructions: {} characters, about {tokens} tokens, {} lines of language card", text.chars().count(), CARD.lines().count());
        assert!(tokens <= BUDGET_TOKENS, "{tokens} tokens: the instructions are over {BUDGET_TOKENS}");
        // (not much under it either: this is the number the report gives)
        assert!(tokens > 500, "{tokens}");
        assert!(!text.contains("RapidQ"), "RapidR words only");
    }

    #[test]
    fn what_the_card_says_compiles() {
        // (every construct the card shows, in one program)
        let program = "\
$APPTYPE GUI
DECLARE SUB GoClick
DIM total AS DOUBLE, name AS STRING, n AS INTEGER
DIM a(10) AS STRING
total = 1.5
name = \"Ana\"
n = 7 MOD 3
total = 2 ^ 3
a(1) = \"x\"
IF n = 1 THEN
  PRINT \"one\"
ELSEIF n <> 2 AND total > 0 OR NOT n THEN
  PRINT \"two\"
ELSE
  PRINT \"other\"
END IF
IF n THEN PRINT \"x\" ELSE PRINT \"y\"
FOR n = 1 TO 10 STEP 2
  IF n > 8 THEN EXIT FOR
NEXT n
WHILE n < 20
  n = n + 1
WEND
DO
  n = n - 1
LOOP UNTIL n < 5
SELECT CASE n
  CASE 1
    PRINT 1
  CASE 2, 3
    PRINT 2
  CASE ELSE
    PRINT 3
END SELECT
SUB Show(a AS INTEGER)
  PRINT a
END SUB
FUNCTION Twice(a AS INTEGER) AS INTEGER
  Result = a * 2
END FUNCTION
Show 5
PRINT \"Total: \"; Twice(5); LEN(name); MID$(name, 1, 2); LEFT$(name, 1); RIGHT$(name, 1); UCASE$(name); LCASE$(name); TRIM$(name); INSTR(name, \"n\"); STR$(n); VAL(\"12\")
CREATE Form AS RForm
  Caption = \"Title\"
  CREATE Go AS RButton
    Caption = \"Go\"
    OnClick = GoClick
  END CREATE
  CREATE Items AS RListBox
    Top = 40
  END CREATE
  CREATE Check AS RCheckBox
    Top = 80
  END CREATE
  CREATE Tick AS RTimer
    Interval = 1000
  END CREATE
END CREATE
SUB GoClick
  Items.AddItems \"a\"
  FOR n = 0 TO Items.ItemCount - 1
    PRINT n
  NEXT n
  IF Check.Checked THEN Items.Clear
  ShowMessage \"text\"
END SUB
Form.ShowModal
";
        let mut a = Analysis::new(Options::default());
        let file = Path::new("/project/main.rr");
        a.update(file, program);
        let errors: Vec<String> = a.diagnostics(file).into_iter().filter(|d| d.severity == Severity::Error).map(|d| d.message).collect();
        assert!(errors.is_empty(), "{errors:?}");
    }

    /// The core rules: 10 to 15 numbered lines, each one action with its
    /// exact tool, under their budget — and every tool they name is one.
    #[test]
    fn the_core_rules_are_short_numbered_and_name_real_tools() {
        let numbered: Vec<&str> = CORE_RULES.lines().filter(|l| l.split('.').next().is_some_and(|n| n.parse::<u32>().is_ok())).collect();
        assert!((10..=15).contains(&numbered.len()), "{} rules", numbered.len());
        for l in &numbered {
            assert!(l.len() <= 200, "one line, plain: {l}");
        }
        let tokens = rapidr_lang::card::estimate_tokens(CORE_RULES);
        println!("core rules: {} characters, about {tokens} tokens", CORE_RULES.len());
        assert!(tokens <= CORE_BUDGET_TOKENS, "{tokens} tokens: over {CORE_BUDGET_TOKENS}");
        // (every snake_case word a tool's name)
        let all = format!("{CORE_RULES}{}", PROMPTS.iter().map(|p| p.3).collect::<String>());
        let not_tools = ["step_over", "step_into", "step_out", "old_text", "new_text"];
        for w in all.split(|c: char| !(c.is_ascii_alphanumeric() || c == '_')).filter(|w| w.contains('_') && w.chars().all(|c| c.is_ascii_lowercase() || c == '_') && !not_tools.contains(w)) {
            assert!(crate::toolset::find(w).is_some(), "{w} isn't a tool");
        }
        for (uri, _, _) in RESOURCES {
            assert!(resource(uri).is_some_and(|t| t.len() > 500), "{uri}");
        }
    }

    #[test]
    fn the_status_says_what_to_do_next() {
        assert!(status_next(false, &[]).contains("find_example"));
        let empty = vec![("main.rr".to_string(), "' MyApp: an empty program.\n".to_string())];
        assert!(status_next(true, &empty).starts_with("This project is empty: call find_example first"));
        let started = vec![("main.rr".to_string(), "PRINT 1\n".to_string())];
        assert!(status_next(true, &started).starts_with("Next: "));
        assert!(is_empty(&[]) && is_empty(&empty) && !is_empty(&started));
    }
}
