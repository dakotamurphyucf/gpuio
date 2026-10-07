//! Exact selection-head reveal through the production flow-document host.
use super::*;
use gpuio_protocol::v1::{Field, Length, Style};

fn mount_flow(app: &mut TestAppContext, nested: bool) -> (Fixture, &mut VisualTestContext) {
    let source = format!("```txt\nbegin\n{}target\n```", "earlier line\n".repeat(100));
    let (f, cx) = mount_source(app, Mode::Markdown, &source);
    let scroll = NodeId::from_parts(1, 1).unwrap();
    let mut style = vec![
        Field::Width(Length::Px(440.)),
        Field::Height(Length::Px(200.)),
        Field::OverflowY(3),
        Field::Shrink(0.),
    ];
    if nested {
        style.extend([
            Field::MarginLeft(Length::Px(300.)),
            Field::MarginTop(Length::Px(320.)),
        ]);
    }
    let mut ops = vec![
        Op::Create(scroll, Kind::Container, String::new(), None),
        Op::SetStyle(scroll, vec![Style::Fields(style)]),
        Op::SetStyle(f.node, vec![Style::Fields(vec![Field::Shrink(0.)])]),
        Op::SetRoot(None),
        Op::Splice(scroll, 0, 0, vec![f.node]),
    ];
    if nested {
        let outer = NodeId::from_parts(2, 1).unwrap();
        ops.extend([
            Op::Create(outer, Kind::Container, String::new(), None),
            Op::SetStyle(
                outer,
                vec![Style::Fields(vec![
                    Field::Width(Length::Px(240.)),
                    Field::Height(Length::Px(180.)),
                    Field::OverflowX(3),
                    Field::OverflowY(3),
                ])],
            ),
            Op::Splice(outer, 0, 0, vec![scroll]),
            Op::SetRoot(Some(outer)),
        ]);
    } else {
        ops.push(Op::SetRoot(Some(scroll)));
    }
    apply(&f.view, cx, ops);
    ready(&f.presentation, cx);
    draw(cx);
    (f, cx)
}
fn document(cx: &mut VisualTestContext) -> gpui::accesskit::NodeId {
    cx.a11y_tree()
        .unwrap()
        .nodes
        .into_iter()
        .find(|(_, node)| {
            node.role() == gpui::accesskit::Role::Document
                && node.supports_action(gpui::accesskit::Action::SetTextSelection)
        })
        .unwrap()
        .0
}
fn range(
    f: &Fixture,
    cx: &mut VisualTestContext,
    needle: &str,
    last_glyph: bool,
) -> gpui::accesskit::TextSelection {
    let text = f
        .presentation
        .read_with(cx, |p, _| p.markdown.clone().unwrap());
    cx.update(|window, cx| {
        let state = text.read(cx);
        let rendered = state.rendered_text().unwrap();
        let start = rendered.text().find(needle).unwrap();
        let end = start + needle.len() - usize::from(last_glyph);
        gpui::accesskit::TextSelection {
            anchor: state
                .rendered_accessible_text_position(window, &rendered.position(start).unwrap())
                .unwrap(),
            focus: state
                .rendered_accessible_text_position(window, &rendered.position(end).unwrap())
                .unwrap(),
        }
    })
}
fn select(f: &Fixture, cx: &mut VisualTestContext, needle: &str) {
    let selection = range(f, cx, needle, false);
    let document = document(cx);
    cx.simulate_a11y_action(gpui::accesskit::ActionRequest {
        action: gpui::accesskit::Action::SetTextSelection,
        target_node: document,
        target_tree: gpui::accesskit::TreeId::ROOT,
        data: Some(gpui::accesskit::ActionData::SetTextSelection(selection)),
    });
    draw(cx);
    draw(cx);
    assert_eq!(cx.update(gpui_base::TextSelection::selected_text), needle);
}
fn visible(f: &Fixture, cx: &mut VisualTestContext, needle: &str, nested: bool) {
    // The logical end may belong to a geometry-free separator. Inspect the last
    // selected glyph without changing the actual selected range or Copy.
    let position = range(f, cx, needle, true).focus;
    let tree = cx.a11y_tree().unwrap();
    let bounds = tree
        .nodes
        .iter()
        .find(|(id, _)| *id == position.node)
        .unwrap()
        .1
        .bounds()
        .expect("last glyph has actual bounds");
    let scale = cx.update(|window, _| f64::from(window.scale_factor()));
    f.view.read_with(cx, |view, _| {
        for slot in 1..=if nested { 2 } else { 1 } {
            let viewport = view.scrolls[&NodeId::from_parts(slot, 1).unwrap()]
                .mask
                .get()
                .unwrap();
            assert!(
                bounds.x0 >= f64::from(f32::from(viewport.left())) * scale - 0.1
                    && bounds.x1 <= f64::from(f32::from(viewport.right())) * scale + 0.1
                    && bounds.y0 >= f64::from(f32::from(viewport.top())) * scale - 0.1
                    && bounds.y1 <= f64::from(f32::from(viewport.bottom())) * scale + 0.1,
                "last glyph {bounds:?} outside viewport {slot}: {viewport:?}"
            );
        }
    });
}
#[test]
fn flow_selection_reveals_precise_head_again_without_a_focus_change() {
    for nested in [false, true] {
        let mut app = TestAppContext::single();
        let (f, cx) = mount_flow(&mut app, nested);
        for needle in ["target", "begin", "target"] {
            select(&f, cx, needle);
            visible(&f, cx, needle, nested);
        }
        let state = f.view.read_with(cx, |view, _| {
            view.scrolls[&NodeId::from_parts(1, 1).unwrap()].clone()
        });
        assert!(state.handle.offset().y < px(-1000.));
        // A manual native scroll after a successful reveal must not snap back.
        state.handle.set_offset(gpui::point(px(0.), px(-300.)));
        draw(cx);
        draw(cx);
        assert_eq!(state.handle.offset().y, px(-300.));
    }
}
#[test]
fn newer_wheel_cancels_flow_reveal_through_the_host() {
    let mut app = TestAppContext::single();
    let (f, cx) = mount_flow(&mut app, false);
    let state = f.view.read_with(cx, |view, _| {
        view.scrolls[&NodeId::from_parts(1, 1).unwrap()].clone()
    });
    let capture = state.clone();
    let observed = Rc::new(RefCell::new(None));
    let seen = observed.clone();
    let position = gpui::point(px(40.), px(40.));
    cx.simulate_mouse_move(position, None, Default::default());
    draw(cx);
    let document = document(cx);
    cx.update(|window, _| {
        window.on_a11y_action(
            document,
            gpui::accesskit::Action::SetTextSelection,
            move |_, window, cx| {
                window.dispatch_event(
                    gpui::PlatformInput::ScrollWheel(gpui::ScrollWheelEvent {
                        position,
                        delta: gpui::ScrollDelta::Pixels(gpui::point(px(0.), px(-32.))),
                        modifiers: Default::default(),
                        touch_phase: gpui::TouchPhase::Moved,
                    }),
                    cx,
                );
                *seen.borrow_mut() = Some(capture.handle.offset());
            },
        )
    });
    select(&f, cx, "target");
    let user_offset = observed.borrow().expect("newer wheel delivered");
    assert!(user_offset.y < px(0.), "wheel actually scrolls");
    assert_eq!(
        state.handle.offset(),
        user_offset,
        "host must preserve newer wheel offset"
    );
}
