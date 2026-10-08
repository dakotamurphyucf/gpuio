//! Preserve raw prepaint registration and cache explicit logical ordering.
use super::*;
use gpui_base::ElementExt as _;
use gpui_base::{
    TextSelection, TextSelectionHandle, TextSelectionLayer, TextSelectionRegistration,
};
struct Ordered {
    participant: TextSelectionHandle,
    renders: Arc<AtomicUsize>,
}
impl Render for Ordered {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        self.renders.fetch_add(1, Ordering::Relaxed);
        let participant = self.participant.clone();
        gpui::canvas(
            |bounds, window, _| window.insert_hitbox(bounds, gpui::HitboxBehavior::Normal),
            move |bounds, hitbox, window, cx| {
                participant.register_in_logical_order(
                    TextSelectionRegistration::new(hitbox, bounds).with_document_order(1),
                    window,
                    cx,
                )
            },
        )
        .size_full()
    }
}
struct Registrations {
    child: Entity<Ordered>,
    legacy: TextSelectionHandle,
    mounted: bool,
}
impl Render for Registrations {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        let legacy = self.legacy.clone();
        div()
            .w(px(440.))
            .h(px(200.))
            .child(TextSelectionLayer)
            .child(
                div()
                    .absolute()
                    .size_full()
                    .on_prepaint(move |bounds, window, cx| {
                        let hitbox = window.insert_hitbox(bounds, gpui::HitboxBehavior::Normal);
                        legacy.register(
                            TextSelectionRegistration::new(hitbox, bounds).with_document_order(20),
                            window,
                            cx,
                        );
                    }),
            )
            .when(self.mounted, |this| {
                this.child(
                    self.child
                        .clone()
                        .cached(gpui::StyleRefinement::default().size_full()),
                )
            })
    }
}
#[test]
fn raw_prepaint_and_cached_logical_order_remain_compatible() {
    let mut app = TestAppContext::single();
    app.update(gpui_base::init);
    let renders = Arc::new(AtomicUsize::new(0));
    let (root, cx) = app.add_window_view(|_, cx| Registrations {
        child: cx.new(|cx| Ordered {
            participant: TextSelectionHandle::new("Cached", cx),
            renders: renders.clone(),
        }),
        legacy: TextSelectionHandle::new("Legacy", cx),
        mounted: true,
    });
    draw(cx);
    let (legacy, cached) = root.read_with(cx, |root, cx| {
        (root.legacy.clone(), root.child.read(cx).participant.clone())
    });
    cx.update(|_, cx| {
        legacy.set_local_selection(true, cx);
        cached.set_local_selection(true, cx);
    });
    draw(cx);
    assert_eq!(
        cx.update(TextSelection::selected_text),
        "Cached\nLegacy",
        "explicit order differs from layout/paint order"
    );
    let count = renders.load(Ordering::Relaxed);
    for _ in 0..3 {
        root.update(cx, |_, cx| cx.notify());
        draw(cx);
        assert_eq!(renders.load(Ordering::Relaxed), count);
        assert_eq!(cx.update(TextSelection::selected_text), "Cached\nLegacy");
    }
    root.update(cx, |root, cx| {
        root.mounted = false;
        cx.notify();
    });
    draw(cx);
    assert_eq!(cx.update(TextSelection::selected_text), "Legacy");
    root.update(cx, |root, cx| {
        root.mounted = true;
        cx.notify();
    });
    draw(cx);
    assert_eq!(cx.update(TextSelection::selected_text), "Legacy");
}
