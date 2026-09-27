//! Managed-row retention, ancestor hiding, modal blocking and captured selection.
use super::super::super::native_test::{mouse, move_mouse};
use super::*;
use gpuio_protocol::list::{Config, IdRun, Order, Retained, Row, ScrollPolicy};
use gpuio_protocol::v1::{Field, Length, Style};

fn id(slot: i64) -> NodeId {
    NodeId::from_parts(slot, 1).unwrap()
}
fn size_style() -> Vec<Style> {
    vec![
        Style::Width(Length::Px(320.)),
        Style::Height(Length::Px(120.)),
    ]
}
fn press(cx: &mut AsyncApp, handle: WindowHandle<View>) -> Point<Pixels> {
    let point = handle
        .update(cx, |v, _, cx| {
            let bounds = v.otps[&node()]
                .state
                .read(cx)
                .layout
                .as_ref()
                .unwrap()
                .bounds;
            point(bounds.left() + px(5.), bounds.center().y)
        })
        .unwrap();
    move_mouse(cx, handle, point, false);
    mouse(cx, handle, point, true);
    handle
        .update(cx, |v, w, cx| {
            assert!(v.otps[&node()].state.read(cx).dragging);
            assert!(w.captured_hitbox().is_some());
        })
        .unwrap();
    point
}
fn stopped(cx: &mut AsyncApp, handle: WindowHandle<View>) {
    handle
        .update(cx, |v, w, cx| {
            let state = v.otps[&node()].state.read(cx);
            assert!(!state.dragging && state.capture.is_none());
            assert!(w.captured_hitbox().is_none());
            assert!(!state.model.editor().is_composing());
        })
        .unwrap();
}
fn pins(cx: &mut AsyncApp, handle: WindowHandle<View>) {
    handle
        .update(cx, |v, w, cx| {
            let pins = v.list_pins(w, cx);
            assert_eq!(
                pins,
                vec![Retained {
                    node: id(1),
                    rows: vec![7]
                }]
            );
            let base = v.session.borrow().tree(v.id).unwrap().revision();
            let result = v.session.borrow_mut().apply_guarded(
                &Transaction {
                    window: v.id,
                    base,
                    revision: base + 1,
                    operations: vec![
                        Op::Remove(node()),
                        Op::Splice(id(1), 0, 1, vec![]),
                        Op::SetListRows(id(1), vec![]),
                    ],
                },
                &pins,
            );
            assert!(matches!(
                result,
                Err(crate::tree::ApplyFailure::Retained(_))
            ));
            assert_eq!(v.session.borrow().tree(v.id).unwrap().revision(), base);
        })
        .unwrap();
}

pub(super) async fn exercise(cx: &mut AsyncApp, handle: WindowHandle<View>, transport: &Transport) {
    let original = snapshot(cx, handle);
    let entity = handle
        .update(cx, |v, _, _| v.otps[&node()].state.entity_id())
        .unwrap();
    apply(
        cx,
        handle,
        vec![
            Op::Create(
                id(1),
                Kind::VirtualList,
                "".into(),
                Some(HandlerId::from_parts(1, 1).unwrap()),
            ),
            Op::SetListConfig(
                id(1),
                Config {
                    estimated_height: 44.,
                    overscan: 0.,
                    max_active: 4,
                    scroll_policy: ScrollPolicy::KeepPosition,
                    scrollbar: true,
                    managed: true,
                },
            ),
            Op::SetListOrder(
                id(1),
                Order {
                    revision: 1,
                    runs: vec![IdRun { first: 7, count: 1 }],
                },
            ),
            Op::SetStyle(id(1), size_style()),
            Op::SetStyle(
                node(),
                vec![
                    Style::Width(Length::Px(300.)),
                    Style::Height(Length::Px(44.)),
                ],
            ),
            Op::SetListRows(
                id(1),
                vec![Row {
                    id: 7,
                    node: node(),
                }],
            ),
            Op::Splice(id(1), 0, 0, vec![node()]),
            Op::SetRoot(Some(id(1))),
        ],
    );
    frame(cx, handle).await;
    assert!(matches!(
        command(cx, handle, o::Command::Focus),
        o::Response::Applied(_)
    ));
    frame(cx, handle).await;
    pins(cx, handle);
    events(transport);
    let pressed = press(cx, handle);
    let before = snapshot(cx, handle);
    #[cfg(target_os = "macos")]
    {
        native_text(cx, handle, "９", true);
        assert!(snapshot(cx, handle).composition.is_some());
        pins(cx, handle);
    }
    apply(
        cx,
        handle,
        vec![Op::SetStyle(
            id(1),
            vec![Style::Fields(vec![Field::Display(3)])],
        )],
    );
    frame(cx, handle).await;
    stopped(cx, handle);
    let hidden = snapshot(cx, handle);
    assert_eq!(hidden.value, before.value);
    assert_eq!(hidden.selection, before.selection);
    assert!(!hidden.focused);
    assert_eq!(
        command(cx, handle, o::Command::Focus),
        o::Response::Failed(o::Error::FocusBlocked)
    );
    assert_eq!(
        command(cx, handle, o::Command::ReadSnapshot),
        o::Response::Applied(hidden)
    );
    assert!(matches!(
        command(
            cx,
            handle,
            o::Command::Replace {
                value: "34".into(),
                selection: o::SelectionPolicy::End,
                undo: o::UndoPolicy::Record,
                if_revision: None
            }
        ),
        o::Response::Applied(_)
    ));
    mouse(cx, handle, pressed, false);
    assert_eq!(snapshot(cx, handle).value, "34");
    apply(cx, handle, vec![Op::SetStyle(id(1), size_style())]);
    frame(cx, handle).await;
    assert_eq!(snapshot(cx, handle).value, "34");
    assert!(matches!(
        command(cx, handle, o::Command::Focus),
        o::Response::Applied(_)
    ));
    frame(cx, handle).await;
    let pressed = press(cx, handle);
    #[cfg(target_os = "macos")]
    native_text(cx, handle, "５", true);
    events(transport);
    apply(
        cx,
        handle,
        vec![
            Op::Create(id(2), Kind::Container, "".into(), None),
            Op::Create(id(3), Kind::FocusScope, "".into(), None),
            Op::SetFocusScope(
                id(3),
                FocusScopeConfig {
                    trap: true,
                    auto_focus: true,
                    restore_focus: true,
                },
            ),
            Op::Create(
                id(4),
                Kind::Button,
                "Modal action".into(),
                Some(HandlerId::from_parts(4, 1).unwrap()),
            ),
            Op::Splice(id(3), 0, 0, vec![id(4)]),
            Op::Splice(id(2), 0, 0, vec![id(1), id(3)]),
            Op::SetRoot(Some(id(2))),
        ],
    );
    frame(cx, handle).await;
    stopped(cx, handle);
    assert_eq!(
        command(cx, handle, o::Command::Focus),
        o::Response::Failed(o::Error::FocusBlocked)
    );
    mouse(cx, handle, pressed, false);
    // Call the actual native delegate after access was lost, not only the focused modal.
    handle
        .update(cx, |v, w, cx| {
            v.otps[&node()]
                .state
                .update(cx, |s, cx| s.replace_text_in_range(None, "9", w, cx));
        })
        .unwrap();
    assert_eq!(snapshot(cx, handle).value, "34");
    apply(
        cx,
        handle,
        vec![
            Op::Splice(id(2), 0, 2, vec![]),
            Op::SetRoot(Some(id(1))),
            Op::Remove(id(4)),
            Op::Remove(id(3)),
            Op::Remove(id(2)),
        ],
    );
    frame(cx, handle).await;
    assert!(matches!(
        command(cx, handle, o::Command::Focus),
        o::Response::Applied(_)
    ));
    assert_eq!(
        handle
            .update(cx, |v, _, _| v.otps[&node()].state.entity_id())
            .unwrap(),
        entity
    );
    apply(
        cx,
        handle,
        vec![
            Op::SetListRows(id(1), vec![]),
            Op::Splice(id(1), 0, 1, vec![]),
            Op::SetRoot(Some(node())),
            Op::Remove(id(1)),
            Op::SetStyle(node(), vec![]),
        ],
    );
    assert!(matches!(
        command(
            cx,
            handle,
            o::Command::Replace {
                value: original.value,
                selection: o::SelectionPolicy::Select(original.selection),
                undo: o::UndoPolicy::Reset,
                if_revision: None
            }
        ),
        o::Response::Applied(_)
    ));
    frame(cx, handle).await;
    events(transport);
    eprintln!(
        "GPUIO_OTP_POLICY_OK: managed row pins, atomic retention rejection, hidden ancestor/modal composition and capture cleanup, explicit hidden commands, stale native delegate and identity retention"
    );
}
