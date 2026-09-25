//! Retained native editor ownership, bounded history/event traffic and disposal.
use super::*;
use std::time::{Duration, Instant};

const OWNERS: i64 = 256;
const BATCH: i64 = 32;
fn id(slot: i64, cycle: i64) -> NodeId {
    // The preceding functional suite retired slots 0 through 4 once.
    NodeId::from_parts(slot, cycle + i64::from(slot <= 4)).unwrap()
}
fn drain(transport: &Transport) -> Vec<Event> {
    let events = transport.mailbox.lock().unwrap().drain(256);
    assert!(
        !events
            .iter()
            .any(|event| matches!(event, Event::Overloaded(_)))
    );
    events
}

pub(super) async fn exercise(cx: &mut AsyncApp, handle: WindowHandle<View>, transport: &Transport) {
    drain(transport);
    let mut retained_baseline = None;
    for cycle in 1..=3 {
        let root = id(0, cycle);
        let first = id(1, cycle);
        let started = Instant::now();
        apply(
            cx,
            handle,
            vec![
                Op::Create(root, Kind::Container, "".into(), None),
                Op::SetRoot(Some(root)),
            ],
        );
        // Mount observations are discrete; bounded batches model the runtime's
        // drain between updates rather than overflowing its 128-event lane.
        for offset in (0..OWNERS).step_by(BATCH as usize) {
            let mut ops = Vec::new();
            let mut children = Vec::new();
            for index in offset..offset + BATCH {
                let child = id(index + 1, cycle);
                ops.extend([
                    Op::Create(
                        child,
                        Kind::NumberInput,
                        "".into(),
                        Some(HandlerId::from_parts(index + 1, 1).unwrap()),
                    ),
                    Op::SetNumberInput(
                        child,
                        n::Config {
                            auto_focus: false,
                            ..config()
                        },
                        n::Value::Number(1.5),
                    ),
                ]);
                children.push(child);
            }
            ops.push(Op::Splice(root, offset, 0, children));
            apply(cx, handle, ops);
            assert_eq!(
                drain(transport)
                    .iter()
                    .filter(|event| matches!(
                        event,
                        Event::NumberInputEvent(_, _, _, _, n::Event::Observed(_))
                    ))
                    .count(),
                BATCH as usize
            );
        }
        frame(cx, handle).await;
        let mount_time = started.elapsed();
        let (owners, inputs, retained) = handle
            .update(cx, |v, _, _| {
                assert_eq!(v.numbers.len(), OWNERS as usize);
                assert!(v.editors.is_empty(), "no duplicate ordinary editor owners");
                (
                    v.numbers
                        .values()
                        .map(|instance| Rc::downgrade(&instance.owner))
                        .collect::<Vec<_>>(),
                    v.numbers
                        .values()
                        .map(|instance| instance.state.downgrade())
                        .collect::<Vec<_>>(),
                    v.session.borrow().retained_bytes(),
                )
            })
            .unwrap();
        if let Some(previous) = retained_baseline {
            assert_eq!(retained, previous);
        } else {
            retained_baseline = Some(retained);
        }
        handle
            .update(cx, |v, w, cx| {
                w.focus(&v.numbers[&first].focus_handle(cx), cx)
            })
            .unwrap();
        frame(cx, handle).await;
        drain(transport);
        let revision = handle
            .update(cx, |v, _, _| {
                v.session.borrow().tree(v.id).unwrap().revision()
            })
            .unwrap();
        let input_started = Instant::now();
        for _ in 0..16 {
            key(cx, handle, "up");
            key(cx, handle, "down");
        }
        frame(cx, handle).await;
        let input_time = input_started.elapsed();
        let observed = drain(transport);
        assert_eq!(observed.iter().filter(|event| matches!(event,
            Event::NumberInputEvent(_, node, _, _, n::Event::Committed(n::Source::Keyboard, _)) if *node == first)).count(), 32);
        handle
            .update(cx, |v, w, cx| {
                let instance = &v.numbers[&first];
                assert_eq!(
                    instance.owner.borrow().model.snapshot().committed,
                    n::Value::Number(1.5)
                );
                // One burst before yielding. Repeated explicit drafts can fill undo
                // history but must retain only its configured payload budget and the
                // latest adjacent observation in the outgoing lane.
                for index in 0..1024 {
                    let text = if index % 2 == 0 { "1" } else { "2" }.repeat(n::MAX_DRAFT_BYTES);
                    assert!(matches!(
                        instance.command(
                            &n::Command::ReplaceDraft {
                                text,
                                selection: n::SelectionPolicy::End,
                                undo: n::UndoPolicy::Record,
                                if_revision: None,
                            },
                            w,
                            cx
                        ),
                        n::Response::Applied(_)
                    ));
                }
                let history = instance.state.read(cx).bridge_history_bytes();
                assert!(
                    history > 0 && history <= EDITOR_HISTORY_BYTES,
                    "retained history {history}"
                );
                assert!(matches!(
                    instance.command(&n::Command::Cancel, w, cx),
                    n::Response::Applied(_)
                ));
                assert_eq!(instance.owner.borrow().model.snapshot().draft, "1.5");
                assert!(matches!(
                    instance.command(&n::Command::Undo, w, cx),
                    n::Response::Applied(_)
                ));
                assert_eq!(
                    instance.owner.borrow().model.snapshot().draft,
                    "2".repeat(n::MAX_DRAFT_BYTES)
                );
                assert!(matches!(
                    instance.command(
                        &n::Command::ReplaceValue {
                            value: n::Value::Number(1.5),
                            selection: n::SelectionPolicy::End,
                            undo: n::UndoPolicy::Reset,
                            if_revision: None,
                        },
                        w,
                        cx
                    ),
                    n::Response::Applied(_)
                ));
                assert_eq!(instance.state.read(cx).bridge_history_bytes(), 0);
                assert_eq!(v.session.borrow().tree(v.id).unwrap().revision(), revision);
                assert!(
                    v.numbers
                        .values()
                        .all(|instance| !instance.owner.borrow().repeat.has_task())
                );
                w.focus(v.root_focus.as_ref().unwrap(), cx);
            })
            .unwrap();
        let burst = drain(transport);
        assert!(
            matches!(&burst[0], Event::NumberInputEvent(_, _, _, _, n::Event::Changed(s)) if s.draft == "2".repeat(n::MAX_DRAFT_BYTES))
        );
        assert!(matches!(
            &burst[1],
            Event::NumberInputEvent(
                _,
                _,
                _,
                _,
                n::Event::Cancelled(n::CancelReason::Programmatic, _)
            )
        ));
        assert!(
            (4..=5).contains(&burst.len()),
            "unexpected event count: {}",
            burst.len()
        );
        assert!(
            matches!(&burst[2], Event::NumberInputEvent(_, _, _, _, n::Event::Changed(s))
            if s.draft == "2".repeat(n::MAX_DRAFT_BYTES))
        );
        assert!(
            matches!(&burst[3], Event::NumberInputEvent(_, _, _, _, n::Event::Observed(s))
            if s.draft == "1.5")
        );
        // Focus notification may flush before this drain or with the next frame.
        if let Some(event) = burst.get(4) {
            assert!(
                matches!(event, Event::NumberInputEvent(_, _, _, _, n::Event::Changed(s))
                if !s.focused && s.draft == "1.5")
            );
        }
        frame(cx, handle).await;
        drain(transport);
        // Allow the preceding focus/caret update to settle; unfocused owners
        // should then neither redraw nor publish stationary observations.
        cx.background_executor()
            .timer(Duration::from_millis(700))
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
        remove.extend((0..OWNERS).map(|index| Op::Remove(id(index + 1, cycle))));
        remove.push(Op::Remove(root));
        apply(cx, handle, remove);
        frame(cx, handle).await;
        assert!(owners.iter().all(|owner| owner.upgrade().is_none()));
        assert!(inputs.iter().all(|input| input.upgrade().is_none()));
        handle
            .update(cx, |v, _, _| {
                assert!(v.numbers.is_empty());
                assert_eq!(v.session.borrow().retained_bytes(), 0);
            })
            .unwrap();
        drain(transport);
        eprintln!(
            "GPUIO_NUMBER_WORKLOAD cycle={cycle} owners={OWNERS} retained_bytes={retained} mount={mount_time:?} keyboard32_and_frame={input_time:?} elapsed={:?}",
            started.elapsed()
        );
    }
    eprintln!(
        "GPUIO_NUMBER_WORKLOAD_OK: 256 editors, three disposal cycles, keyboard, 1024 maximum-size edits, bounded history/coalescing, idle and zero retained owners"
    );
}
