//! Transfer the image root's computed native corner radii to its pixel element.
//! GPUI Div overflow masks are rectangular; Img paints its own radii. Capture at
//! paint time so hover/pressed refinements use GPUI's actual hitbox and state.
use gpui::{
    A11ySubtreeBuilder, App, Bounds, Element, ElementId, GlobalElementId, Hitbox,
    InspectorElementId, InteractiveElement, IntoElement, LayoutId, Pixels, Styled, Window,
    accesskit,
};
use std::{cell::Cell, rc::Rc};
pub(super) type Shared = Rc<Cell<gpui::Corners<Pixels>>>;
enum Mode {
    Capture,
    Apply,
}
pub(super) struct Rounded<E> {
    element: E,
    shared: Shared,
    mode: Mode,
}
impl<E> Rounded<E> {
    pub(super) fn capture(element: E, shared: Shared) -> Self {
        Self {
            element,
            shared,
            mode: Mode::Capture,
        }
    }
    pub(super) fn apply(element: E, shared: Shared) -> Self {
        Self {
            element,
            shared,
            mode: Mode::Apply,
        }
    }
}
impl<E: Element<PrepaintState = Option<Hitbox>> + InteractiveElement + Styled> IntoElement
    for Rounded<E>
{
    type Element = Self;
    fn into_element(self) -> Self {
        self
    }
}
impl<E: Element<PrepaintState = Option<Hitbox>> + InteractiveElement + Styled> Element
    for Rounded<E>
{
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
        match self.mode {
            Mode::Capture => {
                let style =
                    self.element
                        .interactivity()
                        .compute_style(id, prepaint.as_ref(), window, cx);
                self.shared
                    .set(style.corner_radii.to_pixels(window.rem_size()));
            }
            Mode::Apply => {
                let radii = self.shared.get();
                let target = &mut self.element.style().corner_radii;
                target.top_left = Some(radii.top_left.into());
                target.top_right = Some(radii.top_right.into());
                target.bottom_left = Some(radii.bottom_left.into());
                target.bottom_right = Some(radii.bottom_right.into());
            }
        }
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

impl<E: Styled> Styled for Rounded<E> {
    fn style(&mut self) -> &mut gpui::StyleRefinement {
        self.element.style()
    }
}
impl<E: InteractiveElement> InteractiveElement for Rounded<E> {
    fn interactivity(&mut self) -> &mut gpui::Interactivity {
        self.element.interactivity()
    }
}
