//! Native cache reuse must preserve semantic content and action routing.
#[path = "document_cached_owner_test.rs"]
mod owners;
#[path = "document_cached_registration_test.rs"]
mod registration;
#[path = "document_cached_text_test.rs"]
mod text;
use super::*;

struct CachedDocument {
    renders: Arc<AtomicUsize>,
    clicks: Arc<AtomicUsize>,
    focus: gpui::FocusHandle,
    label: &'static str,
    deferred: bool,
}
impl Render for CachedDocument {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        self.renders.fetch_add(1, Ordering::Relaxed);
        let clicks = self.clicks.clone();
        let button = div()
            .id("cached-control")
            .role(accesskit::Role::Button)
            .aria_label(self.label)
            .track_focus(&self.focus)
            .h(px(30.))
            .w(px(180.))
            .on_a11y_action(gpui::AccessibleAction::Click, move |_, _, _| {
                clicks.fetch_add(1, Ordering::Relaxed);
            });
        let button = if self.deferred {
            gpui::deferred(
                div()
                    .id("outer-deferred")
                    .role(accesskit::Role::Group)
                    .child(gpui::deferred(button)),
            )
            .into_any_element()
        } else {
            button.into_any_element()
        };
        let claim = Rc::new(std::cell::Cell::new(None));
        let published = claim.clone();
        div()
            .id("cached-document")
            .role(accesskit::Role::Document)
            .size_full()
            .a11y_synthetic_children(move |builder| {
                let run = builder.synthetic_node_id("selected-run");
                let mut node = accesskit::Node::new(accesskit::Role::TextRun);
                node.set_value("Cached 世界");
                node.set_character_lengths(vec![1, 1, 1, 1, 1, 1, 1, 3, 3]);
                assert!(builder.push_child(run, node));
                claim.set(Some((
                    builder.parent_id(),
                    accesskit::TextSelection {
                        anchor: accesskit::TextPosition {
                            node: run,
                            character_index: 0,
                        },
                        focus: accesskit::TextPosition {
                            node: run,
                            character_index: 9,
                        },
                    },
                )));
            })
            .child(
                gpui::canvas(
                    |_, _, _| (),
                    move |_, _, window, _| {
                        if let Some((document, selection)) = published.get() {
                            assert!(window.publish_document_selection(document, Some(selection)));
                        }
                    },
                )
                .size_full(),
            )
            .child(button)
    }
}
struct Parent {
    child: Option<Entity<CachedDocument>>,
    hidden: bool,
    disabled: bool,
}
impl Render for Parent {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        let hidden = self.hidden;
        let disabled = self.disabled;
        div()
            .id("cache-parent")
            .role(accesskit::Role::Group)
            .w(px(440.))
            .h(px(200.))
            .a11y_synthetic_children(move |builder| {
                if hidden {
                    builder.parent_node().set_hidden();
                }
                if disabled {
                    builder.parent_node().set_disabled();
                }
            })
            .when_some(self.child.clone(), |this, child| {
                this.child(child.cached(gpui::StyleRefinement::default().size_full()))
            })
    }
}
#[test]
fn cached_document_keeps_semantics_and_actions_without_rendering_again() {
    let mut app = TestAppContext::single();
    app.update(gpui_base::init);
    let renders = Arc::new(AtomicUsize::new(0));
    let clicks = Arc::new(AtomicUsize::new(0));
    let (parent, cx) = app.add_window_view(|_, cx| Parent {
        child: Some(cx.new(|cx| CachedDocument {
            renders: renders.clone(),
            clicks: clicks.clone(),
            focus: cx.focus_handle(),
            label: "Cached action",
            deferred: false,
        })),
        hidden: false,
        disabled: false,
    });
    cx.simulate_a11y_active(true);
    cx.run_until_parked();
    cx.update(|window, cx| {
        window.refresh();
        window.draw(cx).clear(cx);
    });
    let initial = cx.a11y_tree().unwrap();
    let control = initial
        .nodes
        .iter()
        .find(|(_, node)| node.label() == Some("Cached action"))
        .expect("initial control")
        .0;
    let document = initial
        .nodes
        .iter()
        .find(|(_, node)| node.role() == accesskit::Role::Document)
        .expect("initial document")
        .0;
    let selected = initial
        .nodes
        .iter()
        .find(|(id, _)| *id == document)
        .unwrap()
        .1
        .text_selection()
        .expect("painted selection claim");
    let count = renders.load(Ordering::Relaxed);
    parent.update(cx, |_, cx| cx.notify());
    draw(cx);
    assert_eq!(
        renders.load(Ordering::Relaxed),
        count,
        "test must actually reuse the cache"
    );
    let cached = cx.a11y_tree().unwrap();
    assert!(
        cached
            .nodes
            .iter()
            .any(|(id, node)| *id == control && node.label() == Some("Cached action")),
        "cached native button disappeared from accessibility"
    );
    assert!(
        cached
            .nodes
            .iter()
            .any(|(id, node)| *id == document && node.role() == accesskit::Role::Document),
        "cached document retained"
    );
    assert_eq!(
        cached
            .nodes
            .iter()
            .find(|(id, _)| *id == document)
            .unwrap()
            .1
            .text_selection(),
        Some(selected)
    );
    cx.update(|window, _| {
        assert!(
            window.accepts_document_selection(document, selected),
            "cached painted scope still authorizes valid positions"
        )
    });
    cx.simulate_a11y_action(accesskit::ActionRequest {
        action: accesskit::Action::Click,
        target_node: control,
        target_tree: accesskit::TreeId::ROOT,
        data: None,
    });
    cx.run_until_parked();
    assert_eq!(
        clicks.load(Ordering::Relaxed),
        1,
        "cached action listener retained"
    );
}

#[test]
fn cached_nodes_restore_actions_after_hidden_ancestry_and_retire_on_unmount() {
    let mut app = TestAppContext::single();
    app.update(gpui_base::init);
    let renders = Arc::new(AtomicUsize::new(0));
    let clicks = Arc::new(AtomicUsize::new(0));
    let (parent, cx) = app.add_window_view(|_, cx| Parent {
        child: Some(cx.new(|cx| CachedDocument {
            renders: renders.clone(),
            clicks: clicks.clone(),
            focus: cx.focus_handle(),
            label: "Cached action",
            deferred: false,
        })),
        hidden: false,
        disabled: false,
    });
    cx.simulate_a11y_active(true);
    cx.run_until_parked();
    cx.update(|window, cx| {
        window.refresh();
        window.draw(cx).clear(cx);
    });
    let control = cx
        .a11y_tree()
        .unwrap()
        .nodes
        .into_iter()
        .find(|(_, node)| node.label() == Some("Cached action"))
        .unwrap()
        .0;
    let count = renders.load(Ordering::Relaxed);
    let click = |cx: &mut VisualTestContext| {
        cx.simulate_a11y_action(accesskit::ActionRequest {
            action: accesskit::Action::Click,
            target_node: control,
            target_tree: accesskit::TreeId::ROOT,
            data: None,
        });
        cx.run_until_parked();
    };
    for hidden in [true, true, false, false] {
        parent.update(cx, |p, cx| {
            p.hidden = hidden;
            cx.notify();
        });
        draw(cx);
        assert_eq!(
            renders.load(Ordering::Relaxed),
            count,
            "postorder parent semantics must not require re-rendering the cached child"
        );
        let before = clicks.load(Ordering::Relaxed);
        click(cx);
        assert_eq!(
            clicks.load(Ordering::Relaxed),
            before + usize::from(!hidden)
        );
    }
    // Disabled inheritance must not contaminate the raw nodes used by replay.
    for disabled in [true, false] {
        parent.update(cx, |p, cx| {
            p.disabled = disabled;
            cx.notify();
        });
        draw(cx);
        let tree = cx.a11y_tree().unwrap();
        let node = &tree.nodes.iter().find(|(id, _)| *id == control).unwrap().1;
        assert_eq!(node.is_disabled(), disabled);
        assert_eq!(node.supports_action(accesskit::Action::Click), !disabled);
        let before = clicks.load(Ordering::Relaxed);
        click(cx);
        assert_eq!(
            clicks.load(Ordering::Relaxed),
            before + usize::from(!disabled)
        );
        assert_eq!(renders.load(Ordering::Relaxed), count);
    }
    parent.update(cx, |p, cx| {
        p.child = None;
        cx.notify();
    });
    draw(cx);
    draw(cx);
    assert!(
        !cx.a11y_tree()
            .unwrap()
            .nodes
            .iter()
            .any(|(id, _)| *id == control)
    );
    let before = clicks.load(Ordering::Relaxed);
    click(cx);
    assert_eq!(
        clicks.load(Ordering::Relaxed),
        before,
        "removed listeners cannot act"
    );
    assert_eq!(
        Arc::strong_count(&clicks),
        1,
        "both old frame listener stores released"
    );
}

#[test]
fn cached_deferred_content_tracks_activation_focus_and_dirty_source() {
    let mut app = TestAppContext::single();
    app.update(gpui_base::init);
    let renders = Arc::new(AtomicUsize::new(0));
    let clicks = Arc::new(AtomicUsize::new(0));
    let (parent, cx) = app.add_window_view(|_, cx| Parent {
        child: Some(cx.new(|cx| CachedDocument {
            renders: renders.clone(),
            clicks: clicks.clone(),
            focus: cx.focus_handle(),
            label: "Cached action",
            deferred: true,
        })),
        hidden: false,
        disabled: false,
    });
    draw(cx); // First populate the scene cache while accessibility is inactive.
    cx.simulate_a11y_active(true);
    draw(cx);
    let control = cx
        .a11y_tree()
        .unwrap()
        .nodes
        .into_iter()
        .find(|(_, node)| node.label() == Some("Cached action"))
        .expect("activation must build cached semantic content")
        .0;
    let count = renders.load(Ordering::Relaxed);
    for expected in 1..=3 {
        parent.update(cx, |_, cx| cx.notify());
        draw(cx);
        assert_eq!(
            renders.load(Ordering::Relaxed),
            count,
            "nested deferred content uses cache"
        );
        assert!(
            cx.a11y_tree()
                .unwrap()
                .nodes
                .iter()
                .any(|(id, _)| *id == control)
        );
        cx.simulate_a11y_action(accesskit::ActionRequest {
            action: accesskit::Action::Click,
            target_node: control,
            target_tree: accesskit::TreeId::ROOT,
            data: None,
        });
        cx.run_until_parked();
        assert_eq!(clicks.load(Ordering::Relaxed), expected);
    }
    cx.simulate_a11y_action(accesskit::ActionRequest {
        action: accesskit::Action::Focus,
        target_node: control,
        target_tree: accesskit::TreeId::ROOT,
        data: None,
    });
    draw(cx);
    assert_eq!(cx.a11y_tree().unwrap().focus, control);
    let count = renders.load(Ordering::Relaxed);
    parent.update(cx, |_, cx| cx.notify());
    draw(cx);
    assert_eq!(renders.load(Ordering::Relaxed), count);
    assert_eq!(cx.a11y_tree().unwrap().focus, control);
    let child = parent.read_with(cx, |parent, _| parent.child.clone().unwrap());
    child.update(cx, |child, cx| {
        child.label = "Updated action";
        cx.notify();
    });
    draw(cx);
    assert!(
        renders.load(Ordering::Relaxed) > count,
        "child notification invalidates cache"
    );
    let count = renders.load(Ordering::Relaxed);
    parent.update(cx, |_, cx| cx.notify());
    draw(cx);
    assert_eq!(renders.load(Ordering::Relaxed), count);
    assert!(
        cx.a11y_tree()
            .unwrap()
            .nodes
            .iter()
            .any(|(id, node)| *id == control && node.label() == Some("Updated action"))
    );
    cx.simulate_a11y_active(false);
    draw(cx);
    cx.simulate_a11y_active(true);
    draw(cx);
    assert!(
        cx.a11y_tree()
            .unwrap()
            .nodes
            .iter()
            .any(|(id, node)| *id == control && node.label() == Some("Updated action"))
    );
}
