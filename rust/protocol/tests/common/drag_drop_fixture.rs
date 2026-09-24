use gpuio_protocol::{HandlerId, NodeId, WindowId, drag_drop::*, v1::*};
fn node(slot: i64) -> NodeId {
    NodeId::from_parts(slot, 1).unwrap()
}
fn handler(slot: i64) -> HandlerId {
    HandlerId::from_parts(slot, 1).unwrap()
}
fn payload() -> Payload {
    Payload::text("hi".into()).unwrap()
}
pub fn request() -> Message {
    Message::Apply(Transaction {
        window: WindowId::from_parts(0, 1).unwrap(),
        base: 0,
        revision: 1,
        operations: vec![
            Op::Create(node(0), Kind::DragSource, "".into(), Some(handler(0))),
            Op::SetDragSource(
                node(0),
                Source::new("S".into(), payload(), false, false).unwrap(),
            ),
            Op::Create(node(1), Kind::DropTarget, "".into(), Some(handler(1))),
            Op::SetDropTarget(
                node(1),
                Target::new("T".into(), vec![Format::Text], false).unwrap(),
            ),
            Op::Splice(node(0), 0, 0, vec![node(1)]),
            Op::SetRoot(Some(node(0))),
        ],
    })
}
pub fn events() -> Vec<Event> {
    let window = WindowId::from_parts(0, 1).unwrap();
    let mut source = vec![
        SourcePhase::Started(payload()),
        SourcePhase::DesktopOffered,
        SourcePhase::DesktopUnavailable,
        SourcePhase::Ended(Outcome::InternalDrop),
        SourcePhase::Ended(Outcome::Unconfirmed),
    ];
    source.extend(
        [
            CancelReason::Escape,
            CancelReason::Hidden,
            CancelReason::Blocked,
            CancelReason::Disabled,
            CancelReason::Removed,
            CancelReason::Reconfigured,
            CancelReason::WindowClosed,
            CancelReason::WindowInactive,
        ]
        .into_iter()
        .map(|reason| SourcePhase::Ended(Outcome::Cancelled(reason))),
    );
    let mut events: Vec<_> = source
        .into_iter()
        .map(|phase| {
            Event::DragSourceEvent(
                window,
                node(0),
                handler(0),
                1,
                SourceSample { gesture: 7, phase },
            )
        })
        .collect();
    events.extend(
        [
            TargetPhase::Entered(Offer::new(&payload(), Origin::Internal)),
            TargetPhase::Moved,
            TargetPhase::Left,
            TargetPhase::Dropped(payload()),
            TargetPhase::Rejected(Rejection::InvalidData),
            TargetPhase::Rejected(Rejection::LimitExceeded),
        ]
        .into_iter()
        .map(|phase| {
            Event::DropTargetEvent(
                window,
                node(1),
                handler(1),
                1,
                TargetSample {
                    gesture: 7,
                    phase,
                    window_x: 120.,
                    window_y: 80.,
                    local_x: -5.,
                    local_y: 30.,
                    modifiers: PointerModifiers {
                        shift: true,
                        ..Default::default()
                    },
                },
            )
        }),
    );
    events
}
