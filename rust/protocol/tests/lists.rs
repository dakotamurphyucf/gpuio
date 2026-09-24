use binprot::BinProtWrite;
use gpuio_protocol::{DecodeError, NodeId, WindowId, decode, list::*, v1::*};

fn node(slot: i64) -> NodeId {
    NodeId::from_parts(slot, 1).unwrap()
}
fn config() -> Config {
    Config {
        estimated_height: 100.,
        overscan: 200.,
        max_active: 32,
        scroll_policy: ScrollPolicy::FollowTailWhenAtEnd,
        scrollbar: true,
        managed: true,
    }
}
fn message(operations: Vec<Op>) -> Message {
    Message::Apply(Transaction {
        window: WindowId::from_parts(0, 1).unwrap(),
        base: 0,
        revision: 1,
        operations,
    })
}
fn encode(value: &impl BinProtWrite) -> Vec<u8> {
    let mut bytes = Vec::new();
    value.binprot_write(&mut bytes).unwrap();
    bytes
}

#[test]
fn independent_ocaml_list_transaction_matches_and_decodes() {
    let request = message(vec![
        Op::Create(node(0), Kind::VirtualList, "".into(), None),
        Op::Create(node(1), Kind::Text, "row".into(), None),
        Op::SetListConfig(node(0), config()),
        Op::SetListOrder(
            node(0),
            Order {
                revision: 1,
                runs: vec![IdRun {
                    first: 1,
                    count: 100_000,
                }],
            },
        ),
        Op::SetListRows(
            node(0),
            vec![Row {
                id: 1,
                node: node(1),
            }],
        ),
        Op::Splice(node(0), 0, 0, vec![node(1)]),
        Op::SetRoot(Some(node(0))),
        Op::InvalidateListRows(node(0), vec![1, 3]),
        Op::ScrollList(
            node(0),
            ScrollRequest {
                serial: 17,
                target: ScrollTarget::Offset(3, 37.5),
            },
        ),
    ]);
    let bytes = encode(&request);
    let hex: String = bytes.iter().map(|byte| format!("{byte:02x}")).collect();
    assert_eq!(
        hex,
        "0300010001090000011900000001010103726f77001c000100000000000059400000000000006940200101011d0001010101fda08601001e0001010101010500010000010101060100011f00010201032000011100030000000000c04240"
    );
    assert_eq!(decode(&bytes).unwrap(), request);
    let mut trailing = bytes.clone();
    trailing.push(0);
    assert_eq!(decode(&trailing), Err(DecodeError::Malformed));
    assert_eq!(
        decode(&bytes[..bytes.len() - 1]),
        Err(DecodeError::Malformed)
    );
}

#[test]
fn rejects_invalid_geometry_and_counts_without_expanding_logical_rows() {
    let mut bad = config();
    bad.estimated_height = 0.;
    for op in [
        Op::SetListConfig(node(0), bad),
        Op::SetListOrder(
            node(0),
            Order {
                revision: 1,
                runs: vec![IdRun {
                    first: 1,
                    count: i64::MAX,
                }],
            },
        ),
        Op::SetListOrder(
            node(0),
            Order {
                revision: 1,
                runs: vec![IdRun {
                    first: i64::MAX,
                    count: 2,
                }],
            },
        ),
        Op::SetListOrder(
            node(0),
            Order {
                revision: 1,
                runs: vec![IdRun { first: 1, count: 3 }, IdRun { first: 3, count: 2 }],
            },
        ),
        Op::ScrollList(
            node(0),
            ScrollRequest {
                serial: 1,
                target: ScrollTarget::Offset(1, -1.),
            },
        ),
        Op::ScrollList(
            node(0),
            ScrollRequest {
                serial: 0,
                target: ScrollTarget::End,
            },
        ),
    ] {
        assert_eq!(
            decode(&encode(&message(vec![op]))),
            Err(DecodeError::Malformed)
        );
    }
    let huge = message(vec![Op::SetListOrder(
        node(0),
        Order {
            revision: 1,
            runs: vec![IdRun { first: 1, count: 1 }; MAX_ID_RUNS + 1],
        },
    )]);
    assert_eq!(decode(&encode(&huge)), Err(DecodeError::LimitExceeded));
}

#[test]
fn independent_ocaml_viewport_event_matches() {
    let events = vec![Event::ListViewport(
        WindowId::from_parts(0, 1).unwrap(),
        node(0),
        gpuio_protocol::HandlerId::from_parts(0, 1).unwrap(),
        1,
        Viewport {
            order_revision: 1,
            visible_first: 0,
            visible_last: 3,
            requested: vec![1, 2, 3],
            pinned: vec![1],
            anchor: Some((1, 4.)),
            following_tail: false,
            at_start: false,
            at_end: false,
            budget_exhausted: false,
        },
    )];
    let bytes = encode(&events);
    let hex: String = bytes.iter().map(|byte| format!("{byte:02x}")).collect();
    assert_eq!(
        hex,
        "011b000100010001010100030301020301010101000000000000104000000000"
    );
}
