use binprot::BinProtWrite;
use gpuio_protocol::animation::*;
#[test]
fn animation_configuration_matches_independent_ocaml_fixture() {
    let config = Config {
        generation: 42,
        targets: vec![
            Target {
                property: Property::Width,
                value: 240.,
            },
            Target {
                property: Property::Opacity,
                value: 0.5,
            },
        ],
        initial: Some(vec![
            Target {
                property: Property::Width,
                value: 0.,
            },
            Target {
                property: Property::Opacity,
                value: 0.,
            },
        ]),
        duration_ms: 100,
        delay_ms: 10,
        easing: Easing::CubicBezier(0.25, 0., 0.75, 1.),
        repeat: Repeat::Alternate,
    };
    assert!(config.is_valid());
    let mut bytes = Vec::new();
    config.binprot_write(&mut bytes).unwrap();
    let encoded: String = bytes.iter().map(|byte| format!("{byte:02x}")).collect();
    assert_eq!(
        encoded,
        include_str!("../../../test/fixtures/animation-v1-config.hex").trim()
    );
}
