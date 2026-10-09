//! Native clamp prerequisites on TestPlatform; public bridge integration follows.
use super::*;
use gpui::{TestAppContext, VisualTestContext};

struct Preview {
    state: Entity<TextViewState>,
    limit: Option<usize>,
    scrollable: bool,
    code_focus: gpui::FocusHandle,
    visits: Arc<std::sync::Mutex<Vec<String>>>,
}
impl Render for Preview {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        let visits = self.visits.clone();
        let code_visits = self.visits.clone();
        let code_focus = self.code_focus.clone();
        div()
            .w(px(240.))
            .child(
                TextView::new(&self.state)
                    .scrollable(self.scrollable)
                    .code_block_actions(move |_, _, _| {
                        let visits = code_visits.clone();
                        let clicks = code_visits.clone();
                        gpui_base::Button::new("preview-code-action")
                            .aria_label("Preview code action")
                            .track_focus(&code_focus)
                            .on_click(move |_, _, _| {
                                clicks.lock().unwrap().push("code-click".to_string());
                            })
                            .child("Copy")
                            .on_a11y_action(gpui::AccessibleAction::Click, move |_, _, _| {
                                visits.lock().unwrap().push("code-action".to_string());
                            })
                    })
                    .when_some(self.limit, |text, limit| text.max_lines(limit))
                    .on_link_click(move |url, _, _, _| {
                        visits.lock().unwrap().push(url.to_string())
                    }),
            )
            .child(
                div()
                    .id("neighbor")
                    .role(gpui::Role::Label)
                    .aria_label("Neighbor")
                    .h(px(24.)),
            )
    }
}
fn draw(cx: &mut VisualTestContext) {
    cx.run_until_parked();
    cx.update(|window, cx| window.draw(cx).clear(cx));
    cx.run_until_parked();
}
fn scene(cx: &mut Context<Preview>) -> Preview {
    let state = cx.new(|cx| {
        let mut state = TextViewState::externally_prepared(cx);
        state.set_prepared(
            gpui_base::text::PreparedMarkdown::parse(
                "[Shown](preview:shown)\n\nfiller\n\nmore filler\n\n[Hidden](preview:hidden)\n\n```ocaml\nlet x = 1\n```",
                Default::default(),
            )
            .unwrap(),
            None,
            cx,
        );
        state
    });
    Preview {
        state,
        limit: Some(2),
        scrollable: false,
        code_focus: cx.focus_handle(),
        visits: Arc::default(),
    }
}

#[test]
fn clamped_document_keyboard_does_not_activate_hidden_links() {
    let mut app = TestAppContext::single();
    app.update(gpui_base::init);
    let (view, cx) = app.add_window_view(|_, cx| scene(cx));
    draw(cx);
    let state = view.read_with(cx, |v, _| v.state.clone());
    assert!(state.read_with(cx, |s, _| s.is_clamped()));
    cx.update(|window, cx| {
        window.activate_window();
        let focus = state.read(cx).focus_handle().clone();
        window.focus(&focus, cx);
    });
    cx.simulate_keystrokes("tab enter");
    draw(cx);
    assert_eq!(
        view.read_with(cx, |v, _| v.visits.lock().unwrap().clone()),
        ["preview:shown"]
    );
    cx.simulate_keystrokes("tab enter");
    draw(cx);
    assert_eq!(
        view.read_with(cx, |v, _| v.visits.lock().unwrap().clone()),
        ["preview:shown"],
        "the clamped-out link cannot be reached or activated through keyboard traversal"
    );
    cx.simulate_keystrokes("shift-tab enter");
    draw(cx);
    assert_eq!(
        view.read_with(cx, |v, _| v.visits.lock().unwrap().clone()),
        ["preview:shown", "preview:shown"],
        "reverse traversal skips the clipped suffix too"
    );
}

#[test]
fn expanding_restores_links_and_reclamping_clears_a_hidden_active_link() {
    let mut app = TestAppContext::single();
    app.update(gpui_base::init);
    let (view, cx) = app.add_window_view(|_, cx| scene(cx));
    view.update(cx, |v, cx| {
        v.limit = None;
        cx.notify();
    });
    draw(cx);
    let state = view.read_with(cx, |v, _| v.state.clone());
    cx.update(|window, cx| {
        window.activate_window();
        let focus = state.read(cx).focus_handle().clone();
        window.focus(&focus, cx);
    });
    cx.simulate_keystrokes("tab tab enter");
    draw(cx);
    assert_eq!(
        view.read_with(cx, |v, _| v.visits.lock().unwrap().clone()),
        ["preview:hidden"]
    );
    view.update(cx, |v, cx| {
        v.limit = Some(2);
        cx.notify();
    });
    draw(cx);
    cx.simulate_keystrokes("enter");
    assert_eq!(
        view.read_with(cx, |v, _| v.visits.lock().unwrap().clone()),
        ["preview:hidden"]
    );
    cx.simulate_keystrokes("tab enter");
    assert_eq!(
        view.read_with(cx, |v, _| v.visits.lock().unwrap().clone()),
        ["preview:hidden", "preview:shown"]
    );
}

#[test]
fn scrollable_mode_clears_clamping_and_retains_logical_link_navigation() {
    let mut app = TestAppContext::single();
    app.update(gpui_base::init);
    let (view, cx) = app.add_window_view(|_, cx| scene(cx));
    draw(cx);
    let state = view.read_with(cx, |v, _| v.state.clone());
    assert!(state.read_with(cx, |s, _| s.is_clamped()));
    view.update(cx, |v, cx| {
        v.scrollable = true;
        cx.notify();
    });
    draw(cx);
    assert!(!state.read_with(cx, |s, _| s.is_clamped()));
    cx.update(|window, cx| {
        window.activate_window();
        let focus = state.read(cx).focus_handle().clone();
        window.focus(&focus, cx);
    });
    cx.simulate_keystrokes("tab tab enter");
    assert_eq!(
        view.read_with(cx, |v, _| v.visits.lock().unwrap().clone()),
        ["preview:hidden"]
    );
}

#[test]
fn clamped_links_are_hidden_from_accessibility_and_reject_actions() {
    let mut app = TestAppContext::single();
    app.update(gpui_base::init);
    let (view, cx) = app.add_window_view(|_, cx| scene(cx));
    cx.simulate_a11y_active(true);
    draw(cx);
    let nodes = cx.a11y_tree().unwrap().nodes;
    let hidden = nodes
        .iter()
        .find(|(_, n)| n.label() == Some("Hidden"))
        .unwrap()
        .0;
    let mut cursor = hidden;
    let mut inaccessible = false;
    loop {
        let node = &nodes.iter().find(|(id, _)| *id == cursor).unwrap().1;
        inaccessible |= node.is_hidden();
        let Some((parent, _)) = nodes.iter().find(|(_, n)| n.children().contains(&cursor)) else {
            break;
        };
        cursor = *parent;
    }
    assert!(
        inaccessible,
        "the clipped link must have a hidden AX ancestor"
    );
    assert!(
        !nodes
            .iter()
            .find(|(_, n)| n.label() == Some("Neighbor"))
            .unwrap()
            .1
            .is_hidden()
    );
    for action in [
        gpui::accesskit::Action::Focus,
        gpui::accesskit::Action::Click,
    ] {
        cx.simulate_a11y_action(gpui::accesskit::ActionRequest {
            action,
            target_node: hidden,
            target_tree: gpui::accesskit::TreeId::ROOT,
            data: None,
        });
        draw(cx);
    }
    cx.simulate_keystrokes("enter");
    assert!(view.read_with(cx, |v, _| v.visits.lock().unwrap().is_empty()));
    let code_action = nodes
        .iter()
        .find(|(_, n)| n.label() == Some("Preview code action"))
        .unwrap()
        .0;
    cx.simulate_a11y_action(gpui::accesskit::ActionRequest {
        action: gpui::accesskit::Action::Click,
        target_node: code_action,
        target_tree: gpui::accesskit::TreeId::ROOT,
        data: None,
    });
    draw(cx);
    assert!(
        view.read_with(cx, |v, _| v.visits.lock().unwrap().is_empty()),
        "clipped code controls cannot invoke accessibility callbacks"
    );
    let shown = nodes
        .iter()
        .find(|(_, n)| n.label() == Some("Shown"))
        .unwrap()
        .0;
    cx.simulate_a11y_action(gpui::accesskit::ActionRequest {
        action: gpui::accesskit::Action::Click,
        target_node: shown,
        target_tree: gpui::accesskit::TreeId::ROOT,
        data: None,
    });
    draw(cx);
    assert_eq!(
        view.read_with(cx, |v, _| v.visits.lock().unwrap().clone()),
        ["preview:shown"]
    );
    view.update(cx, |v, cx| {
        v.limit = None;
        cx.notify();
    });
    draw(cx);
    let expanded = cx.a11y_tree().unwrap().nodes;
    let expanded_link = expanded
        .iter()
        .find(|(_, n)| n.label() == Some("Hidden"))
        .unwrap();
    assert_eq!(
        expanded_link.0, hidden,
        "expansion preserves the semantic link identity"
    );
    assert!(!expanded_link.1.is_hidden());
    cx.simulate_a11y_action(gpui::accesskit::ActionRequest {
        action: gpui::accesskit::Action::Click,
        target_node: hidden,
        target_tree: gpui::accesskit::TreeId::ROOT,
        data: None,
    });
    draw(cx);
    assert_eq!(
        view.read_with(cx, |v, _| v.visits.lock().unwrap().clone()),
        ["preview:shown", "preview:hidden"]
    );
    cx.simulate_a11y_action(gpui::accesskit::ActionRequest {
        action: gpui::accesskit::Action::Click,
        target_node: code_action,
        target_tree: gpui::accesskit::TreeId::ROOT,
        data: None,
    });
    draw(cx);
    assert_eq!(
        view.read_with(cx, |v, _| v.visits.lock().unwrap().clone()),
        ["preview:shown", "preview:hidden", "code-action"],
        "expansion restores the custom control's accessibility callback"
    );
}

#[test]
fn replacing_a_clamped_document_with_short_content_clears_overflow() {
    let mut app = TestAppContext::single();
    app.update(gpui_base::init);
    let (view, cx) = app.add_window_view(|_, cx| scene(cx));
    draw(cx);
    let state = view.read_with(cx, |v, _| v.state.clone());
    assert!(state.read_with(cx, |s, _| s.is_clamped()));
    state.update(cx, |s, cx| {
        s.set_prepared(
            gpui_base::text::PreparedMarkdown::parse("Short", Default::default()).unwrap(),
            None,
            cx,
        );
    });
    draw(cx);
    assert!(!state.read_with(cx, |s, _| s.is_clamped()));
}

#[test]
fn whole_line_clip_restricts_pointer_hit_testing_at_the_preview_edge() {
    let mut app = TestAppContext::single();
    app.update(gpui_base::init);
    let (view, cx) = app.add_window_view(|_, cx| scene(cx));
    let state = view.read_with(cx, |v, _| v.state.clone());
    state.update(cx, |s, cx| {
        s.set_prepared(
            gpui_base::text::PreparedMarkdown::parse(
                "Visible\n\n[Edge](preview:edge)\n\nTail",
                Default::default(),
            )
            .unwrap(),
            None,
            cx,
        );
    });
    cx.simulate_a11y_active(true);
    draw(cx);
    let nodes = cx.a11y_tree().unwrap().nodes;
    let edge = nodes
        .iter()
        .find(|(_, n)| n.label() == Some("Edge"))
        .unwrap();
    assert!(
        edge.1.is_hidden(),
        "the second paragraph straddles the two-line budget"
    );
    let bounds = edge.1.bounds().unwrap();
    let scale = cx.update(|window, _| f64::from(window.scale_factor()));
    cx.simulate_click(
        gpui::point(
            px(((bounds.x0 + bounds.x1) / (2. * scale)) as f32),
            px(((bounds.y0 + 1.) / scale) as f32),
        ),
        gpui::Modifiers::default(),
    );
    assert!(view.read_with(cx, |v, _| v.visits.lock().unwrap().is_empty()));
    view.update(cx, |v, cx| {
        v.limit = None;
        cx.notify();
    });
    draw(cx);
    let nodes = cx.a11y_tree().unwrap().nodes;
    let bounds = nodes
        .iter()
        .find(|(_, n)| n.label() == Some("Edge"))
        .unwrap()
        .1
        .bounds()
        .unwrap();
    cx.simulate_click(
        gpui::point(
            px(((bounds.x0 + bounds.x1) / (2. * scale)) as f32),
            px(((bounds.y0 + bounds.y1) / (2. * scale)) as f32),
        ),
        gpui::Modifiers::default(),
    );
    assert_eq!(
        view.read_with(cx, |v, _| v.visits.lock().unwrap().clone()),
        ["preview:edge"]
    );
}

#[test]
fn removing_a_document_clamp_clears_the_overflow_observation() {
    let mut app = TestAppContext::single();
    app.update(gpui_base::init);
    let (view, cx) = app.add_window_view(|_, cx| scene(cx));
    draw(cx);
    let state = view.read_with(cx, |v, _| v.state.clone());
    assert!(state.read_with(cx, |s, _| s.is_clamped()));
    view.update(cx, |v, cx| {
        v.limit = None;
        cx.notify();
    });
    draw(cx);
    assert!(
        !state.read_with(cx, |s, _| s.is_clamped()),
        "overflow describes the latest painted frame after expanding"
    );
}

#[test]
fn reclamping_previously_focused_code_control_prevents_keyboard_activation() {
    let mut app = TestAppContext::single();
    app.update(gpui_base::init);
    let (view, cx) = app.add_window_view(|_, cx| scene(cx));
    view.update(cx, |v, cx| {
        v.limit = None;
        cx.notify();
    });
    draw(cx);
    cx.update(|window, cx| {
        window.activate_window();
        let focus = view.read(cx).code_focus.clone();
        window.focus(&focus, cx);
    });
    draw(cx);
    let press = |cx: &mut VisualTestContext| {
        let keystroke = gpui::Keystroke::parse("enter").unwrap();
        cx.simulate_event(gpui::KeyDownEvent {
            keystroke: keystroke.clone(),
            is_held: false,
            prefer_character_input: false,
        });
        cx.simulate_event(gpui::KeyUpEvent { keystroke });
    };
    press(cx);
    assert_eq!(
        view.read_with(cx, |v, _| v.visits.lock().unwrap().clone()),
        ["code-click"]
    );
    view.update(cx, |v, cx| {
        v.limit = Some(2);
        cx.notify();
    });
    draw(cx);
    press(cx);
    assert_eq!(
        view.read_with(cx, |v, _| v.visits.lock().unwrap().clone()),
        ["code-click"],
        "a clipped previously focused code button must not accept Enter"
    );
    cx.update(|window, cx| {
        window.focus_next(cx);
        assert!(!view.read(cx).code_focus.is_focused(window));
    });
    view.update(cx, |v, cx| {
        v.limit = None;
        cx.notify();
    });
    draw(cx);
    cx.update(|window, cx| {
        let focus = view.read(cx).code_focus.clone();
        window.focus(&focus, cx);
    });
    draw(cx);
    press(cx);
    assert_eq!(
        view.read_with(cx, |v, _| v.visits.lock().unwrap().clone()),
        ["code-click", "code-click"]
    );
    let keystroke = gpui::Keystroke::parse("enter").unwrap();
    cx.simulate_event(gpui::KeyDownEvent {
        keystroke: keystroke.clone(),
        is_held: false,
        prefer_character_input: false,
    });
    view.update(cx, |v, cx| {
        v.limit = Some(2);
        cx.notify();
    });
    draw(cx);
    view.update(cx, |v, cx| {
        v.limit = None;
        cx.notify();
    });
    draw(cx);
    cx.simulate_event(gpui::KeyUpEvent { keystroke });
    assert_eq!(
        view.read_with(cx, |v, _| v.visits.lock().unwrap().clone()),
        ["code-click", "code-click"],
        "clipping cancels a pending activation across later expansion"
    );
}

#[test]
fn html_reader_keeps_distinct_anchors_and_fences_selection_on_replacement() {
    let mut app = TestAppContext::single();
    app.update(gpui_base::init);
    let (view, cx) = app.add_window_view(|_, cx| {
        let mut preview = scene(cx);
        preview.limit = None;
        preview.state.update(cx, |state,cx| {
            state.set_prepared(gpui_base::text::PreparedText::parse_html(
                "<p><a href='reader:first'>First <b>bold</b></a></p><p><a href='reader:second'><img src='file:///never-load' alt='Second'></a></p><p><a href='reader:first'>Repeated destination</a></p>",
                crate::document_markdown::extensions(Default::default()),
                crate::document_markdown::html_image,
            ).unwrap(),None,cx);
        });
        preview
    });
    draw(cx);
    let state = view.read_with(cx, |view, _| view.state.clone());
    cx.update(|window, cx| {
        window.activate_window();
        let focus = state.read(cx).focus_handle().clone();
        window.focus(&focus, cx);
    });
    for destination in ["reader:first", "reader:second", "reader:first"] {
        cx.simulate_keystrokes("tab enter");
        draw(cx);
        assert_eq!(
            view.read_with(cx, |view, _| view.visits.lock().unwrap().last().cloned()),
            Some(destination.to_string())
        );
    }
    assert_eq!(
        view.read_with(cx, |view, _| view.visits.lock().unwrap().len()),
        3
    );
    state.update(cx, |state, cx| {
        state.select_all(cx);
        assert!(state.has_local_selection());
        state.set_prepared(
            gpui_base::text::PreparedText::parse_html(
                "<p>Replacement</p>",
                crate::document_markdown::extensions(Default::default()),
                crate::document_markdown::html_image,
            )
            .unwrap(),
            Some(999),
            cx,
        );
        assert!(!state.has_local_selection());
    });
    draw(cx);
    cx.simulate_keystrokes("enter");
    draw(cx);
    assert_eq!(
        view.read_with(cx, |view, _| view.visits.lock().unwrap().len()),
        3,
        "replacement must retire old link navigation"
    );
}
