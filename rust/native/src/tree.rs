use gpuio_protocol::{HandlerId, NodeId, WindowId, v1::*};
use std::{
    collections::{BTreeMap, BTreeSet},
    sync::Arc,
};

fn allows_children(kind: Kind) -> bool {
    matches!(
        kind,
        Kind::Container
            | Kind::TabPanel
            | Kind::Panel
            | Kind::Disclosure
            | Kind::Accordion
            | Kind::NavigationStack
            | Kind::Carousel
            | Kind::SplitPane
            | Kind::VirtualList
            | Kind::Animated
            | Kind::AnimationProgram
            | Kind::ContainerQuery
            | Kind::Button
            | Kind::CommandButton
            | Kind::FocusScope
            | Kind::Tooltip
            | Kind::HoverCard
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
pub struct SliderMount {
    pub config: Arc<gpuio_protocol::slider::Config>,
    pub initial: gpuio_protocol::slider::Value,
}

#[derive(Clone, Debug, PartialEq)]
pub struct NumberInputMount {
    pub config: Arc<gpuio_protocol::number_input::Config>,
    pub initial: gpuio_protocol::number_input::Value,
}

#[derive(Clone, Debug, PartialEq)]
pub struct OtpInputMount {
    pub config: Arc<gpuio_protocol::otp_input::Config>,
    pub initial: Arc<str>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ColorInputMount {
    pub config: Arc<gpuio_protocol::color_input::Config>,
    pub initial: gpuio_protocol::color_value::Value,
}

#[derive(Clone, Debug, PartialEq)]
pub struct CalendarMount {
    pub config: Arc<gpuio_protocol::calendar_input::Config>,
    pub initial: gpuio_protocol::calendar::Selection,
    pub initial_month: gpuio_protocol::calendar::Month,
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
    pub commands: Option<Arc<[Arc<CommandConfig>]>>,
    pub command_ref: Option<Arc<str>>,
    pub menu: Option<Arc<MenuConfig>>,
    pub palette: Option<Arc<PaletteConfig>>,
    pub progress: Option<Arc<ProgressConfig>>,
    pub loading: Option<Arc<gpuio_protocol::loading::Config>>,
    pub image: Option<Arc<ImageConfig>>,
    pub avatar: Option<Arc<gpuio_protocol::avatar::Config>>,
    pub rating: Option<Arc<gpuio_protocol::rating::Config>>,
    pub slider: Option<SliderMount>,
    pub number_input: Option<NumberInputMount>,
    pub otp_input: Option<OtpInputMount>,
    pub calendar: Option<CalendarMount>,
    pub color_input: Option<ColorInputMount>,
    pub extension: Option<Arc<gpuio_protocol::extension::Config>>,
    pub extension_command: Option<Arc<gpuio_protocol::extension::Command>>,
    pub split: Option<Arc<gpuio_protocol::split::Config>>,
    pub document: Option<Arc<gpuio_protocol::document::Config>>,
    pub canvas: Option<Arc<gpuio_protocol::canvas_view::Config>>,
    pub animation: Option<Arc<gpuio_protocol::animation::Config>>,
    pub animation_program: Option<Arc<gpuio_protocol::animation_program::Config>>,
    pub navigation_stack: Option<gpuio_protocol::navigation_stack::Config>,
    pub carousel: Option<Arc<gpuio_protocol::carousel::Config>>,
    pub tree_input: bool,
    pub container_query: Option<Arc<gpuio_protocol::container_query::Config>>,
    pub accessibility: Option<Arc<gpuio_protocol::accessibility::Config>>,
    pub list_config: Option<Arc<gpuio_protocol::list::Config>>,
    pub list_order: Option<Arc<gpuio_protocol::list::Order>>,
    pub list_index: Option<Arc<crate::list_index::Index>>,
    pub list_rows: Arc<[gpuio_protocol::list::Row]>,
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
        self.carousel
            .as_ref()
            .map_or(0, |config| config.retained_bytes())
            + self
                .color_input
                .as_ref()
                .map_or(0, |color| 32 + color.config.retained_bytes())
            + self
                .calendar
                .as_ref()
                .map_or(0, |calendar| 32 + calendar.config.retained_bytes())
            + self
                .otp_input
                .as_ref()
                .map_or(0, |s| s.config.retained_bytes() + s.initial.len())
            + self
                .number_input
                .as_ref()
                .map_or(0, |s| s.config.retained_bytes())
            + self
                .slider
                .as_ref()
                .map_or(0, |s| s.config.retained_bytes())
            + self.rating.as_ref().map_or(0, |c| c.retained_bytes())
            + self.avatar.as_ref().map_or(0, |c| c.retained_bytes())
            + self.loading.as_ref().map_or(0, |c| c.retained_bytes())
            + self.navigation_stack.map_or(0, |_| {
                std::mem::size_of::<gpuio_protocol::navigation_stack::Config>()
            })
            + self.text.len()
            + self
                .accessibility
                .as_ref()
                .map_or(0, |config| config.retained_bytes())
            + self
                .container_query
                .as_ref()
                .map_or(0, |config| config.retained_bytes())
            + self
                .animation_program
                .as_ref()
                .map_or(0, |config| config.retained_bytes())
            + self
                .canvas
                .as_ref()
                .map_or(0, |config| config.label.len() + 256)
            + self.extension.as_ref().map_or(0, |config| {
                256 + config.schema.name.len()
                    + config.schema.fingerprint.len()
                    + config.label.len()
                    + config.properties.0.len()
                    + config
                        .command
                        .as_ref()
                        .map_or(0, |command| command.payload.0.len())
            })
            + self
                .extension_command
                .as_ref()
                .map_or(0, |command| command.payload.0.len() + 32)
            + self
                .split
                .as_ref()
                .map_or(0, |config| config.label.len() + 128)
            + self.list_order.as_ref().map_or(0, |order| {
                // Admission units include expanded indexing and GPUI measurement
                // metadata, not merely the compact serialized runs. This is a
                // conservative quota, not a measurement of allocator RSS.
                order
                    .runs
                    .iter()
                    .map(|run| run.count as usize)
                    .sum::<usize>()
                    * 192
                    + std::mem::size_of_val(order.runs.as_slice())
            })
            + std::mem::size_of_val(self.list_rows.as_ref())
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
            + self
                .document
                .as_ref()
                .map_or(0, |config| config.retained_bytes())
            + self.image.as_ref().map_or(0, |config| {
                std::mem::size_of::<ImageConfig>() + config.label.as_ref().map_or(0, String::len)
            })
            + self.animation.as_ref().map_or(0, |config| {
                std::mem::size_of::<gpuio_protocol::animation::Config>()
                    + std::mem::size_of_val(config.targets.as_slice())
                    + config
                        .initial
                        .as_ref()
                        .map_or(0, |targets| std::mem::size_of_val(targets.as_slice()))
            })
            + self.progress.as_ref().map_or(0, |config| {
                std::mem::size_of::<ProgressConfig>() + config.label.len()
            })
            + self.commands.as_ref().map_or(0, |commands| {
                commands
                    .iter()
                    .map(|command| {
                        command.retained_bytes()
                            + std::mem::size_of::<Arc<CommandConfig>>()
                            + 2 * std::mem::size_of::<usize>()
                    })
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
    extension_count: usize,
    canvas_count: usize,
    retained_bytes: usize,
}

#[derive(Debug, PartialEq)]
pub struct Applied {
    pub revision: i64,
    pub touched_records: usize,
    pub validated_nodes: usize,
    pub dirty: Vec<NodeId>,
    pub lists: Vec<ListAction>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum ListAction {
    Invalidate(NodeId, Vec<i64>),
    Scroll(NodeId, gpuio_protocol::list::ScrollRequest),
}

impl Tree {
    pub fn new(window: WindowId) -> Self {
        Self {
            window,
            revision: 0,
            root: None,
            slots: Vec::new(),
            node_count: 0,
            extension_count: 0,
            canvas_count: 0,
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

    /// The only toggle in either supported disclosure header shape. Custom
    /// content is independent; no arbitrary descendant search can steal a link.
    pub fn disclosure_trigger(&self, id: NodeId) -> Option<NodeId> {
        let disclosure = self.get(id)?;
        if disclosure.kind != Kind::Disclosure {
            return None;
        }
        let header = self.get(*disclosure.children.first()?)?;
        let trigger = match header.kind {
            Kind::Button => header,
            Kind::Container => self.get(*header.children.last()?)?,
            _ => return None,
        };
        (trigger.kind == Kind::Button).then_some(trigger.id)
    }

    pub fn disclosure_for_trigger(&self, id: NodeId) -> Option<&Node> {
        let parent = self.get(self.get(id)?.parent?)?;
        let disclosure = if parent.kind == Kind::Container {
            self.get(parent.parent?)?
        } else {
            parent
        };
        (self.disclosure_trigger(disclosure.id) == Some(id)).then_some(disclosure)
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
                && node.kind == Kind::Tooltip
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
    pub fn command(&self, node: NodeId, command: &str) -> Option<(NodeId, &Arc<CommandConfig>)> {
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
        self.apply_guarded(tx, budget, &[])
            .map_err(ApplyFailure::rejection)
    }

    pub fn apply_guarded(
        &mut self,
        tx: &Transaction,
        budget: usize,
        pins: &[gpuio_protocol::list::Retained],
    ) -> Result<Applied, ApplyFailure> {
        self.apply_with_admission(tx, budget, pins, |_| Ok(()))
    }

    pub(crate) fn apply_with_admission(
        &mut self,
        tx: &Transaction,
        budget: usize,
        pins: &[gpuio_protocol::list::Retained],
        admit: impl FnOnce(&[ProgramChange]) -> Result<(), ErrorCode>,
    ) -> Result<Applied, ApplyFailure> {
        if tx.window != self.window {
            return Err(ErrorCode::StaleHandle.into());
        }
        if tx.base != self.revision || self.revision.checked_add(1) != Some(tx.revision) {
            return Err(ErrorCode::InvalidRevision.into());
        }
        if tx.operations.len() > MAX_OPERATIONS {
            return Err(ErrorCode::LimitExceeded.into());
        }
        let mut plan = Plan {
            original: self,
            changes: BTreeMap::new(),
            root: self.root,
            slot_count: self.slots.len(),
            node_count: self.node_count,
            extension_count: self.extension_count,
            canvas_count: self.canvas_count,
            retained_bytes: self.retained_bytes,
            budget: budget.min(MAX_RETAINED_BYTES),
            structural: false,
            lists: Vec::new(),
        };
        let mut extension_updates = BTreeSet::new();
        let mut canvas_updates = BTreeSet::new();
        let mut program_updates = BTreeSet::new();
        let mut query_updates = BTreeSet::new();
        for op in &tx.operations {
            if let Op::SetContainerQuery(id, _) = op
                && !query_updates.insert(*id)
            {
                return Err(ErrorCode::InvalidTree.into());
            }
            if let Op::SetAnimationProgram(id, _) = op
                && !program_updates.insert(*id)
            {
                return Err(ErrorCode::InvalidTree.into());
            }
            if let Op::SetExtension(id, _) = op
                && !extension_updates.insert(*id)
            {
                return Err(ErrorCode::InvalidTree.into());
            }
            if let Op::SetCanvas(id, _) = op
                && !canvas_updates.insert(*id)
            {
                // Each command must reach the mounted state; a later config in
                // this transaction cannot silently replace an earlier one.
                return Err(ErrorCode::InvalidTree.into());
            }
            plan.operation(op)?;
        }
        // A viewport-only update must also validate its unchanged carousel owner.
        // Include it in dirty output so owner scheduling sees the admitted snapshot.
        let carousel_parents = plan
            .changes
            .values()
            .filter_map(|slot| {
                let parent = slot.node.as_ref()?.parent?;
                plan.node(parent).ok()?.carousel.as_ref()?;
                Some(parent)
            })
            .collect::<BTreeSet<_>>();
        for parent in carousel_parents {
            plan.node_mut(parent)?;
        }
        for slot in plan.changes.values() {
            if let Some(node) = &slot.node {
                plan.validate_list(node)?;
                if node.choice_appearance.is_some()
                    && !matches!(
                        node.kind,
                        Kind::Select | Kind::Combobox | Kind::Menu | Kind::CommandPalette
                    )
                {
                    return Err(ErrorCode::InvalidTree.into());
                }
                if node.choice.is_some()
                    && !matches!(
                        node.kind,
                        Kind::RadioGroup | Kind::TabBar | Kind::Select | Kind::Combobox
                    )
                {
                    return Err(ErrorCode::InvalidTree.into());
                }
                if node
                    .control
                    .is_some_and(|control| control.kind() != node.kind)
                {
                    return Err(ErrorCode::InvalidTree.into());
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
                        return Err(ErrorCode::InvalidTree.into());
                    }
                } else if node.combobox_filter.is_some() {
                    return Err(ErrorCode::InvalidTree.into());
                }
                if (node.kind == Kind::DragSource) != node.drag_source.is_some()
                    || (node.kind == Kind::DropTarget) != node.drop_target.is_some()
                    || ((node.drag_source.is_some() || node.drop_target.is_some())
                        && (node.handler.is_none()
                            || !node.text.is_empty()
                            || node.control.is_some()
                            || node.choice.is_some()))
                {
                    return Err(ErrorCode::InvalidTree.into());
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
                    return Err(ErrorCode::InvalidTree.into());
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
                    return Err(ErrorCode::InvalidTree.into());
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
                    return Err(ErrorCode::InvalidTree.into());
                }
                if (node.kind == Kind::ContainerQuery) != node.container_query.is_some()
                    || node.container_query.as_ref().is_some_and(|config| {
                        !config.is_valid()
                            || !node.text.is_empty()
                            || node.children.len() != config.branches.len()
                            || node.control.is_some()
                            || node.choice.is_some()
                    })
                {
                    return Err(ErrorCode::InvalidTree.into());
                }
                if (node.kind == Kind::AnimationProgram) != node.animation_program.is_some()
                    || node.animation_program.as_ref().is_some_and(|config| {
                        !config.is_valid()
                            || !node.text.is_empty()
                            || node.control.is_some()
                            || node.choice.is_some()
                    })
                {
                    return Err(ErrorCode::InvalidTree.into());
                }
                if (node.kind == Kind::Animated) != node.animation.is_some()
                    || node.animation.as_ref().is_some_and(|config| {
                        !config.is_valid()
                            || !node.text.is_empty()
                            || node.control.is_some()
                            || node.choice.is_some()
                    })
                {
                    return Err(ErrorCode::InvalidTree.into());
                }
                if (matches!(node.kind, Kind::Image | Kind::Icon)
                    || node.avatar.as_ref().is_some_and(|c| c.source.is_some()))
                    != node.image.is_some()
                    || node.image.as_ref().is_some_and(|config| {
                        !config.is_valid()
                            || !node.text.is_empty()
                            || !node.children.is_empty()
                            || node.control.is_some()
                            || node.choice.is_some()
                    })
                {
                    return Err(ErrorCode::InvalidTree.into());
                }
                if (node.kind == Kind::Slider) != node.slider.is_some()
                    || node.slider.as_ref().is_some_and(|slider| {
                        !slider.config.is_valid()
                            || !slider.initial.is_valid()
                            || !node.text.is_empty()
                            || !node.children.is_empty()
                            || node.control.is_some()
                            || node.choice.is_some()
                            || node.handler.is_none()
                    })
                {
                    return Err(ErrorCode::InvalidTree.into());
                }
                if (node.kind == Kind::ColorInput) != node.color_input.is_some()
                    || node.color_input.as_ref().is_some_and(|color| {
                        !color.config.is_valid()
                            || !node.text.is_empty()
                            || !node.children.is_empty()
                            || node.control.is_some()
                            || node.choice.is_some()
                            || node.handler.is_none()
                    })
                {
                    return Err(ErrorCode::InvalidTree.into());
                }
                if (node.kind == Kind::Calendar) != node.calendar.is_some()
                    || node.calendar.as_ref().is_some_and(|calendar| {
                        !calendar.config.is_valid()
                            || !calendar.initial.fits(calendar.config.mode)
                            || !node.text.is_empty()
                            || !node.children.is_empty()
                            || node.control.is_some()
                            || node.choice.is_some()
                            || node.handler.is_none()
                    })
                {
                    return Err(ErrorCode::InvalidTree.into());
                }
                if (node.kind == Kind::OtpInput) != node.otp_input.is_some()
                    || node.otp_input.as_ref().is_some_and(|input| {
                        !input.config.is_valid()
                            || !input.config.policy.canonical(&input.initial)
                            || !node.text.is_empty()
                            || !node.children.is_empty()
                            || node.control.is_some()
                            || node.choice.is_some()
                            || node.handler.is_none()
                    })
                {
                    return Err(ErrorCode::InvalidTree.into());
                }
                if (node.kind == Kind::NumberInput) != node.number_input.is_some()
                    || node.number_input.as_ref().is_some_and(|number_input| {
                        !number_input.config.is_valid()
                            || !number_input.initial.is_valid()
                            || !node.text.is_empty()
                            || !node.children.is_empty()
                            || node.control.is_some()
                            || node.choice.is_some()
                            || node.handler.is_none()
                    })
                {
                    return Err(ErrorCode::InvalidTree.into());
                }
                if (node.kind == Kind::Rating) != node.rating.is_some()
                    || node.rating.as_ref().is_some_and(|config| {
                        !config.is_valid()
                            || !node.text.is_empty()
                            || !node.children.is_empty()
                            || node.control.is_some()
                            || node.choice.is_some()
                            || (!config.disabled && !config.read_only && node.handler.is_none())
                    })
                {
                    return Err(ErrorCode::InvalidTree.into());
                }
                if (node.kind == Kind::Avatar) != node.avatar.is_some()
                    || node.avatar.as_ref().is_some_and(|config| {
                        !config.is_valid()
                            || !config.matches_image(node.image.as_deref())
                            || !node.text.is_empty()
                            || !node.children.is_empty()
                            || node.control.is_some()
                            || node.choice.is_some()
                            || (config.source.is_none() && node.handler.is_some())
                    })
                {
                    return Err(ErrorCode::InvalidTree.into());
                }
                if (node.kind == Kind::Extension) != node.extension.is_some()
                    || node.extension.as_ref().is_some_and(|config| {
                        !config.is_valid()
                            || !node.text.is_empty()
                            || !node.children.is_empty()
                            || node.handler.is_none()
                    })
                {
                    return Err(ErrorCode::InvalidTree.into());
                }
                if (node.kind == Kind::SplitPane) != node.split.is_some()
                    || node
                        .split
                        .as_ref()
                        .is_some_and(|config| !config.is_valid() || node.children.len() != 2)
                {
                    return Err(ErrorCode::InvalidTree.into());
                }
                if (node.kind == Kind::CanvasView) != node.canvas.is_some()
                    || node.canvas.as_ref().is_some_and(|config| {
                        !config.is_valid() || !node.text.is_empty() || !node.children.is_empty()
                    })
                {
                    return Err(ErrorCode::InvalidTree.into());
                }
                if (node.kind == Kind::DocumentView) != node.document.is_some()
                    || node.document.as_ref().is_some_and(|config| {
                        !config.is_valid() || !node.text.is_empty() || !node.children.is_empty()
                    })
                {
                    return Err(ErrorCode::InvalidTree.into());
                }
                if (node.kind == Kind::Loading) != node.loading.is_some()
                    || node.loading.as_ref().is_some_and(|config| {
                        !config.is_valid()
                            || node.handler.is_some()
                            || !node.text.is_empty()
                            || !node.children.is_empty()
                            || node.control.is_some()
                            || node.choice.is_some()
                    })
                {
                    return Err(ErrorCode::InvalidTree.into());
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
                    return Err(ErrorCode::InvalidTree.into());
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
                    return Err(ErrorCode::InvalidTree.into());
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
                    return Err(ErrorCode::InvalidTree.into());
                }
                if (node.kind == Kind::CommandScope) != node.commands.is_some()
                    || (node.kind == Kind::CommandButton) != node.command_ref.is_some()
                    || node.commands.as_ref().is_some_and(|commands| {
                        !CommandConfig::registry_entries_are_valid(commands.iter().map(Arc::as_ref))
                            || node.handler.is_none()
                    })
                    || (node.kind == Kind::CommandButton
                        && (node.handler.is_some() || node.control.is_some()))
                {
                    return Err(ErrorCode::InvalidTree.into());
                }
                if (node.kind == Kind::FocusScope) != node.focus_scope.is_some() {
                    return Err(ErrorCode::InvalidTree.into());
                }
                if matches!(node.kind, Kind::Tooltip | Kind::HoverCard) != node.tooltip.is_some()
                    || node
                        .tooltip
                        .as_ref()
                        .is_some_and(|config| !config.is_valid())
                    || (matches!(node.kind, Kind::Tooltip | Kind::HoverCard)
                        && node.children.len() != 2)
                    || (node.kind == Kind::HoverCard
                        && node
                            .tooltip
                            .as_ref()
                            .is_some_and(|config| !config.hoverable || config.skip_delay_ns != 0))
                {
                    return Err(ErrorCode::InvalidTree.into());
                }
                if node.placement.is_some() && node.overlay.is_none() && node.tooltip.is_none() {
                    return Err(ErrorCode::InvalidTree.into());
                }
                if let Some(config) = &node.overlay
                    && (node.kind != Kind::FocusScope
                        || !config.is_valid()
                        || node.handler.is_none()
                        || (config.kind.is_modal()
                            && !node.focus_scope.is_some_and(|scope| scope.trap))
                        || (config.kind == OverlayKind::Popover && Some(node.id) == plan.root))
                {
                    return Err(ErrorCode::InvalidTree.into());
                }
                if node.tree_input
                    && (node.kind != Kind::VirtualList
                        || node.handler.is_none()
                        || !node.accessibility.as_ref().is_some_and(|metadata| {
                            matches!(
                                metadata.role,
                                Some(gpuio_protocol::accessibility::Role::Tree(_))
                            )
                        }))
                {
                    return Err(ErrorCode::InvalidTree.into());
                }
                if (node.kind == Kind::Carousel) != node.carousel.is_some()
                    || node.carousel.as_ref().is_some_and(|config| {
                        !config.is_valid()
                            || node.handler.is_none()
                            || node.text.is_empty()
                            || node.text.len() > 4096
                            || node.text.contains('\0')
                            || !(1..=2).contains(&node.children.len())
                            || node.children.first().is_none_or(|child| {
                                plan.node(*child).map_or(true, |viewport| {
                                    viewport.kind != Kind::NavigationStack
                                        || viewport.children.len() != config.ids.len()
                                        || viewport.navigation_stack.is_none_or(|presentation| {
                                            presentation.selected != config.selected
                                        })
                                })
                            })
                    })
                {
                    return Err(ErrorCode::InvalidTree.into());
                }
                if (node.kind == Kind::NavigationStack) != node.navigation_stack.is_some()
                    || node.navigation_stack.is_some_and(|config| {
                        !config.valid_children(node.children.len())
                            || node.handler.is_some()
                            || node.text.is_empty()
                            || node.text.len() > 4096
                            || node.text.contains('\0')
                            || node.children.iter().enumerate().any(|(index, child)| {
                                plan.node(*child).map_or(true, |child| {
                                    child.kind != Kind::Panel
                                        || (!config.retain
                                            && config.selected != Some(index as i64)
                                            && !child.children.is_empty())
                                })
                            })
                    })
                {
                    return Err(ErrorCode::InvalidTree.into());
                }
                if node.kind == Kind::Panel
                    && (node.text.is_empty() || node.text.len() > 4096 || node.text.contains('\0'))
                {
                    return Err(ErrorCode::InvalidTree.into());
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
                            return Err(ErrorCode::InvalidTree.into());
                        }
                    }
                    Kind::Container
                    | Kind::FocusScope
                    | Kind::Tooltip
                    | Kind::HoverCard
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
                    | Kind::Loading
                    | Kind::Image
                    | Kind::Avatar
                    | Kind::Rating
                    | Kind::Slider
                    | Kind::NumberInput
                    | Kind::OtpInput
                    | Kind::Calendar
                    | Kind::ColorInput
                    | Kind::TabPanel
                    | Kind::Panel
                    | Kind::Disclosure
                    | Kind::Accordion
                    | Kind::NavigationStack
                    | Kind::Carousel
                    | Kind::SplitPane
                    | Kind::Extension
                    | Kind::CanvasView
                    | Kind::DocumentView
                    | Kind::Icon
                    | Kind::Animated
                    | Kind::AnimationProgram
                    | Kind::ContainerQuery
                    | Kind::VirtualList
                    | Kind::Text
                    | Kind::Button => {
                        if node.editor.is_some() {
                            return Err(ErrorCode::InvalidTree.into());
                        }
                    }
                    Kind::Checkbox | Kind::Switch => {
                        let control = node.control.ok_or(ErrorCode::InvalidTree)?;
                        if node.editor.is_some()
                            || (!control.disabled() && node.handler.is_none())
                            || node.text.contains('\0')
                        {
                            return Err(ErrorCode::InvalidTree.into());
                        }
                    }
                    Kind::RadioGroup | Kind::TabBar | Kind::Select => {
                        let config = node.choice.as_ref().ok_or(ErrorCode::InvalidTree)?;
                        if !config.is_valid()
                            || node.editor.is_some()
                            || (!config.disabled && node.handler.is_none())
                        {
                            return Err(ErrorCode::InvalidTree.into());
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
        for id in &dirty {
            plan.validate_button_icons(*id)?;
        }
        if let Some(root) = plan.root {
            dirty.insert(root);
        }
        let touched_records = plan.changes.len();
        for action in &plan.lists {
            plan.validate_list_action(action)?;
        }
        let retained: Vec<_> = pins
            .iter()
            .filter_map(|pin| {
                let node = plan.node(pin.node).ok()?;
                let index = node.list_index.as_ref()?;
                let mounted: BTreeSet<_> = node.list_rows.iter().map(|row| row.id).collect();
                let rows: Vec<_> = pin
                    .rows
                    .iter()
                    .copied()
                    .filter(|id| index.position(*id).is_some() && !mounted.contains(id))
                    .collect();
                (!rows.is_empty()).then_some(gpuio_protocol::list::Retained {
                    node: pin.node,
                    rows,
                })
            })
            .collect();
        if retained
            .iter()
            .map(|notice| notice.rows.len())
            .sum::<usize>()
            > gpuio_protocol::list::MAX_ACTIVE_ROWS
        {
            return Err(ErrorCode::LimitExceeded.into());
        }
        if !retained.is_empty() {
            return Err(ApplyFailure::Retained(retained));
        }
        let mut programs = Vec::new();
        for (index, slot) in &plan.changes {
            let before = self.slots.get(*index).and_then(|s| s.node.as_ref());
            let after = slot.node.as_ref();
            if before.and_then(|n| n.animation_program.as_ref())
                == after.and_then(|n| n.animation_program.as_ref())
                && before.map(|n| n.id) == after.map(|n| n.id)
            {
                continue;
            }
            if let Some(node) = before.filter(|n| n.animation_program.is_some()) {
                programs.push((node.id, None));
            }
            if let Some(node) = after
                && let Some(config) = &node.animation_program
            {
                programs.push((node.id, Some(config.clone())));
            }
        }
        let Plan {
            changes,
            root,
            slot_count,
            node_count,
            extension_count,
            canvas_count,
            retained_bytes,
            lists,
            ..
        } = plan;
        // All validation has succeeded. Reserve before mutating semantic state.
        self.slots
            .try_reserve(slot_count - self.slots.len())
            .map_err(|_| ErrorCode::LimitExceeded)?;
        admit(&programs)?;
        self.slots.resize_with(slot_count, Slot::default);
        for (index, slot) in changes {
            self.slots[index] = slot;
        }
        self.root = root;
        self.node_count = node_count;
        self.extension_count = extension_count;
        self.canvas_count = canvas_count;
        self.retained_bytes = retained_bytes;
        self.revision = tx.revision;
        Ok(Applied {
            revision: tx.revision,
            touched_records,
            validated_nodes,
            dirty: dirty.into_iter().collect(),
            lists,
        })
    }
}

pub(crate) type ProgramChange = (
    NodeId,
    Option<Arc<gpuio_protocol::animation_program::Config>>,
);

/// Invalid transactions and stale row-eviction attempts have different retry semantics.
#[derive(Debug, PartialEq, Eq)]
pub enum ApplyFailure {
    Rejected(ErrorCode),
    Retained(Vec<gpuio_protocol::list::Retained>),
}
impl From<ErrorCode> for ApplyFailure {
    fn from(error: ErrorCode) -> Self {
        Self::Rejected(error)
    }
}
impl ApplyFailure {
    // Existing unguarded callers pass no pins, so Retained is unreachable there.
    pub(crate) fn rejection(self) -> ErrorCode {
        match self {
            Self::Rejected(error) => error,
            Self::Retained(_) => ErrorCode::InvalidTree,
        }
    }
}

struct Plan<'a> {
    original: &'a Tree,
    changes: BTreeMap<usize, Slot>,
    root: Option<NodeId>,
    slot_count: usize,
    node_count: usize,
    extension_count: usize,
    canvas_count: usize,
    retained_bytes: usize,
    budget: usize,
    structural: bool,
    lists: Vec<ListAction>,
}

impl Plan<'_> {
    fn list_contains(index: &crate::list_index::Index, mut ids: impl Iterator<Item = i64>) -> bool {
        ids.all(|id| index.position(id).is_some())
    }

    fn validate_list(&self, node: &Node) -> Result<(), ErrorCode> {
        if node.kind != Kind::VirtualList {
            return if node.list_config.is_none()
                && node.list_order.is_none()
                && node.list_index.is_none()
                && node.list_rows.is_empty()
            {
                Ok(())
            } else {
                Err(ErrorCode::InvalidTree)
            };
        }
        let config = node.list_config.as_ref().ok_or(ErrorCode::InvalidTree)?;
        let order = node.list_order.as_ref().ok_or(ErrorCode::InvalidTree)?;
        let index = node.list_index.as_ref().ok_or(ErrorCode::InvalidTree)?;
        if !config.is_valid()
            || !order.is_valid()
            || !node.text.is_empty()
            || (config.managed && node.list_rows.len() > config.max_active as usize)
            || (!config.managed && node.list_rows.len() != index.len())
            || node.list_rows.len() != node.children.len()
            || !Self::list_contains(index, node.list_rows.iter().map(|row| row.id))
        {
            return Err(ErrorCode::InvalidTree);
        }
        let mut ids = BTreeSet::new();
        let mut children = BTreeSet::new();
        for row in node.list_rows.iter() {
            if !ids.insert(row.id) || !children.insert(row.node) {
                return Err(ErrorCode::InvalidTree);
            }
        }
        if children != node.children.iter().copied().collect() {
            return Err(ErrorCode::InvalidTree);
        }
        Ok(())
    }

    fn validate_list_action(&self, action: &ListAction) -> Result<(), ErrorCode> {
        let id = match action {
            ListAction::Invalidate(id, _) | ListAction::Scroll(id, _) => *id,
        };
        let node = self.node(id)?;
        if node.kind != Kind::VirtualList {
            return Err(ErrorCode::InvalidTree);
        }
        let index = node.list_index.as_ref().ok_or(ErrorCode::InvalidTree)?;
        match action {
            ListAction::Invalidate(_, ids) => {
                if ids.len() > gpuio_protocol::list::MAX_LOGICAL_ROWS
                    || !Self::list_contains(index, ids.iter().copied())
                {
                    return Err(ErrorCode::InvalidTree);
                }
            }
            ListAction::Scroll(_, request) => {
                use gpuio_protocol::list::ScrollTarget;
                if request.serial < 1 {
                    return Err(ErrorCode::InvalidTree);
                }
                let row = match request.target {
                    ScrollTarget::Offset(row, offset) => {
                        if !offset.is_finite() || !(0.0..=1_000_000.0).contains(&offset) {
                            return Err(ErrorCode::InvalidTree);
                        }
                        Some(row)
                    }
                    ScrollTarget::Reveal(row) => Some(row),
                    ScrollTarget::FocusTreeRow(row) => {
                        if !node.tree_input {
                            return Err(ErrorCode::InvalidTree);
                        }
                        Some(row)
                    }
                    ScrollTarget::End => None,
                };
                if !Self::list_contains(index, row.into_iter()) {
                    return Err(ErrorCode::InvalidTree);
                }
            }
        }
        Ok(())
    }

    // Buttons retain one action/focus target. Their optional children represent
    // two fixed decorative icon slots, never nested controls or callbacks.
    // Run for dirty ancestors too: Bind/SetImage can invalidate a slot without
    // changing the structural edges.
    fn validate_button_icons(&self, id: NodeId) -> Result<(), ErrorCode> {
        let node = self.node(id)?;
        if !matches!(node.kind, Kind::Button | Kind::CommandButton) || node.children.is_empty() {
            return Ok(());
        }
        if node.children.len() != 2 {
            return Err(ErrorCode::InvalidTree);
        }
        for slot in node.children.iter() {
            let slot = self.node(*slot)?;
            if slot.kind != Kind::Container
                || slot.handler.is_some()
                || !slot.text.is_empty()
                || slot.children.len() > 1
            {
                return Err(ErrorCode::InvalidTree);
            }
            if let Some(icon) = slot.children.first() {
                let icon = self.node(*icon)?;
                if icon.kind != Kind::Icon
                    || icon.handler.is_some()
                    || !icon
                        .image
                        .as_ref()
                        .is_some_and(|image| image.label.is_none())
                {
                    return Err(ErrorCode::InvalidTree);
                }
            }
        }
        Ok(())
    }
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
        if matches!(op, Op::InvalidateListRows(..) | Op::ScrollList(..)) {
            return self.operation_inner(op);
        }
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
            | Op::SetAnimation(id, ..)
            | Op::SetAnimationProgram(id, ..)
            | Op::SetNavigationStack(id, ..)
            | Op::SetTreeInput(id, ..)
            | Op::SetCarousel(id, ..)
            | Op::SetContainerQuery(id, ..)
            | Op::SetAccessibility(id, ..)
            | Op::SetLoading(id, ..)
            | Op::SetAvatar(id, ..)
            | Op::SetRating(id, ..)
            | Op::SetSlider(id, ..)
            | Op::SetNumberInput(id, ..)
            | Op::SetOtpInput(id, ..)
            | Op::SetColorInput(id, ..)
            | Op::SetCalendar(id, ..)
            | Op::SetListConfig(id, ..)
            | Op::SetListOrder(id, ..)
            | Op::SetListRows(id, ..)
            | Op::InvalidateListRows(id, ..)
            | Op::ScrollList(id, ..)
            | Op::SetImage(id, ..)
            | Op::SetCanvas(id, ..)
            | Op::SetDocument(id, ..)
            | Op::SetExtension(id, ..)
            | Op::SetSplit(id, ..)
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
                if *kind == Kind::CanvasView {
                    if self.canvas_count == 128 {
                        return Err(ErrorCode::LimitExceeded);
                    }
                    self.canvas_count += 1;
                }
                if *kind == Kind::Extension {
                    if self.extension_count == 256 {
                        return Err(ErrorCode::LimitExceeded);
                    }
                    self.extension_count += 1;
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
                            loading: None,
                            image: None,
                            avatar: None,
                            rating: None,
                            slider: None,
                            number_input: None,
                            otp_input: None,
                            calendar: None,
                            color_input: None,
                            extension: None,
                            extension_command: None,
                            split: None,
                            document: None,
                            canvas: None,
                            animation: None,
                            animation_program: None,
                            navigation_stack: None,
                            carousel: None,
                            tree_input: false,
                            container_query: None,
                            accessibility: None,
                            list_config: None,
                            list_order: None,
                            list_index: None,
                            list_rows: Arc::from([]),
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
                if self.node(*id)?.kind == Kind::CanvasView {
                    self.canvas_count -= 1;
                }
                if self.node(*id)?.kind == Kind::Extension {
                    self.extension_count -= 1;
                }
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
            Op::SetTreeInput(id, enabled) => {
                if self.node(*id)?.kind != Kind::VirtualList {
                    return Err(ErrorCode::InvalidTree);
                }
                self.node_mut(*id)?.tree_input = *enabled;
            }
            Op::SetCarousel(id, config) => {
                let node = self.node(*id)?;
                if node.kind != Kind::Carousel
                    || !config.is_valid()
                    || node
                        .carousel
                        .as_ref()
                        .is_some_and(|old| !config.can_replace(old))
                {
                    return Err(ErrorCode::InvalidTree);
                }
                self.node_mut(*id)?.carousel = Some(Arc::new(config.clone()));
            }
            Op::SetNavigationStack(id, config) => {
                if self.node(*id)?.kind != Kind::NavigationStack || !config.is_valid() {
                    return Err(ErrorCode::InvalidTree);
                }
                self.node_mut(*id)?.navigation_stack = Some(*config);
            }
            Op::SetContainerQuery(id, config) => {
                let node = self.node(*id)?;
                if node.kind != Kind::ContainerQuery
                    || !config.is_valid()
                    || node.container_query.as_ref().is_some_and(|old| {
                        config != old.as_ref() && config.generation <= old.generation
                    })
                {
                    return Err(ErrorCode::InvalidTree);
                }
                self.node_mut(*id)?.container_query = Some(Arc::new(config.clone()));
            }
            Op::SetAccessibility(id, config) => {
                let kind = self.node(*id)?.kind;
                if config
                    .as_ref()
                    .is_some_and(|c| !c.is_valid() || !c.supports(kind))
                {
                    return Err(ErrorCode::InvalidTree);
                }
                self.node_mut(*id)?.accessibility = config.clone().map(Arc::new);
            }
            Op::SetAnimationProgram(id, config) => {
                let node = self.node(*id)?;
                if node.kind != Kind::AnimationProgram
                    || !config.is_valid()
                    || node.animation_program.as_ref().is_some_and(|old| {
                        (config != old.as_ref() && config.generation <= old.generation)
                            || (config.program == old.program && config.restart < old.restart)
                    })
                {
                    return Err(ErrorCode::InvalidTree);
                }
                self.node_mut(*id)?.animation_program = Some(Arc::new(config.clone()));
            }
            Op::SetAnimation(id, config) => {
                let node = self.node(*id)?;
                if node.kind != Kind::Animated
                    || !config.is_valid()
                    || node.animation.as_ref().is_some_and(|old| {
                        config != old.as_ref() && config.generation <= old.generation
                    })
                {
                    return Err(ErrorCode::InvalidTree);
                }
                self.node_mut(*id)?.animation = Some(Arc::new(config.clone()));
                self.structural = true;
            }
            Op::SetListConfig(id, config) => {
                if self.node(*id)?.kind != Kind::VirtualList || !config.is_valid() {
                    return Err(ErrorCode::InvalidTree);
                }
                self.node_mut(*id)?.list_config = Some(Arc::new(config.clone()));
            }
            Op::SetListOrder(id, order) => {
                let node = self.node(*id)?;
                if node.kind != Kind::VirtualList
                    || !order.is_valid()
                    || node
                        .list_order
                        .as_ref()
                        .is_some_and(|old| order != old.as_ref() && order.revision <= old.revision)
                {
                    return Err(ErrorCode::InvalidTree);
                }
                if node.list_order.as_deref() == Some(order) {
                    return Ok(());
                }
                let old_bytes = node.list_order.as_ref().map_or(0, |old| {
                    old.runs.iter().map(|run| run.count as usize).sum::<usize>() * 192
                        + std::mem::size_of_val(old.runs.as_slice())
                });
                let new_bytes = order
                    .runs
                    .iter()
                    .map(|run| run.count as usize)
                    .sum::<usize>()
                    * 192
                    + std::mem::size_of_val(order.runs.as_slice());
                if self.retained_bytes - old_bytes + new_bytes > self.budget {
                    return Err(ErrorCode::LimitExceeded);
                }
                let index =
                    crate::list_index::Index::new(order).map_err(|_| ErrorCode::InvalidTree)?;
                let node = self.node_mut(*id)?;
                node.list_order = Some(Arc::new(order.clone()));
                node.list_index = Some(Arc::new(index));
            }
            Op::SetListRows(id, rows) => {
                if self.node(*id)?.kind != Kind::VirtualList || rows.len() > MAX_NODES {
                    return Err(ErrorCode::InvalidTree);
                }
                self.node_mut(*id)?.list_rows = Arc::from(rows.clone());
            }
            Op::InvalidateListRows(id, rows) => {
                self.lists.push(ListAction::Invalidate(*id, rows.clone()));
            }
            Op::ScrollList(id, request) => {
                self.lists.push(ListAction::Scroll(*id, *request));
            }
            Op::SetSlider(id, config, initial) => {
                let node = self.node(*id)?;
                if node.kind != Kind::Slider
                    || !config.is_valid()
                    || !initial.is_valid()
                    || node
                        .slider
                        .as_ref()
                        .is_some_and(|old| !old.initial.same_mode(*initial))
                {
                    return Err(ErrorCode::InvalidTree);
                }
                self.node_mut(*id)?.slider = Some(SliderMount {
                    config: Arc::new(config.clone()),
                    initial: *initial,
                });
            }
            Op::SetColorInput(id, config, initial) => {
                let node = self.node(*id)?;
                if node.kind != Kind::ColorInput
                    || !config.is_valid()
                    || (node.color_input.is_none() && !config.allows(*initial))
                {
                    return Err(ErrorCode::InvalidTree);
                }
                let initial = node
                    .color_input
                    .as_ref()
                    .map_or(*initial, |old| old.initial);
                self.node_mut(*id)?.color_input = Some(ColorInputMount {
                    config: Arc::new(config.as_ref().clone()),
                    initial,
                });
            }
            Op::SetCalendar(id, config, initial, initial_month) => {
                let node = self.node(*id)?;
                if node.kind != Kind::Calendar
                    || !config.is_valid()
                    || !initial.fits(config.mode)
                    || node
                        .calendar
                        .as_ref()
                        .is_some_and(|old| old.config.mode != config.mode)
                    || (node.calendar.is_none()
                        && !config.constraints.allows_selection(*initial, config.mode))
                {
                    return Err(ErrorCode::InvalidTree);
                }
                let (initial, initial_month) = node
                    .calendar
                    .as_ref()
                    .map_or((*initial, *initial_month), |old| {
                        (old.initial, old.initial_month)
                    });
                self.node_mut(*id)?.calendar = Some(CalendarMount {
                    config: Arc::new(config.as_ref().clone()),
                    initial,
                    initial_month,
                });
            }
            Op::SetOtpInput(id, config, initial) => {
                let node = self.node(*id)?;
                if node.kind != Kind::OtpInput
                    || !config.is_valid()
                    || !config.policy.canonical(initial)
                    || node
                        .otp_input
                        .as_ref()
                        .is_some_and(|old| old.config.policy != config.policy)
                {
                    return Err(ErrorCode::InvalidTree);
                }
                // Rerendered seeds never replace an existing placement's original seed.
                let initial = node
                    .otp_input
                    .as_ref()
                    .map_or_else(|| Arc::from(initial.as_str()), |old| old.initial.clone());
                self.node_mut(*id)?.otp_input = Some(OtpInputMount {
                    config: Arc::new(config.clone()),
                    initial,
                });
            }
            Op::SetNumberInput(id, config, initial) => {
                if self.node(*id)?.kind != Kind::NumberInput
                    || !config.is_valid()
                    || !initial.is_valid()
                {
                    return Err(ErrorCode::InvalidTree);
                }
                self.node_mut(*id)?.number_input = Some(NumberInputMount {
                    config: Arc::new(config.clone()),
                    initial: *initial,
                });
            }
            Op::SetRating(id, config) => {
                if self.node(*id)?.kind != Kind::Rating || !config.is_valid() {
                    return Err(ErrorCode::InvalidTree);
                }
                self.node_mut(*id)?.rating = Some(Arc::new(config.clone()));
            }
            Op::SetAvatar(id, config) => {
                if self.node(*id)?.kind != Kind::Avatar || !config.is_valid() {
                    return Err(ErrorCode::InvalidTree);
                }
                let node = self.node_mut(*id)?;
                node.image = config.image().map(Arc::new);
                node.avatar = Some(Arc::new(config.clone()));
            }
            Op::SetImage(id, config) => {
                if !matches!(self.node(*id)?.kind, Kind::Image | Kind::Icon) || !config.is_valid() {
                    return Err(ErrorCode::InvalidTree);
                }
                self.node_mut(*id)?.image = Some(Arc::new(config.clone()));
            }
            Op::SetExtension(id, config) => {
                let node = self.node(*id)?;
                if node.kind != Kind::Extension
                    || !config.is_valid()
                    || node.extension.as_ref().is_some_and(|old| {
                        config.schema != old.schema || config.generation < old.generation
                    })
                {
                    return Err(ErrorCode::InvalidTree);
                }
                let same_generation = node
                    .extension
                    .as_ref()
                    .is_some_and(|old| old.generation == config.generation);
                if same_generation
                    && let (Some(old), Some(next)) = (&node.extension_command, &config.command)
                    && (next.sequence < old.sequence
                        || (next.sequence == old.sequence && next != old.as_ref()))
                {
                    return Err(ErrorCode::InvalidTree);
                }
                let node = self.node_mut(*id)?;
                if !same_generation {
                    node.extension_command = None;
                }
                if let Some(command) = &config.command {
                    node.extension_command = Some(Arc::new(command.clone()));
                }
                node.extension = Some(Arc::new(config.clone()));
            }
            Op::SetSplit(id, config) => {
                let node = self.node(*id)?;
                if node.kind != Kind::SplitPane
                    || !config.is_valid()
                    || node
                        .split
                        .as_ref()
                        .is_some_and(|old| config.reset_generation < old.reset_generation)
                {
                    return Err(ErrorCode::InvalidTree);
                }
                self.node_mut(*id)?.split = Some(Arc::new(config.clone()));
            }
            Op::SetCanvas(id, config) => {
                if self.node(*id)?.kind != Kind::CanvasView || !config.is_valid() {
                    return Err(ErrorCode::InvalidTree);
                }
                self.node_mut(*id)?.canvas = Some(Arc::new(config.clone()));
            }
            Op::SetDocument(id, config) => {
                if self.node(*id)?.kind != Kind::DocumentView || !config.is_valid() {
                    return Err(ErrorCode::InvalidTree);
                }
                self.node_mut(*id)?.document = Some(Arc::new(config.clone()));
            }
            Op::SetLoading(id, config) => {
                if self.node(*id)?.kind != Kind::Loading || !config.is_valid() {
                    return Err(ErrorCode::InvalidTree);
                }
                self.node_mut(*id)?.loading = Some(Arc::new(config.clone()));
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
                self.node_mut(*id)?.commands =
                    Some(commands.iter().cloned().map(Arc::new).collect());
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
                if !matches!(self.node(*id)?.kind, Kind::Tooltip | Kind::HoverCard)
                    || !config.is_valid()
                {
                    return Err(ErrorCode::InvalidTree);
                }
                self.node_mut(*id)?.tooltip = Some(Arc::new(config.clone()));
            }
            Op::SetPlacement(id, placement) => {
                if !matches!(
                    self.node(*id)?.kind,
                    Kind::FocusScope | Kind::Tooltip | Kind::HoverCard
                ) || placement.is_some_and(|placement| !placement.is_valid())
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
                    Kind::RadioGroup | Kind::TabBar | Kind::Select | Kind::Combobox
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
            if node.kind == Kind::Disclosure
                && (node.children.len() != 2
                    || self.node(node.children[1])?.kind != Kind::Panel
                    || {
                        let header = self.node(node.children[0])?;
                        !(header.kind == Kind::Button
                            || (header.kind == Kind::Container
                                && header.children.last().is_some_and(|id| {
                                    self.node(*id).is_ok_and(|node| node.kind == Kind::Button)
                                })))
                    })
            {
                return Err(ErrorCode::InvalidTree);
            }
            if node.kind == Kind::Accordion
                && (node.children.len() > 4096
                    || node.children.iter().any(|id| {
                        !self
                            .node(*id)
                            .is_ok_and(|child| child.kind == Kind::Disclosure)
                    }))
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
                                | Field::Inert(_)
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
