//! Each parent's children, for a runtime's component registry (the
//! desktop's `object.rs`, the web's `object_web.rs`): one index of every
//! parent's children in creation order, made again only when the tree of
//! components changed — a component made, a Parent stored
//! ([`ChildrenIndex::changed`]). Each call of a runtime's `get_children_of`
//! scanned and sorted the whole registry, and the window loop asks for every
//! component's children at every turn (docs/speed-audit-2026-11.md, hot
//! spot 3); the revision also tells what's built from the tree (the
//! owner-draw turn's components, `rapidr_ui_app::lists`) when to build again.
//!
//! The registry stays the runtime's: when the index must be made again it
//! lists its components (key, type, creation order, Parent), and its own
//! case rule for names says which parent a Parent's text names (`fold`: the
//! desktop's keys are lower case and a Parent names its parent in any ASCII
//! case; the web's are upper case, compared as upper case).

use std::collections::HashMap;

use crate::Value;

/// A registry's children index and tree revision.
pub struct ChildrenIndex {
    /// Moves whenever the tree changes.
    revision: u64,
    /// The revision the index was made for (`None`: never made).
    built: Option<u64>,
    /// A parent's name as the index keys it (the runtime's case rule).
    fold: fn(&str) -> String,
    /// Each parent (its name folded) → its children (creation order, key,
    /// type), in creation order.
    by_parent: HashMap<String, Vec<(u32, String, String)>>,
}

impl ChildrenIndex {
    /// An index whose parents' names are compared as `fold` gives them.
    pub fn new(fold: fn(&str) -> String) -> ChildrenIndex {
        ChildrenIndex { revision: 0, built: None, fold, by_parent: HashMap::new() }
    }

    /// The tree changed: a component was made, or a Parent stored.
    pub fn changed(&mut self) {
        self.revision = self.revision.wrapping_add(1);
    }

    /// A number that changes whenever the tree does: what's built from the
    /// tree is good while it stays.
    pub fn revision(&self) -> u64 {
        self.revision
    }

    /// `parent`'s children (key, type) in creation order — not `parent`
    /// itself, should it name itself as its Parent. `scan` lists every
    /// component (key, type, creation order, its Parent if it has one)
    /// when the index has to be made again.
    pub fn children_of(&mut self, parent: &str, scan: impl FnOnce(&mut dyn FnMut(&str, &str, u32, Option<&Value>))) -> Vec<(String, String)> {
        if self.built != Some(self.revision) {
            let fold = self.fold;
            let mut by_parent: HashMap<String, Vec<(u32, String, String)>> = HashMap::new();
            scan(&mut |key, type_name, order, parent| {
                let Some(p) = parent else { return };
                let p = fold(&p.to_string_val());
                // (a component that names itself its parent is no child of its own)
                if fold(key) != p {
                    by_parent.entry(p).or_default().push((order, key.to_string(), type_name.to_string()));
                }
            });
            // (the order they were made in: a CREATE block's)
            for kids in by_parent.values_mut() {
                kids.sort_by_key(|c| c.0);
            }
            self.by_parent = by_parent;
            self.built = Some(self.revision);
        }
        self.by_parent.get(&(self.fold)(parent)).map_or_else(Vec::new, |kids| kids.iter().map(|(_, n, t)| (n.clone(), t.clone())).collect())
    }
}

#[cfg(test)]
mod tests {
    use super::ChildrenIndex;
    use crate::{v_int, v_str, Value};

    /// A registry: (key, type, creation order, Parent).
    type Reg = Vec<(String, String, u32, Option<Value>)>;

    /// The children as a runtime found them before the index: every
    /// component scanned, those whose Parent names `parent` (folded alike,
    /// not `parent` itself), sorted by creation order.
    fn scanned(reg: &Reg, parent: &str, fold: fn(&str) -> String) -> Vec<(String, String)> {
        let mut kids: Vec<_> = reg.iter().filter(|(k, _, _, p)| fold(k) != fold(parent) && p.as_ref().is_some_and(|p| fold(&p.to_string_val()) == fold(parent))).collect();
        kids.sort_by_key(|c| c.2);
        kids.into_iter().map(|(k, t, _, _)| (k.clone(), t.clone())).collect()
    }

    fn check(reg: &Reg, index: &mut ChildrenIndex, fold: fn(&str) -> String, parents: &[&str]) {
        for p in parents {
            let got = index.children_of(p, |add| {
                for (k, t, o, par) in reg {
                    add(k, t, *o, par.as_ref());
                }
            });
            assert_eq!(got, scanned(reg, p, fold), "children of {p:?} in {reg:?}");
        }
    }

    #[test]
    fn the_index_answers_as_a_scan_for_both_case_rules() {
        let lower: fn(&str) -> String = |s| s.to_ascii_lowercase();
        let upper: fn(&str) -> String = |s| s.to_uppercase();
        let mut seed = 0x2545_F491_4F6C_DD1Du64;
        let mut next = |n: u64| {
            seed ^= seed << 13;
            seed ^= seed >> 7;
            seed ^= seed << 17;
            seed % n
        };
        let names = ["Form1", "panel", "PANEL2", "Box", "b1", "B2", "lbl", "Größe", "ÑAME", "ñame"];
        for fold in [lower, upper] {
            for _ in 0..300 {
                let mut index = ChildrenIndex::new(fold);
                let mut reg: Reg = Vec::new();
                for step in 0..40u32 {
                    match next(4) {
                        // (a component made, maybe with a Parent)
                        0 | 1 => {
                            let key = fold(&format!("{}{}", names[next(names.len() as u64) as usize], step));
                            let parent = match next(5) {
                                0 => None,
                                1 => Some(v_int(next(3) as i64)),
                                _ => Some(v_str(names[next(names.len() as u64) as usize])),
                            };
                            reg.push((key, ["RBUTTON", "RPANEL", "RLABEL"][next(3) as usize].to_string(), step, parent));
                        }
                        // (a Parent stored: another parent, itself, a name in another case)
                        2 if !reg.is_empty() => {
                            let i = next(reg.len() as u64) as usize;
                            reg[i].3 = Some(match next(3) {
                                0 => v_str(&reg[i].0.clone()),
                                1 => v_str(&names[next(names.len() as u64) as usize].to_uppercase()),
                                _ => v_str(&names[next(names.len() as u64) as usize].to_lowercase()),
                            });
                        }
                        _ => {}
                    }
                    if next(3) == 0 {
                        index.changed();
                        check(&reg, &mut index, fold, &names);
                    }
                }
                index.changed();
                check(&reg, &mut index, fold, &names);
                // (the keys themselves as parents: a component's own children)
                let keys: Vec<String> = reg.iter().map(|r| r.0.clone()).collect();
                let keys: Vec<&str> = keys.iter().map(String::as_str).collect();
                check(&reg, &mut index, fold, &keys);
            }
        }
    }

    #[test]
    fn the_revision_moves_and_the_index_waits_for_it() {
        let mut index = ChildrenIndex::new(|s| s.to_ascii_lowercase());
        let r0 = index.revision();
        let one = |add: &mut dyn FnMut(&str, &str, u32, Option<&Value>)| add("a", "RBUTTON", 1, Some(&v_str("F")));
        assert_eq!(index.children_of("f", one), vec![("a".to_string(), "RBUTTON".to_string())]);
        // (not made again while the tree hasn't changed: no scan)
        assert_eq!(index.children_of("F", |_| panic!("scanned again")), vec![("a".to_string(), "RBUTTON".to_string())]);
        index.changed();
        assert_ne!(index.revision(), r0);
        assert!(index.children_of("f", |_| {}).is_empty());
    }
}
