//! Same-session window isolation and source identity during live native drags.
use super::*;
use gpuio_protocol::{
    accessibility::{Config as Metadata, Live, Role, TreeItem},
    list::{Config, IdRun, Order, Row, ScrollPolicy},
    tree_input::{Placement, Request},
};

fn metadata(role: Role, label: &str) -> Metadata {
    Metadata {
        role: Some(role),
        label: Some(label.into()),
        description: None,
        live: Live::Off,
        field: None,
        current: None,
    }
}
fn dimensions(width: f64, height: f64) -> Vec<Style> {
    vec![Style::Fields(vec![
        Field::Width(Length::Px(width)),
        Field::Height(Length::Px(height)),
        Field::Shrink(0.),
    ])]
}
fn setup() -> Vec<Op> {
    let mut ops = vec![
        Op::Create(
            node(0),
            Kind::VirtualList,
            String::new(),
            Some(gpuio_protocol::HandlerId::from_parts(0, 1).unwrap()),
        ),
        Op::SetListConfig(
            node(0),
            Config {
                estimated_height: 32.,
                overscan: 0.,
                max_active: 2,
                scroll_policy: ScrollPolicy::KeepPosition,
                scrollbar: false,
                managed: true,
            },
        ),
        order(1, [11, 22]),
        Op::SetStyle(node(0), dimensions(300., 96.)),
        Op::SetAccessibility(node(0), Some(metadata(Role::Tree(true), "Moves"))),
        Op::SetTreeInput(node(0), true),
        Op::SetTreeMoves(node(0), true),
    ];
    for (index, label) in ["Source", "Destination"].into_iter().enumerate() {
        let row = node(1 + index as i64 * 2);
        let content = node(2 + index as i64 * 2);
        ops.extend([
            Op::Create(row, Kind::Container, String::new(), None),
            Op::SetStyle(row, dimensions(300., 32.)),
            Op::SetAccessibility(
                row,
                Some(metadata(
                    Role::TreeItem(TreeItem {
                        level: 1,
                        index: index as i64,
                        count: Some(2),
                        expanded: None,
                        selected: false,
                        disabled: false,
                        busy: false,
                    }),
                    label,
                )),
            ),
            Op::Create(content, Kind::Text, label.into(), None),
            Op::Splice(row, 0, 0, vec![content]),
        ]);
    }
    ops.extend([
        Op::Splice(node(0), 0, 0, vec![node(1), node(3)]),
        Op::SetListRows(
            node(0),
            vec![
                Row {
                    id: 11,
                    node: node(1),
                },
                Row {
                    id: 22,
                    node: node(3),
                },
            ],
        ),
        Op::SetRoot(Some(node(0))),
    ]);
    ops
}
fn order(revision: i64, ids: [i64; 2]) -> Op {
    Op::SetListOrder(
        node(0),
        Order {
            revision,
            runs: ids
                .into_iter()
                .map(|first| IdRun { first, count: 1 })
                .collect(),
        },
    )
}
fn paint(cx: &mut gpui::AsyncApp, handle: WindowHandle<View>) {
    for _ in 0..2 {
        let arena = cx
            .update_window(handle.into(), |_, window, cx| {
                window.refresh();
                window.draw(cx)
            })
            .unwrap();
        cx.update(|cx| arena.clear(cx));
    }
}
fn point(
    cx: &mut gpui::AsyncApp,
    handle: WindowHandle<View>,
    index: usize,
) -> gpui::Point<gpui::Pixels> {
    handle
        .update(cx, |view, _, _| {
            view.lists[&node(0)]
                .borrow()
                .native
                .handle()
                .bounds_for_item(index)
                .unwrap()
                .center()
        })
        .unwrap()
}
fn active(cx: &mut gpui::AsyncApp, handle: WindowHandle<View>) -> bool {
    cx.update_window(handle.into(), |_, _, cx| cx.has_active_drag())
        .unwrap()
}
fn begin(cx: &mut gpui::AsyncApp, handle: WindowHandle<View>, index: usize) {
    let position = point(cx, handle, index);
    super::super::native_test::move_mouse(cx, handle, position, false);
    paint(cx, handle);
    super::super::native_test::mouse(cx, handle, position, true);
    super::super::native_test::move_mouse(
        cx,
        handle,
        position + gpui::point(px(12.), px(0.)),
        true,
    );
    paint(cx, handle);
    assert!(active(cx, handle));
}
fn release(cx: &mut gpui::AsyncApp, handle: WindowHandle<View>, index: usize) {
    let position = point(cx, handle, index) + gpui::point(px(0.), px(8.));
    super::super::native_test::move_mouse(cx, handle, position, true);
    paint(cx, handle);
    super::super::native_test::mouse(cx, handle, position, false);
    paint(cx, handle);
}
fn moves(transport: &Transport) -> Vec<(WindowId, Request)> {
    transport
        .mailbox
        .lock()
        .unwrap()
        .drain(128)
        .into_iter()
        .filter_map(|event| {
            if let Event::TreeInput(window, _, _, _, request @ Request::Move { .. }) = event {
                Some((window, request))
            } else {
                None
            }
        })
        .collect()
}
async fn activated(cx: &mut gpui::AsyncApp, handle: WindowHandle<View>, expected: bool) {
    for _ in 0..100 {
        if handle
            .update(cx, |_, window, _| window.is_window_active())
            .unwrap()
            == expected
        {
            return;
        }
        cx.background_executor()
            .timer(std::time::Duration::from_millis(10))
            .await;
    }
    panic!("tree drag window activation did not become {expected}");
}
fn close(cx: &mut gpui::AsyncApp, handle: WindowHandle<View>) {
    handle
        .update(cx, |view, window, cx| {
            // The same close sequence used by the production CloseWindow handler.
            view.session.borrow_mut().close(view.id).unwrap();
            view.cancel_tree_drag(window, cx);
            window.remove_window();
        })
        .unwrap();
}

pub(super) async fn exercise(cx: &mut gpui::AsyncApp, primary: WindowHandle<View>) {
    let (session, transport) = primary
        .update(cx, |view, _, _| {
            (view.session.clone(), view.transport.clone())
        })
        .unwrap();
    // The preceding focus suite closed slot 1, generation 1.
    let source_id = WindowId::from_parts(1, 2).unwrap();
    let other_id = WindowId::from_parts(2, 1).unwrap();
    let open = |cx: &mut gpui::AsyncApp, id: WindowId, focus: bool| {
        session
            .borrow_mut()
            .open(50 + id.slot() as i64, id, "Tree drag lifecycle", 320., 160.)
            .unwrap();
        let handle = cx.update(|cx| {
            cx.open_window(
                WindowOptions {
                    focus,
                    window_bounds: Some(WindowBounds::Windowed(Bounds::centered(
                        None,
                        size(px(320.), px(160.)),
                        cx,
                    ))),
                    ..Default::default()
                },
                |_, cx| cx.new(|_| View::new(id, session.clone(), transport.clone())),
            )
            .unwrap()
        });
        apply(cx, handle, setup());
        paint(cx, handle);
        handle
    };
    let source = open(cx, source_id, true);
    let other = open(cx, other_id, false);
    source
        .update(cx, |_, window, _| window.activate_window())
        .unwrap();
    activated(cx, source, true).await;
    paint(cx, source);
    moves(&transport);

    begin(cx, source, 0);
    let lease = source
        .update(cx, |view, _, _| view.tree_drag.clone())
        .unwrap();
    apply(cx, source, vec![order(2, [22, 11])]);
    paint(cx, source);
    assert!(
        active(cx, source),
        "sibling reorder preserves the gesture's stable source"
    );
    release(cx, source, 0);
    assert_eq!(
        moves(&transport),
        vec![(
            source_id,
            Request::Move {
                source: 11,
                destination: 22,
                placement: Placement::After
            }
        )]
    );
    assert!(lease.upgrade().is_none());
    apply(cx, source, vec![order(3, [11, 22])]);
    paint(cx, source);

    begin(cx, source, 0);
    release(cx, other, 1);
    assert!(
        moves(&transport).is_empty(),
        "a foreign window never accepts this tree's move"
    );
    key(cx, source, "escape");
    paint(cx, source);
    assert!(!active(cx, source));

    begin(cx, source, 0);
    let lease = source
        .update(cx, |view, _, _| view.tree_drag.clone())
        .unwrap();
    other
        .update(cx, |_, window, _| window.activate_window())
        .unwrap();
    activated(cx, source, false).await;
    activated(cx, other, true).await;
    assert!(
        !active(cx, other),
        "deactivation cancels the native preview immediately"
    );
    assert!(lease.upgrade().is_none());
    release(cx, other, 1);
    assert!(
        moves(&transport).is_empty(),
        "late release after deactivation cannot propose"
    );

    source
        .update(cx, |_, window, _| window.activate_window())
        .unwrap();
    activated(cx, source, true).await;
    paint(cx, source);
    begin(cx, source, 0);
    let lease = source
        .update(cx, |view, _, _| view.tree_drag.clone())
        .unwrap();
    close(cx, source);
    assert!(source.update(cx, |_, _, _| ()).is_err());
    assert!(
        lease.upgrade().is_none(),
        "closing source releases its gesture lease"
    );
    other
        .update(cx, |_, window, _| window.activate_window())
        .unwrap();
    activated(cx, other, true).await;
    assert!(!active(cx, other));
    release(cx, other, 1);
    assert!(moves(&transport).is_empty());
    close(cx, other);
    primary
        .update(cx, |_, window, _| window.activate_window())
        .unwrap();
    activated(cx, primary, true).await;
    paint(cx, primary);
    eprintln!(
        "GPUIO_TREE_DRAG_LIFECYCLE_OK: sibling reorder preserves source; foreign-window drop rejected; real deactivation and close release preview; late releases ignored"
    );
}
