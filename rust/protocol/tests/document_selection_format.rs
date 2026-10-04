use binprot::BinProtWrite;
use gpuio_protocol::{NodeId, WindowId, v1::*};

fn bytes<T: BinProtWrite>(value: &T) -> Vec<u8> {
    let mut out = vec![];
    value.binprot_write(&mut out).unwrap();
    out
}

#[test]
fn selection_format_has_paired_bytes_and_strict_boolean_admission() {
    for markdown in [false, true] {
        let op = Op::SetDocumentSelectionFormat(NodeId::from_parts(0, 1).unwrap(), markdown);
        assert_eq!(bytes(&op), vec![116, 0, 1, u8::from(markdown)]);
        let message = Message::Apply(Transaction {
            window: WindowId::from_parts(0, 1).unwrap(),
            base: 0,
            revision: 1,
            operations: vec![op],
        });
        let mut encoded = bytes(&message);
        assert_eq!(gpuio_protocol::decode(&encoded), Ok(message));
        for length in 0..encoded.len() {
            assert!(gpuio_protocol::decode(&encoded[..length]).is_err());
        }
        *encoded.last_mut().unwrap() = 2;
        assert!(gpuio_protocol::decode(&encoded).is_err());
    }
}
