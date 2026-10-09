//! Rendered TestPlatform layout; no physical macOS input or visual acceptance.
use super::super::View;
use super::*;
use crate::session::Session;
use gpui::{TestAppContext, VisualTestContext};
use gpuio_protocol::{HandlerId, text_area_layout::Config};
use std::{os::fd::AsRawFd, os::unix::net::UnixStream};

fn id(slot: i64) -> NodeId {
    NodeId::from_parts(slot, 1).unwrap()
}
fn editor_config() -> EditorConfig {
    EditorConfig {
        label: "Notes".into(),
        placeholder: "".into(),
        read_only: false,
        disabled: false,
        submit_on_enter: false,
        auto_focus: false,
        min_rows: 8,
        max_rows: 8,
    }
}
pub(super) fn apply(owner: &Entity<View>, cx: &mut VisualTestContext, operations: Vec<Op>) {
    cx.update(|window, cx| {
        owner.update(cx, |view, cx| {
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
        });
        window.draw(cx).clear(cx);
    });
    cx.run_until_parked();
}
pub(super) fn state(owner: &Entity<View>, cx: &mut VisualTestContext) -> Entity<TextareaState> {
    cx.update(|_, cx| {
        let State::Textarea(state) = &owner.read(cx).editors[&id(1)].state else {
            unreachable!()
        };
        state.clone()
    })
}
fn draw(cx: &mut VisualTestContext) {
    cx.update(|window, cx| window.draw(cx).clear(cx));
    cx.run_until_parked();
}
pub(super) fn setup_kind(
    kind: Kind,
    text: &str,
    test: impl FnOnce(Entity<View>, &mut VisualTestContext),
) {
    let mut app = TestAppContext::single();
    app.update(gpui_base::init);
    let (_reader, writer) = UnixStream::pair().unwrap();
    let transport = Arc::new(Transport::new(writer.as_raw_fd()).unwrap());
    let session = Rc::new(RefCell::new(Session::default()));
    let wid = WindowId::from_parts(0, 1).unwrap();
    session.borrow_mut().hello(VERSION, CAPABILITIES).unwrap();
    session
        .borrow_mut()
        .open(1, wid, "Textarea layout", 500., 400.)
        .unwrap();
    let (owner, cx) = app.add_window_view(|_, _| View::new(wid, session, transport));
    apply(
        &owner,
        cx,
        vec![
            Op::Create(id(0), Kind::Container, "".into(), None),
            Op::Create(
                id(1),
                kind,
                text.into(),
                Some(HandlerId::from_parts(1, 1).unwrap()),
            ),
            Op::SetEditor(
                id(1),
                EditorConfig {
                    min_rows: if kind == Kind::Input { 1 } else { 8 },
                    max_rows: if kind == Kind::Input { 1 } else { 8 },
                    ..editor_config()
                },
            ),
            Op::SetStyle(
                id(1),
                vec![
                    Style::Width(Length::Px(240.)),
                    Style::Height(Length::Px(200.)),
                ],
            ),
            Op::Splice(id(0), 0, 0, vec![id(1)]),
            Op::SetRoot(Some(id(0))),
        ],
    );
    cx.update(|window, cx| {
        owner.update(cx, |view, cx| {
            let editor = view.editors.get_mut(&id(1)).unwrap();
            assert!(matches!(
                editor.command(&EditorCommand::Focus, window, cx),
                EditorResult::Applied(_)
            ));
            assert!(matches!(
                editor.command(
                    &EditorCommand::Select(EditorSelection { anchor: 0, head: 0 }),
                    window,
                    cx
                ),
                EditorResult::Applied(_)
            ));
        })
    });
    draw(cx);
    test(owner, cx);
}
fn set_layout(owner: &Entity<View>, cx: &mut VisualTestContext, layout: Option<Config>) {
    apply(owner, cx, vec![Op::SetTextAreaLayout(id(1), layout)]);
}

pub(super) fn setup(text: &str, test: impl FnOnce(Entity<View>, &mut VisualTestContext)) {
    setup_kind(Kind::Textarea, text, test);
}

pub(super) fn request(
    owner: &Entity<View>,
    cx: &mut VisualTestContext,
    command: EditorCommand,
) -> EditorResult {
    cx.update(|window, cx| {
        owner.update(cx, |view, cx| {
            view.editors
                .get_mut(&id(1))
                .unwrap()
                .command(&command, window, cx)
        })
    })
}
fn viewport(
    owner: &Entity<View>,
    cx: &mut VisualTestContext,
) -> gpuio_protocol::editor_viewport::Snapshot {
    match request(owner, cx, EditorCommand::ReadViewport) {
        EditorResult::Viewport(Some(value)) => value,
        other => panic!("expected laid out viewport: {other:?}"),
    }
}
fn snapshot(owner: &Entity<View>, cx: &mut VisualTestContext) -> EditorSnapshot {
    cx.update(|window, cx| owner.read(cx).editors[&id(1)].snapshot(window, cx))
}

#[test]
fn editor_viewport_scroll_acknowledges_request_then_reports_clamped_layout_without_editing() {
    use gpuio_protocol::editor_viewport::Offset;
    let text = (0..60)
        .map(|n| format!("line {n:02}"))
        .collect::<Vec<_>>()
        .join("\n");
    setup(&text, |owner, cx| {
        let first = viewport(&owner, cx);
        assert_eq!(first.offset, Offset { x: 0., y: 0. });
        assert_eq!(first.first_buffer_line, 0);
        assert!(first.width > 100. && first.height > 100. && first.line_height > 0.);
        cx.update(|window, cx| {
            owner.update(cx, |view, cx| {
                view.editors[&id(1)].mark_test_text("mark", window, cx)
            })
        });
        draw(cx);
        let before = snapshot(&owner, cx);
        assert!(before.composition.is_some());
        let previous = viewport(&owner, cx);
        cx.update(|window, cx| {
            owner.update(cx, |view, cx| {
                let editor = view.editors.get_mut(&id(1)).unwrap();
                assert_eq!(
                    editor.command(
                        &EditorCommand::ScrollViewport(Offset { x: 0., y: 200. }),
                        window,
                        cx
                    ),
                    EditorResult::ViewportScrollAccepted
                );
                assert_eq!(
                    editor.command(&EditorCommand::ReadViewport, window, cx),
                    EditorResult::Viewport(Some(previous)),
                    "ack is not a layout completion"
                );
            })
        });
        draw(cx);
        let scrolled = viewport(&owner, cx);
        assert_eq!(
            scrolled.offset.y,
            200.,
            "initial={first:?} prior={previous:?} after={scrolled:?} textbytes={} selection={:?}",
            before.text.len(),
            before.selection
        );
        assert!(scrolled.first_buffer_line > 0);
        assert_eq!(snapshot(&owner, cx), before);
        for _ in 0..3 {
            draw(cx);
            assert_eq!(viewport(&owner, cx), scrolled);
        }
        assert_eq!(
            request(
                &owner,
                cx,
                EditorCommand::ScrollViewport(Offset { x: 1e9, y: 1e9 })
            ),
            EditorResult::ViewportScrollAccepted
        );
        draw(cx);
        let end = viewport(&owner, cx);
        assert!(end.offset.y > 200. && end.offset.y < 1e9);
        assert_eq!(end.offset.x, 0.);
        assert!(end.buffer_line_limit <= 60);
        assert_eq!(snapshot(&owner, cx), before);
        assert_eq!(
            request(
                &owner,
                cx,
                EditorCommand::ScrollViewport(Offset { x: f64::NAN, y: 0. })
            ),
            EditorResult::Failed(EditorError::NativeFailure)
        );
        assert_eq!(viewport(&owner, cx), end);
        // Multiple requests before layout coalesce in the native scroll slot.
        cx.update(|window, cx| {
            owner.update(cx, |view, cx| {
                let editor = view.editors.get_mut(&id(1)).unwrap();
                for y in [300., 100., 0.] {
                    assert_eq!(
                        editor.command(
                            &EditorCommand::ScrollViewport(Offset { x: 0., y }),
                            window,
                            cx
                        ),
                        EditorResult::ViewportScrollAccepted
                    );
                }
            })
        });
        draw(cx);
        assert_eq!(viewport(&owner, cx).offset.y, 0.);
        assert_eq!(snapshot(&owner, cx), before);
    });
}

#[test]
fn editor_viewport_counts_logical_lines_and_scrolls_readonly_disabled_without_focusing() {
    use gpuio_protocol::editor_viewport::Offset;
    setup(&"word ".repeat(200), |owner, cx| {
        let field = state(&owner, cx);
        let first = viewport(&owner, cx);
        assert_eq!(first.first_buffer_line, 0);
        assert_eq!(
            first.buffer_line_limit, 1,
            "soft wrap does not create logical lines"
        );
        let mut config = editor_config();
        config.read_only = true;
        config.disabled = true;
        apply(&owner, cx, vec![Op::SetEditor(id(1), config)]);
        let before = snapshot(&owner, cx);
        assert!(!before.focused);
        set_layout(
            &owner,
            cx,
            Some(Config {
                soft_wrap: false,
                ..Config::default()
            }),
        );
        assert_eq!(
            request(
                &owner,
                cx,
                EditorCommand::ScrollViewport(Offset { x: 100., y: 500. })
            ),
            EditorResult::ViewportScrollAccepted
        );
        draw(cx);
        let value = viewport(&owner, cx);
        assert_eq!(value.offset.x, 100.);
        assert_eq!(value.offset.y, 0.);
        assert_eq!(snapshot(&owner, cx), before);
        // Keep the old native entity alive but retire its tree identity.
        cx.update(|window, cx| {
            owner.update(cx, |view, cx| {
                let base = view.session.borrow().tree(view.id).unwrap().revision();
                view.session
                    .borrow_mut()
                    .apply(&Transaction {
                        window: view.id,
                        base,
                        revision: base + 1,
                        operations: vec![Op::Splice(id(0), 0, 1, vec![]), Op::Remove(id(1))],
                    })
                    .unwrap();
                let editor = view.editors.get_mut(&id(1)).unwrap();
                assert_eq!(
                    editor.command(&EditorCommand::ReadViewport, window, cx),
                    EditorResult::Failed(EditorError::StaleEditor)
                );
                assert_eq!(
                    editor.command(
                        &EditorCommand::ScrollViewport(Offset { x: 0., y: 0. }),
                        window,
                        cx
                    ),
                    EditorResult::Failed(EditorError::StaleEditor)
                );
            })
        });
        assert_eq!(
            cx.update(|_, cx| field.read(cx).scroll_offset().x),
            gpui::px(-100.)
        );
    });
}

#[test]
fn editor_viewport_before_first_layout_is_explicitly_absent() {
    let mut app = TestAppContext::single();
    app.update(gpui_base::init);
    let (_, cx) = app.add_window_view(TextareaState::new);
    cx.update(|window, cx| {
        let state = cx.new(|cx| TextareaState::new(window, cx));
        state.update(cx, |state, cx| {
            assert_eq!(
                super::viewport::command(state, &EditorCommand::ReadViewport, cx),
                EditorResult::Viewport(None)
            );
            assert_eq!(
                super::viewport::command(
                    state,
                    &EditorCommand::ScrollViewport(gpuio_protocol::editor_viewport::Offset {
                        x: 0.,
                        y: 50.
                    }),
                    cx
                ),
                EditorResult::ViewportScrollAccepted
            );
            assert_eq!(
                super::viewport::command(state, &EditorCommand::ReadViewport, cx),
                EditorResult::Viewport(None)
            );
        });
    });
}

#[test]
fn editor_viewport_masked_unicode_scroll_survives_repeated_draws_without_caret_motion() {
    use gpuio_protocol::editor_viewport::Offset;
    setup_kind(Kind::Input, &"界λ👩".repeat(50), |owner, cx| {
        apply(
            &owner,
            cx,
            vec![Op::SetEditorPrivacy(id(1), EditorPrivacy::PasswordHidden)],
        );
        assert!(matches!(
            request(
                &owner,
                cx,
                EditorCommand::Select(EditorSelection { anchor: 5, head: 5 })
            ),
            EditorResult::Applied(_)
        ));
        draw(cx);
        let before = snapshot(&owner, cx);
        assert_eq!(
            request(
                &owner,
                cx,
                EditorCommand::ScrollViewport(Offset { x: 100., y: 0. })
            ),
            EditorResult::ViewportScrollAccepted
        );
        for _ in 0..4 {
            draw(cx);
            assert_eq!(viewport(&owner, cx).offset.x, 100.);
            assert_eq!(snapshot(&owner, cx), before);
        }
    });
}

#[test]
fn editor_viewport_read_keeps_geometry_coherent_between_wheel_input_and_layout() {
    setup(&"line\n".repeat(60), |owner, cx| {
        let field = state(&owner, cx);
        let before = viewport(&owner, cx);
        cx.update(|window, cx| {
            let position = field.read(cx).input_bounds().center();
            window.dispatch_event(
                gpui::PlatformInput::MouseMove(gpui::MouseMoveEvent {
                    position,
                    pressed_button: None,
                    modifiers: Default::default(),
                }),
                cx,
            );
            window.dispatch_event(
                gpui::PlatformInput::ScrollWheel(gpui::ScrollWheelEvent {
                    position,
                    delta: gpui::ScrollDelta::Pixels(gpui::point(gpui::px(0.), gpui::px(-100.))),
                    touch_phase: gpui::TouchPhase::Moved,
                    modifiers: Default::default(),
                }),
                cx,
            );
            assert_eq!(
                field.read(cx).scroll_offset().y,
                gpui::px(-100.),
                "wheel changed live handle before layout"
            );
            owner.update(cx, |view, cx| {
                assert_eq!(
                    view.editors.get_mut(&id(1)).unwrap().command(
                        &EditorCommand::ReadViewport,
                        window,
                        cx
                    ),
                    EditorResult::Viewport(Some(before)),
                    "metadata must not mix live offset and old geometry"
                );
            });
        });
        draw(cx);
        let after = viewport(&owner, cx);
        assert_eq!(after.offset.y, 100.);
        assert!(after.first_buffer_line > before.first_buffer_line);
    });
}
