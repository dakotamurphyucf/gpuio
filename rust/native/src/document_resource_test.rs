//! Mounted resource ownership on TestPlatform, not process RSS or desktop timing.
use super::*;

fn text(blocks: usize) -> String {
    (0..blocks)
        .map(|i| format!("Paragraph {i} — 世界.\n\n```ml\nlet item = {i}\n```\n\n"))
        .collect()
}

#[test]
fn full_document_traversal_and_repeated_window_close_release_profile_resources() {
    // Keep one App/service alive throughout. More than 32 cycles crosses the
    // service's concurrent-window limit and catches retired-window retention.
    let mut app = TestAppContext::single();
    let mut peak_charge = 0;
    let mut visited = 0;
    let mut short_charge = None;
    for cycle in 0..36 {
        let (f, cx) = mount(&mut app, Mode::Markdown);
        install(&f, cx, 1, 0);
        let mut config = f.presentation.read_with(cx, |p, _| (*p.config).clone());
        config.layout = Layout::Viewport(180.);
        apply(&f.view, cx, vec![Op::SetDocument(f.node, config)]);
        // Grow the first document, revisit every block in both directions, then
        // replace it with a short document. Other cycles exercise window churn.
        let mut snapshots = Vec::new();
        for (revision, blocks) in [(2, if cycle == 0 { 96 } else { 2 }), (3, 1)] {
            publish(&f, cx, revision, revision, &text(blocks));
            ready(&f.presentation, cx);
            let markdown = f
                .presentation
                .read_with(cx, |p, _| p.markdown.clone().unwrap());
            let list = markdown.read_with(cx, |m, _| m.list_state().clone());
            assert_eq!(list.item_count(), blocks * 2);
            for snapshot in &snapshots {
                let snapshot: &std::sync::Weak<crate::document_store::Snapshot> = snapshot;
                assert!(
                    snapshot.upgrade().is_none(),
                    "replaced source snapshot was released"
                );
            }
            snapshots.push(
                f.presentation
                    .read_with(cx, |p, _| Arc::downgrade(&p.snapshot)),
            );
            let charge = cx.read(crate::document_host::resources).reserved_bytes;
            assert!(charge > 0);
            peak_charge = peak_charge.max(charge);
            if blocks == 1 {
                assert_eq!(
                    *short_charge.get_or_insert(charge),
                    charge,
                    "the same short document must not grow across window cycles"
                );
            }
            for index in (0..list.item_count()).chain((0..list.item_count()).rev()) {
                list.scroll_to_reveal_item(index);
                draw(cx);
                draw(cx);
                assert!(
                    list.bounds_for_item(index).is_some(),
                    "block {index} was painted"
                );
                assert_eq!(
                    cx.read(crate::document_host::resources).reserved_bytes,
                    charge,
                    "traversal must not accumulate parser/profile reservations"
                );
                visited += 1;
            }
        }
        let stale = sink(&f, cx);
        let weak_presentation = f.presentation.downgrade();
        let weak_profile = f.presentation.read_with(cx, |p, _| {
            Arc::downgrade(p.profile_install.as_ref().unwrap())
        });
        let session = f.view.read_with(cx, |v, _| v.session.clone());
        let source = f.source;
        cx.update(|window, _| window.remove_window());
        drop(f);
        // GPUI releases entities at an App update boundary. Dropping test-owned
        // handles outside that boundary must be followed by the normal flush.
        cx.cx.update(|_| {});
        cx.run_until_parked();
        assert_eq!(stale.check(), Err(gpuio_extension_sdk::Error::Closed));
        // Intentionally retained callbacks keep their reservation until dropped.
        assert!(cx.read(crate::document_host::resources).reserved_bytes > 0);
        drop(stale);
        cx.run_until_parked();
        assert!(weak_presentation.upgrade().is_none());
        assert!(weak_profile.upgrade().is_none());
        assert_eq!(
            session
                .borrow_mut()
                .document_request(Request::Release(source)),
            Response::Ack
        );
        drop(session);
        assert!(
            snapshots
                .iter()
                .all(|snapshot| snapshot.upgrade().is_none()),
            "released source snapshots must not remain in caches or callbacks"
        );
        assert_eq!(
            cx.read(crate::document_host::resources),
            crate::document_host::Resources {
                windows: 0,
                workers: 0,
                completions: 0,
                reserved_bytes: 0,
            },
            "cycle {cycle} returns the shared service to zero live resources"
        );
        assert!(app.windows().is_empty());
    }
    eprintln!(
        "GPUIO_DOCUMENT_RESOURCES_OK windows=36 visited_blocks={visited} peak_charge={peak_charge} final_charge=0; TestPlatform ownership only, not RSS"
    );
}
