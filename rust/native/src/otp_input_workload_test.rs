//! Retained ownership, bounded editor history/event traffic and disposal.
use super::*;
use std::time::{Duration, Instant};

const OWNERS: i64 = 256;
const BATCH: i64 = 32;
fn id(slot: i64, generation: i64) -> NodeId {
    NodeId::from_parts(slot, generation).unwrap()
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
    let mut baseline = None;
    for cycle in 1..=3 {
        let started = Instant::now();
        let root = id(0, cycle);
        let first = id(1, cycle);
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
                let child = id(index + 1, cycle);
                operations.extend([
                    Op::Create(
                        child,
                        Kind::OtpInput,
                        "".into(),
                        Some(HandlerId::from_parts(index + 1, 1).unwrap()),
                    ),
                    Op::SetOtpInput(
                        child,
                        o::Config {
                            policy: o::Policy::new(32, o::Alphabet::AsciiAlphanumeric).unwrap(),
                            auto_focus: false,
                            ..config()
                        },
                        "a".repeat(32),
                    ),
                ]);
                children.push(child);
            }
            operations.push(Op::Splice(root, offset, 0, children));
            apply(cx, handle, operations);
            assert_eq!(
                drain(transport)
                    .iter()
                    .filter(|event| matches!(
                        event,
                        Event::OtpInputEvent(_, _, _, _, o::Event::Observed(_))
                    ))
                    .count(),
                BATCH as usize
            );
        }
        frame(cx, handle).await;
        let mount_time = started.elapsed();
        let (owners, retained) = handle
            .update(cx, |v, _, _| {
                assert_eq!(v.otps.len(), OWNERS as usize);
                assert!(v.editors.is_empty() && v.numbers.is_empty());
                (
                    v.otps
                        .values()
                        .map(|input| input.state.downgrade())
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
            .update(cx, |v, w, cx| w.focus(&v.otps[&first].focus_handle(cx), cx))
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
            key(cx, handle, "left");
            key(cx, handle, "right");
        }
        frame(cx, handle).await;
        let input_time = input_started.elapsed();
        let changed = drain(transport);
        assert!(
            matches!(changed.as_slice(),[Event::OtpInputEvent(_,node,_,_,o::Event::Changed(s))]
            if *node==first && s.selection.head==32 && s.value=="a".repeat(32))
        );
        // Explicit command observations are barriers, so drain between bounded batches.
        for offset in (0..1024).step_by(BATCH as usize) {
            handle
                .update(cx, |v, w, cx| {
                    for index in offset..offset + BATCH {
                        assert!(matches!(
                            v.otps[&first].command(
                                &o::Command::Replace {
                                    value: if index % 2 == 0 { "b" } else { "c" }.repeat(32),
                                    selection: o::SelectionPolicy::End,
                                    undo: o::UndoPolicy::Record,
                                    if_revision: None,
                                },
                                w,
                                cx
                            ),
                            o::Response::Applied(_)
                        ));
                    }
                })
                .unwrap();
            assert_eq!(
                drain(transport)
                    .iter()
                    .filter(|event| matches!(
                        event,
                        Event::OtpInputEvent(_, _, _, _, o::Event::Observed(_))
                    ))
                    .count(),
                BATCH as usize
            );
        }
        handle
            .update(cx, |v, w, cx| {
                let editor = v.otps[&first].state.read(cx).model.editor();
                assert_eq!(editor.history_edits(), edit::MAX_HISTORY_EDITS);
                assert_eq!(editor.history_bytes(), 8192);
                for operation in [o::Command::Undo, o::Command::Redo] {
                    assert!(matches!(
                        v.otps[&first].command(&operation, w, cx),
                        o::Response::Applied(_)
                    ));
                    let editor = v.otps[&first].state.read(cx).model.editor();
                    assert_eq!(editor.history_edits(), edit::MAX_HISTORY_EDITS);
                    assert_eq!(editor.history_bytes(), 8192);
                }
                assert!(matches!(
                    v.otps[&first].command(
                        &o::Command::Clear {
                            undo: o::UndoPolicy::Reset,
                            if_revision: None
                        },
                        w,
                        cx
                    ),
                    o::Response::Applied(_)
                ));
                assert_eq!(
                    v.otps[&first].state.read(cx).model.editor().history_bytes(),
                    0
                );
                assert_eq!(v.session.borrow().tree(v.id).unwrap().revision(), revision);
                w.blur(cx);
            })
            .unwrap();
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
        remove.extend((1..=OWNERS).map(|index| Op::Remove(id(index, cycle))));
        remove.push(Op::Remove(root));
        apply(cx, handle, remove);
        frame(cx, handle).await;
        assert!(owners.iter().all(|owner| owner.upgrade().is_none()));
        handle
            .update(cx, |v, _, _| {
                assert!(v.otps.is_empty());
                assert_eq!(v.session.borrow().retained_bytes(), 0);
            })
            .unwrap();
        drain(transport);
        eprintln!(
            "GPUIO_OTP_WORKLOAD cycle={cycle} owners={OWNERS} retained_bytes={retained} mount={mount_time:?} keyboard32_and_frame={input_time:?} elapsed={:?}",
            started.elapsed()
        );
    }
    handle
        .update(cx, |v, w, _| {
            v.session.borrow_mut().close(v.id).unwrap();
            w.remove_window();
        })
        .unwrap();
    eprintln!(
        "GPUIO_OTP_WORKLOAD_OK: three 256-owner cycles, 1024 full-code edits, history bounds, coalesced native selection, discrete command observations, idle and complete disposal"
    );
}
