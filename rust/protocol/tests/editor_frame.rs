use binprot::BinProtWrite;
use gpuio_protocol::{DecodeError, NodeId, WindowId, decode, editor_frame::Config, v1::*};
fn message(frame: Option<Config>) -> Message {
    Message::Apply(Transaction {
        window: WindowId::from_parts(0, 1).unwrap(),
        base: 0,
        revision: 1,
        operations: vec![Op::SetEditorFrame(NodeId::from_parts(1, 2).unwrap(), frame)],
    })
}
#[test]
fn frame_has_paired_bytes_and_bounded_decoding() {
    let node = NodeId::from_parts(1, 2).unwrap();
    let msg = Message::Apply(Transaction {
        window: WindowId::from_parts(0, 1).unwrap(),
        base: 0,
        revision: 1,
        operations: vec![
            Op::SetEditorFrame(node, None),
            Op::SetEditorFrame(
                node,
                Some(Config {
                    clear_label: Some("Clear".into()),
                    loading: true,
                    gap: 6.,
                }),
            ),
            Op::SetEditorFrame(
                node,
                Some(Config {
                    clear_label: None,
                    loading: false,
                    gap: 0.,
                }),
            ),
        ],
    });
    let mut bytes = vec![];
    msg.binprot_write(&mut bytes).unwrap();
    assert_eq!(
        bytes.iter().map(|b| format!("{b:02x}")).collect::<String>(),
        include_str!("../../../test/fixtures/editor-frame-operation.hex").trim()
    );
    assert_eq!(decode(&bytes), Ok(msg));
    for end in 0..bytes.len() {
        assert!(decode(&bytes[..end]).is_err());
    }
    bytes.push(0);
    assert_eq!(decode(&bytes), Err(DecodeError::Malformed));
    for gap in [f64::NAN, f64::INFINITY, -1., 256.1] {
        let mut bytes = vec![];
        message(Some(Config {
            clear_label: None,
            loading: false,
            gap,
        }))
        .binprot_write(&mut bytes)
        .unwrap();
        assert_eq!(decode(&bytes), Err(DecodeError::Malformed));
    }
    for label in [
        "".to_owned(),
        " \t\n".into(),
        "a\0b".into(),
        "a".repeat(4097),
    ] {
        let mut bytes = vec![];
        message(Some(Config {
            clear_label: Some(label),
            loading: false,
            gap: 6.,
        }))
        .binprot_write(&mut bytes)
        .unwrap();
        assert_eq!(decode(&bytes), Err(DecodeError::Malformed));
    }
}
