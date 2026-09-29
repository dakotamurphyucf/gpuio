//! Production scroll clipping and retained structural visibility. No OS input or
//! activation is needed; controlled panels use the same tree updates as Bonsai.
use super::*;

#[path = "highlight_deferred_test.rs"]
mod deferred;
#[path = "highlight_navigation_test.rs"]
mod navigation;
#[path = "highlight_state_style_test.rs"]
mod state_style;

fn frame_style(width: Length, height: f64) -> Vec<Style> {
    vec![Style::Fields(vec![
        Field::Width(width),
        Field::Height(Length::Px(height)),
        Field::Shrink(0.),
        Field::Background(Fill::Solid(Color::Rgba(0xffffffff))),
    ])]
}
fn panel(hidden: bool) -> Vec<Style> {
    let mut fields = vec![Field::Display(if hidden { 3 } else { 1 })];
    fields.extend([
        Field::Width(Length::Px(350.)),
        Field::Height(Length::Px(60.)),
        Field::Shrink(0.),
    ]);
    vec![Style::Fields(fields)]
}
async fn count(
    cx: &mut AsyncApp,
    handle: WindowHandle<View>,
    transport: &Transport,
    total: i64,
) -> Observation {
    let mut sample = None;
    for _ in 0..500 {
        draw(cx, handle);
        pause(cx).await;
        for (node, callback, value) in observations(transport) {
            if node == id(1) && callback == handler(1) && value.state == ready(total) {
                sample = Some(value);
            }
        }
        let current = handle
            .update(cx, |view, _, _| {
                view.highlights
                    .get(&id(1))
                    .map(|scope| scope.borrow().observation())
            })
            .unwrap();
        if let Some(sample) = &sample
            && current.as_ref().is_some_and(|current| {
                current.epoch == sample.epoch && current.state == ready(total)
            })
        {
            return sample.clone();
        }
    }
    let current = handle
        .update(cx, |view, _, _| {
            view.highlights
                .get(&id(1))
                .map(|scope| scope.borrow().observation())
        })
        .unwrap();
    panic!("missing current visible count {total}: last queued {sample:?}; current {current:?}");
}
fn red_pixels(cx: &mut AsyncApp, handle: WindowHandle<View>) -> usize {
    handle
        .update(cx, |_, window, _| {
            window
                .render_to_image()
                .unwrap()
                .pixels()
                .filter(|p| p.0 == [255, 0, 0, 255])
                .count()
        })
        .unwrap()
}
fn scroll(cx: &mut AsyncApp, handle: WindowHandle<View>, offset: f32) {
    handle
        .update(cx, |view, _, cx| {
            view.scrolls[&id(2)]
                .handle
                .set_offset(gpui::point(px(0.), px(offset)));
            cx.notify();
        })
        .unwrap();
    draw(cx, handle);
}
async fn scroll_checks(cx: &mut AsyncApp, handle: WindowHandle<View>, transport: &Transport) {
    let mut scroller = frame_style(Length::Px(350.), 120.);
    scroller.push(Style::Fields(vec![Field::OverflowY(3)]));
    apply(
        cx,
        handle,
        vec![
            Op::Create(id(0), Kind::Container, "".into(), None),
            Op::SetStyle(id(0), frame_style(Length::Percent(100.), 240.)),
            Op::Create(id(1), Kind::HighlightScope, "".into(), Some(handler(1))),
            Op::SetHighlightScope(id(1), config(0xff0000ff)),
            Op::SetStyle(id(1), frame_style(Length::Percent(100.), 240.)),
            Op::Create(id(2), Kind::Container, "".into(), None),
            Op::SetStyle(id(2), scroller),
            Op::Create(id(3), Kind::Container, "".into(), None),
            Op::SetStyle(id(3), frame_style(Length::Px(350.), 360.)),
            Op::Create(id(4), Kind::Text, "aaa top".into(), None),
            Op::SetStyle(id(4), row(0., false, false)),
            Op::Create(id(5), Kind::Text, "middle".into(), None),
            Op::SetStyle(id(5), row(160., false, false)),
            Op::Create(id(6), Kind::Text, "aaa bottom".into(), None),
            Op::SetStyle(id(6), row(300., false, false)),
            Op::Splice(id(3), 0, 0, vec![id(4), id(5), id(6)]),
            Op::Splice(id(2), 0, 0, vec![id(3)]),
            Op::Splice(id(1), 0, 0, vec![id(2)]),
            Op::Splice(id(0), 0, 0, vec![id(1)]),
            Op::SetRoot(Some(id(0))),
        ],
    );
    let initial = count(cx, handle, transport, 2).await;
    let retained = result(cx, handle);
    assert_eq!(pixel(cx, handle, 24., 2.), [255, 0, 0, 255]);
    assert_eq!(
        pixel(cx, handle, 24., 140.),
        [255, 255, 255, 255],
        "scroll mask clips outside the viewport"
    );
    scroll(cx, handle, -150.);
    pause(cx).await;
    assert_eq!(
        red_pixels(cx, handle),
        0,
        "middle viewport cannot retain a top or distant wash"
    );
    scroll(cx, handle, -240.);
    pause(cx).await;
    assert_eq!(
        pixel(cx, handle, 24., 62.),
        [255, 0, 0, 255],
        "bottom text and wash move together"
    );
    assert!(
        Arc::ptr_eq(&retained, &result(cx, handle)),
        "scrolling reuses matching work"
    );
    assert!(
        observations(transport).is_empty(),
        "scrolling alone changes no count/epoch"
    );
    // Changing offscreen text still changes the logical count; it must not paint
    // through the viewport or move the scroll position.
    scroll(cx, handle, -150.);
    apply(cx, handle, vec![Op::SetText(id(4), "aaa aaa top".into())]);
    let changed = count(cx, handle, transport, 3).await;
    assert!(changed.epoch > initial.epoch);
    assert_eq!(red_pixels(cx, handle), 0);
    assert_eq!(
        handle
            .update(cx, |view, _, _| view.scrolls[&id(2)].handle.offset().y)
            .unwrap(),
        px(-150.)
    );
    scroll(cx, handle, 0.);
    assert!(red_pixels(cx, handle) > 20);
}

async fn panel_checks(cx: &mut AsyncApp, handle: WindowHandle<View>, transport: &Transport) {
    let mut ops = vec![Op::Splice(id(1), 0, 1, vec![])];
    ops.extend((2..=6).rev().map(|n| Op::Remove(id(n))));
    ops.extend([
        Op::Create(id(7), Kind::TabPanel, "First tab".into(), None),
        Op::SetStyle(id(7), panel(false)),
        Op::Create(id(8), Kind::Text, "aaa".into(), None),
        Op::SetStyle(id(8), row(0., false, false)),
        Op::Create(id(9), Kind::TabPanel, "Second tab".into(), None),
        Op::SetStyle(id(9), panel(true)),
        Op::Create(id(10), Kind::Text, "aaa aaa".into(), None),
        Op::SetStyle(id(10), row(0., false, false)),
        Op::Splice(id(7), 0, 0, vec![id(8)]),
        Op::Splice(id(9), 0, 0, vec![id(10)]),
        Op::Splice(id(1), 0, 0, vec![id(7), id(9)]),
    ]);
    apply(cx, handle, ops);
    let first = count(cx, handle, transport, 1).await;
    assert!(red_pixels(cx, handle) > 20);
    apply(
        cx,
        handle,
        vec![
            Op::SetStyle(id(7), panel(true)),
            Op::SetStyle(id(9), panel(false)),
        ],
    );
    let second = count(cx, handle, transport, 2).await;
    assert!(second.epoch > first.epoch);
    let retained = result(cx, handle);
    apply(
        cx,
        handle,
        vec![Op::SetText(id(8), "aaa aaa aaa hidden".into())],
    );
    draw(cx, handle);
    pause(cx).await;
    assert!(
        observations(transport).is_empty(),
        "editing a hidden tab does not change visible matches"
    );
    assert!(
        Arc::ptr_eq(&retained, &result(cx, handle)),
        "hidden text edits reuse the visible result"
    );
    apply(
        cx,
        handle,
        vec![
            Op::SetStyle(id(7), panel(false)),
            Op::SetStyle(id(9), panel(true)),
        ],
    );
    count(cx, handle, transport, 3).await;

    let mut ops = vec![Op::Splice(id(1), 0, 2, vec![])];
    ops.extend((7..=10).rev().map(|n| Op::Remove(id(n))));
    ops.extend([
        Op::Create(id(11), Kind::Accordion, "".into(), None),
        Op::Create(id(12), Kind::Disclosure, "".into(), None),
        Op::Create(id(13), Kind::Button, "Section".into(), Some(handler(2))),
        Op::SetControl(id(13), Control::Button(false)),
        Op::Create(id(14), Kind::Panel, "Folded body".into(), None),
        Op::SetStyle(id(14), panel(true)),
        Op::Create(id(15), Kind::Text, "aaa aaa aaa".into(), None),
        Op::SetStyle(id(15), row(0., false, false)),
        Op::Splice(id(14), 0, 0, vec![id(15)]),
        Op::Splice(id(12), 0, 0, vec![id(13), id(14)]),
        Op::Splice(id(11), 0, 0, vec![id(12)]),
        Op::Splice(id(1), 0, 0, vec![id(11)]),
    ]);
    apply(cx, handle, ops);
    count(cx, handle, transport, 0).await;
    assert_eq!(red_pixels(cx, handle), 0);
    apply(cx, handle, vec![Op::SetStyle(id(14), panel(false))]);
    count(cx, handle, transport, 3).await;
    assert!(red_pixels(cx, handle) > 20);
    apply(cx, handle, vec![Op::SetStyle(id(14), panel(true))]);
    count(cx, handle, transport, 0).await;
    assert_eq!(
        red_pixels(cx, handle),
        0,
        "closed disclosure clears descendant paint"
    );
}

async fn responsive_checks(cx: &mut AsyncApp, handle: WindowHandle<View>, transport: &Transport) {
    use gpuio_protocol::container_query::{Config as Rules, Predicate, Range, Rule};
    let mut ops = vec![Op::Splice(id(1), 0, 1, vec![])];
    ops.extend((11..=15).rev().map(|n| Op::Remove(id(n))));
    ops.extend([
        Op::Create(id(16), Kind::ContainerQuery, "".into(), None),
        Op::SetStyle(id(16), frame_style(Length::Percent(100.), 100.)),
        Op::SetContainerQuery(
            id(16),
            Rules {
                generation: 1,
                branches: vec!["compact".into(), "wide".into()],
                default: 0,
                rules: vec![Rule {
                    condition: Predicate {
                        width: Range {
                            minimum: 450.,
                            maximum: None,
                        },
                        height: Range::ALL,
                    },
                    branch: 1,
                }],
            },
        ),
        Op::Create(id(17), Kind::Container, "".into(), None),
        Op::SetStyle(id(17), panel(false)),
        Op::Create(id(18), Kind::Text, "aaa compact".into(), None),
        Op::SetStyle(id(18), row(0., false, false)),
        Op::Create(id(19), Kind::Container, "".into(), None),
        Op::SetStyle(id(19), panel(false)),
        Op::Create(id(20), Kind::Text, "aaa aaa wide".into(), None),
        Op::SetStyle(id(20), row(0., false, false)),
        Op::Splice(id(17), 0, 0, vec![id(18)]),
        Op::Splice(id(19), 0, 0, vec![id(20)]),
        Op::Splice(id(16), 0, 0, vec![id(17), id(19)]),
        Op::Splice(id(1), 0, 0, vec![id(16)]),
    ]);
    apply(cx, handle, ops);
    let compact = count(cx, handle, transport, 1).await;
    let revision = handle
        .update(cx, |view, _, _| {
            view.session.borrow().tree(view.id).unwrap().revision()
        })
        .unwrap();
    handle
        .update(cx, |_, window, _| window.resize(size(px(520.), px(240.))))
        .unwrap();
    let wide = count(cx, handle, transport, 2).await;
    assert!(wide.epoch > compact.epoch);
    assert_eq!(
        handle
            .update(cx, |view, _, _| view
                .session
                .borrow()
                .tree(view.id)
                .unwrap()
                .revision())
            .unwrap(),
        revision,
        "native resize selects without a tree transaction"
    );
    let retained = result(cx, handle);
    handle
        .update(cx, |_, window, _| window.resize(size(px(560.), px(240.))))
        .unwrap();
    for _ in 0..8 {
        draw(cx, handle);
        pause(cx).await;
    }
    assert!(
        observations(transport).is_empty(),
        "same branch resize is silent"
    );
    assert!(Arc::ptr_eq(&retained, &result(cx, handle)));
    handle
        .update(cx, |_, window, _| window.resize(size(px(400.), px(240.))))
        .unwrap();
    count(cx, handle, transport, 1).await;
    assert!(red_pixels(cx, handle) > 20);
}

pub(super) async fn exercise(
    cx: &mut AsyncApp,
    session: &Rc<RefCell<Session>>,
    transport: &Arc<Transport>,
) {
    let window_id = WindowId::from_parts(0, 2).unwrap();
    session
        .borrow_mut()
        .open(3, window_id, "Highlight visibility", 400., 240.)
        .unwrap();
    let bounds = cx.update(|cx| {
        gpui_base::init(cx);
        Bounds::centered(None, size(px(400.), px(240.)), cx)
    });
    let handle = cx
        .open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                focus: false,
                ..Default::default()
            },
            |_, cx| cx.new(|_| View::new(window_id, session.clone(), transport.clone())),
        )
        .unwrap();
    transport.mailbox.lock().unwrap().drain(128);
    let checked=crate::host::native_test::protect(async {
        scroll_checks(cx,handle,transport).await;
        panel_checks(cx,handle,transport).await;
        responsive_checks(cx,handle,transport).await;
        state_style::exercise(cx,handle,transport).await;
        navigation::exercise(cx,handle,transport).await;
        eprintln!("GPUIO_NATIVE_HIGHLIGHT_VISIBILITY_OK: scroll clipping and result reuse, offscreen changes, retained tabs/disclosures, native responsive selection and silent same-branch resizing");
    }).await;
    let _ = handle.update(cx, |_, window, _| window.remove_window());
    let _ = session.borrow_mut().close(window_id);
    if let Err(error) = checked {
        std::panic::resume_unwind(error);
    }
    deferred::exercise(cx, session, transport).await;
}
