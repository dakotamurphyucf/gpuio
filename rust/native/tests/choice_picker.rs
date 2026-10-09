use gpuio_native::{
    choice_picker_admission::{self as admission, Children},
    tree::{Node, Tree},
};
use gpuio_protocol::{
    HandlerId, NodeId, WindowId, choice_picker::*, decode_choice_picker_presentation, v1::*,
};
use std::{collections::BTreeMap, sync::Arc};
fn id(slot: i64) -> NodeId {
    NodeId::from_parts(slot, 1).unwrap()
}
fn handler(slot: i64) -> HandlerId {
    HandlerId::from_parts(slot, 1).unwrap()
}
fn presentation() -> Presentation {
    let hex = include_str!("../../../test/fixtures/choice-picker-presentation.hex").trim();
    let bytes: Vec<_> = (0..hex.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&hex[i..i + 2], 16).unwrap())
        .collect();
    decode_choice_picker_presentation(&bytes).unwrap()
}
fn mount_transaction(picker: bool) -> (Presentation, Vec<NodeId>, Transaction) {
    let p = presentation();
    let window = WindowId::from_parts(0, 1).unwrap();
    let mut ops = vec![Op::Create(
        id(0),
        if picker {
            Kind::ChoicePicker
        } else {
            Kind::Container
        },
        String::new(),
        if picker { Some(handler(20)) } else { None },
    )];
    if picker {
        ops.push(Op::SetChoicePicker(id(0), Box::new(p.clone())));
    }
    let mut slots = vec![];
    for (i, role) in p.slots.iter().enumerate() {
        let wrapper = id((i * 2 + 1) as i64);
        let child = id((i * 2 + 2) as i64);
        let kind = match role {
            Slot::Query => Kind::Input,
            Slot::Footer => Kind::Button,
            _ => Kind::Text,
        };
        ops.push(Op::Create(wrapper, Kind::Container, String::new(), None));
        ops.push(Op::Create(
            child,
            kind,
            "Content".into(),
            if matches!(role, Slot::Query | Slot::Footer) {
                Some(handler(i as i64))
            } else {
                None
            },
        ));
        if matches!(role, Slot::Query) {
            ops.push(Op::SetEditor(
                child,
                EditorConfig {
                    label: p.config.label.clone(),
                    placeholder: p.config.search_placeholder.clone(),
                    read_only: false,
                    disabled: false,
                    submit_on_enter: false,
                    auto_focus: false,
                    min_rows: 1,
                    max_rows: 1,
                },
            ));
        }
        ops.push(Op::Splice(wrapper, 0, 0, vec![child]));
        slots.push(wrapper);
    }
    ops.push(Op::Splice(id(0), 0, 0, slots.clone()));
    ops.push(Op::SetRoot(Some(id(0))));
    (
        p,
        slots,
        Transaction {
            window,
            base: 0,
            revision: 1,
            operations: ops,
        },
    )
}
fn mounted(picker: bool) -> (Presentation, Vec<NodeId>, Tree) {
    let (p, slots, tx) = mount_transaction(picker);
    let mut tree = Tree::new(tx.window);
    tree.apply(&tx).unwrap();
    (p, slots, tree)
}

#[test]
fn picker_query_rejects_password_even_on_a_descendant_only_update() {
    let (_, _, mut tree) = mounted(true);
    let retained = tree.retained_bytes();
    for privacy in [
        EditorPrivacy::PasswordHidden,
        EditorPrivacy::PasswordRevealed,
    ] {
        let tx = Transaction {
            window: WindowId::from_parts(0, 1).unwrap(),
            base: 1,
            revision: 2,
            operations: vec![Op::SetEditorPrivacy(id(4), privacy)],
        };
        assert_eq!(tree.apply(&tx), Err(ErrorCode::InvalidTree));
        assert_eq!(tree.revision(), 1);
        assert_eq!(
            tree.get(id(4)).unwrap().editor_privacy,
            EditorPrivacy::Plain
        );
        assert_eq!(tree.retained_bytes(), retained);
    }
}

#[test]
fn picker_query_rejects_format_policy_even_on_a_descendant_only_update() {
    let (_, _, mut tree) = mounted(true);
    let retained = tree.retained_bytes();
    let tx = transaction(
        &tree,
        vec![Op::SetEditorFormat(
            id(4),
            Some(gpuio_protocol::input_format::Config::Pattern("99".into())),
        )],
    );
    assert_eq!(tree.apply(&tx), Err(ErrorCode::InvalidTree));
    assert_eq!(tree.revision(), 1);
    assert!(tree.get(id(4)).unwrap().editor_format.is_none());
    assert_eq!(tree.retained_bytes(), retained);
}

#[test]
fn picker_query_rejects_edit_filter_on_a_descendant_only_update() {
    use gpuio_protocol::input_validation::{Matching, Rule, Source};
    let (_, _, mut tree) = mounted(true);
    let before = tree.get(id(4)).unwrap().clone();
    let retained = tree.retained_bytes();
    let tx = transaction(
        &tree,
        vec![Op::SetEditorValidation(
            id(4),
            Some(Rule {
                regex: Source {
                    pattern: "[0-9]*".into(),
                    matching: Matching::WholeValue,
                    case_sensitive: true,
                },
                allow_empty: true,
            }),
        )],
    );
    assert_eq!(tree.apply(&tx), Err(ErrorCode::InvalidTree));
    assert_eq!(tree.revision(), 1);
    assert_eq!(tree.get(id(4)).unwrap(), &before);
    assert_eq!(tree.retained_bytes(), retained);
}
fn forest() -> (Presentation, Vec<NodeId>, BTreeMap<NodeId, Node>) {
    let (p, slots, tree) = mounted(false);
    let nodes = (0..15)
        .map(|i| (id(i), tree.get(id(i)).unwrap().clone()))
        .collect();
    (p, slots, nodes)
}
fn check(
    p: &Presentation,
    slots: &[NodeId],
    nodes: &BTreeMap<NodeId, Node>,
) -> Result<Children, ErrorCode> {
    admission::children(p, slots, |id| nodes.get(&id))
}
#[test]
fn choice_picker_admits_named_slots_and_distinct_query_editor_with_interactive_footer() {
    let (p, slots, nodes) = forest();
    let children = check(&p, &slots, &nodes).unwrap();
    assert_eq!(children.trigger, Some(id(2)));
    assert_eq!(children.query, Some(id(4)));
    assert_eq!(children.empty, Some(id(6)));
    assert_eq!(children.footer, Some(id(8)));
    assert_eq!(children.groups["g"], id(10));
    assert_eq!(children.options["a"], (id(12), Checkmark::Custom));
    assert_eq!(children.options["b"], (id(14), Checkmark::Native));
    assert!(nodes[&id(8)].handler.is_some());
    assert_eq!(check(&p, &slots[..6], &nodes), Err(ErrorCode::InvalidTree));
}
#[test]
fn choice_picker_rejects_late_callbacks_interaction_styles_bad_query_and_malformed_graph() {
    let (p, slots, nodes) = forest();
    for change in 0..8 {
        let mut changed = nodes.clone();
        match change {
            0 => changed.get_mut(&id(12)).unwrap().handler = Some(handler(20)),
            1 => changed.get_mut(&id(12)).unwrap().kind = Kind::Button,
            2 => {
                changed.get_mut(&id(12)).unwrap().style =
                    Arc::from([Style::Fields(vec![Field::UserSelect(true)])])
            }
            3 => {
                changed.get_mut(&id(12)).unwrap().style =
                    Arc::from([Style::Fields(vec![Field::OverflowY(3)])])
            }
            4 => changed.get_mut(&id(4)).unwrap().text = Arc::from("two\nlines"),
            5 => {
                Arc::make_mut(changed.get_mut(&id(4)).unwrap().editor.as_mut().unwrap())
                    .auto_focus = true
            }
            6 => changed.get_mut(&id(1)).unwrap().children = Arc::from([id(4)]),
            7 => changed.get_mut(&id(4)).unwrap().handler = None,
            _ => unreachable!(),
        }
        assert_eq!(
            check(&p, &slots, &changed),
            Err(ErrorCode::InvalidTree),
            "case {change}"
        );
    }
    let mut disabled = p.clone();
    disabled.config.disabled = true;
    assert_eq!(
        check(&disabled, &slots, &nodes),
        Err(ErrorCode::InvalidTree)
    );
    let mut matched = nodes.clone();
    Arc::make_mut(matched.get_mut(&id(4)).unwrap().editor.as_mut().unwrap()).disabled = true;
    assert!(check(&disabled, &slots, &matched).is_ok());
    // Reject a large footer forest before adding its descendants to the worklist.
    let mut large = nodes.clone();
    let template = large[&id(8)].clone();
    let mut children = Vec::new();
    for i in 100..4197 {
        let mut child = template.clone();
        child.id = id(i);
        child.parent = Some(id(8));
        child.kind = Kind::Text;
        child.handler = None;
        children.push(child.id);
        large.insert(child.id, child);
    }
    let footer = large.get_mut(&id(8)).unwrap();
    footer.kind = Kind::Container;
    footer.children = Arc::from(children);
    assert_eq!(check(&p, &slots, &large), Err(ErrorCode::LimitExceeded));
}
#[test]
fn choice_picker_part_styles_share_bounded_choice_vocabulary() {
    let p = presentation();
    assert!(admission::validate(&p).is_ok());
    for style in [
        Style::State(2, vec![Field::Opacity(0.5)]),
        Style::Fields(vec![Field::Width(Length::Px(10.))]),
        Style::Fields(vec![Field::FontSize(-1.)]),
        Style::Fields(vec![Field::Disabled(true)]),
    ] {
        let mut invalid = p.clone();
        invalid.header_style = vec![style];
        assert!(admission::validate(&invalid).is_err());
    }
    let mut excessive = p;
    excessive.header_style = vec![Style::Fields(vec![Field::Opacity(0.5); 128])];
    assert_eq!(
        admission::validate(&excessive),
        Err(ErrorCode::LimitExceeded)
    );
}

fn transaction(tree: &Tree, operations: Vec<Op>) -> Transaction {
    Transaction {
        window: WindowId::from_parts(0, 1).unwrap(),
        base: tree.revision(),
        revision: tree.revision() + 1,
        operations,
    }
}
#[test]
fn picker_tree_revalidates_descendants_and_rolls_back_the_whole_transaction() {
    let (p, _, mut tree) = mounted(true);
    let before: Vec<_> = (0..15).map(|i| tree.get(id(i)).unwrap().clone()).collect();
    let bytes = tree.retained_bytes();
    let mut wrong_query = tree.get(id(4)).unwrap().editor.as_deref().unwrap().clone();
    wrong_query.auto_focus = true;
    for operation in [
        Op::Bind(id(12), Some(handler(30))),
        Op::SetStyle(id(12), vec![Style::Fields(vec![Field::UserSelect(true)])]),
        Op::SetStyle(id(1), vec![Style::Fields(vec![Field::Opacity(0.5)])]),
        Op::SetEditor(id(4), wrong_query),
        Op::Bind(id(0), None),
        Op::SetText(id(0), "unexpected label".into()),
        Op::SetChoicePicker(id(2), Box::new(p.clone())),
        Op::Splice(id(0), 0, 1, vec![]),
    ] {
        let tx = transaction(
            &tree,
            vec![Op::SetText(id(2), "must roll back".into()), operation],
        );
        assert!(tree.apply(&tx).is_err());
        assert_eq!(tree.revision(), 1);
        assert_eq!(tree.retained_bytes(), bytes);
        for node in &before {
            assert_eq!(tree.get(node.id), Some(node));
        }
    }
    // A footer remains interactive, and painting passive content remains valid.
    let tx = transaction(
        &tree,
        vec![
            Op::Bind(id(8), Some(handler(31))),
            Op::SetText(id(12), "updated".into()),
        ],
    );
    tree.apply(&tx).unwrap();
    assert_eq!(tree.get(id(8)).unwrap().handler, Some(handler(31)));
    assert_eq!(&*tree.get(id(12)).unwrap().text, "updated");
}
#[test]
fn picker_config_and_query_policy_update_atomically_and_payload_budget_is_released() {
    let (mut p, _, mut tree) = mounted(true);
    p.config.disabled = true;
    let tx = transaction(&tree, vec![Op::SetChoicePicker(id(0), Box::new(p.clone()))]);
    assert_eq!(tree.apply(&tx), Err(ErrorCode::InvalidTree));
    let mut editor = tree.get(id(4)).unwrap().editor.as_deref().unwrap().clone();
    editor.disabled = true;
    let tx = transaction(
        &tree,
        vec![
            Op::SetChoicePicker(id(0), Box::new(p.clone())),
            Op::SetEditor(id(4), editor),
        ],
    );
    tree.apply(&tx).unwrap();
    let bytes = tree.retained_bytes();
    let original = p.clone();
    p.config.label.push_str(" expanded");
    let mut editor = tree.get(id(4)).unwrap().editor.as_deref().unwrap().clone();
    editor.label = p.config.label.clone();
    let tx = transaction(
        &tree,
        vec![
            Op::SetChoicePicker(id(0), Box::new(p.clone())),
            Op::SetEditor(id(4), editor),
        ],
    );
    assert_eq!(
        tree.apply_with_budget(&tx, bytes),
        Err(ErrorCode::LimitExceeded)
    );
    assert_eq!(tree.retained_bytes(), bytes);
    assert_eq!(
        tree.get(id(0)).unwrap().choice_picker.as_deref(),
        Some(&original)
    );
    tree.apply(&tx).unwrap();
    assert_eq!(tree.retained_bytes() - bytes, 3 * " expanded".len());
    let mut operations = vec![Op::SetRoot(None)];
    operations.extend((0..15).rev().map(|i| Op::Remove(id(i))));
    let tx = transaction(&tree, operations);
    tree.apply(&tx).unwrap();
    assert_eq!(tree.retained_bytes(), 0);
    assert!(tree.is_empty());
}

#[test]
fn picker_signals_preserve_current_model_intents_and_fence_query_remounts() {
    use gpuio_native::session::Session;
    use gpuio_protocol::choice_picker::Event as PickerEvent;
    let (mut p, _, tx) = mount_transaction(true);
    let window = tx.window;
    let mut session = Session::default();
    session.hello(VERSION, CAPABILITIES).unwrap();
    session.open(1, window, "Picker", 400., 200.).unwrap();
    session.apply(&tx).unwrap();
    let query = Query {
        node: id(4),
        snapshot: EditorSnapshot {
            revision: 9,
            text: "λx".into(),
            selection: EditorSelection { anchor: 3, head: 3 },
            composition: None,
            focused: true,
        },
    };
    let toggle = PickerEvent::SelectionRequested(Request::Toggle("a".into()), Some(query.clone()));
    let send = |s: &Session, event| s.choice_picker_event(window, id(0), handler(20), 1, event);
    assert!(send(&session, toggle.clone()).is_some());
    for request in [
        Request::Select("a".into()),
        Request::Toggle("b".into()),
        Request::Toggle("missing".into()),
    ] {
        assert!(
            send(
                &session,
                PickerEvent::SelectionRequested(request, Some(query.clone()))
            )
            .is_none()
        );
    }
    assert!(
        send(
            &session,
            PickerEvent::SelectionRequested(Request::Clear, None)
        )
        .is_none()
    );
    let mut composing = query.clone();
    composing.snapshot.composition = Some(EditorSelection { anchor: 0, head: 2 });
    assert!(
        send(
            &session,
            PickerEvent::SelectionRequested(Request::Clear, Some(composing.clone()))
        )
        .is_none()
    );
    assert!(send(&session, PickerEvent::QueryChanged(composing)).is_some());
    assert!(
        session
            .choice_picker_event(window, id(0), handler(20), -1, toggle.clone())
            .is_none()
    );
    assert!(
        session
            .choice_picker_event(window, id(0), handler(20), 2, toggle.clone())
            .is_none()
    );
    // A newer committed selection does not discard a second intent from the same frame.
    p.config.selected = Selection::Multiple(vec![]);
    let update = transaction(
        session.tree(window).unwrap(),
        vec![Op::SetChoicePicker(id(0), Box::new(p.clone()))],
    );
    session.apply(&update).unwrap();
    let first = send(&session, toggle.clone()).unwrap();
    let second = send(&session, toggle.clone()).unwrap();
    assert_eq!(first, second);
    // An unrelated footer commit preserves the query lease and older tree revision.
    let update = transaction(
        session.tree(window).unwrap(),
        vec![Op::SetText(id(8), "New footer".into())],
    );
    session.apply(&update).unwrap();
    assert!(send(&session, toggle.clone()).is_some());
    let editor = session
        .tree(window)
        .unwrap()
        .get(id(4))
        .unwrap()
        .editor
        .as_deref()
        .unwrap()
        .clone();
    let replacement = NodeId::from_parts(4, 2).unwrap();
    let update = transaction(
        session.tree(window).unwrap(),
        vec![
            Op::Remove(id(4)),
            Op::Create(
                replacement,
                Kind::Input,
                "same revision can recur".into(),
                Some(handler(30)),
            ),
            Op::SetEditor(replacement, editor.clone()),
            Op::Splice(id(3), 0, 1, vec![replacement]),
        ],
    );
    session.apply(&update).unwrap();
    assert!(send(&session, toggle).is_none());
    assert!(send(&session, PickerEvent::QueryChanged(query.clone())).is_none());
    let current = Query {
        node: replacement,
        ..query
    };
    let toggle =
        PickerEvent::SelectionRequested(Request::Toggle("a".into()), Some(current.clone()));
    assert!(send(&session, toggle.clone()).is_some());
    p.config.disabled = true;
    let mut disabled_editor = editor;
    disabled_editor.disabled = true;
    let update = transaction(
        session.tree(window).unwrap(),
        vec![
            Op::SetChoicePicker(id(0), Box::new(p)),
            Op::SetEditor(replacement, disabled_editor),
        ],
    );
    session.apply(&update).unwrap();
    assert!(send(&session, toggle).is_none());
    assert!(
        send(
            &session,
            PickerEvent::OpenRequested(true, OpenReason::Trigger)
        )
        .is_none()
    );
    assert!(
        send(
            &session,
            PickerEvent::Visibility(Visibility::Snapshot(true))
        )
        .is_none()
    );
    let closed = PickerEvent::Visibility(Visibility::Changed(false, VisibilityReason::Unavailable));
    assert!(send(&session, closed.clone()).is_some());
    // Read-only observations remain deliverable as disabling closes the surface.
    assert!(send(&session, PickerEvent::QueryChanged(current)).is_some());
    let update = transaction(
        session.tree(window).unwrap(),
        vec![Op::Bind(id(0), Some(handler(21)))],
    );
    session.apply(&update).unwrap();
    assert!(send(&session, closed.clone()).is_none());
    assert!(
        session
            .choice_picker_event(window, id(0), handler(21), 1, closed.clone())
            .is_some()
    );
    assert!(session.overload(window));
    assert!(
        session
            .choice_picker_event(window, id(0), handler(21), 1, closed.clone())
            .is_none()
    );
    session.close(window).unwrap();
    assert!(
        session
            .choice_picker_event(window, id(0), handler(21), 1, closed)
            .is_none()
    );
}

#[test]
fn public_ocaml_picker_transaction_admits_real_slot_and_query_configuration() {
    // Frozen bytes from the public Core View/Reconciler expect test, not a
    // hand-built native transaction or an encoder/decoder round trip.
    let hex = include_str!("../../../test/fixtures/choice-picker-public-view.hex").trim();
    let bytes: Vec<_> = (0..hex.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&hex[i..i + 2], 16).unwrap())
        .collect();
    let Message::Apply(tx) = gpuio_protocol::decode(&bytes).unwrap() else {
        panic!("public picker transaction");
    };
    let mut tree = Tree::new(tx.window);
    tree.apply(&tx)
        .expect("public OCaml View meets native admission contract");
    let root = tree.get(tree.root().unwrap()).unwrap();
    assert_eq!(root.kind, Kind::ChoicePicker);
    let p = root.choice_picker.as_ref().unwrap();
    assert_eq!(p.config.search, Search::Substring);
    assert_eq!(p.config.selected, Selection::Multiple(vec![]));
    let slots = admission::children(p, &root.children, |id| tree.get(id)).unwrap();
    let query = tree.get(slots.query.unwrap()).unwrap();
    assert_eq!(query.text.as_ref(), "seed");
    assert!(!query.editor.as_ref().unwrap().submit_on_enter);
    assert_eq!(tree.get(slots.footer.unwrap()).unwrap().kind, Kind::Button);
    assert_eq!(slots.options.len(), 1);
    assert!(slots.trigger.is_some());
}

#[test]
fn shared_group_metadata_has_bounded_label_cost_and_atomic_admission() {
    let mut p = presentation();
    p.slots.clear();
    p.config.search = Search::None;
    p.config.selected = Selection::Single(None);
    p.config.options = Collection::Grouped(vec![Group {
        id: "section".into(),
        label: "G".into(),
        items: (0..100)
            .map(|i| Item {
                id: format!("item{i}"),
                label: "Option".into(),
                disabled: false,
            })
            .collect(),
    }]);
    assert!(p.has_valid_shape());
    let window = WindowId::from_parts(0, 1).unwrap();
    let mut tree = Tree::new(window);
    tree.apply(&Transaction {
        window,
        base: 0,
        revision: 1,
        operations: vec![
            Op::Create(id(0), Kind::ChoicePicker, "".into(), Some(handler(0))),
            Op::SetChoicePicker(id(0), Box::new(p.clone())),
            Op::SetRoot(Some(id(0))),
        ],
    })
    .unwrap();
    let before = tree.retained_bytes();
    let Collection::Grouped(groups) = &mut p.config.options else {
        unreachable!()
    };
    groups[0].label = "G".repeat(1024);
    assert!(p.has_valid_shape());
    let update = Transaction {
        window,
        base: 1,
        revision: 2,
        operations: vec![Op::SetChoicePicker(id(0), Box::new(p))],
    };
    // Catalog bytes alone are insufficient: frame/AX metadata is reserved too.
    // Sharing group metadata prevents multiplying the label by 100 options.
    assert_eq!(
        tree.apply_with_budget(&update, before + 1023),
        Err(ErrorCode::LimitExceeded)
    );
    assert_eq!(tree.revision(), 1);
    assert_eq!(tree.retained_bytes(), before);
    tree.apply_with_budget(&update, before + 8192).unwrap();
    assert!(tree.retained_bytes() > before + 1023);
    assert!(tree.retained_bytes() <= before + 8192);
    tree.apply(&Transaction {
        window,
        base: 2,
        revision: 3,
        operations: vec![Op::SetRoot(None), Op::Remove(id(0))],
    })
    .unwrap();
    assert_eq!(tree.retained_bytes(), 0);
}

#[test]
fn picker_query_owns_escape_and_rejects_descendant_clear_override() {
    let (_, _, mut tree) = mounted(true);
    let before = tree.get(id(4)).unwrap().clone();
    let retained = tree.retained_bytes();
    let tx = transaction(&tree, vec![Op::SetEditorClearOnEscape(id(4), true)]);
    assert_eq!(tree.apply(&tx), Err(ErrorCode::InvalidTree));
    assert_eq!(tree.revision(), 1);
    assert_eq!(tree.get(id(4)).unwrap(), &before);
    assert_eq!(tree.retained_bytes(), retained);
}
