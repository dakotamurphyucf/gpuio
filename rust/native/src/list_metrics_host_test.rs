//! Inherited row metrics through the production host; no physical GUI claim.
use super::*;
use gpui::{ListOffset, ScrollDelta, ScrollWheelEvent, TouchPhase, point};
use gpuio_protocol::list::{Config as ListConfig, IdRun, Order, Row, ScrollPolicy};

#[test]
fn inherited_line_height_preserves_pixel_scrolling_across_warm_rows() {
    let mut app = TestAppContext::single();
    app.update(gpui_base::init);
    let (_reader, writer) = UnixStream::pair().unwrap();
    let transport = Arc::new(Transport::new(writer.as_raw_fd()).unwrap());
    let session = Rc::new(RefCell::new(Session::default()));
    session.borrow_mut().hello(VERSION, CAPABILITIES).unwrap();
    session
        .borrow_mut()
        .open(1, w(), "List metrics", 400., 300.)
        .unwrap();
    let (owner, cx) =
        app.add_window_view(|_, _| View::new(w(), session.clone(), transport.clone()));
    let ancestor_style = |height| {
        vec![Style::Fields(vec![
            Field::FontSize(12.),
            Field::LineHeight(Length::Px(height)),
        ])]
    };
    let rows: Vec<_> = (0..100)
        .map(|i| Row {
            id: i + 1,
            node: n(i + 2),
        })
        .collect();
    let mut ops = vec![
        Op::Create(n(0), Kind::Container, "".into(), None),
        Op::SetStyle(n(0), ancestor_style(20.)),
        Op::Create(
            n(1),
            Kind::VirtualList,
            "".into(),
            Some(gpuio_protocol::HandlerId::from_parts(0, 1).unwrap()),
        ),
        Op::SetStyle(n(1), dimensions(200., 120.)),
        Op::SetListConfig(
            n(1),
            ListConfig {
                estimated_height: 20.,
                overscan: 100.,
                max_active: 128,
                scroll_policy: ScrollPolicy::KeepPosition,
                scrollbar: false,
                managed: false,
            },
        ),
        Op::SetListOrder(
            n(1),
            Order {
                revision: 1,
                runs: vec![IdRun {
                    first: 1,
                    count: 100,
                }],
            },
        ),
    ];
    for row in &rows {
        ops.push(Op::Create(
            row.node,
            Kind::Text,
            format!("Row {}", row.id),
            None,
        ));
    }
    ops.extend([
        Op::Splice(n(1), 0, 0, rows.iter().map(|r| r.node).collect()),
        Op::SetListRows(n(1), rows),
        Op::Splice(n(0), 0, 0, vec![n(1)]),
        Op::SetRoot(Some(n(0))),
    ]);
    apply(&owner, cx, ops);
    let handle = owner.read_with(cx, |v, _| v.lists[&n(1)].borrow().native.handle().clone());
    // Prime a distant measurement as a control: paint-only inheritance must
    // keep it, whereas a metrics change must retire it before future scrolling.
    for index in [90, 50] {
        handle.scroll_to(ListOffset {
            item_ix: index,
            offset_in_item: px(5.),
        });
        draw(cx);
    }
    assert_eq!(handle.bounds_for_item(50).unwrap().size.height, px(20.));
    let mut paint_only = ancestor_style(20.);
    paint_only.push(Style::Fields(vec![Field::Foreground(Color::Rgba(
        0xff0000ff,
    ))]));
    apply(&owner, cx, vec![Op::SetStyle(n(0), paint_only)]);
    assert_eq!(handle.bounds_for_item(90).unwrap().size.height, px(20.));
    for height in [40., 12., 28.] {
        handle.scroll_to(ListOffset {
            item_ix: 50,
            offset_in_item: px(5.),
        });
        draw(cx);
        apply(&owner, cx, vec![Op::SetStyle(n(0), ancestor_style(height))]);
        assert_eq!(handle.logical_scroll_top().item_ix, 50);
        assert_eq!(handle.logical_scroll_top().offset_in_item, px(5.));
        assert_eq!(
            handle.bounds_for_item(50).unwrap().size.height,
            px(height as f32)
        );
        assert!(
            handle.bounds_for_item(90).is_none(),
            "distant old metrics must be invalidated"
        );
        let before = handle.bounds_for_item(50).unwrap().origin.y;
        cx.simulate_event(ScrollWheelEvent {
            position: handle.viewport_bounds().origin + point(px(20.), px(20.)),
            delta: ScrollDelta::Pixels(point(px(0.), px(24.))),
            modifiers: Default::default(),
            touch_phase: TouchPhase::Moved,
        });
        draw(cx);
        let after = handle.bounds_for_item(50).unwrap().origin.y;
        assert_eq!(
            after - before,
            px(24.),
            "warm-row remeasurement must not add a scroll jump"
        );
    }
}
