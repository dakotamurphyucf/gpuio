//! The document wrapper follows its native presenter focus and state styling.
use super::*;

fn style(state: i64, hidden: Field) -> Vec<Style> {
    vec![
        Style::Width(Length::Px(320.)),
        Style::Padding(20.),
        Style::State(state, vec![hidden]),
    ]
}
fn pointer(cx: &mut AsyncApp, handle: WindowHandle<View>, x: f32, y: f32) {
    crate::host::native_test::move_mouse(cx, handle, gpui::point(px(x), px(y)), false);
    draw(cx, handle);
}
async fn count(cx: &mut AsyncApp, handle: WindowHandle<View>, transport: &Transport, total: i64) {
    let expected = highlight::State::Ready(vec![highlight::Count {
        total,
        stored: total,
    }]);
    let mut sample = None;
    for _ in 0..500 {
        draw(cx, handle);
        pause(cx).await;
        for event in transport.mailbox.lock().unwrap().drain(128) {
            if let Event::HighlightObserved(_, n, _, _, observed) = event
                && n == node(0)
                && observed.state == expected
            {
                sample = Some(observed);
            }
        }
        let current = handle
            .update(cx, |view, _, _| {
                view.highlights[&node(0)].borrow().observation()
            })
            .unwrap();
        if sample.as_ref().is_some_and(|sample| *sample == current) {
            return;
        }
    }
    panic!(
        "document wrapper did not settle at {total}: {:?}",
        handle
            .update(cx, |view, _, _| view.highlights[&node(0)]
                .borrow()
                .observation())
            .unwrap()
    );
}
async fn steady(cx: &mut AsyncApp, handle: WindowHandle<View>, transport: &Transport) {
    for _ in 0..4 {
        draw(cx, handle);
        pause(cx).await;
    }
    assert!(
        !transport
            .mailbox
            .lock()
            .unwrap()
            .drain(128)
            .iter()
            .any(|event| matches!(event, Event::HighlightObserved(..))),
        "unchanged document style is silent"
    );
}
fn focus_root(cx: &mut AsyncApp, handle: WindowHandle<View>) {
    handle
        .update(cx, |view, window, cx| {
            window.focus(view.root_focus.as_ref().unwrap(), cx)
        })
        .unwrap();
    draw(cx, handle);
}

pub(super) async fn exercise(
    cx: &mut AsyncApp,
    handle: WindowHandle<View>,
    session: &Rc<RefCell<Session>>,
    transport: &Transport,
    source: ResourceId,
    p: &Entity<Presentation>,
) {
    for (index, mode) in [Mode::Code("txt".into()), Mode::Markdown]
        .into_iter()
        .enumerate()
    {
        let base = p.read_with(cx, |p, _| p.installed.as_ref().unwrap().revision);
        pointer(cx, handle, 430., 330.);
        focus_root(cx, handle);
        publish(
            &mut session.borrow_mut(),
            source,
            base,
            "aaa first\n\naaa second\n",
        );
        handle
            .update(cx, |view, _, cx| view.document_changed(source, cx))
            .unwrap();
        apply(
            cx,
            handle,
            vec![
                Op::Bind(node(0), Some(handler(700 + index as i64))),
                Op::SetHighlightScope(node(0), config(0.)),
                Op::SetDocument(node(1), document(source, mode)),
                Op::SetStyle(node(1), style(2, Field::Visibility(1))),
            ],
        );
        installed_revision(cx, handle, transport, p, base + 1).await;
        count(cx, handle, transport, 2).await;
        assert!(red_pixels(cx, handle) > 20);
        let snapshot = p.read_with(cx, |p, _| p.installed.clone().unwrap());
        // Exposed wrapper padding avoids child editor/TextView hitbox occlusion.
        pointer(cx, handle, 10., 10.);
        count(cx, handle, transport, 0).await;
        assert_eq!(red_pixels(cx, handle), 0);
        steady(cx, handle, transport).await;
        pointer(cx, handle, 430., 330.);
        count(cx, handle, transport, 2).await;
        assert!(red_pixels(cx, handle) > 20);

        apply(
            cx,
            handle,
            vec![Op::SetStyle(node(1), style(3, Field::Visibility(1)))],
        );
        draw(cx, handle);
        pause(cx).await;
        pointer(cx, handle, 10., 10.);
        crate::host::native_test::mouse(cx, handle, gpui::point(px(10.), px(10.)), true);
        count(cx, handle, transport, 0).await;
        assert_eq!(red_pixels(cx, handle), 0);
        crate::host::native_test::mouse(cx, handle, gpui::point(px(430.), px(330.)), false);
        count(cx, handle, transport, 2).await;

        for hidden in [Field::Visibility(1), Field::Display(3)] {
            apply(cx, handle, vec![Op::SetStyle(node(1), style(1, hidden))]);
            // The actual editor/Markdown focus owns state; the wrapper adds no stop.
            handle
                .update(cx, |_, window, cx| {
                    window.focus(&p.read(cx).primary_focus(cx), cx)
                })
                .unwrap();
            count(cx, handle, transport, 0).await;
            assert_eq!(red_pixels(cx, handle), 0);
            focus_root(cx, handle);
            count(cx, handle, transport, 2).await;
            assert!(red_pixels(cx, handle) > 20);
        }
        // Toolbar focus belongs to the same document component.
        handle
            .update(cx, |_, window, cx| {
                window.focus(&p.read(cx).buttons["document-copy"].clone(), cx)
            })
            .unwrap();
        count(cx, handle, transport, 0).await;
        focus_root(cx, handle);
        count(cx, handle, transport, 2).await;
        assert!(
            p.read_with(cx, |p, _| Arc::ptr_eq(
                &snapshot,
                p.installed.as_ref().unwrap()
            )),
            "native style changes retain the installed document"
        );
        assert_eq!(
            presentation(cx, handle),
            *p,
            "document presenter identity survives hiding"
        );
        steady(cx, handle, transport).await;
    }
    apply(cx, handle, vec![Op::SetStyle(node(1), vec![])]);
    draw(cx, handle);
    eprintln!(
        "GPUIO_NATIVE_HIGHLIGHT_DOCUMENT_STYLE_OK: source/Markdown wrapper hover/press/release, native and toolbar focus, Display/Visibility, stable observations and retained document/presenter"
    );
}
