//! RapidQ's system tray, the one Windows call RapidR keeps: QNOTIFYICONDATA
//! exists for `Shell_NotifyIcon` (its manual: "its use is primarily
//! restricted to the Shell_NotifyIcon API call"), and a program hears its
//! icon's clicks in its form's WndProc. Both compilers run this pass first:
//!
//! - `DECLARE … Shell_NotifyIconA … LIB "shell32"` (or `Shell_NotifyIcon`,
//!   `Shell_NotifyIconW`, under whatever name the program gives it) goes,
//!   and every call of that name becomes `__shell_notifyicon(message,
//!   data)` — RapidR's tray on every platform (`rapidr_value::tray`); a
//!   QNOTIFYICONDATA named as the argument passes as its id;
//! - a QFORM's `WndProc = Handler` (in its CREATE, or `Form.WndProc =
//!   Handler`) is its event `OnWndProc`: the runtimes fire it with the
//!   tray's messages, (hWnd, uMsg, wParam, lParam) as Windows would.
//!
//! Every other Windows call stays the compilers' clear error.

use std::collections::HashSet;

use crate::*;

/// The builtin the tray's calls become.
pub const SHELL_NOTIFY_ICON: &str = "__shell_notifyicon";

/// Whether DECLARE `d` is Windows' Shell_NotifyIcon (shell32).
fn is_shell_notify_icon(d: &DeclareStatement) -> bool {
    let Some(lib) = &d.lib else { return false };
    let base = lib.trim().rsplit(['\\', '/']).next().unwrap_or("").to_ascii_lowercase();
    if base.trim_end_matches(".dll") != "shell32" {
        return false;
    }
    let api = d.alias.as_deref().unwrap_or(&d.name).trim().to_ascii_lowercase();
    matches!(api.as_str(), "shell_notifyicon" | "shell_notifyicona" | "shell_notifyiconw")
}

/// The pass (see the module docs).
pub fn lower(program: &Program) -> Program {
    let mut program = program.clone();
    let names: HashSet<String> = program
        .statements
        .iter()
        .filter_map(|s| match s {
            Statement::Declare(d) if is_shell_notify_icon(d) => Some(d.name.to_ascii_lowercase()),
            _ => None,
        })
        .collect();
    // The program's forms (CREATE / DIM … AS QFORM): `X.WndProc = H`.
    let mut forms: HashSet<String> = HashSet::new();
    walk(
        &program.statements,
        &mut |s| match s {
            Statement::Create(c) if is_form(&c.type_name) => {
                forms.insert(c.name.to_ascii_lowercase());
            }
            Statement::Dim(d) if is_form(&d.type_name) => forms.extend(d.declarators.iter().map(|v| v.name.to_ascii_lowercase())),
            _ => {}
        },
        &mut |_| {},
    );
    if names.is_empty() && forms.is_empty() {
        return program;
    }
    program.statements.retain(|s| !matches!(s, Statement::Declare(d) if is_shell_notify_icon(d)));
    walk_statements_mut(&mut program.statements, &mut |s| match s {
        Statement::Call(c) => {
            if let Expression::Identifier(id) = &c.callee {
                if names.contains(&id.name.to_ascii_lowercase()) {
                    c.callee = Expression::Identifier(Identifier { span: id.span, name: SHELL_NOTIFY_ICON.into() });
                    data_by_id(&mut c.args);
                }
            }
        }
        Statement::Create(c) if is_form(&c.type_name) => {
            for b in &mut c.body {
                if let Statement::Assignment(a) = b {
                    if let Expression::Identifier(t) = &mut a.target {
                        if t.name.eq_ignore_ascii_case("wndproc") && matches!(a.value, Expression::Identifier(_)) {
                            t.name = "OnWndProc".into();
                        }
                    }
                }
            }
        }
        Statement::Assignment(a) => {
            if let Expression::MemberAccess(m) = &mut a.target {
                if m.member.eq_ignore_ascii_case("wndproc") && matches!(m.object.as_ref(), Expression::Identifier(o) if forms.contains(&o.name.to_ascii_lowercase())) {
                    m.member = "OnWndProc".into();
                }
            }
        }
        _ => {}
    });
    if !names.is_empty() {
        walk_expressions_mut(&mut program.statements, true, &mut |e| {
            if let Expression::FunctionCall(f) = e {
                if let Expression::Identifier(id) = f.callee.as_ref() {
                    if names.contains(&id.name.to_ascii_lowercase()) {
                        let span = id.span;
                        f.callee = Box::new(Expression::Identifier(Identifier { span, name: SHELL_NOTIFY_ICON.into() }));
                        data_by_id(&mut f.args);
                    }
                }
            }
        });
    }
    program
}

fn is_form(type_name: &str) -> bool {
    matches!(canonical_type_name(type_name).to_ascii_uppercase().as_str(), "RFORM" | "RFORMMDI")
}

/// The QNOTIFYICONDATA argument (the second) named by a plain name: its id,
/// as the runtimes know it.
fn data_by_id(args: &mut [Expression]) {
    if let Some(arg) = args.get_mut(1) {
        if let Expression::Identifier(id) = arg {
            *arg = Expression::Literal(Literal { span: id.span, value: LiteralValue::String(id.name.clone()) });
        }
    }
}
