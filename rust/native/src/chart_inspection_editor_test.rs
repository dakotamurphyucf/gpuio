//! Foreground native editor routing and AppKit composition inside a chart card.
use super::*;

fn editor_id(generation: i64) -> NodeId {
    NodeId::from_parts(3, generation).unwrap()
}
fn snapshot(cx: &mut gpui::AsyncApp, handle: WindowHandle<View>, node: NodeId) -> EditorSnapshot {
    handle
        .update(cx, |view, window, cx| {
            view.editors[&node].snapshot(window, cx)
        })
        .unwrap()
}
fn command(
    cx: &mut gpui::AsyncApp,
    handle: WindowHandle<View>,
    node: NodeId,
    command: EditorCommand,
) -> EditorResult {
    handle
        .update(cx, |view, window, cx| {
            view.editors
                .get_mut(&node)
                .unwrap()
                .command(&command, window, cx)
        })
        .unwrap()
}
async fn focus(cx: &mut gpui::AsyncApp, handle: WindowHandle<View>, node: NodeId) {
    handle
        .update(cx, |view, window, cx| {
            window.focus(&view.charts[&id(1)].borrow().input.focus, cx)
        })
        .unwrap();
    key(cx, handle, "home");
    key(cx, handle, "enter");
    draw(cx, handle);
    key(cx, handle, "tab");
    crate::host::editor_test::frame(cx, handle).await;
    assert!(
        snapshot(cx, handle, node).focused,
        "Tab enters the native editor"
    );
}
pub(super) async fn exercise(
    cx: &mut gpui::AsyncApp,
    handle: WindowHandle<View>,
    source: ResourceId,
    session: &SharedSession,
    transport: &Transport,
) {
    for (kind, generation) in [(Kind::Input, 2), (Kind::Textarea, 3)] {
        let node = editor_id(generation);
        let wrapper = NodeId::from_parts(2, generation).unwrap();
        let original = session.borrow().chart(source).unwrap().snapshot().unwrap();
        let mut chart = config(source, 0xff0000ff);
        chart.style.inspection.card.width = 180.;
        chart.inspection_content = vec![Entry {
            target: Some(Target::Slice(7)),
            container: Container::Card,
        }];
        apply(
            cx,
            handle,
            vec![
                Op::SetChart(id(1), Box::new(chart.clone())),
                Op::Create(wrapper, Kind::Container, "".into(), None),
                Op::Create(
                    node,
                    kind,
                    "seed".into(),
                    Some(gpuio_protocol::HandlerId::from_parts(902, generation).unwrap()),
                ),
                Op::SetEditor(
                    node,
                    EditorConfig {
                        label: "Inspection draft".into(),
                        placeholder: "".into(),
                        read_only: false,
                        disabled: false,
                        submit_on_enter: false,
                        auto_focus: false,
                        min_rows: 1,
                        max_rows: if kind == Kind::Input { 1 } else { 2 },
                    },
                ),
                Op::SetStyle(
                    node,
                    vec![Style::Fields(vec![
                        Field::Width(Length::Px(140.)),
                        Field::Height(Length::Px(36.)),
                    ])],
                ),
                Op::Splice(wrapper, 0, 0, vec![node]),
                Op::Splice(id(1), 0, 0, vec![wrapper]),
            ],
        );
        ready(cx, handle, original.revision(), 0xff0000ff).await;
        focus(cx, handle, node).await;
        key(
            cx,
            handle,
            if cfg!(target_os = "macos") {
                "cmd-a"
            } else {
                "ctrl-a"
            },
        );
        crate::host::editor_test::native_text(cx, handle, "draft é", false);
        let before = snapshot(cx, handle, node);
        assert_eq!(before.text, "draft é");
        key(cx, handle, "left");
        let moved = snapshot(cx, handle, node);
        assert!(
            moved.selection.head < before.selection.head,
            "editing key moves the editor caret"
        );
        handle
            .update(cx, |view, _, _| {
                assert_eq!(
                    view.charts[&id(1)].borrow().input.selected,
                    Some(Selection::Slice(7))
                )
            })
            .unwrap();
        // A child style update must retain the native draft and its caret.
        apply(
            cx,
            handle,
            vec![Op::SetStyle(
                node,
                vec![Style::Fields(vec![
                    Field::Width(Length::Px(140.)),
                    Field::Height(Length::Px(36.)),
                    Field::FontSize(13.),
                ])],
            )],
        );
        draw(cx, handle);
        assert_eq!(snapshot(cx, handle, node), moved);
        #[cfg(target_os = "macos")]
        {
            key(cx, handle, "cmd-a");
            crate::host::editor_test::native_text(cx, handle, "に", true);
            let marked = snapshot(cx, handle, node);
            assert_eq!(marked.text, "に");
            assert!(marked.composition.is_some());
            let mut reordered = original.data().clone();
            let Contents::Pie(values) = &mut reordered.contents else {
                panic!("pie")
            };
            values.reverse();
            let revision = cx.update(|cx| publish(session, source, &reordered, cx));
            assert_eq!(
                snapshot(cx, handle, node),
                marked,
                "publication does not reset composition"
            );
            ready(cx, handle, revision, 0xff0000ff).await;
            assert_eq!(
                snapshot(cx, handle, node),
                marked,
                "prepared reorder retains focused composition"
            );
            crate::host::editor_test::native_text(cx, handle, "日本", false);
            let committed = snapshot(cx, handle, node);
            assert_eq!(committed.text, "日本");
            assert!(committed.composition.is_none());
            crate::host::editor_test::native_text(cx, handle, "語", true);
            assert!(snapshot(cx, handle, node).composition.is_some());
        }
        let retained = snapshot(cx, handle, node);
        // Targetless metadata models an ineligible application/resource binding.
        // Retirement must occur before a frame and must reject late native text.
        chart.inspection_content[0].target = None;
        apply(
            cx,
            handle,
            vec![Op::SetChart(id(1), Box::new(chart.clone()))],
        );
        let hidden = snapshot(cx, handle, node);
        assert!(!hidden.focused);
        assert!(
            hidden.composition.is_none(),
            "hiding ends the old text-client composition"
        );
        assert_eq!(hidden.text, retained.text);
        assert_eq!(
            command(cx, handle, node, EditorCommand::Focus),
            EditorResult::Failed(EditorError::FocusBlocked)
        );
        crate::host::editor_test::native_text(cx, handle, "stale", false);
        assert_eq!(
            snapshot(cx, handle, node),
            hidden,
            "retired native input handler cannot edit before repaint"
        );
        chart.inspection_content[0].target = Some(Target::Slice(7));
        apply(
            cx,
            handle,
            vec![Op::SetChart(id(1), Box::new(chart.clone()))],
        );
        draw(cx, handle);
        // Restore original order before keyboard Home selects the first slice.
        let revision = cx.update(|cx| publish(session, source, original.data(), cx));
        ready(cx, handle, revision, 0xff0000ff).await;
        focus(cx, handle, node).await;
        assert_eq!(
            snapshot(cx, handle, node).text,
            retained.text,
            "return preserves the retained native draft"
        );
        #[cfg(target_os = "macos")]
        {
            crate::host::editor_test::native_text(cx, handle, "仮", true);
            assert!(snapshot(cx, handle, node).composition.is_some());
        }
        let live = session.borrow().chart(source).unwrap().snapshot().unwrap();
        let revision = live.revision() + 1;
        stage_data(
            session,
            source,
            live.revision(),
            live.generation() + 1,
            original.data(),
        );
        let crate::session::ChartDispatch::Publish(work) = session
            .borrow_mut()
            .chart_request(Request::Publish(source, revision))
        else {
            panic!("reset")
        };
        cx.update(|cx| {
            assert_eq!(
                session.borrow_mut().complete_chart(work.run()),
                Response::Ack
            );
            crate::host::chart_source_changed(Some(source), cx);
        });
        let reset = snapshot(cx, handle, node);
        assert!(
            !reset.focused && reset.composition.is_none(),
            "source generation reset retires composition before paint"
        );
        assert_eq!(
            command(cx, handle, node, EditorCommand::Focus),
            EditorResult::Failed(EditorError::FocusBlocked)
        );
        crate::host::editor_test::native_text(cx, handle, "late reset", false);
        assert_eq!(snapshot(cx, handle, node), reset);
        ready(cx, handle, revision, 0xff0000ff).await;
        focus(cx, handle, node).await;
        assert_eq!(snapshot(cx, handle, node).text, reset.text);
        // Original-data browsing hides the card and removes native editor focus.
        handle
            .update(cx, |view, window, cx| {
                window.focus(&view.charts[&id(1)].borrow().input.focus, cx)
            })
            .unwrap();
        key(cx, handle, "d");
        assert_eq!(
            command(cx, handle, node, EditorCommand::Focus),
            EditorResult::Failed(EditorError::FocusBlocked)
        );
        assert!(!snapshot(cx, handle, node).focused);
        key(cx, handle, "escape");
        draw(cx, handle);
        focus(cx, handle, node).await;
        apply(
            cx,
            handle,
            vec![
                Op::SetChart(id(1), Box::new(config(source, 0xff0000ff))),
                Op::Splice(id(1), 0, 1, vec![]),
                Op::Remove(node),
                Op::Remove(wrapper),
            ],
        );
        handle
            .update(cx, |view, _, _| assert!(!view.editors.contains_key(&node)))
            .unwrap();
        ready(cx, handle, revision, 0xff0000ff).await;
        transport.mailbox.lock().unwrap().drain(1024);
    }
    eprintln!(
        "GPUIO_INSPECTION_EDITOR_OK: Input/Textarea native Tab/caret/draft, style and source reorder retention, immediate targetless/reset/browser retirement, stale text rejection, recovery and unmount; AppKit composition exercised={}",
        cfg!(target_os = "macos")
    );
}
