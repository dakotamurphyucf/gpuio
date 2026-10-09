use gpuio_native::session::Session;
use gpuio_protocol::{
    HandlerId, NodeId, WindowId,
    calendar_content::{Config, Item, Slot},
    calendar_input as c,
    v1::*,
};
fn id(n: i64) -> NodeId {
    NodeId::from_parts(n, 1).unwrap()
}
fn window() -> WindowId {
    WindowId::from_parts(0, 1).unwrap()
}
fn apply(s: &mut Session, operations: Vec<Op>) -> Result<(), ErrorCode> {
    let base = s.tree(window()).unwrap().revision();
    s.apply(&Transaction {
        window: window(),
        base,
        revision: base + 1,
        operations,
    })
    .map(|_| ())
}
fn metadata(description: &str) -> Config {
    Config {
        items: vec![Item {
            slot: Slot::Next,
            description: Some(description.into()),
        }],
    }
}
#[test]
fn final_slot_shape_passivity_and_heap_accounting_are_atomic() {
    let mut s = Session::default();
    s.hello(VERSION, CAPABILITIES).unwrap();
    s.open(1, window(), "Calendar", 400., 400.).unwrap();
    let config = c::Config {
        mode: c::Mode::Single,
        constraints: c::Constraints::unrestricted(),
        first_weekday: 1,
        labels: c::Labels::english(),
        today: None,
        label: "Dates".into(),
        disabled: false,
        read_only: false,
        auto_focus: false,
    };
    apply(
        &mut s,
        vec![
            Op::Create(
                id(0),
                Kind::Calendar,
                "".into(),
                Some(HandlerId::from_parts(0, 1).unwrap()),
            ),
            Op::SetCalendar(
                id(0),
                Box::new(config),
                c::Selection::Empty,
                c::Month::new(2024, 2).unwrap(),
            ),
            Op::SetRoot(Some(id(0))),
        ],
    )
    .unwrap();
    let empty = s.retained_bytes();
    apply(
        &mut s,
        vec![
            Op::SetCalendarContent(id(0), Some(metadata("Next month"))),
            Op::Create(id(1), Kind::Container, "".into(), None),
            Op::Create(id(2), Kind::Text, "Rich next".into(), None),
            Op::Splice(id(1), 0, 0, vec![id(2)]),
            Op::Splice(id(0), 0, 0, vec![id(1)]),
        ],
    )
    .unwrap();
    let populated = s.retained_bytes();
    assert!(populated > empty);
    apply(
        &mut s,
        vec![Op::SetCalendarContent(
            id(0),
            Some(metadata(&"x".repeat(100))),
        )],
    )
    .unwrap();
    assert_eq!(s.retained_bytes() - populated, 100 - "Next month".len());
    let bytes = s.retained_bytes();
    for operations in [
        vec![Op::SetCalendarContent(id(2), Some(metadata("wrong kind")))],
        vec![Op::SetCalendarContent(id(0), None)],
        vec![Op::Splice(id(1), 0, 1, vec![])],
        vec![Op::Bind(id(2), Some(HandlerId::from_parts(1, 1).unwrap()))],
        vec![Op::SetStyle(
            id(2),
            vec![Style::Fields(vec![Field::UserSelect(true)])],
        )],
        vec![Op::SetStyle(id(1), vec![Style::Width(Length::Px(10.))])],
        vec![
            Op::Create(
                id(3),
                Kind::Button,
                "Nested".into(),
                Some(HandlerId::from_parts(1, 1).unwrap()),
            ),
            Op::Splice(id(1), 0, 1, vec![id(3)]),
            Op::Remove(id(2)),
        ],
        vec![Op::SetCalendarContent(
            id(0),
            Some(Config {
                items: vec![
                    Item {
                        slot: Slot::Next,
                        description: None,
                    },
                    Item {
                        slot: Slot::Next,
                        description: None,
                    },
                ],
            }),
        )],
    ] {
        let revision = s.tree(window()).unwrap().revision();
        assert_eq!(apply(&mut s, operations), Err(ErrorCode::InvalidTree));
        assert_eq!(s.tree(window()).unwrap().revision(), revision);
        assert_eq!(s.retained_bytes(), bytes);
    }
    apply(
        &mut s,
        vec![
            Op::SetCalendarContent(id(0), None),
            Op::Splice(id(0), 0, 1, vec![]),
            Op::Remove(id(2)),
            Op::Remove(id(1)),
        ],
    )
    .unwrap();
    assert_eq!(s.retained_bytes(), empty);
}

// The same bytes are asserted against View.Calendar_content + Reconciler in
// OCaml. A hand-authored Rust transaction missed an empty Fields declaration
// on the public constructor's structural wrappers and let the gallery crash.
#[test]
fn public_ocaml_calendar_content_is_admitted_by_the_native_tree() {
    let hex = include_str!("../../../test/fixtures/calendar-content-public.hex").trim();
    let bytes: Vec<_> = (0..hex.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&hex[i..i + 2], 16).unwrap())
        .collect();
    let Message::Apply(tx) = gpuio_protocol::decode(&bytes).unwrap() else {
        panic!("expected public calendar transaction");
    };
    let mut s = Session::default();
    s.hello(VERSION, CAPABILITIES).unwrap();
    s.open(1, tx.window, "Public calendar", 400., 400.).unwrap();
    s.apply(&tx).unwrap();
    let tree = s.tree(tx.window).unwrap();
    let calendar = tree.get(tree.root().unwrap()).unwrap();
    let slots: Vec<_> = calendar
        .children
        .iter()
        .map(|id| tree.get(*id).unwrap())
        .collect();
    assert_eq!(slots.len(), 2);
    assert_eq!(calendar.calendar_content.as_ref().unwrap().items.len(), 2);
    let labels: Vec<_> = slots
        .iter()
        .map(|slot| {
            assert!(slot.style.is_empty());
            assert_eq!(slot.children.len(), 1);
            tree.get(slot.children[0]).unwrap().text.as_ref()
        })
        .collect();
    assert_eq!(labels, ["Back", "Forward"]);
    s.close(tx.window).unwrap();
    assert_eq!(s.retained_bytes(), 0);
}
