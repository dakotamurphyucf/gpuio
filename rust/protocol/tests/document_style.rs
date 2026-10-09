use binprot::BinProtWrite;
use gpuio_protocol::{NodeId, WindowId, document_style::*, v1::*};
fn bytes<T: BinProtWrite>(value: &T) -> Vec<u8> {
    let mut out = vec![];
    value.binprot_write(&mut out).unwrap();
    out
}
fn hex<T: BinProtWrite>(value: &T) -> String {
    bytes(value).iter().map(|b| format!("{b:02x}")).collect()
}
#[test]
fn document_style_has_paired_bytes_and_bounded_decoding() {
    let node = NodeId::from_parts(0, 1).unwrap();
    assert_eq!(
        hex(&Op::SetDocumentTextStyle(node, Some(Config::default()))),
        "76000101000000000000000000000000000000"
    );
    assert_eq!(hex(&Op::SetDocumentTextStyle(node, None)), "76000100");
    let config = Config {
        colors: vec![(Part::Link, Color::Rgba(0x336699ff))],
        paragraph_gap_rem: Some(2.),
        heading_sizes: Some(HeadingSizes {
            h1: 40.,
            h2: 30.,
            h3: 24.,
            h4: 20.,
            h5: 18.,
            h6: 16.,
        }),
        inline_code: InlineCode {
            italic: Some(true),
            font_weight: Some(600),
            underline: Some(Underline {
                color: None,
                thickness: 2.,
                wavy: true,
            }),
            fade_out: Some(0.1),
            ..Default::default()
        },
        table_cell: vec![Style::Fields(vec![Field::PaddingTop(Length::Px(8.))])],
        ..Default::default()
    };
    assert_eq!(
        gpuio_protocol::decode_document_style(&bytes(&config)),
        Ok(config.clone())
    );
    let value = Message::Apply(Transaction {
        window: WindowId::from_parts(0, 1).unwrap(),
        base: 0,
        revision: 1,
        operations: vec![Op::SetDocumentTextStyle(node, Some(config.clone()))],
    });
    let encoded = bytes(&value);
    assert_eq!(gpuio_protocol::decode(&encoded), Ok(value));
    for end in 0..encoded.len() {
        assert!(gpuio_protocol::decode(&encoded[..end]).is_err());
    }
    for value in [
        Config {
            paragraph_gap_rem: Some(f64::NAN),
            ..config.clone()
        },
        Config {
            colors: vec![(Part::Link, Color::Rgba(1)), (Part::Link, Color::Rgba(2))],
            ..config.clone()
        },
        Config {
            code_block: vec![Style::State(1, vec![])],
            ..config.clone()
        },
        Config {
            table: vec![Style::Fields(vec![Field::Height(Length::Px(10.))])],
            ..config.clone()
        },
        Config {
            inline_code: InlineCode {
                fade_out: Some(2.),
                ..Default::default()
            },
            ..config.clone()
        },
    ] {
        assert!(gpuio_protocol::decode_document_style(&bytes(&value)).is_err());
    }
    let too_many = Config {
        code_block: vec![Style::Fields(vec![
            Field::Opacity(1.);
            MAX_DECLARATIONS + 1
        ])],
        ..Default::default()
    };
    assert!(!too_many.is_valid());
    assert!(gpuio_protocol::decode_document_style(&bytes(&too_many)).is_err());
}
