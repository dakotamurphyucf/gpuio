//! Offscreen document controls on the production host's TestPlatform path.
use super::*;
fn replace(f: &Fixture, cx: &mut VisualTestContext, text: &str) {
    f.view.update(cx, |view, cx| {
        for request in [
            Request::Begin(Update {
                id: f.source,
                base: 1,
                revision: 2,
                generation: 2,
                from_byte: 0,
                suffix_bytes: text.len() as i64,
                status: Status::Complete,
            }),
            Request::Chunk(
                f.source,
                2,
                0,
                gpuio_protocol::asset::Chunk::new(text.as_bytes().to_vec()).unwrap(),
            ),
            Request::Publish(f.source, 2),
        ] {
            assert_eq!(
                view.session.borrow_mut().document_request(request),
                Response::Ack
            );
        }
        view.document_changed(f.source, cx);
    });
    ready(&f.presentation, cx);
}
#[test]
fn tab_reaches_unpainted_code_actions_in_a_virtual_document() {
    let mut app = TestAppContext::single();
    let (f, cx) = mount(&mut app, Mode::Markdown);
    let mut text = (0..30)
        .map(|i| format!("Paragraph {i}.\n\n"))
        .collect::<String>();
    text.push_str("```ml\nlet offscreen = 42\n```\n\nTail\n");
    replace(&f, cx, &text);
    let mut config = f.presentation.read_with(cx, |p, _| (*p.config).clone());
    config.layout = Layout::Viewport(180.);
    apply(&f.view, cx, vec![Op::SetDocument(f.node, config)]);
    let markdown = f
        .presentation
        .read_with(cx, |p, _| p.markdown.clone().unwrap());
    let list = markdown.read_with(cx, |m, _| m.list_state().clone());
    list.scroll_to_reveal_item(0);
    draw(cx);
    assert!(
        !cx.a11y_tree()
            .unwrap()
            .nodes
            .iter()
            .any(|(_, n)| n.label() == Some("Inspect code"))
    );
    cx.update(|window, _| window.activate_window());
    ax(cx, "Document content", gpui::accesskit::Action::Focus);
    cx.simulate_keystrokes("tab");
    for _ in 0..4 {
        draw(cx);
    }
    assert_focus(cx, "Copy code");
    cx.simulate_keystrokes("tab");
    draw(cx);
    assert_focus(cx, "Inspect code");
    let keystroke = gpui::Keystroke::parse("enter").unwrap();
    cx.simulate_event(gpui::KeyDownEvent {
        keystroke: keystroke.clone(),
        is_held: false,
        prefer_character_input: false,
    });
    cx.simulate_event(gpui::KeyUpEvent { keystroke });
    draw(cx);
    assert!(
        events(&f)
            .iter()
            .any(|e| matches!(&e.block, Block::Code(_, text) if text == "let offscreen = 42"))
    );
}

fn viewport(f: &Fixture, cx: &mut VisualTestContext) -> Entity<TextViewState> {
    let mut config = f.presentation.read_with(cx, |p, _| (*p.config).clone());
    config.layout = Layout::Viewport(180.);
    apply(&f.view, cx, vec![Op::SetDocument(f.node, config)]);
    f.presentation
        .read_with(cx, |p, _| p.markdown.clone().unwrap())
}
fn tab(cx: &mut VisualTestContext, backward: bool, label: &str) {
    cx.simulate_keystrokes(if backward { "shift-tab" } else { "tab" });
    for _ in 0..6 {
        draw(cx);
    }
    assert_focus(cx, label);
}
#[test]
fn virtual_actions_traverse_both_directions_across_tall_blocks_and_exit() {
    let mut app = TestAppContext::single();
    let (f, cx) = mount(&mut app, Mode::Markdown);
    let filler = (0..35).map(|i| format!("line{i}\n")).collect::<String>();
    let paragraphs = (0..30)
        .map(|i| format!("Paragraph{i}.\n\n"))
        .collect::<String>();
    replace(
        &f,
        cx,
        &format!("```ml\n{filler}```\n\n{paragraphs}| Name |\n| --- |\n| last |\n"),
    );
    let markdown = viewport(&f, cx);
    markdown.read_with(cx, |m, _| m.list_state().scroll_to_reveal_item(0));
    draw(cx);
    cx.update(|w, _| w.activate_window());
    ax(cx, "Document content", gpui::accesskit::Action::Focus);
    for label in ["Copy code", "Inspect code", "Copy table", "Export table"] {
        tab(cx, false, label);
    }
    for label in [
        "Copy table",
        "Inspect code",
        "Copy code",
        "Document content",
    ] {
        tab(cx, true, label);
    }
    for label in [
        "Copy code",
        "Inspect code",
        "Copy table",
        "Export table",
        "Collapse",
    ] {
        tab(cx, false, label);
    }
}
#[test]
fn empty_virtual_action_rows_do_not_trap_tab_or_become_focus_targets() {
    let mut app = TestAppContext::single();
    let (f, cx) = mount(&mut app, Mode::Markdown);
    let mut disabled = actions(2, false);
    disabled.copy_code = false;
    disabled.copy_table = false;
    disabled.table[0].enabled = false;
    apply(&f.view, cx, vec![Op::SetDocumentActions(f.node, disabled)]);
    let markdown = viewport(&f, cx);
    markdown.read_with(cx, |m, _| m.list_state().scroll_to_reveal_item(0));
    draw(cx);
    cx.update(|w, _| w.activate_window());
    ax(cx, "Document content", gpui::accesskit::Action::Focus);
    tab(cx, false, "Collapse");
    assert!(events(&f).is_empty());
}
#[test]
fn rendered_tab_query_is_read_only_and_pending_focus_cannot_steal_focus() {
    let mut app = TestAppContext::single();
    let (f, cx) = mount(&mut app, Mode::Markdown);
    let text = format!("{}```ml\n42\n```\n", "Paragraph.\n\n".repeat(30));
    replace(&f, cx, &text);
    let markdown = viewport(&f, cx);
    markdown.read_with(cx, |m, _| m.list_state().scroll_to_reveal_item(0));
    draw(cx);
    cx.update(|w, _| w.activate_window());
    ax(cx, "Document content", gpui::accesskit::Action::Focus);
    let owner = markdown.read_with(cx, |m, _| m.focus_handle().clone());
    cx.update(|w, cx| {
        let before = w.focused(cx);
        assert!(w.tab_stops_within(&owner).is_empty());
        assert!(w.tab_stop_outside(&owner, false).is_some());
        assert_eq!(w.focused(cx), before);
        w.dispatch_keystroke(gpui::Keystroke::parse("tab").unwrap(), cx);
        assert!(
            owner.is_focused(w),
            "realization must not focus an unpainted control"
        );
        let other = f.presentation.read(cx).buttons["document-collapse"].clone();
        w.focus(&other, cx);
    });
    for _ in 0..6 {
        draw(cx);
    }
    assert_focus(cx, "Collapse");
    assert!(events(&f).is_empty());
}

#[test]
fn repeated_tab_during_virtual_realization_does_not_restart_logical_links() {
    let mut app = TestAppContext::single();
    let (f, cx) = mount(&mut app, Mode::Markdown);
    replace(
        &f,
        cx,
        &format!(
            "[Intro](test:intro)\n\n{}```ml\n42\n```\n",
            "Paragraph.\n\n".repeat(30)
        ),
    );
    let markdown = viewport(&f, cx);
    markdown.read_with(cx, |m, _| m.list_state().scroll_to_reveal_item(0));
    draw(cx);
    cx.update(|w, _| w.activate_window());
    ax(cx, "Document content", gpui::accesskit::Action::Focus);
    // Link traversal, an offscreen control request, and a repeat arrive before
    // queued post-paint focus callbacks can run. The repeat must not scroll back
    // to Intro while retaining a request for a control in a different frame.
    cx.update(|w, cx| {
        for _ in 0..3 {
            w.dispatch_keystroke(gpui::Keystroke::parse("tab").unwrap(), cx);
        }
    });
    for _ in 0..6 {
        draw(cx);
    }
    assert_focus(cx, "Copy code");
    tab(cx, false, "Inspect code");
}
