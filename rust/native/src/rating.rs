//! Application-controlled ratings; only hover preview is retained natively.
use super::choice::Route;
use gpui::{Context, Div, FocusHandle, Stateful, Window, canvas, div, point, prelude::*, px};
use gpuio_protocol::rating::{Appearance, Config, Request};
use std::{cell::RefCell, rc::Rc, sync::Arc};

#[derive(Default)]
pub(super) struct State {
    pub hovered: Option<i64>,
}

fn editable(route: &Route) -> bool {
    route.gate.borrow().allows(route.node)
        && route
            .session
            .borrow()
            .tree(route.window)
            .and_then(|tree| tree.get(route.node))
            .and_then(|node| node.rating.as_ref())
            .is_some_and(|config| config.can_apply(Request::Increase))
}
fn request(
    route: &Route,
    state: &Rc<RefCell<State>>,
    request: Request,
    owner: gpui::EntityId,
    cx: &mut gpui::App,
) {
    if !route.gate.borrow().allows(route.node) {
        return;
    }
    let event = route.session.borrow().request_rating(
        route.window,
        route.node,
        route.handler,
        route.revision,
        request,
    );
    if let Some(event) = event {
        state.borrow_mut().hovered = None;
        cx.notify(owner);
        if !route.transport.input(event) && route.session.borrow_mut().overload(route.window) {
            route.transport.fault(route.window);
        }
    }
}
fn star(filled: bool, color: Option<i64>) -> impl gpui::IntoElement {
    canvas(
        |_, _, _| (),
        move |bounds, _, window, _| {
            let radius = bounds.size.width.min(bounds.size.height) * 0.44;
            let mut path = if filled {
                gpui::PathBuilder::fill()
            } else {
                gpui::PathBuilder::stroke(px(1.5))
            };
            for index in 0..10 {
                let angle = std::f32::consts::PI * index as f32 / 5. - std::f32::consts::FRAC_PI_2;
                let r = radius * if index % 2 == 0 { 1. } else { 0.46 };
                let position = bounds.center() + point(r * angle.cos(), r * angle.sin());
                if index == 0 {
                    path.move_to(position);
                } else {
                    path.line_to(position);
                }
            }
            path.close();
            if let Ok(path) = path.build() {
                let color = match color {
                    Some(color) => gpui::Hsla::from(gpui::rgba(color as u32)),
                    None => {
                        let foreground = window.text_style().color;
                        if filled {
                            foreground
                        } else {
                            foreground.opacity(0.7)
                        }
                    }
                };
                window.paint_path(path, color);
            }
        },
    )
    .size_full()
}
pub(super) struct Render<'a> {
    pub config: &'a Arc<Config>,
    pub appearance: Option<Appearance>,
    pub state: Rc<RefCell<State>>,
    pub focus: FocusHandle,
    pub route: Option<Route>,
    pub pointer: bool,
}
pub(super) fn element<T: 'static>(
    mut base: Stateful<Div>,
    render: Render<'_>,
    _window: &mut Window,
    cx: &mut Context<T>,
) -> Stateful<Div> {
    let Render {
        config,
        appearance,
        state,
        focus,
        route,
        pointer,
    } = render;
    let owner = cx.entity_id();
    if route.as_ref().is_none_or(|route| !editable(route)) || !pointer {
        state.borrow_mut().hovered = None;
    }
    if state
        .borrow()
        .hovered
        .is_some_and(|value| value > config.maximum)
    {
        state.borrow_mut().hovered = None;
    }
    let value = state.borrow().hovered.unwrap_or(config.value);
    base = base
        .role(gpui::Role::Slider)
        .aria_label(config.label.clone())
        .aria_orientation(gpui::accesskit::Orientation::Horizontal)
        .aria_min_numeric_value(0.)
        .aria_max_numeric_value(config.maximum as f64)
        .aria_numeric_value(config.value as f64)
        .aria_numeric_value_step(1.);
    for index in 1..=config.maximum {
        let mut child = div()
            .id(index as usize)
            .flex_none()
            .w(px(config.star_size as f32))
            .h(px(config.star_size as f32))
            .child(star(
                index <= value,
                appearance.and_then(|appearance| {
                    if index <= value {
                        appearance.active
                    } else {
                        appearance.inactive
                    }
                }),
            ));
        if pointer && let Some(route) = &route {
            let hover_state = state.clone();
            let hover_route = route.clone();
            child = child.cursor_pointer().on_hover(move |hovered, _, cx| {
                if *hovered && editable(&hover_route) && hover_state.borrow().hovered != Some(index)
                {
                    hover_state.borrow_mut().hovered = Some(index);
                    cx.notify(owner);
                }
            });
            let route = route.clone();
            let focus = focus.clone();
            let click_state = state.clone();
            child = child.on_click(move |_, window, cx| {
                if editable(&route) {
                    window.focus(&focus, cx);
                    request(&route, &click_state, Request::Toggle(index), owner, cx);
                    cx.stop_propagation();
                }
            });
        }
        base = base.child(child);
    }
    let leave_state = state.clone();
    base = base.on_hover(move |hovered, _, cx| {
        if !hovered && leave_state.borrow_mut().hovered.take().is_some() {
            cx.notify(owner);
        }
    });
    if let Some(route) = route {
        let key_route = route.clone();
        let key_state = state.clone();
        let maximum = config.maximum;
        base = base.on_key_down(move |event, _, cx| {
            if event.keystroke.modifiers.modified() {
                return;
            }
            let intent = match event.keystroke.key.as_str() {
                "right" | "up" => Request::Increase,
                "left" | "down" => Request::Decrease,
                "home" | "delete" | "backspace" => Request::Set(0),
                "end" => Request::Set(maximum),
                _ => return,
            };
            if editable(&key_route) {
                request(&key_route, &key_state, intent, owner, cx);
                cx.stop_propagation();
            }
        });
        for (action, intent) in [
            (gpui::AccessibleAction::Increment, Request::Increase),
            (gpui::AccessibleAction::Decrement, Request::Decrease),
        ] {
            let route = route.clone();
            let state = state.clone();
            base = base.on_a11y_action(action, move |_, _, cx| {
                request(&route, &state, intent, owner, cx);
                cx.stop_propagation();
            });
        }
        base = base.on_a11y_action(gpui::AccessibleAction::SetValue, move |data, _, cx| {
            if let Some(gpui::accesskit::ActionData::NumericValue(value)) = data
                && value.is_finite()
                && value.fract() == 0.
                && (0. ..=gpuio_protocol::rating::MAX_STARS as f64).contains(value)
            {
                request(&route, &state, Request::Set(*value as i64), owner, cx);
            }
            cx.stop_propagation();
        });
    }
    if !pointer {
        base = base.on_mouse_down(gpui::MouseButton::Left, |_, window, _| {
            window.prevent_default()
        });
    }
    base
}
