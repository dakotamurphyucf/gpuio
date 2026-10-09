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

use super::rendered_text::RenderedFragment;
use super::{RenderedAccessiblePartId, RenderedSemanticId, RenderedText, RenderedTextPosition};

#[path = "visual_lines.rs"]
mod visual_lines;

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

#[derive(Clone, Copy)]
pub(super) struct TextRun {
    node: NodeId,
    part: RenderedAccessiblePartId,
    first: usize,
    len: usize,
    source_start: usize,
}

impl TextRun {
    pub(super) fn new(
        projection: &RenderedText,
        node: NodeId,
        part: RenderedAccessiblePartId,
        characters: std::ops::Range<usize>,
    ) -> Option<Self> {
        let source_start = projection.accessible_character_utf16(part, characters.start)?;
        projection.accessible_character_utf16(part, characters.end)?;
        (characters.start <= characters.end).then_some(Self {
            node,
            part,
            first: characters.start,
            len: characters.len(),
            source_start,
        })
    }
}

#[derive(Default)]
struct State {
    document: Option<NodeId>,
    complete: bool,
    completed: HashMap<NodeId, std::ops::Range<usize>>,
    window: Option<gpui::WindowId>,
    projection: Weak<RenderedText>,
    candidates: HashMap<NodeId, RenderedSemanticId>,
    published: Vec<RenderedSemanticAttachment>,
    candidate_runs: HashMap<NodeId, TextRun>,
    runs: HashMap<NodeId, TextRun>,
    ordered_runs: Vec<TextRun>,
}

#[derive(Clone, Default)]
pub(super) struct Frame(Arc<Mutex<State>>);

impl Frame {
    pub fn begin(
        &self,
        window: gpui::WindowId,
        projection: Option<&Arc<RenderedText>>,
        complete: bool,
    ) {
        let mut state = self.0.lock().expect("semantic attachment frame");
        state.projection = projection.map_or_else(Weak::new, Arc::downgrade);
        state.window = Some(window);
        state.complete = complete;
        state.document = None;
        state.completed.clear();
        state.candidates.clear();
        state.published.clear();
        state.candidate_runs.clear();
        state.runs.clear();
        state.ordered_runs.clear();
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

    pub fn complete_owner(
        &self,
        owner: RenderedSemanticId,
        builder: &mut gpui::A11ySubtreeBuilder,
    ) {
        self.complete_owners(owner, owner, builder);
    }

    pub fn complete_owners(
        &self,
        first: RenderedSemanticId,
        last: RenderedSemanticId,
        builder: &mut gpui::A11ySubtreeBuilder,
    ) {
        let mut state = self.0.lock().expect("semantic attachment frame");
        if !state.complete {
            return;
        }
        let Some(projection) = state.projection.upgrade() else {
            return;
        };
        if projection.semantic_node(first).is_none() || projection.semantic_node(last).is_none() {
            return;
        }
        let first = super::logical_accessibility::semantic_span(&projection, first);
        let last = super::logical_accessibility::semantic_span(&projection, last);
        let span = match (first, last) {
            (Some(first), Some(last)) if first.start <= last.end => first.start..last.end,
            (Some(span), None) | (None, Some(span)) => span,
            _ => return,
        };
        let mut children: Vec<_> = builder
            .parent_node()
            .children()
            .iter()
            .map(|id| (*id, None::<std::ops::Range<usize>>))
            .collect();
        let mut roots: HashMap<_, _> = children
            .iter()
            .enumerate()
            .map(|(index, (id, _))| (*id, index))
            .collect();
        // Descendants arrive parent first in reverse postorder. Route each
        // represented interval to the direct child that already owns it.
        builder.visit_descendants(|id, node| {
            let Some(index) = roots.remove(&id) else {
                return;
            };
            if let Some(range) = state.completed.get(&id) {
                let current = &mut children[index].1;
                *current = Some(current.as_ref().map_or_else(
                    || range.clone(),
                    |old| old.start.min(range.start)..old.end.max(range.end),
                ));
                return;
            }
            roots.extend(node.children().iter().map(|child| (*child, index)));
            if let Some(run) = state.candidate_runs.get(&id) {
                let end = projection
                    .accessible_character_utf16(run.part, run.first + run.len)
                    .expect("recorded run range");
                let range = run.source_start..end;
                let current = &mut children[index].1;
                *current = Some(current.as_ref().map_or_else(
                    || range.clone(),
                    |old| old.start.min(range.start)..old.end.max(range.end),
                ));
            }
        });
        if let Some(runs) = super::logical_accessibility::complete_native(
            &projection,
            span.clone(),
            &children,
            builder,
        ) {
            state
                .candidate_runs
                .extend(runs.into_iter().map(|run| (run.node, run)));
            state.completed.insert(builder.parent_id(), span);
        }
    }

    /// Filter against the existing Document's finalized direct children. A row
    /// prepainted only for measurement or a rolled-back attempt is not attached.
    pub fn finish(&self, builder: &mut gpui::A11ySubtreeBuilder, complete: bool) {
        let children = builder.parent_node().children().to_vec();
        let mut state = self.0.lock().expect("semantic attachment frame");
        state.document = Some(builder.parent_id());
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
        let mut actual = Vec::new();
        builder.visit_descendants(|id, node| {
            if node.role() == gpui::Role::TextRun
                && !node.is_hidden()
                && let Some(run) = state.candidate_runs.get(&id)
            {
                actual.push(*run);
            }
        });
        state.candidate_runs.clear();
        if complete
            && ambiguous.is_empty()
            && let Some(projection) = state.projection.upgrade()
        {
            actual.extend(super::logical_accessibility::publish(
                &projection,
                &state.published,
                actual.iter().any(|run| run.len == 0),
                builder,
            ));
        }
        state.runs = actual.iter().map(|run| (run.node, *run)).collect();
        actual.sort_by_key(|run| run.source_start);
        if let Some(projection) = state.projection.upgrade() {
            visual_lines::publish(&projection, &actual, builder);
        }
        state.ordered_runs = actual;
    }

    pub fn record_run(&self, node: NodeId, fragment: &RenderedFragment, count: usize) {
        let mut state = self.0.lock().expect("semantic attachment frame");
        let Some(projection) = state.projection.upgrade() else {
            return;
        };
        let Some((part, characters)) = projection.accessible_fragment(fragment) else {
            return;
        };
        if characters.len() != count {
            return;
        }
        if let Some(run) = TextRun::new(&projection, node, part, characters) {
            state.candidate_runs.insert(node, run);
        }
    }

    pub fn record_reading_run(
        &self,
        node: NodeId,
        part: RenderedAccessiblePartId,
        characters: std::ops::Range<usize>,
    ) {
        let mut state = self.0.lock().expect("semantic attachment frame");
        let Some(projection) = state.projection.upgrade() else {
            return;
        };
        if let Some(run) = TextRun::new(&projection, node, part, characters) {
            state.candidate_runs.insert(node, run);
        }
    }

    pub fn position(
        &self,
        window: gpui::WindowId,
        projection: &Arc<RenderedText>,
        position: gpui::accesskit::TextPosition,
    ) -> Option<RenderedTextPosition> {
        let state = self.0.lock().expect("semantic attachment frame");
        if state.window != Some(window) || !state.projection.ptr_eq(&Arc::downgrade(projection)) {
            return None;
        }
        let run = state.runs.get(&position.node)?;
        if position.character_index > run.len {
            return None;
        }
        projection.accessible_position(run.part, run.first + position.character_index)
    }

    pub fn text_position(
        &self,
        window: gpui::WindowId,
        projection: &Arc<RenderedText>,
        position: &RenderedTextPosition,
    ) -> Option<gpui::accesskit::TextPosition> {
        let source = projection.accessible_utf16_offset(position)?;
        let state = self.0.lock().expect("semantic attachment frame");
        if state.window != Some(window) || !state.projection.ptr_eq(&Arc::downgrade(projection)) {
            return None;
        }
        let index = state
            .ordered_runs
            .partition_point(|run| run.source_start <= source)
            .checked_sub(1)?;
        let run = state.ordered_runs.get(index)?;
        let character = projection
            .accessible_character_in_part(run.part, position)?
            .checked_sub(run.first)?;
        (character <= run.len).then_some(gpui::accesskit::TextPosition {
            node: run.node,
            character_index: character,
        })
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

    pub fn publish_selection(
        &self,
        window: &mut Window,
        projection: &Arc<RenderedText>,
        selection: Option<&super::RenderedSelection>,
    ) -> bool {
        let document = {
            let state = self.0.lock().expect("semantic attachment frame");
            if state.window != Some(window.window_handle().window_id())
                || !state.projection.ptr_eq(&Arc::downgrade(projection))
            {
                return false;
            }
            state.document
        };
        let Some(document) = document else {
            return false;
        };
        let selection = selection.and_then(|selection| {
            Some(gpui::accesskit::TextSelection {
                anchor: self.text_position(
                    window.window_handle().window_id(),
                    projection,
                    selection.anchor(),
                )?,
                focus: self.text_position(
                    window.window_handle().window_id(),
                    projection,
                    selection.head(),
                )?,
            })
        });
        window.publish_document_selection(document, selection)
    }

    pub fn selection(
        &self,
        window: &Window,
        projection: &Arc<RenderedText>,
        selection: &gpui::accesskit::TextSelection,
    ) -> Option<super::RenderedSelection> {
        let state = self.0.lock().expect("semantic attachment frame");
        if state.window != Some(window.window_handle().window_id())
            || !state.projection.ptr_eq(&Arc::downgrade(projection))
            || !window.accepts_document_selection(state.document?, selection)
        {
            return None;
        }
        let position = |position: gpui::accesskit::TextPosition| {
            let run = state.runs.get(&position.node)?;
            if position.character_index > run.len {
                return None;
            }
            projection.accessible_position(run.part, run.first + position.character_index)
        };
        projection
            .selection(&position(selection.anchor)?, &position(selection.focus)?)
            .ok()
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
        self.frame.complete_owner(self.owner, builder);
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
