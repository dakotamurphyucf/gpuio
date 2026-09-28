use binprot::BinProtWrite;
use gpuio_protocol::{NodeId, WindowId, v1::*};

#[test]
fn appended_occlusion_tag_and_strict_request_decode() {
    for mode in 0..=2 {
        let field = Field::PointerOcclusion(mode);
        let mut bytes = vec![];
        field.binprot_write(&mut bytes).unwrap();
        assert_eq!(bytes, vec![66, mode as u8]); // independently checked by OCaml
        let request = Message::Apply(Transaction {
            window: WindowId::from_parts(0, 1).unwrap(),
            base: 0,
            revision: 1,
            operations: vec![Op::SetStyle(
                NodeId::from_parts(0, 1).unwrap(),
                vec![Style::Fields(vec![field])],
            )],
        });
        let mut bytes = vec![];
        request.binprot_write(&mut bytes).unwrap();
        assert_eq!(gpuio_protocol::decode(&bytes), Ok(request));
        for length in 0..bytes.len() {
            assert!(gpuio_protocol::decode(&bytes[..length]).is_err());
        }
        bytes.push(0);
        assert!(gpuio_protocol::decode(&bytes).is_err());
    }
}
