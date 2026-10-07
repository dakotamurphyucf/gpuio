//! Shaped endpoint capture through production pointer dispatch on TestPlatform.
//! Not an OS AX, physical-input, or complete logical selection acceptance test.
use super::*;

fn captured(text: &Entity<gpui_base::TextViewState>, cx: &VisualTestContext) -> (usize, usize) {
    text.read_with(cx, |state, cx| {
        let selection = state
            .captured_rendered_pointer_selection(cx)
            .expect("captured endpoints");
        let text = state.rendered_text().unwrap();
        (
            text.offset(selection.anchor()).unwrap(),
            text.offset(selection.head()).unwrap(),
        )
    })
}

fn drag(cx: &mut VisualTestContext, label: &str, occurrence: usize, backward: bool) {
    let tree = cx.a11y_tree().unwrap();
    let bounds = tree
        .nodes
        .iter()
        .filter(|(_, n)| n.value() == Some(label))
        .nth(occurrence)
        .unwrap()
        .1
        .bounds()
        .unwrap();
    let scale = cx.update(|w, _| f64::from(w.scale_factor()));
    let y = px(((bounds.y0 + bounds.y1) / (2. * scale)) as f32);
    let left = gpui::point(px((bounds.x0 / scale) as f32) + px(1.), y);
    let right = left + gpui::point(px(45.), px(0.));
    let (start, end) = if backward {
        (right, left)
    } else {
        (left, right)
    };
    cx.simulate_mouse_move(start, None, Default::default());
    cx.simulate_mouse_down(start, gpui::MouseButton::Left, Default::default());
    cx.simulate_mouse_move(end, Some(gpui::MouseButton::Left), Default::default());
    cx.simulate_mouse_up(end, gpui::MouseButton::Left, Default::default());
    draw(cx);
}

#[test]
fn pointer_endpoints_keep_direction_and_duplicate_owner_then_expire_on_replacement() {
    let mut app = TestAppContext::single();
    let (f, cx) = mount(&mut app, Mode::Markdown);
    let label = "Repeated 世界 e\u{301} 👨‍👩‍👧‍👦";
    let source = format!("{label}\n\n{label}");
    publish(&f, cx, 2, 2, &source);
    ready(&f.presentation, cx);
    draw(cx);
    let text = f
        .presentation
        .read_with(cx, |p, _| p.markdown.clone().unwrap());
    for occurrence in 0..2 {
        drag(cx, label, occurrence, false);
        let forward = captured(&text, cx);
        assert_eq!(forward.0, occurrence * (label.len() + 1));
        assert!(forward.1 > forward.0);
        // A nonvirtual endpoint must not introduce a block-zero-only Copy filter.
        assert!(
            !cx.update(gpui_base::TextSelection::selected_text)
                .is_empty()
        );
        drag(cx, label, occurrence, true);
        assert_eq!(captured(&text, cx), (forward.1, forward.0));
    }
    // Equal source is still a different preparation. Captured coordinates may
    // remain in the window controller, but cannot address the replacement.
    text.update(cx, |state, cx| {
        state.set_prepared(
            gpui_base::text::PreparedText::parse(&source, Default::default()).unwrap(),
            None,
            cx,
        )
    });
    assert!(text.read_with(cx, |state, cx| {
        state.captured_rendered_pointer_selection(cx).is_none()
    }));
}

#[test]
fn captured_pointer_anchor_survives_virtualization_without_retaining_layout_history() {
    let mut app = TestAppContext::single();
    let (f, cx) = mount(&mut app, Mode::Markdown);
    let source = (0..120)
        .map(|i| format!("Paragraph {i} repeated content\n\n"))
        .collect::<String>();
    publish(&f, cx, 2, 2, &source);
    ready(&f.presentation, cx);
    let mut config = f.presentation.read_with(cx, |p, _| (*p.config).clone());
    config.layout = Layout::Viewport(180.);
    apply(&f.view, cx, vec![Op::SetDocument(f.node, config)]);
    draw(cx);
    let text = f
        .presentation
        .read_with(cx, |p, _| p.markdown.clone().unwrap());
    drag(cx, "Paragraph 0 repeated content", 0, true);
    let before = captured(&text, cx);
    let position = text.read_with(cx, |state, _| state.bounds().center());
    cx.simulate_event(gpui::ScrollWheelEvent {
        position,
        delta: gpui::ScrollDelta::Pixels(gpui::point(px(0.), px(-1500.))),
        modifiers: Default::default(),
        touch_phase: gpui::TouchPhase::Moved,
    });
    draw(cx);
    assert!(
        !cx.a11y_tree()
            .unwrap()
            .nodes
            .iter()
            .any(|(_, n)| n.value() == Some("Paragraph 0 repeated content")),
        "anchor must actually leave the realized tree"
    );
    assert_eq!(captured(&text, cx), before);
    cx.update(gpui_base::TextSelection::clear);
    assert!(text.read_with(cx, |state, cx| {
        state.captured_rendered_pointer_selection(cx).is_none()
    }));
}
