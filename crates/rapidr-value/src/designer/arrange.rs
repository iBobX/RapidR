//! Align, distribute, same size, centre in parent, nudge, z-order and Tab
//! order: each gives the command that does it (one undo step), writing only
//! the properties that change.

use super::command::Command;
use super::layout::Layout;
use super::model::{FormDesign, Item, NodeId};
use crate::layout::Rect;

/// How to line components up (`Align(How)`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AlignHow {
    Left,
    Center,
    Right,
    Top,
    Middle,
    Bottom,
}

impl AlignHow {
    /// RFormDesigner's names: "left", "center", "right", "top", "middle",
    /// "bottom".
    pub fn parse(s: &str) -> Option<AlignHow> {
        Some(match s.trim().to_ascii_lowercase().as_str() {
            "left" => AlignHow::Left,
            "center" | "centre" => AlignHow::Center,
            "right" => AlignHow::Right,
            "top" => AlignHow::Top,
            "middle" => AlignHow::Middle,
            "bottom" => AlignHow::Bottom,
            _ => return None,
        })
    }
}

/// Horizontally or vertically.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Axis {
    Horizontal,
    Vertical,
    Both,
}

impl Axis {
    pub fn parse(s: &str) -> Option<Axis> {
        Some(match s.trim().to_ascii_lowercase().as_str() {
            "horizontal" | "horizontally" | "width" | "h" => Axis::Horizontal,
            "vertical" | "vertically" | "height" | "v" => Axis::Vertical,
            "both" | "" => Axis::Both,
            _ => return None,
        })
    }
}

/// The commands setting a component's Left / Top / Width / Height from
/// `old` to `new` (only those that change).
pub fn set_rect(node: NodeId, old: Rect, new: Rect) -> Vec<Command> {
    let mut out = Vec::new();
    for (name, a, b) in [("Left", old.left, new.left), ("Top", old.top, new.top), ("Width", old.width, new.width), ("Height", old.height, new.height)] {
        if a != b {
            out.push(Command::SetProp { node, name: name.into(), value: Some(b.to_string()) });
        }
    }
    out
}

/// The components' rectangles in the form's client coordinates.
fn absolute(d: &FormDesign, l: &Layout, sel: &[NodeId]) -> Vec<(NodeId, Rect, (i64, i64))> {
    sel.iter()
        .filter(|&&n| n != d.root())
        .filter_map(|&n| {
            let r = l.rect(n)?;
            let o = l.origin(d, n);
            Some((n, Rect::new(r.left + o.0, r.top + o.1, r.width, r.height), o))
        })
        .collect()
}

fn batch(d: &FormDesign, l: &Layout, moves: Vec<(NodeId, Rect)>) -> Command {
    let mut cmds = Vec::new();
    for (n, abs) in moves {
        let (Some(old), o) = (l.rect(n), l.origin(d, n)) else { continue };
        cmds.extend(set_rect(n, old, Rect::new(abs.left - o.0, abs.top - o.1, abs.width, abs.height)));
    }
    Command::Batch(cmds)
}

/// Lines the selection up with its first component (`sel[0]`, the
/// primary).
pub fn align(d: &FormDesign, l: &Layout, sel: &[NodeId], how: AlignHow) -> Command {
    let comps = absolute(d, l, sel);
    let Some(&(_, r0, _)) = comps.first() else { return Command::Batch(Vec::new()) };
    let moves = comps
        .iter()
        .skip(1)
        .map(|&(n, r, _)| {
            let mut r2 = r;
            match how {
                AlignHow::Left => r2.left = r0.left,
                AlignHow::Center => r2.left = r0.left + r0.width / 2 - r.width / 2,
                AlignHow::Right => r2.left = r0.left + r0.width - r.width,
                AlignHow::Top => r2.top = r0.top,
                AlignHow::Middle => r2.top = r0.top + r0.height / 2 - r.height / 2,
                AlignHow::Bottom => r2.top = r0.top + r0.height - r.height,
            }
            (n, r2)
        })
        .collect();
    batch(d, l, moves)
}

/// Spaces three or more components evenly between the first and the last
/// (by position).
pub fn distribute(d: &FormDesign, l: &Layout, sel: &[NodeId], axis: Axis) -> Command {
    let mut comps = absolute(d, l, sel);
    if comps.len() < 3 {
        return Command::Batch(Vec::new());
    }
    let horizontal = axis != Axis::Vertical;
    let start = |r: &Rect| if horizontal { r.left } else { r.top };
    let size = |r: &Rect| if horizontal { r.width } else { r.height };
    comps.sort_by_key(|(_, r, _)| start(r));
    let first = comps[0].1;
    let last = comps[comps.len() - 1].1;
    let total = start(&last) + size(&last) - start(&first);
    let sizes: i64 = comps.iter().map(|(_, r, _)| size(r)).sum();
    let gaps = (comps.len() - 1) as i64;
    let free = total - sizes;
    let mut at = start(&first);
    let mut moves = Vec::new();
    for (k, (n, r, _)) in comps.iter().enumerate() {
        let mut r2 = *r;
        if k > 0 && k < comps.len() - 1 {
            // (the gaps' rounding spread as RapidR's integer pixels allow)
            let pos = start(&first) + (0..k).map(|j| size(&comps[j].1)).sum::<i64>() + free * k as i64 / gaps;
            if horizontal { r2.left = pos } else { r2.top = pos }
        }
        at += size(r);
        moves.push((*n, r2));
    }
    let _ = at;
    batch(d, l, moves)
}

/// Gives the selection the first component's width, height or both.
pub fn same_size(d: &FormDesign, l: &Layout, sel: &[NodeId], axis: Axis) -> Command {
    let comps = absolute(d, l, sel);
    let Some(&(_, r0, _)) = comps.first() else { return Command::Batch(Vec::new()) };
    let moves = comps
        .iter()
        .skip(1)
        .map(|&(n, r, _)| {
            let mut r2 = r;
            if axis != Axis::Vertical {
                r2.width = r0.width;
            }
            if axis != Axis::Horizontal {
                r2.height = r0.height;
            }
            (n, r2)
        })
        .collect();
    batch(d, l, moves)
}

/// Centres each component in its parent's client area.
pub fn center_in_parent(d: &FormDesign, l: &Layout, sel: &[NodeId], axis: Axis) -> Command {
    let mut cmds = Vec::new();
    for &n in sel.iter().filter(|&&n| n != d.root()) {
        let (Some(r), Some(p)) = (l.rect(n), d.parent(n)) else { continue };
        let Some(c) = l.client_of(p) else { continue };
        let mut r2 = r;
        if axis != Axis::Vertical {
            r2.left = (c.width - r.width) / 2;
        }
        if axis != Axis::Horizontal {
            r2.top = (c.height - r.height) / 2;
        }
        cmds.extend(set_rect(n, r, r2));
    }
    Command::Batch(cmds)
}

/// Moves (or, `resize`, sizes) the selection by (dx, dy): the arrow keys.
pub fn nudge(d: &FormDesign, l: &Layout, sel: &[NodeId], dx: i64, dy: i64, resize: bool) -> Command {
    let mut cmds = Vec::new();
    for &n in sel.iter().filter(|&&n| n != d.root()) {
        let Some(r) = l.rect(n) else { continue };
        let r2 = if resize { Rect::new(r.left, r.top, (r.width + dx).max(1), (r.height + dy).max(1)) } else { Rect::new(r.left + dx, r.top + dy, r.width, r.height) };
        cmds.extend(set_rect(n, r, r2));
    }
    Command::Batch(cmds)
}

/// BringToFront: the component's CREATE moves after its last sibling's
/// (created last, it's drawn on top).
pub fn bring_to_front(d: &FormDesign, node: NodeId) -> Option<Command> {
    let parent = d.parent(node)?;
    let body: Vec<&Item> = d.node(parent)?.body.iter().filter(|i| **i != Item::Child(node)).collect();
    let index = body.iter().rposition(|i| matches!(i, Item::Child(_))).map_or(0, |i| i + 1);
    let now = d.node(parent)?.body.iter().position(|i| *i == Item::Child(node))?;
    (now != index).then_some(Command::Move { node, parent, index })
}

/// SendToBack: before its first sibling's CREATE (created first, under the
/// others).
pub fn send_to_back(d: &FormDesign, node: NodeId) -> Option<Command> {
    let parent = d.parent(node)?;
    let body: Vec<&Item> = d.node(parent)?.body.iter().filter(|i| **i != Item::Child(node)).collect();
    let first = body.iter().position(|i| matches!(i, Item::Child(_)));
    let now = d.node(parent)?.body.iter().position(|i| *i == Item::Child(node))?;
    // (no sibling: it stays where it is)
    let index = first?;
    (now != index).then_some(Command::Move { node, parent, index })
}

/// A container's children in Tab order: by TabOrder where set, else
/// creation order (the runtimes' rule: `objects::a11y::tab_walk`).
pub fn tab_order(d: &FormDesign, parent: NodeId) -> Vec<NodeId> {
    let mut keyed: Vec<(i64, NodeId)> = d.children(parent).into_iter().enumerate().map(|(k, c)| (d.node(c).and_then(|n| n.int("TabOrder")).unwrap_or(k as i64), c)).collect();
    keyed.sort_by_key(|(k, _)| *k);
    keyed.into_iter().map(|(_, c)| c).collect()
}

/// Puts a container's children in this Tab order: `TabOrder = i` written
/// for each one whose place isn't already `i`.
pub fn set_tab_order(d: &FormDesign, parent: NodeId, order: &[NodeId]) -> Command {
    let children = d.children(parent);
    let mut cmds = Vec::new();
    for (i, &c) in order.iter().enumerate() {
        let Some(k) = children.iter().position(|&x| x == c) else { continue };
        let key = d.node(c).and_then(|n| n.int("TabOrder")).unwrap_or(k as i64);
        if key != i as i64 {
            cmds.push(Command::SetProp { node: c, name: "TabOrder".into(), value: Some(i.to_string()) });
        }
    }
    Command::Batch(cmds)
}

#[cfg(test)]
mod tests {
    use super::super::model::{Prop, SubItem, Subtree};
    use super::*;

    fn comp(name: &str, ty: &str, props: &[(&str, &str)]) -> Subtree {
        Subtree { id: 0, name: name.into(), type_written: ty.into(), body: props.iter().map(|(n, v)| SubItem::Prop(Prop { name: n.to_string(), value: v.to_string() })).collect() }
    }

    fn form() -> FormDesign {
        let mut f = comp("Form", "QFORM", &[("Width", "400"), ("Height", "300")]);
        for (n, l, t, w) in [("A", "10", "10", "50"), ("B", "100", "40", "80"), ("C", "300", "70", "60")] {
            f.body.push(SubItem::Child(comp(n, "QBUTTON", &[("Left", l), ("Top", t), ("Width", w), ("Height", "25")])));
        }
        FormDesign::from_subtree(f)
    }

    fn run(d: &mut FormDesign, c: Command) {
        c.apply(d).unwrap();
    }

    #[test]
    fn align_distribute_and_size() {
        let mut d = form();
        let ids = [d.find("A").unwrap(), d.find("B").unwrap(), d.find("C").unwrap()];
        let l = Layout::of(&d);
        let c = align(&d, &l, &ids, AlignHow::Top);
        assert_eq!(c.flatten().len(), 2, "only B's and C's Top");
        run(&mut d, c);
        assert_eq!(d.node(ids[2]).unwrap().prop("Top"), Some("10"));
        let l = Layout::of(&d);
        // distribute: A 10..60, C 300..360 → B (80 wide) at 10+50+ (300-10-50-80-60... )
        let c = distribute(&d, &l, &ids, Axis::Horizontal);
        run(&mut d, c);
        // total 350, sizes 190, free 160 → gaps 80: B at 60 + 80 = 140
        assert_eq!(d.node(ids[1]).unwrap().int("Left"), Some(140));
        let l = Layout::of(&d);
        let c = same_size(&d, &l, &ids, Axis::Horizontal);
        run(&mut d, c);
        assert_eq!(d.node(ids[2]).unwrap().int("Width"), Some(50));
        let l = Layout::of(&d);
        let c = center_in_parent(&d, &l, &ids[..1], Axis::Horizontal);
        run(&mut d, c);
        assert_eq!(d.node(ids[0]).unwrap().int("Left"), Some((398 - 50) / 2));
        let l = Layout::of(&d);
        let c = nudge(&d, &l, &ids[..1], 1, 0, false);
        run(&mut d, c);
        assert_eq!(d.node(ids[0]).unwrap().int("Left"), Some(175));
    }

    #[test]
    fn z_order_moves_create_blocks() {
        let mut d = form();
        let a = d.find("A").unwrap();
        let c = bring_to_front(&d, a).unwrap();
        run(&mut d, c);
        assert_eq!(d.children(d.root()), vec![d.find("B").unwrap(), d.find("C").unwrap(), a]);
        assert!(bring_to_front(&d, a).is_none(), "already on top");
        let c = send_to_back(&d, a).unwrap();
        run(&mut d, c);
        assert_eq!(d.children(d.root())[0], a);
    }

    #[test]
    fn tab_order_writes_only_what_changes() {
        let mut d = form();
        let (a, b, c) = (d.find("A").unwrap(), d.find("B").unwrap(), d.find("C").unwrap());
        assert_eq!(tab_order(&d, d.root()), vec![a, b, c]);
        let cmd = set_tab_order(&d, d.root(), &[c, a, b]);
        assert_eq!(cmd.flatten().len(), 3);
        run(&mut d, cmd);
        assert_eq!(tab_order(&d, d.root()), vec![c, a, b]);
        assert!(set_tab_order(&d, d.root(), &[c, a, b]).is_empty());
    }
}
