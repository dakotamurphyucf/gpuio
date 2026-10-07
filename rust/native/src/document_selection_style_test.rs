//! Selection colors cascade into source and Markdown without replacing content.
use super::*;
use crate::host::native_test::{mouse, move_mouse};

fn backward_pointer(
    cx: &mut AsyncApp,
    handle: WindowHandle<View>,
    text: &Entity<gpui_base::TextViewState>,
    held: bool,
) {
    let bounds = text.read_with(cx, |text, _| text.bounds());
    let end = gpui::point(bounds.left() + px(1.), bounds.top() + px(10.));
    let start = end + gpui::point(px(45.), px(0.));
    move_mouse(cx, handle, start, false);
    mouse(cx, handle, start, true);
    move_mouse(cx, handle, end, true);
    draw(cx, handle);
    if !held {
        mouse(cx, handle, end, false);
        draw(cx, handle);
    }
}

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

pub(super) fn directional(
    cx: &mut AsyncApp,
    handle: WindowHandle<View>,
    presentation: &Entity<Presentation>,
    selected: &str,
    start: usize,
) {
    let text = presentation.read_with(cx, |p, _| p.markdown.clone().unwrap());
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
    draw(cx, handle);
    let baseline = handle
        .update(cx, |_, window, _| window.render_to_image().unwrap())
        .unwrap();
    assert!(
        baseline
            .pixels()
            .filter(|pixel| pixel.0 == [255, 0, 0, 255])
            .count()
            > 20,
        "reference search wash must be present"
    );
    text.update(cx, |state, cx| {
        let projection = state.rendered_text().unwrap();
        assert_eq!(&projection.text()[start..start + selected.len()], selected);
        let request = state
            .prepare_rendered_selection(
                &projection.position(start + selected.len()).unwrap(),
                &projection.position(start).unwrap(),
            )
            .unwrap();
        state.apply_rendered_selection(request, cx).unwrap();
    });
    draw(cx, handle);
    let painted = handle
        .update(cx, |_, window, _| window.render_to_image().unwrap())
        .unwrap();
    assert_eq!(baseline.dimensions(), painted.dimensions());
    // These fixtures each occupy one visual row. Fill foreground glyph holes
    // in the reference wash per column: a color emoji can itself contain pure
    // white pixels, so "white before" does not mean "outside the selection".
    let mut columns = vec![None::<(u32, u32)>; baseline.width() as usize];
    for (x, y, pixel) in baseline.enumerate_pixels() {
        if pixel.0 == [255, 0, 0, 255] {
            let span = columns[x as usize].get_or_insert((y, y));
            span.0 = span.0.min(y);
            span.1 = span.1.max(y);
        }
    }
    for (x, y, pixel) in painted.enumerate_pixels() {
        if pixel.0 == [0, 255, 255, 255] {
            assert!(
                columns[x as usize].is_some_and(|(top, bottom)| (top..=bottom).contains(&y)),
                "selection must not bridge unselected visual cells: {selected:?} at ({x}, {y})"
            );
        }
    }
    for (before, after) in baseline.pixels().zip(painted.pixels()) {
        if before.0 == [255, 0, 0, 255] {
            assert_eq!(
                after.0,
                [0, 255, 255, 255],
                "selection must cover every search-wash cell: {selected:?}"
            );
        }
    }
    assert!(
        pixels(cx, handle, [0, 255, 255, 255]) > 20,
        "RTL middle glyph receives native selection paint"
    );
    assert_eq!(
        handle
            .update(cx, |_, window, cx| gpui_base::TextSelection::selected_text(
                window, cx
            ))
            .unwrap(),
        selected
    );
    assert!(text.read_with(cx, |state, _| {
        state.rendered_selection().unwrap().is_backward()
    }));
    text.update(cx, |state, cx| state.clear_selection(cx));
    draw(cx, handle);
    assert_eq!(pixels(cx, handle, [0, 255, 255, 255]), 0);
    assert!(
        red_pixels(cx, handle) > 20,
        "clearing selection keeps the search wash"
    );
    // A one-cluster reference supplies actual glyph-cell edges for pointer
    // validation, independently of the production caret mapping. The mixed
    // prefix case above has disjoint cells and is covered by exact range paint.
    if selected != "A א" {
        let (scale, _) = handle
            .update(cx, |_, window, _| {
                window.activate_window();
                (window.scale_factor(), ())
            })
            .unwrap();
        let first = columns.iter().position(Option::is_some).unwrap();
        let last = columns.iter().rposition(Option::is_some).unwrap();
        let (top, bottom) = columns[first].unwrap();
        let y = px((top + bottom + 1) as f32 / (2. * scale));
        let left = gpui::point(px((first as f32 + 0.5) / scale), y);
        let right = gpui::point(px((last as f32 + 0.5) / scale), y);
        let rtl = selected == "ב" || selected == "ح";
        for (anchor, head, backward) in [(left, right, rtl), (right, left, !rtl)] {
            move_mouse(cx, handle, anchor, false);
            mouse(cx, handle, anchor, true);
            move_mouse(cx, handle, head, true);
            mouse(cx, handle, head, false);
            draw(cx, handle);
            assert_eq!(
                handle
                    .update(cx, |_, window, cx| gpui_base::TextSelection::selected_text(
                        window, cx
                    ))
                    .unwrap(),
                selected,
                "glyph-cell pointer Copy: {selected:?}"
            );
            text.read_with(cx, |state, _| {
                let range = state.rendered_selection().expect("mapped pointer range");
                assert_eq!(range.bytes(), start..start + selected.len());
                assert_eq!(range.is_backward(), backward);
                assert!(state.requested_rendered_selection().is_none());
            });
            assert!(pixels(cx, handle, [0, 255, 255, 255]) > 20);
        }
        text.update(cx, |state, cx| state.clear_selection(cx));
        draw(cx, handle);
    }
    apply(cx, handle, vec![Op::SetStyle(node(1), vec![])]);
    eprintln!(
        "GPUIO_RENDERED_SELECTION_RTL_GPU_OK: selected={selected:?}, directed range, glyph-cell pixels, native pointer/Copy and clear"
    );
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
    for (mode, requested, pointer, same_paragraph, held) in [
        (Mode::Code("txt".into()), false, false, false, false),
        (Mode::Markdown, false, false, false, false),
        (Mode::Markdown, true, false, false, false),
        (Mode::Markdown, false, true, false, false),
        (Mode::Markdown, false, false, true, false),
        (Mode::Markdown, false, true, false, true),
    ] {
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
                    markdown.update(cx, |state, cx| {
                        if requested {
                            let text = state.rendered_text().unwrap();
                            let request = state
                                .prepare_rendered_selection(
                                    &text.position(3).unwrap(),
                                    &text.position(0).unwrap(),
                                )
                                .unwrap();
                            state.apply_rendered_selection(request, cx).unwrap();
                        } else {
                            state.select_all(cx);
                        }
                    });
                    let focus = markdown.read(cx).focus_handle().clone();
                    window.focus(&focus, cx);
                } else {
                    editor.update(cx, |editor, cx| assert!(editor.bridge_select(0, 3, cx)));
                    window.focus(&editor.read(cx).focus_handle(cx), cx);
                }
            })
            .unwrap();
        draw(cx, handle);
        if pointer {
            backward_pointer(cx, handle, markdown.as_ref().unwrap(), held);
        }
        let exact_copy = (requested || pointer).then(|| {
            markdown.as_ref().unwrap().read_with(cx, |state, _| {
                let range = state.rendered_selection().unwrap();
                assert!(range.is_backward());
                assert_eq!(state.requested_rendered_selection().is_some(), requested);
                let text = state.rendered_text().unwrap();
                let selected = text.selected_text(&range).unwrap().trim().to_owned();
                assert!(!selected.is_empty());
                assert!(selected.len() < text.text().trim().len());
                if requested {
                    assert_eq!(selected, "aaa");
                }
                selected
            })
        });
        if let Some(expected) = &exact_copy {
            assert_eq!(
                handle
                    .update(cx, |_, window, cx| gpui_base::TextSelection::selected_text(
                        window, cx
                    ))
                    .unwrap(),
                *expected
            );
        }
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
        let suffix = if same_paragraph {
            "continued β\n"
        } else {
            "\ncontinued β\n"
        };
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
            let selected_text = selected_text.unwrap();
            let expected = if same_paragraph {
                selected_text.strip_suffix('\n').unwrap()
            } else {
                &selected_text
            };
            assert_eq!(
                markdown.read_with(cx, |state, _| state.selected_text()),
                expected
            );
            markdown.read_with(cx, |state, _| {
                let range = state
                    .rendered_selection()
                    .expect("streamed current logical range");
                assert_eq!(
                    state.rendered_text().unwrap().selected_text(&range),
                    Some(expected)
                );
                if same_paragraph {
                    assert_eq!(range.bytes(), 0.."aaa selected words".len());
                }
            });
            crate::host::editor_test::key(cx, handle, "secondary-c");
            assert_eq!(
                cx.update(|cx| cx
                    .read_from_clipboard()
                    .and_then(|item| item.text())
                    .unwrap()),
                expected.trim()
            );
            if let Some(expected) = &exact_copy {
                assert!(
                    markdown
                        .read_with(cx, |s, _| { s.rendered_selection().unwrap().is_backward() })
                );
                assert_eq!(
                    handle
                        .update(cx, |_, window, cx| gpui_base::TextSelection::selected_text(
                            window, cx
                        ))
                        .unwrap(),
                    *expected
                );
                eprintln!(
                    "GPUIO_RENDERED_SELECTION_RANGE_GPU_OK: requested={requested}, pointer={pointer}, backward range, actual selection pixels, window Copy, restyle and streamed retention"
                );
            }
            if held {
                let before = markdown.read_with(cx, |s, _| s.rendered_selection().unwrap());
                let bounds = markdown.read_with(cx, |s, _| s.bounds());
                let end = gpui::point(bounds.left() + px(21.), bounds.top() + px(10.));
                move_mouse(cx, handle, end, true);
                mouse(cx, handle, end, false);
                draw(cx, handle);
                let selected = markdown.read_with(cx, |s, _| {
                    let text = s.rendered_text().unwrap();
                    let after = s.rendered_selection().expect("continued streamed drag");
                    assert_eq!(text.offset(after.anchor()), text.offset(before.anchor()));
                    assert!(
                        text.offset(after.head()).unwrap() > text.offset(before.head()).unwrap()
                    );
                    assert!(after.is_backward());
                    text.selected_text(&after).unwrap().trim().to_owned()
                });
                assert!(!selected.is_empty());
                assert!(pixels(cx, handle, [0, 255, 255, 255]) > 20);
                crate::host::editor_test::key(cx, handle, "secondary-c");
                assert_eq!(
                    cx.update(|cx| cx.read_from_clipboard().unwrap().text().unwrap()),
                    selected
                );
                eprintln!(
                    "GPUIO_RENDERED_HELD_STREAM_OK: current anchor, continued drag, GPU pixels and keyboard Copy"
                );
            }
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
