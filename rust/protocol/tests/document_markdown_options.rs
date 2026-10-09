use binprot::BinProtWrite;
use gpuio_protocol::{
    NodeId, WindowId,
    document::{Frontmatter, MarkdownOptions},
    v1::*,
};
fn bytes<T: BinProtWrite>(value: &T) -> Vec<u8> {
    let mut out = vec![];
    value.binprot_write(&mut out).unwrap();
    out
}
#[test]
fn markdown_parser_options_match_ocaml_and_reject_malformed_data() {
    let node = NodeId::from_parts(0, 1).unwrap();
    for (options, hex) in [
        (MarkdownOptions::default(), "7700010000"),
        (
            MarkdownOptions {
                frontmatter: Frontmatter::DescriptionList,
                mdx: false,
            },
            "7700010200",
        ),
        (
            MarkdownOptions {
                frontmatter: Frontmatter::CodeBlock,
                mdx: true,
            },
            "7700010101",
        ),
    ] {
        let op = Op::SetDocumentMarkdownOptions(node, options);
        assert_eq!(
            bytes(&op)
                .iter()
                .map(|b| format!("{b:02x}"))
                .collect::<String>(),
            hex
        );
        let message = Message::Apply(Transaction {
            window: WindowId::from_parts(0, 1).unwrap(),
            base: 0,
            revision: 1,
            operations: vec![op],
        });
        let data = bytes(&message);
        assert_eq!(gpuio_protocol::decode(&data), Ok(message));
        for end in 0..data.len() {
            assert!(gpuio_protocol::decode(&data[..end]).is_err());
        }
        for index in [data.len() - 2, data.len() - 1] {
            let mut bad = data.clone();
            bad[index] = 3;
            assert!(gpuio_protocol::decode(&bad).is_err());
        }
        let mut trailing = data;
        trailing.push(0);
        assert!(gpuio_protocol::decode(&trailing).is_err());
    }
}
