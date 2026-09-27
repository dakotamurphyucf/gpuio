use binprot::BinProtWrite;
use gpuio_protocol::{NodeId, WindowId, v1::*};

#[test]
fn appended_inert_field_matches_independent_fixture_and_rejects_bad_boolean() {
    let message = Message::Apply(Transaction {
        window: WindowId::from_parts(0, 1).unwrap(),
        base: 0,
        revision: 1,
        operations: vec![Op::SetStyle(
            NodeId::from_parts(0, 1).unwrap(),
            vec![Style::Fields(vec![Field::Inert(true)])],
        )],
    });
    let mut bytes = vec![];
    message.binprot_write(&mut bytes).unwrap();
    let hex: String = bytes.iter().map(|b| format!("{b:02x}")).collect();
    assert_eq!(
        hex,
        include_str!("../../../test/fixtures/inert-request.hex").trim()
    );
    assert_eq!(gpuio_protocol::decode(&bytes), Ok(message));
    for end in 0..bytes.len() {
        assert!(gpuio_protocol::decode(&bytes[..end]).is_err());
    }
    *bytes.last_mut().unwrap() = 2;
    assert!(gpuio_protocol::decode(&bytes).is_err());
    *bytes.last_mut().unwrap() = 0;
    assert!(gpuio_protocol::decode(&bytes).is_ok());
    bytes.push(0);
    assert!(gpuio_protocol::decode(&bytes).is_err());
}
