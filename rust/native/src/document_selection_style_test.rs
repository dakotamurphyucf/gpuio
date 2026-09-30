//! Selection colors cascade into source and Markdown without replacing content.
use super::*;

fn parent_style(color: Option<i64>) -> Vec<Style> {
    let mut fields = vec![
        Field::Width(Length::Px(440.)),
        Field::Height(Length::Px(340.)),
        Field::Background(Fill::Solid(Color::Rgba(0xffffffff))),
    ];
    if let Some(color) = color {
        fields.push(Field::SelectionColor(Color::Rgba(color)));
    }
    vec![Style::Fields(fields)]
}
fn pixels(cx: &mut AsyncApp, handle: WindowHandle<View>, color: [u8; 4]) -> usize {
    handle
        .update(cx, |_, window, _| {
            window
                .render_to_image()
                .unwrap()
                .pixels()
                .filter(|p| p.0 == color)
                .count()
        })
        .unwrap()
}
pub(super) fn publish_streaming(
    session: &mut Session,
    source: ResourceId,
    base: i64,
    generation: i64,
    from: usize,
    text: &str,
) {
    let revision = base + 1;
    for request in [
        Request::Begin(Update {
            id: source,
            base,
            revision,
            generation,
            from_byte: from as i64,
            suffix_bytes: text.len() as i64,
            status: Status::Streaming,
        }),
        Request::Chunk(
            source,
            revision,
            0,
            gpuio_protocol::asset::Chunk::new(text.as_bytes().to_vec()).unwrap(),
        ),
        Request::Publish(source, revision),
    ] {
        assert_eq!(
            session.document_request(request),
            Response::Ack,
            "stream publication base={base}, generation={generation}, from={from}"
        );
    }
}
pub(super) async fn exercise(
    cx: &mut AsyncApp,
    handle: WindowHandle<View>,
    session: &Rc<RefCell<Session>>,
    transport: &Transport,
    source: ResourceId,
    p: &Entity<Presentation>,
) {
    for mode in [Mode::Code("txt".into()), Mode::Markdown] {
        let (base, generation) = p.read_with(cx, |p, _| {
            let source = p.installed.as_ref().unwrap();
            (source.revision, source.generation + 1)
        });
        publish_streaming(
            &mut session.borrow_mut(),
            source,
            base,
            generation,
            0,
            "aaa selected words\n",
        );
        handle
            .update(cx, |view, _, cx| view.document_changed(source, cx))
            .unwrap();
        apply(
            cx,
            handle,
            vec![
                Op::SetDocument(node(1), document(source, mode.clone())),
                Op::SetStyle(node(0), parent_style(Some(0xff00ffff))),
                Op::SetStyle(node(1), vec![]),
            ],
        );
        installed_revision(cx, handle, transport, p, base + 1).await;
        let (snapshot, editor, markdown) = p.read_with(cx, |p, _| {
            (
                p.installed.clone().unwrap(),
                p.editor.clone(),
                p.markdown.clone(),
            )
        });
        handle
            .update(cx, |_, window, cx| {
                window.activate_window();
                if matches!(mode, Mode::Markdown) {
                    let markdown = markdown.as_ref().unwrap();
                    markdown.update(cx, |state, cx| state.select_all(cx));
                    let focus = markdown.read(cx).focus_handle().clone();
                    window.focus(&focus, cx);
                } else {
                    editor.update(cx, |editor, cx| assert!(editor.bridge_select(0, 3, cx)));
                    window.focus(&editor.read(cx).focus_handle(cx), cx);
                }
            })
            .unwrap();
        draw(cx, handle);
        assert!(
            pixels(cx, handle, [255, 0, 255, 255]) > 20,
            "{mode:?}: inherited selection color reaches GPU"
        );
        apply(
            cx,
            handle,
            vec![Op::SetStyle(
                node(1),
                vec![
                    Style::Fields(vec![Field::SelectionColor(Color::Rgba(0x0000ffff))]),
                    Style::Fields(vec![Field::SelectionColor(Color::Rgba(0x00ffffff))]),
                ],
            )],
        );
        draw(cx, handle);
        assert!(
            pixels(cx, handle, [0, 255, 255, 255]) > 20,
            "{mode:?}: last local declaration wins"
        );
        assert_eq!(pixels(cx, handle, [255, 0, 255, 255]), 0);
        apply(
            cx,
            handle,
            vec![Op::SetStyle(node(0), parent_style(Some(0x00ff00ff)))],
        );
        draw(cx, handle);
        assert!(
            pixels(cx, handle, [0, 255, 255, 255]) > 20,
            "{mode:?}: local override survives ancestor restyle"
        );
        apply(cx, handle, vec![Op::SetStyle(node(1), vec![])]);
        draw(cx, handle);
        assert!(
            pixels(cx, handle, [0, 255, 0, 255]) > 20,
            "{mode:?}: clearing local override restores inheritance"
        );
        apply(cx, handle, vec![Op::SetStyle(node(0), parent_style(None))]);
        draw(cx, handle);
        assert_eq!(
            pixels(cx, handle, [0, 255, 0, 255]),
            0,
            "{mode:?}: clearing last declaration restores native default"
        );
        p.read_with(cx, |p, cx| {
            assert!(
                Arc::ptr_eq(p.installed.as_ref().unwrap(), &snapshot),
                "paint restyling never installs a new source snapshot"
            );
            assert_eq!(p.editor, editor);
            assert_eq!(p.markdown, markdown);
            if matches!(mode, Mode::Markdown) {
                assert!(
                    !markdown
                        .as_ref()
                        .unwrap()
                        .read(cx)
                        .selected_text()
                        .is_empty()
                );
            } else {
                assert_eq!(editor.read(cx).bridge_selection(), (0, 3));
            }
        });
        let selected_text = markdown
            .as_ref()
            .map(|state| state.read_with(cx, |state, _| state.selected_text()));
        apply(
            cx,
            handle,
            vec![Op::SetStyle(
                node(1),
                vec![Style::Fields(vec![Field::SelectionColor(Color::Rgba(
                    0x00ffffff,
                ))])],
            )],
        );
        let suffix = "\ncontinued β\n";
        let revision = snapshot.revision + 1;
        publish_streaming(
            &mut session.borrow_mut(),
            source,
            snapshot.revision,
            snapshot.generation,
            snapshot.text.len(),
            suffix,
        );
        handle
            .update(cx, |view, _, cx| view.document_changed(source, cx))
            .unwrap();
        installed_revision(cx, handle, transport, p, revision).await;
        draw(cx, handle);
        assert!(
            pixels(cx, handle, [0, 255, 255, 255]) > 20,
            "{mode:?}: streamed installation keeps selection color"
        );
        if let Some(markdown) = &markdown {
            assert_eq!(
                markdown.read_with(cx, |state, _| state.selected_text()),
                selected_text.unwrap()
            );
        } else {
            assert_eq!(
                editor.read_with(cx, |editor, _| editor.bridge_selection()),
                (0, 3)
            );
        }
        p.read_with(cx, |p, _| {
            assert_eq!(p.editor, editor);
            assert_eq!(p.markdown, markdown);
        });
        apply(cx, handle, vec![Op::SetStyle(node(1), vec![])]);
        if let Some(markdown) = markdown {
            markdown.update(cx, |s, cx| s.clear_selection(cx));
        }
        editor.update(cx, |e, cx| {
            e.bridge_select(0, 0, cx);
        });
    }
    eprintln!(
        "GPUIO_DOCUMENT_SELECTION_STYLE_OK: source/Markdown inherited color, last-local override, ancestor restyle, local/default restoration, GPU pixels and selected-source/native-entity retention through streamed publication"
    );
}
