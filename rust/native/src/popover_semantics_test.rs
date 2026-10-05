//! Production View on TestPlatform; not physical OS or VoiceOver acceptance.
use super::*;
use crate::session::Session;
use gpui::{Entity, TestAppContext, VisualTestContext, accesskit};
use gpuio_protocol::HandlerId;
use std::{os::fd::AsRawFd, os::unix::net::UnixStream};
pub(super) fn id(n: i64) -> NodeId {
    NodeId::from_parts(n, 1).unwrap()
}
pub(super) fn handler(n: i64) -> HandlerId {
    HandlerId::from_parts(n, 1).unwrap()
}
pub(super) fn draw(cx: &mut VisualTestContext) {
    cx.run_until_parked();
    cx.update(|w, cx| {
        w.simulate_next_frame(cx);
        w.draw(cx).clear(cx);
    });
    cx.run_until_parked();
}
pub(super) fn apply(owner: &Entity<View>, cx: &mut VisualTestContext, operations: Vec<Op>) {
    cx.update(|w, cx| {
        owner.update(cx, |v, cx| {
            let base = v.session.borrow().tree(v.id).unwrap().revision();
            let applied = v
                .session
                .borrow_mut()
                .apply(&Transaction {
                    window: v.id,
                    base,
                    revision: base + 1,
                    operations,
                })
                .unwrap();
            v.update_editors(&applied.dirty, w, cx);
            cx.notify();
        })
    });
    draw(cx);
    draw(cx);
}
pub(super) fn button(n: i64, label: &str) -> Vec<Op> {
    vec![
        Op::Create(id(n), Kind::Button, label.into(), Some(handler(n))),
        Op::SetControl(id(n), Control::Button(false)),
        Op::SetStyle(
            id(n),
            vec![
                Style::Width(Length::Px(160.)),
                Style::Height(Length::Px(30.)),
            ],
        ),
    ]
}
fn panel(n: i64) -> Vec<Op> {
    vec![
        Op::Create(id(n), Kind::FocusScope, "".into(), Some(handler(n))),
        Op::SetFocusScope(
            id(n),
            FocusScopeConfig {
                trap: false,
                auto_focus: true,
                restore_focus: true,
            },
        ),
        Op::SetOverlay(
            id(n),
            Some(OverlayConfig {
                kind: OverlayKind::Popover,
                label: format!("Popup {n}"),
                width: 240.,
                dismiss_on_escape: true,
                dismiss_on_outside_pointer: true,
            }),
        ),
    ]
}
pub(super) fn ax(cx: &VisualTestContext, label: &str) -> (accesskit::NodeId, accesskit::Node) {
    cx.a11y_tree()
        .unwrap()
        .nodes
        .into_iter()
        .find(|(_, n)| n.label() == Some(label))
        .unwrap()
}
fn state(cx: &VisualTestContext, label: &str, expanded: Option<bool>) -> accesskit::NodeId {
    let (id, n) = ax(cx, label);
    assert_eq!(n.role(), accesskit::Role::Button);
    assert_eq!(n.is_expanded(), expanded);
    assert_eq!(n.has_popup(), expanded.map(|_| accesskit::HasPopup::Dialog));
    assert!(!n.supports_action(accesskit::Action::Expand));
    assert!(!n.supports_action(accesskit::Action::Collapse));
    id
}
fn action(cx: &mut VisualTestContext, target: accesskit::NodeId, action: accesskit::Action) {
    cx.simulate_a11y_action(accesskit::ActionRequest {
        action,
        target_node: target,
        target_tree: accesskit::TreeId::ROOT,
        data: None,
    });
    draw(cx);
}
pub(super) fn events(t: &Transport) -> Vec<Event> {
    t.mailbox
        .lock()
        .unwrap()
        .drain(256)
        .into_iter()
        .filter(|e| !matches!(e, Event::Rendered(..)))
        .collect()
}
pub(super) fn with_view(f: impl FnOnce(&Entity<View>, &mut VisualTestContext, &Transport)) {
    let mut app = TestAppContext::single();
    app.update(gpui_base::init);
    let (_read, write) = UnixStream::pair().unwrap();
    let transport = Arc::new(Transport::new(write.as_raw_fd()).unwrap());
    let session = Rc::new(RefCell::new(Session::default()));
    let window = WindowId::from_parts(0, 1).unwrap();
    session.borrow_mut().hello(VERSION, CAPABILITIES).unwrap();
    session
        .borrow_mut()
        .open(1, window, "Popover", 600., 500.)
        .unwrap();
    let (owner, cx) = app.add_window_view(|_, _| View::new(window, session, transport.clone()));
    cx.simulate_a11y_active(true);
    let mut ops = vec![Op::Create(id(0), Kind::Container, "".into(), None)];
    ops.extend(button(1, "Open"));
    ops.extend([
        Op::Splice(id(0), 0, 0, vec![id(1)]),
        Op::SetRoot(Some(id(0))),
    ]);
    apply(&owner, cx, ops);
    events(&transport);
    f(&owner, cx, &transport);
}
#[test]
fn popup_metadata_tracks_accepted_surface_without_owning_activation() {
    with_view(|owner, cx, transport| {
        let original = state(cx, "Open", None);
        let trigger = owner.read_with(cx, |v, _| v.buttons[&id(1)].clone());
        apply(owner, cx, vec![Op::SetPopover(id(0), true)]);
        assert_eq!(state(cx, "Open", Some(false)), original);
        events(transport);
        action(cx, original, accesskit::Action::Click);
        assert!(matches!(events(transport).as_slice(),[Event::Press(_,n,_,_)] if *n==id(1)));
        state(cx, "Open", Some(false)); // Pending application callback is not an open popup.
        cx.update(|w, cx| w.focus(&trigger.focus, cx));
        draw(cx);
        cx.update(|w, cx| {
            let keystroke = gpui::Keystroke::parse("enter").unwrap();
            w.dispatch_event(
                gpui::PlatformInput::KeyDown(gpui::KeyDownEvent {
                    keystroke: keystroke.clone(),
                    is_held: false,
                    prefer_character_input: false,
                }),
                cx,
            );
            w.dispatch_event(
                gpui::PlatformInput::KeyUp(gpui::KeyUpEvent { keystroke }),
                cx,
            );
        });
        draw(cx);
        assert!(matches!(events(transport).as_slice(),[Event::Press(_,n,_,_)] if *n==id(1)));
        let point = owner.read_with(cx, |v, _| v.probes.borrow()[&id(1)].bounds.center());
        cx.simulate_click(point, gpui::Modifiers::default());
        draw(cx);
        assert!(matches!(events(transport).as_slice(),[Event::Press(_,n,_,_)] if *n==id(1)));
        let mut ops = panel(2);
        ops.extend(button(3, "Inside"));
        ops.extend([
            Op::Splice(id(2), 0, 0, vec![id(3)]),
            Op::Splice(id(0), 1, 0, vec![id(2)]),
        ]);
        apply(owner, cx, ops);
        assert_eq!(state(cx, "Open", Some(true)), original);
        assert!(!ax(cx, "Popup 2").1.is_modal());
        cx.update(|w, cx| {
            owner.read_with(cx, |v, _| assert!(v.buttons[&id(3)].focus.is_focused(w)))
        });
        assert!(owner.read_with(cx, |v, _| Rc::ptr_eq(&trigger, &v.buttons[&id(1)])));
        events(transport);
        cx.simulate_keystrokes("escape");
        draw(cx);
        assert!(
            matches!(events(transport).as_slice(),[Event::OverlayDismissed(_,n,_,_,Dismissal::Escape)] if *n==id(2))
        );
        state(cx, "Open", Some(true)); // Dismissal waits for accepted removal.
        let retired = owner.read_with(cx, |v, _| Rc::downgrade(&v.buttons[&id(3)]));
        let old_inside = ax(cx, "Inside").0;
        apply(
            owner,
            cx,
            vec![
                Op::Splice(id(0), 1, 1, vec![]),
                Op::Remove(id(3)),
                Op::Remove(id(2)),
            ],
        );
        assert_eq!(state(cx, "Open", Some(false)), original);
        cx.update(|w, _| assert!(trigger.focus.is_focused(w)));
        assert!(retired.upgrade().is_none());
        events(transport);
        action(cx, old_inside, accesskit::Action::Click);
        assert!(events(transport).is_empty());
        apply(owner, cx, vec![Op::SetPopover(id(0), false)]);
        assert_eq!(state(cx, "Open", None), original);
    });
}
#[test]
fn popup_visibility_and_anchor_disabled_state_are_independent() {
    with_view(|owner, cx, transport| {
        let mut ops = vec![Op::SetPopover(id(0), true)];
        ops.extend(panel(2));
        ops.extend(button(3, "Inside"));
        ops.extend([
            Op::Splice(id(2), 0, 0, vec![id(3)]),
            Op::Splice(id(0), 1, 0, vec![id(2)]),
        ]);
        apply(owner, cx, ops);
        for field in [Field::Display(3), Field::Disabled(true), Field::Inert(true)] {
            apply(
                owner,
                cx,
                vec![Op::SetStyle(id(2), vec![Style::Fields(vec![field])])],
            );
            state(cx, "Open", Some(false));
            apply(owner, cx, vec![Op::SetStyle(id(2), vec![])]);
            state(cx, "Open", Some(true));
        }
        apply(
            owner,
            cx,
            vec![Op::SetControl(id(1), Control::Button(true))],
        );
        let target = state(cx, "Open", Some(true));
        assert!(ax(cx, "Open").1.is_disabled());
        events(transport);
        action(cx, target, accesskit::Action::Click);
        let ignored = events(transport);
        assert!(ignored.is_empty(), "{ignored:?}");
        apply(
            owner,
            cx,
            vec![
                Op::SetControl(id(1), Control::Button(false)),
                Op::SetStyle(
                    id(1),
                    vec![Style::Fields(vec![Field::PointerEvents(false)])],
                ),
            ],
        );
        events(transport);
        action(cx, target, accesskit::Action::Click);
        assert!(matches!(events(transport).as_slice(), [Event::Press(..)]));
        apply(
            owner,
            cx,
            vec![Op::SetStyle(
                id(0),
                vec![Style::Fields(vec![Field::Disabled(true)])],
            )],
        );
        state(cx, "Open", Some(false));
        events(transport);
        action(cx, target, accesskit::Action::Click);
        assert!(events(transport).is_empty());
    });
}
#[test]
fn custom_anchor_does_not_guess_descendants_and_nested_popovers_keep_separate_state() {
    with_view(|owner, cx, _| {
        apply(
            owner,
            cx,
            vec![
                Op::Create(id(2), Kind::Container, "".into(), None),
                Op::Splice(id(0), 0, 1, vec![]),
                Op::Splice(id(2), 0, 0, vec![id(1)]),
                Op::Splice(id(0), 0, 0, vec![id(2)]),
                Op::SetPopover(id(0), true),
            ],
        );
        state(cx, "Open", None);
    });
    with_view(|owner, cx, _| {
        apply(owner, cx, vec![Op::SetPopover(id(0), true)]);
        let mut ops = panel(2);
        ops.push(Op::Create(id(3), Kind::Container, "".into(), None));
        ops.extend(button(4, "Inner"));
        ops.extend([
            Op::SetPopover(id(3), true),
            Op::Splice(id(3), 0, 0, vec![id(4)]),
            Op::Splice(id(2), 0, 0, vec![id(3)]),
            Op::Splice(id(0), 1, 0, vec![id(2)]),
        ]);
        apply(owner, cx, ops);
        state(cx, "Open", Some(true));
        state(cx, "Inner", Some(false));
        let mut ops = panel(5);
        ops.extend(button(6, "Deep"));
        ops.extend([
            Op::Splice(id(5), 0, 0, vec![id(6)]),
            Op::Splice(id(3), 1, 0, vec![id(5)]),
        ]);
        apply(owner, cx, ops);
        state(cx, "Open", Some(true));
        state(cx, "Inner", Some(true));
        let weak = owner.read_with(cx, |v, _| Rc::downgrade(&v.buttons[&id(4)]));
        apply(
            owner,
            cx,
            vec![
                Op::Splice(id(0), 1, 1, vec![]),
                Op::Remove(id(6)),
                Op::Remove(id(5)),
                Op::Remove(id(4)),
                Op::Remove(id(3)),
                Op::Remove(id(2)),
            ],
        );
        state(cx, "Open", Some(false));
        assert!(weak.upgrade().is_none());
        assert_eq!(owner.read_with(cx, |v, _| v.buttons.len()), 1);
    });
}

#[test]
fn command_anchor_retains_registry_routing_and_disabled_gate() {
    with_view(|owner, cx, transport| {
        let command = |enabled| CommandConfig {
            id: "toggle".into(),
            generation: 1,
            label: "Command popup".into(),
            enabled,
            checked: None,
            shortcuts: vec![],
            target: CommandTarget::Callback,
        };
        apply(
            owner,
            cx,
            vec![
                Op::Create(id(2), Kind::CommandScope, "".into(), Some(handler(2))),
                Op::SetCommands(id(2), vec![command(true)]),
                Op::Create(id(3), Kind::CommandButton, "".into(), None),
                Op::SetCommandRef(id(3), "toggle".into()),
                Op::Splice(id(0), 0, 1, vec![id(3)]),
                Op::Remove(id(1)),
                Op::SetPopover(id(0), true),
                Op::Splice(id(2), 0, 0, vec![id(0)]),
                Op::SetRoot(Some(id(2))),
            ],
        );
        let original = state(cx, "Command popup", Some(false));
        events(transport);
        action(cx, original, accesskit::Action::Click);
        assert!(
            matches!(events(transport).as_slice(),[Event::CommandInvoked(_,_,_,_,name,_,_)] if name=="toggle")
        );
        let mut ops = panel(4);
        ops.extend(button(5, "Inside"));
        ops.extend([
            Op::Splice(id(4), 0, 0, vec![id(5)]),
            Op::Splice(id(0), 1, 0, vec![id(4)]),
        ]);
        apply(owner, cx, ops);
        apply(
            owner,
            cx,
            vec![Op::SetCommands(id(2), vec![command(false)])],
        );
        assert_eq!(state(cx, "Command popup", Some(true)), original);
        assert!(ax(cx, "Command popup").1.is_disabled());
        events(transport);
        action(cx, original, accesskit::Action::Click);
        assert!(events(transport).is_empty());
    });
}

#[test]
fn temporarily_unavailable_popup_controls_preserve_focus_return() {
    for initially_disabled in [false, true] {
        with_view(|owner, cx, _| {
            let mut ops = vec![Op::SetPopover(id(0), true)];
            ops.extend(panel(2));
            ops.extend(button(3, "Pending preset"));
            ops.extend([
                Op::SetControl(id(3), Control::Button(initially_disabled)),
                Op::Splice(id(2), 0, 0, vec![id(3)]),
                Op::Splice(id(0), 1, 0, vec![id(2)]),
            ]);
            apply(owner, cx, ops);
            if !initially_disabled {
                apply(
                    owner,
                    cx,
                    vec![Op::SetControl(id(3), Control::Button(true))],
                );
            }
            for _ in 0..3 {
                draw(cx);
            }
            cx.update(|w, cx| {
                owner.read_with(cx, |v, cx| {
                    assert!(
                        v.focus
                            .borrow()
                            .handle(id(2))
                            .unwrap()
                            .contains_focused(w, cx),
                        "pending popup lost focus: initially_disabled={initially_disabled}"
                    );
                });
            });
            apply(
                owner,
                cx,
                vec![Op::SetControl(id(3), Control::Button(false))],
            );
            apply(
                owner,
                cx,
                vec![
                    Op::Splice(id(0), 1, 1, vec![]),
                    Op::Remove(id(3)),
                    Op::Remove(id(2)),
                ],
            );
            cx.update(|w, cx| {
                owner.read_with(cx, |v, _| assert!(v.buttons[&id(1)].focus.is_focused(w)));
            });
        });
    }
}

#[test]
fn direct_anchor_restoration_survives_semantic_open_without_stealing_external_focus() {
    for case in [
        "semantic", "disabled", "inert", "removed", "custom", "moved",
    ] {
        with_view(|owner, cx, transport| {
            let mut ops = vec![Op::Create(id(2), Kind::Container, "".into(), None)];
            ops.extend(button(3, "Elsewhere"));
            ops.extend([
                Op::Splice(id(2), 0, 0, vec![id(0), id(3)]),
                Op::SetRoot(Some(id(2))),
                Op::SetPopover(id(0), case != "custom"),
            ]);
            apply(owner, cx, ops);
            cx.update(|w, cx| {
                let focus = owner.read_with(cx, |v, _| v.buttons[&id(3)].focus.clone());
                w.focus(&focus, cx);
            });
            draw(cx);
            events(transport);
            action(cx, ax(cx, "Open").0, accesskit::Action::Click);
            assert!(
                matches!(events(transport).as_slice(), [Event::Press(_, n, _, _)] if *n == id(1))
            );
            let mut ops = panel(4);
            ops.extend(button(5, "Inside"));
            ops.extend([
                Op::Splice(id(4), 0, 0, vec![id(5)]),
                Op::Splice(id(0), 1, 0, vec![id(4)]),
            ]);
            apply(owner, cx, ops);
            cx.update(|w, cx| {
                owner.read_with(cx, |v, _| assert!(v.buttons[&id(5)].focus.is_focused(w)))
            });
            if case == "moved" {
                cx.update(|w, cx| {
                    let focus = owner.read_with(cx, |v, _| v.buttons[&id(3)].focus.clone());
                    w.focus(&focus, cx);
                });
                draw(cx);
            }
            let mut close = vec![
                Op::Splice(id(0), 1, 1, vec![]),
                Op::Remove(id(5)),
                Op::Remove(id(4)),
            ];
            match case {
                "disabled" => close.push(Op::SetControl(id(1), Control::Button(true))),
                "inert" => close.push(Op::SetStyle(
                    id(1),
                    vec![Style::Fields(vec![Field::Inert(true)])],
                )),
                "removed" => {
                    close.extend(button(6, "Replacement anchor"));
                    close.extend([Op::Splice(id(0), 0, 1, vec![id(6)]), Op::Remove(id(1))]);
                }
                _ => {}
            }
            apply(owner, cx, close);
            let expected = if case == "semantic" { id(1) } else { id(3) };
            cx.update(|w, cx| {
                owner.read_with(cx, |v, _| {
                    assert!(v.buttons[&expected].focus.is_focused(w), "{case}");
                })
            });
        });
    }
}
