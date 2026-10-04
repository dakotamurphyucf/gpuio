use binprot::BinProtWrite;
use gpuio_protocol::{
    decode_table_header_target,
    table::*,
    table_header::{MAX_HEADERS, MAX_TARGET_BYTES, Target as Header},
};

fn bytes(value: &Header) -> Vec<u8> {
    let mut result = Vec::new();
    value.binprot_write(&mut result).unwrap();
    result
}
fn group(level: i64, ids: &[&str]) -> Header {
    Header::Group {
        level,
        columns: ids.iter().map(|id| (*id).into()).collect(),
    }
}
fn column(id: &str) -> Column {
    Column {
        id: id.into(),
        label: "Repeated label".into(),
        width: 100.,
        min_width: 40.,
        max_width: 200.,
        pin: Pin::Unpinned,
        alignment: Alignment::Left,
        resizable: true,
        movable: true,
        sortable: true,
    }
}

#[test]
fn custom_header_targets_match_independent_bytes_and_bounded_adversarial_inputs() {
    let fixtures = include_str!("../../../test/fixtures/table-header-targets.hex");
    for (target, fixture) in [Header::Column("a".into()), group(0, &["a", "β"])]
        .into_iter()
        .zip(fixtures.lines())
    {
        let expected = fixture
            .as_bytes()
            .chunks_exact(2)
            .map(|s| u8::from_str_radix(std::str::from_utf8(s).unwrap(), 16).unwrap())
            .collect::<Vec<_>>();
        assert_eq!(bytes(&target), expected);
        assert_eq!(decode_table_header_target(&expected), Ok(target));
        for end in 0..expected.len() {
            assert!(decode_table_header_target(&expected[..end]).is_err());
        }
        let mut trailing = expected;
        trailing.push(0);
        assert!(decode_table_header_target(&trailing).is_err());
    }
    for invalid in [
        Header::Column("".into()),
        Header::Column("a\0b".into()),
        group(-1, &["a"]),
        group(4, &["a"]),
        group(0, &[]),
        group(0, &["a", "a"]),
        group(0, &["β", "a"]),
    ] {
        assert!(!invalid.is_valid());
        assert!(decode_table_header_target(&bytes(&invalid)).is_err());
    }
    assert!(decode_table_header_target(&[2]).is_err());
    assert!(decode_table_header_target(&[0, 1, 0xff]).is_err());
    assert!(decode_table_header_target(&vec![0; MAX_TARGET_BYTES + 1]).is_err());
    let maximum = Header::Group {
        level: 3,
        columns: (0..64)
            .map(|i| format!("{i:02}{}", "x".repeat(254)))
            .collect(),
    };
    assert!(bytes(&maximum).len() <= MAX_TARGET_BYTES);
    assert_eq!(
        decode_table_header_target(&bytes(&maximum)),
        Ok(maximum.clone())
    );
    assert!(maximum.retained_bytes() > 64 * 256);
    let Header::Group { level, mut columns } = maximum else {
        unreachable!()
    };
    columns.push("z".into());
    assert!(decode_table_header_target(&bytes(&Header::Group { level, columns })).is_err());
    assert_eq!(MAX_HEADERS, 320);
}

#[test]
fn group_targets_follow_exact_membership_not_labels_or_display_order() {
    let g = |ids: &[&str]| Group {
        label: "Repeated label".into(),
        columns: ids.iter().map(|s| (*s).into()).collect(),
    };
    let schema = Schema {
        columns: ["a", "β", "c"].map(column).into(),
        headers: vec![
            vec![g(&["a", "β"]), g(&["c"])],
            vec![g(&["a"]), g(&["β"]), g(&["c"])],
        ],
    };
    assert!(schema.is_valid());
    let moved = schema.moved("β", Some("a")).unwrap();
    let target = group(0, &["a", "β"]);
    assert!(target.matches(&schema) && target.matches(&moved));
    let mut renamed = moved;
    renamed.headers[0][0].label = "Renamed".into();
    assert!(target.matches(&renamed));
    assert!(!group(0, &["a"]).matches(&schema));
    assert!(group(1, &["a"]).matches(&schema));
    assert!(!group(2, &["a"]).matches(&schema));
    assert!(!group(0, &["a", "c"]).matches(&schema));
    assert!(!Header::Column("missing".into()).matches(&schema));
}

#[test]
fn header_operations_match_paired_fixtures_and_checked_transaction_decoder() {
    use gpuio_protocol::{NodeId, WindowId, v1::*};
    let node = NodeId::from_parts(0, 1).unwrap();
    for (target, fixture) in [
        (
            Some(group(0, &["a", "β"])),
            include_str!("../../../test/fixtures/table-header-operation.hex"),
        ),
        (
            None,
            include_str!("../../../test/fixtures/table-header-clear.hex"),
        ),
    ] {
        let operation = Op::SetTableHeader(node, target);
        let mut bytes = Vec::new();
        operation.binprot_write(&mut bytes).unwrap();
        assert_eq!(
            bytes
                .iter()
                .map(|byte| format!("{byte:02x}"))
                .collect::<String>(),
            fixture.trim()
        );
        let message = Message::Apply(Transaction {
            window: WindowId::from_parts(0, 1).unwrap(),
            base: 0,
            revision: 1,
            operations: vec![operation],
        });
        bytes.clear();
        message.binprot_write(&mut bytes).unwrap();
        assert_eq!(gpuio_protocol::decode(&bytes), Ok(message));
        for end in 0..bytes.len() {
            assert!(gpuio_protocol::decode(&bytes[..end]).is_err());
        }
        bytes.push(0);
        assert!(gpuio_protocol::decode(&bytes).is_err());
    }
}
