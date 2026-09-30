//! A real list nested in an ordinary scroller must consume only its own movement.
use super::*;
use gpuio_protocol::list::{Config, IdRun, Order, Row, ScrollPolicy};

fn list_offset(cx: &mut gpui::AsyncApp, window: WindowHandle<View>) -> gpui::Pixels {
    window
        .update(cx, |view, _, _| {
            -view.lists[&node(16)]
                .borrow()
                .native
                .handle()
                .scroll_px_offset_for_scrollbar()
                .y
        })
        .unwrap()
}

fn list_bounds(cx: &mut gpui::AsyncApp, window: WindowHandle<View>) -> Bounds<gpui::Pixels> {
    window
        .update(cx, |view, _, _| {
            view.lists[&node(16)]
                .borrow()
                .native
                .handle()
                .viewport_bounds()
        })
        .unwrap()
}

async fn position(
    cx: &mut gpui::AsyncApp,
    window: WindowHandle<View>,
    item_ix: usize,
    offset: f32,
) {
    window
        .update(cx, |view, window, _| {
            view.lists[&node(16)]
                .borrow()
                .native
                .handle()
                .scroll_to(gpui::ListOffset {
                    item_ix,
                    offset_in_item: px(offset),
                });
            window.refresh();
        })
        .unwrap();
    frame(cx, window).await;
}

pub(super) async fn exercise(cx: &mut gpui::AsyncApp, window: WindowHandle<View>) {
    let mut outer = scrolling(360., 240., false);
    outer.push(Style::Fields(vec![Field::OverflowX(3)]));
    let mut operations = vec![
        Op::Create(node(15), Kind::Container, "".into(), None),
        Op::SetStyle(node(15), outer),
        Op::Create(node(16), Kind::VirtualList, "".into(), None),
        Op::SetStyle(node(16), dimensions(300., 180.)),
        Op::SetListConfig(
            node(16),
            Config {
                estimated_height: 100.,
                overscan: 0.,
                max_active: 16,
                scroll_policy: ScrollPolicy::KeepPosition,
                scrollbar: false,
                managed: false,
            },
        ),
        Op::SetListOrder(
            node(16),
            Order {
                revision: 1,
                runs: vec![IdRun { first: 1, count: 6 }],
            },
        ),
        Op::Create(node(17), Kind::Container, "".into(), None),
        Op::SetStyle(node(17), dimensions(280., 100.)),
        Op::Create(node(18), Kind::Container, "".into(), None),
        Op::SetStyle(node(18), scrolling(240., 60., false)),
        Op::Create(node(19), Kind::Text, "Nested content\n".repeat(30), None),
        Op::SetStyle(node(19), dimensions(240., 500.)),
        Op::Splice(node(18), 0, 0, vec![node(19)]),
        Op::Splice(node(17), 0, 0, vec![node(18)]),
    ];
    for slot in 20..=24 {
        operations.extend([
            Op::Create(node(slot), Kind::Text, format!("Row {}", slot - 18), None),
            Op::SetStyle(node(slot), dimensions(280., 100.)),
        ]);
    }
    let rows: Vec<_> = std::iter::once(17).chain(20..=24).collect();
    operations.extend([
        Op::Create(node(25), Kind::Text, "Outer extent".into(), None),
        Op::SetStyle(node(25), dimensions(700., 900.)),
        Op::SetListRows(
            node(16),
            rows.iter()
                .enumerate()
                .map(|(index, slot)| Row {
                    id: index as i64 + 1,
                    node: node(*slot),
                })
                .collect(),
        ),
        Op::Splice(
            node(16),
            0,
            0,
            rows.iter().map(|slot| node(*slot)).collect(),
        ),
        Op::Splice(node(15), 0, 0, vec![node(16), node(25)]),
        Op::SetRoot(Some(node(15))),
    ]);
    apply(cx, window, operations);
    frame(cx, window).await;
    let viewport_bounds = list_bounds(cx, window);
    let point = viewport_bounds.origin + gpui::point(px(270.), px(90.));
    wheel(cx, window, point, 0., -1.).await;
    assert_eq!(list_offset(cx, window), px(1.));
    assert_eq!(
        offset(cx, window, 15),
        Default::default(),
        "list wheel also moved ancestor"
    );
    wheel(cx, window, point, 0., -49.).await;
    assert_eq!(list_offset(cx, window), px(50.));
    wheel_sample(
        cx,
        window,
        point,
        gpui::ScrollDelta::Lines(gpui::point(0., -1.)),
        gpui::TouchPhase::Moved,
    )
    .await;
    assert_eq!(list_offset(cx, window), px(70.));
    assert_eq!(offset(cx, window, 15), Default::default());

    position(cx, window, 0, 0.).await;
    let child_point = bounds(cx, window, 18).center();
    wheel(cx, window, child_point, 0., -30.).await;
    assert_eq!(offset(cx, window, 18).y, px(-30.));
    assert_eq!(list_offset(cx, window), px(0.));
    assert_eq!(offset(cx, window, 15), Default::default());
    window
        .update(cx, |view, window, _| {
            let handle = &view.scrolls[&node(18)].handle;
            handle.set_offset(-handle.max_offset());
            window.refresh();
        })
        .unwrap();
    frame(cx, window).await;
    wheel(cx, window, child_point, 0., -25.).await;
    assert_eq!(list_offset(cx, window), px(25.));
    assert_eq!(offset(cx, window, 15), Default::default());

    position(cx, window, 5, 100.).await;
    assert_eq!(list_offset(cx, window), px(420.));
    wheel(cx, window, point, 0., -25.).await;
    assert_eq!(list_offset(cx, window), px(420.));
    assert_eq!(
        offset(cx, window, 15).y,
        px(-25.),
        "boundary should reach ancestor"
    );
    let point = list_bounds(cx, window).center();
    wheel(cx, window, point, 0., 20.).await;
    assert_eq!(list_offset(cx, window), px(400.));
    assert_eq!(offset(cx, window, 15).y, px(-25.));
    wheel(cx, window, point, -35., 0.).await;
    assert_eq!(list_offset(cx, window), px(400.));
    assert_eq!(offset(cx, window, 15), gpui::point(px(-35.), px(-25.)));

    let point = list_bounds(cx, window).center();
    window
        .update(cx, |_, window, cx| {
            for dy in [-1., -2.] {
                window.dispatch_event(
                    gpui::PlatformInput::ScrollWheel(gpui::ScrollWheelEvent {
                        position: point,
                        delta: gpui::ScrollDelta::Pixels(gpui::point(px(0.), px(dy))),
                        touch_phase: gpui::TouchPhase::Moved,
                        modifiers: Default::default(),
                    }),
                    cx,
                );
            }
        })
        .unwrap();
    frame(cx, window).await;
    assert_eq!(list_offset(cx, window), px(403.));
    assert_eq!(offset(cx, window, 15), gpui::point(px(-35.), px(-25.)));

    let owner = window
        .update(cx, |view, _, _| Rc::downgrade(&view.lists[&node(16)]))
        .unwrap();
    let mut remove = vec![Op::SetRoot(None)];
    remove.extend((15..=25).rev().map(|slot| Op::Remove(node(slot))));
    apply(cx, window, remove);
    assert!(
        owner.upgrade().is_none(),
        "wheel callbacks retain removed list"
    );
    frame(cx, window).await;
    window
        .update(cx, |view, _, _| {
            assert!(view.lists.is_empty());
            assert!(view.scrolls.is_empty());
        })
        .unwrap();
    eprintln!(
        "GPUIO_LIST_SCROLL_ROUTING_OK: consumed precise/discrete/coalesced list movement, nested child consumption/boundary, list boundary/horizontal propagation and weak owner disposal"
    );
}
