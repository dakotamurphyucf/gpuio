use binprot::BinProtWrite;
use gpuio_native::tree::Tree;
use gpuio_protocol::{HandlerId, NodeId, WindowId, v1::*};
fn id(slot: i64) -> NodeId {
    NodeId::from_parts(slot, 1).unwrap()
}
fn handler(slot: i64) -> HandlerId {
    HandlerId::from_parts(slot, 1).unwrap()
}
fn tx(base: i64, operations: Vec<Op>) -> Transaction {
    Transaction {
        window: WindowId::from_parts(0, 1).unwrap(),
        base,
        revision: base + 1,
        operations,
    }
}
fn choice(ids: &[&str]) -> ChoiceConfig {
    ChoiceConfig {
        label: "Mode".into(),
        items: ids
            .iter()
            .map(|id| ChoiceItem {
                id: (*id).into(),
                label: format!("Plain {id}"),
                disabled: false,
            })
            .collect(),
        selected: Some("a".into()),
        disabled: false,
    }
}
#[test]
fn rich_labels_revalidate_late_changes_and_leave_plain_controls_compatible() {
    for kind in [Kind::Checkbox, Kind::Switch] {
        let mut tree = Tree::new(WindowId::from_parts(0, 1).unwrap());
        let control = if kind == Kind::Checkbox {
            Control::Checkbox(CheckState::Checked, false)
        } else {
            Control::Switch(true, false)
        };
        tree.apply(&tx(
            0,
            vec![
                Op::Create(id(0), kind, "Name".into(), Some(handler(0))),
                Op::SetControl(id(0), control),
                Op::Create(id(1), Kind::Text, "Rich".into(), None),
                Op::Splice(id(0), 0, 0, vec![id(1)]),
                Op::SetRoot(Some(id(0))),
            ],
        ))
        .unwrap();
        let retained = tree.retained_bytes();
        for invalid in [
            Op::Bind(id(1), Some(handler(1))),
            Op::SetStyle(id(1), vec![Style::State(2, vec![Field::UserSelect(true)])]),
            Op::SetText(id(0), " ".into()),
        ] {
            assert!(
                tree.apply(&tx(
                    1,
                    vec![Op::SetText(id(1), "Must rollback".into()), invalid]
                ))
                .is_err()
            );
            assert_eq!(tree.revision(), 1);
            assert_eq!(tree.get(id(1)).unwrap().text.as_ref(), "Rich");
            assert_eq!(tree.retained_bytes(), retained);
        }
        assert!(
            tree.apply(&tx(
                1,
                vec![
                    Op::Create(id(2), Kind::Text, "Extra".into(), None),
                    Op::Splice(id(0), 1, 0, vec![id(2)])
                ]
            ))
            .is_err()
        );
        assert_eq!(tree.retained_bytes(), retained);
        tree.apply(&tx(
            1,
            vec![
                Op::Splice(id(0), 0, 1, vec![]),
                Op::Remove(id(1)),
                Op::SetText(id(0), "Plain".into()),
            ],
        ))
        .unwrap();
        assert_eq!(tree.get(id(0)).unwrap().handler, Some(handler(0)));
        tree.apply(&tx(2, vec![Op::SetRoot(None), Op::Remove(id(0))]))
            .unwrap();
        assert_eq!(tree.retained_bytes(), 0);
    }
}
#[test]
fn choice_slots_validate_against_final_options_and_reject_hidden_interactivity() {
    for kind in [Kind::RadioGroup, Kind::TabBar] {
        let mut tree = Tree::new(WindowId::from_parts(0, 1).unwrap());
        tree.apply(&tx(
            0,
            vec![
                Op::Create(id(0), kind, "".into(), Some(handler(0))),
                Op::SetChoice(id(0), choice(&["a", "b"])),
                Op::Create(id(1), Kind::Container, "".into(), None),
                Op::Create(id(2), Kind::Container, "".into(), None),
                Op::Create(id(3), Kind::Text, "Rich A".into(), None),
                Op::Splice(id(1), 0, 0, vec![id(3)]),
                Op::Splice(id(0), 0, 0, vec![id(1), id(2)]),
                Op::SetRoot(Some(id(0))),
            ],
        ))
        .unwrap();
        let retained = tree.retained_bytes();
        for invalid in [
            Op::SetText(id(1), "Unexpected slot text".into()),
            Op::Bind(id(3), Some(handler(1))),
            Op::Splice(id(0), 0, 2, vec![id(1)]),
            Op::SetChoice(id(0), choice(&["a"])),
            Op::SetStyle(id(3), vec![Style::Fields(vec![Field::Inert(true)])]),
        ] {
            assert!(
                tree.apply(&tx(
                    1,
                    vec![Op::SetText(id(3), "Must rollback".into()), invalid]
                ))
                .is_err()
            );
            assert_eq!(tree.revision(), 1);
            assert_eq!(tree.get(id(3)).unwrap().text.as_ref(), "Rich A");
            assert_eq!(tree.retained_bytes(), retained);
        }
        tree.apply(&tx(
            1,
            vec![
                Op::SetChoice(id(0), choice(&["b", "a"])),
                Op::Splice(id(0), 0, 2, vec![id(2), id(1)]),
            ],
        ))
        .unwrap();
        assert_eq!(tree.get(id(0)).unwrap().children.as_ref(), &[id(2), id(1)]);
        assert_eq!(tree.get(id(1)).unwrap().children.as_ref(), &[id(3)]);
        tree.apply(&tx(
            2,
            vec![
                Op::Splice(id(0), 0, 2, vec![]),
                Op::Splice(id(1), 0, 1, vec![]),
                Op::Remove(id(3)),
                Op::Remove(id(1)),
                Op::Remove(id(2)),
            ],
        ))
        .unwrap();
        assert!(tree.get(id(0)).unwrap().children.is_empty());
        tree.apply(&tx(3, vec![Op::SetRoot(None), Op::Remove(id(0))]))
            .unwrap();
        assert_eq!(tree.retained_bytes(), 0);
    }
}
#[test]
fn rich_label_capability_has_independent_ocaml_bytes() {
    assert_eq!(CAP_CONTROL_LABELS, 1_i64 << 61);
    assert_eq!(CAPABILITIES, i64::MAX);
    let message = Message::Hello(VERSION, CAP_CONTROL_LABELS);
    let mut bytes = Vec::new();
    message.binprot_write(&mut bytes).unwrap();
    assert_eq!(
        bytes.iter().map(|b| format!("{b:02x}")).collect::<String>(),
        include_str!("../../../test/fixtures/control-labels-hello.hex").trim()
    );
    assert_eq!(gpuio_protocol::decode(&bytes), Ok(message));
}

#[test]
fn names_use_the_same_blank_policy_as_core_string_strip() {
    for (name, valid) in [("\t\n\u{b}\u{c}\r ", false), ("\u{a0}", true)] {
        let mut tree = Tree::new(WindowId::from_parts(0, 1).unwrap());
        let result = tree.apply(&tx(
            0,
            vec![
                Op::Create(id(0), Kind::Checkbox, name.into(), Some(handler(0))),
                Op::SetControl(id(0), Control::Checkbox(CheckState::Unchecked, false)),
                Op::Create(id(1), Kind::Text, "Caption".into(), None),
                Op::Splice(id(0), 0, 0, vec![id(1)]),
                Op::SetRoot(Some(id(0))),
            ],
        ));
        assert_eq!(result.is_ok(), valid, "name {name:?}");
    }
}
