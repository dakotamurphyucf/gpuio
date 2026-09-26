//! In-window tree moves carry native row identity, never application payloads.
use super::View;
use gpui::{
    App, Bounds, Context, Div, Pixels, Render, SharedString, Stateful, Window, canvas, div,
    prelude::*, rgba,
};
use gpuio_protocol::{
    HandlerId, NodeId, WindowId,
    list::Row,
    tree_input::{Placement, Request},
};
use std::{
    cell::{Cell, RefCell},
    rc::{Rc, Weak},
};

#[derive(Clone, Copy)]
struct Route {
    window: WindowId,
    owner: NodeId,
    handler: HandlerId,
    row: Row,
}

pub(super) struct Lease {
    route: Route,
    cancelled: Cell<bool>,
}
struct Drag {
    route: Route,
    label: SharedString,
    lease: RefCell<Weak<Lease>>,
}
struct Preview {
    label: SharedString,
    _lease: Rc<Lease>,
}
impl Render for Preview {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div()
            .px_3()
            .py_1()
            .rounded_md()
            .bg(rgba(0x253044ef))
            .text_color(rgba(0xffffffff))
            .child(self.label.clone())
    }
}
fn placement(bounds: Bounds<Pixels>, y: Pixels, branch: bool) -> Placement {
    let fraction = (y - bounds.origin.y) / bounds.size.height;
    if fraction < if branch { 0.25 } else { 0.5 } {
        Placement::Before
    } else if branch && fraction < 0.75 {
        Placement::Inside
    } else {
        Placement::After
    }
}
impl View {
    fn tree_drag_current(&self, route: Route, window: &Window) -> bool {
        if route.window != self.id
            || !window.is_window_active()
            || !self.focus.borrow().allows(route.owner)
            || !self.focus.borrow().allows(route.row.node)
        {
            return false;
        }
        let session = self.session.borrow();
        let Some(tree) = session.tree(self.id) else {
            return false;
        };
        let Some(owner) = tree.get(route.owner) else {
            return false;
        };
        if !owner.tree_moves
            || !owner.tree_input
            || owner.handler != Some(route.handler)
            || !owner.list_rows.contains(&route.row)
        {
            return false;
        }
        tree.get(route.row.node).is_some_and(|node| {
            super::pointer_enabled(tree, node.id)
                && node.accessibility.as_ref().is_some_and(|metadata| {
                    matches!(metadata.role, Some(gpuio_protocol::accessibility::Role::TreeItem(item)) if !item.disabled)
                })
        })
    }

    pub(super) fn cancel_tree_drag(&mut self, window: &mut Window, cx: &mut App) -> bool {
        if let Some(lease) = self.tree_drag.upgrade() {
            lease.cancelled.set(true);
            self.tree_drag = Weak::new();
            cx.stop_active_drag(window);
            window.refresh();
            true
        } else {
            false
        }
    }
    pub(super) fn sync_tree_drag(&mut self, window: &mut Window, cx: &mut App) {
        if let Some(lease) = self.tree_drag.upgrade()
            && (lease.cancelled.get() || !self.tree_drag_current(lease.route, window))
        {
            self.cancel_tree_drag(window, cx);
        }
    }
    fn accepts_tree_drag(&self, drag: &Drag, destination: Route, window: &Window) -> bool {
        drag.lease
            .borrow()
            .upgrade()
            .is_some_and(|lease| !lease.cancelled.get())
            && drag.route.window == destination.window
            && drag.route.owner == destination.owner
            && drag.route.handler == destination.handler
            && drag.route.row.id != destination.row.id
            && self.tree_drag_current(drag.route, window)
            && self.tree_drag_current(destination, window)
    }
    pub(super) fn tree_row_drag(
        &mut self,
        row: Stateful<Div>,
        owner: NodeId,
        binding: Row,
        branch: bool,
        cx: &mut Context<Self>,
    ) -> Stateful<Div> {
        let route = {
            let session = self.session.borrow();
            let tree = session.tree(self.id).expect("live window");
            let node = tree.get(owner).expect("live tree");
            if !node.tree_moves {
                return row;
            }
            Route {
                window: self.id,
                owner,
                handler: node.handler.expect("tree input handler"),
                row: binding,
            }
        };
        let label = self
            .session
            .borrow()
            .tree(self.id)
            .and_then(|tree| tree.get(binding.node))
            .and_then(|node| node.accessibility.as_ref())
            .and_then(|metadata| metadata.label.as_ref())
            .cloned()
            .unwrap_or_else(|| "Tree item".into());
        let weak = cx.weak_entity();
        let drop_view = weak.clone();
        let accept_view = weak.clone();
        let bounds = Rc::new(Cell::new(None::<Bounds<Pixels>>));
        let captured = bounds.clone();
        row.on_drag(
            Drag {
                route,
                label: label.into(),
                lease: RefCell::new(Weak::new()),
            },
            move |drag, _, window, cx| {
                let lease = Rc::new(Lease {
                    route: drag.route,
                    cancelled: Cell::new(false),
                });
                *drag.lease.borrow_mut() = Rc::downgrade(&lease);
                let valid = weak
                    .update(cx, |view, _| {
                        view.tree_drag = Rc::downgrade(&lease);
                        view.tree_drag_current(drag.route, window)
                            && window.captured_hitbox().is_none()
                    })
                    .unwrap_or(false);
                if !valid {
                    lease.cancelled.set(true);
                    let weak = weak.clone();
                    window.defer(cx, move |window, cx| {
                        let _ = weak.update(cx, |view, cx| {
                            view.sync_tree_drag(window, cx);
                        });
                    });
                }
                cx.new(|_| Preview {
                    label: drag.label.clone(),
                    _lease: lease,
                })
            },
        )
        .can_drop(move |value, window, cx| {
            value.downcast_ref::<Drag>().is_some_and(|drag| {
                accept_view
                    .update(cx, |view, _| view.accepts_tree_drag(drag, route, window))
                    .unwrap_or(false)
            })
        })
        .drag_over::<Drag>(|style, _, _, _| style.bg(rgba(0x6688ff30)))
        .on_drop(move |drag: &Drag, window, cx| {
            let _ = drop_view.update(cx, |view, cx| {
                if view.accepts_tree_drag(drag, route, window)
                    && let Some(bounds) = bounds.get()
                    && bounds.contains(&window.mouse_position())
                {
                    view.tree_request(
                        owner,
                        Request::Move {
                            source: drag.route.row.id,
                            destination: binding.id,
                            placement: placement(bounds, window.mouse_position().y, branch),
                        },
                    );
                    cx.stop_propagation();
                }
            });
        })
        .child(
            canvas(
                move |bounds, _, _| {
                    captured.set(Some(bounds));
                },
                |_, _, _, _| (),
            )
            .absolute()
            .top_0()
            .left_0()
            .size_full(),
        )
    }
}
