use gpuio_native::session::Session;
use gpuio_protocol::{HandlerId, NodeId, WindowId, v1::*};
#[test]
fn motion_only_targets_tooltips_and_teardown_releases_reservation() {
    let id = |n| NodeId::from_parts(n, 1).unwrap();
    let window = WindowId::from_parts(0, 1).unwrap();
    let mut session = Session::default();
    session.hello(VERSION, CAPABILITIES).unwrap();
    session.open(1, window, "Help", 600., 500.).unwrap();
    let tx = |base, operations| Transaction {
        window,
        base,
        revision: base + 1,
        operations,
    };
    let config = TooltipConfig {
        label: "Help".into(),
        width: 200.,
        open_state: TooltipOpenState::Managed(false),
        disabled: false,
        hoverable: true,
        show_delay_ns: 0,
        hide_delay_ns: 0,
        skip_delay_ns: 0,
    };
    session
        .apply(&tx(
            0,
            vec![
                Op::Create(id(0), Kind::Container, "".into(), None),
                Op::Create(
                    id(1),
                    Kind::Tooltip,
                    "".into(),
                    Some(HandlerId::from_parts(1, 1).unwrap()),
                ),
                Op::SetTooltip(id(1), config.clone()),
                Op::SetTooltipMotion(id(1), true),
                Op::Create(id(2), Kind::Text, "Anchor".into(), None),
                Op::Create(id(3), Kind::Text, "Help".into(), None),
                Op::Splice(id(1), 0, 0, vec![id(2), id(3)]),
                Op::Splice(id(0), 0, 0, vec![id(1)]),
                Op::SetRoot(Some(id(0))),
            ],
        ))
        .unwrap();
    let bytes = session.retained_bytes();
    assert_eq!(
        session.apply(&tx(1, vec![Op::SetTooltipMotion(id(0), true)])),
        Err(ErrorCode::InvalidTree)
    );
    assert_eq!(session.retained_bytes(), bytes);
    assert_eq!(session.tree(window).unwrap().revision(), 1);
    session
        .apply(&tx(1, vec![Op::SetTooltipMotion(id(1), false)]))
        .unwrap();
    assert_eq!(
        session.retained_bytes(),
        bytes,
        "stable frame reservation survives presentation updates"
    );
    assert!(
        !session
            .tree(window)
            .unwrap()
            .get(id(1))
            .unwrap()
            .tooltip_motion
    );
    let mut ops = vec![Op::SetRoot(None)];
    for n in [2, 3, 1, 0] {
        ops.push(Op::Remove(id(n)));
    }
    session.apply(&tx(2, ops)).unwrap();
    assert_eq!(session.retained_bytes(), 0);
    // Hover cards are deliberately excluded: the pinned render path is unanimated.
    session
        .apply(&tx(
            3,
            vec![
                Op::Create(
                    id(4),
                    Kind::HoverCard,
                    "".into(),
                    Some(HandlerId::from_parts(4, 1).unwrap()),
                ),
                Op::SetTooltip(id(4), config),
                Op::Create(id(5), Kind::Text, "Anchor".into(), None),
                Op::Create(id(6), Kind::Text, "Content".into(), None),
                Op::Splice(id(4), 0, 0, vec![id(5), id(6)]),
                Op::SetRoot(Some(id(4))),
            ],
        ))
        .unwrap();
    assert_eq!(
        session.apply(&tx(4, vec![Op::SetTooltipMotion(id(4), true)])),
        Err(ErrorCode::InvalidTree)
    );
}
