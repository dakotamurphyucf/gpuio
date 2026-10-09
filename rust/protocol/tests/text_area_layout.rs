use binprot::BinProtWrite;
use gpuio_protocol::{
    NodeId, WindowId, decode,
    text_area_layout::{Config, WrappingIndent},
    v1::*,
};

fn encode(config: Option<Config>) -> Vec<u8> {
    let mut bytes = vec![];
    message(vec![config]).binprot_write(&mut bytes).unwrap();
    bytes
}
fn message(configs: Vec<Option<Config>>) -> Message {
    Message::Apply(Transaction {
        window: WindowId::from_parts(0, 1).unwrap(),
        base: 0,
        revision: 1,
        operations: configs
            .into_iter()
            .map(|c| Op::SetTextAreaLayout(NodeId::from_parts(1, 2).unwrap(), c))
            .collect(),
    })
}

#[test]
fn text_area_layout_has_independent_operation_bytes_and_bounded_decoding() {
    let msg = message(vec![
        None,
        Some(Config::default()),
        Some(Config {
            soft_wrap: false,
            wrapping_indent: WrappingIndent::FlushLeft,
            show_whitespace: true,
            cursor_margin_lines: Some(256),
        }),
    ]);
    let hex = include_str!("../../../test/fixtures/textarea-layout-operation.hex").trim();
    let expected: Vec<_> = hex
        .as_bytes()
        .chunks_exact(2)
        .map(|p| u8::from_str_radix(std::str::from_utf8(p).unwrap(), 16).unwrap())
        .collect();
    let mut bytes = vec![];
    msg.binprot_write(&mut bytes).unwrap();
    assert_eq!(bytes, expected);
    assert_eq!(decode(&bytes).unwrap(), msg);
    for end in 0..bytes.len() {
        assert!(decode(&bytes[..end]).is_err());
    }
    bytes.push(0);
    assert!(decode(&bytes).is_err());
    for margin in [-1, 257, i64::MAX] {
        assert!(
            decode(&encode(Some(Config {
                cursor_margin_lines: Some(margin),
                ..Config::default()
            })))
            .is_err()
        );
    }
    let baseline = encode(Some(Config::default()));
    for index in 10..=13 {
        let mut bytes = baseline.clone();
        bytes[index] = 2;
        assert!(decode(&bytes).is_err(), "bad layout field at {index}");
    }
}
