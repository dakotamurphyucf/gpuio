use binprot::BinProtWrite;
use gpuio_protocol::{
    NodeId, WindowId, decode,
    tab_appearance::{Config, Variant},
    v1::*,
};
fn node() -> NodeId {
    NodeId::from_parts(0, 1).unwrap()
}
fn bytes<T: BinProtWrite>(value: &T) -> Vec<u8> {
    let mut bytes = vec![];
    value.binprot_write(&mut bytes).unwrap();
    bytes
}
fn message(config: Config) -> Message {
    Message::Apply(Transaction {
        window: WindowId::from_parts(0, 1).unwrap(),
        base: 0,
        revision: 1,
        operations: vec![Op::SetTabAppearance(node(), Some(config))],
    })
}
#[test]
fn paired_appended_operation_and_all_variants_decode_strictly() {
    let pill = Config {
        variant: Variant::Pill,
        ..Config::default()
    };
    assert_eq!(
        bytes(&Op::SetTabAppearance(node(), Some(pill)))
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect::<String>(),
        "62000101020000000000004040000000000000104000000000000028400000"
    );
    for variant in [
        Variant::Tab,
        Variant::Outline,
        Variant::Pill,
        Variant::Segmented,
        Variant::Underline,
    ] {
        let message = message(Config {
            variant,
            tab_style: vec![Style::State(
                7,
                vec![Field::Foreground(Color::Rgba(0x11223344))],
            )],
            item_styles: vec![(
                "α".into(),
                vec![Style::Fields(vec![Field::Width(Length::Px(140.))])],
            )],
            ..Config::default()
        });
        let encoded = bytes(&message);
        assert_eq!(decode(&encoded), Ok(message));
        for length in 0..encoded.len() {
            assert!(decode(&encoded[..length]).is_err());
        }
        let mut extra = encoded;
        extra.push(0);
        assert!(decode(&extra).is_err());
    }
}
#[test]
fn invalid_geometry_duplicates_and_aggregate_declarations_are_rejected() {
    for height in [f64::NAN, f64::INFINITY, 0., 15.9, 256.1] {
        assert!(
            decode(&bytes(&message(Config {
                height,
                ..Config::default()
            })))
            .is_err()
        );
    }
    for config in [
        Config {
            tab_style: vec![Style::Fields(vec![]); 256],
            item_styles: vec![("a".into(), vec![Style::Fields(vec![])])],
            ..Config::default()
        },
        Config {
            item_styles: vec![("same".into(), vec![]), ("same".into(), vec![])],
            ..Config::default()
        },
        Config {
            item_styles: vec![(String::new(), vec![])],
            ..Config::default()
        },
        Config {
            tab_style: vec![Style::State(4, vec![Field::Opacity(0.5)])],
            ..Config::default()
        },
        Config {
            tab_style: vec![Style::Fields(vec![Field::Opacity(0.5); 128])],
            item_styles: vec![(
                "a".into(),
                vec![Style::Fields(vec![Field::Opacity(0.5); 129])],
            )],
            ..Config::default()
        },
    ] {
        assert!(decode(&bytes(&message(config))).is_err());
    }
}
