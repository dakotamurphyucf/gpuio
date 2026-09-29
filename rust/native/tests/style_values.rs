use gpuio_native::session::Session;
use gpuio_protocol::{NodeId, WindowId, v1::*};

#[test]
fn finite_grid_and_text_choices_have_stable_ocaml_field_bytes() {
    for (field, bytes) in [
        (Field::GridColumnMinimum(0), [0x0f, 0]),
        (Field::GridColumnMinimum(1), [0x0f, 1]),
        (Field::GridColumnMinimum(2), [0x0f, 2]),
        (Field::GridRowMinimum(0), [0x10, 0]),
        (Field::GridRowMinimum(1), [0x10, 1]),
        (Field::GridRowMinimum(2), [0x10, 2]),
        (Field::WhiteSpace(0), [0x36, 0]),
        (Field::WhiteSpace(1), [0x36, 1]),
        (Field::TextDecoration(0), [0x39, 0]),
        (Field::TextDecoration(1), [0x39, 1]),
        (Field::TextDecoration(2), [0x39, 2]),
        (Field::TextDecoration(3), [0x39, 3]),
    ] {
        let mut encoded = Vec::new();
        binprot::BinProtWrite::binprot_write(&field, &mut encoded).unwrap();
        assert_eq!(encoded, bytes);
    }
}

#[test]
fn extended_style_values_validate_before_atomic_publication() {
    assert_eq!(CAPABILITIES & CAP_STYLE_VALUES, 1_i64 << 44);
    let window = WindowId::from_parts(0, 1).unwrap();
    let node = NodeId::from_parts(0, 1).unwrap();
    let mut session = Session::default();
    session.hello(VERSION, CAPABILITIES).unwrap();
    session.open(1, window, "style values", 400., 300.).unwrap();
    session
        .apply(&Transaction {
            window,
            base: 0,
            revision: 1,
            operations: vec![
                Op::Create(node, Kind::Text, "original".into(), None),
                Op::SetRoot(Some(node)),
            ],
        })
        .unwrap();
    for value in 0..=21 {
        let base = session.tree(window).unwrap().revision();
        session
            .apply(&Transaction {
                window,
                base,
                revision: base + 1,
                operations: vec![Op::SetStyle(
                    node,
                    vec![Style::Fields(vec![
                        Field::Cursor(value),
                        Field::TextOverflow(value % 3),
                        Field::GridColumnMinimum(value % 3),
                        Field::GridRowMinimum(value % 3),
                        Field::WhiteSpace(value % 2),
                        Field::TextDecoration(value % 4),
                    ])],
                )],
            })
            .unwrap();
    }
    let base = session.tree(window).unwrap().revision();
    let retained = session.retained_bytes();
    for invalid in [
        Field::Cursor(-1),
        Field::Cursor(22),
        Field::TextOverflow(-1),
        Field::TextOverflow(3),
        Field::GridColumnMinimum(-1),
        Field::GridColumnMinimum(3),
        Field::GridRowMinimum(-1),
        Field::GridRowMinimum(3),
        Field::WhiteSpace(-1),
        Field::WhiteSpace(2),
        Field::TextDecoration(-1),
        Field::TextDecoration(4),
    ] {
        assert_eq!(
            session
                .apply(&Transaction {
                    window,
                    base,
                    revision: base + 1,
                    operations: vec![
                        Op::SetText(node, "must not publish".into()),
                        Op::SetStyle(node, vec![Style::Fields(vec![invalid])]),
                    ]
                })
                .unwrap_err(),
            ErrorCode::Malformed
        );
        assert_eq!(session.tree(window).unwrap().revision(), base);
        assert_eq!(
            session
                .tree(window)
                .unwrap()
                .get(node)
                .unwrap()
                .text
                .as_ref(),
            "original"
        );
        assert_eq!(session.retained_bytes(), retained);
    }
    session.close(window).unwrap();
    assert_eq!(session.retained_bytes(), 0);
}
