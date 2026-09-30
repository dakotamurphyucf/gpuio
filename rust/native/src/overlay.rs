//! Application-controlled surfaces; placement, occlusion and dismissal routing
//! stay native. Dismissal never releases a trap before an accepted tree update.
use super::choice::Route;
use gpui::{
    AnyElement, App, Bounds, Div, Element, GlobalElementId, InspectorElementId, IntoElement,
    LayoutId, Pixels, Stateful, Window, deferred, div, prelude::*, px, rgba,
};
use gpuio_protocol::v1::*;
use std::{cell::Cell, rc::Rc, sync::Arc};

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

/// Dynamic hover/focus refinements run after the panel builder. Sheets own
/// geometry in those states too; visual/content styling remains unrestricted.
pub(super) fn constrain_state_style(kind: OverlayKind, style: &mut gpui::StyleRefinement) {
    match kind {
        OverlayKind::SheetLeft
        | OverlayKind::SheetRight
        | OverlayKind::SheetTop
        | OverlayKind::SheetBottom => {
            style.size = Default::default();
            style.min_size = Default::default();
            style.max_size = Default::default();
            style.margin = Default::default();
            style.inset = Default::default();
            style.position = None;
            style.flex_grow = None;
            style.flex_shrink = None;
            style.flex_basis = None;
        }
        OverlayKind::Dialog | OverlayKind::Popover | OverlayKind::AlertDialog => {}
    }
}

pub(super) fn element(
    mut panel: Stateful<Div>,
    config: Arc<OverlayConfig>,
    placement: Placement,
    route: Route,
    scrolling: Option<&std::rc::Rc<super::scroll::State>>,
    window: &Window,
    node: &crate::tree::Node,
) -> AnyElement {
    // Retained hidden panels intentionally retire active overlay scopes. Do not
    // create a deferred surface (or require an anchor) until it is visible again.
    if !route.gate.borrow().interactive(route.node) {
        return div().into_any_element();
    }
    let priority = route.gate.borrow().layer(route.node);
    let modal = config.kind.is_modal();
    let viewport = window.viewport_size();
    // Edge-attached sheets own geometry; arbitrary panel dimensions cannot move
    // their trap offscreen or leave a gap along the attached edge.
    let sheet_size = match config.kind {
        OverlayKind::SheetLeft | OverlayKind::SheetRight => Some(gpui::size(
            px(config.width as f32).min(viewport.width),
            viewport.height,
        )),
        OverlayKind::SheetTop | OverlayKind::SheetBottom => Some(gpui::size(
            viewport.width,
            px(config.width as f32).min(viewport.height),
        )),
        OverlayKind::Dialog | OverlayKind::Popover | OverlayKind::AlertDialog => None,
    };
    if let Some(size) = sheet_size {
        panel = panel
            .w(size.width)
            .h(size.height)
            .min_w(size.width)
            .max_w(size.width)
            .min_h(size.height)
            .max_h(size.height)
            .m_0()
            .relative()
            .flex_shrink_0();
        panel.style().inset = Default::default();
    }
    let key = route.clone();
    let action = route.clone();
    let outside = route.clone();
    let escape = config.dismiss_on_escape;
    // These are bubbling listeners: child native widgets consume their own Escape.
    panel = panel
        .role(if config.kind == OverlayKind::AlertDialog {
            gpui::Role::AlertDialog
        } else {
            gpui::Role::Dialog
        })
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
    let panel = match scrolling {
        Some(state) => super::highlight_style::Frame::new(
            super::scroll::Frame::new(panel, state, route.gate.clone(), node.id),
            node,
            &route.gate,
        )
        .into_any_element(),
        None => super::highlight_style::Frame::new(panel, node, &route.gate).into_any_element(),
    };
    let bounds = Rc::new(Cell::new(Bounds::default()));
    route.gate.borrow_mut().surface(route.node, bounds.clone());
    let panel = SurfaceBounds {
        content: panel,
        bounds,
    };
    let panel = crate::semantics::State {
        hidden: false,
        metadata: None,
        live: None,
        element: panel,
        disabled: false,
        read_only: false,
        modal,
    }
    .into_any_element();
    if modal {
        let backdrop = div()
            .w(viewport.width)
            .h(viewport.height)
            .flex()
            .map(|backdrop| match config.kind {
                OverlayKind::SheetLeft => backdrop.items_start().justify_start(),
                OverlayKind::SheetRight => backdrop.items_start().justify_end(),
                OverlayKind::SheetTop => backdrop.items_start().justify_start(),
                OverlayKind::SheetBottom => backdrop.items_end().justify_start(),
                OverlayKind::Dialog | OverlayKind::AlertDialog => {
                    backdrop.items_center().justify_center().p(px(16.))
                }
                OverlayKind::Popover => unreachable!("nonmodal surface"),
            })
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
            placement,
            content: panel,
        })
        .with_priority(priority)
        .into_any_element()
    }
}

/// Measure the panel itself, outside its scrolling content. Logical ancestors
/// must recognize a deferred popup even when it extends beyond their layout.
struct SurfaceBounds {
    content: AnyElement,
    bounds: Rc<Cell<Bounds<Pixels>>>,
}
impl IntoElement for SurfaceBounds {
    type Element = Self;
    fn into_element(self) -> Self {
        self
    }
}
impl Element for SurfaceBounds {
    type RequestLayoutState = ();
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
    ) -> (LayoutId, ()) {
        (self.content.request_layout(window, cx), ())
    }
    fn prepaint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>,
        _: &mut (),
        window: &mut Window,
        cx: &mut App,
    ) {
        self.bounds
            .set(bounds.intersect(&window.content_mask().bounds));
        self.content.prepaint(window, cx);
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
        self.content.paint(window, cx);
    }
}

/// A zero-layout deferred surface positioned in viewport coordinates using this
/// frame's layout. It does not inherit the mounting ancestor's scroll offset.
pub(super) struct ViewportSurface {
    pub(super) content: AnyElement,
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
