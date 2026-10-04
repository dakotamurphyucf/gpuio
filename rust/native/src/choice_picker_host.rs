//! Accepted-tree lifetime and focus visibility for the mounted picker owner.
//! Popup rendering and gesture routing live in choice_picker_view.
use super::*;
use crate::choice_picker_state::State;
use gpuio_protocol::{
    HandlerId,
    choice_picker::{OpenState, Presentation, Slot},
};

/// One user-initiated focus handoff, awaiting controlled visibility acceptance.
/// The weak source and blur subscription cannot keep a retired editor alive.
pub(super) struct PendingFocus {
    open: bool,
    source: gpui::WeakFocusHandle,
    cancelled: Rc<Cell<bool>>,
    _subscription: gpui::Subscription,
}
impl PendingFocus {
    pub(super) fn new(
        open: bool,
        source: &gpui::FocusHandle,
        window: &mut Window,
        cx: &mut Context<View>,
    ) -> Self {
        let cancelled = Rc::new(Cell::new(false));
        let on_blur = cancelled.clone();
        let subscription = cx.on_blur(source, window, move |_, _, _| on_blur.set(true));
        Self {
            open,
            source: source.downgrade(),
            cancelled,
            _subscription: subscription,
        }
    }

    fn is_current(&self, window: &Window) -> bool {
        !self.cancelled.get()
            && self
                .source
                .upgrade()
                .is_some_and(|source| source.is_focused(window))
    }
}

pub(super) struct Owner {
    pub(super) state: State,
    pub(super) pending_focus: Option<PendingFocus>,
    pub(super) presentation: Arc<Presentation>,
    wrappers: Arc<[NodeId]>,
    pub(super) query: Option<NodeId>,
    pub(super) clear_focus: gpui::FocusHandle,
    eligible: bool,
    handler: HandlerId,
    gated_open: bool,
    measured_visible: Option<bool>,
    pub(super) painted: Rc<Cell<bool>>,
    pub(super) render: Option<super::choice_picker_view::Rows>,
    pub(super) children: Arc<crate::choice_picker_admission::Children>,
    pub(super) focus_subscription: Option<(gpui::FocusHandle, HandlerId, gpui::Subscription)>,
    pub(super) trigger: Rc<std::cell::Cell<Bounds<gpui::Pixels>>>,
    pub(super) popup: Rc<std::cell::Cell<Bounds<gpui::Pixels>>>,
}

impl View {
    pub(super) fn begin_picker_paint(&self) {
        for owner in self.pickers.values() {
            owner.painted.set(false);
        }
    }

    /// Sample after deferred content has painted, including nested footer pickers.
    /// Keep accepted owners: culling must not replay managed initial visibility.
    pub(super) fn finish_picker_paint(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let mut changed = false;
        for owner in self.pickers.values_mut() {
            let visible = owner.painted.get();
            changed |= owner.measured_visible.unwrap_or(true) != visible;
            owner.measured_visible = Some(visible);
        }
        if changed {
            self.sync_choice_pickers(&[], window, cx);
            cx.notify();
        }
    }

    /// Run after admission/editor synchronization. Preserve managed initial state
    /// across accepted updates and release owners only with their tree identities.
    pub(super) fn sync_choice_pickers(
        &mut self,
        dirty: &[NodeId],
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let shared = self.session.clone();
        let session = shared.borrow();
        let Some(tree) = session.tree(self.id) else {
            for owner in self.pickers.values() {
                self.focus
                    .borrow_mut()
                    .replace_picker_hidden(&owner.wrappers, &[]);
            }
            self.pickers.clear();
            return;
        };
        self.pickers.retain(|id, owner| {
            let keep = tree
                .get(*id)
                .is_some_and(|node| node.choice_picker.is_some());
            if !keep {
                self.focus
                    .borrow_mut()
                    .replace_picker_hidden(&owner.wrappers, &[]);
            }
            keep
        });
        let mut ids: std::collections::BTreeSet<_> = self.pickers.keys().copied().collect();
        ids.extend(
            dirty
                .iter()
                .copied()
                .filter(|id| tree.get(*id).is_some_and(|n| n.choice_picker.is_some())),
        );
        // Parent gates must settle before nested pickers in interactive footers.
        // Node allocation order is not a parent-order guarantee.
        let mut ordered: Vec<_> = ids
            .into_iter()
            .map(|id| {
                let mut depth = 0;
                let mut parent = tree.get(id).and_then(|n| n.parent);
                while let Some(id) = parent {
                    depth += 1;
                    parent = tree.get(id).and_then(|n| n.parent);
                }
                (depth, id)
            })
            .collect();
        ordered.sort_unstable();
        let mut events = Vec::new();
        let mut query_snapshots = Vec::new();
        let mut focus_handoffs = Vec::new();
        for (_, id) in ordered {
            let node = tree.get(id).expect("retained picker");
            let presentation = node.choice_picker.as_ref().expect("picker presentation");
            let handler = node.handler.expect("admitted picker observer");
            let query = presentation
                .slots
                .iter()
                .position(|role| matches!(role, Slot::Query))
                .and_then(|index| node.children.get(index))
                .and_then(|wrapper| tree.get(*wrapper))
                .and_then(|wrapper| wrapper.children.first())
                .copied();
            let eligible = self.focus.borrow().allows(id)
                && self
                    .pickers
                    .get(&id)
                    .is_none_or(|owner| owner.measured_visible.unwrap_or(true));
            let initial = !self.pickers.contains_key(&id);
            let children = if !initial && !dirty.contains(&id) {
                self.pickers[&id].children.clone()
            } else {
                Arc::new(
                    crate::choice_picker_admission::children(presentation, &node.children, |id| {
                        tree.get(id)
                    })
                    .expect("admitted picker children"),
                )
            };
            let owner = self.pickers.entry(id).or_insert_with(|| Owner {
                state: State::new(Arc::new(presentation.config.clone()), eligible, query)
                    .expect("admitted picker state"),
                pending_focus: None,
                presentation: presentation.clone(),
                wrappers: Arc::from([]),
                query,
                clear_focus: cx.focus_handle(),
                eligible,
                handler,
                gated_open: false,
                measured_visible: None,
                painted: Default::default(),
                render: None,
                children: children.clone(),
                focus_subscription: None,
                trigger: Default::default(),
                popup: Default::default(),
            });
            let previous_open = owner.gated_open;
            let rebound = owner.handler != handler;
            if rebound
                || owner.query != query
                || !eligible
                || presentation.config.disabled
                || !matches!(presentation.config.open_state, OpenState::Controlled(_))
                || owner
                    .pending_focus
                    .as_ref()
                    .is_some_and(|pending| !pending.is_current(window))
            {
                owner.pending_focus = None;
            }
            if (initial || rebound || owner.query != query)
                && let Some(query) = query
            {
                query_snapshots.push(query);
            }
            if !Arc::ptr_eq(&owner.presentation, presentation)
                || owner.query != query
                || owner.eligible != eligible
            {
                let config = if owner.presentation.config == presentation.config {
                    owner.state.config().clone()
                } else {
                    Arc::new(presentation.config.clone())
                };
                let changed = owner
                    .state
                    .configure(config, eligible, query)
                    .expect("admitted picker update");
                if !initial && !rebound {
                    events.extend(changed.into_iter().map(|e| (id, handler, e)));
                }
            }
            if previous_open != owner.state.is_open()
                && let Some(pending) = owner.pending_focus.take()
                && pending.open == owner.state.is_open()
            {
                // Validate before the hidden-content gate blurs a closing query.
                // All handoffs run below, after every parent/child gate settles.
                focus_handoffs.push((id, pending.open));
            }
            if initial || rebound {
                events.push((id, handler, owner.state.snapshot()));
            }
            if initial
                || previous_open != owner.state.is_open()
                || !Arc::ptr_eq(&owner.wrappers, &node.children)
                || !Arc::ptr_eq(&owner.presentation, presentation)
            {
                let hidden: Vec<_> = node
                    .children
                    .iter()
                    .zip(&presentation.slots)
                    .filter_map(|(id, slot)| {
                        (!owner.state.is_open() && !matches!(slot, Slot::Trigger)).then_some(*id)
                    })
                    .collect();
                self.focus
                    .borrow_mut()
                    .replace_picker_hidden(&owner.wrappers, &hidden);
            }
            owner.presentation = presentation.clone();
            owner.wrappers = node.children.clone();
            owner.query = query;
            owner.eligible = eligible;
            owner.handler = handler;
            owner.gated_open = owner.state.is_open();
            owner.children = children;
            if dirty.contains(&id)
                && let Some(rows) = &mut owner.render
            {
                // Update retained caches even if this window is occluded and will
                // not draw. Old catalogs must not outlive their payload charge.
                let query_text = query
                    .and_then(|id| self.editors.get(&id))
                    .map_or_else(String::new, |editor| editor.snapshot(window, cx).text);
                rows.reconcile(
                    owner.state.config().clone(),
                    query_text,
                    (presentation.estimated_row_height, presentation.overscan),
                );
                rows.invalidate();
            }
        }
        let revision = tree.revision();
        drop(session);
        // Removing/retiring the optional clear control must not strand focus on
        // its retained handle. Preserve the trigger when it remains eligible.
        if let Some((id, eligible)) = self.pickers.iter().find_map(|(id, owner)| {
            (owner.clear_focus.is_focused(window)
                && (!owner.presentation.config.clearable
                    || !owner.eligible
                    || owner.presentation.config.disabled))
                .then_some((*id, owner.eligible && !owner.presentation.config.disabled))
        }) {
            if eligible && let Some(button) = self.buttons.get(&id) {
                window.focus(&button.focus, cx);
            } else {
                window.blur(cx);
            }
        }
        // A retained query/footer can still own keyboard focus from the previous
        // frame. Drop that focus as soon as its popup closes, before layout.
        let focused = self.focus.borrow().focused_node(window, cx).or_else(|| {
            self.editors
                .iter()
                .find_map(|(id, editor)| editor.focus_handle(cx).is_focused(window).then_some(*id))
        });
        if focused.is_some_and(|node| self.focus.borrow().picker_hides(node)) {
            window.blur(cx);
        }
        for (node, handler, event) in events {
            let event = self
                .session
                .borrow()
                .choice_picker_event(self.id, node, handler, revision, event);
            if let Some(event) = event
                && !self.transport.input(event)
                && self.session.borrow_mut().overload(self.id)
            {
                self.transport.fault(self.id);
            }
        }
        for (id, open) in focus_handoffs {
            if open {
                if let Some(query) = self.pickers[&id].query
                    && let Some(editor) = self.editors.get_mut(&query)
                {
                    editor.command(&EditorCommand::Focus, window, cx);
                }
            } else if self.focus.borrow().allows(id)
                && let Some(button) = self.buttons.get(&id)
            {
                window.focus(&button.focus, cx);
            }
        }
        for query in query_snapshots {
            if let Some(editor) = self.editors.get(&query) {
                editor.publish_picker_snapshot(window, cx);
            }
        }
    }
}
