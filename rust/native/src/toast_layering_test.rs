//! Production layered Host on TestPlatform. No physical desktop acceptance.
use super::*;
use gpui::{Entity, TestAppContext, VisualTestContext};
use gpuio_protocol::{HandlerId, toast_layering::Layering};
use std::{os::fd::AsRawFd, os::unix::net::UnixStream};
fn n(i: i64) -> NodeId {
    NodeId::from_parts(i, 1).unwrap()
}
fn w() -> WindowId {
    WindowId::from_parts(0, 1).unwrap()
}
fn layering() -> Layering {
    Layering {
        peek: 14.,
        gap: 14.,
        width_step: 0.05,
        visible: 3,
    }
}
fn draw(cx: &mut VisualTestContext) {
    cx.run_until_parked();
    cx.update(|window, cx| window.draw(cx).clear(cx));
    cx.run_until_parked();
}
fn apply(owner: &Entity<View>, cx: &mut VisualTestContext, operations: Vec<Op>) {
    cx.update(|window, cx| {
        owner.update(cx, |v, cx| {
            let base = v.session.borrow().tree(w()).unwrap().revision();
            let applied = v
                .session
                .borrow_mut()
                .apply(&Transaction {
                    window: w(),
                    base,
                    revision: base + 1,
                    operations,
                })
                .unwrap();
            v.update_editors(&applied.dirty, window, cx);
            cx.notify();
        })
    });
    draw(cx);
}
fn initial() -> Vec<Op> {
    let mut ops = vec![
        Op::Create(n(0), Kind::ToastStack, "".into(), None),
        Op::SetToastStack(
            n(0),
            ToastStackConfig {
                label: "Activity notifications".into(),
                corner: ToastCorner::BottomRight,
                width: 240.,
                max_visible: 3,
            },
        ),
        Op::SetToastLayering(n(0), Some(layering())),
    ];
    for slot in [1, 3, 5] {
        ops.extend([
            Op::Create(
                n(slot),
                Kind::Toast,
                "".into(),
                Some(HandlerId::from_parts(slot, 1).unwrap()),
            ),
            Op::SetToast(
                n(slot),
                ToastConfig {
                    label: format!("Saved {slot}"),
                    close_label: format!("Dismiss {slot}"),
                    timeout_ns: Some(60_000_000_000),
                    politeness: ToastPoliteness::Polite,
                },
            ),
            Op::Create(
                n(slot + 1),
                Kind::Input,
                format!("Draft {slot}"),
                Some(HandlerId::from_parts(slot + 1, 1).unwrap()),
            ),
            Op::SetEditor(
                n(slot + 1),
                EditorConfig {
                    label: format!("Draft {slot}"),
                    placeholder: String::new(),
                    read_only: false,
                    disabled: false,
                    submit_on_enter: false,
                    auto_focus: false,
                    min_rows: 1,
                    max_rows: 1,
                },
            ),
            Op::Splice(n(slot), 0, 0, vec![n(slot + 1)]),
        ]);
    }
    ops.extend([
        Op::Splice(n(0), 0, 0, vec![n(1), n(3), n(5)]),
        Op::SetRoot(Some(n(0))),
    ]);
    ops
}
#[test]
fn layering_expands_by_scope_focus_retains_editors_and_resets_without_remount() {
    let mut app = TestAppContext::single();
    app.update(gpui_base::init);
    let (_reader, writer) = UnixStream::pair().unwrap();
    let transport = Arc::new(Transport::new(writer.as_raw_fd()).unwrap());
    let session = Rc::new(RefCell::new(Session::default()));
    session.borrow_mut().hello(VERSION, CAPABILITIES).unwrap();
    session
        .borrow_mut()
        .open(1, w(), "Layered", 400., 300.)
        .unwrap();
    let (owner, cx) =
        app.add_window_view(|_, _| View::new(w(), session.clone(), transport.clone()));
    cx.simulate_a11y_active(true);
    cx.update(|window, _| window.activate_window());
    apply(&owner, cx, initial());
    let old = owner.read_with(cx, |v, cx| v.editors[&n(2)].focus_handle(cx));
    let front = owner.read_with(cx, |v, cx| v.editors[&n(6)].focus_handle(cx));
    let scope = owner.read_with(cx, |v, _| v.focus.borrow().handle(n(0)).unwrap());
    assert!(
        !cx.update(|window, cx| scope.contains_focused(window, cx)),
        "mount must never autofocus"
    );
    owner.read_with(cx, |v, _| {
        assert!(!v.focus.borrow().allows(n(2)));
        assert!(!v.focus.borrow().allows(n(4)));
        assert!(v.focus.borrow().allows(n(6)));
        assert_eq!(
            v.toasts[&n(1)].status(),
            (false, false),
            "decorative card pauses expiry"
        );
    });
    assert!(cx.update(|window, cx| owner.read(cx).focus.borrow().can_focus(&scope, window)));
    let front_bounds = owner.read_with(cx, |v, _| v.toasts[&n(5)].bounds.get());
    cx.simulate_mouse_move(
        front_bounds.origin + gpui::point(px(8.), px(8.)),
        None,
        Default::default(),
    );
    draw(cx);
    assert!(
        owner.read_with(cx, |v, _| v.focus.borrow().allows(n(2))),
        "hover expands the production stack"
    );
    cx.simulate_mouse_move(gpui::point(px(1.), px(1.)), None, Default::default());
    draw(cx);
    assert!(
        !owner.read_with(cx, |v, _| v.focus.borrow().allows(n(2))),
        "leaving collapses without deleting the editor"
    );
    cx.update(|window, cx| window.focus(&scope, cx));
    draw(cx);
    owner.read_with(cx, |v, _| {
        assert!(v.focus.borrow().allows(n(2)));
        assert_eq!(
            v.toasts[&n(5)].status(),
            (false, false),
            "focus pauses every active timer"
        );
    });
    cx.update(|window, cx| window.focus(&old, cx));
    apply(
        &owner,
        cx,
        vec![Op::SetToastLayering(
            n(0),
            Some(Layering {
                peek: 30.,
                ..layering()
            }),
        )],
    );
    assert!(cx.update(|window, _| old.is_focused(window)));
    apply(&owner, cx, vec![Op::SetToastLayering(n(0), None)]);
    assert!(cx.update(|window, _| old.is_focused(window)));
    owner.read_with(cx, |v, cx| {
        assert_eq!(old, v.editors[&n(2)].focus_handle(cx));
        assert_eq!(front, v.editors[&n(6)].focus_handle(cx));
    });
    apply(
        &owner,
        cx,
        vec![Op::SetToastLayering(n(0), Some(layering()))],
    );
    cx.update(|window, cx| window.blur(cx));
    draw(cx);
    assert!(!owner.read_with(cx, |v, _| v.focus.borrow().allows(n(2))));
    // The named Group is an accessibility entry point before hidden back cards.
    let nodes = cx.a11y_tree().unwrap().nodes;
    assert!(
        nodes
            .iter()
            .any(|(_, node)| node.label() == Some("Activity notifications")
                && node.supports_action(gpui::accesskit::Action::Focus))
    );
    apply(
        &owner,
        cx,
        vec![Op::Splice(n(0), 0, 3, vec![n(5), n(3), n(1)])],
    );
    assert!(owner.read_with(cx, |v, _| v.focus.borrow().allows(n(2))));
    assert!(!owner.read_with(cx, |v, _| v.focus.borrow().allows(n(6))));
    let weak = owner.read_with(cx, |v, _| Rc::downgrade(&v.toast_stacks[&n(0)].layered));
    let mut remove = vec![Op::SetRoot(None), Op::Splice(n(0), 0, 3, vec![])];
    for slot in [1, 3, 5] {
        remove.extend([
            Op::Splice(n(slot), 0, 1, vec![]),
            Op::Remove(n(slot + 1)),
            Op::Remove(n(slot)),
        ]);
    }
    remove.push(Op::Remove(n(0)));
    apply(&owner, cx, remove);
    assert!(weak.upgrade().is_none());
    assert!(owner.read_with(cx, |v, _| v.toasts.is_empty()
        && v.toast_stacks.is_empty()
        && v.editors.is_empty()));
}

#[test]
fn layered_group_keyboard_scroll_zero_area_and_empty_stack_retire_focus_and_expiry() {
    let mut app = TestAppContext::single();
    app.update(gpui_base::init);
    let (_reader, writer) = UnixStream::pair().unwrap();
    let transport = Arc::new(Transport::new(writer.as_raw_fd()).unwrap());
    let session = Rc::new(RefCell::new(Session::default()));
    session.borrow_mut().hello(VERSION, CAPABILITIES).unwrap();
    session
        .borrow_mut()
        .open(1, w(), "Layered", 400., 220.)
        .unwrap();
    let (owner, cx) =
        app.add_window_view(|_, _| View::new(w(), session.clone(), transport.clone()));
    cx.simulate_resize(gpui::size(px(400.), px(220.)));
    cx.update(|window, _| window.activate_window());
    let mut ops = initial();
    for slot in [1, 3, 5] {
        ops.push(Op::SetStyle(
            n(slot),
            vec![Style::Fields(vec![Field::Height(Length::Px(180.))])],
        ));
    }
    apply(&owner, cx, ops);
    let scope = owner.read_with(cx, |v, _| v.focus.borrow().handle(n(0)).unwrap());
    let state = owner.read_with(cx, |v, _| v.toast_stacks[&n(0)].layered.clone());
    // An oversized collapsed footprint still anchors its newest edge exactly.
    let front = owner.read_with(cx, |v, _| v.toasts[&n(5)].bounds.get());
    assert_eq!(front.bottom(), px(204.));
    cx.update(|window, cx| window.focus(&scope, cx));
    draw(cx);
    assert!(state.borrow().max_scroll() > px(300.));
    assert_eq!(state.borrow().scroll(), state.borrow().max_scroll());
    cx.simulate_keystrokes("home");
    draw(cx);
    assert_eq!(state.borrow().scroll(), px(0.));
    assert!(owner.read_with(cx, |v, _| v.focus.borrow().allows(n(2))));
    assert!(!owner.read_with(cx, |v, _| v.focus.borrow().allows(n(6))));
    cx.simulate_keystrokes("end");
    draw(cx);
    assert_eq!(state.borrow().scroll(), state.borrow().max_scroll());
    apply(
        &owner,
        cx,
        vec![Op::SetToastPlacement(
            n(0),
            Some(gpuio_protocol::toast_placement::Placement {
                anchor: gpuio_protocol::toast_placement::Anchor::BottomRight,
                top: 16384.,
                right: 16384.,
                bottom: 16384.,
                left: 16384.,
            }),
        )],
    );
    assert!(!cx.update(|window, _| scope.is_focused(window)));
    owner.read_with(cx, |v, _| {
        for slot in [1, 3, 5] {
            assert_eq!(v.toasts[&n(slot)].status(), (false, false));
            assert!(!v.focus.borrow().allows(n(slot + 1)));
        }
    });
    apply(&owner, cx, vec![Op::SetToastPlacement(n(0), None)]);
    // Remove all children but preserve the stack: no empty Group focus stop.
    let mut ops = vec![Op::Splice(n(0), 0, 3, vec![])];
    for slot in [1, 3, 5] {
        ops.extend([
            Op::Splice(n(slot), 0, 1, vec![]),
            Op::Remove(n(slot + 1)),
            Op::Remove(n(slot)),
        ]);
    }
    apply(&owner, cx, ops);
    assert!(!cx.update(|window, cx| owner.read(cx).focus.borrow().can_focus(&scope, window)));
    assert_eq!(state.borrow().max_scroll(), px(0.));
    // GPUI/editor callbacks already queued by preceding frames may run once
    // after their owner retires. They must not renew work for the empty stack.
    cx.update(|window, cx| window.simulate_next_frame(cx));
    draw(cx);
    assert_eq!(cx.update(|window, cx| window.simulate_next_frame(cx)), 0);
}

#[test]
fn accepted_removal_releases_layered_geometry_without_waiting_for_another_draw() {
    let mut app = TestAppContext::single();
    app.update(gpui_base::init);
    let (_reader, writer) = UnixStream::pair().unwrap();
    let transport = Arc::new(Transport::new(writer.as_raw_fd()).unwrap());
    let session = Rc::new(RefCell::new(Session::default()));
    session.borrow_mut().hello(VERSION, CAPABILITIES).unwrap();
    session
        .borrow_mut()
        .open(1, w(), "Retirement", 400., 300.)
        .unwrap();
    let (owner, cx) =
        app.add_window_view(|_, _| View::new(w(), session.clone(), transport.clone()));
    apply(&owner, cx, initial());
    assert_eq!(
        owner.read_with(cx, |v, _| v.toast_stacks[&n(0)]
            .layered
            .borrow()
            .retained_items()),
        3
    );
    for (slot, remaining) in [(5, 2), (3, 1), (1, 0)] {
        // Commit the tree and synchronize native owners while no frame is drawn.
        cx.update(|window, cx| {
            owner.update(cx, |v, cx| {
                let base = v.session.borrow().tree(w()).unwrap().revision();
                let changed = v
                    .session
                    .borrow_mut()
                    .apply(&Transaction {
                        window: w(),
                        base,
                        revision: base + 1,
                        operations: vec![
                            Op::Splice(n(0), remaining, 1, vec![]),
                            Op::Splice(n(slot), 0, 1, vec![]),
                            Op::Remove(n(slot + 1)),
                            Op::Remove(n(slot)),
                        ],
                    })
                    .unwrap();
                v.update_editors(&changed.dirty, window, cx);
                assert_eq!(
                    v.toast_stacks[&n(0)].layered.borrow().retained_items(),
                    remaining as usize
                );
                assert!(!v.toasts.contains_key(&n(slot)));
                assert!(!v.editors.contains_key(&n(slot + 1)));
            })
        });
    }
}
