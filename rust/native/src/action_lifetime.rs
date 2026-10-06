//! Frame callbacks must not regain eligibility after their retained slot hides.
use gpuio_protocol::NodeId;
use std::{cell::Cell, collections::BTreeMap, rc::Rc};

#[derive(Clone)]
pub(super) struct Lease {
    generation: u64,
    live: Rc<Cell<bool>>,
}
impl Lease {
    pub(super) fn is_live(&self) -> bool {
        self.live.get()
    }
    pub(super) fn element_id(&self) -> gpui::ElementId {
        ("gpuio-action-lifetime", self.generation).into()
    }
}

#[derive(Default)]
pub(super) struct Registry {
    next: u64,
    leases: BTreeMap<NodeId, Lease>,
}
impl Registry {
    pub(super) fn lease(&mut self, node: NodeId) -> Lease {
        self.leases
            .entry(node)
            .or_insert_with(|| {
                self.next = self.next.checked_add(1).expect("action lifetime exhausted");
                Lease {
                    generation: self.next,
                    live: Rc::new(Cell::new(true)),
                }
            })
            .clone()
    }
    pub(super) fn retire(&mut self, mut eligible: impl FnMut(NodeId) -> bool) {
        self.leases.retain(|node, lease| {
            if eligible(*node) {
                true
            } else {
                lease.live.set(false);
                false
            }
        });
    }
}
impl Drop for Registry {
    fn drop(&mut self) {
        for lease in self.leases.values() {
            lease.live.set(false);
        }
    }
}
