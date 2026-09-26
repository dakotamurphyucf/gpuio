//! GPUI event dispatch and real clipboard; separate from physical AppKit input.
use super::*;
#[cfg(target_os = "macos")]
#[path = "appkit_input.rs"]
mod appkit;

fn key(cx: &mut gpui::AsyncApp, handle: gpui::WindowHandle<View>, key: &str) {
    for down in [true, false] {
        cx.update_window(handle.into(), |_, window, cx| {
            let keystroke = gpui::Keystroke::parse(key).unwrap();
            let event = if down {
                gpui::PlatformInput::KeyDown(gpui::KeyDownEvent {
                    keystroke,
                    is_held: false,
                    prefer_character_input: false,
                })
            } else {
                gpui::PlatformInput::KeyUp(gpui::KeyUpEvent { keystroke })
            };
            window.dispatch_event(event, cx);
        })
        .unwrap();
    }
}
fn copy(cx: &mut gpui::AsyncApp, window: gpui::WindowHandle<View>) {
    key(
        cx,
        window,
        if cfg!(target_os = "macos") {
            "cmd-c"
        } else {
            "ctrl-c"
        },
    );
}
fn clipboard(cx: &mut gpui::AsyncApp) -> String {
    cx.update(|cx| cx.read_from_clipboard().unwrap().text().unwrap())
}
fn select(cx: &mut gpui::AsyncApp, window: gpui::WindowHandle<View>, value: Selection) {
    window
        .update(cx, |view, window, cx| {
            let native = view.tables[&node(0)].borrow().native.clone();
            native.update(cx, |state, cx| assert!(state.replace_selection(value, cx)));
            native.focus_handle(cx).focus(window, cx);
        })
        .unwrap();
}
fn requests(cx: &mut gpui::AsyncApp, window: gpui::WindowHandle<View>) -> Vec<wire::Request> {
    window
        .update(cx, |view, _, _| {
            view.transport
                .mailbox
                .lock()
                .unwrap()
                .drain(128)
                .into_iter()
                .filter_map(|event| {
                    if let Event::TableInput(w, n, h, _, input) = event {
                        assert_eq!(
                            (w, n, h),
                            (window_id(), node(0), HandlerId::from_parts(0, 1).unwrap())
                        );
                        assert_eq!((input.schema_revision, input.query_generation), (1, 0));
                        Some(input.request)
                    } else {
                        None
                    }
                })
                .collect()
        })
        .unwrap()
}
fn assert_selection(
    cx: &mut gpui::AsyncApp,
    window: gpui::WindowHandle<View>,
    expected: wire::Selection,
) {
    window
        .update(cx, |view, window, cx| {
            let native = view.tables[&node(0)].borrow();
            assert!(native.native.focus_handle(cx).is_focused(window));
            assert_eq!(selection(native.native.read(cx).selection()), expected);
        })
        .unwrap();
}

pub(super) async fn exercise(cx: &mut gpui::AsyncApp, window: gpui::WindowHandle<View>) {
    #[cfg(target_os = "macos")]
    appkit::table(cx, window).await;
    requests(cx, window);
    let position = window
        .update(cx, |view, _, _| {
            view.probes.borrow()[&node(3)].bounds.center()
        })
        .unwrap();
    super::super::super::native_test::move_mouse(cx, window, position, false);
    super::super::super::native_test::mouse(cx, window, position, true);
    super::super::super::native_test::mouse(cx, window, position, false);
    frame(cx, window).await;
    assert_selection(cx, window, wire::Selection::Cell(1, "name".into()));
    key(cx, window, "down");
    key(cx, window, "right");
    frame(cx, window).await;
    assert_selection(cx, window, wire::Selection::Cell(2, "value".into()));
    key(cx, window, "enter");
    key(cx, window, "shift-f10");
    copy(cx, window);
    assert_eq!(clipboard(cx), "日本語 row 2");
    let events = requests(cx, window);
    assert!(
        events.ends_with(&[
            wire::Request::Activate(2, Some("value".into())),
            wire::Request::Context(wire::Selection::Cell(2, "value".into())),
            wire::Request::Copy(wire::Selection::Cell(2, "value".into())),
        ]),
        "ordered activation/context/copy: {events:?}"
    );

    let unicode = "日本語 👨‍👩‍👧‍👦\t\"e\u{301}\"\nnext";
    apply(
        cx,
        window,
        vec![Op::SetTableCell(
            node(2),
            wire::Cell {
                column: "name".into(),
                copy_text: unicode.into(),
            },
        )],
    );
    select(
        cx,
        window,
        Selection::Cell {
            row: RowKey(1),
            column: "name".into(),
        },
    );
    frame(cx, window).await;
    copy(cx, window);
    assert_eq!(clipboard(cx), unicode, "one cell is copied verbatim");
    select(cx, window, Selection::Row(RowKey(1)));
    frame(cx, window).await;
    copy(cx, window);
    assert_eq!(
        clipboard(cx),
        "\"日本語 👨‍👩‍👧‍👦\t\"\"e\u{301}\"\"\nnext\"\t日本語 row 1"
    );
    let previous = clipboard(cx);
    for selected in [
        Selection::Column("name".into()),
        Selection::Cell {
            row: RowKey(50_001),
            column: "name".into(),
        },
    ] {
        select(cx, window, selected.clone());
        frame(cx, window).await;
        requests(cx, window);
        copy(cx, window);
        assert_eq!(
            clipboard(cx),
            previous,
            "unavailable selections must not truncate"
        );
        assert_eq!(
            requests(cx, window),
            vec![wire::Request::Copy(selection(&selected))]
        );
    }
    select(
        cx,
        window,
        Selection::Cell {
            row: RowKey(1),
            column: "name".into(),
        },
    );
    let mut disabled = config();
    disabled.disabled = true;
    apply(cx, window, vec![Op::SetTable(node(0), disabled)]);
    frame(cx, window).await;
    // Force a stale logical focus to exercise the live input policy, even when
    // normal traversal has already made the disabled table ineligible.
    select(
        cx,
        window,
        Selection::Cell {
            row: RowKey(1),
            column: "name".into(),
        },
    );
    requests(cx, window);
    for key_name in ["down", "enter", "shift-f10"] {
        key(cx, window, key_name);
    }
    copy(cx, window);
    assert!(requests(cx, window).is_empty());
    assert_eq!(clipboard(cx), previous);
    apply(cx, window, vec![Op::SetTable(node(0), config())]);
    select(cx, window, Selection::Empty);
    frame(cx, window).await;
    command_and_child(cx, window).await;
    complete_column_and_hidden(cx, window).await;
    eprintln!(
        "GPUIO_TABLE_INPUT_OK: pointer focus, keys, ordered activation/context, exact Unicode and quoted row clipboard, unavailable data and disabled policy"
    );
}

async fn command_and_child(cx: &mut gpui::AsyncApp, window: gpui::WindowHandle<View>) {
    let command = CommandConfig {
        id: "copy".into(),
        label: "Copy selection".into(),
        generation: 1,
        enabled: true,
        checked: None,
        shortcuts: vec![],
        target: CommandTarget::Native(NativeCommand::Copy),
    };
    apply(
        cx,
        window,
        vec![
            Op::Create(
                node(61),
                Kind::CommandScope,
                String::new(),
                Some(HandlerId::from_parts(61, 1).unwrap()),
            ),
            Op::SetCommands(node(61), vec![command.clone()]),
            Op::Create(node(62), Kind::CommandButton, String::new(), None),
            Op::SetCommandRef(node(62), "copy".into()),
            Op::Splice(node(61), 0, 0, vec![node(0), node(62)]),
            Op::SetRoot(Some(node(61))),
        ],
    );
    select(
        cx,
        window,
        Selection::Cell {
            row: RowKey(2),
            column: "name".into(),
        },
    );
    frame(cx, window).await;
    window
        .update(cx, |view, window, cx| {
            assert!(view.command_available(&command, window, cx));
            let mut paste = command.clone();
            paste.target = CommandTarget::Native(NativeCommand::Paste);
            assert!(!view.command_available(&paste, window, cx));
        })
        .unwrap();
    key(cx, window, "tab");
    frame(cx, window).await;
    window
        .update(cx, |view, window, _| {
            assert_eq!(
                view.focus.borrow().focused_node(window),
                Some(node(62)),
                "Tab leaves table"
            );
        })
        .unwrap();
    cx.update(|cx| cx.write_to_clipboard(gpui::ClipboardItem::new_string("sentinel".into())));
    requests(cx, window);
    key(cx, window, "enter");
    frame(cx, window).await;
    assert_eq!(
        clipboard(cx),
        "日本語 row 2",
        "toolbar retains table command target"
    );
    assert_selection(cx, window, wire::Selection::Cell(2, "name".into()));
    assert_eq!(
        requests(cx, window),
        vec![wire::Request::Copy(wire::Selection::Cell(2, "name".into()))]
    );

    // Prepend a native editor to the existing cell; its pointer, keyboard and
    // clipboard belong to it even though the table remains its focus ancestor.
    apply(
        cx,
        window,
        vec![
            Op::Create(
                node(63),
                Kind::Input,
                "child 日本語".into(),
                Some(HandlerId::from_parts(63, 1).unwrap()),
            ),
            Op::SetEditor(
                node(63),
                EditorConfig {
                    label: "Cell editor".into(),
                    placeholder: String::new(),
                    read_only: false,
                    disabled: false,
                    submit_on_enter: true,
                    auto_focus: false,
                    min_rows: 1,
                    max_rows: 1,
                },
            ),
            Op::SetStyle(
                node(63),
                vec![
                    Style::Width(Length::Px(140.)),
                    Style::Height(Length::Px(28.)),
                ],
            ),
            Op::Create(node(64), Kind::Container, String::new(), None),
            Op::Splice(node(2), 0, 1, vec![]),
            Op::Splice(node(64), 0, 0, vec![node(63), node(3)]),
            Op::Splice(node(2), 0, 0, vec![node(64)]),
        ],
    );
    frame(cx, window).await;
    let point = window
        .update(cx, |view, _, cx| {
            view.editors[&node(63)].input_bounds(cx).center()
        })
        .unwrap();
    requests(cx, window);
    super::super::super::native_test::move_mouse(cx, window, point, false);
    frame(cx, window).await;
    super::super::super::native_test::mouse(cx, window, point, true);
    super::super::super::native_test::mouse(cx, window, point, false);
    frame(cx, window).await;
    window
        .update(cx, |view, window, cx| {
            assert!(
                view.editors[&node(63)].focus_handle(cx).is_focused(window),
                "pointer keeps child focus"
            );
            assert_eq!(view.command_editor(window, cx), Some(node(63)));
        })
        .unwrap();
    for input in [
        "left",
        "right",
        "up",
        "down",
        "home",
        "end",
        "enter",
        "shift-f10",
    ] {
        key(cx, window, input);
    }
    key(
        cx,
        window,
        if cfg!(target_os = "macos") {
            "cmd-a"
        } else {
            "ctrl-a"
        },
    );
    copy(cx, window);
    assert_eq!(clipboard(cx), "child 日本語");
    window
        .update(cx, |view, window, cx| {
            let route = {
                let session = view.session.borrow();
                let tree = session.tree(view.id).unwrap();
                super::super::super::command::Route::new(
                    tree,
                    node(61),
                    &Arc::new(command.clone()),
                    CommandSource::Button(node(62)),
                )
            };
            assert!(view.invoke_command(&route, window, cx));
        })
        .unwrap();
    assert_eq!(clipboard(cx), "child 日本語");
    assert!(
        requests(cx, window).is_empty(),
        "child input must not emit table actions"
    );

    #[cfg(target_os = "macos")]
    appkit::editor(cx, window).await;

    // Disabling an ancestor table makes retained editors inert, including their
    // direct input-handler and command gates, not just the table's own actions.
    let mut disabled = config();
    disabled.disabled = true;
    apply(cx, window, vec![Op::SetTable(node(0), disabled)]);
    window
        .update(cx, |view, window, cx| {
            assert!(!view.focus.borrow().allows(node(63)));
            assert!(!view.command_available(&command, window, cx));
        })
        .unwrap();
    let before = window
        .update(cx, |view, window, cx| {
            view.editors[&node(63)].snapshot(window, cx)
        })
        .unwrap();
    key(cx, window, "backspace");
    let after = window
        .update(cx, |view, window, cx| {
            view.editors[&node(63)].snapshot(window, cx)
        })
        .unwrap();
    assert_eq!(before.text, after.text);
    apply(
        cx,
        window,
        vec![
            Op::SetTable(node(0), config()),
            Op::Splice(node(64), 0, 2, vec![]),
            Op::Splice(node(2), 0, 1, vec![node(3)]),
            Op::Remove(node(63)),
            Op::Remove(node(64)),
            Op::SetRoot(Some(node(0))),
            Op::Splice(node(61), 0, 2, vec![]),
            Op::Remove(node(62)),
            Op::Remove(node(61)),
        ],
    );
    select(cx, window, Selection::Empty);
    frame(cx, window).await;
    eprintln!(
        "GPUIO_TABLE_COMMAND_FOCUS_OK: Tab exit, toolbar Copy, embedded editor priority and disabled ancestry"
    );
}

async fn complete_column_and_hidden(cx: &mut gpui::AsyncApp, window: gpui::WindowHandle<View>) {
    apply(cx, window, vec![Op::SetListOrder(node(0), order(2, 12))]);
    select(cx, window, Selection::Column("name".into()));
    frame(cx, window).await;
    copy(cx, window);
    let mut expected = "\"日本語 👨‍👩‍👧‍👦\t\"\"e\u{301}\"\"\nnext\"".to_owned();
    for row in 2..=12 {
        expected.push_str(&format!("\n日本語 row {row}"));
    }
    assert_eq!(
        clipboard(cx),
        expected,
        "complete column follows logical order"
    );
    apply(
        cx,
        window,
        vec![Op::SetListOrder(node(0), order(3, 100_000))],
    );
    select(
        cx,
        window,
        Selection::Cell {
            row: RowKey(1),
            column: "name".into(),
        },
    );
    frame(cx, window).await;
    requests(cx, window);
    apply(
        cx,
        window,
        vec![Op::SetStyle(
            node(0),
            vec![Style::Fields(vec![Field::Display(3)])],
        )],
    );
    // Do not paint: callbacks from the previous visible frame must still consult
    // current retained policy before moving selection, writing or delivering input.
    for key_name in ["down", "enter", "shift-f10"] {
        key(cx, window, key_name);
    }
    copy(cx, window);
    assert!(requests(cx, window).is_empty(), "hidden stale-frame input");
    assert_eq!(clipboard(cx), expected);
    apply(
        cx,
        window,
        vec![Op::SetStyle(
            node(0),
            vec![
                Style::Width(Length::Px(520.)),
                Style::Height(Length::Px(300.)),
            ],
        )],
    );
    select(cx, window, Selection::Empty);
    frame(cx, window).await;
}
