use gpuio_native::session::Session;
use gpuio_protocol::{NodeId, WindowId, v1::*};

#[test]
fn focus_scope_is_structural_and_rejects_incompatible_or_missing_configuration() {
    let window = WindowId::from_parts(0, 1).unwrap();
    let root = NodeId::from_parts(0, 1).unwrap();
    let child = NodeId::from_parts(1, 1).unwrap();
    let scope = FocusScopeConfig {
        trap: true,
        auto_focus: false,
        restore_focus: true,
    };
    let mut session = Session::default();
    session.hello(VERSION, CAPABILITIES).unwrap();
    session.open(1, window, "scope", 100., 100.).unwrap();
    let tx = |base, operations| Transaction {
        window,
        base,
        revision: base + 1,
        operations,
    };
    assert_eq!(
        session.apply(&tx(
            0,
            vec![
                Op::Create(root, Kind::FocusScope, "".into(), None),
                Op::SetRoot(Some(root))
            ]
        )),
        Err(ErrorCode::InvalidTree)
    );
    assert_eq!(session.retained_bytes(), 0);
    session
        .apply(&tx(
            0,
            vec![
                Op::Create(root, Kind::FocusScope, "".into(), None),
                Op::SetFocusScope(root, scope),
                Op::Create(child, Kind::Text, "child".into(), None),
                Op::Splice(root, 0, 0, vec![child]),
                Op::SetRoot(Some(root)),
            ],
        ))
        .unwrap();
    let bytes = session.retained_bytes();
    assert_eq!(
        session.apply(&tx(1, vec![Op::SetFocusScope(child, scope)])),
        Err(ErrorCode::InvalidTree)
    );
    assert_eq!(session.retained_bytes(), bytes);
    assert_eq!(
        session
            .tree(window)
            .unwrap()
            .get(root)
            .unwrap()
            .children
            .as_ref(),
        &[child]
    );
    session
        .apply(&tx(
            1,
            vec![Op::Remove(child), Op::Remove(root), Op::SetRoot(None)],
        ))
        .unwrap();
    assert_eq!(session.retained_bytes(), 0);
}
