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
fn apply(owner: &Entity<View>, cx: &mut VisualTestContext, operations: Vec<Op>) {
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
fn mount(app: &mut TestAppContext) -> (Entity<View>, VisualTestContext, UnixStream) {
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
    apply(
        &owner,
        &mut cx,
        vec![
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
        ],
    );
    (owner, cx, reader)
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
    query(&owner, &mut cx, "EXECUTE λ");
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
            assert_eq!(input.read(cx).value().as_str(), "EXECUTE λ");
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
