//! Hints for syntax from other languages (docs/manual/diagnostics.md,
//! "Coming from another language?"): what someone from Python, JavaScript,
//! C or Pascal — or a small AI model — writes in RapidR, and the RapidR form
//! of it, in plain words.
//!
//! A hint is an ADDITIONAL line under the compiler's own message: the
//! messages stay RC.EXE's (`.reference/rapidq-compiler-messages.txt`), a
//! hint only says the fix. It is found from the message and the source
//! line it is about ([`for_error`]), so one table serves every place that
//! shows a compiler error — `rapidr check` and the other commands
//! ([`annotate`]), Studio's Problems pane and the editor's squiggles
//! (`FileDiagnostic::hint`), `rapidr lsp`, and the AI tools' check_code.
//!
//! Every hint is one [`Hint`]: a stable `id` (its message key in a catalog
//! is `hint.<id>`) and its English `text`. The texts live in [`HINTS`] and
//! [`FOREIGN_FUNCTIONS`] and nowhere else, so a message catalog can take
//! them over whole: read a text with [`Hint::text`] (the one place a
//! translation would be looked up).

/// One hint: a stable id and its English text.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Hint {
    /// The stable id (`set`, `mod`, `for-foreign` …); a catalog key is `hint.<id>`.
    pub id: &'static str,
    /// The words, in English: what to write instead.
    pub text: &'static str,
}

impl Hint {
    /// The catalog key of this hint's message.
    pub fn key(&self) -> String {
        format!("hint.{}", self.id)
    }

    /// The hint's words, in English: what the build report and the AI's
    /// tools carry.
    pub fn text(&self) -> &'static str {
        self.text
    }

    /// The hint's words in language `tag`: the compiler catalog's
    /// translation (`hint.<id>` in `rapidr-diagnostics/lang/<tag>.po`,
    /// the same file the compiler's messages are in), else English.
    pub fn text_in(&self, tag: &str) -> String {
        rapidr_diagnostics::messages::translate_text(&self.key(), self.text, tag).unwrap_or_else(|| self.text.to_string())
    }
}

/// Every text this module adds to the compiler catalog, as (key, English):
/// each hint (`hint.<id>`, once) and the word that labels one
/// (`hint.label`). `rapidr i18n compiler` writes them into
/// `compiler.pot` after the messages.
pub fn catalog_entries() -> Vec<(String, String)> {
    let mut out: Vec<(String, String)> = vec![("hint.label".to_string(), "hint".to_string())];
    for x in HINTS.iter().chain(FOREIGN_FUNCTIONS.iter().map(|(_, x)| x)) {
        if !out.iter().any(|(k, _)| *k == x.key()) {
            out.push((x.key(), x.text.to_string()));
        }
    }
    out
}

const fn h(id: &'static str, text: &'static str) -> Hint {
    Hint { id, text }
}

/// Every hint about a line (a habit that never compiles, or a compiler
/// message that has a known fix), by id.
pub const HINTS: &[Hint] = &[
    // --- statements ---
    h("set", "RapidR has no SET: write x = 1 (the name, =, then the value)"),
    h("let", "RapidR has no LET: write x = 1 (the name, =, then the value)"),
    h("var", "a variable is declared with DIM name AS type (DIM total AS DOUBLE); a constant with CONST NAME = value"),
    h("as-as", "a name takes one `AS type`: DIM ready AS INTEGER (not AS INTEGER AS INTEGER)"),
    h("for-foreign", "a counting loop is FOR i = 1 TO 10 … NEXT (STEP 2 to skip); RapidR has no `for … in range()`, `for (…;…;…)` or FOR EACH — to visit a list's items count them: FOR i = 0 TO List.ItemCount - 1"),
    h("end-loop", "a FOR loop ends with NEXT, a WHILE with WEND, a DO with LOOP — not END FOR, END WHILE or END DO"),
    h("endif", "it's END IF, two words"),
    h("endword", "END SUB, END FUNCTION, END IF, END SELECT, END WITH, END TYPE and END CREATE are two words with a space"),
    h("elseif", "it's ELSEIF, one word: IF … THEN … ELSEIF … THEN … ELSE … END IF"),
    h("if-then", "IF needs THEN on the same line: IF a = 1 THEN … END IF"),
    h("missing-next", "every FOR needs its NEXT: FOR i = 1 TO 3 … NEXT i"),
    h("missing-wend", "every WHILE needs its WEND: WHILE a < 3 … WEND"),
    h("missing-loop", "every DO needs its LOOP: DO … LOOP UNTIL a > 3"),
    h("missing-end-if", "every block IF needs its END IF: IF a THEN, the lines, END IF (a one-line IF needs none)"),
    h("else-if", "ELSE IF starts a second IF that needs an END IF of its own: write ELSEIF, one word — IF … THEN … ELSEIF … THEN … ELSE … END IF"),
    h("missing-end-select", "every SELECT CASE needs its END SELECT"),
    h("and-and", "RapidR uses AND and OR: IF a > 0 AND b > 0 THEN (`&&` is `&`: it joins text and adds numbers, so such a test is never what you meant)"),
    h("else-line","a one-line IF takes its ELSE on the same line (IF a THEN x ELSE y); for several lines write IF a THEN, the lines, ELSE, the lines, END IF"),
    h("colon", "no `:` after IF, ELSE, FOR, WHILE or SUB: write IF a = 1 THEN, the statements on the lines below, then END IF"),
    h("braces", "BASIC has no braces: a block ends with its own words — END SUB, END FUNCTION, END IF, NEXT, WEND, LOOP"),
    h("semicolon", "statements end at the end of the line: no `;`"),
    h("function", "a routine is SUB Name(a AS INTEGER) … END SUB, or FUNCTION Name(a AS INTEGER) AS INTEGER … Result = … END FUNCTION"),
    h("return", "a FUNCTION gives its value with `Result = …` (or `FunctionName = …`); RETURN only goes back from a GOSUB"),
    h("switch", "RapidR has no switch: write SELECT CASE x, CASE 1, CASE ELSE, END SELECT"),
    h("break", "RapidR has no break: leave a loop with EXIT FOR, EXIT DO or EXIT WHILE"),
    h("continue", "RapidR has no continue: put the rest of the loop inside IF … THEN … END IF"),
    h("include", "to use another file write $INCLUDE \"file.rr\" at the top: RapidR has no #include or import"),
    h("new", "components are made with CREATE Name AS RButton … END CREATE (or DIM Name AS RButton), not `new`"),
    h("event", "an event takes a SUB's name: OnChange = ValueChanged — with DECLARE SUB ValueChanged at the top and SUB ValueChanged … END SUB below; leave the line out if nothing should happen"),
    h("console-log", "use PRINT to write a line: PRINT \"hi\" (console.log is JavaScript); in a window, set a label's Caption"),
    h("print", "to write a line: PRINT \"Total: \"; total (no brackets around the list); in a window, set a label's Caption instead"),
    // --- operators ---
    h("mod", "RapidR has no %: a remainder is MOD — 7 MOD 3 is 1 (a \\ b is whole division, a / b exact)"),
    h("compound", "the shortcuts are += -= *= /= &=; for any other write it out: x = x ^ 2, x = x \\ 2, x = x MOD 3"),
    h("pow", "RapidR has no **: a power is ^ — 2 ^ 3 is 8"),
    h("intdiv", "RapidR has no //: whole division is \\ — 7 \\ 2 is 3 (a / b gives decimals)"),
    h("ternary", "RapidR has no a ? b : c: write IF a THEN x = 1 ELSE x = 2, or x = IIF(a, 1, 2)"),
    h("ne", "`!=` isn't BASIC: \"not equal\" is `<>` — IF a <> b THEN"),
    h("bang", "`!` isn't BASIC: \"not equal\" is `<>` (IF a <> b THEN), and \"is false\" is IF x = 0 THEN"),
    h("or", "`||` isn't BASIC: use OR — IF a > 0 OR b > 0 THEN"),
    h("brackets", "BASIC has no `[ ]`: an array is DIM names(3) AS STRING, filled one item at a time — names(1) = \"Ana\"; its items are names(i)"),
    h("squote", "text goes in double quotes — s = \"hello\" (a single quote starts a comment)"),
    h("comment", "a comment starts with ' (an apostrophe), not //"),
    // --- values ---
    h("null", "BASIC has no null: use \"\" for an empty string, 0 for a number"),
    h("bool", "truth values are numbers: 1 is true, 0 is false — DIM done AS INTEGER: done = 1, then IF done THEN (a .rr program's True is -1 while a check box's Checked is 1: don't compare with True)"),
    h("length", "a string's length is LEN(s); a list's items are .ItemCount"),
];

/// Functions of other languages and their RapidR names, by the name the
/// program called (any case): `hint.fn-<name>`.
pub const FOREIGN_FUNCTIONS: &[(&str, Hint)] = &[
    ("str", h("fn-str", "RapidR has no str(): text from a number is STR$(n) — PRINT \"n = \" + STR$(n)")),
    ("tostring", h("fn-tostring", "RapidR has no toString(): text from a number is STR$(n)")),
    ("int", h("fn-int", "RapidR has no int(): a number from text is VAL(s); INT(x) cuts the decimals of a number")),
    ("float", h("fn-float", "RapidR has no float(): a number from text is VAL(s)")),
    ("parseint", h("fn-parseint", "RapidR has no parseInt(): a number from text is VAL(s)")),
    ("parsefloat", h("fn-parsefloat", "RapidR has no parseFloat(): a number from text is VAL(s)")),
    ("input", h("fn-input", "RapidR has no input(): in a console program INPUT name; in a window an REdit, read its Text")),
    ("prompt", h("fn-prompt", "RapidR has no prompt(): ask with INPUTBOX$(\"Question\") or read an REdit's Text")),
    ("alert", h("fn-alert", "RapidR has no alert(): a message box is ShowMessage \"text\"")),
    ("toupper", h("fn-toupper", "upper case is UCASE$(s)")),
    ("touppercase", h("fn-touppercase", "upper case is UCASE$(s)")),
    ("upper", h("fn-upper", "upper case is UCASE$(s)")),
    ("tolower", h("fn-tolower", "lower case is LCASE$(s)")),
    ("tolowercase", h("fn-tolowercase", "lower case is LCASE$(s)")),
    ("lower", h("fn-lower", "lower case is LCASE$(s)")),
    ("strlen", h("fn-strlen", "a string's length is LEN(s)")),
    ("length", h("fn-length", "a string's length is LEN(s); a list's items are .ItemCount")),
    ("substr", h("fn-substr", "part of a string is MID$(s, start, count), LEFT$(s, n) or RIGHT$(s, n); the first character is 1")),
    ("substring", h("fn-substring", "part of a string is MID$(s, start, count), LEFT$(s, n) or RIGHT$(s, n); the first character is 1")),
    ("random", h("fn-random", "a random number is RND (0 up to 1): INT(RND * 6) + 1 is a die")),
    ("rand", h("fn-rand", "a random number is RND (0 up to 1): INT(RND * 6) + 1 is a die")),
    ("sleep", h("fn-sleep", "to wait write SLEEP 500 (milliseconds); in a window use an RTimer")),
    ("trim", h("fn-trim", "trimming is TRIM$(s) (LTRIM$ and RTRIM$ for one side)")),
];

/// A hint by its id.
pub fn hint(id: &str) -> Option<&'static Hint> {
    HINTS.iter().find(|x| x.id == id).or_else(|| FOREIGN_FUNCTIONS.iter().map(|(_, x)| x).find(|x| x.id == id))
}

fn id(id: &str) -> &'static Hint {
    hint(id).unwrap_or_else(|| panic!("no hint {id:?}"))
}

// ---- reading a line -----------------------------------------------------------------------

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
            // (as many bytes as the character had: a position in the result is one in the line)
            _ if quoted => out.extend(std::iter::repeat_n(' ', c.len_utf8())),
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

/// The other-language habits on one source line, the most telling first.
/// Meant for a line the compiler reported an error on (`==`, `&&`, `+=` and
/// `++` are RapidQ's too: they compile and work, so they're no habit).
pub fn habits(line: &str) -> Vec<&'static Hint> {
    let code = code_of(line);
    let t = code.trim();
    let lower = t.to_ascii_lowercase();
    let w = words_upper(t);
    let mut out: Vec<&'static Hint> = Vec::new();
    let mut add = |name: &str| {
        let hint = id(name);
        if !out.iter().any(|x| x.id == hint.id) {
            out.push(hint);
        }
    };
    if t.contains("!=") {
        add("ne");
    }
    if t.contains("||") {
        add("or");
    }
    if let Some(k) = w.iter().position(|x| x == "AS") {
        if w[k + 1..].iter().skip(1).any(|x| x == "AS") && w.first().is_some_and(|f| f == "DIM" || f == "CONST" || f == "STATIC") && !t.contains(',') {
            add("as-as");
        }
    }
    // (a routine written the other languages' way: said before its braces)
    if lower.starts_with("function ") && (t.contains('{') || t.contains("=>")) || lower.starts_with("def ") || lower.starts_with("void ") || lower.starts_with("public ") || lower.starts_with("private sub") {
        add("function");
    }
    if t.contains('{') || t.contains('}') {
        add("braces");
    }
    if t.ends_with(';') && !w.first().is_some_and(|f| f == "PRINT" || f == "LPRINT" || f == "WRITE") {
        add("semicolon");
    }
    if t.contains('[') || t.contains(']') {
        add("brackets");
    }
    if w.first().is_some_and(|f| f == "RETURN") && w.len() > 1 {
        add("return");
    }
    if lower.starts_with("print(") || lower.starts_with("console.") || lower.starts_with("printf") || lower.starts_with("echo ") {
        add("print");
    }
    if lower.starts_with("var ") || (t.starts_with("const ") && t.contains('=')) {
        add("var");
    }
    if lower.starts_with("let ") && t.contains('=') {
        add("let");
    }
    if lower.contains("else if") || lower.starts_with("elif") || w.iter().any(|x| x == "ELSIF") {
        add("elseif");
    }
    if w.iter().any(|x| x == "ENDIF") {
        add("endif");
    }
    if lower.contains(".length") || lower.contains(".size()") || lower.contains(".count()") {
        add("length");
    }
    if w.iter().any(|x| x == "NULL" || x == "NONE" || x == "NIL" || x == "UNDEFINED") {
        add("null");
    }
    if w.first().is_some_and(|f| f == "NEW") || lower.contains("= new ") {
        add("new");
    }
    if lower.contains("//") && !lower.contains("://") {
        add(if int_division(t) { "intdiv" } else { "comment" });
    }
    // (an event bound to nothing: OnChange = (), OnClick = "", OnClick =)
    let compact: String = t.chars().filter(|c| !c.is_whitespace()).collect::<String>().to_ascii_uppercase();
    if compact.starts_with("ON") && (compact.ends_with("=()") || compact.ends_with("=\"\"") || compact.ends_with('=') || compact.contains("=SUB") || compact.contains("=FUNCTION")) {
        add("event");
    }
    if w.iter().any(|x| x == "TRUE" || x == "FALSE") {
        add("bool");
    }
    if lower.starts_with("switch ") || lower.starts_with("switch(") {
        add("switch");
    }
    if lower.starts_with("#include") || lower.starts_with("import ") || lower.starts_with("using ") || lower.starts_with("from ") && lower.contains(" import ") {
        add("include");
    }
    out
}

/// Whether the `//` on a code line is a whole division (`7 // 2`, `a // b`:
/// a number or name before it and a number or `(` after), not a comment.
fn int_division(code: &str) -> bool {
    let Some(i) = code.find("//") else { return false };
    let (before, after) = (code[..i].trim_end(), code[i + 2..].trim());
    let value_before = before.chars().last().is_some_and(|c| c.is_alphanumeric() || c == ')' || c == '_');
    let word_after = after.split_whitespace().count() == 1;
    let number_after = after.chars().next().is_some_and(|c| c.is_ascii_digit() || c == '(') && word_after;
    let assigned = before.contains('=') || before.to_ascii_uppercase().starts_with("PRINT");
    value_before && assigned && number_after
}

/// A `'` text: `s = 'hello'` — a quote after `=`, `(`, `,`, `+` or `&` and
/// another one later with no double quote between (the first one begins a
/// comment in BASIC, so the compiler sees an unfinished statement).
fn single_quoted_text(line: &str) -> bool {
    let bytes = line.as_bytes();
    let mut quoted = false;
    for (i, &b) in bytes.iter().enumerate() {
        match b {
            b'"' => quoted = !quoted,
            b'\'' if !quoted => {
                let before = line[..i].trim_end();
                let opens = before.ends_with(['=', '(', ',', '+', '&']);
                let rest = &line[i + 1..];
                return opens && rest.contains('\'') && !rest[..rest.find('\'').unwrap_or(0)].contains('"');
            }
            _ => {}
        }
    }
    false
}

// ---- reading a compiler error -----------------------------------------------------------------

/// The name a message quotes first (`Unknown SUB or FUNCTION 'ENDIF' — …`).
fn quoted(message: &str) -> Option<&str> {
    message.split('\'').nth(1).filter(|n| !n.is_empty())
}

/// The first of `above`'s lines (nearest first) with one of these habits.
fn habit_above(above: &[&str], ids: &[&str]) -> Option<&'static Hint> {
    above.iter().rev().take(40).find_map(|l| habits(l).into_iter().find(|x| ids.contains(&x.id)))
}

/// The hint for a compiler error: `message` is the compiler's text, `line`
/// the source line it is about, `above` the lines before it (a block
/// closer's error is far from the opener that was written wrongly). `None`
/// when the error has no known fix: nothing is said.
pub fn for_error(message: &str, line: &str, above: &[&str], below: &[&str]) -> Option<&'static Hint> {
    let code = code_of(line);
    let t = code.trim();
    let lower = t.to_ascii_lowercase();
    let w = words_upper(t);
    let first = w.first().map(String::as_str).unwrap_or("");
    let by_message = if message == "Syntax error in SET statement" && first == "SET" {
        Some(id("set"))
    } else if let Some(c) = message.strip_prefix("Unexpected character: ") {
        match c.trim() {
            "%" => Some(id(if t.contains("%=") { "compound" } else { "mod" })),
            "!" => Some(id(if t.contains("!=") { "ne" } else { "bang" })),
            "|" => Some(id("or")),
            "?" => Some(id("ternary")),
            "#" if lower.starts_with("#include") || lower.starts_with("#import") => Some(id("include")),
            _ => None,
        }
    } else if message == "Syntax error in FOR statement" {
        // (a FOR written the other languages' way — not a counting FOR that
        // is merely unfinished)
        let after_for = lower.strip_prefix("for").unwrap_or("").trim_start();
        let foreign = after_for.starts_with('(') || w.iter().any(|x| x == "IN" || x == "EACH" || x == "OF") || t.contains(';');
        foreign.then(|| id("for-foreign"))
    } else if message == "Syntax error in END statement" || message.starts_with("Syntax error in END statement (") {
        if first == "END" && w.get(1).is_some_and(|x| matches!(x.as_str(), "FOR" | "WHILE" | "DO" | "LOOP" | "NEXT" | "WEND")) {
            Some(id("end-loop"))
        } else {
            habit_above(above, &["function", "colon", "braces"])
        }
    } else if message == "Syntax error in CASE statement" {
        above.iter().rev().take(12).any(|l| l.trim_start().to_ascii_lowercase().starts_with("switch")).then(|| id("switch"))
    } else if message == "Syntax error in ELSE statement" || message == "Syntax error in ELSEIF statement" {
        // (an ELSE after a one-line IF, on a line of its own)
        let one_line_if = above.iter().rev().map(|l| code_of(l)).find(|c| !c.trim().is_empty()).is_some_and(|c| {
            let c = c.trim().to_ascii_uppercase();
            c.starts_with("IF ") && c.split_once(" THEN").is_some_and(|(_, rest)| !rest.trim().is_empty())
        });
        if t.contains('{') || t.contains('}') {
            Some(id("braces"))
        } else {
            one_line_if.then(|| id("else-line"))
        }
    } else if message == "Syntax error in IF statement" || message == "Syntax error in WHILE statement" {
        if t.contains('{') || t.contains('}') {
            Some(id("braces"))
        } else if t.ends_with(':') {
            Some(id("colon"))
        } else if message == "Syntax error in IF statement" && first == "IF" && !w.iter().any(|x| x == "THEN") {
            Some(id("if-then"))
        } else {
            None
        }
    } else if message == "Expected end-of-line but got THEN" && lower.starts_with("elif") {
        Some(id("elseif"))
    } else if let Some(token) = message.strip_prefix("Expected end-of-line but got ") {
        match token.trim() {
            ";" => Some(id("semicolon")),
            "?" => Some(id("ternary")),
            "*" if t.contains("**") => Some(id("pow")),
            "/" if t.contains("//") => Some(id(if int_division(t) { "intdiv" } else { "comment" })),
            _ => None,
        }
    } else if message.starts_with("Syntax error: unexpected '") {
        match quoted(message) {
            Some("{" | "}") => Some(id(if lower.starts_with("function ") || lower.starts_with("def ") { "function" } else { "braces" })),
            Some("#") => Some(id("include")),
            Some("[" | "]") => Some(id("brackets")),
            _ => None,
        }
    } else if message.starts_with("Syntax error near ") {
        if lower.starts_with("switch ") || lower.starts_with("switch(") {
            Some(id("switch"))
        } else if single_quoted_text(line) {
            Some(id("squote"))
        } else if ["^=", "\\=", "|=", "<<=", ">>="].iter().any(|o| t.contains(o)) {
            Some(id("compound"))
        } else {
            None
        }
    } else if let Some(name) = message.strip_prefix("Unknown SUB or FUNCTION '").and_then(|r| r.split('\'').next()) {
        unknown_name(name)
    } else if let Some(closer) = message.strip_prefix("Missing ").and_then(|r| r.strip_suffix(" statement")) {
        // (RC.EXE's warnings for a block never closed; the line is the opener's)
        match closer {
            "NEXT" => Some(id("missing-next")),
            "WEND" => Some(id("missing-wend")),
            "LOOP" => Some(id("missing-loop")),
            "END SELECT" => Some(id("missing-end-select")),
            // (the usual cause: ELSE IF, one word too many, below the IF)
            "END IF" => Some(id(if below.iter().take(80).any(|l| code_of(l).trim().to_ascii_uppercase().starts_with("ELSE IF ")) { "else-if" } else { "missing-end-if" })),
            _ => None,
        }
    } else {
        None
    };
    by_message.or_else(|| habits(line).into_iter().next())
}

/// A name the compiler doesn't know that another language writes.
fn unknown_name(name: &str) -> Option<&'static Hint> {
    let lower = name.to_ascii_lowercase();
    match lower.as_str() {
        "endif" => Some(id("endif")),
        "endsub" | "endfunction" | "endselect" | "endwith" | "endtype" | "endcreate" | "endprop" => Some(id("endword")),
        "var" | "const" => Some(id("var")),
        "let" => Some(id("let")),
        "break" => Some(id("break")),
        "continue" => Some(id("continue")),
        "switch" => Some(id("switch")),
        "elif" | "elsif" => Some(id("elseif")),
        "import" | "include" | "using" => Some(id("include")),
        "console" | "printf" | "echo" => Some(id("print")),
        "def" | "func" | "fn" | "void" => Some(id("function")),
        _ => FOREIGN_FUNCTIONS.iter().find(|(n, _)| *n == lower).map(|(_, x)| x),
    }
}

/// The hint under the compiler error at `line` (0-based) of `text`.
pub fn for_error_in(message: &str, text: &str, line: usize) -> Option<&'static Hint> {
    let lines: Vec<&str> = text.lines().collect();
    let here = lines.get(line).copied().unwrap_or("");
    let above = &lines[..line.min(lines.len())];
    let below = lines.get(line + 1..).unwrap_or(&[]);
    for_error(message, here, above, below)
}

/// A build's error report (`file:line:col: error: message`, a line each)
/// with each error's hint under it, indented (`  hint: …`). `read` gives a
/// file's text by the path the report names. A report line that isn't an
/// error with a place stays as it is.
pub fn annotate(report: &str, read: &dyn Fn(&str) -> Option<String>) -> String {
    let mut out: Vec<String> = Vec::new();
    let mut texts: Vec<(String, Option<String>)> = Vec::new();
    for l in report.lines() {
        out.push(l.to_string());
        let Some((place, message)) = l.split_once(": error: ") else { continue };
        let mut parts = place.rsplitn(3, ':');
        let (Some(_col), Some(line_no), Some(file)) = (parts.next(), parts.next(), parts.next()) else { continue };
        let Ok(line_no) = line_no.trim().parse::<usize>() else { continue };
        if !texts.iter().any(|(f, _)| f == file) {
            texts.push((file.to_string(), read(file)));
        }
        let Some(text) = texts.iter().find(|(f, _)| f == file).and_then(|(_, t)| t.as_deref()) else { continue };
        if let Some(hint) = for_error_in(message, text, line_no.saturating_sub(1)) {
            out.push(format!("  hint: {}", hint.text()));
        }
    }
    let mut joined = out.join("\n");
    if report.ends_with('\n') {
        joined.push('\n');
    }
    joined
}

// ---- compiler messages that mean a habit (types, undeclared names) ---------------------------------

/// What a compiler message means, said as a fix (id, hint): data types
/// from other languages, the type of an event's argument.
pub fn for_message(message: &str) -> Option<(&'static str, String)> {
    let after = |prefix: &str| message.strip_prefix(prefix).map(|r| r.trim().trim_matches(['\'', '`', '"']).to_string());
    // (Robert's evening with DeepSeek, 2026-10-20: "expecting DOUBLE but got
    // STRING" on lines storing what a socket read — the variable was never
    // declared, and an undeclared name is a DOUBLE: docs/rai-mistake-ledger.md)
    if message.starts_with("Type mismatch, expecting type DOUBLE, but got STRING") {
        return Some(("undeclared", "the variable on the left is a DOUBLE — declared so, or never declared (an undeclared name is a DOUBLE): DIM it AS STRING before its first use, or store a number in it".to_string()));
    }
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
            _ => {
                let r = component_for(&ty)?;
                format!("{ty} isn't a RapidR type: the component is {r} (rapidr_describe {r})")
            }
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

#[cfg(test)]
mod tests {
    use super::*;

    /// The hint id for an error: the message and the program's line (1-based).
    fn id_of(message: &str, program: &str, line: usize) -> Option<&'static str> {
        for_error_in(message, program, line - 1).map(|x| x.id)
    }

    #[test]
    fn the_compiler_catalog_has_every_hint() {
        // (rapidr i18n compiler writes them into compiler.pot after the messages;
        // a language's file translates them there, hint.<id> being the context)
        let pot = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/../rapidr-diagnostics/lang/compiler.pot")).expect("compiler.pot");
        let template = rapidr_i18n::po::read_messages(&pot);
        let entries = catalog_entries();
        assert_eq!(entries.len(), HINTS.len() + FOREIGN_FUNCTIONS.len() + 1);
        for (key, en) in &entries {
            let found = template.iter().find(|m| &m.context == key);
            assert_eq!(found.map(|m| m.id.as_str()), Some(en.as_str()), "{key} is missing from (or differs in) compiler.pot: rapidr i18n compiler");
        }
        assert_eq!(template.iter().filter(|m| m.context.starts_with("hint.")).count(), entries.len(), "compiler.pot has hints that no longer exist");
        // (English is the hint's own text; a language without it, English too)
        assert_eq!(hint("set").unwrap().text_in("en"), hint("set").unwrap().text);
        assert_eq!(hint("set").unwrap().text_in("zz"), hint("set").unwrap().text);
    }

    #[test]
    fn every_hint_has_a_unique_id_and_a_short_plain_text() {
        let mut seen: Vec<&str> = Vec::new();
        for x in HINTS.iter().chain(FOREIGN_FUNCTIONS.iter().map(|(_, x)| x)) {
            assert!(!seen.contains(&x.id), "id {} twice", x.id);
            seen.push(x.id);
            assert!(!x.text.is_empty() && x.text.chars().count() < 260, "{}: {}", x.id, x.text);
            assert!(!x.text.contains('\n'), "{}: one line", x.id);
            assert_eq!(x.key(), format!("hint.{}", x.id));
        }
    }

    #[test]
    fn set_has_no_such_statement() {
        assert_eq!(id_of("Syntax error in SET statement", "DIM x AS INTEGER\nSET x = 1\n", 2), Some("set"));
        assert!(hint("set").unwrap().text.starts_with("RapidR has no SET: write x = 1"));
        // (a PROPERTY SET is not a statement of this kind)
        assert_eq!(id_of("Syntax error in SET statement", "x = 1\n", 1), None);
    }

    #[test]
    fn let_and_var_are_declared_or_assigned_the_basic_way() {
        assert_eq!(id_of("Unknown SUB or FUNCTION 'let' — did you mean GET$?", "let x = 1\n", 1), Some("let"));
        assert_eq!(id_of("Unknown SUB or FUNCTION 'var' — did you mean VAL?", "var x = 1\n", 1), Some("var"));
        assert_eq!(id_of("Unknown SUB or FUNCTION 'const'", "const x = 1\n", 1), Some("var"));
    }

    #[test]
    fn percent_is_mod() {
        assert_eq!(id_of("Unexpected character: %", "a = 7 % 3\n", 1), Some("mod"));
        assert!(hint("mod").unwrap().text.contains("MOD"));
        assert_eq!(id_of("Unexpected character: %", "x %= 3\n", 1), Some("compound"));
    }

    #[test]
    fn the_shortcuts_that_dont_exist() {
        assert_eq!(id_of("Syntax error near 'x'", "x ^= 2\n", 1), Some("compound"));
        assert_eq!(id_of("Syntax error near 'x'", "x \\= 2\n", 1), Some("compound"));
        // (+= -= *= /= &= are RapidQ's: they compile, no error to say a word under)
        assert!(habits("x += 1").is_empty() && habits("x -= 1").is_empty() && habits("x *= 2").is_empty() && habits("x /= 2").is_empty() && habits("x++").is_empty());
    }

    #[test]
    fn for_loops_of_other_languages() {
        let m = "Syntax error in FOR statement";
        assert_eq!(id_of(m, "FOR i IN range(10)\n", 1), Some("for-foreign"));
        assert_eq!(id_of(m, "for i in range(1, 10):\n", 1), Some("for-foreign"));
        assert_eq!(id_of(m, "for (i = 0; i < 10; i++)\n", 1), Some("for-foreign"));
        assert_eq!(id_of(m, "FOR EACH s IN names\n", 1), Some("for-foreign"));
        assert_eq!(id_of(m, "for s of names\n", 1), Some("for-foreign"));
        // (a counting FOR that is only unfinished: no hint)
        assert_eq!(id_of(m, "FOR i = 1 TO\n", 1), None);
    }

    #[test]
    fn else_if_elif_and_the_end_words() {
        assert_eq!(id_of("Expected end-of-line but got THEN", "IF a THEN\n  b = 1\nelif a = 2 THEN\n", 3), Some("elseif"));
        assert_eq!(id_of("Unknown SUB or FUNCTION 'ENDIF' — x", "ENDIF\n", 1), Some("endif"));
        assert_eq!(id_of("Unknown SUB or FUNCTION 'ENDSUB' — x", "ENDSUB\n", 1), Some("endword"));
        assert_eq!(id_of("Unknown SUB or FUNCTION 'ENDSELECT' — x", "ENDSELECT\n", 1), Some("endword"));
        assert_eq!(id_of("Syntax error in END statement", "FOR i = 1 TO 3\n  PRINT i\nEND FOR\n", 3), Some("end-loop"));
        assert_eq!(id_of("Syntax error in END statement", "WHILE a\n  a = 0\nEND WHILE\n", 3), Some("end-loop"));
        assert_eq!(id_of("Syntax error in IF statement", "IF a = 1\n  b = 1\nEND IF\n", 1), Some("if-then"));
        assert_eq!(id_of("Syntax error in IF statement", "IF a = 1 THEN\n", 1), None);
    }

    #[test]
    fn an_else_after_a_one_line_if() {
        let program = "IF a = 1 THEN PRINT 1\nELSE PRINT 2\n";
        assert_eq!(id_of("Syntax error in ELSE statement", program, 2), Some("else-line"));
        // (after a block IF, an ELSE is fine; any error there has no such hint)
        assert_eq!(id_of("Syntax error in ELSE statement", "IF a = 1 THEN\n  PRINT 1\nELSE PRINT 2\n", 3), None);
    }

    #[test]
    fn blocks_never_closed_are_rc_exes_warnings() {
        assert_eq!(id_of("Missing NEXT statement", "FOR i = 1 TO 3\n  PRINT i\n", 1), Some("missing-next"));
        assert_eq!(id_of("Missing WEND statement", "WHILE a < 3\n  a = a + 1\n", 1), Some("missing-wend"));
        assert_eq!(id_of("Missing LOOP statement", "DO\n  a = a + 1\n", 1), Some("missing-loop"));
        assert_eq!(id_of("Missing END SELECT statement", "SELECT CASE a\n  CASE 1\n", 1), Some("missing-end-select"));
        assert_eq!(id_of("Missing END IF statement", "IF a THEN\n  b = 1\n", 1), Some("missing-end-if"));
        // (the usual cause: ELSE IF)
        assert_eq!(id_of("Missing END IF statement", "IF a THEN\n  b = 1\nELSE IF c THEN\n  b = 2\nEND IF\n", 1), Some("else-if"));
    }

    #[test]
    fn braces_and_colons() {
        assert_eq!(id_of("Syntax error in IF statement", "IF a = 1 {\n", 1), Some("braces"));
        assert_eq!(id_of("Syntax error: unexpected '}'", "}\n", 1), Some("braces"));
        assert_eq!(id_of("Syntax error: unexpected '{'", "function hello(a) {\n", 1), Some("function"));
        assert_eq!(id_of("Syntax error in IF statement", "if a == 1:\n  PRINT 1\n", 1), Some("colon"));
        assert_eq!(id_of("Syntax error in IF statement", "IF a = 1:\n", 1), Some("colon"));
        assert_eq!(id_of("Syntax error in WHILE statement", "WHILE a < 3:\n", 1), Some("colon"));
    }

    #[test]
    fn def_and_function_of_other_languages() {
        // (the error is at the block's closer, far from the opener)
        assert_eq!(id_of("Syntax error in END statement", "def hello():\n  PRINT 1\nEND SUB\n", 3), Some("function"));
        assert_eq!(id_of("Unknown SUB or FUNCTION 'def' — x", "def hello():\n", 1), Some("function"));
    }

    #[test]
    fn print_and_friends() {
        assert_eq!(for_error("Syntax error near 'x'", "print(\"hello\")", &[], &[]).map(|x| x.id), Some("print"));
        assert_eq!(id_of("Unknown SUB or FUNCTION 'console' — x", "console.log(1)\n", 1), Some("print"));
    }

    #[test]
    fn comparison_and_logic_operators() {
        assert_eq!(id_of("Unexpected character: !", "IF a != 1 THEN PRINT 1\n", 1), Some("ne"));
        assert_eq!(id_of("Unexpected character: !", "IF a !== 1 THEN PRINT 1\n", 1), Some("ne"));
        assert_eq!(id_of("Unexpected character: !", "IF !a THEN PRINT 1\n", 1), Some("bang"));
        assert_eq!(id_of("Unexpected character: |", "IF a || b THEN PRINT 1\n", 1), Some("or"));
        // (&& and === compile; no error, no hint)
        assert!(habits("IF a > 0 && b > 0 THEN").is_empty());
    }

    #[test]
    fn null_none_true_false() {
        assert_eq!(for_error("Type mismatch", "x = null", &[], &[]).map(|x| x.id), Some("null"));
        assert_eq!(for_error("Type mismatch", "x = None", &[], &[]).map(|x| x.id), Some("null"));
        assert_eq!(for_error("Type mismatch", "running = true", &[], &[]).map(|x| x.id), Some("bool"));
    }

    #[test]
    fn power_whole_division_and_the_ternary() {
        assert_eq!(id_of("Expected end-of-line but got *", "a = 2 ** 3\n", 1), Some("pow"));
        assert_eq!(id_of("Expected end-of-line but got /", "a = 7 // 2\n", 1), Some("intdiv"));
        assert_eq!(id_of("Expected end-of-line but got ?", "b = a > 0 ? 1 : 2\n", 1), Some("ternary"));
        // (a trailing // note is a comment)
        assert_eq!(id_of("Syntax error near 'x'", "x = 1 // one\n", 1), Some("comment"));
        assert_eq!(id_of("Expected end-of-line but got /", "// a note about this\n", 1), Some("comment"));
    }

    #[test]
    fn semicolons_and_the_square_brackets() {
        assert_eq!(id_of("Expected end-of-line but got ;", "a = 1;\n", 1), Some("semicolon"));
        assert_eq!(id_of("Syntax error: unexpected '['", "x = [1, 2]\n", 1), Some("brackets"));
        assert!(habits("PRINT a;").is_empty(), "PRINT's ; is BASIC");
    }

    #[test]
    fn break_continue_switch_include() {
        assert_eq!(id_of("Unknown SUB or FUNCTION 'break' — x", "IF a THEN break\n", 1), Some("break"));
        assert_eq!(id_of("Unknown SUB or FUNCTION 'continue' — x", "IF a THEN continue\n", 1), Some("continue"));
        assert_eq!(id_of("Syntax error in CASE statement", "switch a\n  CASE 1\n", 2), Some("switch"));
        assert_eq!(id_of("Syntax error near 'switch'", "switch (a) {\n", 1), Some("switch"));
        assert_eq!(id_of("Unexpected character: #", "#include <stdio.h>\n", 1), Some("include"));
        assert_eq!(id_of("Syntax error: unexpected '#'", "#include <stdio.h>\n", 1), Some("include"));
    }

    #[test]
    fn single_quoted_text_is_a_comment() {
        assert_eq!(id_of("Syntax error near 's'", "s = 'hello'\n", 1), Some("squote"));
        assert_eq!(id_of("Syntax error near 's'", "PRINT 'hi there'\n", 1), None);
        assert_eq!(id_of("Syntax error near 'x'", "x = 1 ' it's fine\n", 1), None);
    }

    #[test]
    fn functions_of_other_languages_have_rapidr_names() {
        assert_eq!(id_of("Unknown SUB or FUNCTION 'toupper' — x", "s = toupper(\"a\")\n", 1), Some("fn-toupper"));
        assert_eq!(id_of("Unknown SUB or FUNCTION 'parseInt' — x", "n = parseInt(\"5\")\n", 1), Some("fn-parseint"));
        assert_eq!(id_of("Unknown SUB or FUNCTION 'alert' — x", "alert(\"hi\")\n", 1), Some("fn-alert"));
        assert_eq!(id_of("Unknown SUB or FUNCTION 'Zzyzx' — x", "Zzyzx\n", 1), None);
    }

    #[test]
    fn the_habits_that_were_friendly_hints_before_are_still_found() {
        let ids = |line: &str| habits(line).iter().map(|x| x.id).collect::<Vec<_>>();
        assert_eq!(ids("DIM waitingForOperand AS BOOLEAN AS BOOLEAN"), ["as-as"]);
        assert_eq!(ids("x = 1;"), ["semicolon"]);
        assert_eq!(ids("return total"), ["return"]);
        assert_eq!(ids("var total = 0"), ["var"]);
        assert_eq!(ids("let total = 0"), ["let"]);
        assert_eq!(ids("} else if (x > 1) {"), ["braces", "elseif"]);
        assert_eq!(ids("ENDIF"), ["endif"]);
        assert_eq!(ids("n = name.length"), ["length"]);
        assert_eq!(ids("b = new Button()"), ["new"]);
        assert_eq!(ids("OnClick = \"\""), ["event"]);
        // (inside strings and comments: nothing)
        assert!(ids("PRINT \"a == b && c\" ' x != y").is_empty());
        assert!(ids("Caption = \"[ok]\"").is_empty());
        assert!(ids("URL = \"http://x\"").is_empty());
    }

    #[test]
    fn an_error_report_gets_its_hints_under_it() {
        let report = "a.rr:2:1: error: Syntax error in SET statement\na.rr:3:1: error: Something nobody has a hint for\nwarning: x\n";
        let text = "DIM x AS INTEGER\nSET x = 1\nPRINT x\n";
        let out = annotate(report, &|f| (f == "a.rr").then(|| text.to_string()));
        let lines: Vec<&str> = out.lines().collect();
        assert_eq!(lines[0], "a.rr:2:1: error: Syntax error in SET statement");
        assert_eq!(lines[1], format!("  hint: {}", hint("set").unwrap().text));
        assert_eq!(lines[2], "a.rr:3:1: error: Something nobody has a hint for");
        assert_eq!(lines.len(), 4, "{out}");
        assert!(out.ends_with('\n'));
        // (a file that can't be read: the report as it was)
        assert_eq!(annotate(report, &|_| None), report);
    }

    #[test]
    fn messages_read_for_what_they_mean() {
        let (id, t) = for_message("Type mismatch, expecting type DOUBLE, but got STRING").unwrap();
        assert!(id == "undeclared" && t.contains("DIM it AS STRING"), "{t}");
        let (_, t) = for_message("Unknown data type BOOLEAN").unwrap();
        assert!(t.contains("INTEGER"), "{t}");
        let (_, t) = for_message("Unknown type TextBox").unwrap();
        assert!(t.contains("REdit"), "{t}");
        assert!(for_message("Syntax error near 'x'").is_none());
    }
}
