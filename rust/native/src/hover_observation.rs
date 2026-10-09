//! Asynchronous native hover history, independent of action/focus ownership.
use super::*;
use gpuio_protocol::HandlerId;

pub(super) struct State {
    window: WindowId,
    node: NodeId,
    session: Rc<RefCell<Session>>,
    transport: Arc<Transport>,
    gate: focus::Shared,
    raw: Cell<bool>,
    eligible: Cell<bool>,
    reported: Cell<Option<(HandlerId, bool)>>,
}
impl State {
    fn publish(&self, expected: Option<HandlerId>, window: &mut Window, cx: &mut App) {
        let event = {
            let session = self.session.borrow();
            let Some(tree) = session.tree(self.window) else {
                return;
            };
            let Some(node) = tree.get(self.node) else {
                return;
            };
            if node.hover_handler != expected {
                return;
            }
            let gate = self.gate.borrow();
            let hovered = self.raw.get()
                && self.eligible.get()
                && gate.visible(self.node)
                && gate.allows(self.node)
                && !gate.disabled(self.node);
            let observation = expected.map(|handler| (handler, hovered));
            if self.reported.get() == observation {
                return;
            }
            let event = expected.and_then(|handler| {
                session.hover_changed(self.window, self.node, handler, tree.revision(), hovered)
            });
            if expected.is_none() || event.is_some() {
                self.reported.set(observation);
            }
            event
        };
        if let Some(event) = event
            && !self.transport.input(event)
        {
            // Rendering holds a shared Session borrow. Defer only fault mutation;
            // enqueue accepted observations now to preserve input order and bounds.
            let session = self.session.clone();
            let transport = self.transport.clone();
            let id = self.window;
            window.defer(cx, move |_, _| {
                if session.borrow_mut().overload(id) {
                    transport.fault(id);
                }
            });
        }
    }
}
impl View {
    pub(super) fn observe_button_hover(
        &mut self,
        element: gpui::Stateful<gpui::Div>,
        node: &crate::tree::Node,
        eligible: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> gpui::Stateful<gpui::Div> {
        let state = if node.hover_handler.is_some() {
            Some(
                self.hover_observations
                    .entry(node.id)
                    .or_insert_with(|| {
                        Rc::new(State {
                            window: self.id,
                            node: node.id,
                            session: self.session.clone(),
                            transport: self.transport.clone(),
                            gate: self.focus.clone(),
                            raw: Cell::new(false),
                            eligible: Cell::new(eligible),
                            reported: Cell::new(None),
                        })
                    })
                    .clone(),
            )
        } else {
            self.hover_observations.get(&node.id).cloned()
        };
        let Some(state) = state else {
            return element;
        };
        state.eligible.set(eligible);
        state.publish(node.hover_handler, window, cx);
        let expected = node.hover_handler;
        // Once subscribed, continue native tracking until accepted-node retirement. This
        // keeps stationary-pointer resubscription correct without resetting the
        // element's click/press state or adding a polling task.
        element.on_hover(move |hovered, window, cx| {
            state.raw.set(*hovered);
            state.publish(expected, window, cx);
        })
    }

    pub(super) fn retire_unvisited_hover(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let session = self.session.borrow();
        let tree = session.tree(self.id);
        for (node, state) in &self.hover_observations {
            if !self.visited.contains(node) {
                state.eligible.set(false);
                let handler = tree
                    .and_then(|tree| tree.get(*node))
                    .and_then(|node| node.hover_handler);
                state.publish(handler, window, cx);
            }
        }
        // At most one state per accepted node; its generation prevents slot reuse
        // from preserving old observations. No unbounded history of evicted rows.
        self.hover_observations
            .retain(|node, _| tree.is_some_and(|tree| tree.get(*node).is_some()));
    }
}
