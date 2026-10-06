//! RapidR Studio's visual designer model (docs/ide-plan.md I4, lane
//! L-DMODEL): GUI-free, the same on the desktop and the web (it builds for
//! wasm32 with the rest of this crate). RDESIGNSURFACE draws and drives it
//! (`objects::design`); the IDE's form designer, inspector and shell use
//! the [`Designer`] API.
//!
//! * [`model`] — the form as its CREATE blocks: a tree of components
//!   (parents, z-order = creation order, Tab order), each with its
//!   assignments in source order.
//! * [`command`] — every change as a command that returns its exact undo;
//!   [`History`] undoes and redoes.
//! * [`layout`] — where everything is: the CREATE blocks replayed through
//!   the runtimes' own layout engine (`crate::layout::engine`), so Align,
//!   Anchors, Constraints and the resize preview are the running program's.
//! * [`snap`] — the grid, smart guides (edges, centres, baselines, margins,
//!   equal spacing).
//! * [`arrange`] — align, distribute, same size, centre, nudge, z-order,
//!   Tab order.
//! * [`inspect`] — the inspected object: rows from the registry, values from
//!   the source, a typed value as an undoable command.
//! * [`text`] — the CREATE text of new components; `rapidr-designer` reads
//!   a program's CREATE blocks into this model and turns each command into
//!   the smallest text edit (two-way sync).
//! * [`value`] — reading and writing property values.

pub mod arrange;
pub mod command;
pub mod inspect;
pub mod layout;
pub mod model;
pub mod snap;
pub mod text;
pub mod value;

pub use command::{Command, CommandError, History};
pub use layout::Layout;
pub use model::{FormDesign, Item, Node, NodeId, Prop, SubItem, Subtree};
pub use snap::{Guide, GuideKind, Snapper};

use crate::layout::{Rect, AK_BOTTOM, AK_LEFT, AK_RIGHT, AK_TOP, DEFAULT_ANCHORS};

/// The selected components; the first is the primary one (the reference
/// for align and same size, the one the inspector shows values of).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Selection {
    ids: Vec<NodeId>,
}

impl Selection {
    pub fn ids(&self) -> &[NodeId] {
        &self.ids
    }

    pub fn primary(&self) -> Option<NodeId> {
        self.ids.first().copied()
    }

    pub fn contains(&self, id: NodeId) -> bool {
        self.ids.contains(&id)
    }

    pub fn len(&self) -> usize {
        self.ids.len()
    }

    pub fn is_empty(&self) -> bool {
        self.ids.is_empty()
    }

    /// Only `id` (a click).
    pub fn set(&mut self, id: NodeId) {
        self.ids = vec![id];
    }

    /// Shift / Ctrl+click: in or out of the selection.
    pub fn toggle(&mut self, id: NodeId) {
        match self.ids.iter().position(|&i| i == id) {
            Some(k) => {
                self.ids.remove(k);
            }
            None => self.ids.push(id),
        }
    }

    pub fn add(&mut self, id: NodeId) {
        if !self.contains(id) {
            self.ids.push(id);
        }
    }

    pub fn set_all(&mut self, ids: Vec<NodeId>) {
        self.ids = ids;
        self.ids.dedup();
    }

    pub fn clear(&mut self) {
        self.ids.clear();
    }

    /// Drops what the form no longer has.
    pub fn retain(&mut self, d: &FormDesign) {
        self.ids.retain(|&i| d.node(i).is_some());
    }
}

/// What Copy / Cut keep: the components as trees, and as CREATE text (for
/// the system clipboard).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Clip {
    pub trees: Vec<Subtree>,
    pub text: String,
}

/// The four sides of a component's anchor pins (`Anchors`' bits).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Side {
    Left,
    Top,
    Right,
    Bottom,
}

impl Side {
    pub fn bit(self) -> i64 {
        match self {
            Side::Left => AK_LEFT,
            Side::Top => AK_TOP,
            Side::Right => AK_RIGHT,
            Side::Bottom => AK_BOTTOM,
        }
    }
}

/// A form being designed: the model, its selection, its undo history and
/// its snapping settings. Every change goes through [`Designer::execute`]
/// (or the operations below, which build commands), and is journaled for
/// the text side ([`Designer::take_applied`]).
#[derive(Clone, Debug)]
pub struct Designer {
    pub design: FormDesign,
    pub selection: Selection,
    pub history: History,
    pub snapper: Snapper,
    /// Commands applied since the last [`Designer::take_applied`] (done,
    /// undone as their undo, redone), in order.
    applied: Vec<Command>,
}

impl Designer {
    pub fn new(design: FormDesign) -> Designer {
        Designer { design, selection: Selection::default(), history: History::default(), snapper: Snapper::default(), applied: Vec::new() }
    }

    /// The form laid out as the program lays it out.
    pub fn layout(&self) -> Layout {
        Layout::of(&self.design)
    }

    /// The resize preview: the form laid out at `width` × `height` (its
    /// Width / Height), as the running program resized by its user — the
    /// design itself unchanged.
    pub fn preview(&self, width: i64, height: i64) -> Layout {
        let mut l = self.layout();
        l.resize(width, height);
        l
    }

    /// Applies a command as one undoable step.
    pub fn execute(&mut self, cmd: Command) -> Result<(), CommandError> {
        if cmd.is_empty() {
            return Ok(());
        }
        self.history.execute(&mut self.design, cmd.clone())?;
        self.applied.push(cmd);
        self.selection.retain(&self.design);
        Ok(())
    }

    pub fn undo(&mut self) -> bool {
        match self.history.undo(&mut self.design) {
            Some(c) => {
                self.applied.push(c);
                self.selection.retain(&self.design);
                true
            }
            None => false,
        }
    }

    pub fn redo(&mut self) -> bool {
        match self.history.redo(&mut self.design) {
            Some(c) => {
                self.applied.push(c);
                self.selection.retain(&self.design);
                true
            }
            None => false,
        }
    }

    /// The commands applied since the last call (for the text side).
    pub fn take_applied(&mut self) -> Vec<Command> {
        std::mem::take(&mut self.applied)
    }

    fn sel(&self) -> Vec<NodeId> {
        self.selection.ids().to_vec()
    }

    /// Adds a component from the toolbox at `rect` inside `parent` (the
    /// form when `None`), selected. Returns its id.
    pub fn add_component(&mut self, type_name: &str, rect: Rect, parent: Option<NodeId>) -> Result<NodeId, CommandError> {
        let parent = parent.unwrap_or(self.design.root());
        let tree = text::new_component(&self.design, type_name, rect);
        let name = tree.name.clone();
        let index = self.design.node(parent).map_or(0, |p| p.body.len());
        self.execute(Command::Insert { parent, index, tree })?;
        let id = self.design.find(&name).ok_or(CommandError::NoSuchComponent(0))?;
        self.selection.set(id);
        Ok(id)
    }

    /// The selected components without those inside another selected one.
    fn tops(&self) -> Vec<NodeId> {
        let sel = self.sel();
        sel.iter().copied().filter(|&n| n != self.design.root() && !sel.iter().any(|&o| o != n && self.design.is_within(n, o))).collect()
    }

    /// Deletes the selection.
    pub fn delete(&mut self) -> Result<(), CommandError> {
        let cmds = self.tops().into_iter().map(|node| Command::Remove { node }).collect();
        self.execute(Command::Batch(cmds))?;
        self.selection.clear();
        Ok(())
    }

    pub fn copy(&self) -> Clip {
        let trees: Vec<Subtree> = self.tops().into_iter().filter_map(|n| self.design.subtree(n)).collect();
        let text = trees.iter().map(|t| text::write_create(t, "", &text::Style::default())).collect();
        Clip { trees, text }
    }

    pub fn cut(&mut self) -> Result<Clip, CommandError> {
        let clip = self.copy();
        self.delete()?;
        Ok(clip)
    }

    /// Pastes into the primary selection when it's a container, else into
    /// its parent (the form with nothing selected): names kept unique,
    /// handlers unbound, offset by the grid when pasted where the copies
    /// came from. The pasted components are selected.
    pub fn paste(&mut self, clip: &Clip) -> Result<Vec<NodeId>, CommandError> {
        let target = match self.selection.primary() {
            Some(p) if self.design.node(p).is_some_and(|n| n.is_container() || n.is_form()) => p,
            Some(p) => self.design.parent(p).unwrap_or(self.design.root()),
            None => self.design.root(),
        };
        let mut taken = Vec::new();
        let mut names = Vec::new();
        let mut cmds = Vec::new();
        let mut index = self.design.node(target).map_or(0, |p| p.body.len());
        for t in &clip.trees {
            let mut tree = self.design.pasteable(t, &mut taken);
            // (where it came from still has its original: shift the copy)
            if self.design.find(&t.name).is_some_and(|orig| self.design.parent(orig) == Some(target)) {
                let g = self.snapper.grid.max(1);
                for p in ["Left", "Top"] {
                    if let Some(v) = tree.prop(p).and_then(value::int) {
                        tree.set_prop(p, (v + g).to_string());
                    }
                }
            }
            names.push(tree.name.clone());
            cmds.push(Command::Insert { parent: target, index, tree });
            index += 1;
        }
        self.execute(Command::Batch(cmds))?;
        let ids: Vec<NodeId> = names.iter().filter_map(|n| self.design.find(n)).collect();
        self.selection.set_all(ids.clone());
        Ok(ids)
    }

    /// Copy and paste in one step.
    pub fn duplicate(&mut self) -> Result<Vec<NodeId>, CommandError> {
        let clip = self.copy();
        // (beside the originals, in their parent)
        if let Some(p) = self.selection.primary().and_then(|p| self.design.parent(p)) {
            self.selection.set(p);
        }
        self.paste(&clip)
    }

    pub fn align(&mut self, how: arrange::AlignHow) -> Result<(), CommandError> {
        let c = arrange::align(&self.design, &self.layout(), &self.sel(), how);
        self.execute(c)
    }

    pub fn distribute(&mut self, axis: arrange::Axis) -> Result<(), CommandError> {
        let c = arrange::distribute(&self.design, &self.layout(), &self.sel(), axis);
        self.execute(c)
    }

    pub fn same_size(&mut self, axis: arrange::Axis) -> Result<(), CommandError> {
        let c = arrange::same_size(&self.design, &self.layout(), &self.sel(), axis);
        self.execute(c)
    }

    pub fn center_in_parent(&mut self, axis: arrange::Axis) -> Result<(), CommandError> {
        let c = arrange::center_in_parent(&self.design, &self.layout(), &self.sel(), axis);
        self.execute(c)
    }

    pub fn nudge(&mut self, dx: i64, dy: i64, resize: bool) -> Result<(), CommandError> {
        let c = arrange::nudge(&self.design, &self.layout(), &self.sel(), dx, dy, resize);
        self.execute(c)
    }

    pub fn bring_to_front(&mut self) -> Result<(), CommandError> {
        let cmds = self.tops().into_iter().filter_map(|n| arrange::bring_to_front(&self.design, n)).collect();
        self.execute(Command::Batch(cmds))
    }

    pub fn send_to_back(&mut self) -> Result<(), CommandError> {
        let cmds = self.tops().into_iter().filter_map(|n| arrange::send_to_back(&self.design, n)).collect();
        self.execute(Command::Batch(cmds))
    }

    /// Puts a container's children in this Tab order.
    pub fn set_tab_order(&mut self, parent: NodeId, order: &[NodeId]) -> Result<(), CommandError> {
        let c = arrange::set_tab_order(&self.design, parent, order);
        self.execute(c)
    }

    /// Places a component at `rect` (a drag or a resize ended).
    pub fn place(&mut self, node: NodeId, rect: Rect) -> Result<(), CommandError> {
        let Some(old) = self.layout().rect(node) else { return Err(CommandError::NoSuchComponent(node)) };
        self.execute(Command::Batch(arrange::set_rect(node, old, rect)))
    }

    /// A component's Anchors (what its CREATE block says, else akLeft +
    /// akTop).
    pub fn anchors(&self, node: NodeId) -> i64 {
        self.design.node(node).and_then(|n| n.int("Anchors")).unwrap_or(DEFAULT_ANCHORS)
    }

    /// Sets the Anchors of the selection (written as `akLeft + akRight …`;
    /// not written where they are already these).
    pub fn set_anchors(&mut self, bits: i64) -> Result<(), CommandError> {
        let mut cmds = Vec::new();
        for n in self.sel().into_iter().filter(|&n| n != self.design.root()) {
            let has = self.design.node(n).and_then(|x| x.prop("Anchors")).is_some();
            if self.anchors(n) != bits || (!has && bits != DEFAULT_ANCHORS) {
                cmds.push(Command::SetProp { node: n, name: "Anchors".into(), value: Some(value::write_anchors(bits)) });
            }
        }
        self.execute(Command::Batch(cmds))
    }

    /// An anchor pin clicked: that side on or off for the primary selection
    /// (and the rest of the selection follows its new Anchors).
    pub fn toggle_anchor(&mut self, side: Side) -> Result<(), CommandError> {
        let Some(p) = self.selection.primary() else { return Ok(()) };
        self.set_anchors(self.anchors(p) ^ side.bit())
    }

    /// The inspector's rows for the selection.
    pub fn inspect(&self) -> inspect::Inspected {
        inspect::inspect(&self.design, self.selection.ids())
    }

    /// The inspector set a value: one undoable step on every selected
    /// component (`None`: back to the default).
    pub fn set_property(&mut self, prop: &str, typed: Option<&str>) -> Result<(), CommandError> {
        let c = inspect::set_value(&self.design, self.selection.ids(), prop, typed);
        self.execute(c)
    }

    /// Renames a component (its CREATE; references are the language
    /// service's rename, I3).
    pub fn rename(&mut self, node: NodeId, name: &str) -> Result<(), CommandError> {
        self.execute(Command::Rename { node, name: name.to_string() })
    }
}

#[cfg(test)]
mod tests;
