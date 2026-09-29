//! Shared Copy and retirement across ordinary text and prepared Markdown.
use super::*;
use crate::host::{
    editor_test::{frame, key},
    native_test::{mouse, move_mouse},
};

fn point(
    cx: &mut AsyncApp,
    handle: WindowHandle<View>,
    id: NodeId,
    byte: usize,
) -> gpui::Point<gpui::Pixels> {
    handle
        .update(cx, |view, _, _| view.selections[&id].borrow().point(byte))
        .unwrap()
}
fn copy(cx: &mut AsyncApp, handle: WindowHandle<View>) -> String {
    cx.update(|cx| cx.write_to_clipboard(gpui::ClipboardItem::new_string("unchanged".into())));
    key(cx, handle, "secondary-c");
    cx.update(|cx| {
        cx.read_from_clipboard()
            .and_then(|item| item.text())
            .unwrap()
    })
}
async fn drag(
    cx: &mut AsyncApp,
    handle: WindowHandle<View>,
    start: gpui::Point<gpui::Pixels>,
    end: gpui::Point<gpui::Pixels>,
) {
    move_mouse(cx, handle, start, false);
    mouse(cx, handle, start, true);
    move_mouse(cx, handle, end, true);
    mouse(cx, handle, end, false);
    frame(cx, handle).await;
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
    let revision = session
        .borrow()
        .document(source)
        .unwrap()
        .snapshot()
        .revision;
    publish(
        &mut session.borrow_mut(),
        source,
        revision,
        "markdown **β** value\n",
    );
    handle
        .update(cx, |view, _, cx| view.document_changed(source, cx))
        .unwrap();
    let mut config = document(source, Mode::Markdown);
    config.layout = Layout::Viewport(90.);
    apply(
        cx,
        handle,
        vec![
            Op::SetDocument(node(1), config),
            Op::SetStyle(node(1), vec![]),
            Op::SetStyle(
                node(0),
                vec![Style::Fields(vec![
                    Field::Width(Length::Px(440.)),
                    Field::Height(Length::Px(340.)),
                    Field::Display(1),
                    Field::Direction(1),
                    Field::UserSelect(true),
                    Field::FontSize(18.),
                ])],
            ),
            Op::Create(node(4), Kind::Text, "  before α".into(), None),
            Op::Create(node(5), Kind::Text, "after ζ  ".into(), None),
            Op::Splice(node(0), 0, 1, vec![node(4), node(1), node(5)]),
        ],
    );
    installed_revision(cx, handle, transport, p, revision + 1).await;
    frame(cx, handle).await;
    let start = point(cx, handle, node(4), 0);
    let end = point(cx, handle, node(5), "after ζ  ".len());
    let markdown = p.read_with(cx, |p, _| p.markdown.clone().unwrap());
    drag(cx, handle, start, end).await;
    let expected = "  before α\nmarkdown β value\nafter ζ  ";
    assert_eq!(
        copy(cx, handle),
        expected,
        "ordinary-origin mixed Copy uses rendered Markdown"
    );
    handle
        .update(cx, |_, window, cx| {
            let focus = markdown.read(cx).focus_handle().clone();
            window.focus(&focus, cx)
        })
        .unwrap();
    assert_eq!(
        copy(cx, handle),
        expected,
        "Markdown-focused Copy preserves neighboring whitespace"
    );
    key(cx, handle, "secondary-a");
    frame(cx, handle).await;
    assert_eq!(
        copy(cx, handle),
        "markdown β value",
        "Markdown Select All replaces the earlier cross-node selection"
    );
    drag(cx, handle, end, start).await;
    assert_eq!(
        copy(cx, handle),
        expected,
        "reverse mixed Copy stays in rendered order"
    );
    let bounds = markdown.read_with(cx, |m, _| m.bounds());
    let markdown_start = gpui::point(bounds.left() + px(1.), bounds.top() + px(10.));
    drag(cx, handle, markdown_start, end).await;
    assert_eq!(
        copy(cx, handle),
        "markdown β value\nafter ζ  ",
        "a drag starting in Markdown copies its ordinary-text neighbor too"
    );
    drag(cx, handle, start, end).await;
    publish(
        &mut session.borrow_mut(),
        source,
        revision + 1,
        "replacement **λ** body\n",
    );
    handle
        .update(cx, |view, _, cx| view.document_changed(source, cx))
        .unwrap();
    installed_revision(cx, handle, transport, p, revision + 2).await;
    frame(cx, handle).await;
    handle
        .update(cx, |_, w, cx| {
            assert!(
                gpui_base::TextSelection::selected_text(w, cx).is_empty(),
                "replacing selected Markdown retires the whole mixed geometric range"
            )
        })
        .unwrap();
    let start = point(cx, handle, node(4), 0);
    let end = point(cx, handle, node(5), "after ζ  ".len());
    drag(cx, handle, start, end).await;
    assert_eq!(
        copy(cx, handle),
        "  before α\nreplacement λ body\nafter ζ  ",
        "a fresh mixed gesture selects the installed replacement only"
    );
    // Return the fixture to its original tree before outer resource cleanup.
    apply(
        cx,
        handle,
        vec![
            Op::Splice(node(0), 0, 3, vec![node(1)]),
            Op::Remove(node(4)),
            Op::Remove(node(5)),
        ],
    );
    frame(cx, handle).await;
    handle
        .update(cx, |_, w, cx| {
            assert!(
                gpui_base::TextSelection::selected_text(w, cx).is_empty(),
                "removing ordinary endpoints retires mixed selection"
            )
        })
        .unwrap();
    cx.update(|cx| {
        cx.write_to_clipboard(
            saved.unwrap_or_else(|| gpui::ClipboardItem::new_string(String::new())),
        )
    });
    eprintln!(
        "GPUIO_MIXED_SELECTION_OK: ordinary/Markdown order, forward/reverse drag, focus-independent Copy, local Select All, Markdown-origin drag, installed-source retirement and endpoint unmount"
    );
}
