//! Real native selection input remains separate from document controls.
use super::*;
use crate::host::{
    editor_test::key,
    native_test::{mouse, move_mouse},
};

fn style(value: Option<bool>) -> Vec<Style> {
    vec![Style::Fields(
        value.into_iter().map(Field::UserSelect).collect(),
    )]
}
fn parent(value: Option<bool>) -> Vec<Style> {
    let mut fields = vec![
        Field::Width(Length::Px(440.)),
        Field::Height(Length::Px(340.)),
    ];
    fields.extend(value.map(Field::UserSelect));
    vec![Style::Fields(fields)]
}
fn selected(cx: &mut AsyncApp, p: &Entity<Presentation>) -> String {
    p.read_with(cx, |p, cx| {
        if !p.source_mode {
            p.markdown.as_ref().unwrap().read(cx).selected_text()
        } else {
            p.editor.read(cx).selected_value().to_string()
        }
    })
}
fn focus(cx: &mut AsyncApp, handle: WindowHandle<View>, p: &Entity<Presentation>) {
    handle
        .update(cx, |_, window, cx| {
            window.activate_window();
            window.focus(&p.read(cx).primary_focus(cx), cx);
        })
        .unwrap();
    draw(cx, handle);
}
fn clipboard(cx: &mut AsyncApp) -> Option<String> {
    cx.update(|cx| cx.read_from_clipboard().and_then(|item| item.text()))
}
fn source_pointer(cx: &mut AsyncApp, handle: WindowHandle<View>, p: &Entity<Presentation>) {
    let editor = p.read_with(cx, |p, _| p.editor.clone());
    editor.update(cx, |e, cx| {
        assert!(e.bridge_select(0, 0, cx));
    });
    draw(cx, handle);
    let bounds = editor.read_with(cx, |e, _| e.range_to_bounds(&(0..3)).unwrap());
    let start = gpui::point(bounds.left() + px(1.), bounds.center().y);
    let end = gpui::point(bounds.right(), bounds.center().y);
    for click_count in [1, 2, 3] {
        move_mouse(cx, handle, start, false);
        handle
            .update(cx, |_, window, cx| {
                window.dispatch_event(
                    gpui::PlatformInput::MouseDown(gpui::MouseDownEvent {
                        position: start,
                        button: gpui::MouseButton::Left,
                        modifiers: Default::default(),
                        click_count,
                        first_mouse: false,
                    }),
                    cx,
                );
            })
            .unwrap();
        move_mouse(cx, handle, end, true);
        mouse(cx, handle, end, false);
        assert!(
            selected(cx, p).is_empty(),
            "disabled click/drag count={click_count}"
        );
    }
    // Explicit bridge selection still serves search/navigation, but cannot enable Copy.
    editor.update(cx, |e, cx| {
        assert!(e.bridge_select(0, 3, cx));
    });
    assert_eq!(selected(cx, p).len(), 3);
    key(cx, handle, "secondary-c");
    assert_eq!(clipboard(cx).as_deref(), Some("selection sentinel"));
    editor.update(cx, |e, cx| {
        e.bridge_select(0, 0, cx);
    });
}

fn markdown_pointer(cx: &mut AsyncApp, handle: WindowHandle<View>, p: &Entity<Presentation>) {
    let bounds = p.read_with(cx, |p, cx| p.markdown.as_ref().unwrap().read(cx).bounds());
    let start = gpui::point(bounds.left() + px(2.), bounds.top() + px(10.));
    let end = start + gpui::point(px(60.), px(0.));
    for click_count in [1, 2, 3] {
        move_mouse(cx, handle, start, false);
        handle
            .update(cx, |_, window, cx| {
                window.dispatch_event(
                    gpui::PlatformInput::MouseDown(gpui::MouseDownEvent {
                        position: start,
                        button: gpui::MouseButton::Left,
                        modifiers: Default::default(),
                        click_count,
                        first_mouse: false,
                    }),
                    cx,
                );
            })
            .unwrap();
        move_mouse(cx, handle, end, true);
        draw(cx, handle);
        mouse(cx, handle, end, false);
        draw(cx, handle);
        assert!(
            selected(cx, p).is_empty(),
            "disabled Markdown click/drag count={click_count}"
        );
        key(cx, handle, "secondary-c");
        assert_eq!(clipboard(cx).as_deref(), Some("selection sentinel"));
    }
    focus(cx, handle, p);
}

async fn cancel_drag(cx: &mut AsyncApp, handle: WindowHandle<View>, p: &Entity<Presentation>) {
    let bounds = p.read_with(cx, |p, cx| {
        if !p.source_mode {
            p.markdown.as_ref().unwrap().read(cx).bounds()
        } else {
            p.editor.read(cx).range_to_bounds(&(0..3)).unwrap()
        }
    });
    let y = bounds.top() + px(10.);
    let start = gpui::point(bounds.left() + px(2.), y);
    let end = gpui::point(bounds.left() + px(22.), y);
    move_mouse(cx, handle, start, false);
    mouse(cx, handle, start, true);
    move_mouse(cx, handle, end, true);
    draw(cx, handle);
    pause(cx).await;
    draw(cx, handle);
    assert!(
        !selected(cx, p).is_empty(),
        "drag fixture must select actual text; bounds={bounds:?}"
    );
    apply(cx, handle, vec![Op::SetStyle(node(1), style(Some(false)))]);
    draw(cx, handle);
    move_mouse(cx, handle, end + gpui::point(px(80.), px(0.)), true);
    draw(cx, handle);
    mouse(cx, handle, end, false);
    assert!(
        selected(cx, p).is_empty(),
        "restyle cancels an in-progress drag"
    );
    pause(cx).await;
    draw(cx, handle);
    apply(cx, handle, vec![Op::SetStyle(node(1), style(None))]);
    draw(cx, handle);
    pause(cx).await;
    draw(cx, handle);
    assert!(
        selected(cx, p).is_empty(),
        "retired geometry selection must not resurrect"
    );
}

async fn modal_selection(cx: &mut AsyncApp, handle: WindowHandle<View>, p: &Entity<Presentation>) {
    let markdown = p.read_with(cx, |p, _| p.markdown.clone().unwrap());
    markdown.update(cx, |m, cx| m.select_all(cx));
    draw(cx, handle);
    assert!(
        !handle
            .update(cx, |_, w, cx| gpui_base::TextSelection::selected_text(
                w, cx
            ))
            .unwrap()
            .is_empty()
    );
    apply(
        cx,
        handle,
        vec![
            Op::Create(node(2), Kind::FocusScope, "".into(), None),
            Op::SetFocusScope(
                node(2),
                FocusScopeConfig {
                    trap: true,
                    auto_focus: true,
                    restore_focus: true,
                },
            ),
            Op::Create(node(3), Kind::Button, "Modal control".into(), None),
            Op::Splice(node(2), 0, 0, vec![node(3)]),
            Op::Splice(node(0), 1, 0, vec![node(2)]),
        ],
    );
    draw(cx, handle);
    pause(cx).await;
    draw(cx, handle);
    assert!(
        selected(cx, p).is_empty(),
        "opening a trap clears background selection"
    );
    // A stale/programmatic background selection cannot enter the active modal's Copy.
    markdown.update(cx, |m, cx| m.select_all(cx));
    draw(cx, handle);
    handle
        .update(cx, |_, w, cx| {
            assert!(gpui_base::TextSelection::selected_text(w, cx).is_empty());
            assert!(!gpui_base::TextSelection::has_selection(w, cx));
        })
        .unwrap();
    apply(
        cx,
        handle,
        vec![
            Op::Splice(node(0), 1, 1, vec![]),
            Op::Splice(node(2), 0, 1, vec![]),
            Op::Remove(node(3)),
            Op::Remove(node(2)),
        ],
    );
    draw(cx, handle);
    pause(cx).await;
    draw(cx, handle);
    assert!(
        selected(cx, p).is_empty(),
        "closing a trap does not restore stale selection"
    );
}

pub(super) async fn exercise(
    cx: &mut AsyncApp,
    handle: WindowHandle<View>,
    session: &Rc<RefCell<Session>>,
    transport: &Transport,
    source: ResourceId,
    p: &Entity<Presentation>,
) {
    let saved = cx.update(|cx| cx.read_from_clipboard());
    for mode in [Mode::Code("txt".into()), Mode::Diff, Mode::Markdown] {
        let text = if matches!(mode, Mode::Diff) {
            "--- a/a.ml\n+++ b/a.ml\n@@ -1 +1 @@\n-aaa old\n+aaa new\n"
        } else {
            "aaa selectable text β\n\n[Local link](https://example.com/selection)\n"
        };
        let base = p.read_with(cx, |p, _| p.installed.as_ref().unwrap().revision);
        let generation = p.read_with(cx, |p, _| p.installed.as_ref().unwrap().generation + 1);
        selection_style::publish_streaming(
            &mut session.borrow_mut(),
            source,
            base,
            generation,
            0,
            text,
        );
        handle
            .update(cx, |view, _, cx| view.document_changed(source, cx))
            .unwrap();
        apply(
            cx,
            handle,
            vec![
                Op::SetDocument(node(1), document(source, mode.clone())),
                Op::SetStyle(node(0), parent(None)),
                Op::SetStyle(node(1), style(None)),
                Op::Bind(node(1), Some(handler(110))),
            ],
        );
        installed_revision(cx, handle, transport, p, base + 1).await;
        focus(cx, handle, p);
        key(cx, handle, "secondary-a");
        draw(cx, handle);
        assert!(
            !selected(cx, p).is_empty(),
            "{mode:?}: document default allows selection"
        );
        let (snapshot, editor, markdown) = p.read_with(cx, |p, _| {
            (
                p.installed.clone().unwrap(),
                p.editor.clone(),
                p.markdown.clone(),
            )
        });
        apply(cx, handle, vec![Op::SetStyle(node(0), parent(Some(false)))]);
        draw(cx, handle);
        assert!(
            selected(cx, p).is_empty(),
            "{mode:?}: disabling clears stale selection"
        );
        p.read_with(cx, |p, cx| {
            assert!(!p.user_selectable);
            assert!(!p.editor.read(cx).is_user_selectable());
            assert_eq!(p.editor, editor);
            assert_eq!(p.markdown, markdown);
            assert!(Arc::ptr_eq(p.installed.as_ref().unwrap(), &snapshot));
        });
        cx.update(|cx| {
            cx.write_to_clipboard(gpui::ClipboardItem::new_string("selection sentinel".into()))
        });
        for command in [
            "secondary-a",
            "shift-right",
            "secondary-shift-right",
            "shift-down",
            "secondary-c",
        ] {
            key(cx, handle, command);
        }
        draw(cx, handle);
        assert!(
            selected(cx, p).is_empty(),
            "{mode:?}: keyboard selection stays disabled"
        );
        assert_eq!(
            clipboard(cx).as_deref(),
            Some("selection sentinel"),
            "{mode:?}: no stale selection Copy"
        );
        if !matches!(mode, Mode::Markdown) {
            source_pointer(cx, handle, p);
        } else {
            markdown_pointer(cx, handle, p);
            // Links use their own keyboard path and queued bridge event.
            transport.mailbox.lock().unwrap().drain(128);
            key(cx, handle, "tab");
            draw(cx, handle);
            key(cx, handle, "enter");
            pause(cx).await;
            let events = transport.mailbox.lock().unwrap().drain(128);
            assert!(events.iter().any(|event| matches!(event, Event::DocumentNavigation(_, _, _, _, _, _, Navigation::Link(url)) if url == "https://example.com/selection")), "disabled selection preserves link activation: {events:?}");
        }
        // Explicit toolbar Copy is independent of selection.
        handle
            .update(cx, |_, window, cx| {
                let focus = p.read(cx).buttons["document-copy"].clone();
                window.focus(&focus, cx)
            })
            .unwrap();
        draw(cx, handle);
        key(cx, handle, "space");
        assert_eq!(
            clipboard(cx).as_deref(),
            Some(text),
            "{mode:?}: explicit Copy source remains usable"
        );
        apply(cx, handle, vec![Op::SetStyle(node(1), style(Some(true)))]);
        draw(cx, handle);
        focus(cx, handle, p);
        key(cx, handle, "secondary-a");
        draw(cx, handle);
        assert!(
            !selected(cx, p).is_empty(),
            "{mode:?}: local true overrides ancestor false"
        );
        key(cx, handle, "secondary-c");
        let expected = selected(cx, p);
        let expected = if matches!(mode, Mode::Markdown) {
            expected.trim()
        } else {
            &expected
        };
        assert_eq!(
            clipboard(cx).as_deref(),
            Some(expected),
            "{mode:?}: native selection Copy after re-enable"
        );
        apply(cx, handle, vec![Op::SetStyle(node(1), style(None))]);
        draw(cx, handle);
        assert!(
            selected(cx, p).is_empty(),
            "{mode:?}: unset restores inherited false"
        );
        apply(cx, handle, vec![Op::SetStyle(node(0), parent(None))]);
        draw(cx, handle);
        assert!(
            selected(cx, p).is_empty(),
            "{mode:?}: re-enabling does not resurrect old selection"
        );
        focus(cx, handle, p);
        key(cx, handle, "secondary-a");
        draw(cx, handle);
        assert!(
            !selected(cx, p).is_empty(),
            "{mode:?}: unset ancestor restores default true"
        );
        if let Some(markdown) = &markdown {
            markdown.update(cx, |m, cx| m.clear_selection(cx));
        }
        editor.update(cx, |e, cx| {
            e.bridge_select(0, 0, cx);
        });
        draw(cx, handle);
        cancel_drag(cx, handle, p).await;
        apply(cx, handle, vec![Op::SetStyle(node(1), style(Some(false)))]);
        draw(cx, handle);
        selection_style::publish_streaming(
            &mut session.borrow_mut(),
            source,
            base + 1,
            generation,
            text.len(),
            "\ncontinued aaa β\n",
        );
        handle
            .update(cx, |view, _, cx| view.document_changed(source, cx))
            .unwrap();
        installed_revision(cx, handle, transport, p, base + 2).await;
        focus(cx, handle, p);
        key(cx, handle, "secondary-a");
        draw(cx, handle);
        assert!(
            selected(cx, p).is_empty(),
            "{mode:?}: streaming must not re-enable user selection"
        );
        p.read_with(cx, |p, cx| {
            assert_eq!(p.editor, editor);
            assert_eq!(p.markdown, markdown);
            assert!(!p.editor.read(cx).is_user_selectable());
        });
        apply(cx, handle, vec![Op::SetStyle(node(1), style(None))]);
        draw(cx, handle);
        if matches!(mode, Mode::Markdown) {
            modal_selection(cx, handle, p).await;
        }
    }
    apply(cx, handle, vec![Op::Bind(node(1), None)]);
    cx.update(|cx| {
        cx.write_to_clipboard(
            saved.unwrap_or_else(|| gpui::ClipboardItem::new_string(String::new())),
        )
    });
    eprintln!(
        "GPUIO_DOCUMENT_SELECTION_POLICY_OK: code/diff/Markdown default, inherited disable, local override, unset, stale-range clearing, keyboard and source/Markdown single/double/triple-click pointer rejection, programmatic selection, selection Copy rejection, explicit Copy source, Markdown keyboard link, retained native identity, active-drag cancellation, no selection resurrection, streamed policy retention, and modal Copy isolation"
    );
}
