//! The designer's changes as commands: each one applied gives the command
//! that undoes it exactly, so [`History`] can undo and redo any sequence.
//! Moving, resizing, aligning, anchoring, the inspector's edits and Tab
//! order are [`Command::SetProp`]s; adding, pasting and duplicating are
//! [`Command::Insert`]s; deleting and cutting [`Command::Remove`]s;
//! z-order and reparenting [`Command::Move`]s. A [`Command::Batch`] is one
//! step (align five components: one undo).
//!
//! The text side (docs/ide-plan.md I4, two-way sync) turns each applied
//! command into the smallest edit of the CREATE blocks: `rapidr-designer`.

use super::model::{prop_key, FormDesign, Item, NodeId, Prop, Subtree};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Command {
    /// Sets a property's value text (its last assignment, else a new line
    /// after the block's last property); `None` drops every assignment.
    SetProp { node: NodeId, name: String, value: Option<String> },
    /// Puts an assignment at a place in the block (undo of a drop).
    InsertProp { node: NodeId, index: usize, prop: Prop },
    /// Drops the assignment at a place in the block (undo of an insert).
    RemoveProp { node: NodeId, index: usize },
    /// A component (and what's inside it) put under `parent`, at `index`
    /// of its block (`usize::MAX`: after everything).
    Insert { parent: NodeId, index: usize, tree: Subtree },
    /// A component taken out (with what's inside it).
    Remove { node: NodeId },
    /// A component moved to `parent`'s block at `index` (counted once it's
    /// taken out of its place; z-order: later is on top; reparenting).
    Move { node: NodeId, parent: NodeId, index: usize },
    Rename { node: NodeId, name: String },
    /// Several commands as one step.
    Batch(Vec<Command>),
}

/// Why a command couldn't apply (the form is left as it was).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CommandError {
    NoSuchComponent(NodeId),
    /// The form itself can't be removed or moved.
    TheForm,
    /// Into itself or what's inside it.
    Cycle,
    NameTaken(String),
    /// Not an assignment there.
    NoProperty(NodeId, usize),
}

impl std::fmt::Display for CommandError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CommandError::NoSuchComponent(id) => write!(f, "no component #{id}"),
            CommandError::TheForm => write!(f, "the form itself can't be removed or moved"),
            CommandError::Cycle => write!(f, "a component can't go inside itself"),
            CommandError::NameTaken(n) => write!(f, "a component is already named {n}"),
            CommandError::NoProperty(id, i) => write!(f, "no property at {i} of component #{id}"),
        }
    }
}

impl Command {
    /// Applies the command; returns the one that undoes it.
    pub fn apply(&self, d: &mut FormDesign) -> Result<Command, CommandError> {
        match self {
            Command::SetProp { node, name, value } => {
                let n = d.node_mut(*node).ok_or(CommandError::NoSuchComponent(*node))?;
                match value {
                    Some(v) => match n.prop_index(name) {
                        Some(i) => {
                            let Item::Prop(p) = &mut n.body[i] else { unreachable!("prop_index finds props") };
                            let old = std::mem::replace(&mut p.value, v.clone());
                            Ok(Command::SetProp { node: *node, name: p.name.clone(), value: Some(old) })
                        }
                        None => {
                            let at = n.body.iter().rposition(|i| matches!(i, Item::Prop(_))).map_or(0, |i| i + 1);
                            n.body.insert(at, Item::Prop(Prop { name: name.clone(), value: v.clone() }));
                            Ok(Command::RemoveProp { node: *node, index: at })
                        }
                    },
                    None => {
                        // (every assignment of it, last first; the undo puts
                        // them back first to last)
                        let key = prop_key(name);
                        let mut undo = Vec::new();
                        while let Some(i) = n.body.iter().rposition(|it| matches!(it, Item::Prop(p) if prop_key(&p.name) == key)) {
                            let Item::Prop(p) = n.body.remove(i) else { unreachable!("matched a prop") };
                            undo.push(Command::InsertProp { node: *node, index: i, prop: p });
                        }
                        undo.reverse();
                        Ok(Command::Batch(undo))
                    }
                }
            }
            Command::InsertProp { node, index, prop } => {
                let n = d.node_mut(*node).ok_or(CommandError::NoSuchComponent(*node))?;
                let at = (*index).min(n.body.len());
                n.body.insert(at, Item::Prop(prop.clone()));
                Ok(Command::RemoveProp { node: *node, index: at })
            }
            Command::RemoveProp { node, index } => {
                let n = d.node_mut(*node).ok_or(CommandError::NoSuchComponent(*node))?;
                match n.body.get(*index) {
                    Some(Item::Prop(_)) => {
                        let Item::Prop(prop) = n.body.remove(*index) else { unreachable!("checked") };
                        Ok(Command::InsertProp { node: *node, index: *index, prop })
                    }
                    _ => Err(CommandError::NoProperty(*node, *index)),
                }
            }
            Command::Insert { parent, index, tree } => {
                let p = d.node(*parent).ok_or(CommandError::NoSuchComponent(*parent))?;
                let at = (*index).min(p.body.len());
                for t in tree.all() {
                    if d.find(&t.name).is_some() {
                        return Err(CommandError::NameTaken(t.name.clone()));
                    }
                }
                let id = d.attach(Some((*parent, at)), tree.clone(), true);
                Ok(Command::Remove { node: id })
            }
            Command::Remove { node } => {
                if *node == d.root() {
                    return Err(CommandError::TheForm);
                }
                let (tree, parent, at) = d.detach(*node).ok_or(CommandError::NoSuchComponent(*node))?;
                Ok(Command::Insert { parent, index: at, tree })
            }
            Command::Move { node, parent, index } => {
                if *node == d.root() {
                    return Err(CommandError::TheForm);
                }
                if d.node(*parent).is_none() {
                    return Err(CommandError::NoSuchComponent(*parent));
                }
                if d.node(*node).is_none() {
                    return Err(CommandError::NoSuchComponent(*node));
                }
                if d.is_within(*parent, *node) {
                    return Err(CommandError::Cycle);
                }
                let (tree, from, at) = d.detach(*node).ok_or(CommandError::NoSuchComponent(*node))?;
                let len = d.node(*parent).map_or(0, |p| p.body.len());
                let to = (*index).min(len);
                d.attach(Some((*parent, to)), tree, true);
                Ok(Command::Move { node: *node, parent: from, index: at })
            }
            Command::Rename { node, name } => {
                if d.find(name).is_some_and(|other| other != *node) {
                    return Err(CommandError::NameTaken(name.clone()));
                }
                let n = d.node_mut(*node).ok_or(CommandError::NoSuchComponent(*node))?;
                let old = std::mem::replace(&mut n.name, name.clone());
                Ok(Command::Rename { node: *node, name: old })
            }
            Command::Batch(cmds) => {
                let mut undo = Vec::with_capacity(cmds.len());
                for c in cmds {
                    match c.apply(d) {
                        Ok(u) => undo.push(u),
                        Err(e) => {
                            // (the form left as it was)
                            for u in undo.into_iter().rev() {
                                let _ = u.apply(d);
                            }
                            return Err(e);
                        }
                    }
                }
                undo.reverse();
                Ok(Command::Batch(undo))
            }
        }
    }

    /// Whether it changes nothing (an empty batch).
    pub fn is_empty(&self) -> bool {
        matches!(self, Command::Batch(c) if c.iter().all(Command::is_empty))
    }

    /// The batch's commands, flattened (a single command: itself).
    pub fn flatten(&self) -> Vec<&Command> {
        match self {
            Command::Batch(c) => c.iter().flat_map(Command::flatten).collect(),
            c => vec![c],
        }
    }
}

/// Undo and redo: each step is a command and the one that undoes it.
#[derive(Clone, Debug, Default)]
pub struct History {
    undo: Vec<(Command, Command)>,
    redo: Vec<(Command, Command)>,
}

impl History {
    /// Applies `cmd` as a new step (redo is forgotten). Empty steps aren't
    /// kept.
    pub fn execute(&mut self, d: &mut FormDesign, cmd: Command) -> Result<(), CommandError> {
        if cmd.is_empty() {
            return Ok(());
        }
        let undo = cmd.apply(d)?;
        self.undo.push((cmd, undo));
        self.redo.clear();
        Ok(())
    }

    /// Undoes the last step: the command that undid it (for the text
    /// side), or `None` with nothing to undo.
    pub fn undo(&mut self, d: &mut FormDesign) -> Option<Command> {
        let (cmd, undo) = self.undo.pop()?;
        match undo.apply(d) {
            Ok(redo) => {
                self.redo.push((redo, cmd));
                Some(undo)
            }
            Err(_) => None,
        }
    }

    /// Does again the last step undone.
    pub fn redo(&mut self, d: &mut FormDesign) -> Option<Command> {
        let (cmd, _) = self.redo.pop()?;
        match cmd.apply(d) {
            Ok(undo) => {
                self.undo.push((cmd.clone(), undo));
                Some(cmd)
            }
            Err(_) => None,
        }
    }

    pub fn can_undo(&self) -> bool {
        !self.undo.is_empty()
    }

    pub fn can_redo(&self) -> bool {
        !self.redo.is_empty()
    }

    pub fn clear(&mut self) {
        self.undo.clear();
        self.redo.clear();
    }
}
