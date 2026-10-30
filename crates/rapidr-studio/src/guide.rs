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

/// The workflow: what to do, in order.
const WORKFLOW: &str = "\
RapidR Studio, the IDE of RapidR (a modern BASIC), with the user watching every call. The open project IS the user's program: put your work in its files. create_project only makes a separate new project.

Work in this order:
1. studio_status says what is open.
2. find_example for what the user wants: it returns a working program. Start from it and change names, captions and logic; don't write from nothing.
3. write_file (a new file, or the recipe into an empty main.rr) or propose_edit (a few lines of a file: never resend a whole file to change a few lines). Paths are relative to the project's folder: main.rr.
4. check_code after every write. Each error has a hint line with the RapidR form: apply it.
5. The same error twice: stop rewriting; call find_example and start from its recipe again.
6. rapidr_describe before a component or builtin you are unsure of (never invent one); search_manual for \"how do I\".
7. run_program, read_output; app_layout, app_screenshot, app_click and app_type to see and use the running program (the user sees a pointer; the first look of a run may wait for them). ask_rai asks RapidR's own assistant (may cost the user money).
";

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

/// The server's instructions: the workflow, then the card.
pub fn server_instructions() -> String {
    format!("{WORKFLOW}\n{CARD}")
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
        for must in ["find_example", "check_code", "hint", "The open project IS the user's program", "create_project only makes a separate new project", "stop rewriting", "propose_edit", "studio_status"] {
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
