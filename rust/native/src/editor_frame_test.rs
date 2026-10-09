//! Production input frame on TestPlatform. No OS keyboard/IME/AX claim.
use super::*;
use gpui::TestAppContext;
use std::{os::fd::AsRawFd, os::unix::net::UnixStream};
fn id(n: i64) -> NodeId {
    NodeId::from_parts(n, 1).unwrap()
}
fn config(read_only: bool, disabled: bool) -> EditorConfig {
    EditorConfig {
        label: "Draft".into(),
        placeholder: "".into(),
        read_only,
        disabled,
        submit_on_enter: false,
        auto_focus: false,
        min_rows: 1,
        max_rows: 1,
    }
}
fn frame(loading: bool) -> gpuio_protocol::editor_frame::Config {
    gpuio_protocol::editor_frame::Config {
        clear_label: Some("Clear draft".into()),
        loading,
        gap: 6.,
    }
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
    cx.notify();
}
fn command(view: &mut View, command: EditorCommand, window: &mut Window, cx: &mut App) {
    assert!(matches!(
        view.editors
            .get_mut(&id(0))
            .unwrap()
            .command(&command, window, cx),
        EditorResult::Applied(_)
    ));
}
fn target(view: &View, cx: &App) -> Option<ClearTarget> {
    let session = view.session.borrow();
    view.clear_target(session.tree(view.id).unwrap().get(id(0)).unwrap(), cx)
}
fn mount() -> Vec<Op> {
    let mut ops = vec![
        Op::Create(
            id(0),
            Kind::Input,
            "seed λ".into(),
            Some(gpuio_protocol::HandlerId::from_parts(0, 1).unwrap()),
        ),
        Op::SetEditor(id(0), config(false, false)),
        Op::SetEditorFrame(id(0), Some(frame(false))),
        Op::SetStyle(
            id(0),
            vec![
                Style::Width(Length::Px(360.)),
                Style::Height(Length::Px(42.)),
            ],
        ),
    ];
    for n in 1..=4 {
        ops.push(Op::Create(id(n), Kind::Container, "".into(), None));
    }
    ops.extend([
        Op::Create(id(5), Kind::Text, "Prefix".into(), None),
        Op::Splice(id(1), 0, 0, vec![id(5)]),
        Op::Create(id(6), Kind::Text, "Suffix".into(), None),
        Op::Splice(id(4), 0, 0, vec![id(6)]),
        Op::Splice(id(0), 0, 0, (1..=4).map(id).collect()),
        Op::SetRoot(Some(id(0))),
    ]);
    ops
}
fn loading() -> Vec<Op> {
    let spinner = gpuio_protocol::spinner::Config {
        label: "Loading draft".into(),
        animated: false,
        period_ms: 800,
        easing: gpuio_protocol::animation::Easing::Linear,
        source: None,
    };
    vec![
        Op::SetEditorFrame(id(0), Some(frame(true))),
        Op::Create(id(7), Kind::Loading, "".into(), None),
        Op::SetLoading(id(7), spinner.loading()),
        Op::SetSpinner(id(7), spinner),
        Op::Splice(id(2), 0, 0, vec![id(7)]),
    ]
}
fn ready() -> Vec<Op> {
    vec![
        Op::SetEditorFrame(id(0), Some(frame(false))),
        Op::Splice(id(2), 0, 1, vec![]),
        Op::Remove(id(7)),
    ]
}

#[test]
fn clear_is_undoable_and_old_actions_cannot_clear_new_drafts_or_policies() {
    let mut app = TestAppContext::single();
    app.update(gpui_base::init);
    let (_reader, writer) = UnixStream::pair().unwrap();
    let transport = Arc::new(Transport::new(writer.as_raw_fd()).unwrap());
    let session = Rc::new(RefCell::new(Session::default()));
    let wid = WindowId::from_parts(0, 1).unwrap();
    session.borrow_mut().hello(VERSION, CAPABILITIES).unwrap();
    session
        .borrow_mut()
        .open(1, wid, "Input frame", 500., 240.)
        .unwrap();
    let (owner, cx) =
        app.add_window_view(|_, _| View::new(wid, session.clone(), transport.clone()));
    cx.simulate_a11y_active(true);
    cx.update(|window, cx| {
        owner.update(cx, |view, cx| apply(view, window, cx, mount()));
        window.draw(cx).clear(cx);
    });
    cx.run_until_parked();
    let original_focus = cx.update(|window, cx| {
        owner.update(cx, |view, cx| {
            command(view, EditorCommand::Focus, window, cx);
            let focus = view.editors[&id(0)].focus_handle(cx);
            let old = target(view, cx).expect("mounted clear");
            view.clear_input(&old, window, cx);
            let cleared = view.editors[&id(0)].snapshot(window, cx);
            assert_eq!(cleared.text, "");
            assert!(cleared.focused);
            assert_eq!(cleared.selection, EditorSelection { anchor: 0, head: 0 });
            assert!(target(view, cx).is_none());
            command(view, EditorCommand::Undo, window, cx);
            assert_eq!(view.editors[&id(0)].snapshot(window, cx).text, "seed λ");
            view.clear_input(&old, window, cx);
            assert_eq!(
                view.editors[&id(0)].snapshot(window, cx).text,
                "seed λ",
                "old revision rejected"
            );
            let old = target(view, cx).unwrap();
            apply(
                view,
                window,
                cx,
                vec![Op::SetEditor(id(0), config(true, false))],
            );
            assert!(target(view, cx).is_none());
            view.clear_input(&old, window, cx);
            apply(
                view,
                window,
                cx,
                vec![Op::SetEditor(id(0), config(false, false))],
            );
            view.clear_input(&old, window, cx);
            assert_eq!(
                view.editors[&id(0)].snapshot(window, cx).text,
                "seed λ",
                "policy roundtrip rejects old action"
            );
            let old = target(view, cx).unwrap();
            apply(view, window, cx, loading());
            assert!(target(view, cx).is_none());
            assert_eq!(view.editors[&id(0)].focus_handle(cx), focus);
            view.clear_input(&old, window, cx);
            focus
        });
        window.draw(cx).clear(cx);
        owner.read(cx).editors[&id(0)].focus_handle(cx)
    });
    let tree = cx.a11y_tree().unwrap();
    let (_, field) = tree
        .nodes
        .iter()
        .find(|(_, n)| n.label() == Some("Draft"))
        .unwrap();
    assert!(field.is_busy());
    for label in ["Prefix", "Suffix", "Loading draft"] {
        assert_eq!(
            tree.nodes
                .iter()
                .filter(|(_, n)| n.label() == Some(label))
                .count(),
            1,
            "slot mounted once"
        );
    }
    assert!(
        !tree
            .nodes
            .iter()
            .any(|(_, n)| n.label() == Some("Clear draft"))
    );
    // Loading is presentation only: ordinary typing still changes the draft.
    cx.simulate_keystrokes("x");
    cx.update(|window, cx| {
        owner.update(cx, |view, cx| {
            assert!(view.editors[&id(0)].snapshot(window, cx).text.contains('x'));
            apply(view, window, cx, ready());
            assert_eq!(view.editors[&id(0)].focus_handle(cx), original_focus);
            let old = target(view, cx).unwrap();
            view.editors[&id(0)].mark_test_text("界", window, cx);
            let composing = view.editors[&id(0)].snapshot(window, cx);
            assert!(composing.composition.is_some());
            assert!(target(view, cx).is_none());
            view.clear_input(&old, window, cx);
            assert_eq!(view.editors[&id(0)].snapshot(window, cx), composing);
        });
        window.draw(cx).clear(cx);
    });
    let tree = cx.a11y_tree().unwrap();
    assert!(
        !tree
            .nodes
            .iter()
            .find(|(_, n)| n.label() == Some("Draft"))
            .unwrap()
            .1
            .is_busy()
    );
}

#[test]
fn slots_leave_editor_room_and_clear_pointer_preserves_native_focus() {
    for multiline in [false, true] {
        let mut app = TestAppContext::single();
        app.update(gpui_base::init);
        let (_reader, writer) = UnixStream::pair().unwrap();
        let transport = Arc::new(Transport::new(writer.as_raw_fd()).unwrap());
        let session = Rc::new(RefCell::new(Session::default()));
        let wid = WindowId::from_parts(0, 1).unwrap();
        session.borrow_mut().hello(VERSION, CAPABILITIES).unwrap();
        session
            .borrow_mut()
            .open(1, wid, "Frame geometry", 500., 240.)
            .unwrap();
        let (owner, cx) =
            app.add_window_view(|_, _| View::new(wid, session.clone(), transport.clone()));
        cx.simulate_a11y_active(true);
        cx.update(|window, cx| {
            owner.update(cx, |view, cx| {
                let mut ops = mount();
                if multiline {
                    if let Op::Create(_, kind, _, _) = &mut ops[0] {
                        *kind = Kind::Textarea;
                    }
                    ops.push(Op::SetEditorFrame(
                        id(0),
                        Some(gpuio_protocol::editor_frame::Config {
                            clear_label: None,
                            ..frame(false)
                        }),
                    ));
                    ops.push(Op::SetStyle(
                        id(0),
                        vec![
                            Style::Width(Length::Px(360.)),
                            Style::Height(Length::Px(120.)),
                        ],
                    ));
                }
                apply(view, window, cx, ops);
            });
            window.draw(cx).clear(cx);
        });
        cx.run_until_parked();
        for width in [360., 220.] {
            cx.update(|window, cx| {
                owner.update(cx, |view, cx| {
                    apply(
                        view,
                        window,
                        cx,
                        vec![Op::SetStyle(
                            id(0),
                            vec![
                                Style::Width(Length::Px(width)),
                                Style::Height(Length::Px(if multiline { 120. } else { 42. })),
                            ],
                        )],
                    )
                });
                window.draw(cx).clear(cx);
            });
            let tree = cx.a11y_tree().unwrap();
            let label_bounds = |label| {
                tree.nodes
                    .iter()
                    .find(|(_, n)| n.label() == Some(label))
                    .unwrap()
                    .1
                    .bounds()
                    .unwrap()
            };
            let prefix = label_bounds("Prefix");
            let suffix = label_bounds("Suffix");
            let scale = cx.update(|window, _| window.scale_factor()) as f64;
            let bounds = cx.update(|_, cx| owner.read(cx).editors[&id(0)].input_bounds(cx));
            assert!(
                f32::from(bounds.size.width) > 30.,
                "editor has width: {bounds:?}"
            );
            assert!(
                f32::from(bounds.size.height) > 10.,
                "editor has height: {bounds:?}"
            );
            if multiline {
                assert!(
                    f32::from(bounds.size.height) >= 100.,
                    "textarea fills declared height: {bounds:?}"
                );
            }
            assert!(
                (f32::from(bounds.left()) as f64) * scale >= prefix.x1,
                "prefix overlaps editor: {prefix:?} {bounds:?}"
            );
            assert!(
                (f32::from(bounds.right()) as f64) * scale <= suffix.x0,
                "editor overlaps suffix: {suffix:?} {bounds:?}"
            );
            assert!(
                suffix.x1 <= width * scale + 1.,
                "suffix exceeds frame: {suffix:?}"
            );
        }
        if multiline {
            let before = cx.update(|window, cx| {
                owner.update(cx, |view, cx| {
                    apply(
                        view,
                        window,
                        cx,
                        vec![
                            Op::SetEditor(
                                id(0),
                                EditorConfig {
                                    min_rows: 3,
                                    max_rows: 6,
                                    ..config(false, false)
                                },
                            ),
                            Op::SetStyle(id(0), vec![Style::Width(Length::Px(300.))]),
                        ],
                    );
                });
                window.draw(cx).clear(cx);
                owner.read(cx).editors[&id(0)].input_bounds(cx).size.height
            });
            assert!(
                f32::from(before) >= 70.,
                "auto-grow honors minimum rows: {before:?}"
            );
            cx.update(|window, cx| {
                owner.update(cx, |view, cx| {
                    command(
                        view,
                        EditorCommand::Replace(
                            "one\ntwo\nthree\nfour\nfive".into(),
                            EditorSelectionPolicy::End,
                            EditorUndoPolicy::Record,
                            None,
                        ),
                        window,
                        cx,
                    )
                });
                window.draw(cx).clear(cx);
                let after = owner.read(cx).editors[&id(0)].input_bounds(cx).size.height;
                assert!(after > before, "auto-grow expands: {before:?} -> {after:?}");
            });
        }
        if !multiline {
            let tree = cx.a11y_tree().unwrap();
            let clear = tree
                .nodes
                .iter()
                .find(|(_, n)| n.label() == Some("Clear draft"))
                .unwrap()
                .1
                .bounds()
                .unwrap();
            let scale = cx.update(|window, _| window.scale_factor()) as f64;
            let point = gpui::point(
                px(((clear.x0 + clear.x1) / (2. * scale)) as f32),
                px(((clear.y0 + clear.y1) / (2. * scale)) as f32),
            );
            cx.simulate_mouse_down(point, gpui::MouseButton::Left, gpui::Modifiers::default());
            cx.simulate_mouse_up(point, gpui::MouseButton::Left, gpui::Modifiers::default());
            cx.update(|window, cx| {
                let state = owner.read(cx).editors[&id(0)].snapshot(window, cx);
                assert_eq!(state.text, "");
                assert!(state.focused);
            });
        }
    }
}

#[test]
fn rejected_frame_clear_preserves_selection_revision_and_history() {
    use gpuio_protocol::input_validation::{Matching, Rule, Source};
    let mut app = TestAppContext::single();
    app.update(gpui_base::init);
    let (_reader, writer) = UnixStream::pair().unwrap();
    let transport = Arc::new(Transport::new(writer.as_raw_fd()).unwrap());
    let session = Rc::new(RefCell::new(Session::default()));
    let wid = WindowId::from_parts(0, 1).unwrap();
    session.borrow_mut().hello(VERSION, CAPABILITIES).unwrap();
    session
        .borrow_mut()
        .open(1, wid, "Guarded clear", 500., 240.)
        .unwrap();
    let (owner, cx) = app.add_window_view(|_, _| View::new(wid, session, transport));
    cx.update(|window, cx| {
        owner.update(cx, |view, cx| {
            apply(view, window, cx, mount());
            command(view, EditorCommand::Focus, window, cx);
            command(
                view,
                EditorCommand::Replace(
                    "edited λ".into(),
                    EditorSelectionPolicy::Start,
                    EditorUndoPolicy::Record,
                    None,
                ),
                window,
                cx,
            );
            command(
                view,
                EditorCommand::Select(EditorSelection { anchor: 4, head: 1 }),
                window,
                cx,
            );
            apply(
                view,
                window,
                cx,
                vec![Op::SetEditorValidation(
                    id(0),
                    Some(Rule {
                        regex: Source {
                            pattern: ".+".into(),
                            matching: Matching::WholeValue,
                            case_sensitive: true,
                        },
                        allow_empty: false,
                    }),
                )],
            );
        });
        window.draw(cx).clear(cx);
    });
    cx.update(|window, cx| {
        owner.update(cx, |view, cx| {
            let before = view.editors[&id(0)].snapshot(window, cx);
            let clear = target(view, cx).unwrap();
            view.clear_input(&clear, window, cx);
            assert_eq!(view.editors[&id(0)].snapshot(window, cx), before);
            command(view, EditorCommand::Undo, window, cx);
            assert_eq!(view.editors[&id(0)].snapshot(window, cx).text, "seed λ");
            command(view, EditorCommand::Redo, window, cx);
            assert_eq!(view.editors[&id(0)].snapshot(window, cx).text, "edited λ");
            apply(view, window, cx, vec![Op::SetEditorValidation(id(0), None)]);
            let clear = target(view, cx).unwrap();
            view.clear_input(&clear, window, cx);
            assert_eq!(view.editors[&id(0)].snapshot(window, cx).text, "");
            command(view, EditorCommand::Undo, window, cx);
            assert_eq!(view.editors[&id(0)].snapshot(window, cx).text, "edited λ");
        })
    });
}
