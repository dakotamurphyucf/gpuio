//! Deferred surfaces resolve the current frame's trigger bounds during prepaint,
//! after ordinary layout. This avoids positioning from a previous-frame cache.
use gpui::{
    AnyElement, App, Bounds, Element, GlobalElementId, InspectorElementId, IntoElement, LayoutId,
    Pixels, Point, Size, Window, px,
};
use std::{cell::Cell, rc::Rc};

pub(super) struct Surface {
    pub trigger: Rc<Cell<Bounds<Pixels>>>,
    pub content: AnyElement,
}

fn origin(
    trigger: Bounds<Pixels>,
    size: Size<Pixels>,
    viewport: Size<Pixels>,
    margin: Pixels,
) -> Point<Pixels> {
    let below = trigger.bottom();
    let above = trigger.top() - size.height;
    let y = if below + size.height <= viewport.height - margin {
        below
    } else if above >= margin || trigger.top() > viewport.height - trigger.bottom() {
        above
    } else {
        below
    };
    Point::new(
        trigger
            .left()
            .min(viewport.width - margin - size.width)
            .max(margin),
        y.min(viewport.height - margin - size.height).max(margin),
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
            origin(trigger(20., 20.), popup, viewport, px(8.)),
            point(px(20.), px(60.))
        );
        assert_eq!(
            origin(trigger(300., 220.), popup, viewport, px(8.)),
            point(px(192.), px(120.))
        );
        assert_eq!(
            origin(trigger(-10., -100.), popup, viewport, px(8.)),
            point(px(8.), px(8.))
        );
    }
}
