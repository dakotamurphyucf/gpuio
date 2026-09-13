use gpuio_native::session::{CommandInvocation, Session};
use gpuio_protocol::{HandlerId, NodeId, WindowId, v1::*};
#[path = "../../protocol/tests/common/palette_fixture.rs"]
mod fixture;
fn node(slot: i64) -> NodeId {
    NodeId::from_parts(slot, 1).unwrap()
}
#[test]
fn palette_references_dismissal_permissions_and_retained_lifetimes_are_atomic() {
    let window = WindowId::from_parts(0, 1).unwrap();
    let mut session = Session::default();
    session.hello(VERSION, CAPABILITIES).unwrap();
    session.open(1, window, "Palette", 400., 300.).unwrap();
    let Message::Apply(transaction) = fixture::request() else {
        panic!("fixture transaction")
    };
    session.apply(&transaction).unwrap();
    let palette_handler = HandlerId::from_parts(1, 1).unwrap();
    assert_eq!(
        session.palette_dismissed(
            window,
            node(1),
            palette_handler,
            1,
            PaletteDismissal::Escape
        ),
        Some(fixture::events()[2].clone())
    );
    assert!(
        session
            .palette_dismissed(
                window,
                node(1),
                palette_handler,
                1,
                PaletteDismissal::OutsidePointer
            )
            .is_none()
    );
    assert!(
        session
            .palette_dismissed(
                window,
                node(1),
                palette_handler,
                1,
                PaletteDismissal::Selected("missing".into())
            )
            .is_none()
    );
    let request = CommandInvocation {
        scope: node(0),
        handler: HandlerId::from_parts(0, 1).unwrap(),
        revision: 1,
        command: "run",
        generation: 1,
        source: CommandSource::Palette(node(1)),
    };
    assert!(session.invoke_command(window, request).is_some());
    let before = session.retained_bytes();
    let mut missing = fixture::config();
    missing.commands.push("missing".into());
    let mut duplicate = fixture::config();
    duplicate.commands.push("run".into());
    for config in [missing, duplicate] {
        assert_eq!(
            session.apply(&Transaction {
                window,
                base: 1,
                revision: 2,
                operations: vec![Op::SetPalette(node(1), config)]
            }),
            Err(ErrorCode::InvalidTree)
        );
        assert_eq!(session.retained_bytes(), before);
        assert_eq!(session.tree(window).unwrap().revision(), 1);
    }
    session
        .apply(&Transaction {
            window,
            base: 1,
            revision: 2,
            operations: vec![Op::Splice(node(0), 0, 1, vec![]), Op::Remove(node(1))],
        })
        .unwrap();
    assert!(session.invoke_command(window, request).is_none());
    assert!(
        session
            .palette_dismissed(
                window,
                node(1),
                palette_handler,
                1,
                PaletteDismissal::Escape
            )
            .is_none()
    );
    assert!(session.retained_bytes() + PALETTE_HISTORY_BYTES < before);
    session.close(window).unwrap();
    assert_eq!(session.retained_bytes(), 0);
}
