//! Immediate choice navigation and validated semantic event routing.
use super::SharedSession;
use crate::transport::Transport;
use gpuio_protocol::{HandlerId, NodeId, WindowId, v1::*};
use std::sync::Arc;

#[derive(Default)]
pub(super) struct State {
    pub(super) active: Option<String>,
}
impl State {
    pub(super) fn reconcile(&mut self, config: &ChoiceConfig, focused: bool) {
        if focused && self.active.as_ref().is_some_and(|id| config.can_select(id)) {
            return;
        }
        self.active = config
            .selected
            .as_ref()
            .filter(|id| config.can_select(id))
            .cloned()
            .or_else(|| {
                config
                    .items
                    .iter()
                    .find(|item| !config.disabled && !item.disabled)
                    .map(|item| item.id.clone())
            });
    }
    pub(super) fn navigate(&mut self, config: &ChoiceConfig, key: &str) -> bool {
        let enabled = config
            .items
            .iter()
            .filter(|item| !config.disabled && !item.disabled)
            .collect::<Vec<_>>();
        if enabled.is_empty() {
            self.active = None;
            return false;
        }
        let current = enabled
            .iter()
            .position(|item| self.active.as_ref() == Some(&item.id));
        let next = match key {
            "home" => 0,
            "end" => enabled.len() - 1,
            "right" | "down" => current.map_or(0, |i| (i + 1) % enabled.len()),
            "left" | "up" => current.map_or(enabled.len() - 1, |i| {
                (i + enabled.len() - 1) % enabled.len()
            }),
            _ => return false,
        };
        self.active = Some(enabled[next].id.clone());
        true
    }
}

#[derive(Clone)]
pub(super) struct Route {
    pub window: WindowId,
    pub node: NodeId,
    pub handler: HandlerId,
    pub revision: i64,
    pub session: SharedSession,
    pub transport: Arc<Transport>,
}
impl Route {
    pub(super) fn select_combobox(&self, selected: &str, snapshot: EditorSnapshot) {
        if snapshot.composition.is_some() {
            return;
        }
        let event = {
            let session = self.session.borrow();
            let valid_kind = session
                .tree(self.window)
                .and_then(|tree| tree.get(self.node))
                .is_some_and(|node| node.kind == Kind::Combobox);
            if !valid_kind {
                return;
            }
            session
                .choose(
                    self.window,
                    self.node,
                    self.handler,
                    self.revision,
                    selected,
                )
                .map(|_| {
                    Event::ComboboxSelected(
                        self.window,
                        self.node,
                        self.handler,
                        self.revision,
                        selected.to_owned(),
                        snapshot,
                    )
                })
        };
        if let Some(event) = event
            && !self.transport.input(event)
            && self.session.borrow_mut().overload(self.window)
        {
            self.transport.fault(self.window);
        }
    }
    pub(super) fn select(&self, selected: &str) {
        let event = self.session.borrow().choose(
            self.window,
            self.node,
            self.handler,
            self.revision,
            selected,
        );
        if let Some(event) = event
            && !self.transport.input(event)
            && self.session.borrow_mut().overload(self.window)
        {
            self.transport.fault(self.window);
        }
    }
}
