use binprot::BinProtWrite;
use gpuio_protocol::{
    NodeId, WindowId, decode,
    loading::{Config, Kind},
    v1::{Message, Op, Transaction},
};
fn request(configs: Vec<Config>) -> Message {
    Message::Apply(Transaction {
        window: WindowId::from_parts(0, 1).unwrap(),
        base: 0,
        revision: 1,
        operations: configs
            .into_iter()
            .enumerate()
            .flat_map(|(i, c)| {
                let id = NodeId::from_parts(i as i64, 1).unwrap();
                [
                    Op::Create(id, gpuio_protocol::v1::Kind::Loading, "".into(), None),
                    Op::SetLoading(id, c),
                ]
            })
            .collect(),
    })
}
fn encode(message: &Message) -> Vec<u8> {
    let mut bytes = vec![];
    message.binprot_write(&mut bytes).unwrap();
    bytes
}
#[test]
fn paired_fixture_strict_decode_and_bounds() {
    let configs = vec![
        (Kind::Skeleton, 100),
        (Kind::Shimmer, 1200),
        (Kind::Spinner, 60000),
    ]
    .into_iter()
    .map(|(kind, period_ms)| Config {
        kind,
        period_ms,
        label: "Load".into(),
        animated: true,
    })
    .collect();
    let message = request(configs);
    let bytes = encode(&message);
    assert_eq!(
        bytes.iter().map(|b| format!("{b:02x}")).collect::<String>(),
        include_str!("../../../test/fixtures/loading-request.hex").trim()
    );
    assert_eq!(decode(&bytes), Ok(message));
    for n in 0..bytes.len() {
        assert!(decode(&bytes[..n]).is_err());
    }
    let mut extra = bytes.clone();
    extra.push(0);
    assert!(decode(&extra).is_err());
    let mut invalid_utf8 = bytes;
    let at = invalid_utf8.windows(4).position(|w| w == b"Load").unwrap();
    invalid_utf8[at] = 255;
    assert!(decode(&invalid_utf8).is_err());
    for label in ["".into(), " \t".into(), "x\0y".into(), "x".repeat(4097)] {
        assert!(
            decode(&encode(&request(vec![Config {
                kind: Kind::Spinner,
                label,
                animated: false,
                period_ms: 1200
            }])))
            .is_err()
        );
    }
    for period_ms in [i64::MIN, 0, 99, 60001, i64::MAX] {
        assert!(
            decode(&encode(&request(vec![Config {
                kind: Kind::Spinner,
                label: "Load".into(),
                animated: false,
                period_ms
            }])))
            .is_err()
        );
    }
    let maximum = request(vec![Config {
        kind: Kind::Spinner,
        label: "x".repeat(4096),
        animated: false,
        period_ms: 60000,
    }]);
    assert_eq!(decode(&encode(&maximum)), Ok(maximum));
}
