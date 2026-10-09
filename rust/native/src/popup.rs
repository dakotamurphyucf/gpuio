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
    pub geometry: Option<gpuio_protocol::placement_geometry::Config>,
    pub content: AnyElement,
}

fn origin(
    trigger: Bounds<Pixels>,
    size: Size<Pixels>,
    viewport: Size<Pixels>,
    margin: Pixels,
    placement: Placement,
) -> Point<Pixels> {
    let margins = margins(viewport, margin);
    let vertical = matches!(placement.side, Side::Top | Side::Bottom);
    let margin = if vertical {
        margins.height
    } else {
        margins.width
    };
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
    clamp(desired, size, viewport, margins)
}

fn margins(viewport: Size<Pixels>, margin: Pixels) -> Size<Pixels> {
    gpui::size(
        margin.min(viewport.width / 2.).max(px(0.)),
        margin.min(viewport.height / 2.).max(px(0.)),
    )
}
fn clamp(
    desired: Point<Pixels>,
    size: Size<Pixels>,
    viewport: Size<Pixels>,
    margins: Size<Pixels>,
) -> Point<Pixels> {
    Point::new(
        desired
            .x
            .min(viewport.width - margins.width - size.width)
            .max(margins.width),
        desired
            .y
            .min(viewport.height - margins.height - size.height)
            .max(margins.height),
    )
}
fn resolve(
    trigger: Bounds<Pixels>,
    size: Size<Pixels>,
    content: Bounds<Pixels>,
    placement: Placement,
    geometry: Option<gpuio_protocol::placement_geometry::Config>,
) -> Point<Pixels> {
    use gpuio_protocol::placement_geometry::Corner;
    let viewport = content.size;
    let margin = px(geometry.map_or(8., |g| g.viewport_margin) as f32);
    let local = match geometry.and_then(|g| g.point) {
        None => origin(
            Bounds::new(trigger.origin - content.origin, trigger.size),
            size,
            viewport,
            margin,
            placement,
        ),
        Some(p) => {
            // Explicit points remain window coordinates; only fitting is local.
            let x = px(p.x as f32)
                - content.origin.x
                - if matches!(p.corner, Corner::TopRight | Corner::BottomRight) {
                    size.width
                } else {
                    px(0.)
                };
            let y = px(p.y as f32)
                - content.origin.y
                - if matches!(p.corner, Corner::BottomLeft | Corner::BottomRight) {
                    size.height
                } else {
                    px(0.)
                };
            clamp(Point::new(x, y), size, viewport, margins(viewport, margin))
        }
    };
    content.origin + local
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
        let desired = resolve(
            self.trigger.get(),
            child.size,
            crate::window_frame::content_bounds(window),
            self.placement,
            self.geometry,
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
    fn asymmetric_content_translates_anchors_but_keeps_explicit_points_in_window_space() {
        use gpuio_protocol::placement_geometry::{Config, Corner, Point as WirePoint};
        let content = Bounds::new(point(px(21.), px(0.)), size(px(479.), px(379.)));
        let trigger = Bounds::new(point(px(30.), px(50.)), size(px(100.), px(20.)));
        let popup = size(px(80.), px(60.));
        assert_eq!(
            resolve(trigger, popup, content, Placement::default(), None),
            point(px(30.), px(70.))
        );
        let config = |x, y| {
            Some(Config {
                viewport_margin: 8.,
                point: Some(WirePoint {
                    corner: Corner::TopLeft,
                    x,
                    y,
                }),
            })
        };
        assert_eq!(
            resolve(
                trigger,
                popup,
                content,
                Placement::default(),
                config(200., 150.)
            ),
            point(px(200.), px(150.))
        );
        assert_eq!(
            resolve(
                trigger,
                popup,
                content,
                Placement::default(),
                config(-1e6, 1e6)
            ),
            point(px(29.), px(311.))
        );
    }
    #[test]
    fn corner_points_never_flip_and_margins_use_current_content_bounds() {
        use gpuio_protocol::placement_geometry::{Config, Corner, Point as WirePoint};
        let trigger = Bounds::new(point(px(200.), px(380.)), size(px(0.), px(0.)));
        let popup = size(px(80.), px(60.));
        let viewport = size(px(500.), px(400.));
        for (corner, x, y) in [
            (Corner::TopLeft, 200., 150.),
            (Corner::TopRight, 120., 150.),
            (Corner::BottomLeft, 200., 90.),
            (Corner::BottomRight, 120., 90.),
        ] {
            assert_eq!(
                resolve(
                    trigger,
                    popup,
                    Bounds::new(Point::default(), viewport),
                    Placement::default(),
                    Some(Config {
                        viewport_margin: 8.,
                        point: Some(WirePoint {
                            corner,
                            x: 200.,
                            y: 150.
                        })
                    })
                ),
                point(px(x), px(y))
            );
        }
        let corner = Some(Config {
            viewport_margin: 8.,
            point: Some(WirePoint {
                corner: Corner::TopLeft,
                x: 200.,
                y: 380.,
            }),
        });
        assert_eq!(
            resolve(
                trigger,
                popup,
                Bounds::new(Point::default(), viewport),
                Placement::default(),
                corner
            ),
            point(px(200.), px(332.))
        );
        assert_eq!(
            resolve(
                trigger,
                popup,
                Bounds::new(Point::default(), viewport),
                Placement::default(),
                None
            ),
            point(px(200.), px(320.))
        );
        let large = Some(Config {
            viewport_margin: 40.,
            point: Some(WirePoint {
                corner: Corner::TopLeft,
                x: 1e6,
                y: 1e6,
            }),
        });
        assert_eq!(
            resolve(
                trigger,
                popup,
                Bounds::new(point(px(5.), px(5.)), size(px(490.), px(390.))),
                Placement::default(),
                large
            ),
            point(px(375.), px(295.))
        );
        let extreme = Some(Config {
            viewport_margin: 16384.,
            point: Some(WirePoint {
                corner: Corner::BottomRight,
                x: -1e6,
                y: -1e6,
            }),
        });
        assert_eq!(
            resolve(
                trigger,
                size(px(300.), px(200.)),
                Bounds::new(point(px(5.), px(5.)), size(px(190.), px(90.))),
                Placement::default(),
                extreme
            ),
            point(px(100.), px(50.))
        );
        assert_eq!(
            resolve(
                trigger,
                popup,
                Bounds::new(Point::default(), viewport),
                Placement::default(),
                Some(Config {
                    viewport_margin: 0.,
                    point: Some(WirePoint {
                        corner: Corner::TopLeft,
                        x: -1e6,
                        y: -1e6
                    })
                })
            ),
            point(px(0.), px(0.))
        );
    }
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
