//! Real GPUI window event dispatch; OS cross-application transfer is a separate
//! test because injected GPUI MouseDown is not an AppKit NSEvent.
use super::*;
use gpuio_protocol::drag_drop::*;
fn source(text: &str, disabled: bool) -> Source {
    Source::new(
        "Drag text".into(),
        Payload::text(text.into()).unwrap(),
        disabled,
        false,
    )
    .unwrap()
}
fn target(formats: Vec<Format>, disabled: bool) -> Target {
    Target::new("Drop here".into(), formats, disabled).unwrap()
}
fn region(left: f64, width: f64) -> Vec<Style> {
    vec![Style::Fields(vec![
        Field::Position(1),
        Field::Left(Length::Px(left)),
        Field::Top(Length::Px(100.)),
        Field::Width(Length::Px(width)),
        Field::Height(Length::Px(120.)),
        Field::Background(Fill::Solid(Color::Rgba(0x4488ccff))),
    ])]
}
fn events(transport: &Transport) -> Vec<Event> {
    transport
        .mailbox
        .lock()
        .unwrap()
        .drain(128)
        .into_iter()
        .filter(|event| {
            matches!(
                event,
                Event::DragSourceEvent(..) | Event::DropTargetEvent(..)
            )
        })
        .collect()
}
fn draw(cx: &mut gpui::AsyncApp, handle: WindowHandle<View>) {
    cx.update_window(handle.into(), |_, window, cx| {
        window.refresh();
        window.draw(cx).clear(cx);
    })
    .unwrap();
}
fn moving(cx: &mut gpui::AsyncApp, handle: WindowHandle<View>, x: f32) {
    super::super::native_test::move_mouse(cx, handle, gpui::point(px(x), px(130.)), true);
}
fn begin(cx: &mut gpui::AsyncApp, handle: WindowHandle<View>) {
    let point = gpui::point(px(40.), px(130.));
    super::super::native_test::move_mouse(cx, handle, point, false);
    super::super::native_test::mouse(cx, handle, point, true);
    moving(cx, handle, 50.);
    draw(cx, handle);
    assert!(
        cx.update_window(handle.into(), |_, _, cx| cx.has_active_drag())
            .unwrap(),
        "GPUI owns an active drag"
    );
}
fn release(cx: &mut gpui::AsyncApp, handle: WindowHandle<View>) {
    super::super::native_test::mouse(cx, handle, gpui::point(px(240.), px(130.)), false);
    draw(cx, handle);
}
fn dropped(events: &[Event]) -> Vec<(NodeId, Payload)> {
    events
        .iter()
        .filter_map(|event| match event {
            Event::DropTargetEvent(
                _,
                node,
                _,
                _,
                TargetSample {
                    phase: TargetPhase::Dropped(payload),
                    ..
                },
            ) => Some((*node, payload.clone())),
            _ => None,
        })
        .collect()
}
fn outcomes(events: &[Event]) -> Vec<Outcome> {
    events
        .iter()
        .filter_map(|event| match event {
            Event::DragSourceEvent(
                _,
                _,
                _,
                _,
                SourceSample {
                    phase: SourcePhase::Ended(outcome),
                    ..
                },
            ) => Some(*outcome),
            _ => None,
        })
        .collect()
}
pub(super) async fn exercise(
    cx: &mut gpui::AsyncApp,
    handle: WindowHandle<View>,
    transport: &Transport,
) {
    let handler = |slot| Some(gpuio_protocol::HandlerId::from_parts(slot, 1).unwrap());
    apply(
        cx,
        handle,
        vec![
            Op::Create(node(90), Kind::DragSource, "".into(), handler(90)),
            Op::SetDragSource(node(90), source("first", false)),
            Op::SetStyle(node(90), region(10., 110.)),
            Op::Create(node(91), Kind::DropTarget, "".into(), handler(91)),
            Op::SetDropTarget(node(91), target(vec![Format::Text], false)),
            Op::SetStyle(node(91), region(200., 180.)),
            Op::Create(node(92), Kind::DropTarget, "".into(), handler(92)),
            Op::SetDropTarget(node(92), target(vec![Format::Files], false)),
            Op::SetStyle(
                node(92),
                vec![Style::Fields(vec![
                    Field::Width(Length::Px(170.)),
                    Field::Height(Length::Px(110.)),
                ])],
            ),
            Op::Splice(node(91), 0, 0, vec![node(92)]),
            Op::Splice(node(0), 4, 0, vec![node(90), node(91)]),
        ],
    );
    frame(cx, handle).await;
    events(transport);
    begin(cx, handle);
    apply(
        cx,
        handle,
        vec![Op::SetDragSource(node(90), source("second", false))],
    );
    draw(cx, handle);
    moving(cx, handle, 240.);
    release(cx, handle);
    let first = events(transport);
    assert_eq!(
        dropped(&first),
        vec![(node(91), Payload::text("first".into()).unwrap())],
        "rejecting nested target leaves immutable offer for accepting parent: {first:?}"
    );
    assert_eq!(outcomes(&first), vec![Outcome::InternalDrop]);
    assert_eq!(
        first
            .iter()
            .filter(|e| matches!(
                e,
                Event::DragSourceEvent(
                    _,
                    _,
                    _,
                    _,
                    SourceSample {
                        phase: SourcePhase::Started(_),
                        ..
                    }
                )
            ))
            .count(),
        1
    );

    apply(
        cx,
        handle,
        vec![Op::SetDropTarget(
            node(92),
            target(vec![Format::Text], false),
        )],
    );
    draw(cx, handle);
    begin(cx, handle);
    moving(cx, handle, 240.);
    release(cx, handle);
    let next = events(transport);
    assert_eq!(
        dropped(&next),
        vec![(node(92), Payload::text("second".into()).unwrap())],
        "nearest accepting child consumes once"
    );
    assert_eq!(outcomes(&next), vec![Outcome::InternalDrop]);

    begin(cx, handle);
    key(cx, handle, "escape");
    draw(cx, handle);
    assert_eq!(
        outcomes(&events(transport)),
        vec![Outcome::Cancelled(CancelReason::Escape)]
    );
    release(cx, handle);
    assert!(dropped(&events(transport)).is_empty());

    begin(cx, handle);
    apply(
        cx,
        handle,
        vec![Op::SetDragSource(node(90), source("second", true))],
    );
    draw(cx, handle);
    assert_eq!(
        outcomes(&events(transport)),
        vec![Outcome::Cancelled(CancelReason::Disabled)]
    );
    assert!(
        !cx.update_window(handle.into(), |_, _, cx| cx.has_active_drag())
            .unwrap()
    );
    release(cx, handle);
    events(transport);

    apply(
        cx,
        handle,
        vec![Op::SetDragSource(node(90), source("third", false))],
    );
    draw(cx, handle);
    begin(cx, handle);
    moving(cx, handle, 150.);
    super::super::native_test::mouse(cx, handle, gpui::point(px(150.), px(130.)), false);
    draw(cx, handle);
    let unhandled = events(transport);
    assert!(dropped(&unhandled).is_empty());
    assert_eq!(outcomes(&unhandled), vec![Outcome::Unconfirmed]);
    begin(cx, handle);
    let mut hidden = region(10., 110.);
    hidden.push(Style::Fields(vec![Field::Visibility(1)]));
    apply(cx, handle, vec![Op::SetStyle(node(90), hidden)]);
    draw(cx, handle);
    assert_eq!(
        outcomes(&events(transport)),
        vec![Outcome::Cancelled(CancelReason::Hidden)]
    );
    release(cx, handle);
    events(transport);
    apply(cx, handle, vec![Op::SetStyle(node(90), region(10., 110.))]);
    draw(cx, handle);

    begin(cx, handle);
    moving(cx, handle, 240.);
    events(transport);
    apply(
        cx,
        handle,
        vec![Op::SetDropTarget(
            node(92),
            target(vec![Format::Files], false),
        )],
    );
    let changed = events(transport);
    assert!(changed.iter().any(|e| matches!(e, Event::DropTargetEvent(_, id, _, _, TargetSample {phase:TargetPhase::Left,..}) if *id == node(92))), "changing acceptance ends hover without requiring another mouse move");
    release(cx, handle);
    assert_eq!(
        dropped(&events(transport)),
        vec![(node(91), Payload::text("third".into()).unwrap())]
    );

    // Native incoming file paths have unknown directory metadata, preserve raw
    // bytes, and use the same nested target acceptance path.
    apply(
        cx,
        handle,
        vec![Op::SetDropTarget(
            node(92),
            target(vec![Format::Files], false),
        )],
    );
    draw(cx, handle);
    let point = gpui::point(px(240.), px(130.));
    let raw_path = std::path::PathBuf::from(std::ffi::OsStr::from_bytes(b"/tmp/\xff"));
    cx.update_window(handle.into(), |_, window, cx| {
        window.dispatch_event(
            gpui::PlatformInput::FileDrop(gpui::FileDropEvent::Entered {
                position: point,
                paths: gpui::ExternalPaths([raw_path].into_iter().collect()),
            }),
            cx,
        );
        window.dispatch_event(
            gpui::PlatformInput::FileDrop(gpui::FileDropEvent::Submit { position: point }),
            cx,
        );
    })
    .unwrap();
    draw(cx, handle);
    let incoming = dropped(&events(transport));
    assert_eq!(incoming.len(), 1);
    assert_eq!(incoming[0].0, node(92));
    let PayloadRef::Files(files) = incoming[0].1.as_ref() else {
        panic!()
    };
    assert_eq!(files[0].path.as_bytes(), b"/tmp/\xff");
    assert_eq!(files[0].is_directory, None);

    // Oversized incoming offers reject atomically; never deliver a prefix.
    cx.update_window(handle.into(), |_, window, cx| {
        let paths = gpui::ExternalPaths(
            (0..129)
                .map(|_| std::path::PathBuf::from("/tmp/a"))
                .collect(),
        );
        window.dispatch_event(
            gpui::PlatformInput::FileDrop(gpui::FileDropEvent::Entered {
                position: point,
                paths,
            }),
            cx,
        );
        window.dispatch_event(
            gpui::PlatformInput::FileDrop(gpui::FileDropEvent::Submit { position: point }),
            cx,
        );
    })
    .unwrap();
    draw(cx, handle);
    let rejected = events(transport);
    assert!(dropped(&rejected).is_empty());
    assert_eq!(
        rejected
            .iter()
            .filter(|e| matches!(
                e,
                Event::DropTargetEvent(
                    _,
                    _,
                    _,
                    _,
                    TargetSample {
                        phase: TargetPhase::Rejected(Rejection::LimitExceeded),
                        ..
                    }
                )
            ))
            .count(),
        1
    );
    // A fresh source gesture is still usable after incoming native sessions.
    begin(cx, handle);
    events(transport);
    let old_config = cx
        .update(|cx| super::super::drag_drop::source_config_weak(cx))
        .unwrap();
    apply(
        cx,
        handle,
        vec![
            Op::Splice(node(0), 4, 2, vec![]),
            Op::Remove(node(90)),
            Op::Remove(node(92)),
            Op::Remove(node(91)),
        ],
    );
    draw(cx, handle);
    assert!(
        !cx.update_window(handle.into(), |_, _, cx| cx.has_active_drag())
            .unwrap()
    );
    assert!(
        outcomes(&events(transport)).is_empty(),
        "removed handlers receive no late terminal callback"
    );
    assert!(
        old_config.upgrade().is_none(),
        "removed source config is not retained by the ghost or manager"
    );
    assert!(
        cx.update(|cx| super::super::drag_drop::is_empty(cx)),
        "gesture and hover state is disposed"
    );
    println!(
        "GPUIO_DRAG_DROP_NATIVE_OK: nested acceptance, immutable snapshot, cancellation, removal, raw file drops and bounds"
    );
}
use std::os::unix::ffi::OsStrExt;
