//! Formatting: re-indents blocks (SUB, FUNCTION, TYPE, CREATE, WITH, IF,
//! FOR, WHILE, DO, SELECT CASE …) and removes trailing blanks. It never
//! changes a token, so it can't change what a program does: words keep
//! their case, strings and comments their text, line breaks stay.

use crate::TextEdit;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Block {
    Plain,
    /// SELECT CASE: its CASEs one level in, their bodies two.
    Select,
}

/// Edits re-indenting `text`, one level being `indent`.
pub(crate) fn format(text: &str, indent: &str) -> Vec<TextEdit> {
    let mut edits = Vec::new();
    let mut stack: Vec<Block> = Vec::new();
    let mut in_rust = false;
    let mut continued = false;
    let mut offset = 0;
    for raw in text.split_inclusive('\n') {
        let line_start = offset;
        offset += raw.len();
        let line = raw.trim_end_matches(['\n', '\r']);
        let ending = &raw[line.len()..];
        let content = line.trim();
        let upper = content.to_ascii_uppercase();
        let first = first_words(&upper);

        if in_rust {
            if first.0 == "RUSTEND" {
                in_rust = false;
            } else {
                continue;
            }
        }
        // The line's own level: closers and middles go out first.
        let mut level = stack.len();
        let mut select_case_line = false;
        match first.0.as_str() {
            "END" if matches!(first.1.as_str(), "SUB" | "FUNCTION" | "SUBI" | "FUNCTIONI" | "IF" | "TYPE" | "CREATE" | "WITH" | "SELECT" | "CONSTRUCTOR" | "EVENT" | "PROPERTY") => {
                if first.1 == "SELECT" {
                    // (the open CASE, then the SELECT)
                    if stack.last() == Some(&Block::Plain) && stack.len() >= 2 && stack[stack.len() - 2] == Block::Select {
                        stack.pop();
                    }
                }
                stack.pop();
                level = stack.len();
            }
            "NEXT" | "WEND" | "LOOP" => {
                stack.pop();
                level = stack.len();
            }
            "ELSE" | "ELSEIF" => {
                level = stack.len().saturating_sub(1);
            }
            "CASE" => {
                if stack.last() == Some(&Block::Plain) && stack.len() >= 2 && stack[stack.len() - 2] == Block::Select {
                    stack.pop();
                }
                level = stack.len();
                select_case_line = true;
            }
            _ => {}
        }
        let is_label = is_label_line(content);
        let is_directive = content.starts_with('$') || content.starts_with('#');
        let mut want = if content.is_empty() || is_label || is_directive && level == 0 {
            String::new()
        } else {
            indent.repeat(level)
        };
        if continued && !content.is_empty() {
            want.push_str(indent);
        }
        let new_line = if content.is_empty() { String::new() } else { format!("{want}{content}") };
        if new_line != line {
            edits.push(TextEdit { start: line_start, end: line_start + line.len(), text: new_line });
        }
        let _ = ending;
        continued = code_part(content).trim_end().ends_with(" _") || code_part(content).trim_end() == "_";

        // What the line opens.
        if select_case_line {
            stack.push(Block::Plain);
            continue;
        }
        match first.0.as_str() {
            "RUSTSTART" => in_rust = true,
            "SUB" | "FUNCTION" | "SUBI" | "FUNCTIONI" | "CONSTRUCTOR" | "WITH" | "CREATE" | "WHILE" | "DO" | "FOR" => {
                if !closes_on_same_line(&upper, &first.0) {
                    stack.push(Block::Plain);
                }
            }
            "PRIVATE" | "PUBLIC" if matches!(first.1.as_str(), "SUB" | "FUNCTION") => stack.push(Block::Plain),
            "EVENT" | "PROPERTY" if starts_block(&upper) => stack.push(Block::Plain),
            "TYPE" if !upper.contains(" AS ") || upper.contains(" EXTENDS ") => stack.push(Block::Plain),
            "SELECT" => stack.push(Block::Select),
            "IF" if opens_if(&upper) => stack.push(Block::Plain),
            "ELSE" | "ELSEIF" => {}
            _ => {}
        }
    }
    edits
}

/// The first two words (upper case), for `END IF`, `SELECT CASE`.
fn first_words(upper: &str) -> (String, String) {
    let code = code_part(upper);
    let mut words = code.split(|c: char| !(c.is_ascii_alphanumeric() || c == '_' || c == '$')).filter(|w| !w.is_empty());
    (words.next().unwrap_or("").to_string(), words.next().unwrap_or("").to_string())
}

/// A line without its comment (strings kept).
fn code_part(line: &str) -> &str {
    let mut in_string = false;
    for (i, c) in line.char_indices() {
        match c {
            '"' => in_string = !in_string,
            '\'' if !in_string => return &line[..i],
            _ => {}
        }
    }
    line
}

/// `IF … THEN` with nothing after THEN opens a block; `IF x THEN y` doesn't.
fn opens_if(upper: &str) -> bool {
    let code = code_part(upper).trim_end();
    code.ends_with(" THEN") || code == "THEN"
}

/// A one-line loop (`FOR i = 1 TO 3: PRINT i: NEXT`).
fn closes_on_same_line(upper: &str, opener: &str) -> bool {
    let code = code_part(upper);
    let closer = match opener {
        "FOR" => "NEXT",
        "WHILE" => "WEND",
        "DO" => "LOOP",
        "WITH" => "END WITH",
        "SUB" => "END SUB",
        "FUNCTION" => "END FUNCTION",
        _ => return false,
    };
    code.split(':').skip(1).any(|part| part.trim_start().starts_with(closer))
}

/// `EVENT OnClick` / `PROPERTY SET x(v)` in a TYPE (blocks), not
/// `PROPERTY SET Name` on a field line.
fn starts_block(upper: &str) -> bool {
    let code = code_part(upper);
    code.starts_with("EVENT") || code.contains('(')
}

/// `name:` alone on a line.
fn is_label_line(content: &str) -> bool {
    let code = code_part(content).trim_end();
    code.len() > 1
        && code.ends_with(':')
        && code[..code.len() - 1].chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
        && !code.starts_with(|c: char| c.is_ascii_digit())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn apply(text: &str, edits: &[TextEdit]) -> String {
        let mut out = text.to_string();
        for e in edits.iter().rev() {
            out.replace_range(e.start..e.end, &e.text);
        }
        out
    }

    #[test]
    fn blocks_are_indented() {
        let src = "SUB Go(n AS INTEGER)\nIF n > 0 THEN\nPRINT n\nELSE\nPRINT 0\nEND IF\nIF n THEN PRINT \"one line\"\nSELECT CASE n\nCASE 1\nPRINT 1\nCASE ELSE\nPRINT 2\nEND SELECT\nFOR i = 1 TO 3: PRINT i: NEXT\nEND SUB   \nCREATE F AS QFORM\nCaption = \"x\" ' a comment\nCREATE B AS QBUTTON\nLeft = 1\nEND CREATE\nEND CREATE\n";
        let want = "SUB Go(n AS INTEGER)\n    IF n > 0 THEN\n        PRINT n\n    ELSE\n        PRINT 0\n    END IF\n    IF n THEN PRINT \"one line\"\n    SELECT CASE n\n        CASE 1\n            PRINT 1\n        CASE ELSE\n            PRINT 2\n    END SELECT\n    FOR i = 1 TO 3: PRINT i: NEXT\nEND SUB\nCREATE F AS QFORM\n    Caption = \"x\" ' a comment\n    CREATE B AS QBUTTON\n        Left = 1\n    END CREATE\nEND CREATE\n";
        let got = apply(src, &format(src, "    "));
        assert_eq!(got, want);
        // idempotent
        assert!(format(&got, "    ").is_empty());
    }

    #[test]
    fn types_and_labels() {
        let src = "TYPE P\nx AS INTEGER\nEND TYPE\nstart:\nGOTO start\n";
        let got = apply(src, &format(src, "  "));
        assert_eq!(got, "TYPE P\n  x AS INTEGER\nEND TYPE\nstart:\nGOTO start\n");
    }
}
