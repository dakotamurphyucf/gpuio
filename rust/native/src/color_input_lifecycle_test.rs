//! Native ownership through managed rows, modal/visibility gates and window close.
use super::*;
use gpuio_protocol::{
    list::{Config, IdRun, Order, Retained, Row, ScrollPolicy},
    v1::{FocusScopeConfig, Length},
};
fn id(slot: i64) -> NodeId {
    NodeId::from_parts(slot, 1).unwrap()
}
fn command(cx: &mut AsyncApp, handle: WindowHandle<View>, command: c::Command) -> c::Snapshot {
    match handle
        .update(cx, |v, w, cx| {
            v.color_inputs[&node()].command(&command, w, cx)
        })
        .unwrap()
    {
        c::Response::Applied(s) => s,
        other => panic!("{other:?}"),
    }
}
fn palette(pointer: bool) -> Vec<Style> {
    vec![Style::Fields(vec![
        Field::Background(Fill::Solid(Color::Rgba(0x182332ff))),
        Field::Foreground(Color::Rgba(0xe3e9f3ff)),
        Field::PointerEvents(pointer),
    ])]
}
fn list_style() -> Vec<Style> {
    vec![
        Style::Width(Length::Px(340.)),
        Style::Height(Length::Px(540.)),
    ]
}
async fn drag(cx: &mut AsyncApp, handle: WindowHandle<View>) -> Point<Pixels> {
    let p = position(cx, handle, 0, 0.2);
    move_mouse(cx, handle, p, false);
    frame(cx, handle).await;
    mouse(cx, handle, p, true);
    frame(cx, handle).await;
    assert!(snapshot(cx, handle).interaction.is_some());
    p
}
async fn activation(cx: &mut AsyncApp, handle: WindowHandle<View>, active: bool) {
    for _ in 0..100 {
        if handle.update(cx, |_, w, _| w.is_window_active()).unwrap() == active {
            return;
        }
        cx.background_executor()
            .timer(std::time::Duration::from_millis(10))
            .await;
    }
    panic!("window activation did not become {active}");
}
pub(super) async fn exercise(cx: &mut AsyncApp, handle: WindowHandle<View>, transport: &Transport) {
    apply(
        cx,
        handle,
        vec![set(config()), Op::SetStyle(node(), palette(false))],
    );
    frame(cx, handle).await;
    command(cx, handle, c::Command::Reset { if_revision: None });
    let original = snapshot(cx, handle).value;
    let p = position(cx, handle, 0, 0.2);
    move_mouse(cx, handle, p, false);
    mouse(cx, handle, p, true);
    mouse(cx, handle, p, false);
    frame(cx, handle).await;
    assert_eq!(snapshot(cx, handle).value, original);
    assert!(snapshot(cx, handle).interaction.is_none());
    apply(cx, handle, vec![Op::SetStyle(node(), palette(true))]);
    frame(cx, handle).await;
    let owner = handle
        .update(cx, |v, _, _| v.color_inputs[&node()].state.entity_id())
        .unwrap();
    apply(
        cx,
        handle,
        vec![
            Op::Create(
                id(1),
                Kind::VirtualList,
                "".into(),
                Some(HandlerId::from_parts(10, 1).unwrap()),
            ),
            Op::SetListConfig(
                id(1),
                Config {
                    estimated_height: 500.,
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
            Op::SetStyle(id(1), list_style()),
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
    command(cx, handle, c::Command::Focus(c::Field::Hex));
    frame(cx, handle).await;
    handle
        .update(cx, |v, w, cx| {
            let input = v.color_inputs[&node()].state.read(cx).editors.fields[0]
                .state
                .clone();
            input.update(cx, |s, cx| {
                let len = s.value().encode_utf16().count();
                s.replace_and_mark_text_in_range(Some(0..len), "#abcdef", Some(0..7), w, cx);
            });
        })
        .unwrap();
    frame(cx, handle).await;
    assert!(snapshot(cx, handle).draft.unwrap().composing);
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
    apply(
        cx,
        handle,
        vec![Op::SetStyle(
            id(1),
            vec![Style::Fields(vec![Field::Display(3)])],
        )],
    );
    frame(cx, handle).await;
    frame(cx, handle).await;
    assert!(snapshot(cx, handle).interaction.is_none());
    assert_eq!(snapshot(cx, handle).value, original);
    handle
        .update(cx, |v, w, cx| {
            assert!(v.list_pins(w, cx).is_empty());
            let input = &v.color_inputs[&node()];
            assert!(!input.is_composing(cx));
            assert!(!input.focused(w, cx));
            assert_eq!(
                input.command(&c::Command::Focus(c::Field::Hex), w, cx),
                c::Response::Failed(c::Error::FocusBlocked)
            );
            input.state.update(cx, |s, cx| {
                s.choose(
                    Value::Color(Rgba::new(255, 0, 0, 255)),
                    c::Source::Palette,
                    false,
                    w,
                    cx,
                )
            });
        })
        .unwrap();
    assert_eq!(snapshot(cx, handle).value, original);
    command(
        cx,
        handle,
        c::Command::Set {
            value: Value::Color(Rgba::new(255, 0, 0, 128)),
            if_revision: None,
        },
    );
    apply(cx, handle, vec![Op::SetStyle(id(1), list_style())]);
    frame(cx, handle).await;
    let baseline = snapshot(cx, handle).committed;
    let p = drag(cx, handle).await;
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
                Some(HandlerId::from_parts(13, 1).unwrap()),
            ),
            Op::Splice(id(3), 0, 0, vec![id(4)]),
            Op::Splice(id(2), 0, 0, vec![id(1), id(3)]),
            Op::SetRoot(Some(id(2))),
        ],
    );
    frame(cx, handle).await;
    frame(cx, handle).await;
    assert!(snapshot(cx, handle).interaction.is_none());
    assert_eq!(snapshot(cx, handle).value, baseline);
    handle
        .update(cx, |v, w, cx| {
            assert!(w.captured_hitbox().is_none());
            assert_eq!(
                v.color_inputs[&node()].command(&c::Command::Focus(c::Field::Hex), w, cx),
                c::Response::Failed(c::Error::FocusBlocked)
            );
            v.color_inputs[&node()].state.update(cx, |s, cx| {
                s.choose(Value::Empty, c::Source::Clear, true, w, cx)
            });
        })
        .unwrap();
    mouse(cx, handle, p, false);
    assert_eq!(snapshot(cx, handle).value, baseline);
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
    command(cx, handle, c::Command::Focus(c::Field::Hex));
    assert_eq!(
        handle
            .update(cx, |v, _, _| v.color_inputs[&node()].state.entity_id())
            .unwrap(),
        owner
    );
    apply(
        cx,
        handle,
        vec![
            Op::SetListRows(id(1), vec![]),
            Op::Splice(id(1), 0, 1, vec![]),
            Op::SetRoot(Some(node())),
            Op::Remove(id(1)),
        ],
    );
    frame(cx, handle).await;
    events(transport);
    independent_window(cx, handle, transport).await;
    eprintln!(
        "GPUIO_COLOR_LIFECYCLE_OK: pointer policy, composition/list pins, hidden/modal cancellation, stale callbacks, capture cleanup and independent windows"
    );
}
async fn independent_window(cx: &mut AsyncApp, handle: WindowHandle<View>, transport: &Transport) {
    if !handle.update(cx, |_, w, _| w.is_window_active()).unwrap() {
        handle.update(cx, |_, w, _| w.activate_window()).unwrap();
        activation(cx, handle, true).await;
    }
    let original = snapshot(cx, handle).committed;
    let p = drag(cx, handle).await;
    let (session, bridge) = handle
        .update(cx, |v, _, _| (v.session.clone(), v.transport.clone()))
        .unwrap();
    let other_id = WindowId::from_parts(1, 1).unwrap();
    session
        .borrow_mut()
        .open(2, other_id, "Independent color", 340., 360.)
        .unwrap();
    let other = cx.update(|cx| {
        cx.open_window(
            WindowOptions {
                inactive_frame_interval: None,
                window_bounds: Some(WindowBounds::Windowed(Bounds::centered(
                    None,
                    size(px(340.), px(360.)),
                    cx,
                ))),
                ..Default::default()
            },
            |_, cx| cx.new(|_| View::new(other_id, session.clone(), bridge.clone())),
        )
        .unwrap()
    });
    activation(cx, handle, false).await;
    activation(cx, other, true).await;
    assert!(snapshot(cx, handle).interaction.is_none());
    assert_eq!(snapshot(cx, handle).value, original);
    assert!(
        events(transport)
            .iter()
            .any(|e| matches!(e, c::Event::Cancelled(c::CancelReason::WindowInactive, _)))
    );
    apply(
        cx,
        other,
        vec![
            Op::Create(
                node(),
                Kind::ColorInput,
                "".into(),
                Some(HandlerId::from_parts(0, 1).unwrap()),
            ),
            set(config()),
            Op::SetStyle(node(), palette(true)),
            Op::SetRoot(Some(node())),
        ],
    );
    frame(cx, other).await;
    let next = Value::Color(Rgba::new(17, 34, 51, 255));
    command(
        cx,
        other,
        c::Command::Set {
            value: next,
            if_revision: None,
        },
    );
    events(transport);
    drag(cx, other).await;
    let output = transport.mailbox.lock().unwrap().drain(128);
    assert!(!output.is_empty());
    assert!(
        output
            .iter()
            .all(|e| matches!(e,Event::ColorInputEvent(w,_,_,_,_) if *w==other_id))
    );
    let (weak, fields) = other
        .update(cx, |v, w, cx| {
            let input = &v.color_inputs[&node()].state;
            let weak = input.downgrade();
            let fields = input
                .read(cx)
                .editors
                .fields
                .iter()
                .map(|f| f.state.downgrade())
                .collect::<Vec<_>>();
            v.session.borrow_mut().close(v.id).unwrap();
            v.close_color_inputs(w, cx);
            assert!(w.captured_hitbox().is_none());
            w.remove_window();
            (weak, fields)
        })
        .unwrap();
    frame(cx, handle).await;
    assert!(weak.upgrade().is_none() && fields.iter().all(|f| f.upgrade().is_none()));
    assert!(session.borrow().tree(other_id).is_none());
    mouse(cx, handle, p, false);
    assert_eq!(snapshot(cx, handle).value, original);
    command(cx, handle, c::Command::Reset { if_revision: None });
    events(transport);
}
