//! Input ordered between source publication and prepared-content installation.
use super::*;
use gpuio_protocol::document::Mode;

fn publish_reset(session: &Rc<RefCell<Session>>, source: ResourceId, text: &str) -> i64 {
    let snapshot = session.borrow().document(source).unwrap().snapshot();
    let generation = snapshot.generation + 1;
    publish(
        &mut session.borrow_mut(),
        source,
        snapshot.revision,
        generation,
        0,
        text,
    );
    generation
}

fn reset_source(
    cx: &mut gpui::AsyncApp,
    window: gpui::WindowHandle<View>,
    session: &Rc<RefCell<Session>>,
    source: ResourceId,
    text: &str,
) -> i64 {
    let generation = publish_reset(session, source, text);
    window
        .update(cx, |view, _, cx| view.document_changed(source, cx))
        .unwrap();
    generation
}

fn enter(window: &mut Window, cx: &mut App) {
    let keystroke = gpui::Keystroke::parse("enter").unwrap();
    window.dispatch_event(
        gpui::PlatformInput::KeyDown(gpui::KeyDownEvent {
            keystroke: keystroke.clone(),
            is_held: false,
            prefer_character_input: false,
        }),
        cx,
    );
    window.dispatch_event(
        gpui::PlatformInput::KeyUp(gpui::KeyUpEvent { keystroke }),
        cx,
    );
}

fn initial_collapse(cx: &mut gpui::AsyncApp, window: gpui::WindowHandle<View>, collapsed: bool) {
    window
        .update(cx, |view, window, cx| {
            let (base, mut config) = {
                let session = view.session.borrow();
                let tree = session.tree(id()).unwrap();
                (
                    tree.revision(),
                    (**tree.get(node()).unwrap().document.as_ref().unwrap()).clone(),
                )
            };
            config.initially_collapsed = collapsed;
            let applied = view
                .session
                .borrow_mut()
                .apply(&Transaction {
                    window: id(),
                    base,
                    revision: base + 1,
                    operations: vec![Op::SetDocument(node(), config)],
                })
                .unwrap();
            view.update_editors(&applied.dirty, window, cx);
            cx.notify();
        })
        .unwrap();
}

pub(super) async fn exercise(
    cx: &mut gpui::AsyncApp,
    window: gpui::WindowHandle<View>,
    session: &Rc<RefCell<Session>>,
) {
    let Response::Created(source) = session.borrow_mut().document_request(Request::Create) else {
        panic!("create reset fixture");
    };
    for (mode, text) in [
        (Mode::Code("ml".into()), "let reset = 1\n"),
        (Mode::Markdown, "# Reset\n\nA **prepared** document.\n"),
        (
            Mode::Diff,
            "--- a/a.ml\n+++ b/a.ml\n@@ -1 +1 @@\n-let a = 1\n+let a = 2\n",
        ),
    ] {
        reset_source(cx, window, session, source, text);
        configure(cx, window, source, mode, "");
        let presentation = settle(cx, window).await;
        for initially_collapsed in [false, true] {
            for refresh_before_input in [false, true] {
                initial_collapse(cx, window, initially_collapsed);
                reset_source(cx, window, session, source, text);
                let _ = settle(cx, window).await;
                assert_eq!(
                    presentation.read_with(cx, |p, _| p.collapsed),
                    initially_collapsed
                );

                presentation.update(cx, |p, _| p.defer_prepared_install = true);
                let generation = if refresh_before_input {
                    let generation = reset_source(cx, window, session, source, text);
                    draw(cx, window);
                    presentation.read_with(cx, |p, _| {
                        assert!(!p.ready);
                        assert_eq!(p.snapshot.generation, generation);
                        assert_ne!(p.installed.as_ref().unwrap().generation, generation);
                    });
                    cx.update_window(window.into(), |_, window, cx| {
                        window.focus(
                            &presentation.read(cx).buttons["document-collapse"].clone(),
                            cx,
                        );
                        enter(window, cx);
                    })
                    .unwrap();
                    generation
                } else {
                    // Publication, assertion and key delivery share one native
                    // update so no event-loop frame can refresh the presenter
                    // before the action. Preparation remains held independently.
                    cx.update_window(window.into(), |_, window, cx| {
                        let generation = publish_reset(session, source, text);
                        let state = presentation.read(cx);
                        assert_ne!(state.snapshot.generation, generation);
                        assert_eq!(state.lease.snapshot().generation, generation);
                        if state.markdown.is_some() {
                            let installed = state.installed.as_ref().unwrap();
                            assert!(!state.allows_link_focus(
                                Some((installed.generation, installed.revision)),
                                &presentation,
                                cx,
                            ));
                        }
                        window.focus(&state.buttons["document-collapse"].clone(), cx);
                        enter(window, cx);
                        generation
                    })
                    .unwrap()
                };
                window
                    .update(cx, |view, _, cx| view.document_changed(source, cx))
                    .unwrap();
                assert_eq!(
                    presentation.read_with(cx, |p, _| p.collapsed),
                    !initially_collapsed
                );
                presentation.read_with(cx, |p, cx| {
                    if p.markdown.is_some() {
                        let installed = p.installed.as_ref().unwrap();
                        assert!(!p.allows_link_focus(
                            Some((installed.generation, installed.revision)),
                            &presentation,
                            cx,
                        ));
                    }
                });
                presentation.update(cx, |p, cx| {
                    p.defer_prepared_install = false;
                    cx.notify();
                });
                let _ = settle(cx, window).await;
                presentation.read_with(cx, |p, _| {
                    assert_eq!(p.installed.as_ref().unwrap().generation, generation);
                    assert_eq!(
                        p.collapsed, !initially_collapsed,
                        "prepared reset overwrote a newer collapse/expand"
                    );
                });

                let snapshot = session.borrow().document(source).unwrap().snapshot();
                publish(
                    &mut session.borrow_mut(),
                    source,
                    snapshot.revision,
                    snapshot.generation,
                    snapshot.text.len(),
                    "\n",
                );
                window
                    .update(cx, |view, _, cx| view.document_changed(source, cx))
                    .unwrap();
                let _ = settle(cx, window).await;
                assert_eq!(
                    presentation.read_with(cx, |p, _| p.collapsed),
                    !initially_collapsed,
                    "append retains the explicit interaction"
                );

                reset_source(cx, window, session, source, text);
                let _ = settle(cx, window).await;
                assert_eq!(
                    presentation.read_with(cx, |p, _| p.collapsed),
                    initially_collapsed,
                    "a later reset without a newer interaction restores the configured initial state"
                );
            }
        }
    }
    assert_eq!(
        session
            .borrow_mut()
            .document_request(Request::Release(source)),
        Response::Ack
    );
    eprintln!(
        "GPUIO_DOCUMENT_RESET_INTERACTION_OK: code/Markdown/diff, collapsed/expanded defaults, native Enter before/after refresh during held installation, append retention and later reset defaults"
    );
}
