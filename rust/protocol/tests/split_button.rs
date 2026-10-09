use binprot::BinProtWrite;
use gpuio_protocol::{
    decode_split_button_config,
    split_button::{Config, Parts},
    v1::*,
};

fn encode(value: &Config) -> Vec<u8> {
    let mut bytes = vec![];
    value.binprot_write(&mut bytes).unwrap();
    bytes
}

#[test]
fn split_operation_tag_and_reset_match_independent_bytes() {
    use gpuio_protocol::{NodeId, WindowId, decode};
    let id = NodeId::from_parts(0, 1).unwrap();
    let message = Message::Apply(Transaction {
        window: WindowId::from_parts(0, 1).unwrap(),
        base: 0,
        revision: 1,
        operations: vec![
            Op::SetSplitButton(
                id,
                Some(Config {
                    parts: Parts::Split,
                    surface: vec![],
                    menu_open: vec![],
                }),
            ),
            Op::SetSplitButton(id, None),
        ],
    });
    let mut bytes = vec![];
    message.binprot_write(&mut bytes).unwrap();
    assert_eq!(
        bytes
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect::<String>(),
        include_str!("../../../test/fixtures/split-button-operation.hex").trim()
    );
    assert_eq!(decode(&bytes), Ok(message));
    for end in 0..bytes.len() {
        assert!(decode(&bytes[..end]).is_err());
    }
    bytes.push(0);
    assert!(decode(&bytes).is_err());
}

#[test]
fn split_paint_matches_independent_bytes_and_rejects_incomplete_envelopes() {
    let config = Config {
        parts: Parts::Split,
        surface: vec![Style::Fields(vec![Field::Foreground(Color::Rgba(0x11))])],
        menu_open: vec![Style::Fields(vec![Field::Foreground(Color::Rgba(0x22))])],
    };
    let bytes = encode(&config);
    assert_eq!(
        bytes
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect::<String>(),
        include_str!("../../../test/fixtures/split-button-paint.hex").trim()
    );
    assert_eq!(decode_split_button_config(&bytes), Ok(config));
    for end in 0..bytes.len() {
        assert!(decode_split_button_config(&bytes[..end]).is_err());
    }
    let mut extra = bytes.clone();
    extra.push(0);
    assert!(decode_split_button_config(&extra).is_err());
    let mut bad_tag = bytes;
    bad_tag[0] = 3;
    assert!(decode_split_button_config(&bad_tag).is_err());
    for (tag, parts) in [(0, Parts::Primary), (1, Parts::Menu), (2, Parts::Split)] {
        assert_eq!(
            decode_split_button_config(&[tag, 0, 0]),
            Ok(Config {
                parts,
                surface: vec![],
                menu_open: vec![]
            })
        );
    }
}

#[test]
fn split_paint_rejects_layout_interaction_and_excess_declarations() {
    let config = |style| Config {
        parts: Parts::Split,
        surface: vec![style],
        menu_open: vec![],
    };
    for style in [
        Style::Fields(vec![Field::Width(Length::Px(12.))]),
        Style::Fields(vec![Field::Opacity(0.5)]),
        Style::Fields(vec![Field::Disabled(true)]),
        Style::Fields(vec![Field::PointerEvents(false)]),
        Style::State(2, vec![Field::Foreground(Color::Rgba(0))]),
        Style::Foreground(Color::Rgba(0)),
    ] {
        let config = config(style);
        assert!(!config.has_valid_shape());
        assert!(decode_split_button_config(&encode(&config)).is_err());
    }
    let mut config = config(Style::Fields(vec![Field::Foreground(Color::Rgba(0)); 32]));
    config.menu_open = config.surface.clone();
    assert!(config.has_valid_shape());
    assert_eq!(
        decode_split_button_config(&encode(&config)),
        Ok(config.clone())
    );
    config
        .menu_open
        .push(Style::Fields(vec![Field::Foreground(Color::Rgba(0))]));
    assert!(!config.has_valid_shape());
    assert!(decode_split_button_config(&encode(&config)).is_err());
    config.surface = vec![Style::Fields(vec![]); 65];
    config.menu_open.clear();
    assert!(!config.has_valid_shape());
    assert!(decode_split_button_config(&encode(&config)).is_err());
}
