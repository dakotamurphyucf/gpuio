//! Retained Host rendering on TestPlatform; no physical desktop claim.
use super::behavior::{apply, draw};
use super::*;
use crate::session::Session;
use gpui::TestAppContext;
use gpuio_protocol::table_header::Target as Header;
use std::os::unix::net::UnixStream;

#[test]
fn rich_headers_render_controls_without_body_row_ownership() {
    exercise(false);
}

#[test]
fn partially_visible_composite_header_retires_only_its_clipped_control() {
    exercise(true);
}

fn exercise(composite: bool) {
    let mut app = TestAppContext::single();
    app.update(|cx| {
        gpui_base::init(cx);
        gpuio_table_adapter::init(cx);
    });
    let (_reader, writer) = UnixStream::pair().unwrap();
    let transport = Arc::new(Transport::new(writer.as_raw_fd()).unwrap());
    let session = Rc::new(RefCell::new(Session::default()));
    session.borrow_mut().hello(VERSION, CAPABILITIES).unwrap();
    session
        .borrow_mut()
        .open(1, window_id(), "Headers", 520., 300.)
        .unwrap();
    let (view, cx) =
        app.add_window_view(|_, _| View::new(window_id(), session.clone(), transport.clone()));
    cx.simulate_a11y_active(true);
    cx.update(|window, _| window.activate_window());
    let mut ops = initial();
    ops.retain(|op| !matches!(op, Op::SetTable(..)));
    let mut config = config();
    config.schema.headers = vec![vec![
        wire::Group {
            label: "Repeated".into(),
            columns: vec!["name".into()],
        },
        wire::Group {
            label: "Repeated".into(),
            columns: vec!["value".into()],
        },
    ]];
    if composite {
        config.schema.headers.push(config.schema.headers[0].clone());
    }
    ops.push(Op::SetTable(node(0), config.clone()));
    let handler = HandlerId::from_parts(10, 1).unwrap();
    for (slot, target) in [
        (61, Header::Column("name".into())),
        (
            63,
            Header::Group {
                level: if composite { 1 } else { 0 },
                columns: vec!["value".into()],
            },
        ),
    ] {
        ops.extend([
            Op::Create(node(slot), Kind::Container, String::new(), None),
            Op::SetTableHeader(node(slot), Some(target)),
            Op::Create(
                node(slot + 1),
                Kind::Button,
                format!("Header {slot}"),
                Some(handler),
            ),
            Op::SetStyle(
                node(slot + 1),
                vec![
                    Style::Width(Length::Px(90.)),
                    Style::Height(Length::Px(24.)),
                ],
            ),
            Op::Splice(node(slot), 0, 0, vec![node(slot + 1)]),
            Op::Splice(node(0), 0, 0, vec![node(slot)]),
        ]);
    }
    if composite {
        ops.extend([
            Op::Splice(node(61), 0, 1, vec![]),
            Op::Create(node(65), Kind::Container, String::new(), None),
            Op::SetStyle(
                node(65),
                vec![
                    Style::Width(Length::Px(400.)),
                    Style::Height(Length::Px(24.)),
                    Style::Direction(0),
                ],
            ),
            Op::Create(
                node(66),
                Kind::Button,
                "Right control".into(),
                Some(HandlerId::from_parts(11, 1).unwrap()),
            ),
            Op::SetStyle(
                node(66),
                vec![
                    Style::Width(Length::Px(90.)),
                    Style::Height(Length::Px(24.)),
                    Style::Fields(vec![Field::MarginLeft(Length::Px(160.))]),
                ],
            ),
            Op::Splice(node(65), 0, 0, vec![node(62), node(66)]),
            Op::Splice(node(61), 0, 0, vec![node(65)]),
        ]);
    }
    apply(&view, cx, ops);
    let native = view.read_with(cx, |v, _| v.tables[&node(0)].borrow().native.clone());
    let button = view.read_with(cx, |v, _| v.buttons[&node(62)].clone());
    for slot in [62, 64] {
        let bounds = view.read_with(cx, |v, _| v.probes.borrow()[&node(slot)].bounds);
        assert!(bounds.size.width > px(0.) && bounds.size.height > px(0.));
    }
    view.read_with(cx, |v, _| {
        let probes = v.probes.borrow();
        let leaf = probes[&node(62)].bounds;
        let group = probes[&node(64)].bounds;
        assert!(
            group.top() < leaf.top(),
            "group content occupies its own native level"
        );
        assert!(
            group.left() > leaf.left(),
            "repeated labels do not alias the pinned group"
        );
        if composite {
            assert!(
                group.top() >= px(32.),
                "explicit level one remains below the identical level-zero label"
            );
        }
    });
    let center = view.read_with(cx, |v, _| v.probes.borrow()[&node(62)].bounds.center());
    transport.mailbox.lock().unwrap().drain(1024);
    cx.simulate_click(center, gpui::Modifiers::default());
    draw(cx);
    let events = transport.mailbox.lock().unwrap().drain(1024);
    assert!(
        events.iter().any(
            |event| matches!(event, Event::Press(_, n, h, _) if *n == node(62) && *h == handler)
        ),
        "{events:?}"
    );
    assert!(
        !events
            .iter()
            .any(|event| matches!(event, Event::TableInput(..))),
        "child action must not select or sort the enclosing column: {events:?}"
    );
    cx.update(|window, cx| {
        view.update(cx, |v, cx| {
            button.focus.focus(window, cx);
            assert!(
                v.list_pins(window, cx)
                    .iter()
                    .all(|pin| pin.rows.is_empty())
            );
        })
    });
    transport.mailbox.lock().unwrap().drain(1024);
    cx.update(|window, cx| {
        let key = gpui::Keystroke::parse("enter").unwrap();
        window.dispatch_event(
            gpui::PlatformInput::KeyDown(gpui::KeyDownEvent {
                keystroke: key.clone(),
                is_held: false,
                prefer_character_input: false,
            }),
            cx,
        );
        window.dispatch_event(
            gpui::PlatformInput::KeyUp(gpui::KeyUpEvent { keystroke: key }),
            cx,
        );
    });
    let events = transport.mailbox.lock().unwrap().drain(1024);
    assert_eq!(
        events
            .iter()
            .filter(|event| matches!(event, Event::Press(_, n, _, _) if *n == node(62)))
            .count(),
        1,
        "one keyboard activation: {events:?}"
    );
    assert!(
        !events
            .iter()
            .any(|event| matches!(event, Event::TableInput(..))),
        "keyboard activation must remain owned by the header control"
    );
    cx.simulate_mouse_move(center, None, Default::default());
    cx.simulate_mouse_down(center, gpui::MouseButton::Left, Default::default());
    let moved = center + gpui::point(px(25.), px(0.));
    cx.simulate_mouse_move(moved, Some(gpui::MouseButton::Left), Default::default());
    assert!(
        !cx.update(|_, cx| cx.has_active_drag()),
        "dragging a header button must not reorder its column"
    );
    cx.simulate_mouse_up(moved, gpui::MouseButton::Left, Default::default());
    draw(cx);
    let header = cx
        .a11y_tree()
        .unwrap()
        .nodes
        .into_iter()
        .find(|(_, node)| {
            node.role() == gpui::accesskit::Role::ColumnHeader && node.label() == Some("name")
        })
        .expect("native column header semantics")
        .1
        .bounds()
        .expect("header bounds");
    let scale = cx.update(|window, _| f64::from(window.scale_factor()));
    let bare = gpui::point(
        px((header.x1 / scale) as f32 - 30.),
        px(((header.y0 + header.y1) * 0.5 / scale) as f32),
    );
    cx.simulate_mouse_move(bare, None, Default::default());
    cx.simulate_mouse_down(bare, gpui::MouseButton::Left, Default::default());
    cx.simulate_mouse_move(
        bare + gpui::point(px(20.), px(0.)),
        Some(gpui::MouseButton::Left),
        Default::default(),
    );
    assert!(
        cx.update(|_, cx| cx.has_active_drag()),
        "bare native header still supports column dragging: header={header:?} bare={bare:?} button={center:?}"
    );
    cx.update(|window, cx| {
        assert!(cx.stop_active_drag(window));
    });
    cx.simulate_mouse_up(bare, gpui::MouseButton::Left, Default::default());
    draw(cx);
    apply(&view, cx, vec![Op::SetText(node(62), "Updated".into())]);
    assert!(Rc::ptr_eq(
        &button,
        &view.read_with(cx, |v, _| v.buttons[&node(62)].clone())
    ));
    config.schema_revision = 2;
    apply(
        &view,
        cx,
        vec![
            Op::SetTableHeader(node(61), Some(Header::Column("value".into()))),
            Op::SetTable(node(0), config),
        ],
    );
    assert_eq!(
        native.entity_id(),
        view.read_with(cx, |v, _| v.tables[&node(0)].borrow().native.entity_id())
    );
    assert!(Rc::ptr_eq(
        &button,
        &view.read_with(cx, |v, _| v.buttons[&node(62)].clone())
    ));
    let button_ax = cx
        .a11y_tree()
        .unwrap()
        .nodes
        .into_iter()
        .find(|(_, node)| {
            node.role() == gpui::accesskit::Role::Button && node.label() == Some("Updated")
        })
        .expect("rich header button semantics")
        .0;
    native.update(cx, |t, cx| {
        t.horizontal_scroll_handle
            .set_offset(gpui::point(px(-120.), px(0.)));
        cx.notify();
    });
    draw(cx);
    view.read_with(cx, |v, _| {
        assert!(
            !v.focus.borrow().allows(node(62)),
            "clipped header button must retire input"
        )
    });
    if composite {
        view.read_with(cx, |v, _| {
            assert!(
                v.focus.borrow().allows(node(66)),
                "visible sibling stays interactive"
            )
        });
        let right = cx
            .a11y_tree()
            .unwrap()
            .nodes
            .into_iter()
            .find(|(_, node)| {
                node.role() == gpui::accesskit::Role::Button
                    && node.label() == Some("Right control")
            })
            .expect("visible sibling semantics")
            .0;
        transport.mailbox.lock().unwrap().drain(1024);
        cx.simulate_a11y_action(gpui::accesskit::ActionRequest {
            action: gpui::accesskit::Action::Click,
            target_node: right,
            target_tree: gpui::accesskit::TreeId::ROOT,
            data: None,
        });
        draw(cx);
        let events = transport.mailbox.lock().unwrap().drain(1024);
        assert_eq!(
            events
                .iter()
                .filter(|event| matches!(event, Event::Press(_, n, _, _) if *n == node(66)))
                .count(),
            1,
            "visible sibling acts once: {events:?}"
        );
        assert!(
            !events
                .iter()
                .any(|event| matches!(event, Event::TableInput(..)))
        );
    }
    transport.mailbox.lock().unwrap().drain(1024);
    cx.simulate_a11y_action(gpui::accesskit::ActionRequest {
        action: gpui::accesskit::Action::Click,
        target_node: button_ax,
        target_tree: gpui::accesskit::TreeId::ROOT,
        data: None,
    });
    draw(cx);
    assert!(
        !transport
            .mailbox
            .lock()
            .unwrap()
            .drain(1024)
            .iter()
            .any(|event| matches!(event, Event::Press(..)))
    );

    native.update(cx, |t, cx| {
        t.horizontal_scroll_handle
            .set_offset(gpui::point(px(0.), px(0.)));
        cx.notify();
    });
    draw(cx);
    view.read_with(cx, |v, _| assert!(v.focus.borrow().allows(node(62))));
    let mut remove = vec![Op::SetRoot(None)];
    if composite {
        remove.extend([
            Op::Remove(node(66)),
            Op::Remove(node(62)),
            Op::Remove(node(65)),
        ]);
    }
    remove.extend(
        (0..65)
            .rev()
            .filter(|slot| !composite || *slot != 62)
            .map(|slot| Op::Remove(node(slot))),
    );
    apply(&view, cx, remove);
    view.read_with(cx, |v, _| {
        assert!(v.tables.is_empty() && v.buttons.is_empty());
        assert_eq!(v.session.borrow().tree(v.id).unwrap().retained_bytes(), 0);
    });
}
