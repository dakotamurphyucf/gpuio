//! Production renderer/selection checks. The window remains in the background;
//! keys are dispatched through GPUI, not injected into another desktop app.
use super::*;
use gpuio_protocol::text_content::{Content, Span};
use std::os::fd::{AsRawFd, FromRawFd, OwnedFd};

fn id(slot: i64) -> NodeId {
    NodeId::from_parts(slot, 1).unwrap()
}
fn content(text: &str, first: i64, second: i64) -> Content {
    Content {
        text: text.into(),
        spans: vec![
            Span {
                start_byte: 0,
                end_byte: 2,
                foreground: first,
            },
            Span {
                start_byte: 3,
                end_byte: 5,
                foreground: second,
            },
            Span {
                start_byte: 9,
                end_byte: 11,
                foreground: first,
            },
            Span {
                start_byte: 12,
                end_byte: 18,
                foreground: second,
            },
        ],
    }
}
pub(super) fn apply(cx: &mut gpui::AsyncApp, handle: WindowHandle<View>, operations: Vec<Op>) {
    handle
        .update(cx, |view, window, cx| {
            let base = view.session.borrow().tree(view.id).unwrap().revision();
            let applied = view
                .session
                .borrow_mut()
                .apply(&Transaction {
                    window: view.id,
                    base,
                    revision: base + 1,
                    operations,
                })
                .unwrap();
            view.update_editors(&applied.dirty, window, cx);
            cx.notify();
        })
        .unwrap();
}
pub(super) fn draw(cx: &mut gpui::AsyncApp, handle: WindowHandle<View>) {
    cx.update_window(handle.into(), |_, window, cx| {
        window.refresh();
        window.draw(cx).clear(cx);
    })
    .unwrap();
}
fn row(top: f64, selectable: bool) -> Vec<Style> {
    vec![Style::Fields(vec![
        Field::Position(1),
        Field::Left(Length::Px(20.)),
        Field::Top(Length::Px(top)),
        Field::Width(Length::Px(280.)),
        Field::Height(Length::Px(90.)),
        Field::FontSize(24.),
        Field::LineHeight(Length::Px(30.)),
        Field::UserSelect(selectable),
        Field::Foreground(Color::Rgba(0x008000ff)),
    ])]
}
fn painted(cx: &mut gpui::AsyncApp, handle: WindowHandle<View>, top: f32) -> [usize; 3] {
    handle
        .update(cx, |_, window, _| {
            let image = window.render_to_image().unwrap();
            let scale = window.scale_factor();
            let mut count = [0; 3];
            for y in (top * scale) as u32..((top + 28.) * scale) as u32 {
                for x in (20. * scale) as u32..(300. * scale) as u32 {
                    let [r, g, b, _] = image.get_pixel(x, y).0;
                    count[0] += usize::from(r > 180 && g < 80 && b < 80);
                    count[1] += usize::from(b > 180 && r < 80 && g < 80);
                    count[2] += usize::from(g > 70 && g < 180 && r < 80 && b < 80);
                }
            }
            count
        })
        .unwrap()
}
pub(super) fn geometry(
    cx: &mut gpui::AsyncApp,
    handle: WindowHandle<View>,
) -> Vec<gpui::Point<gpui::Pixels>> {
    handle
        .update(cx, |view, _, _| {
            let state = view.selections[&id(2)].borrow();
            let layout = state.layout();
            state
                .text
                .char_indices()
                .filter_map(|(i, _)| layout.position_for_index(i))
                .collect()
        })
        .unwrap()
}
pub(super) fn assert_geometry(
    actual: &[gpui::Point<gpui::Pixels>],
    expected: &[gpui::Point<gpui::Pixels>],
) {
    assert_eq!(actual.len(), expected.len());
    for (actual, expected) in actual.iter().zip(expected) {
        // CoreText may sum f32 advances in different run groups. The measured
        // difference is ~0.00003 logical px, not a new wrap or visible movement.
        // Preserve exact line positions and allow less than 1/1024 px in x.
        assert_eq!(
            actual.y, expected.y,
            "color runs must preserve line wrapping"
        );
        assert!(
            (actual.x - expected.x).abs() <= px(1. / 1024.),
            "glyph position changed: {actual:?} vs {expected:?}"
        );
    }
}
pub(super) fn copied(cx: &mut gpui::AsyncApp, handle: WindowHandle<View>) -> String {
    editor_test::key(cx, handle, "secondary-c");
    cx.update(|cx| {
        cx.read_from_clipboard()
            .and_then(|item| item.text())
            .unwrap()
    })
}
#[cfg(target_os = "macos")]
pub(super) fn accessible_labels(
    cx: &mut gpui::AsyncApp,
    handle: WindowHandle<View>,
) -> Vec<String> {
    use objc2::{msg_send, runtime::AnyObject};
    use objc2_foundation::NSString;
    use raw_window_handle::HasWindowHandle;
    unsafe fn visit(object: *mut AnyObject, depth: usize, labels: &mut Vec<String>) {
        if object.is_null() || depth > 16 {
            return;
        }
        unsafe {
            let role: *mut NSString = msg_send![object, accessibilityRole];
            if !role.is_null() && (*role).to_string() == "AXStaticText" {
                let value: *mut NSString = msg_send![object, accessibilityTitle];
                if !value.is_null() {
                    labels.push((*value).to_string());
                }
            }
            let children: *mut AnyObject = msg_send![object, accessibilityChildren];
            if children.is_null() {
                return;
            }
            let count: usize = msg_send![children, count];
            assert!(count < 512);
            for index in 0..count {
                let child: *mut AnyObject = msg_send![children,objectAtIndex:index];
                visit(child, depth + 1, labels);
            }
        }
    }
    let raw = handle
        .update(cx, |_, window, _| {
            let raw_window_handle::RawWindowHandle::AppKit(raw) =
                window.window_handle().unwrap().as_raw()
            else {
                panic!("AppKit");
            };
            raw.ns_view.as_ptr() as usize
        })
        .unwrap();
    let mut labels = vec![];
    unsafe {
        let view = raw as *mut AnyObject;
        let native_window: *mut AnyObject = msg_send![view, window];
        let content: *mut AnyObject = msg_send![native_window, contentView];
        visit(content, 0, &mut labels);
    }
    labels
}
async fn exercise(cx: &mut gpui::AsyncApp, handle: WindowHandle<View>) {
    let source = "AA BB CC\né 世界 e\u{301} 👩‍💻 repeated words wrap onto another line";
    apply(
        cx,
        handle,
        vec![
            Op::Create(id(0), Kind::Container, String::new(), None),
            Op::SetStyle(
                id(0),
                vec![Style::Fields(vec![
                    Field::Width(Length::Px(340.)),
                    Field::Height(Length::Px(240.)),
                    Field::Background(Fill::Solid(Color::Rgba(0xffffffff))),
                ])],
            ),
            Op::Create(id(1), Kind::Text, source.into(), None),
            Op::SetStyle(id(1), row(10., false)),
            Op::Create(id(2), Kind::Text, source.into(), None),
            Op::SetStyle(id(2), row(130., true)),
            Op::Splice(id(0), 0, 0, vec![id(1), id(2)]),
            Op::SetRoot(Some(id(0))),
        ],
    );
    draw(cx, handle);
    let plain = geometry(cx, handle);
    assert!(
        plain.iter().any(|p| p.y > plain[0].y + px(30.)),
        "fixture wraps"
    );
    apply(
        cx,
        handle,
        vec![
            Op::SetStyledText(id(1), content(source, 0xff0000ff, 0x0000ffff)),
            Op::SetStyledText(id(2), content(source, 0xff0000ff, 0x0000ffff)),
        ],
    );
    draw(cx, handle);
    assert_geometry(&geometry(cx, handle), &plain);
    for top in [10., 130.] {
        let counts = painted(cx, handle, top);
        assert!(
            counts.iter().all(|count| *count > 20),
            "both runs and inherited glyphs must paint: {counts:?}"
        );
    }
    for top in [40., 160.] {
        let counts = painted(cx, handle, top);
        assert!(
            counts[0] > 20 && counts[1] > 20,
            "Unicode foreground glyphs: {counts:?}"
        );
    }
    #[cfg(target_os = "macos")]
    {
        let mut labels = vec![];
        for _ in 0..100 {
            labels = accessible_labels(cx, handle); // Enable lazy AccessKit tree.
            if labels
                .iter()
                .filter(|label| label.as_str() == source)
                .count()
                == 2
            {
                break;
            }
            cx.background_executor()
                .timer(std::time::Duration::from_millis(10))
                .await;
            draw(cx, handle);
        }
        if labels.is_empty() {
            eprintln!(
                "AX debug: {:?}",
                cx.update_window(handle.into(), |_, window, _| window.debug_a11y_tree_json())
                    .unwrap()
            );
        }
        assert_eq!(
            labels
                .iter()
                .filter(|label| label.as_str() == source)
                .count(),
            2,
            "full source AX labels: {labels:?}"
        );
        eprintln!("GPUIO_STYLED_TEXT_MACOS_AX_OK: two complete logical source labels");
    }
    let state = handle
        .update(cx, |view, window, cx| {
            let state = view.selections[&id(2)].clone();
            window.focus(&state.borrow().focus, cx);
            state
        })
        .unwrap();
    draw(cx, handle);
    editor_test::key(cx, handle, "secondary-a");
    draw(cx, handle);
    assert_eq!(copied(cx, handle), source, "copy is logical Unicode source");
    let selection = state.borrow().selection.clone();
    apply(
        cx,
        handle,
        vec![Op::SetStyledText(
            id(2),
            content(source, 0x0000ffff, 0xff0000ff),
        )],
    );
    draw(cx, handle);
    handle
        .update(cx, |view, _, _| {
            assert!(Rc::ptr_eq(&state, &view.selections[&id(2)]))
        })
        .unwrap();
    assert_eq!(
        state.borrow().selection,
        selection,
        "color-only update preserves selection"
    );
    assert_eq!(copied(cx, handle), source);
    assert_geometry(&geometry(cx, handle), &plain);
    apply(cx, handle, vec![Op::SetText(id(2), source.into())]);
    draw(cx, handle);
    assert_eq!(
        state.borrow().selection,
        selection,
        "plain transition preserves same-source selection"
    );
    assert_eq!(copied(cx, handle), source);
    editor_test::key(cx, handle, "home");
    draw(cx, handle);
    assert_eq!(
        &painted(cx, handle, 130.)[..2],
        &[0, 0],
        "plain text removes old foregrounds"
    );
    let weak = Rc::downgrade(&state);
    drop(state);
    apply(
        cx,
        handle,
        vec![
            Op::SetRoot(None),
            Op::Remove(id(2)),
            Op::Remove(id(1)),
            Op::Remove(id(0)),
        ],
    );
    draw(cx, handle);
    draw(cx, handle);
    assert!(
        weak.upgrade().is_none(),
        "unmounted selection state released"
    );
    handle
        .update(cx, |view, _, _| {
            assert_eq!(
                view.session
                    .borrow()
                    .tree(view.id)
                    .unwrap()
                    .retained_bytes(),
                0
            )
        })
        .unwrap();
    eprintln!(
        "GPUIO_STYLED_TEXT_OK: ordinary/selectable GPU foregrounds and inherited gaps, mixed Unicode wrap geometry, dispatched-key clipboard, same-source selection retention and teardown"
    );
}
pub(crate) fn run() {
    let failure = Rc::new(RefCell::new(None));
    let task_failure = failure.clone();
    let mut fds = [0; 2];
    assert_eq!(unsafe { libc::pipe(fds.as_mut_ptr()) }, 0);
    let _reader = unsafe { OwnedFd::from_raw_fd(fds[0]) };
    let writer = unsafe { OwnedFd::from_raw_fd(fds[1]) };
    let transport = Arc::new(Transport::new(writer.as_raw_fd()).unwrap());
    gpui_platform::application().run(move |cx| {
        cx.set_quit_mode(gpui::QuitMode::Explicit);
        let saved = cx.read_from_clipboard();
        let session = Rc::new(RefCell::new(Session::default()));
        session.borrow_mut().hello(VERSION, CAPABILITIES).unwrap();
        let window_id = WindowId::from_parts(0, 1).unwrap();
        session
            .borrow_mut()
            .open(1, window_id, "Styled text check", 340., 240.)
            .unwrap();
        let handle = cx
            .open_window(
                WindowOptions {
                    window_bounds: Some(WindowBounds::Windowed(Bounds::centered(
                        None,
                        size(px(340.), px(240.)),
                        cx,
                    ))),
                    focus: false,
                    ..Default::default()
                },
                |_, cx| cx.new(|_| View::new(window_id, session, transport)),
            )
            .unwrap();
        cx.spawn(async move |cx| {
            let checked = native_test::protect(exercise(cx, handle)).await;
            *task_failure.borrow_mut() = checked.err();
            let _ = handle.update(cx, |_, window, _| window.remove_window());
            cx.update(|cx| {
                cx.write_to_clipboard(
                    saved.unwrap_or_else(|| gpui::ClipboardItem::new_string(String::new())),
                );
                stop_application(cx);
            });
        })
        .detach();
    });
    if let Some(error) = failure.borrow_mut().take() {
        std::panic::resume_unwind(error);
    }
}
