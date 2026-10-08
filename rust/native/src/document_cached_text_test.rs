//! Native text-selection lifecycle during actual cached scene reuse.
use super::*;
use gpui_base::{
    TextSelection, TextSelectionHandle, TextSelectionLayer, TextSelectionRegistration,
    TextSelectionScopeId, text_selection_scope,
};

struct Text {
    text: Entity<TextViewState>,
    renders: Arc<AtomicUsize>,
    layer_inside: bool,
    deferred: bool,
    scope: TextSelectionScopeId,
}
impl Render for Text {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        self.renders.fetch_add(1, Ordering::Relaxed);
        let text = text_selection_scope(self.scope, TextView::new(&self.text)).into_any_element();
        let text = if self.deferred {
            gpui::deferred(text).into_any_element()
        } else {
            text
        };
        div()
            .size_full()
            .when(self.layer_inside, |this| this.child(TextSelectionLayer))
            .child(text)
    }
}
struct TextParent {
    child: Entity<Text>,
    mounted: bool,
    layer_inside: bool,
    plain: Option<TextSelectionHandle>,
    plain_first: bool,
}
fn plain(handle: &TextSelectionHandle) -> impl IntoElement {
    let handle = handle.clone();
    gpui::canvas(
        |bounds, window, _| window.insert_hitbox(bounds, gpui::HitboxBehavior::Normal),
        move |bounds, hitbox, window, cx| {
            handle.register_in_paint_order(
                TextSelectionRegistration::new(hitbox, bounds),
                window,
                cx,
            )
        },
    )
    .absolute()
    .top(px(0.))
    .h(px(20.))
    .w(px(100.))
}
impl Render for TextParent {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div()
            .w(px(440.))
            .h(px(200.))
            .when(!self.layer_inside, |this| this.child(TextSelectionLayer))
            .when(self.plain_first, |this| {
                this.when_some(self.plain.as_ref(), |this, handle| {
                    this.child(plain(handle))
                })
            })
            .when(self.mounted, |this| {
                this.child(
                    self.child
                        .clone()
                        .cached(gpui::StyleRefinement::default().size_full()),
                )
            })
            .when(!self.plain_first, |this| {
                this.when_some(self.plain.as_ref(), |this, handle| {
                    this.child(plain(handle))
                })
            })
    }
}
fn run_case(active: bool, layer_inside: bool, deferred: bool) {
    let mut app = TestAppContext::single();
    app.update(gpui_base::init);
    let renders = Arc::new(AtomicUsize::new(0));
    let scope = TextSelectionScopeId::new();
    let (parent, cx) = app.add_window_view(|_, cx| TextParent {
        child: cx.new(|cx| Text {
            text: cx.new(|cx| {
                let mut text = TextViewState::externally_prepared(cx);
                text.set_prepared(
                    PreparedText::parse("Cached 世界", MarkdownExtensions::default()).unwrap(),
                    None,
                    cx,
                );
                text
            }),
            renders: renders.clone(),
            layer_inside,
            deferred,
            scope,
        }),
        mounted: true,
        layer_inside,
        plain: None,
        plain_first: false,
    });
    let text = parent.read_with(cx, |p, cx| p.child.read(cx).text.clone());
    cx.simulate_a11y_active(active);
    cx.run_until_parked();
    cx.update(|window, cx| TextSelection::activate_scope(scope, window, cx));
    text.update(cx, |text, cx| text.select_all(cx));
    cx.update(|window, cx| {
        window.refresh();
        window.draw(cx).clear(cx);
    });
    assert_eq!(
        cx.update(TextSelection::selected_text),
        "Cached 世界",
        "fresh paint selected content"
    );
    let count = renders.load(Ordering::Relaxed);
    for _ in 0..4 {
        parent.update(cx, |_, cx| cx.notify());
        draw(cx);
        let selected = cx.update(TextSelection::selected_text);
        assert_eq!(
            renders.load(Ordering::Relaxed),
            count,
            "actual cache reuse, selected={selected:?}"
        );
        assert_eq!(selected, "Cached 世界");
    }
    if active {
        let range = cx.update(|window, cx| {
            let state = text.read(cx);
            let projection = state.rendered_text().unwrap();
            let range = accesskit::TextSelection {
                anchor: state
                    .rendered_accessible_text_position(window, &projection.position(0).unwrap())
                    .unwrap(),
                focus: state
                    .rendered_accessible_text_position(window, &projection.position(6).unwrap())
                    .unwrap(),
            };
            assert!(
                state
                    .prepare_accessible_selection(&range, window, cx)
                    .is_some()
            );
            range
        });
        let document = cx
            .a11y_tree()
            .unwrap()
            .nodes
            .iter()
            .find(|(_, node)| node.role() == accesskit::Role::Document)
            .unwrap()
            .0;
        cx.simulate_a11y_action(accesskit::ActionRequest {
            action: accesskit::Action::SetTextSelection,
            target_node: document,
            target_tree: accesskit::TreeId::ROOT,
            data: Some(accesskit::ActionData::SetTextSelection(range)),
        });
        draw(cx);
        assert_eq!(cx.update(TextSelection::selected_text), "Cached");
        let after_action = renders.load(Ordering::Relaxed);
        parent.update(cx, |_, cx| cx.notify());
        draw(cx);
        assert_eq!(renders.load(Ordering::Relaxed), after_action);
        assert_eq!(cx.update(TextSelection::selected_text), "Cached");
    }
    parent.update(cx, |parent, cx| {
        parent.mounted = false;
        cx.notify();
    });
    draw(cx);
    assert_eq!(
        cx.update(TextSelection::selected_text),
        "",
        "detached participant retired"
    );
    parent.update(cx, |parent, cx| {
        parent.mounted = true;
        cx.notify();
    });
    draw(cx);
    assert_eq!(
        cx.update(TextSelection::selected_text),
        "",
        "retained text does not resurrect selection"
    );
}
#[test]
fn cached_text_selection_survives_reuse_and_retires_on_unmount() {
    for active in [false, true] {
        for layer_inside in [false, true] {
            for deferred in [false, true] {
                run_case(active, layer_inside, deferred);
            }
        }
    }
}
#[test]
fn cached_text_and_uncached_plain_text_share_current_copy_order() {
    let mut app = TestAppContext::single();
    app.update(gpui_base::init);
    let renders = Arc::new(AtomicUsize::new(0));
    let (parent, cx) = app.add_window_view(|_, cx| TextParent {
        child: cx.new(|cx| Text {
            text: cx.new(|cx| {
                let mut text = TextViewState::externally_prepared(cx);
                text.set_prepared(
                    PreparedText::parse("Cached", MarkdownExtensions::default()).unwrap(),
                    None,
                    cx,
                );
                text
            }),
            renders: renders.clone(),
            layer_inside: false,
            deferred: false,
            scope: TextSelectionScopeId::default(),
        }),
        mounted: true,
        layer_inside: false,
        plain: Some(TextSelectionHandle::new("Plain", cx)),
        plain_first: false,
    });
    let (text, plain) = parent.read_with(cx, |p, cx| {
        (p.child.read(cx).text.clone(), p.plain.clone().unwrap())
    });
    draw(cx);
    cx.update(|_, cx| plain.set_local_selection(true, cx));
    text.update(cx, |text, cx| text.select_all(cx));
    cx.update(|window, cx| {
        window.refresh();
        window.draw(cx).clear(cx);
    });
    assert_eq!(cx.update(TextSelection::selected_text), "Cached\nPlain");
    let count = renders.load(Ordering::Relaxed);
    for _ in 0..4 {
        parent.update(cx, |_, cx| cx.notify());
        draw(cx);
        assert_eq!(renders.load(Ordering::Relaxed), count);
        assert_eq!(cx.update(TextSelection::selected_text), "Cached\nPlain");
    }
    // The plain canvas is absolute: its order changes without moving the
    // cached text or invalidating that entity's layout key.
    parent.update(cx, |parent, cx| {
        parent.plain_first = true;
        cx.notify();
    });
    draw(cx);
    assert_eq!(renders.load(Ordering::Relaxed), count);
    assert_eq!(cx.update(TextSelection::selected_text), "Plain\nCached");
}
