use gpuio_native::session::Session;
use gpuio_protocol::{HandlerId, NodeId, WindowId, v1::*};
#[path = "../../protocol/tests/common/progress_fixture.rs"]
mod fixture;
#[test]
fn progress_updates_validate_atomically_and_release_metadata() {
    let window = WindowId::from_parts(0, 1).unwrap();
    let node = NodeId::from_parts(0, 1).unwrap();
    let mut session = Session::default();
    session.hello(VERSION, CAPABILITIES).unwrap();
    session.open(1, window, "Progress", 400., 300.).unwrap();
    let Message::Apply(initial) = fixture::request() else {
        panic!()
    };
    session.apply(&initial).unwrap();
    let before = session.retained_bytes();
    for fraction in [f64::NAN, f64::INFINITY, -0.1, 1.1] {
        assert_eq!(
            session.apply(&Transaction {
                window,
                base: 1,
                revision: 2,
                operations: vec![Op::SetProgress(
                    node,
                    ProgressConfig {
                        label: "Download".into(),
                        fraction: Some(fraction)
                    }
                )],
            }),
            Err(ErrorCode::InvalidTree)
        );
        assert_eq!(session.retained_bytes(), before);
        assert_eq!(session.tree(window).unwrap().revision(), 1);
    }
    assert_eq!(
        session.apply(&Transaction {
            window,
            base: 1,
            revision: 2,
            operations: vec![Op::Bind(node, Some(HandlerId::from_parts(0, 1).unwrap()))],
        }),
        Err(ErrorCode::InvalidTree)
    );
    for (index, fraction) in [Some(0.), Some(1.), None].into_iter().enumerate() {
        session
            .apply(&Transaction {
                window,
                base: index as i64 + 1,
                revision: index as i64 + 2,
                operations: vec![Op::SetProgress(
                    node,
                    ProgressConfig {
                        label: "Download".into(),
                        fraction,
                    },
                )],
            })
            .unwrap();
        assert_eq!(
            session
                .tree(window)
                .unwrap()
                .get(node)
                .unwrap()
                .progress
                .as_ref()
                .unwrap()
                .fraction,
            fraction
        );
    }
    session
        .apply(&Transaction {
            window,
            base: 4,
            revision: 5,
            operations: vec![Op::SetRoot(None), Op::Remove(node)],
        })
        .unwrap();
    assert_eq!(session.retained_bytes(), 0);
}
