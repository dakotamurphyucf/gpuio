//! Bounded native event batches, idle rendering and complete owner disposal.
use super::*;
use std::time::{Duration, Instant};
const OWNERS: i64 = 64;
const BATCH: i64 = 16;
fn id(slot: i64, generation: i64) -> NodeId {
    NodeId::from_parts(slot, generation).unwrap()
}
fn drain(transport: &Transport) -> Vec<Event> {
    let output = transport.mailbox.lock().unwrap().drain(256);
    assert!(!output.iter().any(|e| matches!(e, Event::Overloaded(_))));
    output
}
pub(super) async fn exercise(cx: &mut AsyncApp, handle: WindowHandle<View>, transport: &Transport) {
    drain(transport);
    let mut baseline = None;
    for generation in 1..=3 {
        let start = Instant::now();
        let root = id(0, generation);
        let first = id(1, generation);
        apply(
            cx,
            handle,
            vec![
                Op::Create(root, Kind::Container, "".into(), None),
                Op::SetRoot(Some(root)),
            ],
        );
        for offset in (0..OWNERS).step_by(BATCH as usize) {
            let mut operations = Vec::new();
            let mut children = Vec::new();
            for index in offset..offset + BATCH {
                let child = id(index + 1, generation);
                operations.extend([
                    Op::Create(
                        child,
                        Kind::Calendar,
                        "".into(),
                        Some(HandlerId::from_parts(index + 1, 1).unwrap()),
                    ),
                    Op::SetCalendar(
                        child,
                        Box::new(c::Config {
                            auto_focus: false,
                            ..config()
                        }),
                        c::Selection::Empty,
                        c::Month::new(2024, 2).unwrap(),
                    ),
                ]);
                children.push(child);
            }
            operations.push(Op::Splice(root, offset, 0, children));
            apply(cx, handle, operations);
            assert_eq!(
                drain(transport)
                    .iter()
                    .filter(|e| matches!(
                        e,
                        Event::CalendarEvent(_, _, _, _, c::Event::Observed(_))
                    ))
                    .count(),
                BATCH as usize
            );
        }
        frame(cx, handle).await;
        let (weak, retained) = handle
            .update(cx, |v, _, _| {
                assert_eq!(v.calendars.len(), OWNERS as usize);
                assert!(v.editors.is_empty() && v.numbers.is_empty() && v.otps.is_empty());
                (
                    v.calendars
                        .values()
                        .map(|c| c.state.downgrade())
                        .collect::<Vec<_>>(),
                    v.session.borrow().retained_bytes(),
                )
            })
            .unwrap();
        if let Some(before) = baseline {
            assert_eq!(retained, before);
        } else {
            baseline = Some(retained);
        }
        handle
            .update(cx, |v, w, cx| {
                assert!(matches!(
                    v.calendars[&first].command(&c::Command::Focus, w, cx),
                    c::Response::Applied(_)
                ))
            })
            .unwrap();
        frame(cx, handle).await;
        drain(transport);
        for _ in 0..8 {
            key(cx, handle, "left");
            key(cx, handle, "right");
        }
        frame(cx, handle).await;
        let output = drain(transport);
        assert_eq!(output.len(), 16); // Calendar navigation is discrete, not coalesced.
        assert!(
            output.iter().all(
                |e| matches!(e,Event::CalendarEvent(_,n,_,_,c::Event::Changed(_)) if *n==first)
            )
        );
        for offset in (0..256).step_by(BATCH as usize) {
            handle
                .update(cx, |v, w, cx| {
                    for index in offset..offset + BATCH {
                        let selection =
                            c::Selection::Single(date(2024, 2, if index % 2 == 0 { 1 } else { 2 }));
                        assert!(matches!(
                            v.calendars[&first].command(
                                &c::Command::Replace {
                                    selection,
                                    if_revision: None
                                },
                                w,
                                cx
                            ),
                            c::Response::Applied(_)
                        ));
                    }
                })
                .unwrap();
            let output = drain(transport);
            assert_eq!(output.len(), BATCH as usize);
            assert!(output.iter().all(
                |e| matches!(e,Event::CalendarEvent(_,n,_,_,c::Event::Observed(_)) if *n==first)
            ));
        }
        handle.update(cx, |_, w, cx| w.blur(cx)).unwrap();
        frame(cx, handle).await;
        drain(transport);
        cx.background_executor()
            .timer(Duration::from_millis(100))
            .await;
        drain(transport);
        let renders = handle.update(cx, |v, _, _| v.render_count).unwrap();
        cx.background_executor()
            .timer(Duration::from_millis(150))
            .await;
        assert_eq!(
            handle.update(cx, |v, _, _| v.render_count).unwrap(),
            renders
        );
        assert!(drain(transport).is_empty());
        let mut remove = vec![Op::SetRoot(None), Op::Splice(root, 0, OWNERS, vec![])];
        remove.extend((1..=OWNERS).map(|slot| Op::Remove(id(slot, generation))));
        remove.push(Op::Remove(root));
        apply(cx, handle, remove);
        frame(cx, handle).await;
        assert!(weak.iter().all(|owner| owner.upgrade().is_none()));
        handle
            .update(cx, |v, _, _| {
                assert!(v.calendars.is_empty());
                assert_eq!(v.session.borrow().retained_bytes(), 0);
            })
            .unwrap();
        drain(transport);
        eprintln!(
            "GPUIO_CALENDAR_WORKLOAD cycle={generation} owners={OWNERS} retained_bytes={retained} elapsed={:?}",
            start.elapsed()
        );
    }
    handle
        .update(cx, |v, w, _| {
            v.session.borrow_mut().close(v.id).unwrap();
            w.remove_window();
        })
        .unwrap();
    eprintln!(
        "GPUIO_CALENDAR_WORKLOAD_OK: three 64-owner cycles, discrete native/command events, idle and complete disposal"
    );
}
