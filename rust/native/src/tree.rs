use gpuio_protocol::{HandlerId, NodeId, WindowId, v1::*};
use std::{
    collections::{BTreeMap, BTreeSet},
    sync::Arc,
};

fn allows_children(kind: Kind) -> bool {
    matches!(
        kind,
        Kind::Container
            | Kind::FocusScope
            | Kind::Tooltip
            | Kind::CommandScope
            | Kind::Menu
            | Kind::Toast
            | Kind::ToastStack
            | Kind::PointerArea
            | Kind::DragSource
            | Kind::DropTarget
    )
}

#[derive(Clone, Debug, PartialEq)]
pub struct Node {
    pub id: NodeId,
    pub kind: Kind,
    pub text: Arc<str>,
    pub editor: Option<Arc<EditorConfig>>,
    pub control: Option<Control>,
    pub choice: Option<Arc<ChoiceConfig>>,
    pub focus_scope: Option<FocusScopeConfig>,
    pub overlay: Option<Arc<OverlayConfig>>,
    pub tooltip: Option<Arc<TooltipConfig>>,
    pub commands: Option<Arc<[CommandConfig]>>,
    pub command_ref: Option<Arc<str>>,
    pub menu: Option<Arc<MenuConfig>>,
    pub palette: Option<Arc<PaletteConfig>>,
    pub progress: Option<Arc<ProgressConfig>>,
    pub toast: Option<Arc<ToastConfig>>,
    pub toast_stack: Option<Arc<ToastStackConfig>>,
    pub drag_source: Option<Arc<gpuio_protocol::drag_drop::Source>>,
    pub drop_target: Option<Arc<gpuio_protocol::drag_drop::Target>>,
    pub pointer: Option<Arc<PointerConfig>>,
    pub placement: Option<Placement>,
    pub combobox_filter: Option<ComboboxFilter>,
    pub choice_appearance: Option<Arc<ChoiceAppearance>>,
    pub style: Arc<[Style]>,
    pub handler: Option<HandlerId>,
    pub children: Arc<[NodeId]>,
    pub parent: Option<NodeId>,
}

impl Node {
    fn payload_bytes(&self) -> usize {
        self.text.len()
            + self
                .drag_source
                .as_ref()
                .map_or(0, |config| config.retained_bytes())
            + self
                .drop_target
                .as_ref()
                .map_or(0, |config| config.retained_bytes())
            + self
                .pointer
                .as_ref()
                .map_or(0, |config| config.retained_bytes())
            + self
                .toast
                .as_ref()
                .map_or(0, |config| config.retained_bytes())
            + self.toast_stack.as_ref().map_or(0, |config| {
                std::mem::size_of::<ToastStackConfig>() + config.label.len()
            })
            + self.progress.as_ref().map_or(0, |config| {
                std::mem::size_of::<ProgressConfig>() + config.label.len()
            })
            + self.commands.as_ref().map_or(0, |commands| {
                commands
                    .iter()
                    .map(CommandConfig::retained_bytes)
                    .sum::<usize>()
            })
            + self.command_ref.as_ref().map_or(0, |id| id.len())
            + self.menu.as_ref().map_or(0, |menu| menu.retained_bytes())
            + self
                .palette
                .as_ref()
                .map_or(0, |palette| palette.retained_bytes())
            + self.tooltip.as_ref().map_or(0, |config| {
                std::mem::size_of::<TooltipConfig>() + config.label.len()
            })
            + self
                .placement
                .map_or(0, |_| std::mem::size_of::<Placement>())
            + self.overlay.as_ref().map_or(0, |config| {
                std::mem::size_of::<OverlayConfig>() + config.label.len()
            })
            + self.choice_appearance.as_ref().map_or(0, |appearance| {
                crate::appearance::retained_bytes(appearance)
            })
            + self
                .choice
                .as_ref()
                .map_or(0, |config| config.retained_bytes())
            + self
                .editor
                .as_ref()
                .map_or(0, |config| config.label.len() + config.placeholder.len())
            + if matches!(self.kind, Kind::Input | Kind::Textarea | Kind::Combobox) {
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

    /// Nearest enabled tooltip whose anchor subtree contains this node.
    pub fn tooltip_description(&self, node: NodeId) -> Option<&str> {
        let mut child = node;
        while let Some(parent) = self.get(child)?.parent {
            let node = self.get(parent)?;
            if let Some(config) = &node.tooltip
                && !config.disabled
                && node.children.first() == Some(&child)
            {
                return Some(&config.label);
            }
            child = parent;
        }
        None
    }

    /// Resolve the nearest matching registry entry, preserving shadowing even
    /// when that entry is disabled.
    pub fn command(&self, node: NodeId, command: &str) -> Option<(NodeId, &CommandConfig)> {
        let mut cursor = Some(node);
        while let Some(id) = cursor {
            let node = self.get(id)?;
            if let Some(commands) = &node.commands
                && let Some(command) = commands.iter().find(|entry| entry.id == command)
            {
                return Some((id, command));
            }
            cursor = node.parent;
        }
        None
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
                if node.choice_appearance.is_some()
                    && !matches!(
                        node.kind,
                        Kind::Select | Kind::Combobox | Kind::Menu | Kind::CommandPalette
                    )
                {
                    return Err(ErrorCode::InvalidTree);
                }
                if node.choice.is_some()
                    && !matches!(node.kind, Kind::RadioGroup | Kind::Select | Kind::Combobox)
                {
                    return Err(ErrorCode::InvalidTree);
                }
                if node
                    .control
                    .is_some_and(|control| control.kind() != node.kind)
                {
                    return Err(ErrorCode::InvalidTree);
                }
                if node.kind == Kind::Combobox {
                    let choices = node.choice.as_ref().ok_or(ErrorCode::InvalidTree)?;
                    let editor = node.editor.as_ref().ok_or(ErrorCode::InvalidTree)?;
                    if node.combobox_filter.is_none()
                        || !choices.is_valid()
                        || choices.label != editor.label
                        || choices.disabled != editor.disabled
                        || editor.read_only
                        || editor.submit_on_enter
                    {
                        return Err(ErrorCode::InvalidTree);
                    }
                } else if node.combobox_filter.is_some() {
                    return Err(ErrorCode::InvalidTree);
                }
                if (node.kind == Kind::DragSource) != node.drag_source.is_some()
                    || (node.kind == Kind::DropTarget) != node.drop_target.is_some()
                    || ((node.drag_source.is_some() || node.drop_target.is_some())
                        && (node.handler.is_none()
                            || !node.text.is_empty()
                            || node.control.is_some()
                            || node.choice.is_some()))
                {
                    return Err(ErrorCode::InvalidTree);
                }
                if (node.kind == Kind::PointerArea) != node.pointer.is_some()
                    || node.pointer.as_ref().is_some_and(|config| {
                        !config.is_valid()
                            || node.handler.is_none()
                            || !node.text.is_empty()
                            || node.control.is_some()
                            || node.choice.is_some()
                    })
                {
                    return Err(ErrorCode::InvalidTree);
                }
                if (node.kind == Kind::Toast) != node.toast.is_some()
                    || node.toast.as_ref().is_some_and(|config| {
                        !config.is_valid()
                            || node.handler.is_none()
                            || !node.text.is_empty()
                            || node.control.is_some()
                            || node.choice.is_some()
                    })
                {
                    return Err(ErrorCode::InvalidTree);
                }
                if (node.kind == Kind::ToastStack) != node.toast_stack.is_some()
                    || node.toast_stack.as_ref().is_some_and(|config| {
                        !config.is_valid()
                            || node.handler.is_some()
                            || !node.text.is_empty()
                            || node.children.len() > MAX_TOASTS
                            || node.control.is_some()
                            || node.choice.is_some()
                    })
                {
                    return Err(ErrorCode::InvalidTree);
                }
                if (node.kind == Kind::Progress) != node.progress.is_some()
                    || node.progress.as_ref().is_some_and(|config| {
                        !config.is_valid()
                            || node.handler.is_some()
                            || !node.text.is_empty()
                            || !node.children.is_empty()
                            || node.control.is_some()
                            || node.choice.is_some()
                    })
                {
                    return Err(ErrorCode::InvalidTree);
                }
                if (node.kind == Kind::CommandPalette) != node.palette.is_some()
                    || node.palette.as_ref().is_some_and(|config| {
                        !config.is_valid()
                            || node.handler.is_none()
                            || !node.text.is_empty()
                            || !node.children.is_empty()
                            || node.control.is_some()
                            || node.choice.is_some()
                    })
                {
                    return Err(ErrorCode::InvalidTree);
                }
                if (node.kind == Kind::Menu) != node.menu.is_some()
                    || node.menu.as_ref().is_some_and(|menu| {
                        !menu.is_valid()
                            || node.handler.is_some()
                            || node.control.is_some()
                            || node.choice.is_some()
                            || node.children.len()
                                != usize::from(menu.presentation == MenuPresentation::Context)
                    })
                {
                    return Err(ErrorCode::InvalidTree);
                }
                if (node.kind == Kind::CommandScope) != node.commands.is_some()
                    || (node.kind == Kind::CommandButton) != node.command_ref.is_some()
                    || node.commands.as_ref().is_some_and(|commands| {
                        !CommandConfig::registry_is_valid(commands) || node.handler.is_none()
                    })
                    || (node.kind == Kind::CommandButton
                        && (node.handler.is_some() || node.control.is_some()))
                {
                    return Err(ErrorCode::InvalidTree);
                }
                if (node.kind == Kind::FocusScope) != node.focus_scope.is_some() {
                    return Err(ErrorCode::InvalidTree);
                }
                if (node.kind == Kind::Tooltip) != node.tooltip.is_some()
                    || node
                        .tooltip
                        .as_ref()
                        .is_some_and(|config| !config.is_valid())
                    || (node.kind == Kind::Tooltip && node.children.len() != 2)
                {
                    return Err(ErrorCode::InvalidTree);
                }
                if node.placement.is_some() && node.overlay.is_none() && node.tooltip.is_none() {
                    return Err(ErrorCode::InvalidTree);
                }
                if let Some(config) = &node.overlay
                    && (node.kind != Kind::FocusScope
                        || !config.is_valid()
                        || node.handler.is_none()
                        || (config.kind == OverlayKind::Dialog
                            && !node.focus_scope.is_some_and(|scope| scope.trap))
                        || (config.kind == OverlayKind::Popover && Some(node.id) == plan.root))
                {
                    return Err(ErrorCode::InvalidTree);
                }
                match node.kind {
                    Kind::Input | Kind::Textarea | Kind::Combobox => {
                        let config = node.editor.as_ref().ok_or(ErrorCode::InvalidTree)?;
                        if node.handler.is_none()
                            || !config.is_valid()
                            || node.text.contains('\0')
                            || (matches!(node.kind, Kind::Input | Kind::Combobox)
                                && (config.min_rows != 1
                                    || config.max_rows != 1
                                    || node.text.contains(['\r', '\n'])))
                        {
                            return Err(ErrorCode::InvalidTree);
                        }
                    }
                    Kind::Container
                    | Kind::FocusScope
                    | Kind::Tooltip
                    | Kind::Toast
                    | Kind::ToastStack
                    | Kind::PointerArea
                    | Kind::DragSource
                    | Kind::DropTarget
                    | Kind::CommandScope
                    | Kind::CommandButton
                    | Kind::Menu
                    | Kind::CommandPalette
                    | Kind::Progress
                    | Kind::Text
                    | Kind::Button => {
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
            | Op::SetFocusScope(id, ..)
            | Op::SetOverlay(id, ..)
            | Op::SetPlacement(id, ..)
            | Op::SetTooltip(id, ..)
            | Op::SetCommands(id, ..)
            | Op::SetCommandRef(id, ..)
            | Op::SetMenu(id, ..)
            | Op::SetPalette(id, ..)
            | Op::SetProgress(id, ..)
            | Op::SetToast(id, ..)
            | Op::SetToastStack(id, ..)
            | Op::SetDragSource(id, ..)
            | Op::SetDropTarget(id, ..)
            | Op::SetPointer(id, ..)
            | Op::SetComboboxFilter(id, ..)
            | Op::SetChoiceAppearance(id, ..)
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
                            choice_appearance: None,
                            combobox_filter: None,
                            focus_scope: None,
                            overlay: None,
                            tooltip: None,
                            commands: None,
                            command_ref: None,
                            menu: None,
                            palette: None,
                            progress: None,
                            toast: None,
                            toast_stack: None,
                            drag_source: None,
                            drop_target: None,
                            pointer: None,
                            placement: None,
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
                if matches!(
                    self.node(*id)?.kind,
                    Kind::Input | Kind::Textarea | Kind::Combobox
                ) {
                    // Native editor contents change only through explicit commands.
                    return Err(ErrorCode::InvalidTree);
                }
                validate_text(text)?;
                self.node_mut(*id)?.text = Arc::from(text.as_str());
            }
            Op::SetEditor(id, config) => {
                if !matches!(
                    self.node(*id)?.kind,
                    Kind::Input | Kind::Textarea | Kind::Combobox
                ) || !config.is_valid()
                {
                    return Err(ErrorCode::InvalidTree);
                }
                self.node_mut(*id)?.editor = Some(Arc::new(config.clone()));
            }
            Op::SetToast(id, config) => {
                if self.node(*id)?.kind != Kind::Toast || !config.is_valid() {
                    return Err(ErrorCode::InvalidTree);
                }
                self.node_mut(*id)?.toast = Some(Arc::new(config.clone()));
            }
            Op::SetDragSource(id, config) => {
                if self.node(*id)?.kind != Kind::DragSource {
                    return Err(ErrorCode::InvalidTree);
                }
                self.node_mut(*id)?.drag_source = Some(Arc::new(config.clone()));
            }
            Op::SetDropTarget(id, config) => {
                if self.node(*id)?.kind != Kind::DropTarget {
                    return Err(ErrorCode::InvalidTree);
                }
                self.node_mut(*id)?.drop_target = Some(Arc::new(config.clone()));
            }
            Op::SetPointer(id, config) => {
                if self.node(*id)?.kind != Kind::PointerArea || !config.is_valid() {
                    return Err(ErrorCode::InvalidTree);
                }
                self.node_mut(*id)?.pointer = Some(Arc::new(config.clone()));
            }
            Op::SetToastStack(id, config) => {
                if self.node(*id)?.kind != Kind::ToastStack || !config.is_valid() {
                    return Err(ErrorCode::InvalidTree);
                }
                self.node_mut(*id)?.toast_stack = Some(Arc::new(config.clone()));
            }
            Op::SetProgress(id, config) => {
                if self.node(*id)?.kind != Kind::Progress || !config.is_valid() {
                    return Err(ErrorCode::InvalidTree);
                }
                self.node_mut(*id)?.progress = Some(Arc::new(config.clone()));
            }
            Op::SetPalette(id, config) => {
                if self.node(*id)?.kind != Kind::CommandPalette || !config.is_valid() {
                    return Err(ErrorCode::InvalidTree);
                }
                self.node_mut(*id)?.palette = Some(Arc::new(config.clone()));
                self.structural = true;
            }
            Op::SetMenu(id, config) => {
                if self.node(*id)?.kind != Kind::Menu || !config.is_valid() {
                    return Err(ErrorCode::InvalidTree);
                }
                self.node_mut(*id)?.menu = Some(Arc::new(config.clone()));
                self.structural = true;
            }
            Op::SetCommands(id, commands) => {
                if self.node(*id)?.kind != Kind::CommandScope
                    || !CommandConfig::registry_is_valid(commands)
                {
                    return Err(ErrorCode::InvalidTree);
                }
                self.node_mut(*id)?.commands = Some(Arc::from(commands.as_slice()));
                self.structural = true;
            }
            Op::SetCommandRef(id, command) => {
                if self.node(*id)?.kind != Kind::CommandButton
                    || !CommandConfig::valid_text(command, 256)
                {
                    return Err(ErrorCode::InvalidTree);
                }
                self.node_mut(*id)?.command_ref = Some(Arc::from(command.as_str()));
                self.structural = true;
            }
            Op::SetTooltip(id, config) => {
                if self.node(*id)?.kind != Kind::Tooltip || !config.is_valid() {
                    return Err(ErrorCode::InvalidTree);
                }
                self.node_mut(*id)?.tooltip = Some(Arc::new(config.clone()));
            }
            Op::SetPlacement(id, placement) => {
                if !matches!(self.node(*id)?.kind, Kind::FocusScope | Kind::Tooltip)
                    || placement.is_some_and(|placement| !placement.is_valid())
                {
                    return Err(ErrorCode::InvalidTree);
                }
                self.node_mut(*id)?.placement = *placement;
            }
            Op::SetOverlay(id, config) => {
                if self.node(*id)?.kind != Kind::FocusScope
                    || config.as_ref().is_some_and(|config| !config.is_valid())
                {
                    return Err(ErrorCode::InvalidTree);
                }
                self.node_mut(*id)?.overlay = config.clone().map(Arc::new);
            }
            Op::SetFocusScope(id, config) => {
                if self.node(*id)?.kind != Kind::FocusScope {
                    return Err(ErrorCode::InvalidTree);
                }
                self.node_mut(*id)?.focus_scope = Some(*config);
            }
            Op::SetComboboxFilter(id, filter) => {
                if self.node(*id)?.kind != Kind::Combobox {
                    return Err(ErrorCode::InvalidTree);
                }
                self.node_mut(*id)?.combobox_filter = Some(*filter);
            }
            Op::SetChoiceAppearance(id, appearance) => {
                if !matches!(
                    self.node(*id)?.kind,
                    Kind::Select | Kind::Combobox | Kind::Menu | Kind::CommandPalette
                ) {
                    return Err(ErrorCode::InvalidTree);
                }
                crate::appearance::validate(appearance)?;
                self.node_mut(*id)?.choice_appearance = Some(Arc::new(appearance.clone()));
            }
            Op::SetChoice(id, config) => {
                if !matches!(
                    self.node(*id)?.kind,
                    Kind::RadioGroup | Kind::Select | Kind::Combobox
                ) || !config.is_valid()
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
                if !allows_children(node.kind) {
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
            if parent.is_none()
                && node
                    .overlay
                    .as_ref()
                    .is_some_and(|config| config.kind == OverlayKind::Popover)
            {
                return Err(ErrorCode::InvalidTree);
            }
            if !allows_children(node.kind) && !node.children.is_empty() {
                return Err(ErrorCode::InvalidTree);
            }
            if node.kind == Kind::Toast
                && !parent.is_some_and(|parent| {
                    self.node(parent)
                        .is_ok_and(|node| node.kind == Kind::ToastStack)
                })
            {
                return Err(ErrorCode::InvalidTree);
            }
            if node.kind == Kind::ToastStack
                && (node.children.len() > MAX_TOASTS
                    || node
                        .children
                        .iter()
                        .any(|id| !self.node(*id).is_ok_and(|node| node.kind == Kind::Toast)))
            {
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
        let mut platform_menus = 0;
        for id in &seen {
            let node = self.node(*id)?;
            let mut references = node
                .menu
                .as_ref()
                .map_or_else(Vec::new, |menu| menu.command_ids());
            references.extend(node.command_ref.as_deref());
            if let Some(config) = &node.palette {
                references.extend(config.commands.iter().map(String::as_str));
            }
            if node
                .menu
                .as_ref()
                .is_some_and(|menu| menu.presentation == MenuPresentation::PlatformBar)
            {
                platform_menus += 1;
                if platform_menus > 1 {
                    return Err(ErrorCode::InvalidTree);
                }
            }
            for command in references {
                let mut cursor = Some(*id);
                let mut found = false;
                while let Some(id) = cursor {
                    let node = self.node(id)?;
                    if node
                        .commands
                        .as_ref()
                        .is_some_and(|commands| commands.iter().any(|entry| entry.id == command))
                    {
                        found = true;
                        break;
                    }
                    cursor = node.parent;
                }
                if !found {
                    return Err(ErrorCode::InvalidTree);
                }
            }
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
