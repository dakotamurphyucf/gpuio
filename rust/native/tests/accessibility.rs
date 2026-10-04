use gpuio_native::session::Session;
use gpuio_protocol::{
    NodeId, WindowId,
    accessibility::{Config, Field, Live, Role},
    v1::*,
};

#[test]
fn transcript_metadata_replacement_rollback_and_retirement() {
    let w = WindowId::from_parts(0, 1).unwrap();
    let root = NodeId::from_parts(0, 1).unwrap();
    let child = NodeId::from_parts(1, 1).unwrap();
    let mut session = Session::default();
    session.hello(VERSION, CAPABILITIES).unwrap();
    session.open(1, w, "transcript", 320., 200.).unwrap();
    let tx = |base, operations| Transaction {
        window: w,
        base,
        revision: base + 1,
        operations,
    };
    let quiet = Config {
        role: Some(Role::Log),
        label: Some("Conversation".into()),
        description: None,
        live: Live::Off,
        field: None,
        current: None,
    };
    session
        .apply(&tx(
            0,
            vec![
                Op::Create(root, Kind::Container, "".into(), None),
                Op::Create(child, Kind::Button, "Copy response".into(), None),
                Op::Splice(root, 0, 0, vec![child]),
                Op::SetAccessibility(root, Some(quiet.clone())),
                Op::SetRoot(Some(root)),
            ],
        ))
        .unwrap();
    let bytes = session.retained_bytes();
    let polite = Config {
        live: Live::Polite,
        ..quiet.clone()
    };
    assert!(
        session
            .apply(&tx(
                1,
                vec![
                    Op::SetAccessibility(root, Some(polite.clone())),
                    Op::SetAccessibility(child, Some(polite.clone())),
                ]
            ))
            .is_err()
    );
    assert_eq!(session.tree(w).unwrap().revision(), 1);
    assert_eq!(
        session
            .tree(w)
            .unwrap()
            .get(root)
            .unwrap()
            .accessibility
            .as_deref(),
        Some(&quiet)
    );
    assert_eq!(session.retained_bytes(), bytes);
    session
        .apply(&tx(
            1,
            vec![Op::SetAccessibility(root, Some(polite.clone()))],
        ))
        .unwrap();
    let tree = session.tree(w).unwrap();
    assert_eq!(
        tree.get(root).unwrap().accessibility.as_deref(),
        Some(&polite)
    );
    assert_eq!(tree.get(root).unwrap().children.as_ref(), &[child]);
    assert!(tree.get(child).unwrap().accessibility.is_none());
    session
        .apply(&tx(2, vec![Op::SetAccessibility(root, None)]))
        .unwrap();
    assert!(session.retained_bytes() < bytes);
    session.close(w).unwrap();
    assert_eq!(session.retained_bytes(), 0);
}

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
        current: None,
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

#[test]
fn toolbar_orientation_replacement_and_invalid_child_metadata_are_atomic() {
    use gpuio_protocol::accessibility::Orientation;
    let w = WindowId::from_parts(0, 1).unwrap();
    let root = NodeId::from_parts(0, 1).unwrap();
    let button = NodeId::from_parts(1, 1).unwrap();
    let mut session = Session::default();
    session.hello(VERSION, CAPABILITIES).unwrap();
    session.open(1, w, "toolbar", 320., 200.).unwrap();
    let tx = |base, operations| Transaction {
        window: w,
        base,
        revision: base + 1,
        operations,
    };
    let metadata = |orientation| Config {
        role: Some(Role::Toolbar(orientation)),
        label: Some("Formatting".into()),
        description: None,
        live: Live::Off,
        current: None,
        field: None,
    };
    let horizontal = metadata(Orientation::Horizontal);
    let vertical = metadata(Orientation::Vertical);
    session
        .apply(&tx(
            0,
            vec![
                Op::Create(root, Kind::Container, "".into(), None),
                Op::Create(button, Kind::Button, "Bold".into(), None),
                Op::Splice(root, 0, 0, vec![button]),
                Op::SetAccessibility(root, Some(horizontal.clone())),
                Op::SetRoot(Some(root)),
            ],
        ))
        .unwrap();
    let before = session.retained_bytes();
    assert!(
        session
            .apply(&tx(
                1,
                vec![
                    Op::SetAccessibility(root, Some(vertical.clone())),
                    Op::SetAccessibility(button, Some(vertical.clone())),
                ]
            ))
            .is_err()
    );
    assert_eq!(session.tree(w).unwrap().revision(), 1);
    assert_eq!(
        session
            .tree(w)
            .unwrap()
            .get(root)
            .unwrap()
            .accessibility
            .as_deref(),
        Some(&horizontal)
    );
    assert_eq!(session.retained_bytes(), before);
    session
        .apply(&tx(
            1,
            vec![Op::SetAccessibility(root, Some(vertical.clone()))],
        ))
        .unwrap();
    assert_eq!(
        session
            .tree(w)
            .unwrap()
            .get(root)
            .unwrap()
            .accessibility
            .as_deref(),
        Some(&vertical)
    );
    assert_eq!(
        session
            .tree(w)
            .unwrap()
            .get(root)
            .unwrap()
            .children
            .as_ref(),
        &[button]
    );
    session
        .apply(&tx(2, vec![Op::SetAccessibility(root, None)]))
        .unwrap();
    assert!(session.retained_bytes() < before);
    session
        .apply(&tx(
            3,
            vec![
                Op::SetRoot(None),
                Op::Splice(root, 0, 1, vec![]),
                Op::Remove(button),
                Op::Remove(root),
            ],
        ))
        .unwrap();
    assert_eq!(session.retained_bytes(), 0);
}

#[test]
fn structural_table_semantics_reject_invalid_spans_and_control_ownership_atomically() {
    use gpuio_protocol::accessibility::{TableCell, TableInfo};
    let w = WindowId::from_parts(0, 1).unwrap();
    let root = NodeId::from_parts(0, 1).unwrap();
    let button = NodeId::from_parts(1, 1).unwrap();
    let mut session = Session::default();
    session.hello(VERSION, CAPABILITIES).unwrap();
    session.open(1, w, "Table", 300., 200.).unwrap();
    let tx = |base, operations| Transaction {
        window: w,
        base,
        revision: base + 1,
        operations,
    };
    let config = Config {
        role: Some(Role::Table(TableInfo {
            rows: Some(0),
            columns: Some(3),
        })),
        label: Some("Summary".into()),
        description: None,
        live: Live::Off,
        field: None,
        current: None,
    };
    session
        .apply(&tx(
            0,
            vec![
                Op::Create(root, Kind::Container, String::new(), None),
                Op::Create(button, Kind::Button, "Action".into(), None),
                Op::Splice(root, 0, 0, vec![button]),
                Op::SetAccessibility(root, Some(config.clone())),
                Op::SetRoot(Some(root)),
            ],
        ))
        .unwrap();
    let bytes = session.retained_bytes();
    for (owner, role) in [
        (button, Role::TableRow(0)),
        (
            root,
            Role::TableCell(TableCell {
                row: 0,
                column: 1023,
                column_span: 2,
            }),
        ),
    ] {
        assert!(
            session
                .apply(&tx(
                    1,
                    vec![
                        Op::SetAccessibility(root, None),
                        Op::SetAccessibility(
                            owner,
                            Some(Config {
                                role: Some(role),
                                ..config.clone()
                            })
                        )
                    ]
                ))
                .is_err()
        );
        assert_eq!(session.tree(w).unwrap().revision(), 1);
        assert_eq!(
            session
                .tree(w)
                .unwrap()
                .get(root)
                .unwrap()
                .accessibility
                .as_deref(),
            Some(&config)
        );
        assert_eq!(session.retained_bytes(), bytes);
    }
    session.close(w).unwrap();
    assert_eq!(session.retained_bytes(), 0);
}
