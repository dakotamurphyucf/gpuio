use binprot::BinProtWrite;
use gpuio_protocol::{NodeId, WindowId, decode, v1::*};

#[test]
fn menu_flag_uses_paired_opcode_and_strict_boolean_decoding() {
    for enabled in [false, true] {
        let op = Op::SetChoiceMenu(NodeId::from_parts(0, 1).unwrap(), enabled);
        let mut bytes = Vec::new();
        op.binprot_write(&mut bytes).unwrap();
        assert_eq!(bytes, [102, 0, 1, u8::from(enabled)]);
        let message = Message::Apply(Transaction {
            window: WindowId::from_parts(0, 1).unwrap(),
            base: 0,
            revision: 1,
            operations: vec![op],
        });
        let mut bytes = Vec::new();
        message.binprot_write(&mut bytes).unwrap();
        assert_eq!(decode(&bytes), Ok(message));
        for n in 0..bytes.len() {
            assert!(decode(&bytes[..n]).is_err());
        }
        *bytes.last_mut().unwrap() = 2;
        assert!(decode(&bytes).is_err());
    }
}
