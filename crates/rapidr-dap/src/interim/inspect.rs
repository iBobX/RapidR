//! Looking into the stopped program: frames, variables and the little the
//! interim program end can evaluate without the session evaluator (I0) —
//! a variable, an array element, a TYPE's field, a component's property.

use std::rc::Rc;

use rapidr_bytecode::{Function, Module};
use rapidr_value::{Instance, Value};
use rapidr_vm::{Host, Vm};

use crate::session_wire::Variable;

/// `variables { ref }` of the globals (the numbers rapidr-session uses).
pub const GLOBALS_REF: u32 = 1;
/// `variables { ref }` of frame `i`'s locals: this plus `i`.
pub const LOCALS_REF: u32 = 1_000;
/// Children of a value shown in a stop: this plus the value's index; valid
/// until the program goes on.
pub const CHILDREN_REF: u32 = 1_000_000;
/// Elements of an array shown when `variables` doesn't say how many.
pub const PAGE: u32 = 100;

/// What the interim end can't evaluate.
pub const NEEDS_EVALUATOR: &str = "needs the session evaluator (I0): only variables, array elements, fields and properties for now";

/// The line of `ip` in `f`, and whether `ip` is that statement's start
/// (not where a call returns into the middle of the line that made it).
pub fn line_at(f: &Function, ip: usize) -> Option<(u32, bool)> {
    let mut best = None;
    for &(off, line) in &f.line_info {
        if off as usize <= ip {
            best = Some((line, off as usize == ip));
        } else {
            break;
        }
    }
    best
}

/// Frame `index`'s file and line (counted from the outermost).
pub fn frame_location<H: Host + ?Sized>(vm: &Vm<'_, H>, module: &Module, index: usize) -> Option<(Option<String>, u32)> {
    let frame = vm.frames.get(index)?;
    // (a caller's ip is past the call — or the host operation that ran a
    // handler —: its line is the one before)
    let ip = if index + 1 < vm.frames.len() { frame.ip.saturating_sub(1) } else { frame.ip };
    let line = module.functions.get(frame.fn_index as usize)?.get_line_for_ip(ip)?;
    Some(match module.source_map.locate(line) {
        Some((file, l)) => (Some(file.to_string()), l),
        None => (None, line),
    })
}

/// A BASIC name as the compiler keys it: any case, no type suffix.
fn key(name: &str) -> String {
    name.trim().to_ascii_lowercase().trim_end_matches(['$', '%', '&', '!', '#']).to_string()
}

/// Compiler-made names (`__for_end1`, the result slot) aren't shown.
fn hidden(name: &str) -> bool {
    name.is_empty() || name.starts_with("__")
}

/// Values whose children were offered in this stop.
#[derive(Default)]
pub struct Children(Vec<Value>);

impl Children {
    pub fn clear(&mut self) {
        self.0.clear();
    }

    fn add(&mut self, v: &Value) -> u32 {
        self.0.push(v.clone());
        CHILDREN_REF + (self.0.len() - 1) as u32
    }

    /// A value as the debugger shows it, its kind, and a reference to its
    /// children (0: none).
    pub fn render(&mut self, v: &Value) -> (String, String, u32) {
        match v {
            Value::Null => ("Empty".into(), "Empty".into(), 0),
            Value::Integer(_) => (rapidr_value::format::print_text(v), "Integer".into(), 0),
            Value::Double(_) => (rapidr_value::format::print_text(v), "Double".into(), 0),
            Value::Boolean(b) => ((if *b { "True" } else { "False" }).into(), "Boolean".into(), 0),
            Value::String(s) => (quoted(s), "String".into(), 0),
            Value::Array(a) => {
                let bounds = a.borrow().bounds.iter().map(|(lo, hi)| format!("{lo} TO {hi}")).collect::<Vec<_>>().join(", ");
                (format!("Array({bounds})"), "Array".into(), self.add(v))
            }
            Value::Object(o) => (o.type_name.clone(), o.type_name.clone(), self.add(v)),
        }
    }

    /// The variables of a scope or of a value's children.
    pub fn variables<H: Host + ?Sized>(&mut self, vm: &Vm<'_, H>, module: &Module, reference: u32, start: u32, count: u32) -> Result<Vec<Variable>, String> {
        let mut named: Vec<(String, Value)> = Vec::new();
        if reference == GLOBALS_REF {
            for (name, v) in vm.global_values(module) {
                if !hidden(name) {
                    named.push((name.to_string(), v.clone()));
                }
            }
            named.sort_by_key(|(n, _)| n.to_ascii_lowercase());
        } else if (LOCALS_REF..CHILDREN_REF).contains(&reference) {
            let i = (reference - LOCALS_REF) as usize;
            let frame = vm.frames.get(i).ok_or_else(|| format!("no frame {i}"))?;
            let names = module.functions.get(frame.fn_index as usize).map(|f| &f.local_names[..]).unwrap_or(&[]);
            for (slot, v) in frame.locals.iter().enumerate() {
                match names.get(slot) {
                    Some(name) if !hidden(name) => named.push((name.clone(), v.clone())),
                    _ => {}
                }
            }
        } else if reference >= CHILDREN_REF {
            let value = self.0.get((reference - CHILDREN_REF) as usize).cloned().ok_or("that value is gone (the program went on)")?;
            match &value {
                Value::Array(a) => {
                    let a = a.borrow();
                    let end = start.saturating_add(count).min(a.data.len() as u32);
                    for k in start..end {
                        named.push((index_label(&a.bounds, k as usize), a.data[k as usize].clone()));
                    }
                }
                Value::Object(o) => {
                    for (name, f) in o.names.iter().zip(o.fields.borrow().iter()) {
                        named.push((name.clone(), f.clone()));
                    }
                }
                _ => {}
            }
        } else {
            return Err(format!("no variables {reference}"));
        }
        Ok(named
            .into_iter()
            .map(|(name, v)| {
                let (value, kind, reference) = self.render(&v);
                let count = match &v {
                    Value::Array(a) => a.borrow().data.len() as u32,
                    Value::Object(o) => o.names.len() as u32,
                    _ => 0,
                };
                Variable { name, value, kind, reference, count }
            })
            .collect())
    }
}

/// `"text"` with BASIC's view of a quote (`""`) and control characters
/// escaped, as a debugger shows a string.
fn quoted(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\"\""),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\x{:02X}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

/// `(2, 3)`: element `k`'s subscripts (stored by the last one first).
fn index_label(bounds: &[(i64, i64)], mut k: usize) -> String {
    let mut parts = vec![0i64; bounds.len()];
    for (d, &(lo, hi)) in bounds.iter().enumerate().rev() {
        let extent = (hi - lo + 1).max(1) as usize;
        parts[d] = lo + (k % extent) as i64;
        k /= extent;
    }
    format!("({})", parts.iter().map(|p| p.to_string()).collect::<Vec<_>>().join(", "))
}

/// Where a value lives, to read it or write it.
enum Place {
    Local(usize, usize),
    Global(usize),
    Element(Value, Vec<i64>),
    Field(Rc<Instance>, usize),
    Property(String, String),
}

/// One step of a path: a name and its subscripts, if any.
struct Part {
    name: String,
    index: Option<Vec<String>>,
}

/// `a`, `a(1, i)`, `p.Name`, `Form.Caption`, `list(2).Items` …: the
/// parts, or `None` for anything else (an expression).
fn parse_path(text: &str) -> Option<Vec<Part>> {
    let chars: Vec<char> = text.trim().chars().collect();
    let mut i = 0;
    let mut parts = Vec::new();
    loop {
        let begin = i;
        if i < chars.len() && (chars[i].is_alphabetic() || chars[i] == '_') {
            i += 1;
            while i < chars.len() && (chars[i].is_alphanumeric() || chars[i] == '_') {
                i += 1;
            }
            while i < chars.len() && matches!(chars[i], '$' | '%' | '&' | '!' | '#') {
                i += 1;
            }
        }
        if i == begin {
            return None;
        }
        let name: String = chars[begin..i].iter().collect();
        while i < chars.len() && chars[i] == ' ' {
            i += 1;
        }
        let mut index = None;
        if i < chars.len() && chars[i] == '(' {
            let close = chars[i..].iter().position(|&c| c == ')')? + i;
            let inner: String = chars[i + 1..close].iter().collect();
            index = Some(inner.split(',').map(|s| s.trim().to_string()).collect());
            i = close + 1;
        }
        parts.push(Part { name, index });
        while i < chars.len() && chars[i] == ' ' {
            i += 1;
        }
        if i == chars.len() {
            return Some(parts);
        }
        if chars[i] != '.' {
            return None;
        }
        i += 1;
    }
}

/// A literal: a number, a quoted string, True / False.
fn literal(text: &str) -> Option<Value> {
    let t = text.trim();
    if let Some(inner) = t.strip_prefix('"').and_then(|s| s.strip_suffix('"')) {
        return Some(Value::String(inner.replace("\"\"", "\"")));
    }
    if t.eq_ignore_ascii_case("true") {
        return Some(Value::Boolean(true));
    }
    if t.eq_ignore_ascii_case("false") {
        return Some(Value::Boolean(false));
    }
    if let Ok(n) = t.parse::<i64>() {
        return Some(Value::Integer(n));
    }
    t.parse::<f64>().ok().filter(|d| d.is_finite()).map(Value::Double)
}

fn find_local<H: Host + ?Sized>(vm: &Vm<'_, H>, module: &Module, frame: Option<usize>, name: &str) -> Option<Place> {
    let f = frame?;
    let fr = vm.frames.get(f)?;
    let func = module.functions.get(fr.fn_index as usize)?;
    let k = key(name);
    let slot = func.local_names.iter().position(|n| !hidden(n) && key(n) == k)?;
    (slot < fr.locals.len()).then_some(Place::Local(f, slot))
}

fn find_global<H: Host + ?Sized>(vm: &Vm<'_, H>, module: &Module, name: &str) -> Option<Place> {
    let k = key(name);
    vm.globals
        .iter()
        .enumerate()
        .find(|(i, v)| v.is_some() && module.strings.get(*i).is_some_and(|n| !hidden(n) && key(n) == k))
        .map(|(i, _)| Place::Global(i))
}

fn read<H: Host + ?Sized>(vm: &mut Vm<'_, H>, place: &Place) -> Result<Value, String> {
    match place {
        Place::Local(f, slot) => Ok(vm.frames[*f].locals[*slot].clone()),
        Place::Global(i) => Ok(vm.globals[*i].clone().unwrap_or(Value::Null)),
        Place::Element(Value::Array(a), idx) => a.borrow().get(idx),
        Place::Element(..) => Err("not an array".into()),
        Place::Field(o, slot) => Ok(o.get(*slot)),
        Place::Property(id, prop) => vm.host.get_prop(id, prop),
    }
}

fn write<H: Host + ?Sized>(vm: &mut Vm<'_, H>, place: &Place, value: Value) -> Result<(), String> {
    match place {
        Place::Local(f, slot) => vm.frames[*f].locals[*slot] = value,
        Place::Global(i) => vm.globals[*i] = Some(value),
        Place::Element(Value::Array(a), idx) => a.borrow_mut().set(idx, value)?,
        Place::Element(..) => return Err("not an array".into()),
        Place::Field(o, slot) => o.set(*slot, value),
        Place::Property(id, prop) => vm.host.set_prop(id, prop, value)?,
    }
    Ok(())
}

/// A subscript: a whole number or a variable holding one.
fn subscript<H: Host + ?Sized>(vm: &mut Vm<'_, H>, module: &Module, frame: Option<usize>, text: &str) -> Result<i64, String> {
    if let Ok(n) = text.parse::<i64>() {
        return Ok(n);
    }
    match parse_path(text) {
        Some(parts) => {
            let place = resolve(vm, module, frame, &parts)?;
            Ok(read(vm, &place)?.to_i64())
        }
        None => Err(NEEDS_EVALUATOR.into()),
    }
}

fn resolve<H: Host + ?Sized>(vm: &mut Vm<'_, H>, module: &Module, frame: Option<usize>, parts: &[Part]) -> Result<Place, String> {
    let head = &parts[0];
    let mut place = match find_local(vm, module, frame, &head.name).or_else(|| find_global(vm, module, &head.name)) {
        Some(p) => p,
        // (a component by its name: `Form.Caption`)
        None if parts.len() > 1 && head.index.is_none() && parts[1..].iter().all(|p| p.index.is_none()) => {
            let prop = parts[1..].iter().map(|p| p.name.as_str()).collect::<Vec<_>>().join(".");
            return Ok(Place::Property(head.name.clone(), prop));
        }
        None => return Err(format!("{}: no such variable here", head.name)),
    };
    let mut first = true;
    for part in parts {
        if !first {
            let value = read(vm, &place)?;
            place = match &value {
                Value::Object(o) => match o.names.iter().position(|n| n.eq_ignore_ascii_case(&part.name)) {
                    Some(slot) => Place::Field(o.clone(), slot),
                    // (a TYPE EXTENDS a component: the component's)
                    None => Place::Property(o.id.clone(), part.name.clone()),
                },
                Value::String(id) if !id.is_empty() => Place::Property(id.clone(), part.name.clone()),
                _ => return Err(format!("{}: not an object", part.name)),
            };
        }
        first = false;
        if let Some(index) = &part.index {
            let value = read(vm, &place)?;
            if !matches!(value, Value::Array(_)) {
                return Err(format!("{}: not an array", part.name));
            }
            let mut idx = Vec::with_capacity(index.len());
            for s in index {
                idx.push(subscript(vm, module, frame, s)?);
            }
            place = Place::Element(value, idx);
        }
    }
    Ok(place)
}

/// The value of `expr` in `frame` (a path or a literal).
pub fn evaluate<H: Host + ?Sized>(vm: &mut Vm<'_, H>, module: &Module, frame: Option<usize>, expr: &str) -> Result<Value, String> {
    let text = expr.trim();
    if let Some(v) = literal(text) {
        return Ok(v);
    }
    let parts = parse_path(text).ok_or(NEEDS_EVALUATOR)?;
    let place = resolve(vm, module, frame, &parts)?;
    read(vm, &place)
}

/// Sets what `target` names in `frame` to `value` (a literal or a path);
/// returns the new value.
pub fn assign<H: Host + ?Sized>(vm: &mut Vm<'_, H>, module: &Module, frame: Option<usize>, target: &str, value: &str) -> Result<Value, String> {
    let parts = parse_path(target).ok_or(NEEDS_EVALUATOR)?;
    let place = resolve(vm, module, frame, &parts)?;
    let v = evaluate(vm, module, frame, value)?;
    write(vm, &place, v)?;
    read(vm, &place)
}

/// `name = value` at the debug console: the target and the value.
pub fn split_assignment(text: &str) -> Option<(&str, &str)> {
    let (target, value) = text.split_once('=')?;
    parse_path(target)?;
    Some((target.trim(), value.trim()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn paths_and_literals_parse() {
        let p = parse_path("list(2, i).Items").unwrap();
        assert_eq!(p.len(), 2);
        assert_eq!(p[0].index.as_deref(), Some(&["2".to_string(), "i".to_string()][..]));
        assert_eq!(p[1].name, "Items");
        assert!(parse_path("a$").is_some());
        assert!(parse_path("a + 1").is_none());
        assert!(parse_path("1a").is_none());
        assert!(matches!(literal("\"a\"\"b\""), Some(Value::String(s)) if s == "a\"b"));
        assert!(matches!(literal("-3"), Some(Value::Integer(-3))));
        assert!(matches!(literal("2.5"), Some(Value::Double(_))));
        assert!(literal("x").is_none());
        assert_eq!(split_assignment("p.Name = \"x\""), Some(("p.Name", "\"x\"")));
        assert_eq!(index_label(&[(1, 2), (0, 2)], 4), "(2, 1)");
    }
}
