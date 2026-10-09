//! RPROJECTTREE's model (docs/ide-components.md §3.5): the data the kernel
//! draws (`rapidr-ui-kernel`'s `components/panels/project_tree.rs`) and the
//! program's members and events through the runtime glue.
//!
//! The project is `rapidr_project`'s, read through the runtime's file hooks
//! ([`Runtime::read_file`]: the disk on the desktop, the page's own files on
//! the web) — a `.rrproj`, or a `.bas` / `.rr` file and what it
//! `$INCLUDE`s — or given as text (`LoadText`, `SetFileText`). The tree
//! ([`model`]) groups its files by kind, lists each form's components
//! ([`scan`]), and keeps what is open, selected, renamed, deleted, dragged.

pub mod model;
pub mod scan;

#[cfg(test)]
mod tests;

pub use model::{Confirm, Drag, DropAt, Node, NodeKind, ProjectTree, KEY_PROJECT};

use super::runtime::{basic_bool, truth, Runtime};
use crate::Value;

crate::panel_models!(ProjectTree);

/// What the user did to it (from the kernel), for the program to hear.
#[derive(Clone, Debug, PartialEq)]
pub enum User {
    /// The selection moved to node `key` (the model has it): OnSelect.
    Select(String),
    /// Node `key` opened (a double click, Enter): OnOpen.
    Open(String),
    /// File `old` renamed `new` in place (a valid name): OnRename, which
    /// may cancel.
    Rename(String, String),
    /// The file NewFile added named `new` (`old`: its free name): OnNewFile.
    Named(String, String),
    /// File `path`'s removal confirmed: OnDelete, which may cancel.
    Delete(String),
    /// File `path` dragged to `new` at `index` of the project's order:
    /// OnMove.
    Move(String, String, usize),
}

fn int(n: i64) -> Value {
    Value::Integer(n)
}

/// A file's path as the program reads it through the runtime (the
/// project's folder already before it).
fn reader<R: Runtime>(rt: R) -> impl Fn(&str) -> Option<String> {
    move |p: &str| rt.read_file(p)
}

/// The file `path` names (a file's path, any case), else the selected one.
fn file_arg(name: &str, path: Option<String>) -> Option<String> {
    with(name, |m| {
        let p = path.filter(|p| !p.trim().is_empty()).unwrap_or_else(|| m.selected.clone());
        m.file(&p).map(|f| f.path.clone())
    })
    .flatten()
}

/// Opens the nodes above `key`, selects it and has it scrolled to.
fn reveal(m: &mut ProjectTree, key: &str) {
    m.open_above(key);
    m.selected = key.to_string();
    m.reveal = true;
}

/// Where Save writes the project: its `.rrproj` (a source file's project
/// beside it, named after it).
fn save_path(m: &ProjectTree) -> String {
    let p = m.path.trim().replace('\\', "/");
    if p.to_ascii_lowercase().ends_with(".rrproj") {
        return m.path.trim().to_string();
    }
    let name = model::name_of(&p);
    let stem = match name.rfind('.') {
        Some(i) if i > 0 => &name[..i],
        _ if name.is_empty() => if m.project.name.is_empty() { "Project" } else { m.project.name.as_str() },
        _ => name,
    };
    format!("{}{stem}.rrproj", model::folder_of(&p))
}

/// Renames file `old` to `new` after OnRename (unless it cancels).
fn rename<R: Runtime>(rt: R, name: &str, old: String, new: String) {
    let n = name.to_string();
    rt.fire_then(
        name,
        "onrename",
        &[Value::String(old.clone()), Value::String(new.clone()), int(0)],
        Box::new(move |args: &[Value]| {
            if !args.get(2).is_some_and(truth) {
                with_mut(&n, |m| m.apply_rename(&old, &new));
            }
            rt.invalidate();
        }),
    );
}

/// Its methods; `None`: not one of its own.
pub fn rt_method<R: Runtime>(rt: R, name: &str, method: &str, args: &[Value]) -> Option<Value> {
    let arg = |i: usize| args.get(i).cloned().unwrap_or(Value::Null);
    let s = |i: usize| arg(i).to_string_val();
    let read = reader(rt);
    let out = match method {
        "refresh" => {
            let path = with(name, |m| m.path.clone()).unwrap_or_default();
            let again = !path.is_empty() && with_mut(name, |m| m.load(&path, None, &read));
            if !again {
                with_mut(name, |m| {
                    let given = m.given.clone();
                    m.texts.retain(|k, _| given.contains(k));
                    m.scan_all(&read);
                });
            }
            int(0)
        }
        "loadtext" => basic_bool(with_mut(name, |m| m.load(&s(1), Some(s(0)), &read))),
        "setfiletext" => {
            with_mut(name, |m| m.set_text(&s(0), &s(1)));
            int(0)
        }
        // ComponentNames(ExceptPath): the names the other files' CREATE
        // blocks make, separated by commas (a designer's ReservedNames)
        "componentnames" => Value::String(with(name, |m| m.component_names(&s(0)).join(",")).unwrap_or_default()),
        "file" => {
            let i = arg(0).to_i64();
            Value::String(with(name, |m| usize::try_from(i).ok().and_then(|i| m.project.file(i)).map(|f| f.path.clone())).flatten().unwrap_or_default())
        }
        "filekind" => Value::String(with(name, |m| m.file(&s(0)).map(|f| f.kind.as_str().to_string())).flatten().unwrap_or_default()),
        "addfile" => {
            let kind = model::parse_kind(&s(1));
            if args.len() > 1 && !s(1).trim().is_empty() && kind.is_none() {
                return Some(basic_bool(false));
            }
            basic_bool(with_mut(name, |m| m.add_file(&s(0), kind, &read)))
        }
        "removefile" => basic_bool(with_mut(name, |m| m.remove_file(&s(0)))),
        "filemodified" => {
            let key = s(0).replace('\\', "/").to_lowercase();
            let on = with_mut(name, |m| {
                if let Some(v) = args.get(1) {
                    if truth(v) {
                        m.dirty.insert(key.clone());
                    } else {
                        m.dirty.remove(&key);
                    }
                }
                m.dirty.contains(&key)
            });
            if args.len() > 1 {
                rt.invalidate();
            }
            basic_bool(on)
        }
        "newfile" => {
            let Some(kind) = model::parse_kind(&s(0)) else { return Some(Value::String(String::new())) };
            let wanted = (args.len() > 1).then(|| s(1));
            let path = with_mut(name, |m| {
                let path = m.free_name(kind, wanted.as_deref());
                m.add_file(&path, Some(kind), &|_| None);
                reveal(m, &path);
                m.new_file = Some(path.clone());
                m.edit_request = Some(path.clone());
                path
            });
            rt.focus(name);
            Value::String(path)
        }
        "rename" => {
            let Some(old) = file_arg(name, args.first().map(Value::to_string_val)) else { return Some(int(0)) };
            if args.len() > 1 {
                if let Some(Ok(new)) = with(name, |m| m.validate(&old, &s(1))) {
                    if new != old {
                        rename(rt, name, old, new);
                    }
                }
            } else {
                with_mut(name, |m| {
                    reveal(m, &old);
                    m.edit_request = Some(old.clone());
                });
                rt.focus(name);
            }
            int(0)
        }
        "delete" => {
            let Some(path) = file_arg(name, args.first().map(Value::to_string_val)) else { return Some(int(0)) };
            with_mut(name, |m| {
                reveal(m, &path);
                m.confirm = Some(Confirm { path: path.clone(), ..Confirm::default() });
            });
            rt.focus(name);
            int(0)
        }
        "reveal" => {
            with_mut(name, |m| {
                if let Some(key) = m.key_of(&s(0)) {
                    reveal(m, &key);
                }
            });
            int(0)
        }
        "save" => {
            let (path, text) = with(name, |m| (save_path(m), m.project.to_toml())).unwrap_or_default();
            let ok = !path.is_empty() && rt.write_file(&path, &text);
            if ok {
                with_mut(name, |m| {
                    m.path = path;
                    m.modified = false;
                });
            }
            basic_bool(ok)
        }
        "projecttext" => Value::String(with(name, |m| m.project.to_toml()).unwrap_or_default()),
        "expandall" => {
            with_mut(name, |m| m.open_all(true));
            int(0)
        }
        "collapseall" => {
            with_mut(name, |m| m.open_all(false));
            int(0)
        }
        _ => return None,
    };
    Some(out)
}

/// Its properties its model answers.
pub fn rt_get<R: Runtime>(_rt: R, name: &str, prop: &str) -> Option<Value> {
    let m = |f: &dyn Fn(&ProjectTree) -> Value| with(name, f).unwrap_or_else(|| f(&ProjectTree::default()));
    Some(match prop {
        "project" => m(&|t| Value::String(t.path.clone())),
        "projectname" => m(&|t| Value::String(if t.loaded { t.project.name.clone() } else { String::new() })),
        "showfiles" => m(&|t| basic_bool(t.show_files)),
        "showforms" => m(&|t| basic_bool(t.show_forms)),
        "showcomponents" => m(&|t| basic_bool(t.show_components)),
        "selected" => m(&|t| Value::String(ProjectTree::public(&t.selected))),
        "filecount" => m(&|t| int(t.project.file_count() as i64)),
        "modified" => m(&|t| basic_bool(t.modified)),
        _ => return None,
    })
}

/// Its properties its model keeps: whether `prop` was one.
pub fn rt_set<R: Runtime>(rt: R, name: &str, prop: &str, v: &Value) -> bool {
    match prop {
        "project" => {
            let path = v.to_string_val();
            let read = reader(rt);
            with_mut(name, |m| {
                if path.trim().is_empty() {
                    *m = ProjectTree { show_files: m.show_files, show_forms: m.show_forms, show_components: m.show_components, ..ProjectTree::default() };
                } else if !m.load(&path, None, &read) {
                    // (nothing there: an empty tree that remembers the name)
                    *m = ProjectTree { path: path.clone(), show_files: m.show_files, show_forms: m.show_forms, show_components: m.show_components, ..ProjectTree::default() };
                }
            });
        }
        "showfiles" => with_mut(name, |m| m.show_files = truth(v)),
        "showforms" => with_mut(name, |m| m.show_forms = truth(v)),
        "showcomponents" => with_mut(name, |m| m.show_components = truth(v)),
        "selected" => with_mut(name, |m| {
            m.selected = m.key_of(&v.to_string_val()).unwrap_or_default();
            if !m.selected.is_empty() {
                let key = m.selected.clone();
                reveal(m, &key);
            }
        }),
        "modified" => with_mut(name, |m| m.modified = truth(v)),
        _ => return false,
    }
    true
}

/// What the user did.
pub fn rt_user<R: Runtime>(rt: R, name: &str, action: User) {
    match action {
        User::Select(key) => rt.fire(name, "onselect", &[Value::String(ProjectTree::public(&key))]),
        User::Open(key) => rt.fire(name, "onopen", &[Value::String(ProjectTree::public(&key))]),
        User::Rename(old, new) => rename(rt, name, old, new),
        User::Named(old, new) => {
            let kind = with_mut(name, |m| {
                m.apply_rename(&old, &new);
                m.new_file = None;
                m.file(&new).map(|f| f.kind.as_str().to_string()).unwrap_or_default()
            });
            rt.fire(name, "onnewfile", &[Value::String(new), Value::String(kind)]);
        }
        User::Delete(path) => {
            let n = name.to_string();
            rt.fire_then(
                name,
                "ondelete",
                &[Value::String(path.clone()), int(0)],
                Box::new(move |args: &[Value]| {
                    if !args.get(1).is_some_and(truth) {
                        let before = with(&n, |m| m.selected.clone()).unwrap_or_default();
                        let after = with_mut(&n, |m| {
                            m.remove_file(&path);
                            m.selected.clone()
                        });
                        if after != before {
                            rt.fire(&n, "onselect", &[Value::String(ProjectTree::public(&after))]);
                        }
                    }
                    rt.invalidate();
                }),
            );
        }
        User::Move(path, new, index) => {
            with_mut(name, |m| m.apply_move(&path, &new, index));
            rt.fire(name, "onmove", &[Value::String(path), Value::String(new), int(index as i64)]);
        }
    }
}
