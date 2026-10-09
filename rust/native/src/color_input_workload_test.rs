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
                        Kind::ColorInput,
                        "".into(),
                        Some(HandlerId::from_parts(index + 1, 1).unwrap()),
                    ),
                    Op::SetColorInput(
                        child,
                        Box::new(config()),
                        Value::Color(Rgba::new(0, 255, 0, 255)),
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
                        Event::ColorInputEvent(_, _, _, _, c::Event::Observed(_))
                    ))
                    .count(),
                BATCH as usize
            );
        }
        frame(cx, handle).await;
        let (weak, fields, retained) = handle
            .update(cx, |v, _, cx| {
                assert_eq!(v.color_inputs.len(), OWNERS as usize);
                assert!(v.editors.is_empty() && v.numbers.is_empty() && v.otps.is_empty());
                (
                    v.color_inputs
                        .values()
                        .map(|c| c.state.downgrade())
                        .collect::<Vec<_>>(),
                    v.color_inputs
                        .values()
                        .flat_map(|input| {
                            input
                                .state
                                .read(cx)
                                .editors
                                .fields
                                .iter()
                                .map(|field| field.state.downgrade())
                                .collect::<Vec<_>>()
                        })
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
                let focus = v.color_inputs[&first].state.read(cx).channel_focus[0].clone();
                w.focus(&focus, cx);
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
        assert_eq!(output.len(), 16); // ColorInput navigation is discrete, not coalesced.
        assert!(
            output.iter().all(
                |e| matches!(e,Event::ColorInputEvent(_,n,_,_,c::Event::Committed(c::Source::Keyboard, _)) if *n==first)
            )
        );
        for offset in (0..256).step_by(BATCH as usize) {
            handle
                .update(cx, |v, w, cx| {
                    for index in offset..offset + BATCH {
                        let value = Value::Color(Rgba::new(
                            if index % 2 == 0 { 255 } else { 0 },
                            255,
                            0,
                            255,
                        ));
                        assert!(matches!(
                            v.color_inputs[&first].command(
                                &c::Command::Set {
                                    value,
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
                |e| matches!(e,Event::ColorInputEvent(_,n,_,_,c::Event::Observed(_)) if *n==first)
            ));
        }
        handle
            .update(cx, |v, w, cx| {
                let owner = v.color_inputs[&first].state.clone();
                let field = owner.read(cx).editors.fields[0].state.clone();
                w.focus(&field.read(cx).focus_handle(cx), cx);
                for index in 0..256 {
                    let text = if index % 2 == 0 { "1" } else { "2" }.repeat(c::MAX_DRAFT_BYTES);
                    field.update(cx, |input, cx| {
                        let length = input.value().encode_utf16().count();
                        input.replace_text_in_range(Some(0..length), &text, w, cx);
                    });
                    owner.update(cx, |state, cx| state.observe_editor(0, w, cx));
                }
                let history = field.read(cx).bridge_history_bytes();
                assert!(history > 0 && history <= 64 * 1024);
            })
            .unwrap();
        let output = drain(transport);
        assert_eq!(output.len(), 2, "Started plus coalesced latest preview");
        assert!(
            matches!(output.last(), Some(Event::ColorInputEvent(_, n, _, _, c::Event::Preview(s))) if *n == first && s.draft.as_ref().unwrap().text.len() == c::MAX_DRAFT_BYTES)
        );
        handle
            .update(cx, |v, w, cx| {
                assert!(matches!(
                    v.color_inputs[&first].command(&c::Command::Cancel, w, cx),
                    c::Response::Applied(_)
                ));
            })
            .unwrap();
        handle.update(cx, |_, w, cx| w.blur(cx)).unwrap();
        frame(cx, handle).await;
        drain(transport);
        // Blur and native editor observations can schedule follow-up frames.
        // Establish bounded quiescence before measuring idle rather than assuming
        // that a fixed 100ms delay drained a loaded desktop's pending frames.
        let deadline = Instant::now() + Duration::from_secs(2);
        let mut quiet_since = Instant::now();
        let mut renders = handle.update(cx, |v, _, _| v.render_count).unwrap();
        loop {
            cx.background_executor()
                .timer(Duration::from_millis(25))
                .await;
            let current = handle.update(cx, |v, _, _| v.render_count).unwrap();
            if current != renders {
                renders = current;
                quiet_since = Instant::now();
            }
            if quiet_since.elapsed() >= Duration::from_millis(250) {
                break;
            }
            assert!(
                Instant::now() < deadline,
                "color editors did not settle after blur"
            );
        }
        drain(transport);
        cx.background_executor()
            .timer(Duration::from_millis(600))
            .await;
        assert_eq!(
            handle.update(cx, |v, _, _| v.render_count).unwrap(),
            renders,
            "settled color editors redrew during 600ms idle"
        );
        assert!(drain(transport).is_empty());
        let mut remove = vec![Op::SetRoot(None), Op::Splice(root, 0, OWNERS, vec![])];
        remove.extend((1..=OWNERS).map(|slot| Op::Remove(id(slot, generation))));
        remove.push(Op::Remove(root));
        apply(cx, handle, remove);
        frame(cx, handle).await;
        assert!(weak.iter().all(|owner| owner.upgrade().is_none()));
        assert!(fields.iter().all(|field| field.upgrade().is_none()));
        handle
            .update(cx, |v, _, _| {
                assert!(v.color_inputs.is_empty());
                assert_eq!(v.session.borrow().retained_bytes(), 0);
            })
            .unwrap();
        drain(transport);
        eprintln!(
            "GPUIO_COLOR_WORKLOAD cycle={generation} owners={OWNERS} retained_bytes={retained} elapsed={:?}",
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
        "GPUIO_COLOR_WORKLOAD_OK: three 64-owner cycles, discrete native/command events, bounded/coalesced drafts/history, idle and owner/child disposal"
    );
}
