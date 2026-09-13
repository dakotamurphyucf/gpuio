//! Application-controlled surfaces; placement, occlusion and dismissal routing
//! stay native. Dismissal never releases a trap before an accepted tree update.
use super::choice::Route;
use gpui::{
    AnyElement, App, Bounds, Div, Element, GlobalElementId, InspectorElementId, IntoElement,
    LayoutId, Pixels, Stateful, Window, deferred, div, prelude::*, px, rgba,
};
use gpuio_protocol::v1::*;
use std::sync::Arc;

fn dismiss(route: &Route, reason: Dismissal) {
    if !route.gate.borrow().top_overlay(route.node) {
        return;
    }
    let event = route.session.borrow().dismiss(
        route.window,
        route.node,
        route.handler,
        route.revision,
        reason,
    );
    if let Some(event) = event
        && !route.transport.input(event)
        && route.session.borrow_mut().overload(route.window)
    {
        route.transport.fault(route.window);
    }
}

pub(super) fn element(
    mut panel: Stateful<Div>,
    config: Arc<OverlayConfig>,
    route: Route,
    window: &Window,
) -> AnyElement {
    let priority = route.gate.borrow().layer(route.node);
    let modal = config.kind == OverlayKind::Dialog;
    let key = route.clone();
    let action = route.clone();
    let outside = route.clone();
    let escape = config.dismiss_on_escape;
    // These are bubbling listeners: child native widgets consume their own Escape.
    panel = panel
        .role(gpui::Role::Dialog)
        .aria_label(config.label.clone())
        .occlude()
        .on_key_down(move |event, _, cx| {
            if event.keystroke.key == "escape"
                && !event.keystroke.modifiers.modified()
                && key.gate.borrow().top_overlay(key.node)
            {
                if escape {
                    dismiss(&key, Dismissal::Escape);
                }
                cx.stop_propagation();
            }
        })
        .on_action(move |_: &gpui_base::input::Escape, _, cx| {
            if action.gate.borrow().top_overlay(action.node) {
                if escape {
                    dismiss(&action, Dismissal::Escape);
                }
                cx.stop_propagation();
            } else {
                cx.propagate();
            }
        })
        .on_mouse_down_out(move |event, _, _| {
            if !outside
                .gate
                .borrow()
                .surface_contains(outside.node, event.position)
            {
                dismiss(&outside, Dismissal::OutsidePointer);
            }
        })
        .on_any_mouse_down(|_, _, cx| cx.stop_propagation())
        .on_scroll_wheel(|_, _, cx| cx.stop_propagation());
    let panel = crate::semantics::State {
        element: panel,
        disabled: false,
        read_only: false,
        modal,
    }
    .into_any_element();
    if modal {
        let viewport = window.viewport_size();
        let backdrop = div()
            .w(viewport.width)
            .h(viewport.height)
            .flex()
            .items_center()
            .justify_center()
            .p(px(16.))
            .bg(rgba(0x00000080))
            .occlude()
            .on_any_mouse_down(|_, window, cx| {
                window.prevent_default();
                cx.stop_propagation();
            })
            .on_scroll_wheel(|_, _, cx| cx.stop_propagation())
            .child(panel);
        deferred(ViewportSurface {
            content: backdrop.into_any_element(),
        })
        .with_priority(priority)
        .into_any_element()
    } else {
        let trigger = route
            .gate
            .borrow()
            .anchor(route.node)
            .expect("mounted overlay scope");
        deferred(super::popup::Surface {
            trigger,
            content: panel,
        })
        .with_priority(priority)
        .into_any_element()
    }
}

/// A zero-layout deferred surface positioned in viewport coordinates using this
/// frame's layout. It does not inherit the mounting ancestor's scroll offset.
struct ViewportSurface {
    content: AnyElement,
}
impl IntoElement for ViewportSurface {
    type Element = Self;
    fn into_element(self) -> Self {
        self
    }
}
impl Element for ViewportSurface {
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
        let offset = gpui::point(px(0.), px(0.)) - window.layout_bounds(*layout).origin;
        window.with_element_offset(offset, |window| self.content.prepaint(window, cx));
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
