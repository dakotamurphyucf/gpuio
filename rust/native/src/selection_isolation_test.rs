//! Window-local selection and release across reused protocol window slots.
use super::*;

fn selected(cx: &mut gpui::AsyncApp, window: WindowHandle<View>) -> String {
    window
        .update(cx, |_, w, cx| {
            gpui_base::TextSelection::selected_text(w, cx)
        })
        .unwrap()
}
fn tree() -> Vec<Op> {
    vec![
        Op::Create(id(0), Kind::Container, "".into(), None),
        Op::Create(id(1), Kind::Text, "other α".into(), None),
        Op::Create(id(2), Kind::Text, "window β".into(), None),
        Op::SetStyle(
            id(0),
            vec![Style::Fields(vec![
                Field::Display(1),
                Field::Direction(1),
                Field::FontSize(18.),
                Field::Width(Length::Px(340.)),
                Field::Height(Length::Px(160.)),
                Field::UserSelect(true),
            ])],
        ),
        Op::Splice(id(0), 0, 0, vec![id(1), id(2)]),
        Op::SetRoot(Some(id(0))),
    ]
}
pub(super) async fn exercise(cx: &mut gpui::AsyncApp, first: WindowHandle<View>) {
    let (session, transport) = first
        .update(cx, |v, _, _| (v.session.clone(), v.transport.clone()))
        .unwrap();
    drag(cx, first, (id(2), 2), (id(4), 5)).await;
    let expected = "pha β\nmiddle 😀\nomega";
    assert_eq!(copy(cx, first), expected);
    for generation in 1..=2 {
        let second_id = WindowId::from_parts(1, generation).unwrap();
        session
            .borrow_mut()
            .open(
                100 + generation,
                second_id,
                "Selection isolation",
                340.,
                160.,
            )
            .unwrap();
        let bounds = cx.update(|cx| Bounds::centered(None, size(px(340.), px(160.)), cx));
        let second = cx
            .open_window(
                WindowOptions {
                    window_bounds: Some(WindowBounds::Windowed(bounds)),
                    focus: false,
                    ..Default::default()
                },
                |_, cx| cx.new(|_| View::new(second_id, session.clone(), transport.clone())),
            )
            .unwrap();
        let checked = protect(async {
            apply(cx, second, tree());
            frame(cx, second).await;
            assert!(
                selected(cx, second).is_empty(),
                "new window generation starts empty"
            );
            assert_eq!(
                selected(cx, first),
                expected,
                "mounting another window cannot reset the first"
            );
            drag(cx, second, (id(1), 0), (id(2), "window β".len())).await;
            assert_eq!(copy(cx, second), "other α\nwindow β");
            assert_eq!(
                copy(cx, first),
                expected,
                "same NodeIds must not mix windows' Copy"
            );
            second
                .update(cx, |_, w, cx| gpui_base::TextSelection::clear(w, cx))
                .unwrap();
            assert!(selected(cx, second).is_empty());
            assert_eq!(
                selected(cx, first),
                expected,
                "clearing one window is local"
            );
            drag(cx, second, (id(1), 0), (id(2), "window β".len())).await;
            apply(cx, second, vec![Op::SetText(id(2), "replacement γ".into())]);
            frame(cx, second).await;
            assert!(
                selected(cx, second).is_empty(),
                "replacement clears that window's geometry"
            );
            assert_eq!(selected(cx, first), expected);
            drag(cx, second, (id(1), 0), (id(2), "replacement γ".len())).await;
            assert_eq!(copy(cx, second), "other α\nreplacement γ");
            second
                .update(cx, |v, _, _| {
                    v.selections.values().map(Rc::downgrade).collect::<Vec<_>>()
                })
                .unwrap()
        })
        .await;
        let _ = second.update(cx, |_, w, _| w.remove_window());
        session.borrow_mut().close(second_id).unwrap();
        let owners = checked.unwrap_or_else(|error| std::panic::resume_unwind(error));
        frame(cx, first).await;
        assert!(
            owners.iter().all(|owner| owner.upgrade().is_none()),
            "closed window releases selection owners"
        );
        assert_eq!(
            copy(cx, first),
            expected,
            "surviving window keeps its range after close"
        );
    }
    eprintln!(
        "GPUIO_SELECTION_WINDOWS_OK: same-ID independent Copy/clear/source changes, close releases owners, reused window generation starts empty and survivor stays selected"
    );
}
