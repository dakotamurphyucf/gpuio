//! Inherited text metrics are resolved during layout, after ancestor styles.
//! Invalidate before entering ListState's layout borrow; retain its scroll anchor.
use gpui::{
    A11ySubtreeBuilder, App, Bounds, Element, ElementId, GlobalElementId, InspectorElementId,
    IntoElement, LayoutId, ListState, Pixels, TextStyle, Window, accesskit,
};
use std::{cell::RefCell, rc::Rc};

#[derive(PartialEq)]
struct Metrics {
    text: TextStyle,
    rem: Pixels,
    scale: f32,
}

#[derive(Clone, Default)]
pub(super) struct Cache(Rc<RefCell<Option<Metrics>>>);

pub(super) struct Layout<E> {
    element: E,
    list: ListState,
    cache: Cache,
}

pub(super) fn observe<E>(element: E, list: ListState, cache: Cache) -> Layout<E> {
    Layout {
        element,
        list,
        cache,
    }
}

impl<E: Element> IntoElement for Layout<E> {
    type Element = Self;
    fn into_element(self) -> Self {
        self
    }
}

impl<E: Element> Element for Layout<E> {
    type RequestLayoutState = E::RequestLayoutState;
    type PrepaintState = E::PrepaintState;
    fn id(&self) -> Option<ElementId> {
        self.element.id()
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
        let mut text = window.text_style();
        // Paint-only inherited fields do not change row heights.
        text.color = gpui::black();
        text.background_color = None;
        text.underline = None;
        text.strikethrough = None;
        let next = Metrics {
            text,
            rem: window.rem_size(),
            scale: window.scale_factor(),
        };
        let mut cache = self.cache.0.borrow_mut();
        if cache.as_ref() != Some(&next) {
            self.list.remeasure_items(0..self.list.item_count());
            *cache = Some(next);
        }
        drop(cache);
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
        self.element
            .prepaint(id, inspector, bounds, layout, window, cx)
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
            .paint(id, inspector, bounds, layout, prepaint, window, cx);
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
