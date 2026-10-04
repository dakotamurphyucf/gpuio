use gpuio_native::session::Session;
use gpuio_protocol::{
    HandlerId, NodeId, WindowId,
    animation::Easing,
    progress_presentation::{Config, Shape, Transition},
    v1::*,
};
fn id(slot: i64) -> NodeId {
    NodeId::from_parts(slot, 1).unwrap()
}
#[test]
fn circle_children_and_legacy_reset_validate_atomically_with_bounded_ownership() {
    let window = WindowId::from_parts(0, 1).unwrap();
    let mut session = Session::default();
    session.hello(VERSION, CAPABILITIES).unwrap();
    session.open(1, window, "Progress", 100., 100.).unwrap();
    let config = Config {
        progress: ProgressConfig {
            label: "Working".into(),
            fraction: Some(0.5),
        },
        shape: Shape::Circle,
        transition: Transition::Tween {
            duration_ms: 200,
            easing: Easing::EaseOut,
        },
    };
    let tx = |base, operations| Transaction {
        window,
        base,
        revision: base + 1,
        operations,
    };
    session
        .apply(&tx(
            0,
            vec![
                Op::Create(id(0), Kind::Progress, "".into(), None),
                Op::SetProgressPresentation(id(0), config.clone()),
                Op::Create(
                    id(1),
                    Kind::Button,
                    "Cancel".into(),
                    Some(HandlerId::from_parts(0, 1).unwrap()),
                ),
                Op::Splice(id(0), 0, 0, vec![id(1)]),
                Op::SetRoot(Some(id(0))),
            ],
        ))
        .unwrap();
    let bytes = session.retained_bytes();
    assert!(bytes >= config.retained_bytes() + gpuio_native::progress_clock::RESERVED_BYTES);
    for operations in [
        vec![Op::SetProgress(id(0), config.progress.clone())],
        vec![Op::SetProgressPresentation(
            id(0),
            Config {
                shape: Shape::Linear,
                ..config.clone()
            },
        )],
        vec![Op::SetProgressPresentation(id(1), config.clone())],
        vec![Op::SetProgressPresentation(
            id(0),
            Config {
                transition: Transition::Tween {
                    duration_ms: 0,
                    easing: Easing::Ease,
                },
                ..config.clone()
            },
        )],
        vec![
            Op::SetProgressPresentation(
                id(0),
                Config {
                    progress: ProgressConfig {
                        label: "Changed".into(),
                        fraction: Some(0.9),
                    },
                    ..config.clone()
                },
            ),
            Op::SetText(id(0), "illegal".into()),
        ],
    ] {
        assert!(session.apply(&tx(1, operations)).is_err());
        assert_eq!(session.retained_bytes(), bytes);
        let tree = session.tree(window).unwrap();
        assert_eq!(tree.revision(), 1);
        assert_eq!(
            tree.get(id(0)).unwrap().progress_presentation.as_deref(),
            Some(&config)
        );
        assert_eq!(tree.get(id(0)).unwrap().children.as_ref(), &[id(1)]);
    }
    session
        .apply(&tx(
            1,
            vec![
                Op::SetProgress(id(0), config.progress.clone()),
                Op::Splice(id(0), 0, 1, vec![]),
                Op::Remove(id(1)),
            ],
        ))
        .unwrap();
    let node = session.tree(window).unwrap().get(id(0)).unwrap();
    let presentation = node.progress_presentation.as_ref().unwrap();
    assert_eq!(presentation.shape, Shape::Linear);
    assert_eq!(presentation.transition, Transition::Immediate);
    assert!(node.children.is_empty());
    session.close(window).unwrap();
    assert_eq!(session.retained_bytes(), 0);
}
