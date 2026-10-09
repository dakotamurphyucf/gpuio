//! Mixed cached and uncached controls must retire callback owners promptly.
use super::*;

struct Mixed {
    cached: Entity<CachedDocument>,
    transient: Option<Arc<()>>,
}
impl Render for Mixed {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div()
            .w(px(440.))
            .h(px(200.))
            .child(
                self.cached
                    .clone()
                    .cached(gpui::StyleRefinement::default().size_full()),
            )
            .when_some(self.transient.clone(), |this, owner| {
                this.child(
                    div()
                        .id("uncached-control")
                        .role(accesskit::Role::Button)
                        .aria_label("Temporary")
                        .on_a11y_action(gpui::AccessibleAction::Click, move |_, _, _| {
                            let _ = &owner;
                        }),
                )
            })
    }
}
#[test]
fn unrelated_uncached_callback_owners_retire_in_the_removal_frame() {
    let mut app = TestAppContext::single();
    app.update(gpui_base::init);
    let renders = Arc::new(AtomicUsize::new(0));
    let clicks = Arc::new(AtomicUsize::new(0));
    let owner = Arc::new(());
    let weak = Arc::downgrade(&owner);
    let (parent, cx) = app.add_window_view(|_, cx| Mixed {
        cached: cx.new(|cx| CachedDocument {
            renders: renders.clone(),
            clicks: clicks.clone(),
            focus: cx.focus_handle(),
            label: "Cached",
            deferred: false,
        }),
        transient: Some(owner),
    });
    cx.simulate_a11y_active(true);
    cx.run_until_parked();
    cx.update(|window, cx| {
        window.refresh();
        window.draw(cx).clear(cx);
    });
    assert!(weak.upgrade().is_some());
    let count = renders.load(Ordering::Relaxed);
    let control = cx
        .a11y_tree()
        .unwrap()
        .nodes
        .iter()
        .find(|(_, node)| node.label() == Some("Cached"))
        .unwrap()
        .0;
    parent.update(cx, |parent, _| {
        parent.transient = None;
    });
    // Observe one removal frame; a second frame must not hide delayed release.
    cx.update(|window, cx| window.draw(cx).clear(cx));
    assert!(
        weak.upgrade().is_none(),
        "unrelated cache retained the removed callback owner"
    );
    assert_eq!(
        renders.load(Ordering::Relaxed),
        count,
        "child actually reused its cache"
    );
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
        "live cached callback still works"
    );
}
