//! Deferred surfaces resolve the current frame's trigger bounds during prepaint,
//! after ordinary layout. This avoids positioning from a previous-frame cache.
use gpui::{
    AnyElement, App, Bounds, Element, GlobalElementId, InspectorElementId, IntoElement, LayoutId,
    Pixels, Point, Size, Window, px,
};
use gpuio_protocol::v1::{Align, Placement, Side};
use std::{cell::Cell, rc::Rc};

pub(super) struct Surface {
    pub trigger: Rc<Cell<Bounds<Pixels>>>,
    pub placement: Placement,
    pub content: AnyElement,
}

fn origin(
    trigger: Bounds<Pixels>,
    size: Size<Pixels>,
    viewport: Size<Pixels>,
    margin: Pixels,
    placement: Placement,
) -> Point<Pixels> {
    let vertical = matches!(placement.side, Side::Top | Side::Bottom);
    let (near, far, extent, viewport_extent, cross_near, cross_far, cross_extent) = if vertical {
        (
            trigger.top(),
            trigger.bottom(),
            size.height,
            viewport.height,
            trigger.left(),
            trigger.right(),
            size.width,
        )
    } else {
        (
            trigger.left(),
            trigger.right(),
            size.width,
            viewport.width,
            trigger.top(),
            trigger.bottom(),
            size.height,
        )
    };
    let gap = px(placement.offset as f32);
    let before = near - extent - gap;
    let after = far + gap;
    let prefer_before = matches!(placement.side, Side::Top | Side::Left);
    let (preferred, alternate, available, alternate_available) = if prefer_before {
        (before, after, near - margin, viewport_extent - margin - far)
    } else {
        (after, before, viewport_extent - margin - far, near - margin)
    };
    let fits = |origin| origin >= margin && origin + extent <= viewport_extent - margin;
    let primary = if !fits(preferred) && (fits(alternate) || alternate_available > available) {
        alternate
    } else {
        preferred
    };
    let cross = match placement.align {
        Align::Start => cross_near,
        Align::Center => cross_near + (cross_far - cross_near - cross_extent) / 2.,
        Align::End => cross_far - cross_extent,
    };
    let desired = if vertical {
        Point::new(cross, primary)
    } else {
        Point::new(primary, cross)
    };
    Point::new(
        desired
            .x
            .min(viewport.width - margin - size.width)
            .max(margin),
        desired
            .y
            .min(viewport.height - margin - size.height)
            .max(margin),
    )
}

impl IntoElement for Surface {
    type Element = Self;
    fn into_element(self) -> Self {
        self
    }
}
impl Element for Surface {
    type RequestLayoutState = LayoutId;
    type PrepaintState = ();
    fn id(&self) -> Option<gpui::ElementId> {
        None
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
    ) -> (LayoutId, LayoutId) {
        let child = self.content.request_layout(window, cx);
        let layout = window.request_layout(
            gpui::Style {
                position: gpui::Position::Absolute,
                display: gpui::Display::Flex,
                ..Default::default()
            },
            [child],
            cx,
        );
        (layout, child)
    }
    fn prepaint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        _: Bounds<Pixels>,
        layout: &mut LayoutId,
        window: &mut Window,
        cx: &mut App,
    ) {
        let child = window.layout_bounds(*layout);
        let margin = px(8.) + window.client_inset().unwrap_or(px(0.));
        let desired = origin(
            self.trigger.get(),
            child.size,
            window.viewport_size(),
            margin,
            self.placement,
        );
        let offset = desired - child.origin;
        window.with_element_offset(offset, |window| {
            self.content.prepaint(window, cx);
        });
    }
    fn paint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        _: Bounds<Pixels>,
        _: &mut LayoutId,
        _: &mut (),
        window: &mut Window,
        cx: &mut App,
    ) {
        self.content.paint(window, cx);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use gpui::{point, size};
    #[test]
    fn popup_flips_and_clamps_using_current_trigger_geometry() {
        let viewport = size(px(400.), px(300.));
        let popup = size(px(200.), px(100.));
        let trigger = |x, y| Bounds::new(point(px(x), px(y)), size(px(120.), px(40.)));
        assert_eq!(
            origin(
                trigger(20., 20.),
                popup,
                viewport,
                px(8.),
                Placement::default()
            ),
            point(px(20.), px(60.))
        );
        assert_eq!(
            origin(
                trigger(300., 220.),
                popup,
                viewport,
                px(8.),
                Placement::default()
            ),
            point(px(192.), px(120.))
        );
        assert_eq!(
            origin(
                trigger(-10., -100.),
                popup,
                viewport,
                px(8.),
                Placement::default()
            ),
            point(px(8.), px(8.))
        );
    }
    #[test]
    fn preferred_side_alignment_offset_and_flip_are_consistent() {
        let trigger = Bounds::new(point(px(200.), px(150.)), size(px(100.), px(40.)));
        let popup = size(px(80.), px(60.));
        let viewport = size(px(500.), px(400.));
        for (side, align, x, y) in [
            (Side::Top, Align::Start, 200., 90.),
            (Side::Top, Align::Center, 210., 90.),
            (Side::Top, Align::End, 220., 90.),
            (Side::Bottom, Align::Center, 210., 190.),
            (Side::Left, Align::Start, 120., 150.),
            (Side::Left, Align::Center, 120., 140.),
            (Side::Right, Align::End, 300., 130.),
        ] {
            assert_eq!(
                origin(
                    trigger,
                    popup,
                    viewport,
                    px(8.),
                    Placement {
                        side,
                        align,
                        offset: 0.
                    }
                ),
                point(px(x), px(y))
            );
        }
        assert_eq!(
            origin(
                trigger,
                popup,
                viewport,
                px(8.),
                Placement {
                    side: Side::Top,
                    align: Align::Center,
                    offset: 10.
                }
            ),
            point(px(210.), px(80.))
        );
        assert_eq!(
            origin(
                trigger,
                popup,
                viewport,
                px(8.),
                Placement {
                    side: Side::Top,
                    align: Align::Center,
                    offset: -5.
                }
            ),
            point(px(210.), px(95.))
        );
        let edge = Bounds::new(point(px(5.), px(10.)), trigger.size);
        assert_eq!(
            origin(
                edge,
                popup,
                viewport,
                px(8.),
                Placement {
                    side: Side::Top,
                    align: Align::Start,
                    offset: 10.
                }
            ),
            point(px(8.), px(60.))
        );
        assert_eq!(
            origin(
                edge,
                popup,
                viewport,
                px(8.),
                Placement {
                    side: Side::Left,
                    align: Align::End,
                    offset: 0.
                }
            ),
            point(px(105.), px(8.))
        );
    }
}
