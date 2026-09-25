use binprot::BinProtWrite;
use gpuio_protocol::{HandlerId, NodeId, WindowId, decode, split::*, v1::*};
fn config() -> Config {
    Config {
        label: "x".into(),
        axis: Axis::Horizontal,
        initial_first: 160.,
        minimum_first: 80.,
        maximum_first: 280.,
        minimum_second: 80.,
        keyboard_step: 16.,
        reset_generation: 0,
    }
}
fn message(config: Config) -> Message {
    let node = NodeId::from_parts(0, 1).unwrap();
    Message::Apply(Transaction {
        window: WindowId::from_parts(0, 1).unwrap(),
        base: 0,
        revision: 1,
        operations: vec![
            Op::Create(
                node,
                Kind::SplitPane,
                "".into(),
                Some(HandlerId::from_parts(0, 1).unwrap()),
            ),
            Op::SetSplit(node, config),
        ],
    })
}
fn bytes(value: &impl BinProtWrite) -> Vec<u8> {
    let mut bytes = Vec::new();
    value.binprot_write(&mut bytes).unwrap();
    bytes
}
fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}
#[test]
fn independent_split_request_and_event_fixtures() {
    let message = message(config());
    let encoded = bytes(&message);
    assert_eq!(decode(&encoded).unwrap(), message);
    assert_eq!(
        hex(&encoded),
        "0300010001020000011d000100012200010178000000000000006440000000000000544000000000008071400000000000005440000000000000304000"
    );
    let events = vec![Event::SplitResized(
        WindowId::from_parts(0, 1).unwrap(),
        NodeId::from_parts(0, 1).unwrap(),
        HandlerId::from_parts(0, 1).unwrap(),
        1,
        0,
        Snapshot {
            first: 160.,
            second: 240.,
        },
    )];
    assert_eq!(
        hex(&bytes(&events)),
        "0125000100010001010000000000000064400000000000006e40"
    );
}
#[test]
fn reject_nonfinite_invalid_ranges_and_generations() {
    for value in [f64::NAN, f64::INFINITY, -1., 300.] {
        let mut c = config();
        c.minimum_first = value;
        assert!(decode(&bytes(&message(c))).is_err());
    }
    let mut c = config();
    c.reset_generation = -1;
    assert!(decode(&bytes(&message(c))).is_err());
    let mut c = config();
    c.label = "bad\0label".into();
    assert!(decode(&bytes(&message(c))).is_err());
}
