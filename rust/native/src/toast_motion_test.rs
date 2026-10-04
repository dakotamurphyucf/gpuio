//! Production phase scheduling/delivery on TestPlatform; no OS window acceptance.
use super::*;
use gpui::{Entity, TestAppContext, VisualTestContext};
use gpuio_protocol::{HandlerId, toast_motion::Config as Motion};
use std::{os::fd::AsRawFd, os::unix::net::UnixStream, time::Duration};
fn n(i: i64) -> NodeId {
    NodeId::from_parts(i, 1).unwrap()
}
fn w() -> WindowId {
    WindowId::from_parts(0, 1).unwrap()
}
fn draw(cx: &mut VisualTestContext) {
    cx.run_until_parked();
    cx.update(|w, cx| {
        w.simulate_next_frame(cx);
        w.draw(cx).clear(cx);
    });
    cx.run_until_parked();
}
fn apply(owner: &Entity<View>, cx: &mut VisualTestContext, operations: Vec<Op>) {
    cx.update(|window, cx| {
        owner.update(cx, |v, cx| {
            let base = v.session.borrow().tree(w()).unwrap().revision();
            let dirty = v
                .session
                .borrow_mut()
                .apply(&Transaction {
                    window: w(),
                    base,
                    revision: base + 1,
                    operations,
                })
                .unwrap();
            v.update_editors(&dirty.dirty, window, cx);
            cx.notify();
        })
    });
    draw(cx);
}
fn tick(cx: &mut VisualTestContext, ms: u64) {
    cx.executor().advance_clock(Duration::from_millis(ms));
    draw(cx);
}
fn editor(slot: i64) -> Vec<Op> {
    vec![
        Op::Create(
            n(slot),
            Kind::Input,
            "Draft".into(),
            Some(HandlerId::from_parts(slot, 1).unwrap()),
        ),
        Op::SetEditor(
            n(slot),
            EditorConfig {
                label: format!("Editor {slot}"),
                placeholder: String::new(),
                read_only: false,
                disabled: false,
                submit_on_enter: false,
                auto_focus: false,
                min_rows: 1,
                max_rows: 1,
            },
        ),
    ]
}
fn config(timeout_ns: Option<i64>) -> ToastConfig {
    ToastConfig {
        label: "Saved".into(),
        close_label: "Dismiss".into(),
        timeout_ns,
        politeness: ToastPoliteness::Polite,
    }
}
fn mount(motion: Option<Motion>, timeout: Option<i64>) -> Vec<Op> {
    let mut ops = vec![
        Op::SetToastMotion(n(2), motion),
        Op::Create(
            n(3),
            Kind::Toast,
            String::new(),
            Some(HandlerId::from_parts(3, 1).unwrap()),
        ),
        Op::SetToast(n(3), config(timeout)),
    ];
    ops.extend(editor(4));
    ops.extend([
        Op::Splice(n(3), 0, 0, vec![n(4)]),
        Op::Splice(n(2), 0, 0, vec![n(3)]),
    ]);
    ops
}
fn dismiss(owner: &Entity<View>, cx: &mut VisualTestContext, reason: ToastDismissal) {
    cx.update(|w, cx| owner.update(cx, |v, cx| v.close_toast(n(3), reason, w, cx)));
    draw(cx);
}
fn events(transport: &Transport) -> Vec<ToastDismissal> {
    transport
        .mailbox
        .lock()
        .unwrap()
        .drain(100)
        .into_iter()
        .filter_map(|e| {
            if let Event::ToastDismissed(_, _, _, _, reason) = e {
                Some(reason)
            } else {
                None
            }
        })
        .collect()
}
fn setup(f: impl FnOnce(&Entity<View>, &mut VisualTestContext, &Transport)) {
    let mut app = TestAppContext::single();
    app.update(gpui_base::init);
    let (_reader, writer) = UnixStream::pair().unwrap();
    let transport = Arc::new(Transport::new(writer.as_raw_fd()).unwrap());
    let session = Rc::new(RefCell::new(Session::default()));
    session.borrow_mut().hello(VERSION, CAPABILITIES).unwrap();
    session
        .borrow_mut()
        .open(1, w(), "Motion", 500., 400.)
        .unwrap();
    let (owner, cx) = app.add_window_view(|_, _| View::new(w(), session, transport.clone()));
    cx.update(|w, cx| {
        w.activate_window();
        cx.set_reduce_motion(false);
    });
    cx.run_until_parked();
    let mut initial = vec![Op::Create(n(0), Kind::Container, String::new(), None)];
    initial.extend(editor(1));
    initial.extend([
        Op::Create(n(2), Kind::ToastStack, String::new(), None),
        Op::SetToastStack(
            n(2),
            ToastStackConfig {
                label: "Notifications".into(),
                corner: ToastCorner::BottomRight,
                width: 240.,
                max_visible: 3,
            },
        ),
        Op::Splice(n(0), 0, 0, vec![n(1), n(2)]),
        Op::SetRoot(Some(n(0))),
    ]);
    apply(&owner, cx, initial);
    let outside = owner.read_with(cx, |v, cx| v.editors[&n(1)].focus_handle(cx));
    cx.update(|w, cx| w.focus(&outside, cx));
    draw(cx);
    f(&owner, cx, &transport);
}
#[test]
fn entry_precedes_active_expiry_and_accepted_timeout_survives_config_updates() {
    setup(|owner, cx, transport| {
        apply(owner, cx, mount(Some(Motion::default()), Some(100_000_000)));
        assert_eq!(
            owner.read_with(cx, |v, _| v.toasts[&n(3)].status()),
            (false, false)
        );
        assert_eq!(
            owner.read_with(cx, |v, _| v.toasts[&n(3)].phase_status()),
            (false, true, false)
        );
        tick(cx, 399);
        assert!(events(transport).is_empty());
        assert_eq!(
            owner.read_with(cx, |v, _| v.toasts[&n(3)].status()),
            (false, false)
        );
        tick(cx, 1);
        assert_eq!(
            owner.read_with(cx, |v, _| v.toasts[&n(3)].status()),
            (false, true)
        );
        tick(cx, 100);
        assert_eq!(
            owner.read_with(cx, |v, _| v.toasts[&n(3)].status()),
            (true, false)
        );
        assert_eq!(
            owner.read_with(cx, |v, _| v.toasts[&n(3)].phase_status()),
            (false, true, true)
        );
        assert!(events(transport).is_empty());
        assert!(!owner.read_with(cx, |v, _| v.focus.borrow().allows(n(4))));
        apply(owner, cx, vec![Op::SetToast(n(3), config(None))]);
        tick(cx, 199);
        assert!(events(transport).is_empty());
        tick(cx, 1);
        assert_eq!(events(transport), vec![ToastDismissal::Timeout]);
        assert_eq!(
            owner.read_with(cx, |v, _| v.toasts[&n(3)].phase_status()),
            (true, false, false)
        );
        assert!(
            owner.read_with(cx, |v, _| v.editors.contains_key(&n(4))),
            "application still owns mounted child"
        );
        tick(cx, 1000);
        assert!(events(transport).is_empty());
    });
}
#[test]
fn interrupted_exit_restores_focus_immediately_and_is_inert_until_one_terminal_event() {
    setup(|owner, cx, transport| {
        apply(owner, cx, mount(Some(Motion::default()), None));
        tick(cx, 150);
        let outside = owner.read_with(cx, |v, cx| v.editors[&n(1)].focus_handle(cx));
        let inside = owner.read_with(cx, |v, cx| v.editors[&n(4)].focus_handle(cx));
        cx.update(|w, cx| w.focus(&inside, cx));
        draw(cx);
        dismiss(owner, cx, ToastDismissal::Escape);
        assert!(cx.update(|w, _| outside.is_focused(w)));
        assert_eq!(
            owner.read_with(cx, |v, _| v.toasts[&n(3)].phase_status()),
            (false, true, true)
        );
        assert!(!owner.read_with(cx, |v, _| v.focus.borrow().allows(n(4))));
        assert!(events(transport).is_empty());
        dismiss(owner, cx, ToastDismissal::CloseButton);
        tick(cx, 200);
        assert_eq!(events(transport), vec![ToastDismissal::Escape]);
        tick(cx, 300);
        assert!(events(transport).is_empty());
    });
}
#[test]
fn enabling_motion_after_legacy_paint_does_not_replay_and_reset_completes_exit() {
    setup(|owner, cx, transport| {
        apply(owner, cx, mount(None, None));
        apply(
            owner,
            cx,
            vec![Op::SetToastMotion(n(2), Some(Motion::default()))],
        );
        assert_eq!(
            owner.read_with(cx, |v, _| v.toasts[&n(3)].phase_status()),
            (false, false, false)
        );
        dismiss(owner, cx, ToastDismissal::CloseButton);
        assert!(events(transport).is_empty());
        apply(owner, cx, vec![Op::SetToastMotion(n(2), None)]);
        assert_eq!(events(transport), vec![ToastDismissal::CloseButton]);
        assert_eq!(
            owner.read_with(cx, |v, _| v.toasts[&n(3)].phase_status()),
            (true, false, false)
        );
        apply(
            owner,
            cx,
            vec![Op::SetToastMotion(n(2), Some(Motion::default()))],
        );
        tick(cx, 500);
        assert!(events(transport).is_empty());
        assert!(owner.read_with(cx, |v, _| v.closed_toasts().contains(&n(3))));
    });
}

#[test]
fn reduced_inactive_hidden_disabled_and_zero_area_settle_exit_without_replay() {
    for policy in 0..5 {
        setup(|owner, cx, transport| {
            apply(owner, cx, mount(Some(Motion::default()), None));
            tick(cx, 150);
            dismiss(owner, cx, ToastDismissal::CloseButton);
            assert!(events(transport).is_empty());
            match policy {
                0 => {
                    cx.update(|_, cx| cx.set_reduce_motion(true));
                    draw(cx);
                }
                1 => {
                    cx.deactivate_window();
                    draw(cx);
                }
                2 => apply(
                    owner,
                    cx,
                    vec![Op::SetStyle(
                        n(2),
                        vec![Style::Fields(vec![Field::Display(3)])],
                    )],
                ),
                3 => apply(
                    owner,
                    cx,
                    vec![Op::SetStyle(
                        n(2),
                        vec![Style::Fields(vec![Field::Disabled(true)])],
                    )],
                ),
                _ => apply(
                    owner,
                    cx,
                    vec![Op::SetToastPlacement(
                        n(2),
                        Some(gpuio_protocol::toast_placement::Placement {
                            anchor: gpuio_protocol::toast_placement::Anchor::BottomRight,
                            top: 16384.,
                            bottom: 16384.,
                            left: 16384.,
                            right: 16384.,
                        }),
                    )],
                ),
            }
            assert_eq!(
                events(transport),
                vec![ToastDismissal::CloseButton],
                "policy {policy}"
            );
            assert_eq!(
                owner.read_with(cx, |v, _| v.toasts[&n(3)].phase_status()),
                (true, false, false)
            );
            tick(cx, 1000);
            assert!(events(transport).is_empty());
        });
    }
}

#[test]
fn removal_overload_and_window_close_discard_accepted_exit_without_publication() {
    for policy in 0..3 {
        setup(|owner, cx, transport| {
            apply(owner, cx, mount(Some(Motion::default()), None));
            tick(cx, 150);
            dismiss(owner, cx, ToastDismissal::CloseButton);
            assert!(events(transport).is_empty());
            match policy {
                0 => apply(
                    owner,
                    cx,
                    vec![
                        Op::Splice(n(2), 0, 1, vec![]),
                        Op::Splice(n(3), 0, 1, vec![]),
                        Op::Remove(n(4)),
                        Op::Remove(n(3)),
                    ],
                ),
                1 => {
                    cx.update(|w, cx| {
                        owner.update(cx, |v, cx| {
                            v.session.borrow_mut().overload(v.id);
                            v.schedule_toasts(w, cx);
                        })
                    });
                }
                _ => {
                    cx.update(|w, cx| {
                        owner.update(cx, |v, cx| {
                            v.session.borrow_mut().close(v.id).unwrap();
                            v.sync_toasts(w, cx);
                        })
                    });
                }
            }
            tick(cx, 1000);
            assert!(events(transport).is_empty(), "retirement policy {policy}");
            owner.read_with(cx, |v, _| {
                if policy == 1 {
                    assert_eq!(v.toasts[&n(3)].phase_status(), (true, false, false));
                } else {
                    assert!(v.toasts.is_empty());
                    assert!(!v.closed_toasts().contains(&n(3)));
                }
            });
        });
    }
}

#[test]
fn optional_motion_preserves_expanded_order_and_zero_durations_remain_immediate() {
    setup(|owner, cx, transport| {
        apply(owner, cx, mount(None, None));
        let mut second = vec![
            Op::Create(
                n(5),
                Kind::Toast,
                String::new(),
                Some(HandlerId::from_parts(5, 1).unwrap()),
            ),
            Op::SetToast(n(5), config(None)),
        ];
        second.extend(editor(6));
        second.extend([
            Op::Splice(n(5), 0, 0, vec![n(6)]),
            Op::Splice(n(2), 1, 0, vec![n(5)]),
        ]);
        apply(owner, cx, second);
        for corner in [
            ToastCorner::TopLeft,
            ToastCorner::TopRight,
            ToastCorner::BottomLeft,
            ToastCorner::BottomRight,
        ] {
            apply(
                owner,
                cx,
                vec![
                    Op::SetToastMotion(n(2), None),
                    Op::SetToastStack(
                        n(2),
                        ToastStackConfig {
                            label: "Notifications".into(),
                            corner,
                            width: 240.,
                            max_visible: 3,
                        },
                    ),
                ],
            );
            let old = owner.read_with(cx, |v, _| {
                (v.toasts[&n(3)].bounds.get(), v.toasts[&n(5)].bounds.get())
            });
            assert!(old.0.top() < old.1.top());
            apply(
                owner,
                cx,
                vec![Op::SetToastMotion(
                    n(2),
                    Some(Motion {
                        enter_ms: 0,
                        exit_ms: 0,
                        offset: 0.,
                        ..Default::default()
                    }),
                )],
            );
            let new = owner.read_with(cx, |v, _| {
                (v.toasts[&n(3)].bounds.get(), v.toasts[&n(5)].bounds.get())
            });
            assert_eq!(
                old, new,
                "enabling motion alone cannot change column order or positions"
            );
        }
        dismiss(owner, cx, ToastDismissal::CloseButton);
        assert_eq!(events(transport), vec![ToastDismissal::CloseButton]);
        assert_eq!(
            owner.read_with(cx, |v, _| v.toasts[&n(3)].phase_status()),
            (true, false, false)
        );
    });
}

#[test]
fn overflow_retires_nested_modal_input_and_semantics_without_waiting_for_exit() {
    setup(|owner, cx, transport| {
        cx.simulate_a11y_active(true);
        apply(owner, cx, mount(Some(Motion::default()), None));
        tick(cx, 400);
        apply(
            owner,
            cx,
            vec![
                Op::Create(
                    n(5),
                    Kind::FocusScope,
                    String::new(),
                    Some(HandlerId::from_parts(5, 1).unwrap()),
                ),
                Op::SetFocusScope(
                    n(5),
                    FocusScopeConfig {
                        trap: true,
                        auto_focus: true,
                        restore_focus: true,
                    },
                ),
                Op::SetOverlay(
                    n(5),
                    Some(OverlayConfig {
                        kind: OverlayKind::Dialog,
                        label: "Toast dialog".into(),
                        width: 200.,
                        dismiss_on_escape: true,
                        dismiss_on_outside_pointer: false,
                    }),
                ),
                Op::Create(
                    n(6),
                    Kind::Button,
                    "Dialog action".into(),
                    Some(HandlerId::from_parts(6, 1).unwrap()),
                ),
                Op::SetControl(n(6), Control::Button(false)),
                Op::Splice(n(5), 0, 0, vec![n(6)]),
                Op::Splice(n(3), 1, 0, vec![n(5)]),
            ],
        );
        draw(cx);
        let inside = owner.read_with(cx, |v, _| v.buttons[&n(6)].focus.clone());
        let outside = owner.read_with(cx, |v, cx| v.editors[&n(1)].focus_handle(cx));
        assert!(cx.update(|w, _| inside.is_focused(w)));
        assert!(!owner.read_with(cx, |v, _| v.focus.borrow().allows(n(1))));
        apply(
            owner,
            cx,
            vec![
                Op::SetToastStack(
                    n(2),
                    ToastStackConfig {
                        label: "Notifications".into(),
                        corner: ToastCorner::BottomRight,
                        width: 240.,
                        max_visible: 1,
                    },
                ),
                Op::Create(
                    n(7),
                    Kind::Toast,
                    String::new(),
                    Some(HandlerId::from_parts(7, 1).unwrap()),
                ),
                Op::SetToast(n(7), config(None)),
                Op::Splice(n(2), 1, 0, vec![n(7)]),
            ],
        );
        assert!(
            cx.update(|w, _| outside.is_focused(w)),
            "retired modal cannot block saved focus restoration"
        );
        assert!(owner.read_with(cx, |v, _| v.focus.borrow().allows(n(1))));
        assert!(!owner.read_with(cx, |v, _| v.focus.borrow().allows(n(6))));
        assert!(
            !cx.a11y_tree()
                .unwrap()
                .nodes
                .iter()
                .any(|(_, n)| n.label() == Some("Dialog action"))
        );
        assert!(events(transport).is_empty());
        assert_eq!(
            owner.read_with(cx, |v, _| v.toasts[&n(3)].phase_status()),
            (false, true, true)
        );
        tick(cx, 200);
        assert_eq!(events(transport), vec![ToastDismissal::Overflow]);
    });
}

#[test]
fn composition_escape_precedes_animated_toast_dismissal() {
    setup(|owner, cx, transport| {
        apply(owner, cx, mount(Some(Motion::default()), None));
        tick(cx, 400);
        let focus = owner.read_with(cx, |v, cx| v.editors[&n(4)].focus_handle(cx));
        cx.update(|w, cx| w.focus(&focus, cx));
        draw(cx);
        cx.update(|w, cx| owner.update(cx, |v, cx| v.editors[&n(4)].mark_test_text("に", w, cx)));
        draw(cx);
        assert!(owner.read_with(cx, |v, cx| v.editors[&n(4)].is_composing(cx)));
        cx.simulate_keystrokes("escape");
        draw(cx);
        assert!(!owner.read_with(cx, |v, cx| v.editors[&n(4)].is_composing(cx)));
        assert!(!owner.read_with(cx, |v, _| v.toasts[&n(3)].status().0));
        assert!(events(transport).is_empty());
        cx.simulate_keystrokes("escape");
        draw(cx);
        assert_eq!(
            owner.read_with(cx, |v, _| v.toasts[&n(3)].phase_status()),
            (false, true, true)
        );
        assert!(events(transport).is_empty());
        tick(cx, 200);
        assert_eq!(events(transport), vec![ToastDismissal::Escape]);
    });
}

#[test]
fn production_reorder_animates_current_bounds_without_remounting_editor_owners() {
    setup(|owner, cx, _| {
        apply(owner, cx, mount(Some(Motion::default()), None));
        let mut second = vec![
            Op::Create(
                n(5),
                Kind::Toast,
                String::new(),
                Some(HandlerId::from_parts(5, 1).unwrap()),
            ),
            Op::SetToast(n(5), config(None)),
        ];
        second.extend(editor(6));
        second.extend([
            Op::Splice(n(5), 0, 0, vec![n(6)]),
            Op::Splice(n(2), 1, 0, vec![n(5)]),
        ]);
        apply(owner, cx, second);
        tick(cx, 400);
        let before = owner.read_with(cx, |v, _| {
            (v.toasts[&n(3)].bounds.get(), v.toasts[&n(5)].bounds.get())
        });
        let editor = owner.read_with(cx, |v, cx| v.editors[&n(4)].focus_handle(cx));
        apply(owner, cx, vec![Op::Splice(n(2), 0, 2, vec![n(5), n(3)])]);
        tick(cx, 16);
        let animated = owner.read_with(cx, |v, _| v.toasts[&n(3)].bounds.get());
        assert!(
            animated.top() > before.0.top() && animated.top() < before.1.top(),
            "native Host must supply the spring clock"
        );
        tick(cx, 2000);
        assert_eq!(
            owner.read_with(cx, |v, _| v.toasts[&n(3)].bounds.get()),
            before.1
        );
        assert_eq!(
            owner.read_with(cx, |v, cx| v.editors[&n(4)].focus_handle(cx)),
            editor
        );
        apply(owner, cx, vec![Op::SetToastMotion(n(2), None)]);
        assert_eq!(
            owner.read_with(cx, |v, _| v.toast_stacks[&n(2)]
                .layered
                .borrow()
                .retained_items()),
            0,
            "metadata reset immediately releases measured descriptors"
        );
    });
}
