use binprot::BinProtWrite;
use gpuio_protocol::{
    DecodeError, NodeId, WindowId, decode,
    v1::{EditorPrivacy, Message, Op, Transaction},
};

#[test]
fn privacy_matches_independent_ocaml_bytes_and_rejects_malformed_payloads() {
    let node = NodeId::from_parts(1, 2).unwrap();
    let message = Message::Apply(Transaction {
        window: WindowId::from_parts(0, 1).unwrap(),
        base: 0,
        revision: 1,
        operations: [
            EditorPrivacy::Plain,
            EditorPrivacy::PasswordHidden,
            EditorPrivacy::PasswordRevealed,
        ]
        .map(|privacy| Op::SetEditorPrivacy(node, privacy))
        .to_vec(),
    });
    let mut bytes = vec![];
    message.binprot_write(&mut bytes).unwrap();
    assert_eq!(
        bytes.iter().map(|b| format!("{b:02x}")).collect::<String>(),
        include_str!("../../../test/fixtures/editor-privacy-operation.hex").trim()
    );
    assert_eq!(decode(&bytes), Ok(message));
    for end in 0..bytes.len() {
        assert!(decode(&bytes[..end]).is_err());
    }
    let mut extra = bytes.clone();
    extra.push(0);
    assert_eq!(decode(&extra), Err(DecodeError::Malformed));
    // Independent fixture: privacy is the final byte of each four-byte operation.
    for offset in [9, 13, 17] {
        for tag in [3, 127, 255] {
            let mut malformed = bytes.clone();
            malformed[offset] = tag;
            assert_eq!(decode(&malformed), Err(DecodeError::Malformed));
        }
    }
}
