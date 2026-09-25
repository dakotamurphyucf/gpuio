use gpuio_native::session::Session;
use gpuio_protocol::{
    NodeId, WindowId,
    accessibility::{Config, Field, Live, Role},
    v1::*,
};

#[test]
fn semantic_admission_is_atomic_and_payload_is_reclaimed() {
    let w = WindowId::from_parts(0, 1).unwrap();
    let n = NodeId::from_parts(0, 1).unwrap();
    let mut session = Session::default();
    session.hello(VERSION, CAPABILITIES).unwrap();
    session.open(1, w, "semantics", 320., 200.).unwrap();
    let tx = |base, operations| Transaction {
        window: w,
        base,
        revision: base + 1,
        operations,
    };
    let heading = Config {
        role: Some(Role::Heading(2)),
        label: Some("Settings".into()),
        description: None,
        live: Live::Off,
        field: None,
    };
    session
        .apply(&tx(
            0,
            vec![
                Op::Create(n, Kind::Text, "Settings".into(), None),
                Op::SetAccessibility(n, Some(heading.clone())),
                Op::SetRoot(Some(n)),
            ],
        ))
        .unwrap();
    let bytes = session.retained_bytes();
    let field = Config {
        role: None,
        label: None,
        field: Some(Field {
            label: "Name".into(),
            help: None,
            error: None,
            required: true,
        }),
        ..heading.clone()
    };
    for bad in [
        field,
        Config {
            role: Some(Role::Link),
            ..heading.clone()
        },
        Config {
            label: Some("bad\0label".into()),
            ..heading.clone()
        },
    ] {
        assert!(
            session
                .apply(&tx(
                    1,
                    vec![
                        Op::SetText(n, "must rollback".into()),
                        Op::SetAccessibility(n, Some(bad))
                    ]
                ))
                .is_err()
        );
        let tree = session.tree(w).unwrap();
        assert_eq!(tree.revision(), 1);
        assert_eq!(tree.get(n).unwrap().text.as_ref(), "Settings");
        assert_eq!(
            tree.get(n).unwrap().accessibility.as_deref(),
            Some(&heading)
        );
        assert_eq!(session.retained_bytes(), bytes);
    }
    session
        .apply(&tx(1, vec![Op::SetAccessibility(n, None)]))
        .unwrap();
    assert!(session.retained_bytes() < bytes);
    assert!(
        session
            .tree(w)
            .unwrap()
            .get(n)
            .unwrap()
            .accessibility
            .is_none()
    );
    session
        .apply(&tx(2, vec![Op::SetRoot(None), Op::Remove(n)]))
        .unwrap();
    assert_eq!(session.retained_bytes(), 0);
}
