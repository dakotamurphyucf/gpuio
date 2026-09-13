use gpuio_protocol::{HandlerId, NodeId, WindowId, v1::*};
use std::{
    collections::{BTreeMap, BTreeSet},
    sync::Arc,
};

#[derive(Clone, Debug, PartialEq)]
pub struct Node {
    pub id: NodeId,
    pub kind: Kind,
    pub text: Arc<str>,
    pub editor: Option<Arc<EditorConfig>>,
    pub control: Option<Control>,
    pub choice: Option<Arc<ChoiceConfig>>,
    pub style: Arc<[Style]>,
    pub handler: Option<HandlerId>,
    pub children: Arc<[NodeId]>,
    pub parent: Option<NodeId>,
}

impl Node {
    fn payload_bytes(&self) -> usize {
        self.text.len()
            + self
                .choice
                .as_ref()
                .map_or(0, |config| config.retained_bytes())
            + self
                .editor
                .as_ref()
                .map_or(0, |config| config.label.len() + config.placeholder.len())
            + if matches!(self.kind, Kind::Input | Kind::Textarea) {
                EDITOR_RESERVED_BYTES
            } else {
                0
            }
            + std::mem::size_of_val(self.style.as_ref())
            + self
                .style
                .iter()
                .map(crate::style::retained_bytes)
                .sum::<usize>()
            + std::mem::size_of_val(self.children.as_ref())
    }
}

#[derive(Clone, Debug, Default)]
struct Slot {
    generation: u32,
    node: Option<Node>,
}

#[derive(Debug)]
pub struct Tree {
    window: WindowId,
    revision: i64,
    root: Option<NodeId>,
    slots: Vec<Slot>,
    node_count: usize,
    retained_bytes: usize,
}

#[derive(Debug, PartialEq, Eq)]
pub struct Applied {
    pub revision: i64,
    pub touched_records: usize,
    pub validated_nodes: usize,
    pub dirty: Vec<NodeId>,
}

impl Tree {
    pub fn new(window: WindowId) -> Self {
        Self {
            window,
            revision: 0,
            root: None,
            slots: Vec::new(),
            node_count: 0,
            retained_bytes: 0,
        }
    }

    pub fn revision(&self) -> i64 {
        self.revision
    }
    pub fn retained_bytes(&self) -> usize {
        self.retained_bytes
    }
    pub fn root(&self) -> Option<NodeId> {
        self.root
    }
    pub fn len(&self) -> usize {
        self.node_count
    }
    pub fn is_empty(&self) -> bool {
        self.node_count == 0
    }

    pub fn get(&self, id: NodeId) -> Option<&Node> {
        self.slots
            .get(id.slot())
            .filter(|s| s.generation == id.generation())?
            .node
            .as_ref()
    }

    pub fn accepts_handler(&self, node: NodeId, handler: HandlerId) -> bool {
        self.get(node)
            .is_some_and(|node| node.handler == Some(handler))
    }

    /// Validate a touched-record overlay before publishing any mutation. Text and
    /// style-only edits do not traverse unrelated nodes. Structural validation
    /// walks the final tree iteratively, bounded by MAX_NODES and MAX_DEPTH.
    pub fn apply(&mut self, tx: &Transaction) -> Result<Applied, ErrorCode> {
        self.apply_with_budget(tx, MAX_RETAINED_BYTES)
    }

    /// Payload budget excludes fixed-size slots, independently bounded by MAX_NODES.
    /// A session may lower the budget to enforce its aggregate memory ceiling.
    pub fn apply_with_budget(
        &mut self,
        tx: &Transaction,
        budget: usize,
    ) -> Result<Applied, ErrorCode> {
        if tx.window != self.window {
            return Err(ErrorCode::StaleHandle);
        }
        if tx.base != self.revision || self.revision.checked_add(1) != Some(tx.revision) {
            return Err(ErrorCode::InvalidRevision);
        }
        if tx.operations.len() > MAX_OPERATIONS {
            return Err(ErrorCode::LimitExceeded);
        }
        let mut plan = Plan {
            original: self,
            changes: BTreeMap::new(),
            root: self.root,
            slot_count: self.slots.len(),
            node_count: self.node_count,
            retained_bytes: self.retained_bytes,
            budget: budget.min(MAX_RETAINED_BYTES),
            structural: false,
        };
        for op in &tx.operations {
            plan.operation(op)?;
        }
        for slot in plan.changes.values() {
            if let Some(node) = &slot.node {
                if node.choice.is_some() && !matches!(node.kind, Kind::RadioGroup | Kind::Select) {
                    return Err(ErrorCode::InvalidTree);
                }
                if node
                    .control
                    .is_some_and(|control| control.kind() != node.kind)
                {
                    return Err(ErrorCode::InvalidTree);
                }
                match node.kind {
                    Kind::Input | Kind::Textarea => {
                        let config = node.editor.as_ref().ok_or(ErrorCode::InvalidTree)?;
                        if node.handler.is_none()
                            || !config.is_valid()
                            || node.text.contains('\0')
                            || (node.kind == Kind::Input
                                && (config.min_rows != 1
                                    || config.max_rows != 1
                                    || node.text.contains(['\r', '\n'])))
                        {
                            return Err(ErrorCode::InvalidTree);
                        }
                    }
                    Kind::Container | Kind::Text | Kind::Button => {
                        if node.editor.is_some() {
                            return Err(ErrorCode::InvalidTree);
                        }
                    }
                    Kind::Checkbox | Kind::Switch => {
                        let control = node.control.ok_or(ErrorCode::InvalidTree)?;
                        if node.editor.is_some()
                            || (!control.disabled() && node.handler.is_none())
                            || node.text.contains('\0')
                        {
                            return Err(ErrorCode::InvalidTree);
                        }
                    }
                    Kind::RadioGroup | Kind::Select => {
                        let config = node.choice.as_ref().ok_or(ErrorCode::InvalidTree)?;
                        if !config.is_valid()
                            || node.editor.is_some()
                            || (!config.disabled && node.handler.is_none())
                        {
                            return Err(ErrorCode::InvalidTree);
                        }
                    }
                }
            }
        }
        let validated_nodes = if plan.structural {
            plan.validate_structure()?
        } else {
            0
        };
        let mut dirty = BTreeSet::new();
        for slot in plan.changes.values() {
            let mut id = slot.node.as_ref().map(|n| n.id);
            while let Some(current) = id {
                if !dirty.insert(current) {
                    break;
                }
                id = plan.node(current)?.parent;
            }
        }
        if let Some(root) = plan.root {
            dirty.insert(root);
        }
        let touched_records = plan.changes.len();
        let Plan {
            changes,
            root,
            slot_count,
            node_count,
            retained_bytes,
            ..
        } = plan;
        // All validation has succeeded. Reserve before mutating semantic state.
        self.slots
            .try_reserve(slot_count - self.slots.len())
            .map_err(|_| ErrorCode::LimitExceeded)?;
        self.slots.resize_with(slot_count, Slot::default);
        for (index, slot) in changes {
            self.slots[index] = slot;
        }
        self.root = root;
        self.node_count = node_count;
        self.retained_bytes = retained_bytes;
        self.revision = tx.revision;
        Ok(Applied {
            revision: tx.revision,
            touched_records,
            validated_nodes,
            dirty: dirty.into_iter().collect(),
        })
    }
}

struct Plan<'a> {
    original: &'a Tree,
    changes: BTreeMap<usize, Slot>,
    root: Option<NodeId>,
    slot_count: usize,
    node_count: usize,
    retained_bytes: usize,
    budget: usize,
    structural: bool,
}

impl Plan<'_> {
    fn slot(&self, index: usize) -> Option<&Slot> {
        self.changes
            .get(&index)
            .or_else(|| self.original.slots.get(index))
    }

    fn node(&self, id: NodeId) -> Result<&Node, ErrorCode> {
        self.slot(id.slot())
            .filter(|s| s.generation == id.generation())
            .and_then(|s| s.node.as_ref())
            .ok_or(ErrorCode::StaleHandle)
    }

    fn node_mut(&mut self, id: NodeId) -> Result<&mut Node, ErrorCode> {
        self.node(id)?;
        let slot = self.slot(id.slot()).expect("validated slot").clone();
        Ok(self
            .changes
            .entry(id.slot())
            .or_insert(slot)
            .node
            .as_mut()
            .expect("validated node"))
    }

    fn operation(&mut self, op: &Op) -> Result<(), ErrorCode> {
        let target = match op {
            Op::Create(id, ..)
            | Op::Remove(id)
            | Op::SetText(id, ..)
            | Op::SetStyle(id, ..)
            | Op::SetEditor(id, ..)
            | Op::SetControl(id, ..)
            | Op::SetChoice(id, ..)
            | Op::Bind(id, ..)
            | Op::Splice(id, ..) => Some(*id),
            Op::SetRoot(_) => None,
        };
        let bytes = |plan: &Self| {
            target
                .and_then(|id| plan.slot(id.slot()))
                .and_then(|slot| slot.node.as_ref())
                .map_or(0, Node::payload_bytes)
        };
        let before = bytes(self);
        self.operation_inner(op)?;
        self.retained_bytes = self
            .retained_bytes
            .checked_sub(before)
            .and_then(|remaining| remaining.checked_add(bytes(self)))
            .filter(|total| *total <= self.budget)
            .ok_or(ErrorCode::LimitExceeded)?;
        Ok(())
    }

    fn operation_inner(&mut self, op: &Op) -> Result<(), ErrorCode> {
        match op {
            Op::Create(id, kind, text, handler) => {
                validate_text(text)?;
                if id.slot() > self.slot_count || id.slot() >= MAX_NODES {
                    return Err(ErrorCode::LimitExceeded);
                }
                let old_generation = if id.slot() == self.slot_count {
                    0
                } else {
                    let old = self.slot(id.slot()).ok_or(ErrorCode::StaleHandle)?;
                    if old.node.is_some() {
                        return Err(ErrorCode::InvalidTree);
                    }
                    old.generation
                };
                if old_generation.checked_add(1) != Some(id.generation()) {
                    return Err(ErrorCode::StaleHandle);
                }
                if id.slot() == self.slot_count {
                    self.slot_count += 1;
                }
                self.node_count += 1;
                self.changes.insert(
                    id.slot(),
                    Slot {
                        generation: id.generation(),
                        node: Some(Node {
                            id: *id,
                            kind: *kind,
                            text: Arc::from(text.as_str()),
                            editor: None,
                            control: None,
                            choice: None,
                            style: Arc::from([]),
                            handler: *handler,
                            children: Arc::from([]),
                            parent: None,
                        }),
                    },
                );
                self.structural = true;
            }
            Op::Remove(id) => {
                self.node(*id)?;
                self.changes.insert(
                    id.slot(),
                    Slot {
                        generation: id.generation(),
                        node: None,
                    },
                );
                self.node_count -= 1;
                self.structural = true;
            }
            Op::SetText(id, text) => {
                if matches!(self.node(*id)?.kind, Kind::Input | Kind::Textarea) {
                    // Native editor contents change only through explicit commands.
                    return Err(ErrorCode::InvalidTree);
                }
                validate_text(text)?;
                self.node_mut(*id)?.text = Arc::from(text.as_str());
            }
            Op::SetEditor(id, config) => {
                if !matches!(self.node(*id)?.kind, Kind::Input | Kind::Textarea)
                    || !config.is_valid()
                {
                    return Err(ErrorCode::InvalidTree);
                }
                self.node_mut(*id)?.editor = Some(Arc::new(config.clone()));
            }
            Op::SetChoice(id, config) => {
                if !matches!(self.node(*id)?.kind, Kind::RadioGroup | Kind::Select)
                    || !config.is_valid()
                {
                    return Err(ErrorCode::InvalidTree);
                }
                self.node_mut(*id)?.choice = Some(Arc::new(config.clone()));
            }
            Op::SetControl(id, control) => {
                if self.node(*id)?.kind != control.kind() {
                    return Err(ErrorCode::InvalidTree);
                }
                self.node_mut(*id)?.control = Some(*control);
            }
            Op::SetStyle(id, style) => {
                validate_style(style)?;
                self.node_mut(*id)?.style = Arc::from(style.as_slice());
            }
            Op::Bind(id, handler) => self.node_mut(*id)?.handler = *handler,
            Op::Splice(parent, offset, remove, insert) => {
                let node = self.node(*parent)?;
                if node.kind != Kind::Container {
                    return Err(ErrorCode::InvalidTree);
                }
                let start = usize::try_from(*offset).map_err(|_| ErrorCode::InvalidTree)?;
                let count = usize::try_from(*remove).map_err(|_| ErrorCode::InvalidTree)?;
                let end = start
                    .checked_add(count)
                    .filter(|end| *end <= node.children.len())
                    .ok_or(ErrorCode::InvalidTree)?;
                let size = node.children.len() - count + insert.len();
                if size > MAX_NODES {
                    return Err(ErrorCode::LimitExceeded);
                }
                let mut children = Vec::with_capacity(size);
                children.extend_from_slice(&node.children[..start]);
                children.extend_from_slice(insert);
                children.extend_from_slice(&node.children[end..]);
                self.node_mut(*parent)?.children = children.into();
                self.structural = true;
            }
            Op::SetRoot(root) => {
                self.root = *root;
                self.structural = true;
            }
        }
        Ok(())
    }

    fn validate_structure(&mut self) -> Result<usize, ErrorCode> {
        let mut stack = Vec::new();
        if let Some(root) = self.root {
            stack.push((root, None, 0));
        }
        let mut seen = BTreeSet::new();
        let mut parents = Vec::new();
        while let Some((id, parent, depth)) = stack.pop() {
            if depth > MAX_DEPTH || !seen.insert(id) {
                return Err(ErrorCode::InvalidTree);
            }
            let node = self.node(id)?;
            if node.kind != Kind::Container && !node.children.is_empty() {
                return Err(ErrorCode::InvalidTree);
            }
            if node.parent != parent {
                parents.push((id, parent));
            }
            // Check edge count before growing traversal storage (duplicate edges
            // must not create unbounded intermediate work).
            if stack.len() + node.children.len() > MAX_NODES {
                return Err(ErrorCode::LimitExceeded);
            }
            stack.extend(
                node.children
                    .iter()
                    .map(|child| (*child, Some(id), depth + 1)),
            );
        }
        if seen.len() != self.node_count {
            return Err(ErrorCode::InvalidTree);
        }
        for (id, parent) in parents {
            self.node_mut(id)?.parent = parent;
        }
        Ok(seen.len())
    }
}

fn validate_text(text: &str) -> Result<(), ErrorCode> {
    if text.len() > MAX_TEXT_BYTES {
        Err(ErrorCode::LimitExceeded)
    } else {
        Ok(())
    }
}

pub fn validate_style(style: &[Style]) -> Result<(), ErrorCode> {
    fn nonnegative(value: f64) -> bool {
        value.is_finite() && (0.0..=1_000_000.0).contains(&value)
    }
    fn length(value: &Length) -> bool {
        match value {
            Length::Auto => true,
            Length::Px(v) | Length::Percent(v) => nonnegative(*v),
        }
    }
    fn color(value: &Color) -> bool {
        match value {
            Color::Rgba(v) => (0..=u32::MAX as i64).contains(v),
            Color::Token(v) => (0..256).contains(v),
        }
    }
    if style.len() > MAX_STYLE_FIELDS {
        return Err(ErrorCode::LimitExceeded);
    }
    for field in style {
        if matches!(
            field,
            Style::Background(Color::Token(_))
                | Style::Foreground(Color::Token(_))
                | Style::HoverBackground(Color::Token(_))
                | Style::PressedBackground(Color::Token(_))
                | Style::FocusBackground(Color::Token(_))
        ) {
            // The public theme adapter resolves tokens before submitting colors.
            return Err(ErrorCode::UnsupportedCapability);
        }
        let valid = match field {
            Style::Width(v)
            | Style::Height(v)
            | Style::MinWidth(v)
            | Style::MinHeight(v)
            | Style::MaxWidth(v)
            | Style::MaxHeight(v) => length(v),
            Style::Padding(v)
            | Style::Gap(v)
            | Style::Grow(v)
            | Style::Shrink(v)
            | Style::FontSize(v)
            | Style::Radius(v) => nonnegative(*v),
            Style::Direction(v) => (0..=3).contains(v),
            Style::Background(v)
            | Style::Foreground(v)
            | Style::HoverBackground(v)
            | Style::PressedBackground(v)
            | Style::FocusBackground(v) => color(v),
            Style::Fields(fields) => {
                crate::style::validate_fields(fields)?;
                true
            }
            Style::State(state, fields) => {
                crate::style::validate_fields(fields)?;
                (1..=7).contains(state)
                    && !fields.iter().any(|field| {
                        matches!(
                            field,
                            Field::PointerEvents(_)
                                | Field::UserSelect(_)
                                | Field::SelectionColor(_)
                                | Field::AccessibleName(_)
                        )
                    })
            }
            Style::Opacity(v) => v.is_finite() && (0.0..=1.0).contains(v),
        };
        if !valid {
            return Err(ErrorCode::Malformed);
        }
    }
    Ok(())
}
