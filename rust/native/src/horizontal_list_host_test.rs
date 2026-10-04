//! Production Host materialization/measurement through TestPlatform. No OS window.
use super::*;
use gpui::{
    Along, Axis as NativeAxis, ListOffset, ScrollDelta, ScrollWheelEvent, TouchPhase, point,
};
use gpuio_protocol::list::{
    Axis as ListAxis, Config as ListConfig, IdRun, Order, Row, ScrollPolicy, ScrollRequest,
    ScrollTarget,
};

#[test]
fn horizontal_host_demands_bounded_items_measures_width_and_retains_streamed_anchor() {
    let mut app = TestAppContext::single();
    app.update(gpui_base::init);
    let (_reader, writer) = UnixStream::pair().unwrap();
    let transport = Arc::new(Transport::new(writer.as_raw_fd()).unwrap());
    let session = Rc::new(RefCell::new(Session::default()));
    session.borrow_mut().hello(VERSION, CAPABILITIES).unwrap();
    session
        .borrow_mut()
        .open(1, w(), "Horizontal", 400., 200.)
        .unwrap();
    let (owner, cx) =
        app.add_window_view(|_, _| View::new(w(), session.clone(), transport.clone()));
    cx.simulate_a11y_active(true);
    apply(
        &owner,
        cx,
        vec![
            Op::Create(
                n(0),
                Kind::VirtualList,
                "".into(),
                Some(gpuio_protocol::HandlerId::from_parts(0, 1).unwrap()),
            ),
            Op::SetStyle(n(0), dimensions(200., 100.)),
            Op::SetListConfig(
                n(0),
                ListConfig {
                    estimated_height: 60.,
                    overscan: 60.,
                    max_active: 12,
                    scroll_policy: ScrollPolicy::FollowTailWhenAtEnd,
                    scrollbar: true,
                    managed: true,
                },
            ),
            Op::SetListAxis(n(0), ListAxis::Horizontal),
            Op::SetListOrder(
                n(0),
                Order {
                    revision: 1,
                    runs: vec![IdRun {
                        first: 1,
                        count: 100_000,
                    }],
                },
            ),
            Op::SetRoot(Some(n(0))),
        ],
    );
    let handle = owner.read_with(cx, |v, _| v.lists[&n(0)].borrow().native.handle().clone());
    let observed = |cx: &mut VisualTestContext| {
        owner.read_with(cx, |v, _| v.lists[&n(0)].borrow().observed.clone().unwrap())
    };
    assert!(observed(cx).following_tail);
    assert!(observed(cx).at_end);
    assert!(observed(cx).requested.len() <= 12);
    assert_eq!(handle.axis(), NativeAxis::Horizontal);
    assert!(
        cx.a11y_tree()
            .unwrap()
            .nodes
            .iter()
            .any(|(_, node)| node.role() == gpui::accesskit::Role::List
                && node.orientation() == Some(gpui::accesskit::Orientation::Horizontal))
    );
    apply(
        &owner,
        cx,
        vec![Op::ScrollList(
            n(0),
            ScrollRequest {
                serial: 1,
                target: ScrollTarget::Offset(50_001, 13.),
            },
        )],
    );
    let placeholder = observed(cx);
    assert_eq!(placeholder.anchor, Some((50_001, 13.)));
    assert!(!placeholder.following_tail);
    assert!(placeholder.requested.len() <= 12);
    assert_eq!(placeholder.visible_first, 50_000);
    assert!(placeholder.visible_last > 50_000 && placeholder.visible_last < 50_010);
    let rows: Vec<_> = placeholder
        .requested
        .iter()
        .enumerate()
        .map(|(ix, id)| Row {
            id: *id,
            node: n(ix as i64 + 1),
        })
        .collect();
    let mut ops = vec![];
    for row in &rows {
        ops.extend([
            Op::Create(row.node, Kind::Text, format!("Item {}", row.id), None),
            Op::SetStyle(row.node, dimensions(40. + (row.id % 3) as f64 * 20., 100.)),
        ]);
    }
    ops.extend([
        Op::SetListRows(n(0), rows.clone()),
        Op::Splice(n(0), 0, 0, rows.iter().map(|r| r.node).collect()),
    ]);
    apply(&owner, cx, ops);
    let anchor = observed(cx).anchor;
    assert_eq!(anchor, Some((50_001, 13.)));
    let bounds = handle.bounds_for_item(50_000).unwrap();
    assert_eq!(bounds.size, gpui::size(px(40.), px(100.)));
    assert_eq!(bounds.origin.x, handle.viewport_bounds().origin.x - px(13.));
    assert_eq!(bounds.origin.y, handle.viewport_bounds().origin.y);
    let row = rows.iter().find(|r| r.id == 50_001).unwrap();
    apply(
        &owner,
        cx,
        vec![
            Op::SetText(row.node, "streamed item".into()),
            Op::SetStyle(row.node, dimensions(140., 100.)),
            Op::InvalidateListRows(n(0), vec![row.id]),
        ],
    );
    assert_eq!(observed(cx).anchor, anchor);
    assert_eq!(handle.bounds_for_item(50_000).unwrap().size.width, px(140.));
    // Cross-axis resize remeasures without resetting the logical anchor.
    apply(&owner, cx, vec![Op::SetStyle(n(0), dimensions(200., 120.))]);
    assert_eq!(observed(cx).anchor, anchor);
    apply(
        &owner,
        cx,
        vec![Op::SetListOrder(
            n(0),
            Order {
                revision: 2,
                runs: vec![
                    IdRun {
                        first: 100_001,
                        count: 1,
                    },
                    IdRun {
                        first: 1,
                        count: 100_000,
                    },
                ],
            },
        )],
    );
    assert_eq!(observed(cx).anchor, anchor);
    assert_eq!(handle.logical_scroll_top().item_ix, 50_001);
    // A vertical gesture does not cancel pending navigation or move this owner.
    let offset = || {
        let offset = handle.logical_scroll_top();
        (offset.item_ix, offset.offset_in_item)
    };
    let before = offset();
    let wheel = |cx: &mut VisualTestContext, delta| {
        cx.simulate_event(ScrollWheelEvent {
            position: handle.viewport_bounds().origin + point(px(20.), px(20.)),
            delta: ScrollDelta::Pixels(delta),
            modifiers: Default::default(),
            touch_phase: TouchPhase::Moved,
        });
        draw(cx);
    };
    wheel(cx, point(px(0.), px(-20.)));
    assert_eq!(offset(), before);
    wheel(cx, point(px(-20.), px(0.)));
    assert_ne!(offset(), before);
    assert_eq!(
        handle
            .scroll_px_offset_for_scrollbar()
            .along(NativeAxis::Vertical),
        px(0.)
    );
    // Commands are fenced and routed along the same physical axis.
    apply(
        &owner,
        cx,
        vec![Op::ScrollList(
            n(0),
            ScrollRequest {
                serial: 2,
                target: ScrollTarget::End,
            },
        )],
    );
    assert!(observed(cx).following_tail && observed(cx).at_end);
    assert!(observed(cx).requested.len() <= 12);
    handle.scroll_to(ListOffset {
        item_ix: 50_001,
        offset_in_item: px(13.),
    });
    draw(cx);
    apply(
        &owner,
        cx,
        vec![Op::ScrollList(
            n(0),
            ScrollRequest {
                serial: 2,
                target: ScrollTarget::End,
            },
        )],
    );
    assert!(
        !observed(cx).following_tail,
        "duplicate command must not replay"
    );
    cx.update(|window, _| window.remove_window());
}
