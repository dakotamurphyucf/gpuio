//! Native tab offsets and measured, stable-ID reveal. No selection or frame clock.
use super::scroll;
use gpui::{
    A11ySubtreeBuilder, App, Bounds, Element, ElementId, GlobalElementId, InspectorElementId,
    InteractiveElement, IntoElement, LayoutId, Pixels, Window, accesskit, canvas, prelude::*, px,
};
use gpuio_protocol::{tab_viewport::Config, v1::ChoiceConfig};
use std::{
    cell::RefCell,
    rc::{Rc, Weak},
    sync::{Arc, Weak as SyncWeak},
};

// Keep bulky Div builder temporaries outside the recursive host renderer.
pub(super) fn style(mut element: gpui::Stateful<gpui::Div>) -> gpui::Stateful<gpui::Div> {
    // Layout ownership must not turn a retained hidden viewport back on.
    if element.style().display != Some(gpui::Display::None) {
        element = element.flex();
    }
    element
        .flex_row()
        .flex_nowrap()
        .min_w_0()
        .overflow_x_scroll()
        .overflow_y_hidden()
}

#[derive(Clone, Copy)]
enum Source {
    Application,
    Navigation,
}
struct Pending {
    target: String,
    source: Source,
}
#[derive(Default)]
pub(super) struct State {
    config: SyncWeak<ChoiceConfig>,
    bounds: Box<[Option<Bounds<Pixels>>]>,
    last_serial: i64,
    pending: Option<Pending>,
    painted: bool,
    saved_offset: Option<gpui::Point<Pixels>>,
    restore_offset: bool,
}
impl State {
    pub(super) fn prepare_layout(&mut self, scroll: &scroll::State) {
        // GPUI clamps its handle to zero for hidden/zero-size layout. Preserve
        // the last painted offset before that clamp, including intervening wheel
        // input, and restore it only when usable paint resumes.
        if self.painted || self.saved_offset.is_none() {
            self.saved_offset = Some(scroll.handle.offset());
        }
        self.restore_offset = !self.painted;
        self.painted = false;
    }
    pub(super) fn begin(&mut self, choices: &Arc<ChoiceConfig>, config: &Config) {
        if !self.config.ptr_eq(&Arc::downgrade(choices)) {
            self.config = Arc::downgrade(choices);
            self.bounds = vec![None; choices.items.len()].into_boxed_slice();
        } else {
            self.bounds.fill(None);
        }
        match &config.reveal {
            Some(request) if request.serial > self.last_serial => {
                self.last_serial = request.serial;
                self.pending = Some(Pending {
                    target: request.target.clone(),
                    source: Source::Application,
                });
            }
            None if self
                .pending
                .as_ref()
                .is_some_and(|p| matches!(p.source, Source::Application)) =>
            {
                self.pending = None
            }
            _ => {}
        }
        if self
            .pending
            .as_ref()
            .is_some_and(|p| !choices.items.iter().any(|item| item.id == p.target))
        {
            self.pending = None;
        }
    }
    pub(super) fn navigate(&mut self, target: &str) {
        self.pending = Some(Pending {
            target: target.to_owned(),
            source: Source::Navigation,
        });
    }
    fn record(&mut self, index: usize, mut bounds: Bounds<Pixels>, scroll: &scroll::State) {
        bounds.origin -= scroll.handle.offset();
        if let Some(slot) = self.bounds.get_mut(index) {
            *slot = Some(bounds);
        }
    }
    fn finish(&mut self, scroll: &scroll::State) -> bool {
        let Some(viewport) = scroll.mask.get() else {
            return false;
        };
        if viewport.size.width <= px(0.) || viewport.size.height <= px(0.) {
            return false;
        }
        let before = scroll.handle.offset();
        if self.restore_offset
            && let Some(saved) = self.saved_offset
        {
            let maximum = scroll.handle.max_offset();
            scroll.handle.set_offset(gpui::point(
                saved.x.clamp(-maximum.x, px(0.)),
                saved.y.clamp(-maximum.y, px(0.)),
            ));
        }
        self.restore_offset = false;
        self.painted = true;
        self.reveal_pending(scroll);
        scroll.handle.offset() != before
    }
    fn reveal_pending(&mut self, scroll: &scroll::State) -> bool {
        let Some(pending) = &self.pending else {
            return false;
        };
        let Some(config) = self.config.upgrade() else {
            self.pending = None;
            return false;
        };
        let Some(index) = config
            .items
            .iter()
            .position(|item| item.id == pending.target)
        else {
            self.pending = None;
            return false;
        };
        let Some(viewport) = scroll.mask.get() else {
            return false;
        };
        if viewport.size.width <= px(0.) || viewport.size.height <= px(0.) {
            return false;
        }
        let Some(mut bounds) = self.bounds[index] else {
            return false;
        };
        if bounds.size.width <= px(0.) || bounds.size.height <= px(0.) {
            return false;
        }
        bounds.origin += scroll.handle.offset();
        self.pending = None;
        scroll.reveal(bounds).1
    }
}
#[derive(Clone)]
pub(super) struct Binding {
    pub state: Rc<RefCell<State>>,
    pub scroll: Rc<scroll::State>,
}
impl Binding {
    pub(super) fn navigate(&self, target: &str) {
        self.state.borrow_mut().navigate(target);
    }
    pub(super) fn finish(&self) -> impl IntoElement {
        let state = Rc::downgrade(&self.state);
        let scroll = Rc::downgrade(&self.scroll);
        canvas(
            |_, _, _| (),
            move |_, _, window, _| {
                if let (Some(state), Some(scroll)) = (state.upgrade(), scroll.upgrade())
                    && state.borrow_mut().finish(&scroll)
                {
                    // refresh() is intentionally ignored during GPUI paint.
                    // Deliver one post-paint wake, then stop once the request is consumed.
                    let state = Rc::downgrade(&state);
                    window.on_next_frame(move |window, _| {
                        if state.upgrade().is_some() {
                            window.refresh();
                        }
                    });
                }
            },
        )
        .absolute()
        .top_0()
        .left_0()
        .size_full()
    }
}

pub(super) struct Measure<E> {
    element: E,
    binding: Option<(Weak<RefCell<State>>, Weak<scroll::State>, usize)>,
    motion: Option<(super::tab_motion::Binding, bool, bool)>,
}
impl<E> Measure<E> {
    pub(super) fn new(
        element: E,
        binding: Option<&Binding>,
        index: usize,
        motion: Option<&super::tab_motion::Binding>,
        selected: bool,
    ) -> Self {
        Self {
            element,
            motion: motion.map(|m| (m.clone(), index == 0, selected)),
            binding: binding.map(|b| (Rc::downgrade(&b.state), Rc::downgrade(&b.scroll), index)),
        }
    }
}
impl<E: InteractiveElement> InteractiveElement for Measure<E> {
    fn interactivity(&mut self) -> &mut gpui::Interactivity {
        self.element.interactivity()
    }
}
impl<E: Element> IntoElement for Measure<E> {
    type Element = Self;
    fn into_element(self) -> Self {
        self
    }
}
impl<E: Element> Element for Measure<E> {
    type RequestLayoutState = E::RequestLayoutState;
    type PrepaintState = E::PrepaintState;
    fn id(&self) -> Option<ElementId> {
        Element::id(&self.element)
    }
    fn source_location(&self) -> Option<&'static std::panic::Location<'static>> {
        self.element.source_location()
    }
    fn request_layout(
        &mut self,
        id: Option<&GlobalElementId>,
        inspector: Option<&InspectorElementId>,
        window: &mut Window,
        cx: &mut App,
    ) -> (LayoutId, Self::RequestLayoutState) {
        self.element.request_layout(id, inspector, window, cx)
    }
    fn prepaint(
        &mut self,
        id: Option<&GlobalElementId>,
        inspector: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>,
        layout: &mut Self::RequestLayoutState,
        window: &mut Window,
        cx: &mut App,
    ) -> Self::PrepaintState {
        let result = self
            .element
            .prepaint(id, inspector, bounds, layout, window, cx);
        if let Some((state, scroll, index)) = &self.binding
            && let (Some(state), Some(scroll)) = (state.upgrade(), scroll.upgrade())
        {
            state.borrow_mut().record(*index, bounds, &scroll);
        }
        if let Some((motion, first, selected)) = &self.motion {
            motion.record(*first, *selected, bounds);
        }
        result
    }
    fn paint(
        &mut self,
        id: Option<&GlobalElementId>,
        inspector: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>,
        layout: &mut Self::RequestLayoutState,
        prepaint: &mut Self::PrepaintState,
        window: &mut Window,
        cx: &mut App,
    ) {
        self.element
            .paint(id, inspector, bounds, layout, prepaint, window, cx)
    }
    fn a11y_role(&self) -> Option<accesskit::Role> {
        self.element.a11y_role()
    }
    fn write_a11y_info(&self, node: &mut accesskit::Node) {
        self.element.write_a11y_info(node);
    }
    fn a11y_synthetic_children(
        &mut self,
        prepaint: &mut Self::PrepaintState,
        builder: &mut A11ySubtreeBuilder,
    ) {
        self.element.a11y_synthetic_children(prepaint, builder);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn reservation_covers_fixed_state_pending_id_scroll_and_measured_boxes() {
        assert!(
            std::mem::size_of::<State>()
                + std::mem::size_of::<scroll::State>()
                + std::mem::size_of::<Pending>()
                + 256
                < Config::owner_reserved_bytes(0)
        );
        assert!(std::mem::size_of::<Option<Bounds<Pixels>>>() <= 64);
    }
}
