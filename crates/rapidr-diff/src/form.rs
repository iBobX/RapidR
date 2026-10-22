//! Forms compared as components and properties (docs/versions-plan.md §2,
//! "diffs that understand RapidR"): what a person changed in the designer,
//! in their words — "GreetButton: Caption "&Greet" → "&Say hello"; Left
//! 112 → 120", "added Label3 (RLabel)".
//!
//! A form is its `CREATE … END CREATE` blocks; the caller reads them (the
//! designer's model: `rapidr_value::designer`) into [`Component`]s:
//! - components are matched by **name**, ignoring case as BASIC does;
//! - **properties** by name, ignoring case; a property set twice counts
//!   with its last value (the one the program ends up with); values are
//!   compared as written (`"OK"`, `10`, `&HFF0000`), spaces around them
//!   ignored;
//! - other lines of a block (`Center`, `AddItems "a", "b"`) are compared
//!   as lines: added or removed;
//! - the order is the new form's; removed components come last, in the
//!   old form's order.

/// One component of a form as its CREATE block says it.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Component {
    pub name: String,
    /// Its type as written (`RButton`, `QBUTTON` …).
    pub type_name: String,
    /// The component it is inside (`None`: the form itself).
    pub parent: Option<String>,
    /// `Name = value` lines, in order, the values as written.
    pub props: Vec<(String, String)>,
    /// The block's other lines (method calls).
    pub code: Vec<String>,
}

/// What happened to a component.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Added,
    Removed,
    Changed,
}

/// A property's old and new value (`None`: not set on that side).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PropChange {
    pub name: String,
    pub old: Option<String>,
    pub new: Option<String>,
}

/// One component that differs.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ComponentChange {
    pub name: String,
    pub type_name: String,
    pub kind: Kind,
    /// Its type before, when it changed.
    pub old_type: Option<String>,
    /// Its parent before and after, when it moved into another one.
    pub parent: Option<(Option<String>, Option<String>)>,
    pub props: Vec<PropChange>,
    pub lines_removed: Vec<String>,
    pub lines_added: Vec<String>,
}

impl ComponentChange {
    /// Whether property `name` (any case) changed.
    pub fn changes(&self, name: &str) -> bool {
        self.props.iter().any(|p| p.name.eq_ignore_ascii_case(name))
    }

    /// Whether its place or size changed (Left, Top, Width, Height).
    pub fn moved_or_sized(&self) -> bool {
        ["left", "top", "width", "height"].iter().any(|p| self.changes(p))
    }

    /// The change in plain words, one line.
    pub fn summary(&self) -> String {
        match self.kind {
            Kind::Added => format!("Added {} ({})", self.name, self.type_name),
            Kind::Removed => format!("Removed {} ({})", self.name, self.type_name),
            Kind::Changed => {
                let mut what: Vec<String> = Vec::new();
                if let Some(t) = &self.old_type {
                    what.push(format!("type {t} \u{2192} {}", self.type_name));
                }
                if let Some((from, to)) = &self.parent {
                    let n = |p: &Option<String>| p.clone().unwrap_or_else(|| "the form".into());
                    what.push(format!("moved from {} into {}", n(from), n(to)));
                }
                for p in &self.props {
                    what.push(match (&p.old, &p.new) {
                        (Some(o), Some(n)) => format!("{} {o} \u{2192} {n}", p.name),
                        (None, Some(n)) => format!("{} = {n} (new)", p.name),
                        (Some(o), None) => format!("{} removed (was {o})", p.name),
                        (None, None) => p.name.clone(),
                    });
                }
                what.extend(self.lines_added.iter().map(|l| format!("added line {l}")));
                what.extend(self.lines_removed.iter().map(|l| format!("removed line {l}")));
                format!("{}: {}", self.name, what.join("; "))
            }
        }
    }
}

/// The last value of each property (by name, any case), in first-set
/// order, with the name as last written.
fn props_of(c: &Component) -> Vec<(String, String)> {
    let mut out: Vec<(String, String)> = Vec::new();
    for (name, value) in &c.props {
        let value = value.trim().to_string();
        match out.iter_mut().find(|(n, _)| n.eq_ignore_ascii_case(name)) {
            Some(slot) => *slot = (name.clone(), value),
            None => out.push((name.clone(), value)),
        }
    }
    out
}

fn same_name(a: &Option<String>, b: &Option<String>) -> bool {
    match (a, b) {
        (Some(a), Some(b)) => a.eq_ignore_ascii_case(b),
        (None, None) => true,
        _ => false,
    }
}

/// The components of `old` and `new` that differ (see the module's notes).
pub fn diff(old: &[Component], new: &[Component]) -> Vec<ComponentChange> {
    let find = |list: &[Component], name: &str| list.iter().position(|c| c.name.eq_ignore_ascii_case(name));
    let mut out = Vec::new();
    for n in new {
        let Some(o) = find(old, &n.name).map(|i| &old[i]) else {
            out.push(ComponentChange { name: n.name.clone(), type_name: n.type_name.clone(), kind: Kind::Added, old_type: None, parent: None, props: Vec::new(), lines_removed: Vec::new(), lines_added: Vec::new() });
            continue;
        };
        let (po, pn) = (props_of(o), props_of(n));
        let mut props = Vec::new();
        for (name, value) in &pn {
            match po.iter().find(|(k, _)| k.eq_ignore_ascii_case(name)) {
                Some((_, was)) if was == value => {}
                Some((_, was)) => props.push(PropChange { name: name.clone(), old: Some(was.clone()), new: Some(value.clone()) }),
                None => props.push(PropChange { name: name.clone(), old: None, new: Some(value.clone()) }),
            }
        }
        for (name, was) in &po {
            if !pn.iter().any(|(k, _)| k.eq_ignore_ascii_case(name)) {
                props.push(PropChange { name: name.clone(), old: Some(was.clone()), new: None });
            }
        }
        let trimmed = |v: &[String]| v.iter().map(|l| l.trim().to_string()).collect::<Vec<_>>();
        let (lo, ln) = (trimmed(&o.code), trimmed(&n.code));
        let lines_added: Vec<String> = ln.iter().filter(|l| !lo.contains(l)).cloned().collect();
        let lines_removed: Vec<String> = lo.iter().filter(|l| !ln.contains(l)).cloned().collect();
        let old_type = (!o.type_name.eq_ignore_ascii_case(&n.type_name)).then(|| o.type_name.clone());
        let parent = (!same_name(&o.parent, &n.parent)).then(|| (o.parent.clone(), n.parent.clone()));
        if props.is_empty() && lines_added.is_empty() && lines_removed.is_empty() && old_type.is_none() && parent.is_none() {
            continue;
        }
        out.push(ComponentChange { name: n.name.clone(), type_name: n.type_name.clone(), kind: Kind::Changed, old_type, parent, props, lines_removed, lines_added });
    }
    for o in old {
        if find(new, &o.name).is_none() {
            out.push(ComponentChange { name: o.name.clone(), type_name: o.type_name.clone(), kind: Kind::Removed, old_type: None, parent: None, props: Vec::new(), lines_removed: Vec::new(), lines_added: Vec::new() });
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn comp(name: &str, ty: &str, parent: Option<&str>, props: &[(&str, &str)]) -> Component {
        Component { name: name.into(), type_name: ty.into(), parent: parent.map(Into::into), props: props.iter().map(|(a, b)| (a.to_string(), b.to_string())).collect(), code: Vec::new() }
    }

    #[test]
    fn components_and_properties() {
        let old = vec![
            comp("Form", "RForm", None, &[("Caption", "\"Hello\""), ("Width", "340")]),
            comp("Button1", "RButton", Some("Form"), &[("Caption", "\"OK\""), ("Left", "10"), ("Default", "1")]),
            comp("Old", "RLabel", Some("Form"), &[]),
        ];
        let new = vec![
            comp("Form", "RForm", None, &[("Caption", "\"Hello\""), ("width", "340")]),
            comp("button1", "RButton", Some("form"), &[("Caption", "\"Save\""), ("Left", "20"), ("Hint", "\"Saves\"")]),
            comp("Label3", "RLabel", Some("Form"), &[("Caption", "\"x\"")]),
        ];
        let d = diff(&old, &new);
        let lines: Vec<String> = d.iter().map(ComponentChange::summary).collect();
        assert_eq!(
            lines,
            [
                "button1: Caption \"OK\" \u{2192} \"Save\"; Left 10 \u{2192} 20; Hint = \"Saves\" (new); Default removed (was 1)",
                "Added Label3 (RLabel)",
                "Removed Old (RLabel)"
            ]
        );
        assert!(d[0].moved_or_sized() && d[0].changes("caption") && !d[0].changes("width"));
        assert_eq!(d[1].kind, Kind::Added);
        assert!(diff(&old, &old).is_empty());
    }

    #[test]
    fn last_value_types_parents_and_lines() {
        let mut a = comp("P", "RPanel", Some("Form"), &[("Left", "1"), ("Left", "5")]);
        a.code = vec!["Center".into()];
        let mut b = comp("P", "RGroupBox", Some("Other"), &[("Left", " 5 ")]);
        b.code = vec!["AddItems \"a\"".into()];
        let d = diff(&[a], &[b]);
        assert_eq!(d[0].summary(), "P: type RPanel \u{2192} RGroupBox; moved from Form into Other; added line AddItems \"a\"; removed line Center");
    }
}
