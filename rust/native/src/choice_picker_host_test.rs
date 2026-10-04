//! Retained native editor/owner lifecycle on TestPlatform, not OS presentation.
use super::*;
use gpui::TestAppContext;
use gpuio_protocol::{
    HandlerId,
    choice_picker::{Event as PickerEvent, *},
};
use std::{os::fd::AsRawFd, os::unix::net::UnixStream};
fn id(slot: i64) -> NodeId {
    NodeId::from_parts(slot, 1).unwrap()
}
fn handler(slot: i64) -> HandlerId {
    HandlerId::from_parts(slot, 1).unwrap()
}
fn apply(view: &mut View, window: &mut Window, cx: &mut Context<View>, operations: Vec<Op>) {
    let base = view.session.borrow().tree(view.id).unwrap().revision();
    let applied = view
        .session
        .borrow_mut()
        .apply(&Transaction {
            window: view.id,
            base,
            revision: base + 1,
            operations,
        })
        .unwrap();
    view.update_editors(&applied.dirty, window, cx);
}
fn config(search: bool, open_state: OpenState) -> Presentation {
    Presentation {
        config: Config {
            label: "Pick".into(),
            options: Collection::Flat(vec![]),
            selected: Selection::Single(None),
            disabled: false,
            search: if search {
                Search::Substring
            } else {
                Search::None
            },
            clearable: true,
            open_state,
            placeholder: "Choose".into(),
            search_placeholder: "Find".into(),
        },
        popup_width: 320.,
        max_height: 320.,
        estimated_row_height: 32.,
        overscan: 64.,
        empty_label: "Empty".into(),
        popup_style: vec![],
        option_style: vec![],
        header_style: vec![],
        empty_style: vec![],
        slots: if search {
            vec![Slot::Query, Slot::Footer]
        } else {
            vec![]
        },
    }
}
#[test]
fn retained_picker_owner_gates_hidden_editor_and_footer_and_preserves_managed_state() {
    let mut app = TestAppContext::single();
    app.update(gpui_base::init);
    let (_reader, writer) = UnixStream::pair().unwrap();
    let transport = Arc::new(Transport::new(writer.as_raw_fd()).unwrap());
    let session = Rc::new(RefCell::new(Session::default()));
    let wid = WindowId::from_parts(0, 1).unwrap();
    session.borrow_mut().hello(VERSION, CAPABILITIES).unwrap();
    session
        .borrow_mut()
        .open(1, wid, "Picker owner", 400., 200.)
        .unwrap();
    let (entity, cx) =
        app.add_window_view(|_, _| View::new(wid, session.clone(), transport.clone()));
    cx.update(|window, cx| {
        entity.update(cx, |view, cx| {
            let mut p = config(true, OpenState::Managed(false));
            let editor = EditorConfig {
                label: "Pick".into(),
                placeholder: "Find".into(),
                read_only: false,
                disabled: false,
                submit_on_enter: false,
                auto_focus: false,
                min_rows: 1,
                max_rows: 1,
            };
            apply(
                view,
                window,
                cx,
                vec![
                    Op::Create(id(0), Kind::ChoicePicker, "".into(), Some(handler(0))),
                    Op::SetChoicePicker(id(0), Box::new(p.clone())),
                    Op::Create(id(1), Kind::Container, "".into(), None),
                    Op::Create(id(2), Kind::Input, "kept query".into(), Some(handler(1))),
                    Op::SetEditor(id(2), editor),
                    Op::Create(id(3), Kind::Container, "".into(), None),
                    Op::Create(id(4), Kind::Button, "Footer".into(), Some(handler(2))),
                    Op::Splice(id(1), 0, 0, vec![id(2)]),
                    Op::Splice(id(3), 0, 0, vec![id(4)]),
                    Op::Splice(id(0), 0, 0, vec![id(1), id(3)]),
                    Op::SetRoot(Some(id(0))),
                ],
            );
            let focus = view.editors[&id(2)].focus_handle(cx);
            assert!(!view.pickers[&id(0)].state.is_open());
            assert!(!view.focus.borrow().allows(id(2)));
            assert!(!view.focus.borrow().allows(id(4)));
            assert!(view.focus.borrow().allows(id(0)));
            assert_eq!(
                view.editors
                    .get_mut(&id(2))
                    .unwrap()
                    .command(&EditorCommand::Focus, window, cx),
                EditorResult::Failed(EditorError::FocusBlocked)
            );
            let identity = view.focus.borrow().visibility_identity();
            view.sync_choice_pickers(&[], window, cx);
            assert!(Rc::ptr_eq(
                &identity,
                &view.focus.borrow().visibility_identity()
            ));
            p.config.open_state = OpenState::Controlled(true);
            apply(
                view,
                window,
                cx,
                vec![Op::SetChoicePicker(id(0), Box::new(p.clone()))],
            );
            assert!(view.pickers[&id(0)].state.is_open());
            assert!(view.focus.borrow().allows(id(2)));
            assert!(view.focus.borrow().allows(id(4)));
            assert_eq!(focus, view.editors[&id(2)].focus_handle(cx));
            assert_eq!(view.editors[&id(2)].snapshot(window, cx).text, "kept query");
            assert!(matches!(
                view.editors
                    .get_mut(&id(2))
                    .unwrap()
                    .command(&EditorCommand::Focus, window, cx),
                EditorResult::Applied(_)
            ));
            assert!(focus.is_focused(window));
            p.config.open_state = OpenState::Controlled(false);
            apply(
                view,
                window,
                cx,
                vec![Op::SetChoicePicker(id(0), Box::new(p.clone()))],
            );
            assert!(
                !focus.is_focused(window),
                "closing blurs the retained editor before layout"
            );
            p.config.open_state = OpenState::Managed(true);
            apply(
                view,
                window,
                cx,
                vec![Op::SetChoicePicker(id(0), Box::new(p))],
            );
            assert!(
                !view.pickers[&id(0)].state.is_open(),
                "managed initial preference is mount-only"
            );
            assert_eq!(focus, view.editors[&id(2)].focus_handle(cx));
            assert!(matches!(
                view.editors.get_mut(&id(2)).unwrap().command(
                    &EditorCommand::ReadSnapshot,
                    window,
                    cx
                ),
                EditorResult::Applied(_)
            ));
            apply(
                view,
                window,
                cx,
                vec![
                    Op::SetRoot(None),
                    Op::Remove(id(4)),
                    Op::Remove(id(3)),
                    Op::Remove(id(2)),
                    Op::Remove(id(1)),
                    Op::Remove(id(0)),
                ],
            );
            assert!(view.pickers.is_empty());
            assert!(view.editors.is_empty());
            assert_eq!(session.borrow().tree(wid).unwrap().retained_bytes(), 0);
        })
    });
    let signals: Vec<_> = transport
        .mailbox
        .lock()
        .unwrap()
        .drain(256)
        .into_iter()
        .filter_map(|e| {
            if let gpuio_protocol::v1::Event::ChoicePickerEvent(_, _, _, _, e) = e {
                Some(e)
            } else {
                None
            }
        })
        .collect();
    assert!(matches!(
        &signals[0],
        PickerEvent::Visibility(Visibility::Snapshot(false))
    ));
    assert!(matches!(&signals[1], PickerEvent::QueryChanged(query)
        if query.node == id(2) && query.snapshot.text == "kept query" && !query.snapshot.focused));
    let visibility: Vec<_> = signals
        .into_iter()
        .filter(|e| !matches!(e, PickerEvent::QueryChanged(_)))
        .collect();
    assert_eq!(
        visibility,
        vec![
            PickerEvent::Visibility(Visibility::Snapshot(false)),
            PickerEvent::Visibility(Visibility::Changed(true, VisibilityReason::Application)),
            PickerEvent::Visibility(Visibility::Changed(false, VisibilityReason::Application))
        ]
    );
}
#[test]
fn picker_without_slots_observes_mount_once_and_rebinding_once() {
    let mut app = TestAppContext::single();
    let (_reader, writer) = UnixStream::pair().unwrap();
    let transport = Arc::new(Transport::new(writer.as_raw_fd()).unwrap());
    let session = Rc::new(RefCell::new(Session::default()));
    let wid = WindowId::from_parts(0, 1).unwrap();
    session.borrow_mut().hello(VERSION, CAPABILITIES).unwrap();
    session
        .borrow_mut()
        .open(1, wid, "Picker observer", 400., 200.)
        .unwrap();
    let (entity, cx) =
        app.add_window_view(|_, _| View::new(wid, session.clone(), transport.clone()));
    cx.update(|window, cx| {
        entity.update(cx, |view, cx| {
            apply(
                view,
                window,
                cx,
                vec![
                    Op::Create(id(0), Kind::ChoicePicker, "".into(), Some(handler(0))),
                    Op::SetChoicePicker(id(0), Box::new(config(false, OpenState::Managed(false)))),
                    Op::SetRoot(Some(id(0))),
                ],
            );
            for _ in 0..200 {
                view.sync_choice_pickers(&[], window, cx);
            }
            apply(view, window, cx, vec![Op::Bind(id(0), Some(handler(1)))]);
            view.sync_choice_pickers(&[], window, cx);
        })
    });
    let signals: Vec<_> = transport
        .mailbox
        .lock()
        .unwrap()
        .drain(256)
        .into_iter()
        .filter_map(|e| {
            if let gpuio_protocol::v1::Event::ChoicePickerEvent(_, _, handler, _, e) = e {
                Some((handler, e))
            } else {
                None
            }
        })
        .collect();
    assert_eq!(
        signals,
        vec![
            (
                handler(0),
                PickerEvent::Visibility(Visibility::Snapshot(false))
            ),
            (
                handler(1),
                PickerEvent::Visibility(Visibility::Snapshot(false))
            )
        ]
    );
}

#[test]
fn nested_picker_visibility_follows_parent_order_instead_of_allocation_order() {
    let mut app = TestAppContext::single();
    let (_reader, writer) = UnixStream::pair().unwrap();
    let transport = Arc::new(Transport::new(writer.as_raw_fd()).unwrap());
    let session = Rc::new(RefCell::new(Session::default()));
    let wid = WindowId::from_parts(0, 1).unwrap();
    session.borrow_mut().hello(VERSION, CAPABILITIES).unwrap();
    session
        .borrow_mut()
        .open(1, wid, "Nested picker", 400., 200.)
        .unwrap();
    let (entity, cx) =
        app.add_window_view(|_, _| View::new(wid, session.clone(), transport.clone()));
    cx.update(|window, cx| {
        entity.update(cx, |view, cx| {
            let mut parent = config(false, OpenState::Controlled(false));
            parent.slots = vec![Slot::Footer];
            apply(
                view,
                window,
                cx,
                vec![
                    Op::Create(id(0), Kind::ChoicePicker, "".into(), Some(handler(0))),
                    Op::SetChoicePicker(
                        id(0),
                        Box::new(config(false, OpenState::Controlled(true))),
                    ),
                    Op::Create(id(1), Kind::ChoicePicker, "".into(), Some(handler(1))),
                    Op::SetChoicePicker(id(1), Box::new(parent.clone())),
                    Op::Create(id(2), Kind::Container, "".into(), None),
                    Op::Splice(id(2), 0, 0, vec![id(0)]),
                    Op::Splice(id(1), 0, 0, vec![id(2)]),
                    Op::SetRoot(Some(id(1))),
                ],
            );
            assert!(!view.pickers[&id(0)].state.is_open());
            assert!(!view.focus.borrow().allows(id(0)));
            parent.config.open_state = OpenState::Controlled(true);
            apply(
                view,
                window,
                cx,
                vec![Op::SetChoicePicker(id(1), Box::new(parent.clone()))],
            );
            assert!(view.pickers[&id(0)].state.is_open());
            assert!(view.focus.borrow().allows(id(0)));
            // Another feature's visibility gate is independent of picker slots.
            view.focus
                .borrow_mut()
                .set_query_hidden([id(0)].into_iter().collect());
            view.sync_choice_pickers(&[], window, cx);
            view.focus
                .borrow_mut()
                .replace_picker_hidden(&[id(2)], &[id(2)]);
            view.focus.borrow_mut().replace_picker_hidden(&[id(2)], &[]);
            assert!(!view.focus.borrow().allows(id(0)));
            view.focus.borrow_mut().set_query_hidden(Default::default());
            view.sync_choice_pickers(&[], window, cx);
            assert!(view.pickers[&id(0)].state.is_open());
        })
    });
    let signals: Vec<_> = transport
        .mailbox
        .lock()
        .unwrap()
        .drain(256)
        .into_iter()
        .filter_map(|e| {
            if let gpuio_protocol::v1::Event::ChoicePickerEvent(_, node, _, _, event) = e {
                Some((node, event))
            } else {
                None
            }
        })
        .collect();
    assert_eq!(
        signals[0],
        (id(1), PickerEvent::Visibility(Visibility::Snapshot(false)))
    );
    assert_eq!(
        signals[1],
        (id(0), PickerEvent::Visibility(Visibility::Snapshot(false)))
    );
    assert_eq!(
        signals[2],
        (
            id(1),
            PickerEvent::Visibility(Visibility::Changed(true, VisibilityReason::Application))
        )
    );
    assert_eq!(
        signals[3],
        (
            id(0),
            PickerEvent::Visibility(Visibility::Changed(true, VisibilityReason::Application))
        )
    );
}

#[test]
fn picker_renders_virtual_rows_and_routes_keyboard_toggles_with_native_query_snapshot() {
    let mut app = TestAppContext::single();
    app.update(gpui_base::init);
    let (_reader, writer) = UnixStream::pair().unwrap();
    let transport = Arc::new(Transport::new(writer.as_raw_fd()).unwrap());
    let session = Rc::new(RefCell::new(Session::default()));
    let wid = WindowId::from_parts(0, 1).unwrap();
    session.borrow_mut().hello(VERSION, CAPABILITIES).unwrap();
    session
        .borrow_mut()
        .open(1, wid, "Rendered picker", 600., 500.)
        .unwrap();
    let (entity, cx) =
        app.add_window_view(|_, _| View::new(wid, session.clone(), transport.clone()));
    cx.simulate_a11y_active(true);
    cx.run_until_parked();
    cx.update(|window, _| window.activate_window());
    let mut p = config(true, OpenState::Managed(false));
    p.config.options = Collection::Grouped(vec![Group {
        id: "section".into(),
        label: "Section".into(),
        items: (0..4096)
            .map(|i| Item {
                id: format!("i{i}"),
                label: format!("Choice {i}"),
                disabled: i == 1,
            })
            .collect(),
    }]);
    p.config.selected = Selection::Multiple(vec![]);
    p.slots = vec![
        Slot::Query,
        Slot::Footer,
        Slot::Option("i0".into(), Checkmark::Custom),
    ];
    p.max_height = 160.;
    cx.update(|window, cx| {
        entity.update(cx, |view, cx| {
            apply(
                view,
                window,
                cx,
                vec![
                    Op::Create(id(0), Kind::Container, "".into(), None),
                    Op::SetStyle(id(0), vec![Style::Padding(20.)]),
                    Op::Create(id(1), Kind::ChoicePicker, "".into(), Some(handler(0))),
                    Op::SetChoicePicker(id(1), Box::new(p.clone())),
                    Op::SetStyle(
                        id(1),
                        vec![
                            Style::Width(Length::Px(240.)),
                            Style::Height(Length::Px(40.)),
                        ],
                    ),
                    Op::Create(id(2), Kind::Container, "".into(), None),
                    Op::Create(id(3), Kind::Input, "".into(), Some(handler(1))),
                    Op::SetEditor(
                        id(3),
                        EditorConfig {
                            label: "Pick".into(),
                            placeholder: "Find".into(),
                            read_only: false,
                            disabled: false,
                            submit_on_enter: false,
                            auto_focus: false,
                            min_rows: 1,
                            max_rows: 1,
                        },
                    ),
                    Op::Create(id(4), Kind::Container, "".into(), None),
                    Op::Create(id(5), Kind::Button, "Footer".into(), Some(handler(2))),
                    Op::Create(id(6), Kind::Container, "".into(), None),
                    Op::Create(id(7), Kind::Container, "".into(), None),
                    Op::SetStyle(
                        id(7),
                        vec![
                            Style::Width(Length::Px(100.)),
                            Style::Height(Length::Px(52.)),
                            Style::Background(Color::Rgba(0xee2244ff)),
                        ],
                    ),
                    Op::Splice(id(2), 0, 0, vec![id(3)]),
                    Op::Splice(id(4), 0, 0, vec![id(5)]),
                    Op::Splice(id(6), 0, 0, vec![id(7)]),
                    Op::Splice(id(1), 0, 0, vec![id(2), id(4), id(6)]),
                    Op::Create(id(8), Kind::Button, "After picker".into(), Some(handler(3))),
                    Op::Splice(id(0), 0, 0, vec![id(1), id(8)]),
                    Op::SetRoot(Some(id(0))),
                ],
            );
            cx.notify();
        });
        window.draw(cx).clear(cx);
        entity.update(cx, |view, cx| window.focus(&view.buttons[&id(1)].focus, cx));
    });
    cx.simulate_keystrokes("enter");
    cx.update(|window, cx| {
        window.draw(cx).clear(cx);
        entity.update(cx, |view, cx| {
            assert!(view.pickers[&id(1)].state.is_open());
            assert!(view.editors[&id(3)].focus_handle(cx).is_focused(window));
            let rows = view.pickers[&id(1)].render.as_ref().unwrap();
            assert_eq!(rows.list.projection().len(), 4097);
            assert!(
                view.pickers[&id(1)].popup.get().size.height <= px(160.),
                "max_height bounds the whole popup"
            );
            assert!(!rows.rendered.is_empty());
            assert!(
                rows.rendered.len() < 32,
                "visible-only row rendering: {:?}",
                rows.rendered
            );
        });
        let red = gpui::Background::from(gpui::rgba(0xee2244ff));
        let rich = window
            .painted_quads()
            .into_iter()
            .find(|q| q.background == red)
            .expect("rich accepted option painted");
        assert_eq!(rich.bounds.size.height.0, 52. * window.scale_factor());
    });
    let tree = cx.a11y_tree().unwrap();
    let focused = &tree
        .nodes
        .iter()
        .find(|(id, _)| *id == tree.focus)
        .unwrap()
        .1;
    assert_eq!(focused.role(), gpui::accesskit::Role::ListBoxOption);
    assert_eq!(
        tree.nodes
            .iter()
            .filter(|(_, node)| node.role() == gpui::accesskit::Role::TextInput
                && node.label() == Some("Pick"))
            .count(),
        1,
        "query is mounted once"
    );
    assert_eq!(
        tree.nodes
            .iter()
            .filter(|(_, node)| node.role() == gpui::accesskit::Role::Button
                && node.label() == Some("Footer"))
            .count(),
        1,
        "footer is mounted once"
    );
    transport.mailbox.lock().unwrap().drain(256);
    for draft in ["a", "b", ""] {
        cx.update(|window, cx| {
            entity.update(cx, |view, cx| {
                assert!(matches!(
                    view.editors.get_mut(&id(3)).unwrap().command(
                        &EditorCommand::Replace(
                            draft.into(),
                            EditorSelectionPolicy::End,
                            EditorUndoPolicy::Record,
                            None
                        ),
                        window,
                        cx
                    ),
                    EditorResult::Applied(_)
                ));
            })
        });
    }
    let observations = transport.mailbox.lock().unwrap().drain(256);
    assert!(
        !observations.iter().any(
            |e| matches!(e, gpuio_protocol::v1::Event::EditorEvent(_, node, ..) if *node == id(3))
        ),
        "query has one owner event stream, not duplicate editor callbacks"
    );
    let drafts: Vec<_> = observations
        .into_iter()
        .filter_map(|e| match e {
            gpuio_protocol::v1::Event::ChoicePickerEvent(
                _,
                node,
                _,
                _,
                PickerEvent::QueryChanged(query),
            ) => {
                assert_eq!(node, id(1));
                assert_eq!(query.node, id(3));
                Some(query.snapshot.text)
            }
            _ => None,
        })
        .collect();
    assert_eq!(
        drafts,
        vec!["a", "b", ""],
        "picker query observations do not coalesce"
    );
    cx.simulate_keystrokes("enter enter");
    let selections: Vec<_> = transport
        .mailbox
        .lock()
        .unwrap()
        .drain(256)
        .into_iter()
        .filter_map(|e| {
            if let gpuio_protocol::v1::Event::ChoicePickerEvent(
                _,
                _,
                _,
                _,
                PickerEvent::SelectionRequested(request, query),
            ) = e
            {
                Some((request, query))
            } else {
                None
            }
        })
        .collect();
    assert_eq!(selections.len(), 2);
    for (request, query) in selections {
        assert_eq!(request, Request::Toggle("i0".into()));
        let query = query.expect("live native query attached");
        assert_eq!(query.node, id(3));
        assert_eq!(query.snapshot.text, "");
        assert!(query.snapshot.composition.is_none());
    }
    cx.update(|window, cx| {
        entity.update(cx, |view, cx| {
            assert_eq!(
                view.picker_query_accessibility_owner(id(1), window, cx),
                Some(view.editors[&id(3)].focus_handle(cx))
            );
            view.editors[&id(3)].mark_test_text("Choice 0", window, cx);
            assert!(view.editors[&id(3)].is_composing(cx));
            assert!(
                view.picker_query_accessibility_owner(id(1), window, cx)
                    .is_none(),
                "marked composition keeps query accessibility focus"
            );
            let route = choice::Route {
                window: wid,
                node: id(1),
                handler: handler(0),
                revision: 1,
                session: session.clone(),
                gate: view.focus.clone(),
                transport: transport.clone(),
            };
            assert!(!view.picker_action(
                &route,
                choice_picker_view::Action::Tab(false),
                window,
                cx
            ));
            assert!(view.editors[&id(3)].focus_handle(cx).is_focused(window));
        });
        window.draw(cx).clear(cx);
    });
    let tree = cx.a11y_tree().unwrap();
    let focused = &tree
        .nodes
        .iter()
        .find(|(id, _)| *id == tree.focus)
        .unwrap()
        .1;
    assert_eq!(
        focused.role(),
        gpui::accesskit::Role::TextInput,
        "marked composition owns AX focus"
    );
    assert_eq!(focused.label(), Some("Pick"));
    transport.mailbox.lock().unwrap().drain(256);
    cx.simulate_keystrokes("escape");
    cx.update(|window, cx| {
        entity.update(cx, |view, cx| {
            assert!(
                view.pickers[&id(1)].state.is_open(),
                "first Escape belongs to composition"
            );
            assert!(!view.editors[&id(3)].is_composing(cx));
            assert!(
                view.picker_query_accessibility_owner(id(1), window, cx)
                    .is_some()
            );
            assert!(matches!(
                view.editors.get_mut(&id(3)).unwrap().command(
                    &EditorCommand::Replace(
                        "".into(),
                        EditorSelectionPolicy::End,
                        EditorUndoPolicy::Record,
                        None
                    ),
                    window,
                    cx
                ),
                EditorResult::Applied(_)
            ));
            let stale_route = choice::Route {
                window: wid,
                node: id(1),
                handler: handler(0),
                revision: 1,
                session: session.clone(),
                gate: view.focus.clone(),
                transport: transport.clone(),
            };
            apply(view, window, cx, vec![Op::Bind(id(1), Some(handler(9)))]);
            assert!(
                !view.picker_action(
                    &stale_route,
                    choice_picker_view::Action::Tab(false),
                    window,
                    cx
                ),
                "retired observer's Tab cannot move focus"
            );
            assert!(view.editors[&id(3)].focus_handle(cx).is_focused(window));
            cx.notify();
        });
        window.draw(cx).clear(cx);
    });
    // Clear sits between trigger and query even though popup children paint later.
    cx.simulate_keystrokes("shift-tab");
    cx.update(|window, cx| {
        entity.read_with(cx, |view, cx| {
            assert!(
                view.pickers[&id(1)].clear_focus.is_focused(window),
                "clear={}, trigger={}, query={}, footer={}, canclear={}",
                view.pickers[&id(1)].clear_focus.is_focused(window),
                view.buttons[&id(1)].focus.is_focused(window),
                view.editors[&id(3)].focus_handle(cx).is_focused(window),
                view.buttons[&id(5)].focus.is_focused(window),
                view.focus
                    .borrow()
                    .can_focus(&view.pickers[&id(1)].clear_focus, window)
            );
        })
    });
    transport.mailbox.lock().unwrap().drain(256);
    cx.simulate_keystrokes("enter");
    let clear_events = transport.mailbox.lock().unwrap().drain(256);
    let clear_queries: Vec<_> = clear_events
        .into_iter()
        .filter_map(|event| match event {
            gpuio_protocol::v1::Event::ChoicePickerEvent(
                _,
                _,
                _,
                _,
                PickerEvent::SelectionRequested(Request::Clear, query),
            ) => Some(query),
            _ => None,
        })
        .collect();
    assert_eq!(clear_queries.len(), 1);
    assert_eq!(clear_queries[0].as_ref().unwrap().node, id(3));
    cx.update(|window, cx| {
        entity.read_with(cx, |view, cx| {
            assert!(view.pickers[&id(1)].state.is_open());
            assert!(view.pickers[&id(1)].clear_focus.is_focused(window));
            assert!(
                view.picker_query_accessibility_owner(id(1), window, cx)
                    .is_none()
            );
        })
    });
    cx.simulate_keystrokes("tab");
    cx.update(|window, cx| {
        entity.read_with(cx, |view, cx| {
            assert!(view.editors[&id(3)].focus_handle(cx).is_focused(window));
        })
    });
    // Footer controls remain independent keyboard targets inside the popup.
    cx.simulate_keystrokes("tab");
    cx.update(|window, cx| {
        entity.update(cx, |view, _| {
            assert!(view.pickers[&id(1)].state.is_open());
            assert!(
                view.buttons[&id(5)].focus.is_focused(window),
                "Tab enters footer"
            );
        });
    });
    transport.mailbox.lock().unwrap().drain(256);
    cx.simulate_keystrokes("enter");
    cx.update(|window, cx| {
        window.dispatch_event(
            gpui::PlatformInput::KeyUp(gpui::KeyUpEvent {
                keystroke: gpui::Keystroke::parse("enter").unwrap(),
            }),
            cx,
        );
    });
    let footer_events = transport.mailbox.lock().unwrap().drain(256);
    assert!(
        !footer_events.iter().any(|e| matches!(
            e,
            gpuio_protocol::v1::Event::ChoicePickerEvent(
                _,
                _,
                _,
                _,
                PickerEvent::SelectionRequested(..)
            )
        )),
        "footer Enter must not select a highlighted option: {footer_events:?}"
    );
    assert!(
        footer_events.iter().any(|e| matches!(e,
            gpuio_protocol::v1::Event::Press(_, node, h, _) if *node == id(5) && *h == handler(2)
        )),
        "footer Enter activates its own callback: {footer_events:?}"
    );
    cx.simulate_keystrokes("tab");
    cx.update(|window, cx| {
        window.draw(cx).clear(cx);
        entity.update(cx, |view, cx| {
            assert!(
                !view.pickers[&id(1)].state.is_open(),
                "Tab out closes popup; focus={:?}, outside={}, trigger={}, footer={}",
                view.focus.borrow().focused_node(window, cx),
                view.buttons[&id(8)].focus.is_focused(window),
                view.buttons[&id(1)].focus.is_focused(window),
                view.buttons[&id(5)].focus.is_focused(window)
            );
            assert!(
                view.buttons[&id(8)].focus.is_focused(window),
                "leaving popup retains destination focus"
            );
        });
    });
    cx.simulate_keystrokes("shift-tab");
    cx.update(|window, cx| {
        entity.read_with(cx, |view, _| {
            assert!(view.pickers[&id(1)].clear_focus.is_focused(window));
        })
    });
    cx.simulate_keystrokes("shift-tab enter");
    cx.update(|window, cx| {
        window.draw(cx).clear(cx);
        entity.update(cx, |view, cx| {
            assert!(view.pickers[&id(1)].state.is_open());
            assert!(view.editors[&id(3)].focus_handle(cx).is_focused(window));
        });
    });
    cx.update(|window, cx| {
        entity.update(cx, |view, cx| {
            assert!(view.pickers[&id(1)].state.is_open());
            assert_eq!(
                view.pickers[&id(1)].state.config().selected,
                Selection::Multiple(vec![]),
                "no optimistic selection"
            );
            assert!(matches!(
                view.editors.get_mut(&id(3)).unwrap().command(
                    &EditorCommand::Replace(
                        "Choice 4000".into(),
                        EditorSelectionPolicy::End,
                        EditorUndoPolicy::Record,
                        None
                    ),
                    window,
                    cx
                ),
                EditorResult::Applied(_)
            ));
            cx.notify();
        })
    });
    cx.update(|window, cx| {
        window.draw(cx).clear(cx);
        entity.update(cx, |view, _| {
            let rows = view.pickers[&id(1)].render.as_ref().unwrap();
            assert_eq!(rows.list.projection().item_count(), 1);
            assert!(rows.list.projection().item_row("i4000").is_some());
        });
    });
    cx.simulate_keystrokes("escape");
    cx.update(|window, cx| {
        entity.update(cx, |view, cx| {
            assert!(!view.pickers[&id(1)].state.is_open());
            assert!(view.buttons[&id(1)].focus.is_focused(window));
            assert_eq!(
                view.editors[&id(3)].snapshot(window, cx).text,
                "Choice 4000"
            );
        })
    });
    cx.update(|window, cx| {
        // Retire the painted popup, then update the closed owner without drawing.
        window.draw(cx).clear(cx);
        entity.update(cx, |view, cx| {
            let old_config = Arc::downgrade(view.pickers[&id(1)].state.config());
            let previous_bytes = view.session.borrow().tree(wid).unwrap().retained_bytes();
            p.config.options = Collection::Flat(vec![Item {
                id: "i0".into(),
                label: "Only remaining choice".into(),
                disabled: false,
            }]);
            apply(
                view,
                window,
                cx,
                vec![Op::SetChoicePicker(id(1), Box::new(p.clone()))],
            );
            assert!(
                old_config.upgrade().is_none(),
                "closed projection releases obsolete catalog without redraw"
            );
            assert!(view.session.borrow().tree(wid).unwrap().retained_bytes() < previous_bytes);
            assert_eq!(
                view.pickers[&id(1)]
                    .render
                    .as_ref()
                    .unwrap()
                    .list
                    .projection()
                    .item_count(),
                0
            );
            let mut remove = vec![Op::SetRoot(None)];
            remove.extend((0..=8).rev().map(|slot| Op::Remove(id(slot))));
            apply(view, window, cx, remove);
            assert!(view.pickers.is_empty());
            assert!(view.editors.is_empty());
            assert_eq!(view.session.borrow().tree(wid).unwrap().retained_bytes(), 0);
        });
    });
}

#[test]
fn nested_picker_keyboard_closes_only_the_nearest_owner() {
    let mut app = TestAppContext::single();
    app.update(gpui_base::init);
    let (_reader, writer) = UnixStream::pair().unwrap();
    let transport = Arc::new(Transport::new(writer.as_raw_fd()).unwrap());
    let session = Rc::new(RefCell::new(Session::default()));
    let wid = WindowId::from_parts(0, 1).unwrap();
    session.borrow_mut().hello(VERSION, CAPABILITIES).unwrap();
    session
        .borrow_mut()
        .open(1, wid, "Nested picker keyboard", 600., 500.)
        .unwrap();
    let (entity, cx) =
        app.add_window_view(|_, _| View::new(wid, session.clone(), transport.clone()));
    cx.update(|window, _| window.activate_window());
    cx.update(|window, cx| {
        entity.update(cx, |view, cx| {
            let mut parent = config(false, OpenState::Controlled(true));
            parent.slots = vec![Slot::Footer];
            apply(
                view,
                window,
                cx,
                vec![
                    Op::Create(id(0), Kind::ChoicePicker, "".into(), Some(handler(0))),
                    Op::SetChoicePicker(id(0), Box::new(parent)),
                    Op::SetStyle(
                        id(0),
                        vec![
                            Style::Width(Length::Px(240.)),
                            Style::Height(Length::Px(40.)),
                        ],
                    ),
                    Op::Create(id(1), Kind::Container, "".into(), None),
                    Op::Create(id(2), Kind::ChoicePicker, "".into(), Some(handler(1))),
                    Op::SetChoicePicker(id(2), Box::new(config(false, OpenState::Managed(false)))),
                    Op::SetStyle(
                        id(2),
                        vec![
                            Style::Width(Length::Px(200.)),
                            Style::Height(Length::Px(40.)),
                        ],
                    ),
                    Op::Splice(id(1), 0, 0, vec![id(2)]),
                    Op::Splice(id(0), 0, 0, vec![id(1)]),
                    Op::SetRoot(Some(id(0))),
                ],
            );
            cx.notify();
        });
        window.draw(cx).clear(cx);
        entity.update(cx, |view, cx| window.focus(&view.buttons[&id(2)].focus, cx));
    });
    transport.mailbox.lock().unwrap().drain(256);
    cx.simulate_keystrokes("enter");
    cx.update(|window, cx| {
        window.draw(cx).clear(cx);
        entity.update(cx, |view, _| assert!(view.pickers[&id(2)].state.is_open()));
    });
    cx.simulate_keystrokes("escape");
    cx.update(|window, cx| {
        window.draw(cx).clear(cx);
        entity.update(cx, |view, _| {
            assert!(view.pickers[&id(0)].state.is_open());
            assert!(!view.pickers[&id(2)].state.is_open());
            assert!(view.buttons[&id(2)].focus.is_focused(window));
        });
    });
    let requests: Vec<_> = transport
        .mailbox
        .lock()
        .unwrap()
        .drain(256)
        .into_iter()
        .filter_map(|event| {
            if let gpuio_protocol::v1::Event::ChoicePickerEvent(
                _,
                node,
                _,
                _,
                PickerEvent::OpenRequested(open, reason),
            ) = event
            {
                Some((node, open, reason))
            } else {
                None
            }
        })
        .collect();
    assert_eq!(
        requests,
        vec![
            (id(2), true, OpenReason::Keyboard),
            (id(2), false, OpenReason::Escape)
        ]
    );
}

#[test]
fn picker_clipping_closes_once_and_restores_only_controlled_visibility() {
    let mut app = TestAppContext::single();
    app.update(gpui_base::init);
    let (_reader, writer) = UnixStream::pair().unwrap();
    let transport = Arc::new(Transport::new(writer.as_raw_fd()).unwrap());
    let session = Rc::new(RefCell::new(Session::default()));
    let wid = WindowId::from_parts(0, 1).unwrap();
    session.borrow_mut().hello(VERSION, CAPABILITIES).unwrap();
    session
        .borrow_mut()
        .open(1, wid, "Clipped picker", 400., 300.)
        .unwrap();
    let (entity, cx) =
        app.add_window_view(|_, _| View::new(wid, session.clone(), transport.clone()));
    let position = |top| {
        vec![Style::Fields(vec![
            Field::Position(1),
            Field::Top(Length::Px(top)),
            Field::Left(Length::Px(10.)),
            Field::Width(Length::Px(200.)),
            Field::Height(Length::Px(40.)),
        ])]
    };
    let observations = || {
        transport
            .mailbox
            .lock()
            .unwrap()
            .drain(256)
            .into_iter()
            .filter_map(|e| match e {
                gpuio_protocol::v1::Event::ChoicePickerEvent(
                    _,
                    _,
                    _,
                    _,
                    PickerEvent::Visibility(v),
                ) => Some(v),
                _ => None,
            })
            .collect::<Vec<_>>()
    };
    let mut p = config(false, OpenState::Managed(true));
    cx.update(|window, cx| {
        entity.update(cx, |view, cx| {
            apply(
                view,
                window,
                cx,
                vec![
                    Op::Create(id(0), Kind::Container, "".into(), None),
                    Op::SetStyle(
                        id(0),
                        vec![Style::Fields(vec![
                            Field::Width(Length::Px(240.)),
                            Field::Height(Length::Px(100.)),
                            Field::OverflowX(2),
                            Field::OverflowY(2),
                        ])],
                    ),
                    Op::Create(id(1), Kind::ChoicePicker, "".into(), Some(handler(0))),
                    Op::SetChoicePicker(id(1), Box::new(p.clone())),
                    Op::SetStyle(id(1), position(10.)),
                    Op::Splice(id(0), 0, 0, vec![id(1)]),
                    Op::SetRoot(Some(id(0))),
                ],
            );
            cx.notify();
        });
        window.draw(cx).clear(cx);
    });
    assert_eq!(observations(), vec![Visibility::Snapshot(true)]);
    let move_to = |top, cx: &mut gpui::VisualTestContext| {
        cx.update(|window, cx| {
            entity.update(cx, |view, cx| {
                apply(view, window, cx, vec![Op::SetStyle(id(1), position(top))]);
                cx.notify();
            });
            window.draw(cx).clear(cx);
        });
    };
    move_to(120., cx);
    entity.read_with(cx, |view, _| assert!(!view.pickers[&id(1)].state.is_open()));
    assert_eq!(
        observations(),
        vec![Visibility::Changed(false, VisibilityReason::Unavailable)]
    );
    cx.update(|window, cx| window.draw(cx).clear(cx));
    assert!(
        observations().is_empty(),
        "idle paint does not repeat closure"
    );
    move_to(10., cx);
    entity.read_with(cx, |view, _| assert!(!view.pickers[&id(1)].state.is_open()));
    assert!(
        observations().is_empty(),
        "managed initial state is not replayed"
    );
    p.config.open_state = OpenState::Controlled(true);
    cx.update(|window, cx| {
        entity.update(cx, |view, cx| {
            apply(
                view,
                window,
                cx,
                vec![Op::SetChoicePicker(id(1), Box::new(p))],
            );
            cx.notify();
        });
        window.draw(cx).clear(cx);
    });
    assert_eq!(
        observations(),
        vec![Visibility::Changed(true, VisibilityReason::Application)]
    );
    move_to(120., cx);
    assert_eq!(
        observations(),
        vec![Visibility::Changed(false, VisibilityReason::Unavailable)]
    );
    move_to(80., cx);
    entity.read_with(cx, |view, _| assert!(view.pickers[&id(1)].state.is_open()));
    assert_eq!(
        observations(),
        vec![Visibility::Changed(true, VisibilityReason::Application)],
        "a partially visible trigger restores controlled visibility"
    );
    cx.update(|window, cx| window.draw(cx).clear(cx));
    assert!(observations().is_empty());

    // Scrolling changes native geometry without an accepted tree revision.
    move_to(10., cx);
    cx.update(|window, cx| {
        entity.update(cx, |view, cx| {
            apply(
                view,
                window,
                cx,
                vec![
                    Op::SetStyle(
                        id(0),
                        vec![Style::Fields(vec![
                            Field::Width(Length::Px(240.)),
                            Field::Height(Length::Px(100.)),
                            Field::OverflowX(2),
                            Field::OverflowY(3),
                        ])],
                    ),
                    Op::Create(id(2), Kind::Container, "".into(), None),
                    Op::SetStyle(
                        id(2),
                        vec![Style::Fields(vec![
                            Field::Height(Length::Px(500.)),
                            Field::Shrink(0.),
                        ])],
                    ),
                    Op::Splice(id(0), 1, 0, vec![id(2)]),
                ],
            );
            cx.notify();
        });
        window.draw(cx).clear(cx);
    });
    assert!(observations().is_empty());
    let revision = session.borrow().tree(wid).unwrap().revision();
    for (offset, open) in [(-150., false), (0., true)] {
        cx.update(|window, cx| {
            entity.update(cx, |view, cx| {
                let scroll = &view.scrolls[&id(0)].handle;
                assert!(scroll.max_offset().y >= px(150.));
                scroll.set_offset(gpui::point(px(0.), px(offset)));
                cx.notify();
            });
            window.draw(cx).clear(cx);
        });
        assert_eq!(session.borrow().tree(wid).unwrap().revision(), revision);
        entity.read_with(cx, |view, _| {
            assert_eq!(view.pickers[&id(1)].state.is_open(), open)
        });
        assert_eq!(
            observations(),
            vec![Visibility::Changed(
                open,
                if open {
                    VisibilityReason::Application
                } else {
                    VisibilityReason::Unavailable
                }
            )]
        );
    }
}

#[test]
fn picker_clear_has_keyboard_focus_and_retains_controlled_selection_until_commit() {
    let mut app = TestAppContext::single();
    app.update(gpui_base::init);
    let (_reader, writer) = UnixStream::pair().unwrap();
    let transport = Arc::new(Transport::new(writer.as_raw_fd()).unwrap());
    let session = Rc::new(RefCell::new(Session::default()));
    let wid = WindowId::from_parts(0, 1).unwrap();
    session.borrow_mut().hello(VERSION, CAPABILITIES).unwrap();
    session
        .borrow_mut()
        .open(1, wid, "Keyboard clear", 400., 300.)
        .unwrap();
    let (entity, cx) =
        app.add_window_view(|_, _| View::new(wid, session.clone(), transport.clone()));
    let mut p = config(false, OpenState::Managed(false));
    p.config.options = Collection::Flat(vec![Item {
        id: "one".into(),
        label: "One".into(),
        disabled: false,
    }]);
    p.config.selected = Selection::Single(Some("one".into()));
    cx.update(|window, cx| {
        entity.update(cx, |view, cx| {
            apply(
                view,
                window,
                cx,
                vec![
                    Op::Create(id(0), Kind::Container, "".into(), None),
                    Op::Create(id(1), Kind::ChoicePicker, "".into(), Some(handler(0))),
                    Op::SetChoicePicker(id(1), Box::new(p.clone())),
                    Op::SetStyle(
                        id(1),
                        vec![
                            Style::Width(Length::Px(240.)),
                            Style::Height(Length::Px(40.)),
                        ],
                    ),
                    Op::Create(id(2), Kind::Button, "After".into(), Some(handler(1))),
                    // A clipped native control selects managed focus traversal.
                    Op::Create(id(3), Kind::Button, "Offscreen".into(), Some(handler(2))),
                    Op::SetStyle(
                        id(3),
                        vec![Style::Fields(vec![
                            Field::Position(1),
                            Field::Top(Length::Px(2000.)),
                            Field::Height(Length::Px(40.)),
                        ])],
                    ),
                    Op::Splice(id(0), 0, 0, vec![id(1), id(2), id(3)]),
                    Op::SetRoot(Some(id(0))),
                ],
            );
            cx.notify();
        });
        window.activate_window();
        window.draw(cx).clear(cx);
        entity.update(cx, |view, cx| window.focus(&view.buttons[&id(1)].focus, cx));
    });
    transport.mailbox.lock().unwrap().drain(256);
    cx.simulate_keystrokes("tab");
    cx.update(|window, cx| {
        window.draw(cx).clear(cx);
        entity.read_with(cx, |view, _| {
            assert!(
                view.pickers[&id(1)].clear_focus.is_focused(window),
                "Tab enters Clear"
            )
        });
    });
    cx.simulate_keystrokes("enter space");
    let events = transport.mailbox.lock().unwrap().drain(256);
    let requests: Vec<_> = events
        .into_iter()
        .filter_map(|event| match event {
            gpuio_protocol::v1::Event::ChoicePickerEvent(_, _, _, _, event) => Some(event),
            _ => None,
        })
        .collect();
    assert_eq!(
        requests,
        vec![PickerEvent::SelectionRequested(Request::Clear, None); 2]
    );
    cx.update(|window, cx| {
        entity.read_with(cx, |view, _| {
            assert!(view.pickers[&id(1)].clear_focus.is_focused(window));
            assert!(!view.pickers[&id(1)].state.is_open());
            assert_eq!(
                view.pickers[&id(1)].state.config().selected,
                Selection::Single(Some("one".into()))
            );
        })
    });
    // A model update removes Clear while it owns focus: return to its trigger.
    p.config.clearable = false;
    cx.update(|window, cx| {
        entity.update(cx, |view, cx| {
            apply(
                view,
                window,
                cx,
                vec![Op::SetChoicePicker(id(1), Box::new(p.clone()))],
            );
            assert!(view.buttons[&id(1)].focus.is_focused(window));
            cx.notify();
        });
        window.draw(cx).clear(cx);
    });
    cx.simulate_keystrokes("tab");
    cx.update(|window, cx| {
        entity.read_with(cx, |view, _| {
            assert!(view.buttons[&id(2)].focus.is_focused(window))
        })
    });
    p.config.clearable = true;
    p.config.open_state = OpenState::Controlled(true);
    cx.update(|window, cx| {
        entity.update(cx, |view, cx| {
            apply(
                view,
                window,
                cx,
                vec![Op::SetChoicePicker(id(1), Box::new(p.clone()))],
            );
            window.focus(&view.buttons[&id(1)].focus, cx);
            cx.notify();
        });
        window.draw(cx).clear(cx);
    });
    cx.simulate_keystrokes("tab");
    cx.update(|window, cx| {
        entity.read_with(cx, |view, _| {
            assert!(view.pickers[&id(1)].clear_focus.is_focused(window))
        })
    });
    cx.simulate_keystrokes("tab");
    cx.update(|window, cx| {
        entity.read_with(cx, |view, _| {
            assert!(view.buttons[&id(2)].focus.is_focused(window))
        })
    });
    cx.simulate_keystrokes("shift-tab");
    cx.update(|window, cx| {
        entity.read_with(cx, |view, _| {
            assert!(view.pickers[&id(1)].clear_focus.is_focused(window))
        })
    });
    p.config.disabled = true;
    cx.update(|window, cx| {
        entity.update(cx, |view, cx| {
            apply(
                view,
                window,
                cx,
                vec![Op::SetChoicePicker(id(1), Box::new(p))],
            );
            assert!(!view.pickers[&id(1)].clear_focus.is_focused(window));
            cx.notify();
        });
        window.draw(cx).clear(cx);
    });
}

#[test]
fn grouped_picker_accessibility_keeps_visible_options_under_stable_named_groups() {
    use gpui::accesskit::{NodeId as AxId, Role, TreeUpdate};
    fn labeled(tree: &TreeUpdate, label: &str, role: Role) -> AxId {
        let matches: Vec<_> = tree
            .nodes
            .iter()
            .filter(|(_, node)| node.label() == Some(label) && node.role() == role)
            .collect();
        assert_eq!(matches.len(), 1, "{label}: {matches:?}");
        matches[0].0
    }
    fn valid(tree: &TreeUpdate) {
        let mut parents = std::collections::BTreeMap::new();
        for (id, node) in &tree.nodes {
            for child in node.children() {
                assert!(
                    tree.nodes.iter().any(|(id, _)| id == child),
                    "missing child"
                );
                assert!(parents.insert(*child, *id).is_none(), "duplicate parent");
            }
        }
        assert_eq!(parents.len() + 1, tree.nodes.len(), "orphaned node");
    }
    let mut app = TestAppContext::single();
    app.update(gpui_base::init);
    let (_reader, writer) = UnixStream::pair().unwrap();
    let transport = Arc::new(Transport::new(writer.as_raw_fd()).unwrap());
    let session = Rc::new(RefCell::new(Session::default()));
    let wid = WindowId::from_parts(0, 1).unwrap();
    session.borrow_mut().hello(VERSION, CAPABILITIES).unwrap();
    session
        .borrow_mut()
        .open(1, wid, "Grouped picker", 600., 500.)
        .unwrap();
    let (entity, cx) =
        app.add_window_view(|_, _| View::new(wid, session.clone(), transport.clone()));
    let mut p = config(false, OpenState::Controlled(true));
    p.max_height = 160.;
    p.config.options = Collection::Grouped(vec![Group {
        id: "tools".into(),
        label: "Tools".into(),
        items: (0..100)
            .map(|i| Item {
                id: format!("item-{i}"),
                label: format!("Tool {i}"),
                disabled: i == 1,
            })
            .collect(),
    }]);
    cx.update(|window, cx| {
        entity.update(cx, |view, cx| {
            apply(
                view,
                window,
                cx,
                vec![
                    Op::Create(id(0), Kind::Container, "".into(), None),
                    Op::SetStyle(id(0), vec![Style::Padding(20.)]),
                    Op::Create(id(1), Kind::ChoicePicker, "".into(), Some(handler(0))),
                    Op::SetChoicePicker(id(1), Box::new(p.clone())),
                    Op::SetStyle(
                        id(1),
                        vec![
                            Style::Width(Length::Px(240.)),
                            Style::Height(Length::Px(40.)),
                        ],
                    ),
                    Op::Splice(id(0), 0, 0, vec![id(1)]),
                    Op::SetRoot(Some(id(0))),
                ],
            );
            cx.notify();
        });
        window.draw(cx).clear(cx);
    });
    assert!(cx.a11y_tree().is_none());
    cx.simulate_a11y_active(true);
    cx.run_until_parked();
    let mut group_id = None;
    let mut first_option = None;
    for offset in [0, 50, 0, 0] {
        cx.update(|window, cx| {
            entity.update(cx, |view, cx| {
                view.pickers[&id(1)]
                    .render
                    .as_ref()
                    .unwrap()
                    .list
                    .handle()
                    .scroll_to(gpui::ListOffset {
                        item_ix: offset,
                        offset_in_item: px(0.),
                    });
                cx.notify();
            });
            window.draw(cx).clear(cx);
        });
        let tree = cx.a11y_tree().unwrap();
        valid(&tree);
        let group = labeled(&tree, "Tools", Role::Group);
        assert_eq!(*group_id.get_or_insert(group), group);
        let node = |id| &tree.nodes.iter().find(|(key, _)| *key == id).unwrap().1;
        let list = labeled(&tree, "Pick", Role::ListBox);
        assert_eq!(node(list).children(), &[group]);
        let options: Vec<_> = node(group)
            .children()
            .iter()
            .filter(|id| node(**id).role() == Role::ListBoxOption)
            .collect();
        assert!(!options.is_empty() && options.len() < 20);
        for option in options {
            assert_eq!(node(*option).size_of_set(), Some(100));
            assert_eq!(node(*option).description(), None);
            assert_eq!(
                node(*option).supports_action(gpui::accesskit::Action::Click),
                !node(*option).is_disabled()
            );
        }
        if offset == 0 {
            let first = labeled(&tree, "Tool 0", Role::ListBoxOption);
            assert_eq!(*first_option.get_or_insert(first), first);
            assert_eq!(node(first).position_in_set(), Some(1));
            assert!(
                node(group)
                    .children()
                    .iter()
                    .any(|id| node(*id).is_hidden()),
                "decorative header"
            );
        } else {
            assert!(
                node(group)
                    .children()
                    .iter()
                    .all(|id| node(*id).role() == Role::ListBoxOption),
                "header is unmounted"
            );
        }
    }
    // A replacement preserves surviving item identity and updates logical membership.
    if let Collection::Grouped(groups) = &mut p.config.options {
        groups[0].label = "Renamed tools".into();
        groups[0].items.truncate(2);
        groups[0].items.reverse();
    }
    cx.update(|window, cx| {
        entity.update(cx, |view, cx| {
            apply(
                view,
                window,
                cx,
                vec![Op::SetChoicePicker(id(1), Box::new(p.clone()))],
            );
            cx.notify();
        });
        window.draw(cx).clear(cx);
    });
    let tree = cx.a11y_tree().unwrap();
    valid(&tree);
    assert_eq!(
        labeled(&tree, "Renamed tools", Role::Group),
        group_id.unwrap()
    );
    assert_eq!(
        labeled(&tree, "Tool 0", Role::ListBoxOption),
        first_option.unwrap()
    );
    for (_, node) in tree
        .nodes
        .iter()
        .filter(|(_, node)| node.role() == Role::ListBoxOption)
    {
        assert_eq!(node.size_of_set(), Some(2));
    }
    // Multiple groups, including a header-only group, preserve catalog order.
    p.max_height = 400.;
    if let Collection::Grouped(groups) = &mut p.config.options {
        groups.push(Group {
            id: "another".into(),
            label: "Another".into(),
            items: vec![Item {
                id: "other".into(),
                label: "Other".into(),
                disabled: false,
            }],
        });
        groups.push(Group {
            id: "empty".into(),
            label: "Empty".into(),
            items: vec![],
        });
    }
    cx.update(|window, cx| {
        entity.update(cx, |view, cx| {
            apply(
                view,
                window,
                cx,
                vec![Op::SetChoicePicker(id(1), Box::new(p.clone()))],
            );
            cx.notify();
        });
        window.draw(cx).clear(cx);
    });
    let tree = cx.a11y_tree().unwrap();
    valid(&tree);
    let list = labeled(&tree, "Pick", Role::ListBox);
    let children = &tree.nodes.iter().find(|(id, _)| *id == list).unwrap().1;
    assert_eq!(
        children.children(),
        &[
            group_id.unwrap(),
            labeled(&tree, "Another", Role::Group),
            labeled(&tree, "Empty", Role::Group)
        ]
    );
    // Switching collection kinds retires the synthetic groups, keeping flat options direct.
    p.config.options = Collection::Flat(vec![Item {
        id: "item-0".into(),
        label: "Tool 0".into(),
        disabled: false,
    }]);
    cx.update(|window, cx| {
        entity.update(cx, |view, cx| {
            apply(
                view,
                window,
                cx,
                vec![Op::SetChoicePicker(id(1), Box::new(p.clone()))],
            );
            cx.notify();
        });
        window.draw(cx).clear(cx);
    });
    let tree = cx.a11y_tree().unwrap();
    valid(&tree);
    assert!(
        !tree
            .nodes
            .iter()
            .any(|(_, node)| node.role() == Role::Group)
    );
    let list = labeled(&tree, "Pick", Role::ListBox);
    let item = labeled(&tree, "Tool 0", Role::ListBoxOption);
    assert_eq!(Some(item), first_option);
    assert_eq!(
        tree.nodes
            .iter()
            .find(|(id, _)| *id == list)
            .unwrap()
            .1
            .children(),
        &[item]
    );
    cx.simulate_a11y_active(false);
    cx.run_until_parked();
    assert!(cx.a11y_tree().is_none());
    cx.simulate_a11y_active(true);
    cx.run_until_parked();
    cx.update(|window, cx| window.draw(cx).clear(cx));
    valid(&cx.a11y_tree().unwrap());
    cx.update(|window, cx| {
        entity.update(cx, |view, cx| {
            apply(
                view,
                window,
                cx,
                vec![Op::Splice(id(0), 0, 1, vec![]), Op::Remove(id(1))],
            );
            cx.notify();
        });
        window.draw(cx).clear(cx);
    });
    let tree = cx.a11y_tree().unwrap();
    valid(&tree);
    assert!(!tree.nodes.iter().any(|(_, node)| matches!(
        node.role(),
        Role::Group | Role::ListBox | Role::ListBoxOption
    )));
}

#[test]
fn picker_inherited_text_changes_retire_offscreen_heights_and_preserve_scroll_anchor() {
    let mut app = TestAppContext::single();
    app.update(gpui_base::init);
    let (_reader, writer) = UnixStream::pair().unwrap();
    let transport = Arc::new(Transport::new(writer.as_raw_fd()).unwrap());
    let session = Rc::new(RefCell::new(Session::default()));
    let wid = WindowId::from_parts(0, 1).unwrap();
    session.borrow_mut().hello(VERSION, CAPABILITIES).unwrap();
    session
        .borrow_mut()
        .open(1, wid, "Picker layout", 600., 500.)
        .unwrap();
    let (entity, cx) =
        app.add_window_view(|_, _| View::new(wid, session.clone(), transport.clone()));
    let mut p = config(false, OpenState::Controlled(true));
    p.max_height = 160.;
    p.config.options = Collection::Flat(
        (0..100)
            .map(|i| Item {
                id: format!("i{i}"),
                label: format!("Item {i}"),
                disabled: false,
            })
            .collect(),
    );
    let ancestor_style = |height| {
        vec![
            Style::Padding(20.),
            Style::Fields(vec![
                Field::FontSize(12.),
                Field::LineHeight(Length::Px(height)),
            ]),
        ]
    };
    cx.update(|window, cx| {
        entity.update(cx, |view, cx| {
            apply(
                view,
                window,
                cx,
                vec![
                    Op::Create(id(0), Kind::Container, "".into(), None),
                    Op::SetStyle(id(0), ancestor_style(20.)),
                    Op::Create(id(1), Kind::ChoicePicker, "".into(), Some(handler(0))),
                    Op::SetChoicePicker(id(1), Box::new(p.clone())),
                    Op::SetStyle(
                        id(1),
                        vec![
                            Style::Width(Length::Px(240.)),
                            Style::Height(Length::Px(40.)),
                        ],
                    ),
                    Op::Splice(id(0), 0, 0, vec![id(1)]),
                    Op::SetRoot(Some(id(0))),
                ],
            );
            cx.notify();
        });
        window.draw(cx).clear(cx);
    });
    for index in [50, 0] {
        cx.update(|window, cx| {
            entity.update(cx, |view, cx| {
                view.pickers[&id(1)]
                    .render
                    .as_ref()
                    .unwrap()
                    .list
                    .handle()
                    .scroll_to(gpui::ListOffset {
                        item_ix: index,
                        offset_in_item: px(5.),
                    });
                cx.notify();
            });
            window.draw(cx).clear(cx);
        });
    }
    cx.update(|window, cx| {
        entity.update(cx, |view, cx| {
            let list = view.pickers[&id(1)].render.as_ref().unwrap().list.handle();
            assert_eq!(list.bounds_for_item(0).unwrap().size.height, px(28.));
            assert_eq!(
                list.bounds_for_item(50).unwrap().size.height,
                px(28.),
                "old offscreen measurement retained before change"
            );
            apply(
                view,
                window,
                cx,
                vec![Op::SetStyle(id(0), ancestor_style(40.))],
            );
            cx.notify();
        });
        window.draw(cx).clear(cx);
        entity.update(cx, |view, _| {
            let list = view.pickers[&id(1)].render.as_ref().unwrap().list.handle();
            assert_eq!(list.bounds_for_item(0).unwrap().size.height, px(48.));
            assert!(
                list.bounds_for_item(50).is_none(),
                "ancestor font changes invalidate offscreen measurements"
            );
            assert_eq!(list.logical_scroll_top().item_ix, 0);
            assert_eq!(list.logical_scroll_top().offset_in_item, px(5.));
        });
    });
    cx.update(|window, cx| {
        entity.update(cx, |view, cx| {
            view.pickers[&id(1)]
                .render
                .as_ref()
                .unwrap()
                .list
                .handle()
                .scroll_to(gpui::ListOffset {
                    item_ix: 50,
                    offset_in_item: px(5.),
                });
            cx.notify();
        });
        window.draw(cx).clear(cx);
        entity.update(cx, |view, cx| {
            assert_eq!(
                view.pickers[&id(1)]
                    .render
                    .as_ref()
                    .unwrap()
                    .list
                    .handle()
                    .bounds_for_item(50)
                    .unwrap()
                    .size
                    .height,
                px(48.)
            );
            apply(
                view,
                window,
                cx,
                vec![Op::SetStyle(id(0), ancestor_style(20.))],
            );
            cx.notify();
        });
        window.draw(cx).clear(cx);
        entity.update(cx, |view, _| {
            let list = view.pickers[&id(1)].render.as_ref().unwrap().list.handle();
            assert_eq!(list.bounds_for_item(50).unwrap().size.height, px(28.));
            assert_eq!(list.logical_scroll_top().item_ix, 50);
            assert_eq!(list.logical_scroll_top().offset_in_item, px(5.));
        });
    });
    // A popup override masks later ancestor line-height changes. Do not discard
    // cached rows on every draw, or for an inherited value that is not effective.
    p.popup_style = vec![Style::Fields(vec![Field::LineHeight(Length::Px(30.))])];
    cx.update(|window, cx| {
        entity.update(cx, |view, cx| {
            apply(
                view,
                window,
                cx,
                vec![Op::SetChoicePicker(id(1), Box::new(p.clone()))],
            );
            cx.notify();
        });
        window.draw(cx).clear(cx);
        entity.update(cx, |view, cx| {
            let list = view.pickers[&id(1)].render.as_ref().unwrap().list.handle();
            assert_eq!(list.bounds_for_item(50).unwrap().size.height, px(38.));
            list.scroll_to(gpui::ListOffset {
                item_ix: 0,
                offset_in_item: px(5.),
            });
            cx.notify();
        });
        window.draw(cx).clear(cx);
        entity.update(cx, |view, cx| {
            let list = view.pickers[&id(1)].render.as_ref().unwrap().list.handle();
            assert_eq!(list.bounds_for_item(50).unwrap().size.height, px(38.));
            apply(
                view,
                window,
                cx,
                vec![Op::SetStyle(id(0), ancestor_style(80.))],
            );
            cx.notify();
        });
        window.draw(cx).clear(cx);
        window.refresh();
        window.draw(cx).clear(cx);
        entity.update(cx, |view, _| {
            let list = view.pickers[&id(1)].render.as_ref().unwrap().list.handle();
            assert_eq!(list.bounds_for_item(0).unwrap().size.height, px(38.));
            assert_eq!(
                list.bounds_for_item(50).unwrap().size.height,
                px(38.),
                "masked change and idle frames retain offscreen cache"
            );
            assert_eq!(list.logical_scroll_top().offset_in_item, px(5.));
        });
    });
}

#[test]
fn controlled_picker_focus_follows_accepted_gestures_without_stealing_focus() {
    let mut app = TestAppContext::single();
    app.update(gpui_base::init);
    let (_reader, writer) = UnixStream::pair().unwrap();
    let transport = Arc::new(Transport::new(writer.as_raw_fd()).unwrap());
    let session = Rc::new(RefCell::new(Session::default()));
    let wid = WindowId::from_parts(0, 1).unwrap();
    session.borrow_mut().hello(VERSION, CAPABILITIES).unwrap();
    session
        .borrow_mut()
        .open(1, wid, "Controlled focus", 600., 500.)
        .unwrap();
    let (entity, cx) =
        app.add_window_view(|_, _| View::new(wid, session.clone(), transport.clone()));
    let mut p = config(true, OpenState::Controlled(false));
    p.slots = vec![Slot::Query];
    let query_config = |disabled| EditorConfig {
        label: "Pick".into(),
        placeholder: "Find".into(),
        read_only: false,
        disabled,
        submit_on_enter: false,
        auto_focus: false,
        min_rows: 1,
        max_rows: 1,
    };
    cx.update(|window, cx| {
        entity.update(cx, |view, cx| {
            apply(
                view,
                window,
                cx,
                vec![
                    Op::Create(id(0), Kind::Container, "".into(), None),
                    Op::Create(id(1), Kind::ChoicePicker, "".into(), Some(handler(0))),
                    Op::SetChoicePicker(id(1), Box::new(p.clone())),
                    Op::SetStyle(
                        id(1),
                        vec![
                            Style::Width(Length::Px(240.)),
                            Style::Height(Length::Px(40.)),
                        ],
                    ),
                    Op::Create(id(2), Kind::Container, "".into(), None),
                    Op::Create(id(3), Kind::Input, "".into(), Some(handler(1))),
                    Op::SetEditor(
                        id(3),
                        EditorConfig {
                            label: "Pick".into(),
                            placeholder: "Find".into(),
                            read_only: false,
                            disabled: false,
                            submit_on_enter: false,
                            auto_focus: false,
                            min_rows: 1,
                            max_rows: 1,
                        },
                    ),
                    Op::Splice(id(2), 0, 0, vec![id(3)]),
                    Op::Splice(id(1), 0, 0, vec![id(2)]),
                    Op::Create(id(4), Kind::Button, "Elsewhere".into(), Some(handler(2))),
                    Op::Splice(id(0), 0, 0, vec![id(1), id(4)]),
                    Op::SetRoot(Some(id(0))),
                ],
            );
            cx.notify();
        });
        window.draw(cx).clear(cx);
        entity.update(cx, |view, cx| window.focus(&view.buttons[&id(1)].focus, cx));
    });
    let gesture = |view: &mut View, action, window: &mut Window, cx: &mut Context<View>| {
        let route = choice::Route {
            window: wid,
            node: id(1),
            handler: view
                .session
                .borrow()
                .tree(wid)
                .unwrap()
                .get(id(1))
                .unwrap()
                .handler
                .unwrap(),
            revision: view.session.borrow().tree(wid).unwrap().revision(),
            session: view.session.clone(),
            gate: view.focus.clone(),
            transport: view.transport.clone(),
        };
        assert!(view.picker_action(&route, action, window, cx));
    };
    cx.update(|window, cx| {
        entity.update(cx, |view, cx| {
            gesture(
                view,
                super::choice_picker_view::Action::Key("enter".into()),
                window,
                cx,
            );
            assert!(
                !view.pickers[&id(1)].state.is_open(),
                "controlled request waits for commit"
            );
            assert!(view.buttons[&id(1)].focus.is_focused(window));
        })
    });
    cx.run_until_parked();
    p.config.open_state = OpenState::Controlled(true);
    cx.update(|window, cx| {
        entity.update(cx, |view, cx| {
            apply(
                view,
                window,
                cx,
                vec![
                    Op::SetChoicePicker(id(1), Box::new(p.clone())),
                    Op::SetEditor(id(3), query_config(p.config.disabled)),
                ],
            );
            assert!(
                view.editors[&id(3)].focus_handle(cx).is_focused(window),
                "accepted controlled open focuses query"
            );
            cx.notify();
        });
        window.draw(cx).clear(cx);
    });
    cx.update(|window, cx| {
        entity.update(cx, |view, cx| {
            gesture(
                view,
                super::choice_picker_view::Action::Key("escape".into()),
                window,
                cx,
            );
            assert!(
                view.editors[&id(3)].focus_handle(cx).is_focused(window),
                "unaccepted close retains query focus"
            );
        })
    });
    cx.run_until_parked();
    p.config.open_state = OpenState::Controlled(false);
    cx.update(|window, cx| {
        entity.update(cx, |view, cx| {
            apply(
                view,
                window,
                cx,
                vec![
                    Op::SetChoicePicker(id(1), Box::new(p.clone())),
                    Op::SetEditor(id(3), query_config(p.config.disabled)),
                ],
            );
            assert!(
                view.buttons[&id(1)].focus.is_focused(window),
                "accepted Escape restores trigger"
            );
        })
    });
    // Moving away and back cancels a pending handoff; the late response may open
    // the controlled popup, but cannot move keyboard focus to its query.
    cx.update(|window, cx| {
        entity.update(cx, |view, cx| {
            gesture(
                view,
                super::choice_picker_view::Action::Key("enter".into()),
                window,
                cx,
            );
        })
    });
    cx.run_until_parked();
    cx.update(|window, cx| {
        entity.update(cx, |view, cx| window.focus(&view.buttons[&id(4)].focus, cx))
    });
    cx.run_until_parked();
    cx.update(|window, cx| {
        entity.update(cx, |view, cx| window.focus(&view.buttons[&id(1)].focus, cx))
    });
    cx.run_until_parked();
    p.config.open_state = OpenState::Controlled(true);
    cx.update(|window, cx| {
        entity.update(cx, |view, cx| {
            apply(
                view,
                window,
                cx,
                vec![
                    Op::SetChoicePicker(id(1), Box::new(p.clone())),
                    Op::SetEditor(id(3), query_config(p.config.disabled)),
                ],
            );
            assert!(view.pickers[&id(1)].state.is_open());
            assert!(
                view.buttons[&id(1)].focus.is_focused(window),
                "late acceptance must not steal focus"
            );
        })
    });
    // Programmatic updates and repeat synchronization also do not claim focus.
    p.config.open_state = OpenState::Controlled(false);
    cx.update(|window, cx| {
        entity.update(cx, |view, cx| {
            apply(
                view,
                window,
                cx,
                vec![
                    Op::SetChoicePicker(id(1), Box::new(p.clone())),
                    Op::SetEditor(id(3), query_config(p.config.disabled)),
                ],
            );
            window.focus(&view.buttons[&id(4)].focus, cx);
        })
    });
    p.config.open_state = OpenState::Controlled(true);
    cx.update(|window, cx| {
        entity.update(cx, |view, cx| {
            apply(
                view,
                window,
                cx,
                vec![
                    Op::SetChoicePicker(id(1), Box::new(p.clone())),
                    Op::SetEditor(id(3), query_config(p.config.disabled)),
                ],
            );
            view.sync_choice_pickers(&[], window, cx);
            assert!(view.buttons[&id(4)].focus.is_focused(window));
        })
    });
    // A close accepted after focus moved elsewhere must preserve that destination.
    cx.update(|window, cx| {
        entity.update(cx, |view, cx| {
            view.editors
                .get_mut(&id(3))
                .unwrap()
                .command(&EditorCommand::Focus, window, cx);
        })
    });
    cx.run_until_parked();
    cx.update(|window, cx| {
        entity.update(cx, |view, cx| {
            gesture(
                view,
                super::choice_picker_view::Action::Key("escape".into()),
                window,
                cx,
            );
        })
    });
    cx.run_until_parked();
    cx.update(|window, cx| {
        entity.update(cx, |view, cx| window.focus(&view.buttons[&id(4)].focus, cx))
    });
    cx.run_until_parked();
    p.config.open_state = OpenState::Controlled(false);
    cx.update(|window, cx| {
        entity.update(cx, |view, cx| {
            apply(
                view,
                window,
                cx,
                vec![
                    Op::SetChoicePicker(id(1), Box::new(p.clone())),
                    Op::SetEditor(id(3), query_config(p.config.disabled)),
                ],
            );
            assert!(view.buttons[&id(4)].focus.is_focused(window));
        })
    });
    // Accepted no-op updates preserve a pending gesture, but observer retirement,
    // disablement and leaving controlled mode fence it permanently.
    for fence in 0..3 {
        p.config.open_state = OpenState::Controlled(false);
        cx.update(|window, cx| {
            entity.update(cx, |view, cx| {
                apply(
                    view,
                    window,
                    cx,
                    vec![
                        Op::SetChoicePicker(id(1), Box::new(p.clone())),
                        Op::SetEditor(id(3), query_config(p.config.disabled)),
                    ],
                );
                window.focus(&view.buttons[&id(1)].focus, cx);
            })
        });
        cx.run_until_parked();
        cx.update(|window, cx| {
            entity.update(cx, |view, cx| {
                gesture(
                    view,
                    super::choice_picker_view::Action::Key("enter".into()),
                    window,
                    cx,
                );
            })
        });
        cx.run_until_parked();
        cx.update(|window, cx| {
            entity.update(cx, |view, cx| {
                apply(
                    view,
                    window,
                    cx,
                    vec![
                        Op::SetChoicePicker(id(1), Box::new(p.clone())),
                        Op::SetEditor(id(3), query_config(p.config.disabled)),
                    ],
                );
                assert!(view.pickers[&id(1)].pending_focus.is_some());
                match fence {
                    0 => apply(view, window, cx, vec![Op::Bind(id(1), Some(handler(10)))]),
                    1 => {
                        p.config.disabled = true;
                        apply(
                            view,
                            window,
                            cx,
                            vec![
                                Op::SetChoicePicker(id(1), Box::new(p.clone())),
                                Op::SetEditor(id(3), query_config(p.config.disabled)),
                            ],
                        );
                        p.config.disabled = false;
                    }
                    _ => {
                        p.config.open_state = OpenState::Managed(false);
                        apply(
                            view,
                            window,
                            cx,
                            vec![
                                Op::SetChoicePicker(id(1), Box::new(p.clone())),
                                Op::SetEditor(id(3), query_config(p.config.disabled)),
                            ],
                        );
                    }
                }
                assert!(view.pickers[&id(1)].pending_focus.is_none());
                p.config.open_state = OpenState::Controlled(true);
                apply(
                    view,
                    window,
                    cx,
                    vec![
                        Op::SetChoicePicker(id(1), Box::new(p.clone())),
                        Op::SetEditor(id(3), query_config(p.config.disabled)),
                    ],
                );
                assert!(view.pickers[&id(1)].state.is_open());
                assert!(!view.editors[&id(3)].focus_handle(cx).is_focused(window));
            })
        });
    }
}

#[test]
fn picker_viewport_resize_preserves_anchor_and_bounds_without_model_updates() {
    let mut app = TestAppContext::single();
    app.update(gpui_base::init);
    let (_reader, writer) = UnixStream::pair().unwrap();
    let transport = Arc::new(Transport::new(writer.as_raw_fd()).unwrap());
    let session = Rc::new(RefCell::new(Session::default()));
    let wid = WindowId::from_parts(0, 1).unwrap();
    session.borrow_mut().hello(VERSION, CAPABILITIES).unwrap();
    session
        .borrow_mut()
        .open(1, wid, "Picker resize", 600., 500.)
        .unwrap();
    let (entity, cx) =
        app.add_window_view(|_, _| View::new(wid, session.clone(), transport.clone()));
    let mut p = config(false, OpenState::Controlled(true));
    p.popup_width = 400.;
    p.max_height = 280.;
    p.config.options = Collection::Flat(
        (0..100)
            .map(|i| Item {
                id: format!("item-{i}"),
                label: format!(
                    "Workspace {i} with a descriptive label that wraps in a narrow popup"
                ),
                disabled: false,
            })
            .collect(),
    );
    cx.update(|window, cx| {
        entity.update(cx, |view, cx| {
            apply(
                view,
                window,
                cx,
                vec![
                    Op::Create(id(0), Kind::Container, "".into(), None),
                    Op::SetStyle(
                        id(0),
                        vec![
                            Style::Padding(20.),
                            Style::Fields(vec![
                                Field::FontSize(12.),
                                Field::LineHeight(Length::Px(20.)),
                            ]),
                        ],
                    ),
                    Op::Create(id(1), Kind::ChoicePicker, "".into(), Some(handler(0))),
                    Op::SetChoicePicker(id(1), Box::new(p.clone())),
                    Op::SetStyle(
                        id(1),
                        vec![
                            Style::Width(Length::Px(180.)),
                            Style::Height(Length::Px(40.)),
                        ],
                    ),
                    Op::Splice(id(0), 0, 0, vec![id(1)]),
                    Op::SetRoot(Some(id(0))),
                ],
            );
            cx.notify();
        });
        window.draw(cx).clear(cx);
        entity.update(cx, |view, cx| {
            view.pickers[&id(1)]
                .render
                .as_ref()
                .unwrap()
                .list
                .handle()
                .scroll_to(gpui::ListOffset {
                    item_ix: 50,
                    offset_in_item: px(5.),
                });
            cx.notify();
        });
        window.draw(cx).clear(cx);
    });
    let revision = session.borrow().tree(wid).unwrap().revision();
    let initial_height = entity.read_with(cx, |view, _| {
        view.pickers[&id(1)]
            .render
            .as_ref()
            .unwrap()
            .list
            .handle()
            .bounds_for_item(50)
            .unwrap()
            .size
            .height
    });
    for (width, height) in [(240., 360.), (240., 140.), (600., 500.)] {
        cx.simulate_resize(gpui::size(px(width), px(height)));
        cx.run_until_parked();
        cx.update(|window, cx| {
            window.draw(cx).clear(cx);
            entity.update(cx, |view, _| {
                let owner = &view.pickers[&id(1)];
                assert!(owner.state.is_open());
                let bounds = owner.popup.get();
                assert!(
                    bounds.left() >= px(8.) && bounds.top() >= px(8.),
                    "{bounds:?}"
                );
                assert!(
                    bounds.right() <= px(width - 8.) && bounds.bottom() <= px(height - 8.),
                    "{bounds:?}"
                );
                let list = owner.render.as_ref().unwrap().list.handle();
                assert_eq!(list.logical_scroll_top().item_ix, 50);
                assert_eq!(list.logical_scroll_top().offset_in_item, px(5.));
                let row = list.bounds_for_item(50).unwrap();
                if width < 400. {
                    assert!(
                        row.size.height > initial_height,
                        "narrow rows must be remeasured: {row:?}, initial={initial_height:?}"
                    );
                } else {
                    assert_eq!(row.size.height, initial_height);
                }
                assert!(list.viewport_bounds().size.height > px(0.));
                assert_eq!(
                    view.session.borrow().tree(wid).unwrap().revision(),
                    revision
                );
            });
        });
    }
    // Scale changes preserve logical geometry and the same scroll anchor.
    for scale in [2., 1.] {
        cx.simulate_scale_factor_change(scale);
        cx.run_until_parked();
        cx.update(|window, cx| {
            window.draw(cx).clear(cx);
            entity.update(cx, |view, _| {
                let list = view.pickers[&id(1)].render.as_ref().unwrap().list.handle();
                assert_eq!(list.logical_scroll_top().item_ix, 50);
                assert_eq!(list.logical_scroll_top().offset_in_item, px(5.));
                assert_eq!(
                    list.bounds_for_item(50).unwrap().size.height,
                    initial_height
                );
                assert_eq!(
                    view.session.borrow().tree(wid).unwrap().revision(),
                    revision
                );
            });
        });
    }
    // Add the public search/footer shape and constrain the whole popup, not only
    // its virtual list. Keep the same native query lease through another resize.
    p.config.search = Search::Substring;
    p.slots = vec![Slot::Query, Slot::Footer];
    cx.simulate_a11y_active(true);
    cx.run_until_parked();
    cx.update(|window, cx| {
        entity.update(cx, |view, cx| {
            apply(
                view,
                window,
                cx,
                vec![
                    Op::SetChoicePicker(id(1), Box::new(p.clone())),
                    Op::Create(id(2), Kind::Container, "".into(), None),
                    Op::Create(id(3), Kind::Input, "".into(), Some(handler(1))),
                    Op::SetEditor(
                        id(3),
                        EditorConfig {
                            label: "Pick".into(),
                            placeholder: "Find".into(),
                            read_only: false,
                            disabled: false,
                            submit_on_enter: false,
                            auto_focus: false,
                            min_rows: 1,
                            max_rows: 1,
                        },
                    ),
                    Op::Create(id(4), Kind::Container, "".into(), None),
                    Op::Create(id(5), Kind::Button, "Footer".into(), Some(handler(2))),
                    Op::SetStyle(id(5), vec![Style::Height(Length::Px(32.))]),
                    Op::Splice(id(2), 0, 0, vec![id(3)]),
                    Op::Splice(id(4), 0, 0, vec![id(5)]),
                    Op::Splice(id(1), 0, 0, vec![id(2), id(4)]),
                ],
            );
            cx.notify();
        })
    });
    let query_focus = entity.read_with(cx, |view, cx| view.editors[&id(3)].focus_handle(cx));
    let revision = session.borrow().tree(wid).unwrap().revision();
    for (width, height) in [(240., 140.), (600., 500.)] {
        cx.simulate_resize(gpui::size(px(width), px(height)));
        cx.run_until_parked();
        cx.update(|window, cx| {
            window.draw(cx).clear(cx);
            entity.update(cx, |view, cx| {
                let owner = &view.pickers[&id(1)];
                let bounds = owner.popup.get();
                let list = owner.render.as_ref().unwrap().list.handle();
                assert!(bounds.bottom() <= px(height - 8.));
                assert!(bounds.right() <= px(width - 8.));
                assert!(
                    list.viewport_bounds().size.height >= px(1.),
                    "query/footer leave a usable list"
                );
                assert_eq!(list.logical_scroll_top().item_ix, 50);
                assert_eq!(list.logical_scroll_top().offset_in_item, px(5.));
                let query = view.editors[&id(3)].input_bounds(cx);
                assert!(query.top() >= bounds.top() && query.bottom() <= bounds.bottom());
                assert_eq!(view.editors[&id(3)].focus_handle(cx), query_focus);
                assert_eq!(
                    view.session.borrow().tree(wid).unwrap().revision(),
                    revision
                );
            });
        });
        let tree = cx.a11y_tree().unwrap();
        let footer = tree
            .nodes
            .iter()
            .find(|(_, node)| node.label() == Some("Footer"))
            .unwrap()
            .1
            .bounds()
            .unwrap();
        assert!(
            footer.y0 >= 8. && footer.y1 <= f64::from(height - 8.),
            "footer stays reachable: {footer:?}"
        );
    }
    // Settle deferred visibility work, then ensure idle rendering adds no frame
    // callbacks. This does not claim process-level idle CPU measurements.
    cx.run_until_parked();
    for _ in 0..3 {
        cx.update(|window, cx| {
            window.draw(cx).clear(cx);
            assert_eq!(window.simulate_next_frame(cx), 0);
        });
        cx.run_until_parked();
    }
}
