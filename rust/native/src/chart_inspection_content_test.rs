//! Foreground native dispatch and pixels for ordinary inspection child Views.
use super::interaction::{key, position};
use super::*;
use crate::host::native_test::{mouse, move_mouse};
use gpuio_protocol::chart_selection::Selection;
#[cfg(target_os = "macos")]
#[path = "chart_inspection_action_test.rs"]
mod accessibility;
#[path = "chart_inspection_aggregate_test.rs"]
mod aggregate;
#[path = "chart_inspection_editor_test.rs"]
mod editor;
use gpuio_protocol::chart_inspection_content::{Container, Entry, Target};
// This standalone fixture allocates the next contiguous retained slots.
fn wrapper() -> NodeId {
    NodeId::from_parts(2, 1).unwrap()
}
fn button() -> NodeId {
    NodeId::from_parts(3, 1).unwrap()
}
fn presses(transport: &Transport) -> usize {
    transport
        .mailbox
        .lock()
        .unwrap()
        .drain(1024)
        .into_iter()
        .filter(|e| matches!(e, Event::Press(_,node,..) if *node==button()))
        .count()
}
fn publish(session: &SharedSession, source: ResourceId, data: &Data, cx: &mut App) -> i64 {
    let old = session.borrow().chart(source).unwrap().snapshot().unwrap();
    stage_data(session, source, old.revision(), old.generation(), data);
    let revision = old.revision() + 1;
    let crate::session::ChartDispatch::Publish(work) = session
        .borrow_mut()
        .chart_request(Request::Publish(source, revision))
    else {
        panic!("publication")
    };
    assert_eq!(
        session.borrow_mut().complete_chart(work.run()),
        Response::Ack
    );
    crate::host::chart_source_changed(Some(source), cx);
    revision
}
pub(super) async fn exercise(
    cx: &mut gpui::AsyncApp,
    handle: WindowHandle<View>,
    source: ResourceId,
    session: &SharedSession,
    transport: &Transport,
) {
    let original = session.borrow().chart(source).unwrap().snapshot().unwrap();
    let mut chart = config(source, 0xff0000ff);
    chart.inspection_content = vec![Entry {
        target: Some(Target::Slice(7)),
        container: Container::Card,
    }];
    chart.style.inspection.card.width = 100.;
    chart.style.inspection.card.background = Some(0xff00ffff);
    apply(
        cx,
        handle,
        vec![
            Op::SetChart(id(1), Box::new(chart.clone())),
            Op::Create(wrapper(), Kind::Container, "".into(), None),
            Op::Create(
                button(),
                Kind::Button,
                "Inspect action".into(),
                Some(gpuio_protocol::HandlerId::from_parts(901, 1).unwrap()),
            ),
            Op::SetStyle(
                button(),
                vec![Style::Fields(vec![
                    Field::Width(Length::Px(80.)),
                    Field::Height(Length::Px(28.)),
                    Field::Background(Fill::Solid(Color::Rgba(0x00ff00ff))),
                ])],
            ),
            Op::Splice(wrapper(), 0, 0, vec![button()]),
            Op::Splice(id(1), 0, 0, vec![wrapper()]),
        ],
    );
    ready(cx, handle, original.revision(), 0xff0000ff).await;
    handle
        .update(cx, |view, window, cx| {
            window.focus(&view.charts[&id(1)].borrow().input.focus, cx)
        })
        .unwrap();
    // Browsing a mark does not commit a selection. Tab must still reach its
    // retained content, and returning to the chart must permit another preview.
    key(cx, handle, "escape");
    key(cx, handle, "home");
    draw(cx, handle);
    key(cx, handle, "tab");
    draw(cx, handle);
    handle
        .update(cx, |view, window, cx| {
            let state = view.charts[&id(1)].borrow();
            assert!(
                state.content.focus.contains_focused(window, cx),
                "Tab retains an uncommitted preview"
            );
            assert!(
                state.input.selected.is_none(),
                "Tab must not commit a selection"
            );
            window.focus(&state.input.focus, cx);
        })
        .unwrap();
    key(cx, handle, "home");
    key(cx, handle, "enter");
    draw(cx, handle);
    let point = handle
        .update(cx, |view, window, _| {
            assert!(view.focus.borrow().allows(button()));
            assert_eq!(
                view.charts[&id(1)].borrow().input.selected,
                Some(Selection::Slice(7))
            );
            let image = window.render_to_image().unwrap();
            assert!(
                image.pixels().filter(|p| p.0 == [0, 255, 0, 255]).count() > 100,
                "custom button pixels"
            );
            assert!(
                image.pixels().filter(|p| p.0 == [255, 0, 255, 255]).count() > 100,
                "native card backing pixels"
            );
            view.probes.borrow()[&button()].bounds.center()
        })
        .unwrap();
    presses(transport);
    move_mouse(cx, handle, point, false);
    draw(cx, handle);
    assert!(
        handle
            .update(cx, |view, _, _| view.focus.borrow().allows(button()))
            .unwrap(),
        "entering card must retain its target"
    );
    // Chart keyboard navigation takes precedence over a pointer resting in
    // the old card. Child editing keys are routed separately through focus.
    key(cx, handle, "end");
    assert!(
        !handle
            .update(cx, |view, _, _| view.focus.borrow().allows(button()))
            .unwrap(),
        "chart navigation supersedes pointer retention"
    );
    key(cx, handle, "home");
    draw(cx, handle);
    move_mouse(cx, handle, point, false);
    mouse(cx, handle, point, true);
    mouse(cx, handle, point, false);
    assert_eq!(
        presses(transport),
        1,
        "ordinary child button receives native pointer dispatch"
    );
    handle
        .update(cx, |view, window, cx| {
            window.focus(&view.charts[&id(1)].borrow().input.focus, cx)
        })
        .unwrap();
    key(cx, handle, "tab");
    draw(cx, handle);
    handle
        .update(cx, |view, window, cx| {
            assert!(
                view.charts[&id(1)]
                    .borrow()
                    .content
                    .focus
                    .contains_focused(window, cx),
                "Tab reaches custom control"
            )
        })
        .unwrap();
    move_mouse(cx, handle, position(40., 180.), false);
    draw(cx, handle);
    assert!(
        handle
            .update(cx, |view, _, _| view.focus.borrow().allows(button()))
            .unwrap(),
        "focused content survives pointer leaving its card"
    );
    // A new publication may move the mark index without remounting or
    // defocusing its exact-ID child, even while new geometry is pending.
    let mut reordered = original.data().clone();
    let Contents::Pie(values) = &mut reordered.contents else {
        panic!("pie data")
    };
    values.reverse();
    let revision = cx.update(|cx| publish(session, source, &reordered, cx));
    handle
        .update(cx, |view, window, cx| {
            assert!(
                view.focus.borrow().allows(button()),
                "same-ID publication retains eligibility before paint"
            );
            assert!(
                view.charts[&id(1)]
                    .borrow()
                    .content
                    .focus
                    .contains_focused(window, cx)
            );
        })
        .unwrap();
    ready(cx, handle, revision, 0xff0000ff).await;
    handle
        .update(cx, |view, window, cx| {
            assert!(
                view.focus.borrow().allows(button()),
                "reordered mark retains the focused child"
            );
            assert!(
                view.charts[&id(1)]
                    .borrow()
                    .content
                    .focus
                    .contains_focused(window, cx)
            );
        })
        .unwrap();
    let revision = cx.update(|cx| publish(session, source, original.data(), cx));
    ready(cx, handle, revision, 0xff0000ff).await;
    handle
        .update(cx, |view, window, cx| {
            window.focus(&view.charts[&id(1)].borrow().input.focus, cx)
        })
        .unwrap();
    key(cx, handle, "end");
    assert!(
        !handle
            .update(cx, |view, _, _| view.focus.borrow().allows(button()))
            .unwrap(),
        "another preview retires child before paint"
    );
    key(cx, handle, "home");
    draw(cx, handle);
    let point = handle
        .update(cx, |view, _, _| {
            view.probes.borrow()[&button()].bounds.center()
        })
        .unwrap();
    presses(transport);
    move_mouse(cx, handle, point, false);
    mouse(cx, handle, point, true);
    let mut absent = original.data().clone();
    let Contents::Pie(values) = &mut absent.contents else {
        panic!("pie data")
    };
    values.retain(|v| v.id != 7);
    let revision = cx.update(|cx| {
        publish(session, source, &absent, cx);
        handle
            .update(cx, |view, _, _| {
                assert!(
                    !view.focus.borrow().allows(button()),
                    "source retirement is immediate"
                )
            })
            .unwrap();
        publish(session, source, original.data(), cx)
    });
    mouse(cx, handle, point, false);
    assert_eq!(
        presses(transport),
        0,
        "retired child gesture cannot return with the source ID"
    );
    ready(cx, handle, revision, 0xff0000ff).await;
    key(cx, handle, "home");
    key(cx, handle, "enter");
    draw(cx, handle);
    // Metadata-only container changes reuse geometry and keep the retained child.
    chart.inspection_content[0].container = Container::Overlay;
    apply(
        cx,
        handle,
        vec![Op::SetChart(id(1), Box::new(chart.clone()))],
    );
    draw(cx, handle);
    handle
        .update(cx, |view, window, _| {
            assert!(view.focus.borrow().allows(button()));
            assert!(
                window
                    .render_to_image()
                    .unwrap()
                    .pixels()
                    .any(|p| p.0 == [0, 255, 0, 255])
            );
            assert!(
                !window
                    .render_to_image()
                    .unwrap()
                    .pixels()
                    .any(|p| p.0 == [255, 0, 255, 255]),
                "Overlay has no native card backing"
            );
        })
        .unwrap();
    #[cfg(target_os = "macos")]
    accessibility::exercise(cx, handle, source, session, transport, &chart).await;
    apply(
        cx,
        handle,
        vec![
            Op::SetChart(id(1), Box::new(config(source, 0xff0000ff))),
            Op::Splice(id(1), 0, 1, vec![]),
            Op::Remove(button()),
            Op::Remove(wrapper()),
        ],
    );
    let revision = session
        .borrow()
        .chart(source)
        .unwrap()
        .snapshot()
        .unwrap()
        .revision();
    ready(cx, handle, revision, 0xff0000ff).await;
    presses(transport);
    eprintln!(
        "GPUIO_INSPECTION_CONTENT_OK: real Card/Overlay pixels, ordinary button, pointer entry, Tab/focus retention, immediate target retirement and stale gesture rejection"
    );
    editor::exercise(cx, handle, source, session, transport).await;
    aggregate::exercise(cx, handle, source, session, transport).await;
}
