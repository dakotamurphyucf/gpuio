//! Mounted plugin text/search/copy/AX contracts; no physical OS input claim.
use super::*;
use gpuio_protocol::highlight;

const SOURCE: &str = "Before `badge` after.\n\n```card\ncontent\n```\n";
const COLOR: u32 = 0x771133ff;

fn scope() -> NodeId {
    NodeId::from_parts(1, 1).unwrap()
}
fn search() -> highlight::Config {
    highlight::Config(vec![highlight::Spec {
        query: Some(highlight::Query {
            text: "Profile".into(),
            case_sensitive: true,
            whole_word: false,
        }),
        ranges: vec![],
        appearance: highlight::Appearance {
            color: COLOR as i64,
            active_color: COLOR as i64,
            radius: 0.,
        },
        active_index: None,
        match_index_offset: 0,
    }])
}
fn settled_search(f: &Fixture, cx: &mut VisualTestContext, expected: highlight::State) {
    for _ in 0..1000 {
        draw(cx);
        let current = f.view.read_with(cx, |v, _| {
            v.highlights[&scope()].borrow().observation().state
        });
        if current == expected {
            // Paint the installed worker result, not merely its observation.
            draw(cx);
            return;
        }
        std::thread::sleep(Duration::from_millis(5));
    }
    panic!(
        "search did not settle: expected {expected:?}, got {:?}",
        f.view
            .read_with(cx, |v, _| v.highlights[&scope()].borrow().observation())
    );
}

#[test]
fn mounted_plugin_presentation_controls_search_copy_and_accessibility() {
    let mut app = TestAppContext::single();
    let (f, cx) = mount(&mut app, Mode::Markdown);
    publish(&f, cx, 2, 2, SOURCE);
    ready(&f.presentation, cx);
    install(&f, cx, 1, 5);
    apply(
        &f.view,
        cx,
        vec![
            Op::Create(scope(), Kind::HighlightScope, "".into(), None),
            Op::SetHighlightScope(scope(), search()),
            Op::Splice(scope(), 0, 0, vec![f.node]),
            Op::SetRoot(Some(scope())),
        ],
    );
    // Reconfigure the same mounted source to ensure old projections/washes retire.
    for (epoch, mode) in [(1, 5), (2, 6), (3, 7), (4, 6), (5, 5)] {
        if epoch > 1 {
            install(&f, cx, epoch, mode);
        }
        let text = f.presentation.read_with(cx, |p, _| {
            assert_eq!(p.installed.as_ref().unwrap().revision, 2);
            assert_eq!(p.installed.as_ref().unwrap().text.to_string(), SOURCE);
            p.markdown.clone().unwrap()
        });
        let displayed = text.read_with(cx, |t, _| t.displayed_text().unwrap());
        assert_eq!(displayed.opaque_nodes(), if mode == 7 { 2 } else { 0 });
        assert_eq!(
            displayed
                .fragments()
                .iter()
                .filter(|f| f.text().contains("Profile"))
                .count(),
            if mode == 6 { 2 } else { 0 }
        );
        let expected = if mode == 7 {
            highlight::State::Failed(highlight::Failure::SourceUnavailable)
        } else {
            let total = if mode == 6 { 2 } else { 0 };
            highlight::State::Ready(vec![highlight::Count {
                total,
                stored: total,
            }])
        };
        settled_search(&f, cx, expected);
        let color: gpui::Hsla = gpui::rgba(COLOR).into();
        assert_eq!(
            cx.update(|w, _| w
                .painted_quads()
                .iter()
                .any(|q| q.background == gpui::Background::from(color))),
            mode == 6,
            "only declared Text glyphs receive document search paint"
        );
        let tree = cx.a11y_tree().unwrap();
        for label in ["Profile badge", "Profile card"] {
            if mode == 6 {
                assert!(
                    tree.nodes
                        .iter()
                        .any(|(_, n)| n.role() == gpui::Role::Label && n.value() == Some(label))
                );
                assert!(
                    !tree
                        .nodes
                        .iter()
                        .any(|(_, n)| n.role() == gpui::Role::Button && n.label() == Some(label))
                );
            } else {
                assert!(
                    tree.nodes
                        .iter()
                        .any(|(_, n)| n.role() == gpui::Role::Button && n.label() == Some(label))
                );
            }
        }
        text.update(cx, |t, cx| t.select_all(cx));
        let plain = text.read_with(cx, |t, _| t.selected_text());
        assert!(
            plain.contains("Profile badge") && plain.contains("Profile card"),
            "{plain:?}"
        );
        assert_eq!(
            cx.update(gpui_base::TextSelection::selected_text),
            plain.trim()
        );
        apply(
            &f.view,
            cx,
            vec![Op::SetDocumentSelectionFormat(f.node, true)],
        );
        assert_eq!(text.read_with(cx, |t, _| t.selected_text()), SOURCE);
        assert_eq!(cx.update(gpui_base::TextSelection::selected_text), SOURCE);
        apply(
            &f.view,
            cx,
            vec![Op::SetDocumentSelectionFormat(f.node, false)],
        );
        assert_eq!(text.read_with(cx, |t, _| t.selected_text()), plain);
        // Copy-format changes retain the prepared projection and current search.
        assert!(Arc::ptr_eq(
            &displayed,
            &text.read_with(cx, |t, _| t.displayed_text().unwrap())
        ));
    }
}

fn drag_label(cx: &mut VisualTestContext, label: &str, backward: bool) {
    drag_label_width(cx, label, backward, None);
}

fn drag_label_width(cx: &mut VisualTestContext, label: &str, backward: bool, width: Option<f32>) {
    let tree = cx.a11y_tree().unwrap();
    let bounds = tree
        .nodes
        .iter()
        .find(|(_, n)| n.value() == Some(label))
        .unwrap_or_else(|| panic!("missing text {label}"))
        .1
        .bounds()
        .unwrap();
    let scale = cx.update(|w, _| f64::from(w.scale_factor()));
    let y = px(((bounds.y0 + bounds.y1) / (2. * scale)) as f32);
    let start = gpui::point(px((bounds.x0 / scale) as f32) + px(1.), y);
    let end_x = width.map_or(px((bounds.x1 / scale) as f32) - px(1.), |width| {
        start.x + px(width)
    });
    let end = gpui::point(end_x, y);
    let (start, end) = if backward { (end, start) } else { (start, end) };
    cx.simulate_mouse_move(start, None, Default::default());
    cx.simulate_mouse_down(start, gpui::MouseButton::Left, Default::default());
    cx.simulate_mouse_move(end, Some(gpui::MouseButton::Left), Default::default());
    cx.simulate_mouse_up(end, gpui::MouseButton::Left, Default::default());
    draw(cx);
}

#[test]
fn projected_block_substrings_copy_unicode_and_clear_without_selecting_siblings() {
    let mut app = TestAppContext::single();
    let (f, cx) = mount(&mut app, Mode::Markdown);
    publish(
        &f,
        cx,
        2,
        2,
        "```card\n世界 hello\n```\n\n```card\n世界 other\n```\n",
    );
    ready(&f.presentation, cx);
    install(&f, cx, 1, 8);
    let text = f
        .presentation
        .read_with(cx, |p, _| p.markdown.clone().unwrap());
    for label in ["世界 hello", "世界 other"] {
        for backward in [false, true] {
            // AX bounds reserve the full block width, not just the glyph advance.
            drag_label_width(cx, label, backward, Some(20.));
            let selected = cx.update(gpui_base::TextSelection::selected_text);
            assert!(
                selected.starts_with('世') && label.starts_with(&selected) && selected != label,
                "{selected:?}"
            );
            apply(
                &f.view,
                cx,
                vec![Op::SetDocumentSelectionFormat(f.node, true)],
            );
            assert_eq!(
                cx.update(gpui_base::TextSelection::selected_text),
                selected,
                "partial projections have no source-character mapping"
            );
            drag_label(cx, label, backward);
            assert_eq!(
                cx.update(gpui_base::TextSelection::selected_text),
                format!("```card\n{label}\n```")
            );
            apply(
                &f.view,
                cx,
                vec![Op::SetDocumentSelectionFormat(f.node, false)],
            );
            assert_eq!(cx.update(gpui_base::TextSelection::selected_text), label);
            cx.update(gpui_base::TextSelection::clear);
            assert!(text.read_with(cx, |t, _| t.selected_text()).is_empty());
            draw(cx);
            assert!(
                cx.update(gpui_base::TextSelection::selected_text)
                    .is_empty()
            );
        }
    }
}

#[test]
fn declared_plugin_text_supports_partial_document_pointer_selection() {
    let mut app = TestAppContext::single();
    let (f, cx) = mount(&mut app, Mode::Markdown);
    publish(&f, cx, 2, 2, SOURCE);
    ready(&f.presentation, cx);
    install(&f, cx, 1, 6);
    let text = f
        .presentation
        .read_with(cx, |p, _| p.markdown.clone().unwrap());
    for label in ["Profile badge", "Profile card"] {
        for backward in [false, true] {
            drag_label(cx, label, backward);
            assert_eq!(text.read_with(cx, |t, _| t.selected_text()).trim(), label);
            assert_eq!(cx.update(gpui_base::TextSelection::selected_text), label);
            // Frame-local Text projections must retain the logical selection.
            for _ in 0..4 {
                draw(cx);
                assert_eq!(text.read_with(cx, |t, _| t.selected_text()).trim(), label);
            }
        }
    }
    publish(&f, cx, 3, 3, "Replacement without custom text.");
    ready(&f.presentation, cx);
    assert!(text.read_with(cx, |t, _| t.selected_text()).is_empty());
    assert!(
        cx.update(gpui_base::TextSelection::selected_text)
            .is_empty()
    );
}

#[test]
fn virtual_focus_crosses_many_passive_plugin_candidates_and_exits() {
    const BLOCKS: usize = 96;
    let mut app = TestAppContext::single();
    let (f, cx) = mount(&mut app, Mode::Markdown);
    install(&f, cx, 1, 6);
    let source = format!(
        "{}```ml\n42\n```\n",
        "```card\npassive\n```\n\n".repeat(BLOCKS)
    );
    publish(&f, cx, 2, 2, &source);
    ready(&f.presentation, cx);
    let mut config = f.presentation.read_with(cx, |p, _| (*p.config).clone());
    config.layout = Layout::Viewport(180.);
    apply(&f.view, cx, vec![Op::SetDocument(f.node, config)]);
    let text = f
        .presentation
        .read_with(cx, |p, _| p.markdown.clone().unwrap());
    text.read_with(cx, |t, _| t.list_state().scroll_to_reveal_item(0));
    draw(cx);
    cx.update(|w, _| w.activate_window());
    ax(cx, "Document content", gpui::accesskit::Action::Focus);
    for (key, label) in [
        ("tab", "Profile code"),
        ("shift-tab", "Document content"),
        ("tab", "Profile code"),
        ("tab", "Collapse"),
    ] {
        cx.simulate_keystrokes(key);
        for _ in 0..(2 * BLOCKS + 8) {
            draw(cx);
            let tree = cx.a11y_tree().unwrap();
            if tree
                .nodes
                .iter()
                .find(|(id, _)| *id == tree.focus)
                .and_then(|(_, n)| n.label())
                == Some(label)
            {
                break;
            }
        }
        assert_focus(cx, label);
    }
    assert!(
        events(&f).is_empty(),
        "traversal must not activate native controls"
    );
}
