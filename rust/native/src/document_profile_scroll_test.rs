//! Independent plugin scrolling through the production reader on TestPlatform.
use super::*;

fn step(cx: &mut VisualTestContext, key: &str, expected: &str) {
    cx.simulate_keystrokes(key);
    for _ in 0..8 {
        draw(cx);
    }
    assert_focus(cx, expected);
}

#[test]
fn nested_scroll_clipping_does_not_trap_reader_focus_or_activate_controls() {
    let mut app = TestAppContext::single();
    let (f, cx) = mount(&mut app, Mode::Markdown);
    install(&f, cx, 1, 9);
    publish(&f, cx, 2, 2, "```card\nNested controls\n```\n");
    ready(&f.presentation, cx);
    let mut config = f.presentation.read_with(cx, |p, _| (*p.config).clone());
    config.layout = Layout::Viewport(180.);
    apply(&f.view, cx, vec![Op::SetDocument(f.node, config)]);
    cx.update(|w, _| w.activate_window());
    ax(cx, "Document content", gpui::accesskit::Action::Focus);
    step(cx, "tab", "Profile scroll top");
    // The independent inner viewport clips the bottom control. Reader traversal
    // must leave the composite instead of focusing an invisible target or looping.
    step(cx, "tab", "Collapse");
    // The host reenters at the reader's composite anchor.
    step(cx, "shift-tab", "Document content");
    step(cx, "tab", "Profile scroll top");
    step(cx, "shift-tab", "Document content");
    assert!(events(&f).is_empty());

    let tree = cx.a11y_tree().unwrap();
    let bounds = tree
        .nodes
        .iter()
        .find(|(_, n)| n.label() == Some("Profile scroll top"))
        .unwrap()
        .1
        .bounds()
        .unwrap();
    let scale = cx.update(|w, _| f64::from(w.scale_factor()));
    let position = gpui::point(
        px(((bounds.x0 + 10.) / scale) as f32),
        px(((bounds.y0 + 10.) / scale) as f32),
    );
    cx.simulate_mouse_move(position, None, Default::default());
    cx.simulate_event(gpui::ScrollWheelEvent {
        position,
        delta: gpui::ScrollDelta::Pixels(gpui::point(px(0.), px(-400.))),
        modifiers: Default::default(),
        touch_phase: gpui::TouchPhase::Moved,
    });
    draw(cx);
    ax(cx, "Document content", gpui::accesskit::Action::Focus);
    step(cx, "tab", "Profile scroll bottom");
    step(cx, "tab", "Collapse");
    step(cx, "shift-tab", "Document content");
    step(cx, "tab", "Profile scroll bottom");
    assert!(events(&f).is_empty());
    let key = gpui::Keystroke::parse("enter").unwrap();
    cx.simulate_event(gpui::KeyDownEvent {
        keystroke: key.clone(),
        is_held: false,
        prefer_character_input: false,
    });
    cx.simulate_event(gpui::KeyUpEvent { keystroke: key });
    draw(cx);
    let emitted = events(&f);
    assert_eq!(emitted.len(), 1);
    assert_eq!(
        (emitted[0].source_generation, emitted[0].source_revision),
        (2, 2)
    );
    assert_eq!(
        emitted[0].signal,
        Signal::Data(gpuio_protocol::extension::Payload(vec![12]))
    );
}
