use binprot::BinProtWrite;
use gpuio_protocol::{DecodeError, NodeId, WindowId, decode, v1::*};
fn definition(label: &str) -> MenuDefinition {
    MenuDefinition {
        label: "Actions".into(),
        disabled: false,
        items: vec![MenuItem::Label(label.into())],
    }
}
fn message(config: MenuConfig) -> Message {
    Message::Apply(Transaction {
        window: WindowId::from_parts(0, 1).unwrap(),
        base: 0,
        revision: 1,
        operations: vec![Op::SetMenu(NodeId::from_parts(0, 1).unwrap(), config)],
    })
}
fn encode(value: &impl BinProtWrite) -> Vec<u8> {
    let mut bytes = vec![];
    value.binprot_write(&mut bytes).unwrap();
    bytes
}
#[test]
fn label_encoding_navigation_references_and_platform_admission_are_explicit() {
    assert_eq!(
        encode(&definition("Section")),
        b"\x07Actions\x00\x01\x03\x07Section"
    );
    for presentation in [
        MenuPresentation::Button,
        MenuPresentation::Context,
        MenuPresentation::Bar,
        MenuPresentation::EditorContext,
    ] {
        let config = MenuConfig {
            presentation,
            menus: vec![definition("Sections · α")],
        };
        assert!(config.is_valid());
        assert!(config.command_ids().is_empty());
        assert!(!config.permits("Sections · α"));
        let request = message(config);
        let bytes = encode(&request);
        assert_eq!(decode(&bytes), Ok(request));
        for end in 0..bytes.len() {
            assert!(decode(&bytes[..end]).is_err());
        }
    }
    let nested = MenuDefinition {
        label: "Outer".into(),
        disabled: false,
        items: vec![MenuItem::Submenu(definition("Section"))],
    };
    let config = MenuConfig {
        presentation: MenuPresentation::PlatformBar,
        menus: vec![nested],
    };
    assert!(!config.is_valid());
    assert_eq!(
        decode(&encode(&message(config))),
        Err(DecodeError::Malformed)
    );
}
#[test]
fn label_lengths_and_aggregate_text_are_bounded_before_allocation() {
    let config = |definition| MenuConfig {
        presentation: MenuPresentation::Button,
        menus: vec![definition],
    };
    for text in ["", " \t", "nul\0"] {
        let value = config(definition(text));
        assert!(!value.is_valid());
        assert_eq!(
            decode(&encode(&message(value))),
            Err(DecodeError::Malformed)
        );
    }
    let accepted = message(config(definition(&"x".repeat(4096))));
    assert_eq!(decode(&encode(&accepted)), Ok(accepted));
    let oversized = config(definition(&"x".repeat(4097)));
    assert!(!oversized.is_valid());
    assert_eq!(
        decode(&encode(&message(oversized))),
        Err(DecodeError::LimitExceeded)
    );
    let mut many = definition("ignored");
    many.items = vec![MenuItem::Label("x".repeat(4096)); 64];
    let value = config(many);
    assert!(!value.is_valid());
    assert_eq!(
        decode(&encode(&message(value))),
        Err(DecodeError::LimitExceeded)
    );
}
