//! Atomic native owners keep their selection boundaries and child controls.
use super::*;
use gpui_base::text::{MarkdownNode, MarkdownPlugin, MarkdownPresentation};

#[derive(Clone)]
struct Card(&'static str);
impl MarkdownPlugin for Card {
    fn name(&self) -> &str {
        "selection-card"
    }
    fn is_block(&self) -> bool {
        true
    }
    fn parse(
        &self,
        node: &gpui_base::text::markdown_ast::Node,
        context: &gpui_base::text::MarkdownParseContext<'_>,
    ) -> Option<MarkdownNode> {
        matches!(node, gpui_base::text::markdown_ast::Node::Blockquote(_)).then(|| {
            MarkdownNode::new("selection-card", ())
                .text(self.0)
                .markdown(context.node_source(node).unwrap_or_default())
        })
    }
    fn presentation(&self, _: &MarkdownNode) -> MarkdownPresentation {
        MarkdownPresentation::Opaque
    }
}

struct Cards {
    text: Entity<TextViewState>,
    extensions: MarkdownExtensions,
    format: gpui_base::text::SelectionFormat,
}
impl Render for Cards {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div()
            .w(px(440.))
            .h(px(200.))
            .child(gpui_base::TextSelectionLayer)
            .child(
                TextView::new(&self.text)
                    .scrollable(true)
                    .selection_format(self.format)
                    .markdown_extensions(self.extensions.clone()),
            )
    }
}

#[test]
fn atomic_accessibility_requests_reject_interiors_and_reveal_both_object_edges() {
    for alternative in ["card alternative", ""] {
        let mut app = TestAppContext::single();
        app.update(gpui_base::init);
        let clicks = Arc::new(AtomicUsize::new(0));
        let clicked = clicks.clone();
        let extensions = MarkdownExtensions::default()
            .plugin(Card(alternative))
            .block_renderer("selection-card", move |_, _, _| {
                let clicks = clicked.clone();
                div()
                    .id("atomic-card")
                    .h(px(700.))
                    .w(px(300.))
                    .role(accesskit::Role::Group)
                    .aria_label("Atomic card")
                    .child(
                        div()
                            .id("card-control")
                            .role(accesskit::Role::Button)
                            .aria_label("Card action")
                            .h(px(24.))
                            .on_a11y_action(gpui::AccessibleAction::Click, move |_, _, _| {
                                clicks.fetch_add(1, Ordering::Relaxed);
                            }),
                    )
            });
        let source = format!("{}> atomic source", "Earlier paragraph\n\n".repeat(40));
        let (view, cx) = app.add_window_view(|_, cx| Cards {
            text: cx.new(|cx| {
                let mut text = TextViewState::externally_prepared(cx);
                text.set_prepared(
                    PreparedText::parse(&source, extensions.clone()).unwrap(),
                    None,
                    cx,
                );
                text
            }),
            extensions: extensions.clone(),
            format: gpui_base::text::SelectionFormat::Plain,
        });
        let text = view.read_with(cx, |view, _| view.text.clone());
        cx.simulate_a11y_active(true);
        draw(cx);
        let document = cx
            .a11y_tree()
            .unwrap()
            .nodes
            .into_iter()
            .find(|(_, node)| node.role() == accesskit::Role::Document)
            .unwrap()
            .0;
        let range = |cx: &mut VisualTestContext, backwards| {
            cx.update(|window, cx| {
                let state = text.read(cx);
                let projection = state.rendered_text().unwrap();
                let part = projection
                    .parts()
                    .iter()
                    .position(|part| part.is_atomic())
                    .unwrap();
                let selected = projection.selection_for_part(part).unwrap();
                let anchor = state
                    .rendered_accessible_text_position(window, selected.anchor())
                    .unwrap();
                let focus = state
                    .rendered_accessible_text_position(window, selected.head())
                    .unwrap();
                if backwards {
                    accesskit::TextSelection {
                        anchor: focus,
                        focus: anchor,
                    }
                } else {
                    accesskit::TextSelection { anchor, focus }
                }
            })
        };
        let send = |cx: &mut VisualTestContext, selection| {
            cx.simulate_a11y_action(accesskit::ActionRequest {
                action: accesskit::Action::SetTextSelection,
                target_node: document,
                target_tree: accesskit::TreeId::ROOT,
                data: Some(accesskit::ActionData::SetTextSelection(selection)),
            });
            draw(cx);
        };
        if !alternative.is_empty() {
            let mut interior = range(cx, false);
            interior.anchor.character_index = 1;
            send(cx, interior);
            assert!(text.read_with(cx, |state, _| state.rendered_selection().is_none()));
        }
        for backwards in [false, true] {
            let selected = range(cx, backwards);
            send(cx, selected);
            assert_eq!(
                cx.update(gpui_base::TextSelection::selected_text),
                alternative
            );
            text.read_with(cx, |state, _| {
                let selected = state
                    .rendered_selection()
                    .expect("accepted whole atomic object");
                assert!(!selected.is_collapsed());
                assert_eq!(selected.is_backward(), backwards);
            });
            let tree = cx.a11y_tree().unwrap();
            let bounds = tree
                .nodes
                .iter()
                .find(|(_, node)| node.label() == Some("Atomic card"))
                .unwrap_or_else(|| {
                    panic!(
                        "alternative={alternative:?} backwards={backwards}; labels={:?}",
                        tree.nodes
                            .iter()
                            .filter_map(|(_, node)| node.label().map(|label| (
                                label,
                                node.role(),
                                node.bounds()
                            )))
                            .collect::<Vec<_>>()
                    )
                })
                .1
                .bounds()
                .unwrap();
            let scale = cx.update(|window, _| f64::from(window.scale_factor()));
            let edge = if backwards { bounds.y0 } else { bounds.y1 };
            assert!(
                edge >= 0. && edge <= 200. * scale + 0.1,
                "alternative={alternative:?}, backwards={backwards}, bounds={bounds:?}"
            );
        }
        let before = text.read_with(cx, |state, _| {
            let selected = state.rendered_selection().unwrap();
            (
                selected.anchor().content_position(),
                selected.head().content_position(),
            )
        });
        view.update(cx, |view, cx| {
            view.format = gpui_base::text::SelectionFormat::Source;
            cx.notify();
        });
        draw(cx);
        assert_eq!(
            cx.update(gpui_base::TextSelection::selected_text),
            "> atomic source"
        );
        text.read_with(cx, |state, _| {
            let selected = state.rendered_selection().unwrap();
            assert_eq!(
                (
                    selected.anchor().content_position(),
                    selected.head().content_position()
                ),
                before
            );
        });
        let tree = cx.a11y_tree().unwrap();
        let control = tree
            .nodes
            .iter()
            .find(|(_, node)| node.label() == Some("Card action"))
            .unwrap()
            .0;
        cx.simulate_a11y_action(accesskit::ActionRequest {
            action: accesskit::Action::Click,
            target_node: control,
            target_tree: accesskit::TreeId::ROOT,
            data: None,
        });
        draw(cx);
        assert_eq!(clicks.load(Ordering::Relaxed), 1);
    }
}

struct OuterScroll {
    text: Entity<TextViewState>,
    scroll: gpui::ListState,
}
impl Render for OuterScroll {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        let text = self.text.clone();
        div()
            .w(px(440.))
            .h(px(200.))
            .child(gpui_base::TextSelectionLayer)
            .child(
                gpui::list(self.scroll.clone(), move |_, _, _| {
                    TextView::new(&text)
                        .on_flow_reveal(|_, _, _| {})
                        .into_any_element()
                })
                .size_full(),
            )
    }
}

#[test]
fn later_input_cancels_accessibility_reveal_before_paint() {
    check_input_reveal(None);
    for input in [LaterInput::Wheel, LaterInput::Pointer, LaterInput::Key] {
        check_input_reveal(Some(input));
    }
}

#[derive(Clone, Copy, Debug)]
enum LaterInput {
    Wheel,
    Pointer,
    Key,
}

fn check_input_reveal(later_input: Option<LaterInput>) {
    let mut app = TestAppContext::single();
    app.update(gpui_base::init);
    let source = format!("```txt\n{}target\n```", "earlier line\n".repeat(100));
    let (view, cx) = app.add_window_view(|_, cx| OuterScroll {
        text: cx.new(|cx| {
            let mut text = TextViewState::externally_prepared(cx);
            text.set_prepared(
                PreparedText::parse(&source, MarkdownExtensions::default()).unwrap(),
                None,
                cx,
            );
            text
        }),
        scroll: gpui::ListState::new(1, gpui::ListAlignment::Top, px(0.)),
    });
    let (text, scroll) = view.read_with(cx, |view, _| (view.text.clone(), view.scroll.clone()));
    cx.simulate_a11y_active(true);
    draw(cx);
    let position = gpui::point(px(40.), px(40.));
    cx.simulate_mouse_move(position, None, Default::default());
    draw(cx);
    let start = text.read_with(cx, |state, _| {
        state
            .rendered_text()
            .unwrap()
            .text()
            .rfind("target")
            .unwrap()
    });
    let selection = native_range(&text, cx, start, start + 6);
    let document = cx
        .a11y_tree()
        .unwrap()
        .nodes
        .into_iter()
        .find(|(_, node)| node.role() == accesskit::Role::Document)
        .unwrap()
        .0;
    let observed = Arc::new(std::sync::Mutex::new(None));
    let capture = observed.clone();
    let scrolling = scroll.clone();
    // Runs after the real Document listener in the same native delivery, before
    // a repaint can consume the pending reveal. No test-only production API.
    if let Some(input) = later_input {
        cx.update(|window, _| {
            window.on_a11y_action(
                document,
                accesskit::Action::SetTextSelection,
                move |_, window, cx| {
                    let event = match input {
                        LaterInput::Wheel => {
                            gpui::PlatformInput::ScrollWheel(gpui::ScrollWheelEvent {
                                position,
                                delta: gpui::ScrollDelta::Pixels(gpui::point(px(0.), px(-32.))),
                                modifiers: Default::default(),
                                touch_phase: gpui::TouchPhase::Moved,
                            })
                        }
                        LaterInput::Pointer => {
                            gpui::PlatformInput::MouseDown(gpui::MouseDownEvent {
                                position,
                                button: gpui::MouseButton::Right,
                                modifiers: Default::default(),
                                click_count: 1,
                                first_mouse: false,
                            })
                        }
                        LaterInput::Key => gpui::PlatformInput::KeyDown(gpui::KeyDownEvent {
                            keystroke: gpui::Keystroke::parse("escape").unwrap(),
                            is_held: false,
                            prefer_character_input: false,
                        }),
                    };
                    window.dispatch_event(event, cx);
                    *capture.lock().unwrap() = Some(scrolling.scroll_px_offset_for_scrollbar());
                },
            )
        });
    }
    cx.simulate_a11y_action(accesskit::ActionRequest {
        action: accesskit::Action::SetTextSelection,
        target_node: document,
        target_tree: accesskit::TreeId::ROOT,
        data: Some(accesskit::ActionData::SetTextSelection(selection)),
    });
    draw(cx);
    if let Some(input) = later_input {
        let user_offset = observed.lock().unwrap().expect("later input delivered");
        if matches!(input, LaterInput::Wheel) {
            assert!(user_offset.y < px(0.), "wheel must actually scroll");
        }
        assert_eq!(
            scroll.scroll_px_offset_for_scrollbar(),
            user_offset,
            "old reveal must not override {input:?}"
        );
    } else {
        assert!(
            scroll.scroll_px_offset_for_scrollbar().y < px(-1000.),
            "positive control: without later input the head must be revealed"
        );
    }
    assert_eq!(cx.update(gpui_base::TextSelection::selected_text), "target");
}
