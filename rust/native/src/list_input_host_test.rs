//! Production persistent-list input through TestPlatform. No OS window or
//! assertion about physical IME/VoiceOver qualification.
use super::*;
use gpui::{Entity, TestAppContext, VisualTestContext};
use gpuio_protocol::{
    HandlerId,
    accessibility::{Config as Metadata, Live, OptionItem, Role},
    list::{Axis, Config as ListConfig, IdRun, Order, Row, ScrollPolicy},
    list_input::{Config, Confirmation, Gesture, Navigation, Request},
};
use std::{os::fd::AsRawFd, os::unix::net::UnixStream};
fn n(slot: i64) -> NodeId {
    NodeId::from_parts(slot, 1).unwrap()
}
fn w() -> WindowId {
    WindowId::from_parts(0, 1).unwrap()
}
fn handler(slot: i64) -> HandlerId {
    HandlerId::from_parts(slot, 1).unwrap()
}
fn config() -> Config {
    Config {
        generation: 1,
        cursor: Some(1),
        query: Some(n(1)),
        selection_on_navigation: false,
        disabled: false,
        busy: false,
    }
}
fn size_style(width: f64, height: f64) -> Vec<Style> {
    vec![Style::Fields(vec![
        Field::Width(Length::Px(width)),
        Field::Height(Length::Px(height)),
        Field::Shrink(0.),
    ])]
}
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
fn editor(label: &str) -> EditorConfig {
    EditorConfig {
        label: label.into(),
        placeholder: String::new(),
        read_only: false,
        disabled: false,
        submit_on_enter: false,
        auto_focus: false,
        min_rows: 1,
        max_rows: 1,
    }
}
fn draw(cx: &mut VisualTestContext) {
    cx.run_until_parked();
    cx.update(|w, cx| w.draw(cx).clear(cx));
    cx.run_until_parked();
}
fn apply(owner: &Entity<View>, cx: &mut VisualTestContext, operations: Vec<Op>) {
    cx.update(|window, cx| {
        owner.update(cx, |view, cx| {
            let base = view.session.borrow().tree(w()).unwrap().revision();
            let applied = view
                .session
                .borrow_mut()
                .apply(&Transaction {
                    window: w(),
                    base,
                    revision: base + 1,
                    operations,
                })
                .unwrap();
            view.update_editors(&applied.dirty, window, cx);
            view.list_actions(&applied.lists, window, cx);
            cx.notify();
        })
    });
    draw(cx);
}
fn setup(
    app: &mut TestAppContext,
    query_first: bool,
) -> (
    Entity<View>,
    &mut VisualTestContext,
    Arc<Transport>,
    UnixStream,
) {
    app.update(gpui_base::init);
    let (reader, writer) = UnixStream::pair().unwrap();
    let transport = Arc::new(Transport::new(writer.as_raw_fd()).unwrap());
    let session = Rc::new(RefCell::new(Session::default()));
    session.borrow_mut().hello(VERSION, CAPABILITIES).unwrap();
    session
        .borrow_mut()
        .open(1, w(), "Selectable list", 500., 400.)
        .unwrap();
    let (owner, cx) =
        app.add_window_view(|_, _| View::new(w(), session.clone(), transport.clone()));
    cx.update(|window, _| window.activate_window());
    cx.simulate_a11y_active(true);
    let mut ops = vec![
        Op::Create(n(0), Kind::Container, "".into(), None),
        Op::SetStyle(
            n(0),
            vec![
                Style::Direction(1),
                Style::Fields(vec![
                    Field::Width(Length::Px(400.)),
                    Field::Height(Length::Px(360.)),
                ]),
            ],
        ),
        Op::Create(n(1), Kind::Input, "Find".into(), Some(handler(1))),
        Op::SetEditor(n(1), editor("Search")),
        Op::SetStyle(n(1), size_style(300., 36.)),
        Op::Create(n(2), Kind::VirtualList, "".into(), Some(handler(2))),
        Op::SetListConfig(
            n(2),
            ListConfig {
                estimated_height: 40.,
                overscan: 0.,
                max_active: 4,
                scroll_policy: ScrollPolicy::KeepPosition,
                scrollbar: false,
                managed: true,
            },
        ),
        Op::SetListOrder(
            n(2),
            Order {
                revision: 1,
                runs: vec![IdRun { first: 1, count: 3 }],
            },
        ),
        Op::SetListInput(n(2), Some(config())),
        Op::SetAccessibility(n(2), Some(metadata(Role::ListBox(true), "Results"))),
        Op::SetStyle(n(2), size_style(300., 140.)),
        Op::Create(n(3), Kind::Button, "After".into(), Some(handler(3))),
        Op::SetStyle(n(3), size_style(80., 32.)),
    ];
    for (slot, label) in [(4, "First"), (5, "Second"), (6, "Disabled")] {
        ops.extend([
            Op::Create(n(slot), Kind::Container, "".into(), None),
            Op::SetStyle(n(slot), size_style(300., 40.)),
            Op::SetAccessibility(
                n(slot),
                Some(metadata(
                    Role::OptionItem(OptionItem {
                        index: slot - 4,
                        count: Some(3),
                        selected: slot == 4,
                        disabled: slot == 6,
                    }),
                    label,
                )),
            ),
        ]);
    }
    ops.extend([
        Op::Create(n(7), Kind::Button, "Inspect".into(), Some(handler(7))),
        Op::SetStyle(n(7), size_style(80., 24.)),
        Op::Create(n(8), Kind::Input, "Inline".into(), Some(handler(8))),
        Op::SetEditor(n(8), editor("Inline editor")),
        Op::SetStyle(n(8), size_style(100., 32.)),
        Op::Splice(n(4), 0, 0, vec![n(7)]),
        Op::Splice(n(5), 0, 0, vec![n(8)]),
        Op::SetListRows(
            n(2),
            (4..=6)
                .map(|slot| Row {
                    id: slot - 3,
                    node: n(slot),
                })
                .collect(),
        ),
        Op::Splice(n(2), 0, 0, vec![n(4), n(5), n(6)]),
        Op::Splice(
            n(0),
            0,
            0,
            if query_first {
                vec![n(1), n(2), n(3)]
            } else {
                vec![n(2), n(1), n(3)]
            },
        ),
        Op::SetRoot(Some(n(0))),
    ]);
    apply(&owner, cx, ops);
    transport.mailbox.lock().unwrap().drain(256);
    (owner, cx, transport, reader)
}
fn requests(transport: &Transport) -> Vec<Request> {
    transport
        .mailbox
        .lock()
        .unwrap()
        .drain(256)
        .into_iter()
        .filter_map(|event| match event {
            Event::ListInput(_, _, _, _, _, request) => Some(request),
            _ => None,
        })
        .collect()
}
fn focus(owner: &Entity<View>, cx: &mut VisualTestContext, node: NodeId) {
    cx.update(|window, cx| {
        owner.update(cx, |view, cx| {
            let handle = if node == n(2) {
                view.lists[&node].borrow().list_focus.clone().unwrap()
            } else {
                view.editors[&node].focus_handle(cx)
            };
            handle.focus(window, cx);
            cx.notify();
        })
    });
    draw(cx);
}
fn ax(cx: &VisualTestContext, label: &str) -> (gpui::accesskit::NodeId, gpui::accesskit::Node) {
    cx.a11y_tree()
        .unwrap()
        .nodes
        .into_iter()
        .find(|(_, node)| node.label() == Some(label))
        .unwrap_or_else(|| panic!("missing {label}"))
}
fn ax_focus(cx: &VisualTestContext) -> String {
    let tree = cx.a11y_tree().unwrap();
    tree.nodes
        .iter()
        .find(|(id, _)| *id == tree.focus)
        .unwrap()
        .1
        .label()
        .unwrap_or("")
        .into()
}
fn point(cx: &mut VisualTestContext, label: &str, right: bool) -> gpui::Point<gpui::Pixels> {
    let b = ax(cx, label).1.bounds().unwrap();
    let scale = cx.update(|w, _| f64::from(w.scale_factor()));
    gpui::point(
        px((if right { b.x1 - 8. } else { (b.x0 + b.x1) / 2. } / scale) as f32),
        px(((b.y0 + b.y1) / 2. / scale) as f32),
    )
}
fn action(
    cx: &mut VisualTestContext,
    label: &str,
    action: gpui::accesskit::Action,
    data: Option<gpui::accesskit::ActionData>,
) {
    cx.simulate_a11y_action(gpui::accesskit::ActionRequest {
        action,
        target_node: ax(cx, label).0,
        target_tree: gpui::accesskit::TreeId::ROOT,
        data,
    });
    draw(cx);
}
#[test]
fn ordered_keys_cursor_selection_and_child_focus_are_independent() {
    let mut app = TestAppContext::single();
    let (owner, cx, transport, _reader) = setup(&mut app, true);
    focus(&owner, cx, n(2));
    assert_eq!(ax_focus(cx), "First");
    let nodes = cx.a11y_tree().unwrap().nodes;
    assert_eq!(
        nodes
            .iter()
            .filter(|(_, n)| n.role() == gpui::accesskit::Role::ListBoxOption)
            .count(),
        3,
        "one semantic row each"
    );
    cx.simulate_keystrokes("down down shift-up space enter shift-f10 escape");
    draw(cx);
    assert_eq!(
        requests(&transport),
        vec![
            Request::Navigate(Navigation::Next, None),
            Request::Navigate(Navigation::Next, None),
            Request::Navigate(Navigation::Previous, Some(Gesture::Range { extend: false })),
            Request::SelectActive(Gesture::Toggle),
            Request::ConfirmActive(Confirmation::Primary),
            Request::ContextActive,
            Request::Cancel
        ]
    );
    let root = owner.read_with(cx, |v, _| {
        v.lists[&n(2)].borrow().list_focus.clone().unwrap()
    });
    apply(
        &owner,
        cx,
        vec![Op::SetListInput(
            n(2),
            Some(Config {
                cursor: Some(2),
                busy: true,
                ..config()
            }),
        )],
    );
    assert_eq!(
        root,
        owner.read_with(cx, |v, _| v.lists[&n(2)]
            .borrow()
            .list_focus
            .clone()
            .unwrap())
    );
    assert_eq!(ax_focus(cx), "Second");
    assert_eq!(ax(cx, "First").1.is_selected(), Some(true));
    assert_eq!(ax(cx, "Second").1.is_selected(), Some(false));
    assert!(ax(cx, "Results").1.is_busy());
    focus(&owner, cx, n(8));
    cx.simulate_keystrokes("down home space enter escape");
    draw(cx);
    assert!(requests(&transport).is_empty());
    focus(&owner, cx, n(2));
    cx.simulate_keystrokes("tab");
    draw(cx);
    cx.update(|window, cx| {
        owner.update(cx, |view, cx| {
            assert_eq!(view.focus.borrow().focused_node(window, cx), Some(n(7)))
        })
    });
}
#[test]
fn query_editor_keeps_caret_and_composition_while_list_handles_its_navigation() {
    let mut app = TestAppContext::single();
    let (owner, cx, transport, _reader) = setup(&mut app, true);
    focus(&owner, cx, n(1));
    assert_eq!(ax_focus(cx), "First");
    cx.simulate_keystrokes("down enter");
    draw(cx);
    assert_eq!(
        requests(&transport),
        vec![
            Request::Navigate(Navigation::Next, None),
            Request::ConfirmActive(Confirmation::Primary)
        ]
    );
    let secondary = if cfg!(target_os = "macos") {
        "cmd-enter"
    } else {
        "ctrl-enter"
    };
    cx.simulate_keystrokes(secondary);
    draw(cx);
    assert_eq!(
        requests(&transport),
        vec![Request::ConfirmActive(Confirmation::Secondary)]
    );
    cx.simulate_keystrokes("left home end space");
    draw(cx);
    assert!(requests(&transport).is_empty());
    cx.update(|window, cx| {
        owner.update(cx, |view, cx| {
            assert!(view.editors[&n(1)].snapshot(window, cx).text.contains(' '));
            view.editors[&n(1)].mark_test_text("候補", window, cx);
            assert!(view.editors[&n(1)].is_composing(cx));
        })
    });
    draw(cx);
    assert_eq!(ax_focus(cx), "Search");
    requests(&transport);
    cx.simulate_keystrokes("escape");
    draw(cx);
    assert!(requests(&transport).is_empty());
    cx.update(|_, cx| {
        owner.update(cx, |view, cx| {
            assert!(!view.editors[&n(1)].is_composing(cx))
        })
    });
    cx.simulate_keystrokes("escape");
    draw(cx);
    assert_eq!(requests(&transport), vec![Request::Cancel]);
}
#[test]
fn pointer_and_accessibility_intents_preserve_context_and_retire_old_presses() {
    use gpui::accesskit::{Action, ActionData};
    let mut app = TestAppContext::single();
    let (owner, cx, transport, _reader) = setup(&mut app, true);
    let p = point(cx, "First", true);
    cx.simulate_click(p, Default::default());
    draw(cx);
    assert_eq!(
        requests(&transport),
        vec![Request::Select(1, Gesture::Replace)]
    );
    for count in [2, 3] {
        cx.simulate_event(gpui::MouseDownEvent {
            position: p,
            modifiers: Default::default(),
            button: gpui::MouseButton::Left,
            click_count: count,
            first_mouse: false,
        });
        cx.simulate_event(gpui::MouseUpEvent {
            position: p,
            modifiers: Default::default(),
            button: gpui::MouseButton::Left,
            click_count: count,
        });
        draw(cx);
        assert_eq!(
            requests(&transport),
            if count == 2 {
                vec![Request::Confirm(1, Confirmation::Primary)]
            } else {
                vec![]
            }
        );
    }
    let p = point(cx, "Second", true);
    cx.simulate_mouse_down(p, gpui::MouseButton::Right, Default::default());
    cx.simulate_mouse_up(p, gpui::MouseButton::Right, Default::default());
    draw(cx);
    assert_eq!(requests(&transport), vec![Request::Context(2)]);
    assert_eq!(ax_focus(cx), "First");
    let p = point(cx, "Inspect", false);
    cx.simulate_click(p, Default::default());
    draw(cx);
    assert!(requests(&transport).is_empty());
    action(cx, "Second", Action::Focus, None);
    assert_eq!(requests(&transport), vec![Request::Focus(2)]);
    for (code, expected) in [
        (crate::semantics::LIST_SELECT, Request::SetSelected(1, true)),
        (
            crate::semantics::LIST_DESELECT,
            Request::SetSelected(1, false),
        ),
        (
            crate::semantics::LIST_CONFIRM,
            Request::Confirm(1, Confirmation::Primary),
        ),
        (
            crate::semantics::LIST_CONFIRM_SECONDARY,
            Request::Confirm(1, Confirmation::Secondary),
        ),
        (crate::semantics::LIST_CONTEXT, Request::Context(1)),
    ] {
        action(
            cx,
            "First",
            Action::CustomAction,
            Some(ActionData::CustomAction(code)),
        );
        assert_eq!(requests(&transport), vec![expected]);
    }
    assert!(!ax(cx, "Disabled").1.supports_action(Action::Click));
    let old = owner.read_with(cx, |v, _| {
        list_input::Route::new(v.session.borrow().tree(w()).unwrap().get(n(2)).unwrap()).unwrap()
    });
    let p = point(cx, "First", true);
    cx.simulate_mouse_down(p, gpui::MouseButton::Left, Default::default());
    apply(
        &owner,
        cx,
        vec![Op::SetListInput(
            n(2),
            Some(Config {
                generation: 2,
                ..config()
            }),
        )],
    );
    cx.simulate_mouse_up(p, gpui::MouseButton::Left, Default::default());
    draw(cx);
    assert!(requests(&transport).is_empty());
    cx.update(|_, cx| {
        owner.update(cx, |view, _| {
            assert!(!view.list_input_request(old, Request::Select(1, Gesture::Toggle)))
        })
    });
}
#[test]
fn disabling_and_removing_input_retires_focus_but_keeps_rows_visible() {
    let mut app = TestAppContext::single();
    let (owner, cx, transport, _reader) = setup(&mut app, true);
    focus(&owner, cx, n(2));
    let old = owner.read_with(cx, |v, _| {
        v.lists[&n(2)].borrow().list_focus.clone().unwrap()
    });
    apply(
        &owner,
        cx,
        vec![Op::SetListInput(
            n(2),
            Some(Config {
                generation: 2,
                disabled: true,
                ..config()
            }),
        )],
    );
    assert!(ax(cx, "Results").1.is_disabled());
    assert!(ax(cx, "First").1.is_disabled());
    cx.update(|window, cx| {
        owner.update(cx, |view, _| {
            assert!(!old.is_focused(window));
            assert!(view.lists[&n(2)].borrow().list_focus.is_none());
        })
    });
    cx.simulate_keystrokes("down enter");
    draw(cx);
    assert!(requests(&transport).is_empty());
    apply(
        &owner,
        cx,
        vec![
            Op::SetListInput(
                n(2),
                Some(Config {
                    generation: 3,
                    ..config()
                }),
            ),
            Op::SetListAxis(n(2), Axis::Horizontal),
        ],
    );
    focus(&owner, cx, n(2));
    cx.simulate_keystrokes("right");
    draw(cx);
    assert_eq!(
        requests(&transport),
        vec![Request::Navigate(Navigation::Next, None)]
    );
    apply(&owner, cx, vec![Op::SetListInput(n(2), None)]);
    assert!(owner.read_with(cx, |v, _| v.lists[&n(2)].borrow().list_focus.is_none()));
}
#[test]
fn query_after_results_keeps_the_same_explicit_accessibility_focus_contract() {
    let mut app = TestAppContext::single();
    let (owner, cx, transport, _reader) = setup(&mut app, false);
    focus(&owner, cx, n(1));
    assert_eq!(ax_focus(cx), "First");
    cx.simulate_keystrokes("down");
    draw(cx);
    assert_eq!(
        requests(&transport),
        vec![Request::Navigate(Navigation::Next, None)]
    );
}
#[test]
fn override_commands_win_and_hidden_owners_do_not_consume_query_keys() {
    let mut app = TestAppContext::single();
    let (owner, cx, transport, _reader) = setup(&mut app, true);
    apply(
        &owner,
        cx,
        vec![
            Op::Create(n(9), Kind::CommandScope, "".into(), Some(handler(9))),
            Op::SetCommands(
                n(9),
                vec![CommandConfig {
                    id: "submit".into(),
                    generation: 1,
                    label: "Submit".into(),
                    enabled: true,
                    checked: None,
                    shortcuts: vec![Shortcut {
                        key: "enter".into(),
                        modifiers: vec![],
                        priority: ShortcutPriority::Override,
                        text_input: ShortcutTextInput::Always,
                        during_composition: false,
                    }],
                    target: CommandTarget::Callback,
                }],
            ),
            Op::Splice(n(9), 0, 0, vec![n(0)]),
            Op::SetRoot(Some(n(9))),
        ],
    );
    focus(&owner, cx, n(1));
    transport.mailbox.lock().unwrap().drain(256);
    cx.simulate_keystrokes("enter");
    draw(cx);
    let events = transport.mailbox.lock().unwrap().drain(256);
    assert_eq!(
        events
            .iter()
            .filter(|event| matches!(event,Event::CommandInvoked(_,_,_,_,id,_,_) if id=="submit"))
            .count(),
        1
    );
    assert!(
        !events
            .iter()
            .any(|event| matches!(event, Event::ListInput(..)))
    );
    apply(
        &owner,
        cx,
        vec![Op::SetStyle(
            n(2),
            vec![Style::Fields(vec![Field::Visibility(1)])],
        )],
    );
    focus(&owner, cx, n(1));
    cx.simulate_keystrokes("down escape");
    draw(cx);
    assert!(requests(&transport).is_empty());
}
#[test]
fn large_cursor_reveal_keeps_native_focus_and_allows_one_row_materialization() {
    let mut app = TestAppContext::single();
    let (owner, cx, transport, _reader) = setup(&mut app, true);
    focus(&owner, cx, n(2));
    apply(
        &owner,
        cx,
        vec![
            Op::SetListOrder(
                n(2),
                Order {
                    revision: 2,
                    runs: vec![IdRun {
                        first: 1,
                        count: 100_000,
                    }],
                },
            ),
            Op::SetListInput(
                n(2),
                Some(Config {
                    cursor: Some(100_000),
                    ..config()
                }),
            ),
        ],
    );
    let observed = owner.read_with(cx, |v, _| v.lists[&n(2)].borrow().observed.clone().unwrap());
    assert!(observed.requested.contains(&100_000));
    assert!(observed.requested.len() <= 4);
    cx.update(|window, cx| {
        owner.update(cx, |view, cx| {
            assert!(
                view.list_pins(window, cx).is_empty(),
                "logical cursor is not an OS-owned eviction veto"
            );
            assert!(
                view.lists[&n(2)]
                    .borrow()
                    .list_focus
                    .as_ref()
                    .unwrap()
                    .is_focused(window)
            );
        })
    });
    apply(
        &owner,
        cx,
        vec![
            Op::Create(n(9), Kind::Container, "".into(), None),
            Op::SetStyle(n(9), size_style(300., 40.)),
            Op::SetAccessibility(
                n(9),
                Some(metadata(
                    Role::OptionItem(OptionItem {
                        index: 99_999,
                        count: Some(100_000),
                        selected: false,
                        disabled: false,
                    }),
                    "Last",
                )),
            ),
            Op::SetListRows(
                n(2),
                vec![Row {
                    id: 100_000,
                    node: n(9),
                }],
            ),
            Op::Splice(n(2), 0, 3, vec![n(9)]),
            Op::Remove(n(4)),
            Op::Remove(n(5)),
            Op::Remove(n(6)),
            Op::Remove(n(7)),
            Op::Remove(n(8)),
            Op::SetListConfig(
                n(2),
                ListConfig {
                    estimated_height: 40.,
                    overscan: 0.,
                    max_active: 1,
                    scroll_policy: ScrollPolicy::KeepPosition,
                    scrollbar: false,
                    managed: true,
                },
            ),
        ],
    );
    assert_eq!(ax_focus(cx), "Last");
    assert_eq!(
        owner.read_with(cx, |v, _| v.lists[&n(2)].borrow().resource_counts()),
        (1, 1)
    );
    requests(&transport);
    cx.simulate_keystrokes("up");
    draw(cx);
    assert_eq!(
        requests(&transport),
        vec![Request::Navigate(Navigation::Previous, None)]
    );
}

#[test]
fn section_decorations_do_not_claim_option_semantics_or_steal_owner_focus() {
    let mut app = TestAppContext::single();
    let (owner, cx, transport, _reader) = setup(&mut app, true);
    apply(
        &owner,
        cx,
        vec![Op::SetAccessibility(
            n(6),
            Some(metadata(Role::Group, "Section")),
        )],
    );
    focus(&owner, cx, n(2));
    let p = point(cx, "Section", true);
    cx.simulate_click(p, Default::default());
    draw(cx);
    assert_eq!(
        ax_focus(cx),
        "First",
        "decoration clicks preserve composite focus"
    );
    assert!(requests(&transport).is_empty());
    assert!(
        !cx.a11y_tree()
            .unwrap()
            .nodes
            .iter()
            .any(|(_, node)| node.role() == gpui::accesskit::Role::ListItem),
        "a listbox section is not an ordinary list item"
    );
    apply(
        &owner,
        cx,
        vec![
            Op::Splice(n(4), 0, 1, vec![]),
            Op::Splice(n(6), 0, 0, vec![n(7)]),
        ],
    );
    let p = point(cx, "Inspect", false);
    cx.simulate_click(p, Default::default());
    draw(cx);
    let events = transport.mailbox.lock().unwrap().drain(256);
    assert!(
        events
            .iter()
            .any(|event| matches!(event,Event::Press(_,node,_,_) if *node==n(7)))
    );
    assert!(
        !events
            .iter()
            .any(|event| matches!(event, Event::ListInput(..)))
    );
    cx.update(|window, cx| {
        owner.update(cx, |view, cx| {
            let pins = view.list_pins(window, cx);
            assert!(
                pins.iter()
                    .any(|pin| pin.node == n(2) && pin.rows.contains(&3)),
                "real child focus still retains its section row"
            );
        })
    });
}
