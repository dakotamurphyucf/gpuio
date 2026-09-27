//! Bounded admission, native input and disposal with many retained slider owners.
use super::*;
use std::time::{Duration, Instant};

const OWNERS: i64 = 1024;
const BATCH: i64 = 64;
fn id(slot: i64, generation: i64) -> NodeId {
    // The preceding functional suite allocated and removed slots 0..=6.
    NodeId::from_parts(slot, generation + i64::from(slot <= 6)).unwrap()
}
fn drain(transport: &Transport) -> Vec<Event> {
    let events = transport.mailbox.lock().unwrap().drain(256);
    assert!(
        !events
            .iter()
            .any(|event| matches!(event, Event::Overloaded(_))),
        "admitted batches must not overload native input"
    );
    events
}

pub(super) async fn exercise(
    cx: &mut gpui::AsyncApp,
    handle: WindowHandle<View>,
    transport: &Transport,
) {
    drain(transport);
    let mut retained_baseline = None;
    for generation in 1..=3 {
        let root = id(0, generation);
        let first = id(1, generation);
        let started = Instant::now();
        apply(
            cx,
            handle,
            vec![
                Op::Create(root, Kind::Container, "".into(), None),
                Op::SetRoot(Some(root)),
            ],
        );
        // Observed-on-mount is a discrete event. Submit batches below the input
        // lane's 128-event capacity, draining between submissions as the runtime
        // does. This does not claim arbitrarily large atomic mount batches.
        for offset in (0..OWNERS).step_by(BATCH as usize) {
            let mut ops = Vec::new();
            let mut children = Vec::new();
            for index in offset..offset + BATCH {
                let node = id(1 + index, generation);
                ops.push(Op::Create(
                    node,
                    Kind::Slider,
                    "".into(),
                    Some(handler(1 + index)),
                ));
                ops.push(Op::SetSlider(node, config(), initial()));
                children.push(node);
            }
            ops.push(Op::Splice(root, offset, 0, children));
            apply(cx, handle, ops);
            let mounted = drain(transport);
            assert_eq!(
                mounted
                    .iter()
                    .filter(|event| matches!(
                        event,
                        Event::SliderEvent(_, _, _, _, s::Event::Observed(_))
                    ))
                    .count(),
                BATCH as usize
            );
        }
        frame(cx, handle).await;
        let (owners, retained) = handle
            .update(cx, |v, _, _| {
                assert_eq!(v.sliders.len(), OWNERS as usize);
                assert_eq!(
                    v.sliders
                        .values()
                        .map(|state| state.borrow().focus.len())
                        .sum::<usize>(),
                    2 * OWNERS as usize
                );
                (
                    v.sliders.values().map(Rc::downgrade).collect::<Vec<_>>(),
                    v.session.borrow().retained_bytes(),
                )
            })
            .unwrap();
        if let Some(previous) = retained_baseline {
            assert_eq!(retained, previous);
        } else {
            retained_baseline = Some(retained);
        }
        let mount_time = started.elapsed();
        let revision = handle
            .update(cx, |v, w, cx| {
                w.focus(&v.sliders[&first].borrow().focus[0].1, cx);
                v.session.borrow().tree(v.id).unwrap().revision()
            })
            .unwrap();
        frame(cx, handle).await;
        let input_started = Instant::now();
        for _ in 0..32 {
            key(cx, handle, "right");
            key(cx, handle, "left");
        }
        frame(cx, handle).await;
        let input_time = input_started.elapsed();
        eprintln!(
            "GPUIO_SLIDER_WORKLOAD_INPUT cycle={generation} mount={mount_time:?} keyboard64_and_frame={input_time:?}"
        );
        let keyboard = drain(transport);
        assert_eq!(keyboard.iter().filter(|event| matches!(event, Event::SliderEvent(_, node, _, _, s::Event::Committed(s::Source::Keyboard, _)) if *node == first)).count(), 64);
        handle
            .update(cx, |v, _, _| {
                assert_eq!(v.sliders[&first].borrow().model.snapshot().value, initial());
                assert_eq!(
                    v.session.borrow().tree(v.id).unwrap().revision(),
                    revision,
                    "native keyboard work must not require tree transactions"
                );
            })
            .unwrap();
        // Continuous pointer updates retain one latest adjacent preview and all
        // discrete boundaries even when no consumer drains during the gesture.
        let bounds = handle
            .update(cx, |v, _, _| v.sliders[&first].borrow().track_bounds)
            .unwrap();
        let at = |fraction| {
            gpui::point(
                bounds.left() + bounds.size.width * fraction,
                bounds.center().y,
            )
        };
        mouse(cx, handle, at(0.4), true);
        // One burst before the event-loop/frame boundary. Calling the outer
        // AsyncApp update helper for every sample forces a full debug redraw
        // per synthetic event, which is a different, frame-throughput workload.
        cx.update_window(handle.into(), |_, window, cx| {
            for index in 0..2000 {
                window.dispatch_event(
                    gpui::PlatformInput::MouseMove(gpui::MouseMoveEvent {
                        position: at(if index % 2 == 0 { 0.2 } else { 0.6 }),
                        pressed_button: Some(gpui::MouseButton::Left),
                        modifiers: Default::default(),
                    }),
                    cx,
                );
            }
        })
        .unwrap();
        mouse(cx, handle, at(0.6), false);
        let gesture = drain(transport);
        assert_eq!(
            gesture.len(),
            3,
            "begin, latest preview and final commit only"
        );
        assert!(matches!(
            &gesture[0],
            Event::SliderEvent(_, _, _, _, s::Event::DragStarted(_))
        ));
        assert!(matches!(
            &gesture[1],
            Event::SliderEvent(_, _, _, _, s::Event::Preview(_))
        ));
        assert!(matches!(
            &gesture[2],
            Event::SliderEvent(_, _, _, _, s::Event::Committed(s::Source::Pointer, _))
        ));
        frame(cx, handle).await;
        let render_count = handle.update(cx, |v, _, _| v.render_count).unwrap();
        cx.background_executor()
            .timer(Duration::from_millis(150))
            .await;
        assert_eq!(
            handle.update(cx, |v, _, _| v.render_count).unwrap(),
            render_count
        );
        assert!(
            drain(transport).is_empty(),
            "stationary owners produce no bridge observations"
        );
        let mut remove = vec![Op::SetRoot(None), Op::Splice(root, 0, OWNERS, vec![])];
        remove.extend((0..OWNERS).map(|index| Op::Remove(id(1 + index, generation))));
        remove.push(Op::Remove(root));
        apply(cx, handle, remove);
        frame(cx, handle).await;
        assert!(owners.iter().all(|owner| owner.upgrade().is_none()));
        handle
            .update(cx, |v, _, _| {
                assert!(v.sliders.is_empty());
                assert_eq!(v.session.borrow().retained_bytes(), 0);
            })
            .unwrap();
        drain(transport);
        eprintln!(
            "GPUIO_SLIDER_WORKLOAD cycle={generation} owners={OWNERS} retained_bytes={retained} mount={mount_time:?} keyboard64_and_frame={input_time:?} elapsed={:?}",
            started.elapsed()
        );
    }
    eprintln!(
        "GPUIO_SLIDER_WORKLOAD_OK: 1024 owners/2048 thumbs, three disposal cycles, native keyboard, 2000 coalesced moves, idle and zero retained owners"
    );
}
