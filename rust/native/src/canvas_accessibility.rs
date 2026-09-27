//! Native semantic objects: one bounded child per interactive scene item, only
//! while accessibility is active. Keyboard focus remains on the canvas.
use super::*;
use gpui::{AccessibleAction, Div, Stateful};
use gpuio_protocol::canvas::{HitRegion, Rect};

type Shared = Rc<RefCell<State>>;
#[derive(Clone)]
struct Route {
    state: Shared,
    token: Rc<()>,
    snapshot: std::sync::Weak<Snapshot>,
    item: i64,
}
impl Route {
    fn invoke(&self, activate: bool, window: &mut Window, cx: &mut App) {
        let mut state = self.state.borrow_mut();
        if !state.valid_callback(&self.token, window, false)
            || !self.snapshot.upgrade().is_some_and(|snapshot| {
                state
                    .native
                    .as_ref()
                    .is_some_and(|native| Arc::ptr_eq(native.snapshot(), &snapshot))
            })
        {
            return;
        }
        state.flush_viewport(cx);
        state.cancel_input(window);
        state.enable();
        let center = state.center();
        let native = state.native.as_mut().unwrap();
        let mut events = native.select_item(self.item);
        events.extend(reveal(native, self.item, center));
        if activate {
            events.extend(native.activate(self.item));
        }
        window.focus(&state.input.focus, cx);
        state.emit(events, cx);
        window.refresh();
    }
}
/// Window-local envelope of the transformed hit region after world clipping.
/// Canvas scrolling may leave it outside the viewport; its semantic object
/// remains discoverable. The region is an AX rectangle, not a new hit target.
fn bounds(
    native: &canvas_state::State,
    item: &gpuio_protocol::canvas_scene::Item,
    hit: &HitRegion,
) -> Rect {
    projected_bounds(native.transform(item), native.viewport(), &item.clips, hit)
}
fn projected_bounds(
    transform: gpuio_protocol::canvas::Transform,
    viewport: gpuio_protocol::canvas_view::Viewport,
    clips: &[Rect],
    hit: &HitRegion,
) -> Rect {
    let rect = input::hit_bounds(hit);
    let points = [
        Point {
            x: rect.x,
            y: rect.y,
        },
        Point {
            x: rect.x + rect.width,
            y: rect.y,
        },
        Point {
            x: rect.x,
            y: rect.y + rect.height,
        },
        Point {
            x: rect.x + rect.width,
            y: rect.y + rect.height,
        },
    ]
    .map(|point| transform.apply(point));
    let mut left = points.iter().map(|p| p.x).fold(f64::INFINITY, f64::min);
    let mut top = points.iter().map(|p| p.y).fold(f64::INFINITY, f64::min);
    let mut right = points.iter().map(|p| p.x).fold(f64::NEG_INFINITY, f64::max);
    let mut bottom = points.iter().map(|p| p.y).fold(f64::NEG_INFINITY, f64::max);
    for clip in clips {
        left = left.max(clip.x);
        top = top.max(clip.y);
        right = right.min(clip.x + clip.width);
        bottom = bottom.min(clip.y + clip.height);
    }
    Rect {
        x: (left - viewport.origin.x) * viewport.zoom,
        y: (top - viewport.origin.y) * viewport.zoom,
        width: (right - left).max(0.) * viewport.zoom,
        height: (bottom - top).max(0.) * viewport.zoom,
    }
}
/// Bring an offscreen semantic selection into view without changing zoom.
/// Pan-disabled canvases preserve their application's explicit viewport policy.
pub(super) fn reveal(native: &mut canvas_state::State, id: i64, center: Point) -> Vec<Observation> {
    let Some(item) = native.item(id) else {
        return vec![];
    };
    let rect = bounds(native, item, &item.interaction.as_ref().unwrap().hit_region);
    if rect.width <= 0.
        || rect.height <= 0.
        || center.x <= 0.
        || center.y <= 0.
        || (rect.x >= 0.
            && rect.y >= 0.
            && rect.x + rect.width <= center.x * 2.
            && rect.y + rect.height <= center.y * 2.)
    {
        return vec![];
    }
    native.pan_by(Point {
        x: center.x - (rect.x + rect.width / 2.),
        y: center.y - (rect.y + rect.height / 2.),
    })
}
pub(super) fn objects(
    mut element: Stateful<Div>,
    state: &Shared,
    window: &Window,
) -> Stateful<Div> {
    if !window.is_a11y_active() {
        return element;
    }
    let state_ref = state.borrow();
    let Some(native) = state_ref.native.as_ref().filter(|_| !state_ref.closed) else {
        return element;
    };
    let snapshot = native.snapshot();
    element = element.aria_description(snapshot.scene.description.clone());
    if state_ref.config.selectable {
        element = element.role(gpui::Role::ListBox);
    }
    let Some(source) = state_ref.config.source else {
        return element;
    };
    let disabled = !state_ref.input_allowed(window, false);
    // Keep semantic envelopes out of the outer element's scroll extents.
    // This layer has exactly the same size as the native painting layer.
    let mut objects = div()
        .absolute()
        .top_0()
        .left_0()
        .size_full()
        .overflow_hidden();
    for id in native.interactive_ids() {
        let item = native.item(*id).expect("indexed canvas interaction");
        let interaction = item.interaction.as_ref().unwrap();
        let rect = bounds(native, item, &interaction.hit_region);
        let route = Route {
            state: state.clone(),
            token: state_ref.input.token.clone(),
            snapshot: Arc::downgrade(snapshot),
            item: *id,
        };
        let mut object = div()
            .id(gpui::SharedString::from(format!(
                "canvas-object-{}-{}-{}-{}",
                source.slot(),
                source.generation(),
                snapshot.generation,
                id
            )))
            .absolute()
            .left(px(rect.x as f32))
            .top(px(rect.y as f32))
            .w(px(rect.width as f32))
            .h(px(rect.height as f32))
            .role(if state_ref.config.selectable {
                gpui::Role::ListBoxOption
            } else {
                gpui::Role::GraphicsObject
            })
            .aria_label(interaction.label.clone())
            .aria_selected(native.selection() == Some(*id));
        if interaction.draggable && state_ref.config.draggable {
            object = object.aria_description(
                "Shift+Arrow moves this object; Alt+Shift+Arrow moves by ten pixels.",
            );
        }
        if !disabled {
            if state_ref.config.selectable {
                let select = route.clone();
                object = object.on_a11y_action(AccessibleAction::Click, move |_, window, cx| {
                    select.invoke(false, window, cx)
                });
                let focus = route.clone();
                object = object.on_a11y_action(AccessibleAction::Focus, move |_, window, cx| {
                    focus.invoke(false, window, cx)
                });
            }
            if native.selection() == Some(*id) {
                object = object.aria_active_descendant();
            }
        }
        if interaction.activatable {
            let mut activate = div()
                .id("activate")
                .absolute()
                .size_full()
                .role(gpui::Role::Button)
                .aria_label(format!("Activate {}", interaction.label));
            if !disabled {
                activate = activate
                    .on_a11y_action(AccessibleAction::Click, move |_, window, cx| {
                        route.invoke(true, window, cx)
                    });
            }
            object = object.child(crate::semantics::State {
                hidden: false,
                metadata: None,
                element: activate,
                disabled,
                read_only: false,
                modal: false,
                live: None,
            });
        }
        objects = objects.child(crate::semantics::State {
            hidden: false,
            metadata: None,
            element: object,
            disabled,
            read_only: false,
            modal: false,
            live: None,
        });
    }
    element.child(objects)
}

#[cfg(test)]
mod tests {
    use super::*;
    use gpuio_protocol::{canvas::Transform, canvas_view::Viewport};
    #[test]
    fn semantic_envelope_applies_affine_world_clip_and_viewport_in_order() {
        let rect = projected_bounds(
            Transform {
                a: 0.,
                b: 1.,
                c: -1.,
                d: 0.,
                tx: 100.,
                ty: 20.,
            },
            Viewport {
                origin: Point { x: 80., y: 20. },
                zoom: 2.,
            },
            &[Rect {
                x: 85.,
                y: 22.,
                width: 20.,
                height: 5.,
            }],
            &HitRegion::Rectangle(Rect {
                x: 0.,
                y: 0.,
                width: 10.,
                height: 20.,
            }),
        );
        assert_eq!(
            rect,
            Rect {
                x: 10.,
                y: 4.,
                width: 30.,
                height: 10.
            }
        );
        let clipped = projected_bounds(
            Transform::IDENTITY,
            Viewport::default(),
            &[Rect {
                x: 100.,
                y: 100.,
                width: 10.,
                height: 10.,
            }],
            &HitRegion::Polygon(vec![
                Point { x: 0., y: 0. },
                Point { x: 10., y: 0. },
                Point { x: 0., y: 10. },
            ]),
        );
        assert_eq!((clipped.width, clipped.height), (0., 0.));
    }
}
