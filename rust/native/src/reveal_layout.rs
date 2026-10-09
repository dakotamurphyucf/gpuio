//! Layout/clip adapter for paint-committed reveal motion. Native ownership,
//! semantic visibility and frame scheduling belong to the host adapter.
use crate::reveal_motion::{Frame, Presentation};
use gpui::{
    AnyElement, App, Bounds, ContentMask, DefiniteLength, Display, Element, ElementId,
    FlexDirection, GlobalElementId, InspectorElementId, IntoElement, LayoutId, Length, Pixels,
    Position, Refineable, Size, Style, StyleRefinement, Window, px, relative,
};

#[derive(Clone, Copy, Debug)]
pub struct Measurement {
    pub natural: Option<Size<Pixels>>,
    pub bounds: Bounds<Pixels>,
}

type Painted = dyn FnMut(&Frame, Measurement, &mut Window, &mut App);

/// Measured reveal is for a panel in ordinary vertical flow. Parent-dependent
/// heights cannot be measured behind a changing height clip without changing
/// their meaning. Preserve those layouts with immediate presentation instead.
/// The host must pass resolved styles (including interaction/container rules).
pub fn can_animate(body: &StyleRefinement, parent: &StyleRefinement) -> bool {
    let mut panel = Style::default();
    panel.refine(body);
    let mut container = Style::default();
    container.refine(parent);
    let relative = |length| matches!(length, Length::Definite(DefiniteLength::Fraction(_)));
    container.display == Display::Flex
        && container.visibility == gpui::Visibility::Visible
        && matches!(
            container.flex_direction,
            FlexDirection::Column | FlexDirection::ColumnReverse
        )
        && container.flex_wrap == gpui::FlexWrap::NoWrap
        && panel.position == Position::Relative
        && panel.display != Display::None
        && panel.visibility == gpui::Visibility::Visible
        && panel.flex_grow == 0.
        && panel.flex_basis == Length::Auto
        && panel.aspect_ratio.is_none()
        && !relative(panel.size.height)
        && !relative(panel.min_size.height)
        && !relative(panel.max_size.height)
        && (panel.flex_shrink == 0.
            || (container.size.height == Length::Auto && container.max_size.height == Length::Auto))
}

/// Transfer the outer box's placement into the clipping layout while leaving
/// decoration and content layout in the body. Natural layout is untouched.
pub fn split_style(body: &mut StyleRefinement, presentation: Presentation) -> Style {
    let Presentation::Height(height) = presentation else {
        return Style {
            display: Display::None,
            ..Style::default()
        };
    };
    let mut original = Style::default();
    original.refine(body);
    let mut outer = Style {
        display: Display::Flex,
        flex_direction: FlexDirection::Column,
        position: original.position,
        inset: original.inset,
        size: original.size,
        min_size: original.min_size,
        max_size: original.max_size,
        margin: original.margin,
        align_self: original.align_self,
        flex_grow: original.flex_grow,
        flex_shrink: original.flex_shrink,
        flex_basis: original.flex_basis,
        grid_location: original.grid_location,
        visibility: original.visibility,
        ..Style::default()
    };
    outer.size.height = px(height as f32).into();
    outer.min_size.height = px(0.).into();
    outer.max_size.height = px(height as f32).into();
    body.position = Some(Position::Relative);
    body.inset.top = Some(Length::Auto);
    body.inset.right = Some(Length::Auto);
    body.inset.bottom = Some(Length::Auto);
    body.inset.left = Some(Length::Auto);
    body.margin.top = Some(px(0.).into());
    body.margin.right = Some(px(0.).into());
    body.margin.bottom = Some(px(0.).into());
    body.margin.left = Some(px(0.).into());
    body.size.width = Some(relative(1.).into());
    body.min_size.width = Some(px(0.).into());
    body.max_size.width = Some(Length::Auto);
    body.flex_basis = Some(Length::Auto);
    body.flex_grow = Some(0.);
    body.flex_shrink = Some(0.);
    body.align_self = Some(gpui::AlignSelf::Stretch);
    body.grid_location = None;
    outer
}

pub struct Reveal {
    id: ElementId,
    body: AnyElement,
    outer: Style,
    frame: Frame,
    painted: Box<Painted>,
}
impl Reveal {
    pub fn new(
        id: impl Into<ElementId>,
        body: AnyElement,
        outer: Style,
        frame: Frame,
        painted: impl FnMut(&Frame, Measurement, &mut Window, &mut App) + 'static,
    ) -> Self {
        Self {
            id: id.into(),
            body,
            outer,
            frame,
            painted: Box::new(painted),
        }
    }
}
impl IntoElement for Reveal {
    type Element = Self;
    fn into_element(self) -> Self {
        self
    }
}
impl Element for Reveal {
    type RequestLayoutState = Option<LayoutId>;
    type PrepaintState = Measurement;
    fn id(&self) -> Option<ElementId> {
        Some(self.id.clone())
    }
    fn source_location(&self) -> Option<&'static std::panic::Location<'static>> {
        None
    }
    fn request_layout(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        window: &mut Window,
        cx: &mut App,
    ) -> (LayoutId, Self::RequestLayoutState) {
        match self.frame.presentation {
            Presentation::Natural => {
                let child = self.body.request_layout(window, cx);
                (child, Some(child))
            }
            Presentation::Height(_) => {
                let child = self.body.request_layout(window, cx);
                (
                    window.request_layout(self.outer.clone(), [child], cx),
                    Some(child),
                )
            }
            Presentation::Closed => (
                window.request_layout(
                    Style {
                        display: Display::None,
                        ..Style::default()
                    },
                    None,
                    cx,
                ),
                None,
            ),
        }
    }
    fn prepaint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>,
        child: &mut Self::RequestLayoutState,
        window: &mut Window,
        cx: &mut App,
    ) -> Self::PrepaintState {
        let natural = child.map(|id| window.layout_bounds(id).size);
        match self.frame.presentation {
            Presentation::Natural => {
                self.body.prepaint(window, cx);
            }
            Presentation::Height(_) => {
                window.with_content_mask(Some(ContentMask { bounds }), |window| {
                    self.body.prepaint(window, cx);
                });
            }
            Presentation::Closed => {}
        }
        Measurement { natural, bounds }
    }
    fn paint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>,
        _: &mut Self::RequestLayoutState,
        measurement: &mut Self::PrepaintState,
        window: &mut Window,
        cx: &mut App,
    ) {
        match self.frame.presentation {
            Presentation::Natural => self.body.paint(window, cx),
            Presentation::Height(_) => window
                .with_content_mask(Some(ContentMask { bounds }), |window| {
                    self.body.paint(window, cx)
                }),
            Presentation::Closed => {}
        }
        (self.painted)(&self.frame, *measurement, window, cx);
    }
}
