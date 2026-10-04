#[path = "support/split_group.rs"]
mod support;
use gpuio_native::{session::Session, tree::Tree};
use gpuio_protocol::{
    split_group::{Snapshot, Source},
    v1::*,
};
use support::*;
fn tx(base: i64, operations: Vec<Op>) -> Transaction {
    Transaction {
        window: w(),
        base,
        revision: base + 1,
        operations,
    }
}
#[test]
fn slots_passive_grips_bounds_and_nonstructural_edits_are_atomic() {
    let mut tree = Tree::new(w());
    tree.apply(&tx(0, initial())).unwrap();
    let baseline = tree.retained_bytes();
    assert!(baseline > 8192 + 3 * 4096);
    for ops in [
        vec![Op::Bind(n(5), Some(h(4)))],
        vec![Op::Bind(n(3), Some(h(4)))],
        vec![Op::SetStyle(
            n(1),
            vec![Style::Fields(vec![Field::Disabled(true)])],
        )],
        vec![Op::SetStyle(
            n(5),
            vec![Style::Fields(vec![Field::UserSelect(true)])],
        )],
        vec![Op::SetSplitGroup(n(4), config(), Default::default())],
        vec![Op::Splice(n(2), 0, 1, vec![])],
        vec![
            Op::Splice(n(2), 0, 1, vec![n(5)]),
            Op::Splice(n(3), 0, 1, vec![n(4)]),
        ],
    ] {
        assert!(tree.apply(&tx(1, ops)).is_err());
        assert_eq!(tree.revision(), 1);
        assert_eq!(tree.retained_bytes(), baseline);
    }
    let mut c = config();
    c.panels.swap(0, 2);
    tree.apply(&tx(
        1,
        vec![
            Op::SetSplitGroup(n(0), c.clone(), Default::default()),
            Op::Splice(n(0), 0, 3, vec![n(11), n(6), n(1)]),
        ],
    ))
    .unwrap();
    assert_eq!(tree.get(n(4)).unwrap().parent, Some(n(2)));
    c.reset_generation = 2;
    tree.apply(&tx(2, vec![Op::SetSplitGroup(n(0), c, Default::default())]))
        .unwrap();
    assert!(
        tree.apply(&tx(
            3,
            vec![Op::SetSplitGroup(n(0), config(), Default::default())]
        ))
        .is_err()
    );
}
#[test]
fn publication_checks_current_config_handler_revision_snapshot_and_shutdown() {
    let mut session = Session::default();
    session.hello(VERSION, CAPABILITIES).unwrap();
    session.open(1, w(), "Split", 300., 120.).unwrap();
    session.apply(&tx(0, initial())).unwrap();
    let c = config();
    let snapshot = Snapshot {
        source: Source::Keyboard,
        sizes: vec![("p0".into(), 110.), ("p1".into(), 90.), ("p2".into(), 100.)],
    };
    assert!(
        session
            .split_group_resized(w(), n(0), h(0), 1, &c, snapshot.clone())
            .is_some()
    );
    for (handler, revision) in [(h(1), 1), (h(0), -1), (h(0), 2)] {
        assert!(
            session
                .split_group_resized(w(), n(0), handler, revision, &c, snapshot.clone())
                .is_none()
        );
    }
    let mut invalid = snapshot.clone();
    invalid.sizes.swap(0, 1);
    assert!(
        session
            .split_group_resized(w(), n(0), h(0), 1, &c, invalid)
            .is_none()
    );
    let mut updated = c.clone();
    updated.panels[0].visible = false;
    session
        .apply(&tx(
            1,
            vec![Op::SetSplitGroup(n(0), updated.clone(), Default::default())],
        ))
        .unwrap();
    assert!(
        session
            .split_group_resized(w(), n(0), h(0), 1, &c, snapshot.clone())
            .is_none()
    );
    assert!(
        session
            .split_group_resized(w(), n(0), h(0), 2, &updated, snapshot.clone())
            .is_some()
    );
    session.overload(w());
    assert!(
        session
            .split_group_resized(w(), n(0), h(0), 2, &updated, snapshot)
            .is_none()
    );
}

#[test]
fn public_core_transactions_mount_reorder_hide_reset_request_and_dispose() {
    let mut tree = Tree::new(w());
    for (i, line) in include_str!("../../../test/fixtures/split-group-public.hex")
        .lines()
        .enumerate()
    {
        let bytes: Vec<_> = (0..line.len())
            .step_by(2)
            .map(|i| u8::from_str_radix(&line[i..i + 2], 16).unwrap())
            .collect();
        let Message::Apply(tx) = gpuio_protocol::decode(&bytes).unwrap() else {
            panic!("expected transaction")
        };
        tree.apply(&tx).unwrap();
        if i < 5 {
            assert_eq!(tree.get(n(3)).unwrap().text.as_ref(), "a");
            assert_eq!(tree.get(n(3)).unwrap().parent, Some(n(2)));
            let expected = if i == 0 {
                [n(1), n(6), n(10)]
            } else {
                [n(10), n(1), n(6)]
            };
            assert_eq!(
                tree.get(n(0)).unwrap().children.as_ref(),
                expected.as_slice()
            );
        }
    }
    assert_eq!(tree.retained_bytes(), 0);
}
