//! RapidR's `END n` (the program ends with exit code n) must not turn a
//! typo in a block's closing word into a silent exit: `END SUBB`,
//! `END FUNCTON` and `END Enum` read as `END` and a bare name, and RapidQ
//! gives a syntax error for each. So a bare name after END is an exit code
//! only if it names something the program declared and can see — a DIM,
//! CONST, DEFxxx, STATIC, a parameter or local of the routine around it, a
//! TYPE's field in its methods, a CREATEd component, a SUB or FUNCTION,
//! a DECLAREd routine, or one of the language's own constants and
//! functions. A name merely used (an implicit variable) doesn't count, with
//! `$TYPECHECK` off too. Anything longer than a bare name — a number, a
//! sign, a parenthesis, `code + 1`, `Count()` — is an expression and stays
//! accepted. The parser runs [`unresolved_end_codes`] after the tree is
//! built and reports each hit as RC.EXE's "Syntax error in END statement"
//! (with [`closer_suggestion`]'s hint when the word is close to a block's
//! closer).

use std::collections::HashSet;

use crate::*;

/// A bare name after END that names nothing the program declared.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnresolvedEnd {
    /// The END statement's place.
    pub span: TextSpan,
    /// The word after it, as written.
    pub word: String,
}

/// The block closers a mistyped word may be meant for.
const CLOSERS: &[&str] = &["SUB", "FUNCTION", "SUBI", "FUNCTIONI", "IF", "SELECT", "WITH", "CREATE", "TYPE", "CONSTRUCTOR", "EVENT", "PROPERTY"];

/// The block closer `word` is probably a typo of (`SUBB` → `SUB`), if any:
/// one letter off for short words, two for longer ones.
pub fn closer_suggestion(word: &str) -> Option<&'static str> {
    let word = word.to_ascii_uppercase();
    if CLOSERS.contains(&word.as_str()) {
        return None;
    }
    let allowed = if word.len() <= 4 { 1 } else { 2 };
    CLOSERS
        .iter()
        .map(|c| (distance(&word, c), *c))
        .filter(|(d, c)| *d > 0 && *d <= allowed && word.len().abs_diff(c.len()) <= allowed)
        .min_by_key(|(d, _)| *d)
        .map(|(_, c)| c)
}

/// The edit distance of two words (insertions, deletions, replacements and
/// swapped neighbours: `WIHT` is one off `WITH`).
fn distance(a: &str, b: &str) -> usize {
    let (a, b): (Vec<char>, Vec<char>) = (a.chars().collect(), b.chars().collect());
    let mut d = vec![vec![0usize; b.len() + 1]; a.len() + 1];
    for (i, row) in d.iter_mut().enumerate() {
        row[0] = i;
    }
    for j in 0..=b.len() {
        d[0][j] = j;
    }
    for i in 1..=a.len() {
        for j in 1..=b.len() {
            let cost = usize::from(a[i - 1] != b[j - 1]);
            d[i][j] = (d[i - 1][j] + 1).min(d[i][j - 1] + 1).min(d[i - 1][j - 1] + cost);
            if i > 1 && j > 1 && a[i - 1] == b[j - 2] && a[i - 2] == b[j - 1] {
                d[i][j] = d[i][j].min(d[i - 2][j - 2] + 1);
            }
        }
    }
    d[a.len()][b.len()]
}

fn key(name: &str) -> String {
    strip_type_suffix(name).to_ascii_lowercase()
}

/// What `stmts` declare (their blocks' too, not the routines'): DIM, CONST,
/// STATIC, DEFxxx, components, and the routines' own names.
fn declared(stmts: &[Statement], into: &mut HashSet<String>) {
    for s in stmts {
        match s {
            Statement::Dim(d) => into.extend(d.declarators.iter().map(|v| key(&v.name))),
            Statement::Const(c) => {
                into.insert(key(&c.name));
            }
            Statement::Create(c) => {
                into.insert(key(&c.name));
                declared(&c.body, into);
            }
            Statement::Declare(d) => {
                into.insert(key(&d.name));
            }
            Statement::Subroutine(r) => {
                into.insert(key(&r.name));
            }
            Statement::Function(f) => {
                into.insert(key(&f.name));
            }
            Statement::Type(t) => {
                into.insert(key(&t.name));
            }
            _ => {
                for body in blocks(s) {
                    declared(body, into);
                }
            }
        }
    }
}

/// The statements inside a block statement (IF, loops, SELECT, WITH).
fn blocks(s: &Statement) -> Vec<&[Statement]> {
    match s {
        Statement::If(i) => {
            let mut out: Vec<&[Statement]> = vec![&i.then_body];
            out.extend(i.elseif_branches.iter().map(|b| b.body.as_slice()));
            out.push(&i.else_body);
            out
        }
        Statement::For(f) => vec![&f.body],
        Statement::While(w) => vec![&w.body],
        Statement::DoLoop(d) => vec![&d.body],
        Statement::With(w) => vec![&w.body],
        Statement::SelectCase(c) => {
            let mut out: Vec<&[Statement]> = c.cases.iter().map(|b| b.body.as_slice()).collect();
            out.push(&c.case_else);
            out
        }
        Statement::Create(c) => vec![&c.body],
        _ => Vec::new(),
    }
}

/// The language's own names: constants, bare functions, True / False.
fn builtin(k: &str) -> bool {
    rapidr_constant(k).is_some() || rapidr_lang::constant(k).is_some() || rapidr_lang::builtin(k).is_some() || matches!(k, "true" | "false")
}

struct Scope<'a> {
    names: HashSet<String>,
    out: &'a mut Vec<UnresolvedEnd>,
}

impl Scope<'_> {
    /// The END statements among `stmts` (and their blocks').
    fn check(&mut self, stmts: &[Statement]) {
        for s in stmts {
            if let Statement::Call(c) = s {
                if matches!(&c.callee, Expression::Identifier(i) if i.name.eq_ignore_ascii_case("END")) {
                    if let [Expression::Identifier(word)] = c.args.as_slice() {
                        let k = key(&word.name);
                        if !self.names.contains(&k) && !builtin(&k) {
                            self.out.push(UnresolvedEnd { span: c.span, word: word.name.clone() });
                        }
                    }
                }
            }
            for body in blocks(s) {
                self.check(body);
            }
        }
    }
}

/// A routine (or a TYPE's method) with its own names added to `outer`.
fn routine(outer: &HashSet<String>, own: &[&str], params: &[Parameter], body: &[Statement], out: &mut Vec<UnresolvedEnd>) {
    let mut names = outer.clone();
    names.extend(own.iter().map(|n| key(n)));
    names.extend(params.iter().map(|p| key(&p.name)));
    declared(body, &mut names);
    Scope { names, out }.check(body);
}

/// The bare names after an END in `program` that name nothing declared in
/// their scope, in source order.
pub fn unresolved_end_codes(program: &Program) -> Vec<UnresolvedEnd> {
    let mut out = Vec::new();
    let mut globals = HashSet::new();
    declared(&program.statements, &mut globals);
    // The main program (routines, which have scopes of their own, are
    // not descended into).
    Scope { names: globals.clone(), out: &mut out }.check(&program.statements);
    for s in &program.statements {
        match s {
            Statement::Subroutine(r) => routine(&globals, &[&r.name], &r.params, &r.body, &mut out),
            Statement::Function(f) => routine(&globals, &[&f.name, "result"], &f.params, &f.body, &mut out),
            Statement::Type(t) => {
                let mut outer = globals.clone();
                outer.extend(t.fields.iter().map(|f| key(&f.name)));
                outer.insert("this".to_string());
                routine(&outer, &[], &[], &t.constructor, &mut out);
                for m in &t.methods {
                    match m {
                        Statement::Subroutine(r) => routine(&outer, &[&r.name], &r.params, &r.body, &mut out),
                        Statement::Function(f) => routine(&outer, &[&f.name, "result"], &f.params, &f.body, &mut out),
                        _ => {}
                    }
                }
                for e in &t.events {
                    routine(&outer, &[], &e.params, &e.body, &mut out);
                }
            }
            _ => {}
        }
    }
    out.sort_by_key(|u| u.span.start);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn typos_of_a_closer_are_suggested_the_closer() {
        assert_eq!(closer_suggestion("SUBB"), Some("SUB"));
        assert_eq!(closer_suggestion("subb"), Some("SUB"));
        assert_eq!(closer_suggestion("FUNCTON"), Some("FUNCTION"));
        assert_eq!(closer_suggestion("IFF"), Some("IF"));
        assert_eq!(closer_suggestion("SELCT"), Some("SELECT"));
        assert_eq!(closer_suggestion("WIHT"), Some("WITH"));
        // (a closer itself, or a word far from all of them: nothing to say)
        assert_eq!(closer_suggestion("SUB"), None);
        assert_eq!(closer_suggestion("Enum"), None);
        assert_eq!(closer_suggestion("code"), None);
        assert_eq!(closer_suggestion("failures"), None);
    }
}
