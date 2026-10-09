use binprot::BinProtWrite;
use gpuio_protocol::{NodeId, WindowId, decode, v1::*};

#[test]
fn editor_context_menu_matches_paired_bytes_and_rejects_invalid_presentations() {
    let message = Message::Apply(Transaction {
        window: WindowId::from_parts(0, 1).unwrap(),
        base: 0,
        revision: 1,
        operations: vec![Op::SetMenu(
            NodeId::from_parts(1, 2).unwrap(),
            MenuConfig {
                presentation: MenuPresentation::EditorContext,
                menus: vec![MenuDefinition {
                    label: "Edit".into(),
                    disabled: false,
                    items: vec![MenuItem::Command("copy".into())],
                }],
            },
        )],
    });
    let mut bytes = vec![];
    message.binprot_write(&mut bytes).unwrap();
    assert_eq!(
        bytes.iter().map(|b| format!("{b:02x}")).collect::<String>(),
        include_str!("../../../test/fixtures/editor-menu-operation.hex").trim()
    );
    assert_eq!(decode(&bytes), Ok(message));
    for end in 0..bytes.len() {
        assert!(decode(&bytes[..end]).is_err());
    }
    let mut extra = bytes.clone();
    extra.push(0);
    assert!(decode(&extra).is_err());
    for tag in [6, 127, 255] {
        let mut invalid = bytes.clone();
        invalid[9] = tag;
        assert!(decode(&invalid).is_err());
    }
    for menus in [
        vec![],
        vec![
            MenuDefinition {
                label: "Edit".into(),
                disabled: false,
                items: vec![]
            };
            2
        ],
    ] {
        assert!(
            !MenuConfig {
                presentation: MenuPresentation::EditorContext,
                menus
            }
            .is_valid()
        );
    }
}
