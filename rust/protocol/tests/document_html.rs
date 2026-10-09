use binprot::BinProtWrite;
use gpuio_protocol::{
    NodeId, WindowId,
    document::{Config, Layout, Mode},
    v1::*,
};
fn bytes<T: BinProtWrite>(value: &T) -> Vec<u8> {
    let mut out = vec![];
    value.binprot_write(&mut out).unwrap();
    out
}
#[test]
fn html_appends_mode_tag_and_roundtrips_document_admission() {
    for (mode, expected) in [
        (Mode::Markdown, vec![0]),
        (Mode::Code("ml".into()), vec![1, 2, b'm', b'l']),
        (Mode::Diff, vec![2]),
        (Mode::Html, vec![3]),
    ] {
        assert_eq!(bytes(&mode), expected);
        let value = Message::Apply(Transaction {
            window: WindowId::from_parts(0, 1).unwrap(),
            base: 0,
            revision: 1,
            operations: vec![Op::SetDocument(
                NodeId::from_parts(0, 1).unwrap(),
                Config {
                    source: None,
                    mode,
                    dark: false,
                    layout: Layout::Flow,
                    label: "Reader".into(),
                    path: None,
                    line_numbers: false,
                    initially_collapsed: false,
                    search: "".into(),
                    images: vec![],
                },
            )],
        });
        let encoded = bytes(&value);
        assert_eq!(gpuio_protocol::decode(&encoded), Ok(value));
        for end in 0..encoded.len() {
            assert!(gpuio_protocol::decode(&encoded[..end]).is_err());
        }
    }
}
