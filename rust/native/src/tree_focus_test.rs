//! Deferred tree focus through production lists and actual GPUI paint.
use super::*;
use gpuio_protocol::{
    accessibility::{Config as Metadata, Live, Role, TreeItem},
    list::{Config, IdRun, Order, Row, ScrollPolicy, ScrollRequest, ScrollTarget},
};

fn apply(cx: &mut gpui::AsyncApp, handle: WindowHandle<View>, operations: Vec<Op>) {
    handle
        .update(cx, |view, window, cx| {
            let pins = view.list_pins(window, cx);
            let base = view.session.borrow().tree(view.id).unwrap().revision();
            let transaction = Transaction {
                window: view.id,
                base,
                revision: base + 1,
                operations,
            };
            let applied = view
                .session
                .borrow_mut()
                .apply_guarded(&transaction, &pins)
                .unwrap_or_else(|error| {
                    panic!("tree focus guarded admission: {error:?}: {transaction:?}")
                });
            view.update_editors(&applied.dirty, window, cx);
            view.list_actions(&applied.lists, window, cx);
            cx.notify();
        })
        .unwrap();
}

fn handler(slot: i64) -> gpuio_protocol::HandlerId {
    gpuio_protocol::HandlerId::from_parts(slot, 1).unwrap()
}
fn inert(value: bool) -> Op {
    Op::SetStyle(
        node(12),
        vec![Style::Fields(vec![
            Field::Width(Length::Px(350.)),
            Field::Height(Length::Px(32.)),
            Field::Inert(value),
        ])],
    )
}
fn metadata(role: Role) -> Metadata {
    Metadata {
        role: Some(role),
        label: None,
        description: None,
        live: Live::Off,
        field: None,
        current: None,
    }
}
fn order(revision: i64, ids: impl IntoIterator<Item = i64>) -> Op {
    Op::SetListOrder(
        node(12),
        Order {
            revision,
            runs: ids
                .into_iter()
                .map(|first| IdRun { first, count: 1 })
                .collect(),
        },
    )
}
fn command(serial: i64, target: ScrollTarget) -> Op {
    Op::ScrollList(node(12), ScrollRequest { serial, target })
}
fn mount(cx: &mut gpui::AsyncApp, handle: WindowHandle<View>, row: i64, disabled: bool) {
    let old = handle
        .update(cx, |view, _, _| {
            view.session
                .borrow()
                .tree(view.id)
                .unwrap()
                .get(node(12))
                .unwrap()
                .children
                .to_vec()
        })
        .unwrap();
    let target = node(old.first().map_or(13, |node| node.slot() as i64 + 1));
    let mut ops = vec![
        Op::Create(target, Kind::Container, String::new(), None),
        Op::SetStyle(
            target,
            vec![Style::Fields(vec![
                Field::Height(Length::Px(32.)),
                Field::Shrink(0.),
            ])],
        ),
        Op::SetAccessibility(
            target,
            Some(metadata(Role::TreeItem(TreeItem {
                level: 1,
                index: 0,
                count: None,
                expanded: None,
                selected: false,
                disabled,
                busy: false,
            }))),
        ),
        Op::SetListRows(
            node(12),
            vec![Row {
                id: row,
                node: target,
            }],
        ),
        Op::Splice(node(12), 0, old.len() as i64, vec![target]),
    ];
    ops.extend(old.into_iter().map(Op::Remove));
    apply(cx, handle, ops);
}
fn pending(cx: &mut gpui::AsyncApp, handle: WindowHandle<View>) -> Option<i64> {
    handle
        .update(cx, |view, _, _| {
            view.lists[&node(12)].borrow().pending_focus_row()
        })
        .unwrap()
}
fn focused(cx: &mut gpui::AsyncApp, handle: WindowHandle<View>) -> Vec<i64> {
    handle
        .update(cx, |view, window, cx| {
            view.lists[&node(12)].borrow().focused(window, cx)
        })
        .unwrap()
}
fn root_focus(cx: &mut gpui::AsyncApp, handle: WindowHandle<View>) {
    handle
        .update(cx, |view, window, cx| {
            view.lists[&node(12)]
                .borrow()
                .tree_focus
                .as_ref()
                .unwrap()
                .focus(window, cx);
        })
        .unwrap();
}
async fn activation(cx: &mut gpui::AsyncApp, window: WindowHandle<View>, active: bool) {
    for _ in 0..100 {
        if window
            .update(cx, |_, window, _| window.is_window_active())
            .unwrap()
            == active
        {
            return;
        }
        cx.background_executor()
            .timer(std::time::Duration::from_millis(10))
            .await;
    }
    panic!("tree focus test window activation did not become {active}");
}

pub(super) async fn exercise(cx: &mut gpui::AsyncApp, handle: WindowHandle<View>) {
    apply(
        cx,
        handle,
        vec![
            Op::Create(
                node(12),
                Kind::VirtualList,
                String::new(),
                Some(handler(20)),
            ),
            Op::SetListConfig(
                node(12),
                Config {
                    estimated_height: 32.,
                    overscan: 0.,
                    max_active: 1,
                    scroll_policy: ScrollPolicy::KeepPosition,
                    scrollbar: false,
                    managed: true,
                },
            ),
            order(1, 1..=100),
            Op::SetStyle(
                node(12),
                vec![Style::Fields(vec![
                    Field::Width(Length::Px(350.)),
                    Field::Height(Length::Px(32.)),
                ])],
            ),
            Op::SetAccessibility(node(12), Some(metadata(Role::Tree(false)))),
            Op::SetTreeInput(node(12), true),
            Op::SetRoot(Some(node(12))),
        ],
    );
    mount(cx, handle, 1, false);
    frame(cx, handle).await;
    let transport = handle
        .update(cx, |view, _, _| view.transport.clone())
        .unwrap();
    // Let any setup frames finish. The following wait never requests a frame:
    // a successful paint-time focus handoff must publish its pin by itself.
    cx.background_executor()
        .timer(std::time::Duration::from_millis(100))
        .await;
    transport.mailbox.lock().unwrap().drain(256);
    apply(cx, handle, vec![command(1, ScrollTarget::FocusTreeRow(1))]);
    let mut observed_pin = false;
    for _ in 0..100 {
        let events = transport.mailbox.lock().unwrap().drain(256);
        if events.iter().any(|event| {
            matches!(event,
            Event::ListViewport(_, owner, _, _, viewport)
                if *owner == node(12) && viewport.pinned == vec![1])
        }) {
            observed_pin = true;
            break;
        }
        cx.background_executor()
            .timer(std::time::Duration::from_millis(10))
            .await;
    }
    assert!(
        observed_pin,
        "paint-time focus must publish its pin while OCaml is idle"
    );
    assert_eq!(focused(cx, handle), vec![1]);
    assert_eq!(pending(cx, handle), None);

    // The single-row budget must release the old row and request a placeholder.
    apply(cx, handle, vec![command(2, ScrollTarget::FocusTreeRow(99))]);
    frame(cx, handle).await;
    assert_eq!(pending(cx, handle), Some(99));
    assert!(focused(cx, handle).is_empty());
    let events = transport.mailbox.lock().unwrap().drain(256);
    assert!(
        events.iter().any(|event| matches!(event,
        Event::ListViewport(_, owner, _, _, viewport)
            if *owner == node(12) && viewport.requested == vec![99] && viewport.pinned.is_empty()))
    );
    mount(cx, handle, 99, false);
    frame(cx, handle).await;
    assert_eq!(focused(cx, handle), vec![99]);
    assert_eq!(pending(cx, handle), None);

    // Stable row identity follows reorder while waiting for materialization.
    apply(cx, handle, vec![command(3, ScrollTarget::FocusTreeRow(75))]);
    frame(cx, handle).await;
    apply(cx, handle, vec![order(2, (1..=100).rev())]);
    frame(cx, handle).await;
    assert_eq!(pending(cx, handle), Some(75));
    mount(cx, handle, 75, false);
    frame(cx, handle).await;
    assert_eq!(focused(cx, handle), vec![75]);

    // A newer ordinary reveal supersedes pending focus; duplicate serials cannot revive it.
    apply(cx, handle, vec![command(4, ScrollTarget::FocusTreeRow(55))]);
    frame(cx, handle).await;
    apply(cx, handle, vec![command(5, ScrollTarget::Reveal(50))]);
    assert_eq!(pending(cx, handle), None);
    apply(cx, handle, vec![command(4, ScrollTarget::FocusTreeRow(55))]);
    assert_eq!(pending(cx, handle), None);

    // Deletion/reinsertion and handler retirement never revive pending commands.
    apply(cx, handle, vec![command(6, ScrollTarget::FocusTreeRow(66))]);
    frame(cx, handle).await;
    apply(
        cx,
        handle,
        vec![order(3, (1..=100).filter(|row| *row != 66))],
    );
    assert_eq!(pending(cx, handle), None);
    apply(cx, handle, vec![order(4, 1..=100)]);
    frame(cx, handle).await;
    assert_eq!(pending(cx, handle), None);
    apply(cx, handle, vec![command(7, ScrollTarget::FocusTreeRow(55))]);
    frame(cx, handle).await;
    apply(cx, handle, vec![Op::Bind(node(12), Some(handler(21)))]);
    assert_eq!(pending(cx, handle), None);
    mount(cx, handle, 55, false);
    frame(cx, handle).await;
    assert!(focused(cx, handle).is_empty());

    // Blur retires the handoff even if the user later focuses the same root again.
    apply(cx, handle, vec![command(8, ScrollTarget::FocusTreeRow(44))]);
    frame(cx, handle).await;
    handle.update(cx, |_, window, cx| window.blur(cx)).unwrap();
    frame(cx, handle).await;
    assert_eq!(pending(cx, handle), None);
    root_focus(cx, handle);
    mount(cx, handle, 44, false);
    frame(cx, handle).await;
    assert!(focused(cx, handle).is_empty());

    apply(cx, handle, vec![command(9, ScrollTarget::FocusTreeRow(33))]);
    frame(cx, handle).await;
    mount(cx, handle, 33, true);
    frame(cx, handle).await;
    assert_eq!(pending(cx, handle), None);
    assert!(focused(cx, handle).is_empty());

    apply(
        cx,
        handle,
        vec![command(10, ScrollTarget::FocusTreeRow(22))],
    );
    frame(cx, handle).await;
    apply(cx, handle, vec![inert(true)]);
    assert_eq!(
        pending(cx, handle),
        None,
        "retire without waiting for a hidden paint"
    );
    frame(cx, handle).await;
    apply(cx, handle, vec![inert(false)]);
    mount(cx, handle, 22, false);
    frame(cx, handle).await;
    assert!(focused(cx, handle).is_empty());

    apply(
        cx,
        handle,
        vec![command(11, ScrollTarget::FocusTreeRow(11))],
    );
    frame(cx, handle).await;
    assert_eq!(pending(cx, handle), Some(11));
    // Actual OS window deactivation drops the request without a paint/poll loop.
    let (session, transport) = handle
        .update(cx, |view, _, _| {
            (view.session.clone(), view.transport.clone())
        })
        .unwrap();
    let other_id = WindowId::from_parts(1, 1).unwrap();
    session
        .borrow_mut()
        .open(2, other_id, "Tree focus isolation", 240., 120.)
        .unwrap();
    let other = cx.update(|cx| {
        cx.open_window(
            WindowOptions {
                focus: true,
                window_bounds: Some(WindowBounds::Windowed(Bounds::centered(
                    None,
                    size(px(240.), px(120.)),
                    cx,
                ))),
                ..Default::default()
            },
            |_, cx| cx.new(|_| View::new(other_id, session.clone(), transport.clone())),
        )
        .unwrap()
    });
    activation(cx, handle, false).await;
    assert_eq!(pending(cx, handle), None);
    // A new focus command scrolls but cannot activate the background window.
    apply(
        cx,
        handle,
        vec![command(12, ScrollTarget::FocusTreeRow(10))],
    );
    assert_eq!(pending(cx, handle), None);
    assert!(!handle.update(cx, |_, w, _| w.is_window_active()).unwrap());
    other
        .update(cx, |view, window, _| {
            view.session.borrow_mut().close(view.id).unwrap();
            window.remove_window();
        })
        .unwrap();
    assert!(other.update(cx, |_, _, _| ()).is_err());
    handle
        .update(cx, |_, window, _| window.activate_window())
        .unwrap();
    activation(cx, handle, true).await;
    frame(cx, handle).await;
    assert_eq!(pending(cx, handle), None);

    // Native wheel scrolling away cancels pending focus even when root focus stays.
    apply(
        cx,
        handle,
        vec![command(13, ScrollTarget::FocusTreeRow(50))],
    );
    frame(cx, handle).await;
    assert_eq!(pending(cx, handle), Some(50));
    let point = gpui::point(px(40.), px(16.));
    super::super::native_test::move_mouse(cx, handle, point, false);
    handle
        .update(cx, |_, window, cx| {
            for phase in [gpui::TouchPhase::Started, gpui::TouchPhase::Ended] {
                window.dispatch_event(
                    gpui::PlatformInput::ScrollWheel(gpui::ScrollWheelEvent {
                        position: point,
                        delta: gpui::ScrollDelta::Pixels(gpui::point(px(0.), px(320.))),
                        touch_phase: phase,
                        modifiers: Default::default(),
                    }),
                    cx,
                );
            }
        })
        .unwrap();
    frame(cx, handle).await;
    assert_eq!(pending(cx, handle), None);
    apply(
        cx,
        handle,
        vec![command(14, ScrollTarget::FocusTreeRow(11))],
    );
    frame(cx, handle).await;
    assert_eq!(pending(cx, handle), Some(11));
    // A zero-width viewport cannot accept native row focus or revive on restore.
    apply(
        cx,
        handle,
        vec![Op::SetStyle(
            node(12),
            vec![Style::Fields(vec![
                Field::Width(Length::Px(0.)),
                Field::Height(Length::Px(32.)),
            ])],
        )],
    );
    frame(cx, handle).await;
    assert_eq!(pending(cx, handle), None);
    apply(cx, handle, vec![inert(false)]);
    frame(cx, handle).await;
    assert!(focused(cx, handle).is_empty());
    apply(
        cx,
        handle,
        vec![command(15, ScrollTarget::FocusTreeRow(11))],
    );
    frame(cx, handle).await;
    assert_eq!(pending(cx, handle), Some(11));
    apply(
        cx,
        handle,
        vec![
            Op::SetRoot(None),
            Op::Remove(node(12)),
            Op::Remove(node(19)),
        ],
    );
    frame(cx, handle).await;
    handle
        .update(cx, |view, _, _| assert!(view.lists.is_empty()))
        .unwrap();
    eprintln!(
        "GPUIO_TREE_FOCUS_OK: sparse mount with one-row cap, stable reorder, superseded/duplicate commands, deletion/handler retirement, blur/disabled/inert/window deactivation/wheel/clipping cancellation and teardown"
    );
}
