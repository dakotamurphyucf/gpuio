//! Frame-owned bindings to real block subtrees. These do not publish a second
//! document or replace native descendants and their input/action ownership.
use std::{
    collections::{HashMap, HashSet},
    sync::{Arc, Mutex, Weak},
};

use gpui::{
    AnyElement, App, Bounds, Element, ElementId, GlobalElementId, InspectorElementId, IntoElement,
    LayoutId, Pixels, Window, accesskit::NodeId,
};

use super::{RenderedSemanticId, RenderedText};

/// A prepared top-level owner and its actual current-frame native subtree.
/// This is not a text-run ID or authorization to dispatch an accessibility action.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RenderedSemanticAttachment {
    owner: RenderedSemanticId,
    node: NodeId,
}

impl RenderedSemanticAttachment {
    pub fn owner(&self) -> RenderedSemanticId {
        self.owner
    }

    pub fn node(&self) -> NodeId {
        self.node
    }
}

#[derive(Default)]
struct State {
    window: Option<gpui::WindowId>,
    projection: Weak<RenderedText>,
    candidates: HashMap<NodeId, RenderedSemanticId>,
    published: Vec<RenderedSemanticAttachment>,
}

#[derive(Clone, Default)]
pub(super) struct Frame(Arc<Mutex<State>>);

impl Frame {
    pub fn begin(&self, window: gpui::WindowId, projection: Option<&Arc<RenderedText>>) {
        let mut state = self.0.lock().expect("semantic attachment frame");
        state.projection = projection.map_or_else(Weak::new, Arc::downgrade);
        state.window = Some(window);
        state.candidates.clear();
        state.published.clear();
    }

    fn record(&self, owner: RenderedSemanticId, node: NodeId) {
        let mut state = self.0.lock().expect("semantic attachment frame");
        if state
            .projection
            .upgrade()
            .is_some_and(|projection| projection.semantic_node(owner).is_some())
        {
            state.candidates.insert(node, owner);
        }
    }

    /// Filter against the existing Document's finalized direct children. A row
    /// prepainted only for measurement or a rolled-back attempt is not attached.
    pub fn finish(&self, children: &[NodeId]) {
        let mut state = self.0.lock().expect("semantic attachment frame");
        let mut seen = HashSet::new();
        let mut ambiguous = HashSet::new();
        state.published = children
            .iter()
            .filter_map(|node| {
                let owner = *state.candidates.get(node)?;
                if !seen.insert(owner) {
                    ambiguous.insert(owner);
                }
                Some(RenderedSemanticAttachment { owner, node: *node })
            })
            .collect();
        state
            .published
            .retain(|item| !ambiguous.contains(&item.owner));
        state.candidates.clear();
    }

    pub fn snapshot(
        &self,
        window: gpui::WindowId,
        projection: &Arc<RenderedText>,
    ) -> Vec<RenderedSemanticAttachment> {
        let state = self.0.lock().expect("semantic attachment frame");
        if state.window == Some(window) && state.projection.ptr_eq(&Arc::downgrade(projection)) {
            state.published.clone()
        } else {
            Vec::new()
        }
    }

    pub fn wrap(&self, block: usize, owner: RenderedSemanticId, element: AnyElement) -> AnyElement {
        Scope {
            block,
            owner,
            frame: self.clone(),
            element,
        }
        .into_any_element()
    }
}

struct Scope {
    block: usize,
    owner: RenderedSemanticId,
    frame: Frame,
    element: AnyElement,
}

impl IntoElement for Scope {
    type Element = Self;
    fn into_element(self) -> Self {
        self
    }
}

impl Element for Scope {
    type RequestLayoutState = ();
    type PrepaintState = ();

    fn id(&self) -> Option<ElementId> {
        // Stable across compatible preparation changes; never put the prepared
        // identity into native element keys, which would remount child controls.
        Some(("rendered-block-scope", self.block).into())
    }

    fn source_location(&self) -> Option<&'static core::panic::Location<'static>> {
        None
    }

    fn a11y_role(&self) -> Option<gpui::Role> {
        Some(gpui::Role::GenericContainer)
    }

    fn a11y_synthetic_children(&mut self, _: &mut (), builder: &mut gpui::A11ySubtreeBuilder) {
        self.frame.record(self.owner, builder.parent_id());
    }

    fn request_layout(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        window: &mut Window,
        cx: &mut App,
    ) -> (LayoutId, ()) {
        (self.element.request_layout(window, cx), ())
    }

    fn prepaint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        _: Bounds<Pixels>,
        _: &mut (),
        window: &mut Window,
        cx: &mut App,
    ) {
        self.element.prepaint(window, cx);
    }

    fn paint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        _: Bounds<Pixels>,
        _: &mut (),
        _: &mut (),
        window: &mut Window,
        cx: &mut App,
    ) {
        self.element.paint(window, cx);
    }
}
