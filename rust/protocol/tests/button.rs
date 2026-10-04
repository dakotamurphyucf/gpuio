use binprot::BinProtWrite;
use gpuio_protocol::{
    button::{Config, Content, Focus, Policy},
    checkable::TabOrder,
    decode_button_config,
};
fn encode(value: &Config) -> Vec<u8> {
    let mut bytes = vec![];
    value.binprot_write(&mut bytes).unwrap();
    bytes
}
#[test]
fn button_policies_match_independent_ocaml_fixtures_and_boundaries() {
    let cases = [
        (
            Config::default(),
            include_str!("../../../test/fixtures/button-default.hex"),
        ),
        (
            Config {
                policy: Policy {
                    loading: true,
                    focus: Focus::Focusable(TabOrder {
                        tab_stop: false,
                        index: -2,
                    }),
                },
                content: Content::Rich,
            },
            include_str!("../../../test/fixtures/button-loading.hex"),
        ),
        (
            Config {
                policy: Policy {
                    loading: false,
                    focus: Focus::Preserve,
                },
                content: Content::Rich,
            },
            include_str!("../../../test/fixtures/button-preserve.hex"),
        ),
    ];
    for (config, expected) in cases {
        let bytes = encode(&config);
        assert_eq!(
            bytes
                .iter()
                .map(|byte| format!("{byte:02x}"))
                .collect::<String>(),
            expected.trim()
        );
        assert_eq!(decode_button_config(&bytes), Ok(config));
        for end in 0..bytes.len() {
            assert!(decode_button_config(&bytes[..end]).is_err());
        }
        let mut extra = bytes.clone();
        extra.push(0);
        assert!(decode_button_config(&extra).is_err());
        for offset in [0, 1, bytes.len() - 1] {
            let mut malformed = bytes.clone();
            malformed[offset] = 2;
            assert!(decode_button_config(&malformed).is_err());
        }
    }
}
#[test]
fn button_policy_validates_order_even_when_loading_or_not_a_tab_stop() {
    for loading in [false, true] {
        for tab_stop in [false, true] {
            for index in [i64::MIN, -1_000_001, 1_000_001, i64::MAX] {
                let config = Config {
                    policy: Policy {
                        loading,
                        focus: Focus::Focusable(TabOrder { tab_stop, index }),
                    },
                    content: Content::Rich,
                };
                assert!(!config.is_valid());
                assert!(decode_button_config(&encode(&config)).is_err());
            }
            for index in [-1_000_000, -1, 0, 1, 1_000_000] {
                let config = Config {
                    policy: Policy {
                        loading,
                        focus: Focus::Focusable(TabOrder { tab_stop, index }),
                    },
                    content: Content::IconSlots,
                };
                assert!(config.is_valid());
                assert_eq!(decode_button_config(&encode(&config)), Ok(config));
            }
        }
    }
    assert!(
        decode_button_config(&[0, 0, 2, 0, 0]).is_err(),
        "invalid tab-stop Boolean"
    );
}
#[test]
fn button_operation_tags_reset_and_config_match_the_ocaml_fixture() {
    use gpuio_protocol::{
        NodeId, WindowId, decode,
        v1::{Message, Op, Transaction},
    };
    let node = NodeId::from_parts(0, 1).unwrap();
    let message = Message::Apply(Transaction {
        window: WindowId::from_parts(0, 1).unwrap(),
        base: 0,
        revision: 1,
        operations: vec![
            Op::SetButtonPresentation(
                node,
                Some(Config {
                    policy: Policy {
                        loading: false,
                        focus: Focus::Preserve,
                    },
                    content: Content::Rich,
                }),
            ),
            Op::SetButtonPresentation(node, None),
        ],
    });
    let mut bytes = vec![];
    message.binprot_write(&mut bytes).unwrap();
    assert_eq!(
        bytes.iter().map(|b| format!("{b:02x}")).collect::<String>(),
        include_str!("../../../test/fixtures/button-operation.hex").trim()
    );
    assert_eq!(decode(&bytes), Ok(message));
    for end in 0..bytes.len() {
        assert!(decode(&bytes[..end]).is_err());
    }
    let mut extra = bytes;
    extra.push(0);
    assert!(decode(&extra).is_err());
}
