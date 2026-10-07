//! Production host/worker/profile rendering on TestPlatform; no physical GUI claim.
use super::*;
#[path = "document_cross_ranges_test.rs"]
mod cross_ranges;
#[path = "document_rendered_endpoints_test.rs"]
mod rendered_endpoints;
#[path = "document_resource_test.rs"]
mod resources;
#[path = "document_profile_scroll_test.rs"]
mod scroll;
#[path = "document_profile_semantics_test.rs"]
mod semantics;
use gpuio_protocol::document_profile::{
    Config as ProfileConfig, Error as ProfileError, Event as ProfileEvent, Instance, Signal, Stage,
};
fn config(epoch: i64, mode: u8) -> ProfileConfig {
    ProfileConfig {
        epoch,
        instance: Some(Instance {
            schema: gpuio_protocol::extension::Schema {
                name: crate::document_profile_fixture::NAME.into(),
                version: 1,
                fingerprint: crate::document_profile_fixture::FINGERPRINT.into(),
            },
            generation: 1,
            properties: gpuio_protocol::extension::Payload(vec![mode]),
        }),
    }
}
fn events(f: &Fixture) -> Vec<ProfileEvent> {
    f.transport
        .mailbox
        .lock()
        .unwrap()
        .drain(128)
        .into_iter()
        .filter_map(|event| match event {
            Event::DocumentProfileEvent(_, _, _, _, _, event) => Some(event),
            _ => None,
        })
        .collect()
}
fn install(f: &Fixture, cx: &mut VisualTestContext, epoch: i64, mode: u8) {
    apply(
        &f.view,
        cx,
        vec![Op::SetDocumentProfile(f.node, config(epoch, mode))],
    );
    ready(&f.presentation, cx);
    draw(cx);
}
#[test]
fn native_profiles_render_real_action_slots_and_stamped_events_in_both_formats() {
    for mode in [Mode::Markdown, Mode::Html] {
        let mut app = TestAppContext::single();
        let (f, cx) = mount(&mut app, mode);
        install(&f, cx, 1, 0);
        assert!(
            f.presentation
                .read_with(cx, |p, _| p.profile_install.is_some())
        );
        let tree = cx.a11y_tree().unwrap();
        assert!(
            tree.nodes
                .iter()
                .any(|(_, n)| n.label() == Some("Profile code"))
        );
        assert!(
            !tree
                .nodes
                .iter()
                .any(|(_, n)| n.label() == Some("Inspect code"))
        );
        f.presentation.read_with(cx, |p, _| {
            let lang = if p.config.mode == Mode::Html {
                "txt"
            } else {
                "ml"
            };
            let code = gpui_base::text::CodeBlock::from_code("let answer = 42", Some(lang));
            let runs = (p.code_highlighter.as_ref().unwrap())(&code);
            assert_eq!(runs.len(), 1);
            assert_eq!(runs[0].1.font_weight, Some(gpui::FontWeight(550.)));
            assert!(runs[0].1.strikethrough.is_some());
        });
        let rect = tree
            .nodes
            .iter()
            .find(|(_, n)| n.label() == Some("Profile code"))
            .unwrap()
            .1
            .bounds()
            .unwrap();
        let scale = cx.update(|w, _| f64::from(w.scale_factor()));
        cx.simulate_click(
            gpui::point(
                px(((rect.x0 + rect.x1) / (2. * scale)) as f32),
                px(((rect.y0 + rect.y1) / (2. * scale)) as f32),
            ),
            gpui::Modifiers::default(),
        );
        draw(cx);
        assert_eq!(events(&f).len(), 1);
        ax(cx, "Profile code", gpui::accesskit::Action::Click);
        let observed = events(&f);
        assert_eq!(observed.len(), 1);
        assert_eq!(
            (
                observed[0].config_epoch,
                observed[0].source_generation,
                observed[0].source_revision
            ),
            (1, 1, 1)
        );
        assert_eq!(
            observed[0].signal,
            Signal::Data(gpuio_protocol::extension::Payload(vec![7]))
        );
        ax(cx, "Profile table", gpui::accesskit::Action::Click);
        assert_eq!(
            events(&f)[0].signal,
            Signal::Data(gpuio_protocol::extension::Payload(vec![8]))
        );
    }
}
#[test]
fn profile_render_and_worker_failures_are_reported_once() {
    for (mode, stage, error) in [
        (1, Stage::Render, ProfileError::Panicked),
        (2, Stage::Configure, ProfileError::Panicked),
        (3, Stage::Highlight, ProfileError::Highlight),
    ] {
        let mut app = TestAppContext::single();
        let (f, cx) = mount(&mut app, Mode::Markdown);
        install(&f, cx, 1, mode);
        let first = events(&f);
        assert_eq!(first.len(), 1);
        assert_eq!(first[0].signal, Signal::Failed(stage, error));
        for _ in 0..3 {
            draw(cx);
        }
        assert!(events(&f).is_empty());
    }
}

fn sink(f: &Fixture, cx: &VisualTestContext) -> gpuio_document_sdk::EventSink {
    f.presentation
        .read_with(cx, |p, _| p.profile_install.as_ref().unwrap().events())
}
fn resume(f: &Fixture, cx: &mut VisualTestContext) {
    f.presentation.update(cx, |p, cx| {
        p.defer_prepared_install = false;
        cx.notify();
    });
    ready(&f.presentation, cx);
    draw(cx);
}
fn publish(f: &Fixture, cx: &mut VisualTestContext, generation: i64, revision: i64, text: &str) {
    f.view.update(cx, |view, cx| {
        for request in [
            Request::Begin(Update {
                id: f.source,
                base: revision - 1,
                revision,
                generation,
                from_byte: 0,
                suffix_bytes: text.len() as i64,
                status: Status::Complete,
            }),
            Request::Chunk(
                f.source,
                revision,
                0,
                gpuio_protocol::asset::Chunk::new(text.as_bytes().to_vec()).unwrap(),
            ),
            Request::Publish(f.source, revision),
        ] {
            assert_eq!(
                view.session.borrow_mut().document_request(request),
                Response::Ack
            );
        }
        view.document_changed(f.source, cx);
    });
}
#[test]
fn properties_handler_clear_and_unmount_revoke_retained_callbacks() {
    use gpuio_extension_sdk::Error;
    let mut app = TestAppContext::single();
    let (f, cx) = mount(&mut app, Mode::Markdown);
    install(&f, cx, 1, 0);
    let old = sink(&f, cx);
    assert_eq!(old.check(), Ok(()));
    f.presentation
        .update(cx, |p, _| p.defer_prepared_install = true);
    apply(
        &f.view,
        cx,
        vec![Op::SetDocumentProfile(f.node, config(2, 0))],
    );
    assert_eq!(old.emit(vec![1]), Err(Error::Closed));
    resume(&f, cx);
    let current = sink(&f, cx);
    current.emit(vec![2]).unwrap();
    assert_eq!(events(&f)[0].config_epoch, 2);

    f.presentation
        .update(cx, |p, _| p.defer_prepared_install = true);
    let handler = gpuio_protocol::HandlerId::from_parts(9, 1).unwrap();
    apply(&f.view, cx, vec![Op::Bind(f.node, Some(handler))]);
    assert_eq!(current.emit(vec![2]), Err(Error::Closed));
    resume(&f, cx);
    let rebound = sink(&f, cx);
    rebound.emit(vec![3]).unwrap();
    let observed = f.transport.mailbox.lock().unwrap().drain(128);
    assert!(observed.iter().any(|e| matches!(e,
        Event::DocumentProfileEvent(_, _, h, _, _, e) if *h == handler && e.config_epoch == 2)));

    apply(
        &f.view,
        cx,
        vec![Op::SetDocumentProfile(
            f.node,
            ProfileConfig {
                epoch: 3,
                instance: None,
            },
        )],
    );
    assert_eq!(rebound.check(), Err(Error::Closed));
    ready(&f.presentation, cx);
    assert!(
        f.presentation
            .read_with(cx, |p, _| p.profile_install.is_none())
    );
    install(&f, cx, 4, 0);
    let retained = sink(&f, cx);
    apply(&f.view, cx, vec![Op::SetRoot(None), Op::Remove(f.node)]);
    // Fixture deliberately retains Presentation, as old native callbacks may do.
    assert_eq!(retained.check(), Err(Error::Closed));
}
#[test]
fn source_reset_revokes_before_paint_and_inline_plugins_report_failures_once() {
    use gpuio_extension_sdk::Error;
    let mut app = TestAppContext::single();
    let (f, cx) = mount(&mut app, Mode::Markdown);
    install(&f, cx, 1, 0);
    let old = sink(&f, cx);
    f.presentation
        .update(cx, |p, _| p.defer_prepared_install = true);
    publish(&f, cx, 2, 2, "Before `badge` after");
    assert_eq!(old.check(), Err(Error::Closed));
    resume(&f, cx);
    ax(cx, "Profile badge", gpui::accesskit::Action::Click);
    let observed = events(&f);
    assert_eq!(observed.len(), 1);
    assert_eq!(
        (observed[0].source_generation, observed[0].source_revision),
        (2, 2)
    );
    assert_eq!(
        observed[0].signal,
        Signal::Data(gpuio_protocol::extension::Payload(vec![9]))
    );
    install(&f, cx, 2, 4);
    assert_eq!(
        events(&f)[0].signal,
        Signal::Failed(Stage::Render, ProfileError::Panicked)
    );
    for _ in 0..3 {
        draw(cx);
    }
    assert!(events(&f).is_empty());
}
#[test]
fn pointer_policy_preserves_keyboard_and_accessibility_but_hidden_input_is_rejected() {
    use gpuio_extension_sdk::Error;
    let mut app = TestAppContext::single();
    let (f, cx) = mount(&mut app, Mode::Markdown);
    install(&f, cx, 1, 0);
    let retained = sink(&f, cx);
    apply(
        &f.view,
        cx,
        vec![Op::SetStyle(
            f.node,
            vec![Style::Fields(vec![Field::PointerEvents(false)])],
        )],
    );
    assert_eq!(retained.guard_pointer(|| Ok(())), Err(Error::Hidden));
    assert_eq!(retained.guard(|| Ok(())), Ok(()));
    ax(cx, "Profile code", gpui::accesskit::Action::Click);
    assert_eq!(events(&f).len(), 1);
    cx.update(|window, _| window.activate_window());
    ax(cx, "Profile code", gpui::accesskit::Action::Focus);
    assert_focus(cx, "Profile code");
    let keystroke = gpui::Keystroke::parse("enter").unwrap();
    cx.simulate_event(gpui::KeyDownEvent {
        keystroke: keystroke.clone(),
        is_held: false,
        prefer_character_input: false,
    });
    cx.simulate_event(gpui::KeyUpEvent { keystroke });
    draw(cx);
    assert_eq!(events(&f).len(), 1);
    apply(
        &f.view,
        cx,
        vec![Op::SetStyle(
            f.node,
            vec![Style::Fields(vec![Field::Inert(true)])],
        )],
    );
    assert_eq!(retained.check(), Err(Error::Hidden));
    apply(&f.view, cx, vec![Op::SetStyle(f.node, vec![])]);
    assert_eq!(retained.check(), Ok(()));
    assert_eq!(
        retained.guard::<()>(|| panic!("fixture input")),
        Err(Error::Panicked)
    );
    draw(cx);
    assert_eq!(
        events(&f)[0].signal,
        Signal::Failed(Stage::Input, ProfileError::Panicked)
    );
    draw(cx);
    assert!(events(&f).is_empty());
}

#[test]
fn image_refresh_preserves_profile_selection_and_retained_accounting() {
    let mut app = TestAppContext::single();
    let (f, cx) = mount(&mut app, Mode::Markdown);
    install(&f, cx, 1, 0);
    let (installed, charge, highlighter, markdown) = f.presentation.read_with(cx, |p, _| {
        (
            p.profile_install.clone().unwrap(),
            Arc::downgrade(p.charge.as_ref().unwrap()),
            p.code_highlighter.clone().unwrap(),
            p.markdown.clone().unwrap(),
        )
    });
    let retained = sink(&f, cx);
    markdown.update(cx, |m, cx| m.select_all(cx));
    draw(cx);
    let selected = markdown.read_with(cx, |m, _| m.selected_text());
    let image = f.view.update(cx, |v, _| {
        let mut session = v.session.borrow_mut();
        let store = session.assets().unwrap();
        let mut bytes = b"P6\n1 1\n255\n".to_vec();
        bytes.extend([20, 100, 240]);
        let id = store
            .begin(gpuio_protocol::asset::Format::Pnm, bytes.len())
            .unwrap();
        store.append(id, 0, &bytes).unwrap();
        store.finish(id).unwrap();
        id
    });
    let mut config = f.presentation.read_with(cx, |p, _| (*p.config).clone());
    config.images = vec![("asset://profile".into(), ImageSource::Reference(image))];
    apply(&f.view, cx, vec![Op::SetDocument(f.node, config)]);
    for _ in 0..200 {
        draw(cx);
        if f.presentation.read_with(cx, |p, _| !p.images.0.is_empty()) {
            break;
        }
        std::thread::sleep(Duration::from_millis(5));
    }
    f.presentation.read_with(cx, |p, _| {
        assert_eq!(p.images.0.len(), 1);
        assert!(Arc::ptr_eq(p.profile_install.as_ref().unwrap(), &installed));
    });
    assert_eq!(markdown.read_with(cx, |m, _| m.selected_text()), selected);
    ax(cx, "Profile code", gpui::accesskit::Action::Click);
    assert_eq!(events(&f).len(), 1);
    apply(&f.view, cx, vec![Op::SetRoot(None), Op::Remove(f.node)]);
    assert_eq!(retained.check(), Err(gpuio_extension_sdk::Error::Closed));
    drop(installed);
    drop(markdown);
    drop(f.presentation);
    draw(cx);
    assert!(
        charge.upgrade().is_some(),
        "retained closures still own their charge"
    );
    drop(retained);
    assert!(
        charge.upgrade().is_some(),
        "highlighter independently retains charge"
    );
    drop(highlighter);
    draw(cx);
    assert!(
        charge.upgrade().is_none(),
        "all profile callbacks released their charge"
    );
}

#[test]
fn block_plugins_render_native_controls_and_old_append_pictures_keep_their_stamp() {
    let mut app = TestAppContext::single();
    let (f, cx) = mount(&mut app, Mode::Markdown);
    install(&f, cx, 1, 5);
    publish(&f, cx, 2, 2, "Before\n\n```card\nhello\n```\n");
    ready(&f.presentation, cx);
    draw(cx);
    ax(cx, "Profile card", gpui::accesskit::Action::Click);
    assert_eq!(
        events(&f)[0].signal,
        Signal::Data(gpuio_protocol::extension::Payload(vec![10]))
    );
    let old = sink(&f, cx);
    f.presentation
        .update(cx, |p, _| p.defer_prepared_install = true);
    publish(&f, cx, 2, 3, "Before\n\n```card\nhello\n```\n\nAppended");
    draw(cx);
    old.emit(vec![10]).unwrap();
    let old_event = events(&f);
    assert_eq!(
        (old_event[0].source_generation, old_event[0].source_revision),
        (2, 2)
    );
    resume(&f, cx);
    assert_eq!(old.check(), Err(gpuio_extension_sdk::Error::Closed));
    ax(cx, "Profile card", gpui::accesskit::Action::Click);
    let new_event = events(&f);
    assert_eq!(
        (new_event[0].source_generation, new_event[0].source_revision),
        (2, 3)
    );
}

#[test]
fn virtual_profile_controls_reveal_inline_block_and_code_targets_in_both_directions() {
    let mut app = TestAppContext::single();
    let (f, cx) = mount(&mut app, Mode::Markdown);
    install(&f, cx, 1, 5);
    let gap = "Paragraph.\n\n".repeat(25);
    publish(
        &f,
        cx,
        2,
        2,
        &format!("{gap}Inline `badge`.\n\n{gap}```card\nReview\n```\n\n{gap}```ml\n42\n```\n"),
    );
    ready(&f.presentation, cx);
    let mut config = f.presentation.read_with(cx, |p, _| (*p.config).clone());
    config.layout = Layout::Viewport(180.);
    apply(&f.view, cx, vec![Op::SetDocument(f.node, config)]);
    let markdown = f
        .presentation
        .read_with(cx, |p, _| p.markdown.clone().unwrap());
    markdown.read_with(cx, |m, _| m.list_state().scroll_to_reveal_item(0));
    draw(cx);
    cx.update(|w, _| w.activate_window());
    ax(cx, "Document content", gpui::accesskit::Action::Focus);
    for (key, label) in [
        ("tab", "Profile badge"),
        ("tab", "Profile card"),
        ("tab", "Profile code"),
        ("shift-tab", "Profile card"),
        ("shift-tab", "Profile badge"),
    ] {
        cx.simulate_keystrokes(key);
        for _ in 0..6 {
            draw(cx);
        }
        assert_focus(cx, label);
    }
    let key = gpui::Keystroke::parse("enter").unwrap();
    cx.simulate_event(gpui::KeyDownEvent {
        keystroke: key.clone(),
        is_held: false,
        prefer_character_input: false,
    });
    cx.simulate_event(gpui::KeyUpEvent { keystroke: key });
    draw(cx);
    let events = events(&f);
    assert_eq!(events.len(), 1);
    assert_eq!(
        (events[0].source_generation, events[0].source_revision),
        (2, 2)
    );
    assert_eq!(
        events[0].signal,
        Signal::Data(gpuio_protocol::extension::Payload(vec![9]))
    );
}

#[test]
fn pending_virtual_focus_is_canceled_when_the_source_resets_before_the_next_paint() {
    let mut app = TestAppContext::single();
    let (f, cx) = mount(&mut app, Mode::Markdown);
    install(&f, cx, 1, 0);
    publish(
        &f,
        cx,
        2,
        2,
        &format!("{}```ml\nold target\n```\n", "Paragraph.\n\n".repeat(30)),
    );
    ready(&f.presentation, cx);
    let mut config = f.presentation.read_with(cx, |p, _| (*p.config).clone());
    config.layout = Layout::Viewport(180.);
    apply(&f.view, cx, vec![Op::SetDocument(f.node, config)]);
    let markdown = f
        .presentation
        .read_with(cx, |p, _| p.markdown.clone().unwrap());
    markdown.read_with(cx, |m, _| m.list_state().scroll_to_reveal_item(0));
    draw(cx);
    cx.update(|w, _| w.activate_window());
    ax(cx, "Document content", gpui::accesskit::Action::Focus);
    cx.update(|window, cx| {
        window.dispatch_keystroke(gpui::Keystroke::parse("tab").unwrap(), cx);
        assert!(
            markdown.read(cx).focus_handle().is_focused(window),
            "request must still be pending before reset"
        );
        f.presentation
            .update(cx, |p, _| p.defer_prepared_install = true);
        f.view.update(cx, |view, cx| {
            let text = "Replacement without controls";
            for request in [
                Request::Begin(Update {
                    id: f.source,
                    base: 2,
                    revision: 3,
                    generation: 3,
                    from_byte: 0,
                    suffix_bytes: text.len() as i64,
                    status: Status::Complete,
                }),
                Request::Chunk(
                    f.source,
                    3,
                    0,
                    gpuio_protocol::asset::Chunk::new(text.as_bytes().to_vec()).unwrap(),
                ),
                Request::Publish(f.source, 3),
            ] {
                assert_eq!(
                    view.session.borrow_mut().document_request(request),
                    Response::Ack
                );
            }
            view.document_changed(f.source, cx);
        });
    });
    for _ in 0..6 {
        draw(cx);
    }
    // A reset can withdraw the old focus owner before installing its replacement.
    // Cancellation may retain the owner or allow normal host blur, but must not
    // transfer focus into a stale action (or an unrelated toolbar button).
    cx.update(|window, cx| {
        assert!(
            window.focused(cx).is_none() || markdown.read(cx).focus_handle().is_focused(window)
        );
    });
    assert!(events(&f).is_empty());
    resume(&f, cx);
    assert!(
        !cx.a11y_tree()
            .unwrap()
            .nodes
            .iter()
            .any(|(_, node)| node.label() == Some("Profile code"))
    );
}
