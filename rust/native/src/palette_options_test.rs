//! Retained production palette behavior on TestPlatform; not OS input evidence.
use super::*;
use crate::{session::Session, transport::Transport};
use gpui::{TestAppContext, VisualTestContext, WindowOptions};
use gpuio_protocol::{HandlerId, WindowId};
use std::os::{fd::AsRawFd, unix::net::UnixStream};
fn id(slot: i64) -> NodeId {
    NodeId::from_parts(slot, 1).unwrap()
}
fn draw(cx: &mut VisualTestContext) {
    cx.run_until_parked();
    cx.update(|w, cx| w.draw(cx).clear(cx));
    cx.run_until_parked();
}
fn admit(owner: &Entity<View>, cx: &mut VisualTestContext, operations: Vec<Op>) {
    cx.update(|w, cx| {
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
            view.update_editors(&applied.dirty, w, cx);
            cx.notify();
        })
    });
}
fn apply(owner: &Entity<View>, cx: &mut VisualTestContext, operations: Vec<Op>) {
    admit(owner, cx, operations);
    draw(cx);
}
fn commands() -> Vec<CommandConfig> {
    [
        ("run", "Run task", true),
        ("other", "Other", true),
        ("disabled", "Unavailable", false),
    ]
    .into_iter()
    .map(|(id, label, enabled)| CommandConfig {
        id: id.into(),
        label: label.into(),
        enabled,
        generation: 1,
        checked: None,
        shortcuts: vec![],
        target: CommandTarget::Callback,
    })
    .collect()
}
fn mount_extra(
    app: &mut TestAppContext,
    extra: Vec<Op>,
) -> (Entity<View>, VisualTestContext, UnixStream) {
    app.update(gpui_base::init);
    let (reader, writer) = UnixStream::pair().unwrap();
    let transport = Arc::new(Transport::new(writer.as_raw_fd()).unwrap());
    let session = Rc::new(RefCell::new(Session::default()));
    let window = WindowId::from_parts(0, 1).unwrap();
    session.borrow_mut().hello(VERSION, CAPABILITIES).unwrap();
    session
        .borrow_mut()
        .open(1, window, "Palette policies", 800., 600.)
        .unwrap();
    let handle = app.update(|cx| {
        cx.open_window(WindowOptions::default(), |_, cx| {
            cx.new(|_| View::new(window, session, transport))
        })
        .unwrap()
    });
    let owner = handle.root(app).unwrap();
    let mut cx = VisualTestContext::from_window(handle.into(), app);
    cx.simulate_resize(gpui::size(px(800.), px(600.)));
    let mut operations = vec![
        Op::Create(
            id(0),
            Kind::CommandScope,
            "".into(),
            Some(HandlerId::from_parts(0, 1).unwrap()),
        ),
        Op::SetCommands(id(0), commands()),
        Op::Create(
            id(1),
            Kind::CommandPalette,
            "".into(),
            Some(HandlerId::from_parts(1, 1).unwrap()),
        ),
        Op::SetPalette(
            id(1),
            PaletteConfig {
                label: "Actions".into(),
                placeholder: "Find".into(),
                commands: commands().into_iter().map(|c| c.id).collect(),
                dismiss_on_outside_pointer: true,
            },
        ),
        Op::Splice(id(0), 0, 0, vec![id(1)]),
        Op::SetRoot(Some(id(0))),
    ];
    operations.extend(extra);
    apply(&owner, &mut cx, operations);
    (owner, cx, reader)
}
fn mount(app: &mut TestAppContext) -> (Entity<View>, VisualTestContext, UnixStream) {
    mount_extra(app, vec![])
}
fn query(owner: &Entity<View>, cx: &mut VisualTestContext, text: &str) {
    cx.update(|w, cx| {
        owner.update(cx, |view, cx| {
            view.palettes[&id(1)]
                .query
                .update(cx, |input, cx| input.set_value(text, w, cx))
        })
    });
    draw(cx);
}
fn rows(owner: &Entity<View>, cx: &VisualTestContext) -> Vec<String> {
    owner.read_with(cx, |view, _| {
        view.palettes[&id(1)]
            .rows
            .iter()
            .map(|row| row.route.config.id.clone())
            .collect()
    })
}
fn options(searchable: bool) -> palette_options::Config {
    palette_options::Config {
        search: palette_options::Search::Substring,
        searchable,
        escape: palette_options::Escape::ClearQueryFirst,
        keywords: vec![palette_options::Keywords {
            command: "run".into(),
            words: vec!["Execute λ".into()],
        }],
    }
}
#[test]
fn policies_preserve_query_owner_and_transfer_focus_when_query_is_hidden() {
    let mut app = TestAppContext::single();
    let (owner, mut cx, _reader) = mount(&mut app);
    let input = owner.read_with(&cx, |view, _| view.palettes[&id(1)].query.clone());
    cx.update(|w, cx| w.focus(&input.read(cx).focus_handle(cx), cx));
    query(&owner, &mut cx, "  EXECUTE λ  ");
    assert!(rows(&owner, &cx).is_empty());
    apply(
        &owner,
        &mut cx,
        vec![Op::SetPaletteOptions(id(1), Some(options(true)))],
    );
    assert_eq!(rows(&owner, &cx), vec!["run"]);
    apply(
        &owner,
        &mut cx,
        vec![Op::SetPaletteOptions(id(1), Some(options(false)))],
    );
    assert_eq!(rows(&owner, &cx), vec!["run", "other", "disabled"]);
    cx.update(|w, cx| {
        owner.update(cx, |view, cx| {
            assert_eq!(view.palettes[&id(1)].query.entity_id(), input.entity_id());
            assert_eq!(input.read(cx).value().as_str(), "  EXECUTE λ  ");
            assert!(view.focus.borrow().handle(id(1)).unwrap().is_focused(w));
            assert!(!input.read(cx).focus_handle(cx).is_focused(w));
        })
    });
    apply(
        &owner,
        &mut cx,
        vec![Op::SetPaletteOptions(id(1), Some(options(true)))],
    );
    cx.update(|w, cx| assert!(input.read(cx).focus_handle(cx).is_focused(w)));
    assert_eq!(rows(&owner, &cx), vec!["run"]);
    // Use GPUI key dispatch rather than calling the palette callback directly.
    cx.simulate_event(gpui::KeyDownEvent {
        keystroke: gpui::Keystroke::parse("escape").unwrap(),
        is_held: false,
        prefer_character_input: false,
    });
    draw(&mut cx);
    owner.read_with(&cx, |view, cx| {
        assert!(!view.palettes[&id(1)].closed);
        assert!(input.read(cx).value().is_empty());
    });
    assert_eq!(rows(&owner, &cx), vec!["run", "other", "disabled"]);
    cx.simulate_event(gpui::KeyDownEvent {
        keystroke: gpui::Keystroke::parse("escape").unwrap(),
        is_held: false,
        prefer_character_input: false,
    });
    draw(&mut cx);
    owner.read_with(&cx, |view, _| assert!(view.palettes[&id(1)].closed));
}
#[test]
fn invalid_options_are_atomic_and_hidden_palette_navigates_enabled_registry_rows() {
    let mut app = TestAppContext::single();
    let (owner, mut cx, _reader) = mount(&mut app);
    let initial_bytes = owner.read_with(&cx, |view, _| {
        view.session
            .borrow()
            .tree(view.id)
            .unwrap()
            .retained_bytes()
    });
    let mut invalid = options(false);
    invalid.keywords[0].command = "missing".into();
    cx.update(|_, cx| {
        owner.update(cx, |view, _| {
            let base = view.session.borrow().tree(view.id).unwrap().revision();
            assert!(
                view.session
                    .borrow_mut()
                    .apply(&Transaction {
                        window: view.id,
                        base,
                        revision: base + 1,
                        operations: vec![Op::SetPaletteOptions(id(1), Some(invalid))]
                    })
                    .is_err()
            );
            let session = view.session.borrow();
            let tree = session.tree(view.id).unwrap();
            assert_eq!(tree.revision(), base);
            assert!(tree.get(id(1)).unwrap().palette_options.is_none());
        })
    });
    apply(
        &owner,
        &mut cx,
        vec![Op::SetPaletteOptions(id(1), Some(options(false)))],
    );
    owner.read_with(&cx, |view, _| {
        assert!(
            view.session
                .borrow()
                .tree(view.id)
                .unwrap()
                .retained_bytes()
                >= initial_bytes + options(false).retained_bytes()
        );
    });
    for expected in ["other", "run"] {
        cx.simulate_event(gpui::KeyDownEvent {
            keystroke: gpui::Keystroke::parse("down").unwrap(),
            is_held: false,
            prefer_character_input: false,
        });
        draw(&mut cx);
        owner.read_with(&cx, |view, _| {
            assert_eq!(view.palettes[&id(1)].selected.as_deref(), Some(expected))
        });
    }
    let mut updated = commands();
    updated[0].enabled = false;
    apply(&owner, &mut cx, vec![Op::SetCommands(id(0), updated)]);
    owner.read_with(&cx, |view, _| {
        assert_eq!(view.palettes[&id(1)].selected.as_deref(), Some("other"))
    });
    apply(&owner, &mut cx, vec![Op::SetPaletteOptions(id(1), None)]);
    owner.read_with(&cx, |view, _| {
        assert_eq!(
            view.session
                .borrow()
                .tree(view.id)
                .unwrap()
                .retained_bytes(),
            initial_bytes
        )
    });
    query(&owner, &mut cx, "RUN");
    assert_eq!(rows(&owner, &cx), vec!["run"]);
    owner.read_with(&cx, |view, _| {
        assert!(view.palettes[&id(1)].selected.is_none())
    });
}

#[test]
fn composition_escape_precedes_clear_and_hidden_query_escape_dismisses() {
    let mut app = TestAppContext::single();
    let (owner, mut cx, _reader) = mount(&mut app);
    apply(
        &owner,
        &mut cx,
        vec![Op::SetPaletteOptions(id(1), Some(options(true)))],
    );
    let input = owner.read_with(&cx, |view, _| view.palettes[&id(1)].query.clone());
    cx.update(|w, cx| {
        w.focus(&input.read(cx).focus_handle(cx), cx);
        input.update(cx, |state, cx| {
            state.replace_and_mark_text_in_range(None, "λ", Some(0..1), w, cx)
        });
    });
    draw(&mut cx);
    owner.read_with(&cx, |view, cx| assert!(view.palettes[&id(1)].composing(cx)));
    cx.simulate_event(gpui::KeyDownEvent {
        keystroke: gpui::Keystroke::parse("escape").unwrap(),
        is_held: false,
        prefer_character_input: false,
    });
    draw(&mut cx);
    owner.read_with(&cx, |view, cx| {
        assert!(!view.palettes[&id(1)].closed);
        assert!(!view.palettes[&id(1)].composing(cx));
    });
    query(&owner, &mut cx, "retained");
    apply(
        &owner,
        &mut cx,
        vec![Op::SetPaletteOptions(id(1), Some(options(false)))],
    );
    cx.simulate_event(gpui::KeyDownEvent {
        keystroke: gpui::Keystroke::parse("escape").unwrap(),
        is_held: false,
        prefer_character_input: false,
    });
    draw(&mut cx);
    owner.read_with(&cx, |view, cx| {
        assert!(view.palettes[&id(1)].closed);
        assert_eq!(input.read(cx).value().as_str(), "retained");
    });
}

#[test]
fn options_on_a_hidden_retired_palette_do_not_require_a_focus_scope_or_reopen_it() {
    let mut app = TestAppContext::single();
    let (owner, mut cx, _reader) = mount(&mut app);
    let input = owner.read_with(&cx, |view, _| view.palettes[&id(1)].query.clone());
    query(&owner, &mut cx, "retained query");
    apply(
        &owner,
        &mut cx,
        vec![Op::SetStyle(
            id(1),
            vec![Style::Fields(vec![Field::Display(3)])],
        )],
    );
    owner.read_with(&cx, |view, _| {
        assert!(view.focus.borrow().handle(id(1)).is_none());
        assert!(view.palettes[&id(1)].closed);
    });
    for searchable in [false, true] {
        apply(
            &owner,
            &mut cx,
            vec![Op::SetPaletteOptions(id(1), Some(options(searchable)))],
        );
        owner.read_with(&cx, |view, cx| {
            assert_eq!(view.palettes[&id(1)].query.entity_id(), input.entity_id());
            assert!(view.palettes[&id(1)].closed);
            assert!(view.focus.borrow().handle(id(1)).is_none());
            assert_eq!(input.read(cx).value().as_str(), "retained query");
        });
    }
}

#[test]
fn grouped_layout_retains_query_filters_headings_and_skips_passive_rows() {
    use gpuio_protocol::palette_layout::{Config, Entry};
    let mut app = TestAppContext::single();
    let (owner, mut cx, _reader) = mount(&mut app);
    let input = owner.read_with(&cx, |view, _| view.palettes[&id(1)].query.clone());
    let layout = Config(vec![
        Entry::Separator,
        Entry::Group("tasks".into(), Some("Tasks".into()), vec![0]),
        Entry::Separator,
        Entry::Separator,
        Entry::Group("other".into(), Some("Other commands".into()), vec![1, 2]),
        Entry::Separator,
    ]);
    apply(
        &owner,
        &mut cx,
        vec![Op::SetPaletteLayout(id(1), Some(layout.clone()))],
    );
    owner.read_with(&cx, |view, _| {
        let state = &view.palettes[&id(1)];
        assert_eq!(state.query.entity_id(), input.entity_id());
        assert_eq!(state.scroll.rows().len(), 6);
        let bounds = |i| state.scroll.handle.bounds_for_item(i).unwrap();
        assert!(
            bounds(2).size.height < bounds(1).size.height,
            "divider has measured short height"
        );
    });
    cx.update(|w, cx| w.focus(&input.read(cx).focus_handle(cx), cx));
    cx.simulate_event(gpui::KeyDownEvent {
        keystroke: gpui::Keystroke::parse("down").unwrap(),
        is_held: false,
        prefer_character_input: false,
    });
    draw(&mut cx);
    owner.read_with(&cx, |view, _| {
        assert_eq!(view.palettes[&id(1)].selected.as_deref(), Some("other"))
    });
    query(&owner, &mut cx, "Run");
    owner.read_with(&cx, |view, _| {
        let rows = view.palettes[&id(1)].scroll.rows();
        assert_eq!(rows.len(), 2);
        assert!(matches!(&rows[0],list::Row::Heading {id,..} if id=="tasks"));
    });
    query(&owner, &mut cx, "missing");
    owner.read_with(&cx, |view, _| {
        assert!(view.palettes[&id(1)].scroll.rows().is_empty())
    });
    query(&owner, &mut cx, "Run");
    // No paint: an occluded window must also retire the old group projection.
    admit(&owner, &mut cx, vec![Op::SetPaletteLayout(id(1), None)]);
    owner.read_with(&cx, |view, cx| {
        let state = &view.palettes[&id(1)];
        assert_eq!(state.query.entity_id(), input.entity_id());
        assert_eq!(input.read(cx).value().as_str(), "Run");
        assert_eq!(state.scroll.rows().len(), 1);
    });
    // A well-formed layout cannot silently omit configured registry commands.
    cx.update(|_, cx| {
        owner.update(cx, |view, _| {
            let mut session = view.session.borrow_mut();
            let revision = session.tree(view.id).unwrap().revision();
            let result = session.apply(&Transaction {
                window: view.id,
                base: revision,
                revision: revision + 1,
                operations: vec![Op::SetPaletteLayout(
                    id(1),
                    Some(Config(vec![Entry::Command(0)])),
                )],
            });
            assert!(result.is_err());
            let tree = session.tree(view.id).unwrap();
            assert_eq!(tree.revision(), revision);
            assert!(tree.get(id(1)).unwrap().palette_layout.is_none());
        })
    });
}

fn content_operations() -> Vec<Op> {
    let mut operations = (2..8)
        .map(|slot| Op::Create(id(slot), Kind::Container, "".into(), None))
        .collect::<Vec<_>>();
    operations.extend([
        Op::Create(
            id(8),
            Kind::Input,
            "draft".into(),
            Some(HandlerId::from_parts(8, 1).unwrap()),
        ),
        Op::SetEditor(
            id(8),
            EditorConfig {
                label: "Header note".into(),
                placeholder: "".into(),
                read_only: false,
                disabled: false,
                submit_on_enter: true,
                auto_focus: false,
                min_rows: 1,
                max_rows: 1,
            },
        ),
        Op::Create(
            id(9),
            Kind::Button,
            "Help".into(),
            Some(HandlerId::from_parts(9, 1).unwrap()),
        ),
        Op::Create(
            id(10),
            Kind::Button,
            "Retry".into(),
            Some(HandlerId::from_parts(10, 1).unwrap()),
        ),
        Op::Create(id(11), Kind::Container, "".into(), None),
        Op::SetStyle(
            id(11),
            vec![Style::Fields(vec![
                Field::Height(Length::Px(100.)),
                Field::Width(Length::Px(180.)),
                Field::Background(Fill::Solid(Color::Rgba(0xff00ffff))),
            ])],
        ),
        Op::Create(
            id(12),
            Kind::Text,
            "Decorative command details".into(),
            None,
        ),
        Op::Splice(id(11), 0, 0, vec![id(12)]),
        Op::Splice(id(2), 0, 0, vec![id(8)]),
        Op::Splice(id(3), 0, 0, vec![id(9)]),
        Op::Splice(id(4), 0, 0, vec![id(10)]),
        Op::Splice(id(5), 0, 0, vec![id(11)]),
        Op::Splice(id(1), 0, 0, (2..8).map(id).collect()),
    ]);
    operations
}
fn mount_content(owner: &Entity<View>, cx: &mut VisualTestContext) {
    apply(owner, cx, content_operations());
}

#[test]
fn rich_rows_are_measured_and_header_editor_owns_enter_and_composition_escape() {
    let mut app = TestAppContext::single();
    let (owner, mut cx, _reader) = mount_extra(&mut app, content_operations());
    cx.update(|window, cx| {
        window.simulate_next_frame(cx);
    });
    draw(&mut cx);
    cx.update(|window, cx| {
        owner.read_with(cx, |view, cx| {
            assert!(
                view.palettes[&id(1)]
                    .query
                    .read(cx)
                    .focus_handle(cx)
                    .is_focused(window),
                "palette opens on query before header controls: header={}, scope={}",
                view.editors[&id(8)].focus_handle(cx).is_focused(window),
                view.focus
                    .borrow()
                    .handle(id(1))
                    .unwrap()
                    .is_focused(window)
            );
        })
    });
    owner.read_with(&cx, |view, _| {
        let state = &view.palettes[&id(1)];
        assert!(state.scroll.handle.bounds_for_item(0).unwrap().size.height >= px(100.));
        assert!(
            !view.focus.borrow().visible(id(10)),
            "empty content is gated while results exist"
        );
    });
    query(&owner, &mut cx, "run");
    owner.read_with(&cx, |view, _| {
        assert!(
            view.palettes[&id(1)]
                .scroll
                .handle
                .viewport_bounds()
                .size
                .height
                >= px(100.)
        )
    });
    apply(
        &owner,
        &mut cx,
        vec![Op::SetStyle(
            id(11),
            vec![Style::Fields(vec![
                Field::Height(Length::Px(140.)),
                Field::Width(Length::Px(180.)),
            ])],
        )],
    );
    owner.read_with(&cx, |view, _| {
        let list = &view.palettes[&id(1)].scroll.handle;
        assert!(list.bounds_for_item(0).unwrap().size.height >= px(140.));
        assert!(list.viewport_bounds().size.height >= px(140.));
    });
    cx.update(|w, cx| {
        owner.update(cx, |view, cx| {
            w.focus(&view.editors[&id(8)].focus_handle(cx), cx);
        })
    });
    cx.simulate_keystrokes("enter");
    draw(&mut cx);
    owner.read_with(&cx, |view, _| assert!(!view.palettes[&id(1)].closed));
    cx.update(|w, cx| {
        owner.update(cx, |view, cx| {
            view.editors[&id(8)].mark_test_text("λ", w, cx)
        })
    });
    draw(&mut cx);
    cx.simulate_keystrokes("escape");
    draw(&mut cx);
    owner.read_with(&cx, |view, cx| {
        assert!(!view.editors[&id(8)].is_composing(cx));
        assert!(
            !view.palettes[&id(1)].closed,
            "child composition must consume the first Escape"
        );
    });
    cx.simulate_keystrokes("escape");
    draw(&mut cx);
    owner.read_with(&cx, |view, _| assert!(view.palettes[&id(1)].closed));
}

#[test]
fn empty_content_rejects_queued_accessibility_action_after_query_results_return() {
    let mut app = TestAppContext::single();
    let (owner, mut cx, _reader) = mount(&mut app);
    cx.simulate_a11y_active(true);
    mount_content(&owner, &mut cx);
    query(&owner, &mut cx, "missing");
    let tree = cx.a11y_tree().unwrap();
    let target = tree
        .nodes
        .iter()
        .find(|(_, node)| node.label() == Some("Retry"))
        .unwrap()
        .0;
    owner.read_with(&cx, |view, _| assert!(view.focus.borrow().visible(id(10))));
    let input = owner.read_with(&cx, |view, _| view.palettes[&id(1)].query.clone());
    cx.update(|w, cx| input.update(cx, |input, cx| input.set_value("run", w, cx)));
    cx.run_until_parked();
    owner.read_with(&cx, |view, _| {
        assert!(!view.focus.borrow().visible(id(10)));
        view.transport.mailbox.lock().unwrap().drain(128);
    });
    cx.simulate_a11y_action(gpui::accesskit::ActionRequest {
        action: gpui::accesskit::Action::Click,
        target_node: target,
        target_tree: gpui::accesskit::TreeId::ROOT,
        data: None,
    });
    cx.run_until_parked();
    owner.read_with(&cx, |view, _| {
        assert!(
            !view
                .transport
                .mailbox
                .lock()
                .unwrap()
                .drain(128)
                .iter()
                .any(|event| matches!(event,Event::Press(_,node,_,_) if *node==id(10)))
        )
    });
}

#[test]
fn palette_slots_reject_shape_and_descendant_handler_mutation_atomically() {
    let mut app = TestAppContext::single();
    let (owner, mut cx, _reader) = mount(&mut app);
    mount_content(&owner, &mut cx);
    for operations in [
        vec![Op::Splice(id(1), 0, 1, vec![])],
        vec![Op::Bind(
            id(12),
            Some(HandlerId::from_parts(12, 1).unwrap()),
        )],
    ] {
        cx.update(|_, cx| {
            owner.update(cx, |view, _| {
                let mut session = view.session.borrow_mut();
                let revision = session.tree(view.id).unwrap().revision();
                assert!(
                    session
                        .apply(&Transaction {
                            window: view.id,
                            base: revision,
                            revision: revision + 1,
                            operations
                        })
                        .is_err()
                );
                assert_eq!(session.tree(view.id).unwrap().revision(), revision);
            })
        });
    }
}

#[test]
fn filtered_rich_activity_stops_frames_and_retires_with_content() {
    let mut app = TestAppContext::single();
    let (owner, mut cx, _reader) = mount_extra(&mut app, content_operations());
    apply(
        &owner,
        &mut cx,
        vec![
            Op::Create(id(13), Kind::Loading, "".into(), None),
            Op::SetSpinner(
                id(13),
                gpuio_protocol::spinner::Config {
                    label: "Row activity".into(),
                    animated: true,
                    period_ms: 1000,
                    easing: gpuio_protocol::animation::Easing::Linear,
                    source: None,
                },
            ),
            Op::Splice(id(11), 1, 0, vec![id(13)]),
        ],
    );
    cx.update(|w, cx| {
        assert!(w.simulate_next_frame(cx) > 0);
        w.draw(cx).clear(cx);
    });
    query(&owner, &mut cx, "other");
    owner.read_with(&cx, |view, _| {
        assert!(!view.focus.borrow().visible(id(13)));
        assert_eq!(view.spinners.len(), 1);
    });
    cx.update(|w, cx| {
        w.simulate_next_frame(cx);
        w.draw(cx).clear(cx);
        assert_eq!(w.simulate_next_frame(cx), 0, "filtered row must be idle");
    });
    query(&owner, &mut cx, "run");
    cx.update(|w, cx| {
        assert!(
            w.simulate_next_frame(cx) > 0,
            "visible row resumes activity"
        );
        w.draw(cx).clear(cx);
    });
    apply(
        &owner,
        &mut cx,
        vec![Op::Splice(id(11), 1, 1, vec![]), Op::Remove(id(13))],
    );
    owner.read_with(&cx, |view, _| assert!(view.spinners.is_empty()));
}

#[test]
fn nested_header_dialog_consumes_escape_before_the_palette() {
    let mut app = TestAppContext::single();
    let (owner, mut cx, _reader) = mount_extra(&mut app, content_operations());
    apply(
        &owner,
        &mut cx,
        vec![
            Op::Create(
                id(13),
                Kind::FocusScope,
                "".into(),
                Some(HandlerId::from_parts(13, 1).unwrap()),
            ),
            Op::SetFocusScope(
                id(13),
                FocusScopeConfig {
                    trap: true,
                    auto_focus: true,
                    restore_focus: true,
                },
            ),
            Op::SetOverlay(
                id(13),
                Some(OverlayConfig {
                    kind: OverlayKind::Dialog,
                    label: "Palette help dialog".into(),
                    width: 300.,
                    dismiss_on_escape: true,
                    dismiss_on_outside_pointer: false,
                }),
            ),
            Op::Splice(id(2), 0, 1, vec![id(13)]),
            Op::Splice(id(13), 0, 0, vec![id(8)]),
        ],
    );
    cx.update(|w, cx| {
        w.simulate_next_frame(cx);
        let handle = owner.read_with(cx, |view, cx| view.editors[&id(8)].focus_handle(cx));
        w.focus(&handle, cx);
    });
    draw(&mut cx);
    owner.read_with(&cx, |view, _| {
        assert!(!view.focus.borrow().top_overlay(id(1)))
    });
    cx.simulate_keystrokes("escape");
    draw(&mut cx);
    owner.read_with(&cx, |view, _| {
        assert!(!view.palettes[&id(1)].closed);
        assert!(
            view.transport
                .mailbox
                .lock()
                .unwrap()
                .drain(128)
                .iter()
                .any(
                    |event| matches!(event, Event::OverlayDismissed(_, node, ..) if *node == id(13))
                )
        );
    });
}

#[test]
fn thousand_rich_rows_keep_a_bounded_viewport_and_anchor_during_updates() {
    let mut app = TestAppContext::single();
    let commands: Vec<_> = (0..1000)
        .map(|index| CommandConfig {
            id: format!("cmd-{index}"),
            label: format!("Command {index}"),
            enabled: true,
            generation: 1,
            checked: None,
            shortcuts: vec![],
            target: CommandTarget::Callback,
        })
        .collect();
    let config = PaletteConfig {
        label: "Rich commands".into(),
        placeholder: "Find".into(),
        commands: commands.iter().map(|c| c.id.clone()).collect(),
        dismiss_on_outside_pointer: true,
    };
    let mut ops = vec![
        Op::SetCommands(id(0), commands),
        Op::SetPalette(id(1), config),
    ];
    let mut slots = vec![];
    for index in 2..5 {
        ops.push(Op::Create(id(index), Kind::Container, "".into(), None));
        slots.push(id(index));
    }
    for index in 0..1000 {
        let slot = id(5 + 2 * index);
        let content = id(6 + 2 * index);
        ops.extend([
            Op::Create(slot, Kind::Container, "".into(), None),
            Op::Create(content, Kind::Text, format!("Details {index}"), None),
            Op::SetStyle(
                content,
                vec![Style::Height(Length::Px(40. + (index % 3) as f64 * 10.))],
            ),
            Op::Splice(slot, 0, 0, vec![content]),
        ]);
        slots.push(slot);
    }
    ops.push(Op::Splice(id(1), 0, 0, slots));
    let (owner, mut cx, _reader) = mount_extra(&mut app, ops);
    let list = owner.read_with(&cx, |view, _| {
        let state = &view.palettes[&id(1)];
        assert!(state.row_bounds.borrow().len() < 30);
        assert_eq!(state.scroll.handle.item_count(), 1000);
        state.scroll.handle.clone()
    });
    list.scroll_to(gpui::ListOffset {
        item_ix: 900,
        offset_in_item: px(7.),
    });
    draw(&mut cx);
    let before = list.logical_scroll_top();
    assert_eq!(before.item_ix, 900);
    apply(
        &owner,
        &mut cx,
        vec![Op::SetStyle(id(1806), vec![Style::Height(Length::Px(85.))])],
    );
    let after = list.logical_scroll_top();
    assert_eq!(after.item_ix, before.item_ix);
    assert_eq!(after.offset_in_item, before.offset_in_item);
    owner.read_with(&cx, |view, _| {
        let state = &view.palettes[&id(1)];
        let rows = state.row_bounds.borrow();
        assert!(rows.len() < 30);
        assert!(rows["cmd-900"].size.height >= px(85.));
    });
}

#[test]
fn palette_tab_order_follows_header_query_and_footer() {
    let mut app = TestAppContext::single();
    let (owner, mut cx, _reader) = mount_extra(&mut app, content_operations());
    cx.update(|w, cx| {
        w.simulate_next_frame(cx);
    });
    cx.simulate_a11y_active(true);
    draw(&mut cx);
    let tree = cx.a11y_tree().unwrap();
    let query_node = tree
        .nodes
        .iter()
        .find(|(_, node)| node.role() == gpui::accesskit::Role::EditableComboBox)
        .unwrap()
        .0;
    assert_eq!(tree.focus, query_node, "query accessibility focus on open");
    cx.simulate_keystrokes("tab");
    draw(&mut cx);
    cx.update(|w, cx| {
        owner.read_with(cx, |view, cx| {
            assert_eq!(
                view.focus.borrow().focused_node(w, cx),
                Some(id(9)),
                "Tab from query should reach footer"
            );
        });
    });
    cx.simulate_keystrokes("shift-tab");
    draw(&mut cx);
    cx.update(|w, cx| {
        owner.read_with(cx, |view, cx| {
            assert!(
                view.palettes[&id(1)]
                    .query
                    .read(cx)
                    .focus_handle(cx)
                    .is_focused(w)
            )
        });
    });
    cx.simulate_keystrokes("shift-tab");
    draw(&mut cx);
    cx.update(|w, cx| {
        owner.read_with(cx, |view, cx| {
            assert!(view.editors[&id(8)].focus_handle(cx).is_focused(w))
        });
    });
}

fn observations(
    owner: &Entity<View>,
    cx: &VisualTestContext,
) -> Vec<gpuio_protocol::palette_state::Snapshot> {
    owner.read_with(cx, |view, _| {
        view.transport
            .mailbox
            .lock()
            .unwrap()
            .drain(128)
            .into_iter()
            .filter_map(|e| {
                if let Event::PaletteObserved(_, _, _, _, s) = e {
                    Some(s)
                } else {
                    None
                }
            })
            .collect()
    })
}
#[test]
fn snapshot_edges_are_ordered_native_owned_and_reattach_without_query_reset() {
    let mut app = TestAppContext::single();
    let (owner, mut cx, _reader) = mount(&mut app);
    assert!(observations(&owner, &cx).is_empty());
    apply(&owner, &mut cx, vec![Op::SetPaletteObserved(id(1), true)]);
    let initial = observations(&owner, &cx);
    assert_eq!(initial.len(), 1);
    assert_eq!(initial[0].query, "");
    assert_eq!(initial[0].selected.as_deref(), Some("run"));
    assert_eq!(initial[0].matched_count, 3); // disabled commands still match
    let input = owner.read_with(&cx, |view, _| view.palettes[&id(1)].query.clone());
    cx.update(|w, cx| w.focus(&input.read(cx).focus_handle(cx), cx));
    draw(&mut cx);
    draw(&mut cx);
    assert!(
        observations(&owner, &cx).is_empty(),
        "focus/paint must not cause an event loop"
    );
    cx.update(|w, cx| {
        owner.update(cx, |view, cx| {
            assert!(view.palette_key(id(1), "down", w, cx))
        })
    });
    draw(&mut cx);
    let selected = observations(&owner, &cx);
    assert_eq!(selected.len(), 1);
    assert_eq!(selected[0].selected.as_deref(), Some("other"));
    assert_eq!(selected[0].query_revision, initial[0].query_revision);
    assert!(selected[0].sequence > initial[0].sequence);
    query(&owner, &mut cx, "Run");
    let changed = observations(&owner, &cx);
    assert_eq!(changed.len(), 1);
    assert_eq!(changed[0].query, "Run");
    assert_eq!(changed[0].matched_count, 1);
    assert!(changed[0].query_revision > selected[0].query_revision);
    query(&owner, &mut cx, "Run");
    let rewritten = observations(&owner, &cx);
    assert_eq!(
        rewritten.len(),
        1,
        "an accepted replacement advances editor identity even with the same text"
    );
    assert!(rewritten[0].query_revision > changed[0].query_revision);
    query(&owner, &mut cx, "none");
    query(&owner, &mut cx, "Run");
    let aba = observations(&owner, &cx);
    assert_eq!(aba.len(), 2);
    assert_eq!(aba[0].selected, None);
    assert_eq!(aba[0].matched_count, 0);
    assert!(aba[1].query_revision > changed[0].query_revision);
    apply(&owner, &mut cx, vec![Op::SetPaletteObserved(id(1), false)]);
    query(&owner, &mut cx, "other");
    assert!(observations(&owner, &cx).is_empty());
    apply(
        &owner,
        &mut cx,
        vec![
            Op::Bind(id(1), Some(HandlerId::from_parts(9, 2).unwrap())),
            Op::SetPaletteObserved(id(1), true),
        ],
    );
    let fresh = observations(&owner, &cx);
    assert_eq!(fresh.len(), 1);
    assert_eq!(fresh[0].query, "other");
    assert_eq!(fresh[0].selected.as_deref(), Some("other"));
    assert_eq!(
        owner.read_with(&cx, |view, _| view.palettes[&id(1)].query.entity_id()),
        input.entity_id()
    );
    assert!(fresh[0].query_revision > aba[1].query_revision);
    let mut empty_config = owner.read_with(&cx, |view, _| (*view.palettes[&id(1)].config).clone());
    empty_config.commands.clear();
    apply(&owner, &mut cx, vec![Op::SetPalette(id(1), empty_config)]);
    let empty = observations(&owner, &cx);
    assert_eq!(empty.len(), 1);
    assert_eq!(empty[0].selected, None);
    assert_eq!(empty[0].query_revision, fresh[0].query_revision);
    cx.update(|w, cx| {
        owner.update(cx, |view, cx| {
            view.close_palette(id(1), PaletteDismissal::Escape, w, cx)
        })
    });
    observations(&owner, &cx);
    query(&owner, &mut cx, "retired");
    assert!(observations(&owner, &cx).is_empty());
}
#[test]
fn composition_changes_query_identity_without_waiting_for_an_observer() {
    let mut app = TestAppContext::single();
    let (owner, mut cx, _reader) = mount_extra(&mut app, vec![Op::SetPaletteObserved(id(1), true)]);
    observations(&owner, &cx);
    let input = owner.read_with(&cx, |view, _| view.palettes[&id(1)].query.clone());
    cx.update(|w, cx| {
        w.focus(&input.read(cx).focus_handle(cx), cx);
        input.update(cx, |q, cx| {
            q.replace_and_mark_text_in_range(None, "λ", Some(0..1), w, cx)
        });
    });
    draw(&mut cx);
    let composing = observations(&owner, &cx);
    assert_eq!(composing.len(), 1);
    assert!(composing[0].composing);
    assert_eq!(composing[0].query, "λ");
    cx.update(|w, cx| input.update(cx, |q, cx| q.unmark_text(w, cx)));
    draw(&mut cx);
    let committed = observations(&owner, &cx);
    assert_eq!(committed.len(), 1);
    assert!(!committed[0].composing);
    assert_eq!(committed[0].query, "λ");
    assert!(committed[0].query_revision > composing[0].query_revision);
}

#[test]
fn coalesced_query_edits_do_not_reuse_identity_when_text_returns() {
    let mut app = TestAppContext::single();
    let (owner, mut cx, _reader) = mount_extra(&mut app, vec![Op::SetPaletteObserved(id(1), true)]);
    query(&owner, &mut cx, "Run");
    let before = observations(&owner, &cx).pop().unwrap();
    let input = owner.read_with(&cx, |view, _| view.palettes[&id(1)].query.clone());
    cx.update(|w, cx| {
        input.update(cx, |q, cx| {
            q.set_value("Other", w, cx);
            q.set_value("Run", w, cx);
        })
    });
    draw(&mut cx);
    let after = observations(&owner, &cx);
    assert!(
        !after.is_empty(),
        "coalesced notifications must not reuse an obsolete query identity"
    );
    assert_eq!(after.last().unwrap().query, "Run");
    assert!(after.last().unwrap().query_revision > before.query_revision);
}

fn palette_command(
    owner: &Entity<View>,
    cx: &mut VisualTestContext,
    observer: HandlerId,
    expected: Option<i64>,
    command: gpuio_protocol::palette_command::Command,
) -> gpuio_protocol::palette_command::Response {
    cx.update(|w, cx| {
        owner.update(cx, |view, cx| {
            view.palette_command(id(1), observer, expected, &command, w, cx)
        })
    })
}

#[test]
fn palette_commands_clear_highlight_retain_it_and_never_activate() {
    use gpuio_protocol::palette_command::{Command as C, Error as E, Response as R};
    let mut app = TestAppContext::single();
    let (owner, mut cx, _reader) = mount_extra(&mut app, vec![Op::SetPaletteObserved(id(1), true)]);
    let handler = HandlerId::from_parts(1, 1).unwrap();
    let initial = observations(&owner, &cx).pop().unwrap();
    let result = palette_command(
        &owner,
        &mut cx,
        handler,
        Some(initial.query_revision),
        C::Highlight(None),
    );
    let R::Applied(cleared) = result else {
        panic!("{result:?}")
    };
    assert_eq!(cleared.selected, None);
    assert_eq!(cleared.query_revision, initial.query_revision);
    draw(&mut cx);
    draw(&mut cx);
    apply(&owner, &mut cx, vec![Op::SetCommands(id(0), commands())]);
    assert_eq!(state(&owner, &cx).1, None);
    assert_eq!(
        palette_command(
            &owner,
            &mut cx,
            handler,
            None,
            C::Highlight(Some("disabled".into()))
        ),
        R::Failed(E::Unavailable)
    );
    assert_eq!(state(&owner, &cx).1, None);
    assert!(matches!(
        palette_command(&owner, &mut cx, handler, None, C::Focus),
        R::Applied(_)
    ));
    cx.simulate_keystrokes("down");
    draw(&mut cx);
    assert_eq!(state(&owner, &cx).1.as_deref(), Some("run"));
    palette_command(&owner, &mut cx, handler, None, C::Highlight(None));
    cx.simulate_keystrokes("up");
    draw(&mut cx);
    assert_eq!(state(&owner, &cx).1.as_deref(), Some("other"));
    let result = palette_command(&owner, &mut cx, handler, None, C::SetQuery("Run".into()));
    let R::Applied(queried) = result else {
        panic!("{result:?}")
    };
    assert_eq!(queried.query, "Run");
    assert_eq!(queried.matched_count, 1);
    assert_eq!(queried.selected.as_deref(), Some("run"));
    assert!(queried.query_revision > initial.query_revision);
    assert_eq!(
        palette_command(
            &owner,
            &mut cx,
            handler,
            None,
            C::Highlight(Some("other".into()))
        ),
        R::Failed(E::Unavailable)
    );
    assert!(!state(&owner, &cx).0);
    owner.read_with(&cx, |view, _| {
        assert!(
            !view
                .transport
                .mailbox
                .lock()
                .unwrap()
                .drain(128)
                .iter()
                .any(|e| matches!(e, Event::CommandInvoked(..) | Event::PaletteDismissed(..)))
        );
    });
}

#[test]
fn palette_command_fence_reads_unnotified_edits_and_subscription_retirement() {
    use gpuio_protocol::palette_command::{Command as C, Error as E, Response as R};
    let mut app = TestAppContext::single();
    let (owner, mut cx, _reader) = mount_extra(&mut app, vec![Op::SetPaletteObserved(id(1), true)]);
    let handler = HandlerId::from_parts(1, 1).unwrap();
    let initial = observations(&owner, &cx).pop().unwrap();
    // No run_until_parked between the ABA edits and request: comparing a cached
    // observer snapshot here would admit the stale request.
    let result = cx.update(|w, cx| {
        owner.update(cx, |view, cx| {
            view.palettes[&id(1)].query.update(cx, |q, cx| {
                q.set_value("Run", w, cx);
                q.set_value("", w, cx);
            });
            view.palette_command(
                id(1),
                handler,
                Some(initial.query_revision),
                &C::SetQuery("bad".into()),
                w,
                cx,
            )
        })
    });
    assert_eq!(result, R::Failed(E::QueryChanged));
    assert_eq!(
        palette_command(&owner, &mut cx, handler, None, C::SetQuery("x\n".into())),
        R::Failed(E::InvalidQuery)
    );
    let replacement = HandlerId::from_parts(1, 2).unwrap();
    apply(&owner, &mut cx, vec![Op::Bind(id(1), Some(replacement))]);
    assert_eq!(
        palette_command(&owner, &mut cx, handler, None, C::ReadSnapshot),
        R::Failed(E::StalePalette)
    );
    assert!(matches!(
        palette_command(&owner, &mut cx, replacement, None, C::ReadSnapshot),
        R::Applied(_)
    ));
    apply(&owner, &mut cx, vec![Op::SetPaletteObserved(id(1), false)]);
    assert_eq!(
        palette_command(&owner, &mut cx, replacement, None, C::Focus),
        R::Failed(E::StalePalette)
    );
}

#[test]
fn palette_commands_preserve_composition_and_reject_hidden_query_focus() {
    use gpui::EntityInputHandler;
    use gpuio_protocol::palette_command::{Command as C, Error as E, Response as R};
    let mut app = TestAppContext::single();
    let (owner, mut cx, _reader) = mount_extra(&mut app, vec![Op::SetPaletteObserved(id(1), true)]);
    let handler = HandlerId::from_parts(1, 1).unwrap();
    let input = owner.read_with(&cx, |view, _| view.palettes[&id(1)].query.clone());
    cx.update(|w, cx| {
        w.focus(&input.read(cx).focus_handle(cx), cx);
        input.update(cx, |q, cx| {
            q.replace_and_mark_text_in_range(None, "λ", Some(0..1), w, cx)
        });
    });
    for command in [C::Focus, C::SetQuery("bad".into()), C::Highlight(None)] {
        assert_eq!(
            palette_command(&owner, &mut cx, handler, None, command),
            R::Failed(E::Composing)
        );
    }
    let R::Applied(snapshot) = palette_command(&owner, &mut cx, handler, None, C::ReadSnapshot)
    else {
        panic!()
    };
    assert_eq!(snapshot.query, "λ");
    assert!(snapshot.composing);
    cx.update(|w, cx| input.update(cx, |q, cx| q.unmark_text(w, cx)));
    let options = gpuio_protocol::palette_options::Config {
        searchable: false,
        ..Default::default()
    };
    apply(
        &owner,
        &mut cx,
        vec![Op::SetPaletteOptions(id(1), Some(options))],
    );
    assert_eq!(
        palette_command(&owner, &mut cx, handler, None, C::Focus),
        R::Failed(E::Unavailable)
    );
    assert!(matches!(
        palette_command(&owner, &mut cx, handler, None, C::SetQuery("Run".into())),
        R::Applied(_)
    ));
}

fn state(
    owner: &Entity<View>,
    cx: &VisualTestContext,
) -> (bool, Option<String>, Vec<(String, bool)>) {
    owner.read_with(cx, |view, _| view.palettes[&id(1)].probe())
}

#[test]
fn palette_loading_preserves_query_selection_composition_and_fences_stale_work() {
    use gpuio_protocol::palette_command::{Command as C, Error as E, Response as R};
    let mut app = TestAppContext::single();
    let (owner, mut cx, _reader) = mount_extra(&mut app, vec![Op::SetPaletteObserved(id(1), true)]);
    let handler = HandlerId::from_parts(1, 1).unwrap();
    let initial = observations(&owner, &cx).pop().unwrap();
    let R::Applied(loading) = palette_command(
        &owner,
        &mut cx,
        handler,
        Some(initial.query_revision),
        C::SetLoading(true),
    ) else {
        panic!()
    };
    assert!(loading.loading);
    assert!(loading.sequence > initial.sequence);
    assert_eq!(loading.query_revision, initial.query_revision);
    assert_eq!(loading.selected, initial.selected);
    assert_eq!(loading.matched_count, initial.matched_count);
    assert_eq!(
        palette_command(&owner, &mut cx, handler, None, C::SetLoading(true)),
        R::Applied(loading.clone())
    );
    let input = owner.read_with(&cx, |view, _| view.palettes[&id(1)].query.clone());
    cx.update(|w, cx| {
        w.focus(&input.read(cx).focus_handle(cx), cx);
        input.update(cx, |q, cx| {
            q.replace_and_mark_text_in_range(None, "λ", Some(0..1), w, cx)
        });
    });
    assert_eq!(
        palette_command(
            &owner,
            &mut cx,
            handler,
            Some(initial.query_revision),
            C::SetLoading(false)
        ),
        R::Failed(E::QueryChanged)
    );
    let R::Applied(composing) =
        palette_command(&owner, &mut cx, handler, None, C::SetLoading(false))
    else {
        panic!()
    };
    assert!(!composing.loading);
    assert!(composing.composing);
    assert_eq!(composing.query, "λ");
    cx.update(|w, cx| input.update(cx, |q, cx| q.unmark_text(w, cx)));
    // Readiness is visual status: no input edit, selection reset or implicit activation.
    query(&owner, &mut cx, "Run");
    let R::Applied(before) = palette_command(&owner, &mut cx, handler, None, C::SetLoading(true))
    else {
        panic!()
    };
    assert_eq!(before.selected.as_deref(), Some("run"));
    cx.simulate_keystrokes("enter");
    draw(&mut cx);
    assert!(
        state(&owner, &cx).0,
        "existing commands still activate while loading"
    );
    assert_eq!(
        palette_command(&owner, &mut cx, handler, None, C::SetLoading(false)),
        R::Failed(E::StalePalette)
    );
}

#[test]
fn palette_loading_gates_empty_content_and_retires_its_painter() {
    use gpuio_protocol::palette_command::Command as C;
    let mut app = TestAppContext::single();
    let mut extra = content_operations();
    extra.push(Op::SetPaletteObserved(id(1), true));
    let (owner, mut cx, _reader) = mount_extra(&mut app, extra);
    let handler = HandlerId::from_parts(1, 1).unwrap();
    cx.simulate_a11y_active(true);
    query(&owner, &mut cx, "missing");
    let target = cx
        .a11y_tree()
        .unwrap()
        .nodes
        .iter()
        .find(|(_, n)| n.label() == Some("Retry"))
        .unwrap()
        .0;
    palette_command(&owner, &mut cx, handler, None, C::SetLoading(true));
    owner.read_with(&cx, |view, _| {
        assert!(
            !view.focus.borrow().visible(id(10)),
            "empty controls gated before paint"
        );
        view.transport.mailbox.lock().unwrap().drain(128);
    });
    cx.simulate_a11y_action(gpui::accesskit::ActionRequest {
        action: gpui::accesskit::Action::Click,
        target_node: target,
        target_tree: gpui::accesskit::TreeId::ROOT,
        data: None,
    });
    draw(&mut cx);
    owner.read_with(&cx, |view, _| {
        assert!(
            !view
                .transport
                .mailbox
                .lock()
                .unwrap()
                .drain(128)
                .iter()
                .any(|e| matches!(e, Event::Press(_, n, _, _) if *n == id(10)))
        );
    });
    assert!(
        !cx.a11y_tree()
            .unwrap()
            .nodes
            .iter()
            .any(|(_, n)| n.label() == Some("Retry"))
    );
    assert!(
        cx.a11y_tree()
            .unwrap()
            .nodes
            .iter()
            .any(|(_, n)| n.label() == Some("Loading commands"))
    );
    assert!(
        cx.a11y_tree()
            .unwrap()
            .nodes
            .iter()
            .any(|(_, n)| n.label() == Some("Actions") && n.is_busy())
    );
    let probe = owner.read_with(&cx, |view, _| view.palettes[&id(1)].loading_probe.clone());
    assert!(probe.get().count > 0);
    cx.update(|_, cx| cx.set_reduce_motion(true));
    // Deliver callbacks already requested by the preceding animated paints.
    // The following static paint must not schedule another animation frame.
    cx.update(|w, cx| w.simulate_next_frame(cx));
    draw(&mut cx);
    assert_eq!(probe.get().phase, 0.);
    assert_eq!(cx.update(|w, cx| w.simulate_next_frame(cx)), 0);
    palette_command(&owner, &mut cx, handler, None, C::SetLoading(false));
    draw(&mut cx);
    let stopped = probe.get().count;
    draw(&mut cx);
    assert_eq!(probe.get().count, stopped);
    assert!(
        cx.a11y_tree()
            .unwrap()
            .nodes
            .iter()
            .any(|(_, n)| n.label() == Some("Retry"))
    );
    assert!(
        !cx.a11y_tree()
            .unwrap()
            .nodes
            .iter()
            .any(|(_, n)| n.label() == Some("Loading commands"))
    );
    cx.update(|_, cx| cx.set_reduce_motion(false));
    palette_command(&owner, &mut cx, handler, None, C::SetLoading(true));
    draw(&mut cx);
    assert!(cx.update(|w, cx| w.simulate_next_frame(cx)) > 0);
    cx.update(|w, cx| {
        owner.update(cx, |view, cx| {
            view.close_palette(id(1), PaletteDismissal::Escape, w, cx)
        })
    });
    draw(&mut cx);
    let closed = probe.get().count;
    // Closing cannot revoke callbacks queued by the preceding live frame.
    cx.update(|w, cx| w.simulate_next_frame(cx));
    draw(&mut cx);
    assert_eq!(probe.get().count, closed);
    assert_eq!(cx.update(|w, cx| w.simulate_next_frame(cx)), 0);
}

#[test]
fn palette_loading_survives_hidden_query_and_covered_overlay_without_editor_mutation() {
    use gpuio_protocol::palette_command::{Command as C, Response as R};
    let mut app = TestAppContext::single();
    let (owner, mut cx, _reader) = mount_extra(&mut app, vec![Op::SetPaletteObserved(id(1), true)]);
    let handler = HandlerId::from_parts(1, 1).unwrap();
    cx.simulate_a11y_active(true);
    query(&owner, &mut cx, "Run");
    let input = owner.read_with(&cx, |view, _| view.palettes[&id(1)].query.clone());
    let revision = input.read_with(&cx, |q, _| q.bridge_revision());
    apply(
        &owner,
        &mut cx,
        vec![Op::SetPaletteOptions(id(1), Some(options(false)))],
    );
    let R::Applied(before) = palette_command(&owner, &mut cx, handler, None, C::SetLoading(true))
    else {
        panic!()
    };
    draw(&mut cx);
    assert!(
        cx.a11y_tree()
            .unwrap()
            .nodes
            .iter()
            .any(|(_, n)| n.label() == Some("Loading commands")
                && n.role() == gpui::Role::ProgressIndicator)
    );
    apply(
        &owner,
        &mut cx,
        vec![
            Op::Create(
                id(2),
                Kind::CommandPalette,
                "".into(),
                Some(HandlerId::from_parts(2, 1).unwrap()),
            ),
            Op::SetPalette(
                id(2),
                PaletteConfig {
                    label: "Cover".into(),
                    placeholder: "Cover query".into(),
                    commands: vec![],
                    dismiss_on_outside_pointer: true,
                },
            ),
            Op::Splice(id(0), 1, 0, vec![id(2)]),
        ],
    );
    owner.read_with(&cx, |view, _| {
        assert!(!view.focus.borrow().top_overlay(id(1)))
    });
    let R::Applied(after) = palette_command(
        &owner,
        &mut cx,
        handler,
        Some(before.query_revision),
        C::SetLoading(false),
    ) else {
        panic!()
    };
    assert!(!after.loading);
    assert_eq!(after.query, before.query);
    assert_eq!(after.selected, before.selected);
    assert_eq!(after.query_revision, before.query_revision);
    assert_eq!(input.read_with(&cx, |q, _| q.bridge_revision()), revision);
    let weak = owner.read_with(&cx, |view, _| {
        Rc::downgrade(&view.palettes[&id(1)].loading_probe)
    });
    apply(
        &owner,
        &mut cx,
        vec![Op::Splice(id(0), 0, 1, vec![]), Op::Remove(id(1))],
    );
    draw(&mut cx);
    assert!(
        weak.upgrade().is_none(),
        "unmount releases the indicator state"
    );
}
