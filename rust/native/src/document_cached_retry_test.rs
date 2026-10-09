//! Rolled-back cached prepaint must not leak nodes or corrupt a later replay.
use super::*;
use gpui::{
    AnyElement, Bounds, Element, ElementId, GlobalElementId, InspectorElementId, LayoutId, Pixels,
};
struct Retried {
    child: Entity<CachedDocument>,
    retry: bool,
}
impl IntoElement for Retried {
    type Element = Self;
    fn into_element(self) -> Self {
        self
    }
}
impl Element for Retried {
    type RequestLayoutState = Option<AnyElement>;
    type PrepaintState = AnyElement;
    fn id(&self) -> Option<ElementId> {
        Some("retry-host".into())
    }
    fn source_location(&self) -> Option<&'static std::panic::Location<'static>> {
        None
    }
    fn request_layout(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        window: &mut Window,
        cx: &mut App,
    ) -> (LayoutId, Self::RequestLayoutState) {
        let mut element = self
            .child
            .clone()
            .cached(gpui::StyleRefinement::default().size_full())
            .into_any_element();
        let layout = element.request_layout(window, cx);
        (layout, Some(element))
    }
    fn prepaint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>,
        state: &mut Self::RequestLayoutState,
        window: &mut Window,
        cx: &mut App,
    ) -> AnyElement {
        let mut element = state.take().unwrap();
        if self.retry {
            let failed: Result<(), ()> = window.transact(|window| {
                // Shift recorded node indices during an attempt that will be
                // discarded, as a list can do while trying a different range.
                let mut ghost = div()
                    .id("discarded-node")
                    .role(accesskit::Role::Button)
                    .aria_label("Discarded")
                    .into_any_element();
                ghost.layout_as_root(bounds.size.into(), window, cx);
                ghost.prepaint_at(bounds.origin, window, cx);
                element.prepaint(window, cx);
                Err(())
            });
            assert!(failed.is_err());
            element = self
                .child
                .clone()
                .cached(gpui::StyleRefinement::default().size_full())
                .into_any_element();
            element.layout_as_root(bounds.size.into(), window, cx);
            element.prepaint_at(bounds.origin, window, cx);
        } else {
            element.prepaint(window, cx);
        }
        element
    }
    fn paint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        _: Bounds<Pixels>,
        _: &mut Self::RequestLayoutState,
        state: &mut AnyElement,
        window: &mut Window,
        cx: &mut App,
    ) {
        state.paint(window, cx);
    }
}
struct RetryParent {
    child: Entity<CachedDocument>,
    retry: bool,
}
impl Render for RetryParent {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div().w(px(440.)).h(px(200.)).child(Retried {
            child: self.child.clone(),
            retry: self.retry,
        })
    }
}
fn run_retry(initial_retry: bool, deferred: bool) {
    let mut app = TestAppContext::single();
    app.update(gpui_base::init);
    let renders = Arc::new(AtomicUsize::new(0));
    let clicks = Arc::new(AtomicUsize::new(0));
    let (root, cx) = app.add_window_view(|_, cx| RetryParent {
        child: cx.new(|cx| CachedDocument {
            renders: renders.clone(),
            clicks: clicks.clone(),
            focus: cx.focus_handle(),
            label: "Cached",
            deferred,
        }),
        retry: initial_retry,
    });
    cx.simulate_a11y_active(true);
    draw(cx);
    cx.update(|window, cx| {
        window.refresh();
        window.draw(cx).clear(cx);
    });
    let control = cx
        .a11y_tree()
        .unwrap()
        .nodes
        .iter()
        .find(|(_, node)| node.label() == Some("Cached"))
        .unwrap()
        .0;
    let initial_ids = cx
        .a11y_tree()
        .unwrap()
        .nodes
        .iter()
        .map(|(id, _)| *id)
        .collect::<std::collections::BTreeSet<_>>();
    let count = renders.load(Ordering::Relaxed);
    for _ in 0..3 {
        root.update(cx, |root, cx| {
            root.retry = true;
            cx.notify();
        });
        draw(cx);
        assert_eq!(
            renders.load(Ordering::Relaxed),
            count,
            "both attempts actually reuse the cached child"
        );
        let tree = cx.a11y_tree().unwrap();
        assert!(
            tree.nodes
                .iter()
                .all(|(_, node)| node.label() != Some("Discarded"))
        );
        assert!(
            tree.nodes
                .iter()
                .any(|(id, node)| *id == control && node.label() == Some("Cached"))
        );
        assert_eq!(
            tree.nodes
                .iter()
                .map(|(id, _)| *id)
                .collect::<std::collections::BTreeSet<_>>(),
            initial_ids
        );
    }
    cx.simulate_a11y_action(accesskit::ActionRequest {
        action: accesskit::Action::Click,
        target_node: control,
        target_tree: accesskit::TreeId::ROOT,
        data: None,
    });
    cx.run_until_parked();
    assert_eq!(clicks.load(Ordering::Relaxed), 1);
}

#[test]
fn cached_prepaint_retry_discards_attempt_and_keeps_live_action() {
    for initial_retry in [false, true] {
        for deferred in [false, true] {
            run_retry(initial_retry, deferred);
        }
    }
}
