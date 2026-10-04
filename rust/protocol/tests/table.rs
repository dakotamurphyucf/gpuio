use binprot::BinProtWrite;
use gpuio_protocol::{
    decode_table_cell, decode_table_command, decode_table_config, decode_table_request, table::*,
};

fn column(id: &str, pin: Pin) -> Column {
    Column {
        id: id.into(),
        label: "Name".into(),
        width: 160.,
        min_width: 40.,
        max_width: 4096.,
        pin,
        alignment: Alignment::Left,
        resizable: true,
        movable: true,
        sortable: true,
    }
}
fn config() -> Config {
    let mut value = column("β", Pin::Unpinned);
    value.label = "Value".into();
    value.alignment = Alignment::Right;
    Config {
        schema_revision: 7,
        query_generation: 3,
        schema: Schema {
            columns: vec![column("a", Pin::Left), value],
            headers: vec![vec![
                Group {
                    label: "Identity".into(),
                    columns: vec!["a".into()],
                },
                Group {
                    label: "Payload".into(),
                    columns: vec!["β".into()],
                },
            ]],
        },
        sort: Some(Sort {
            column: "β".into(),
            direction: Direction::Descending,
        }),
        row_height: 32.,
        overscan: 64.,
        max_active_rows: 32,
        max_active_cells: 64,
        selection_mode: SelectionMode::RowsAndCells,
        column_selection: true,
        disabled: false,
        scrollbar: true,
        label: "Results".into(),
    }
}
fn bytes(value: &impl BinProtWrite) -> Vec<u8> {
    let mut bytes = vec![];
    value.binprot_write(&mut bytes).unwrap();
    bytes
}
fn fixture(value: &impl BinProtWrite, expected: &str) {
    let hex: String = bytes(value)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect();
    assert_eq!(hex, expected.trim());
}
#[test]
fn independent_ocaml_rust_bytes_and_strict_decoding() {
    let config = config();
    let cell = Cell {
        column: "β".into(),
        copy_text: "日本語👨‍👩‍👧‍👦".into(),
    };
    let command = Command {
        serial: 9,
        query_generation: 3,
        target: Target::SetSelection(Selection::Cell(42, "β".into())),
    };
    let request = Request::Resize(vec![("a".into(), 160.), ("β".into(), 240.)]);
    fixture(
        &config,
        include_str!("../../../test/fixtures/table-config.hex"),
    );
    fixture(&cell, include_str!("../../../test/fixtures/table-cell.hex"));
    fixture(
        &command,
        include_str!("../../../test/fixtures/table-command.hex"),
    );
    fixture(
        &request,
        include_str!("../../../test/fixtures/table-request.hex"),
    );
    assert_eq!(decode_table_config(&bytes(&config)), Ok(config.clone()));
    assert_eq!(decode_table_cell(&bytes(&cell)), Ok(cell.clone()));
    assert_eq!(decode_table_command(&bytes(&command)), Ok(command.clone()));
    assert_eq!(decode_table_request(&bytes(&request)), Ok(request.clone()));
    for (encoded, decode) in [
        (
            bytes(&config),
            (|input: &[u8]| decode_table_config(input).is_ok()) as fn(&[u8]) -> bool,
        ),
        (
            bytes(&cell),
            (|input: &[u8]| decode_table_cell(input).is_ok()) as fn(&[u8]) -> bool,
        ),
        (
            bytes(&command),
            (|input: &[u8]| decode_table_command(input).is_ok()) as fn(&[u8]) -> bool,
        ),
        (
            bytes(&request),
            (|input: &[u8]| decode_table_request(input).is_ok()) as fn(&[u8]) -> bool,
        ),
    ] {
        for end in 0..encoded.len() {
            assert!(!decode(&encoded[..end]), "truncation {end}");
        }
        let mut trailing = encoded;
        trailing.push(0);
        assert!(!decode(&trailing));
    }
    assert!(decode_table_request(&[7]).is_err());
    assert!(decode_table_command(&[1, 0, 6]).is_err());
    assert!(
        decode_table_config(&[1, 0, 0xfe, 0xff, 0xff]).is_err(),
        "count bounded before allocation"
    );
    assert!(
        decode_table_cell(&[1, b'a', 1, 0xff]).is_err(),
        "invalid UTF8"
    );
}
#[test]
fn bounded_schema_membership_and_cell_product() {
    let base = config();
    assert!(base.is_valid());
    let invalid = |value: Config| {
        assert!(!value.is_valid());
        assert!(decode_table_config(&bytes(&value)).is_err());
    };
    invalid(Config {
        schema_revision: 0,
        ..base.clone()
    });
    invalid(Config {
        query_generation: -1,
        ..base.clone()
    });
    invalid(Config {
        max_active_rows: i64::MAX,
        ..base.clone()
    });
    invalid(Config {
        max_active_cells: 63,
        ..base.clone()
    });
    invalid(Config {
        row_height: f64::NAN,
        ..base.clone()
    });
    invalid(Config {
        label: "\0".into(),
        ..base.clone()
    });
    let mut value = base.clone();
    value.schema.columns[1].id = "a".into();
    invalid(value);
    let mut value = base.clone();
    value.schema.columns[1].sortable = false;
    invalid(value);
    let mut value = base.clone();
    value.schema.headers[0][0].columns.push("β".into());
    value.schema.headers[0].pop();
    invalid(value);
    let mut value = base.clone();
    value.schema.columns.reverse();
    value.schema.headers.clear();
    invalid(value);
    let mut value = base.clone();
    value.schema.headers.push(vec![]);
    invalid(value);
    let mut maximum = base.clone();
    maximum.schema = Schema {
        columns: (0..64)
            .map(|i| column(&format!("c{i}"), Pin::Unpinned))
            .collect(),
        headers: vec![],
    };
    maximum.sort = None;
    maximum.max_active_rows = 256;
    maximum.max_active_cells = 16384;
    assert_eq!(decode_table_config(&bytes(&maximum)), Ok(maximum.clone()));
    invalid(Config {
        max_active_rows: 257,
        ..maximum.clone()
    });
    invalid(Config {
        max_active_cells: 16385,
        ..maximum.clone()
    });
    let mut oversized = maximum.clone();
    oversized.schema.columns.push(column("65", Pin::Unpinned));
    invalid(oversized);
    let mut oversized = maximum.clone();
    for column in &mut oversized.schema.columns {
        column.label = "a".repeat(4096);
    }
    invalid(oversized);
    let mut nested = maximum;
    nested.schema.columns.truncate(4);
    let group = |ids: &[&str]| Group {
        label: "group".into(),
        columns: ids.iter().map(|id| (*id).into()).collect(),
    };
    nested.schema.headers = vec![
        vec![group(&["c0", "c1"]), group(&["c2", "c3"])],
        vec![
            group(&["c0"]),
            group(&["c1"]),
            group(&["c2"]),
            group(&["c3"]),
        ],
    ];
    assert!(nested.is_valid());
    nested.schema.headers.reverse();
    invalid(nested);
}
#[test]
fn every_command_request_and_text_limit() {
    let selections = [
        Selection::Empty,
        Selection::Row(1),
        Selection::Column("β".into()),
        Selection::Cell(42, "β".into()),
    ];
    let mut targets = vec![
        Target::Reveal(42, None),
        Target::Reveal(42, Some("β".into())),
        Target::ScrollTo(42, 7.5),
        Target::ScrollToColumn("β".into()),
        Target::ScrollToEnd,
        Target::ResetColumns,
    ];
    targets.extend(selections.iter().cloned().map(Target::SetSelection));
    let commands: Vec<_> = targets
        .iter()
        .cloned()
        .map(|target| Command {
            serial: 1,
            query_generation: 0,
            target,
        })
        .collect();
    for target in targets {
        let command = Command {
            serial: 1,
            query_generation: 0,
            target,
        };
        assert_eq!(decode_table_command(&bytes(&command)), Ok(command));
    }
    let mut requests = vec![
        Request::Activate(42, None),
        Request::Activate(42, Some("β".into())),
        Request::Move("β".into(), None),
        Request::Move("β".into(), Some("a".into())),
        Request::Sort("β".into(), None),
        Request::Sort("β".into(), Some(Direction::Ascending)),
        Request::Sort("β".into(), Some(Direction::Descending)),
    ];
    for selection in selections {
        requests.extend([
            Request::Select(selection.clone()),
            Request::Context(selection.clone()),
            Request::Copy(selection),
        ]);
    }
    fixture(
        &(commands, requests.clone()),
        include_str!("../../../test/fixtures/table-intents.hex"),
    );
    for request in requests {
        assert_eq!(decode_table_request(&bytes(&request)), Ok(request));
    }
    for selection in [
        Selection::Row(0),
        Selection::Cell(-1, "a".into()),
        Selection::Column("".into()),
    ] {
        assert!(decode_table_request(&bytes(&Request::Select(selection))).is_err());
    }
    for request in [
        Request::Resize(vec![]),
        Request::Resize(vec![("a".into(), 40.), ("a".into(), 50.)]),
        Request::Resize(vec![("a".into(), f64::INFINITY)]),
        Request::Move("a".into(), Some("a".into())),
    ] {
        assert!(decode_table_request(&bytes(&request)).is_err());
    }
    for text in ["".to_string(), "x".repeat(MAX_COPY_BYTES)] {
        let cell = Cell {
            column: "a".into(),
            copy_text: text,
        };
        assert_eq!(decode_table_cell(&bytes(&cell)), Ok(cell));
    }
    for text in ["\0".into(), "x".repeat(MAX_COPY_BYTES + 1)] {
        assert!(
            decode_table_cell(&bytes(&Cell {
                column: "a".into(),
                copy_text: text
            }))
            .is_err()
        );
    }
}

#[test]
fn transaction_and_event_envelopes_match_ocaml() {
    use gpuio_protocol::{HandlerId, NodeId, WindowId, v1::*};
    let window = WindowId::from_parts(0, 1).unwrap();
    let node = |slot| NodeId::from_parts(slot, 1).unwrap();
    let message = Message::Apply(Transaction {
        window,
        base: 0,
        revision: 1,
        operations: vec![
            Op::SetTable(node(0), config()),
            Op::SetTableCell(
                node(1),
                Cell {
                    column: "β".into(),
                    copy_text: "日本語👨‍👩‍👧‍👦".into(),
                },
            ),
            Op::TableCommand(
                node(0),
                Command {
                    serial: 9,
                    query_generation: 3,
                    target: Target::SetSelection(Selection::Cell(42, "β".into())),
                },
            ),
        ],
    });
    fixture(
        &message,
        include_str!("../../../test/fixtures/table-transaction.hex"),
    );
    let encoded = bytes(&message);
    assert_eq!(gpuio_protocol::decode(&encoded), Ok(message));
    for end in 0..encoded.len() {
        assert!(gpuio_protocol::decode(&encoded[..end]).is_err());
    }
    let event = Event::TableInput(
        window,
        node(0),
        HandlerId::from_parts(0, 1).unwrap(),
        1,
        Input {
            schema_revision: 7,
            query_generation: 3,
            request: Request::Resize(vec![("a".into(), 160.), ("β".into(), 240.)]),
        },
    );
    fixture(
        &event,
        include_str!("../../../test/fixtures/table-event.hex"),
    );
}

#[test]
fn column_moves_preserve_group_membership_and_input_policy() {
    let columns = ["a", "b", "c", "d"]
        .into_iter()
        .map(|id| column(id, Pin::Unpinned))
        .collect();
    let schema = Schema {
        columns,
        headers: vec![vec![
            Group {
                label: "First".into(),
                columns: vec!["a".into(), "b".into()],
            },
            Group {
                label: "Second".into(),
                columns: vec!["c".into(), "d".into()],
            },
        ]],
    };
    let moved = schema.moved("b", Some("a")).unwrap();
    assert_eq!(
        moved
            .columns
            .iter()
            .map(|c| c.id.as_str())
            .collect::<Vec<_>>(),
        ["b", "a", "c", "d"]
    );
    assert_eq!(moved.headers[0][0].columns, ["b", "a"]);
    assert!(schema.moved("a", Some("d")).is_none());
    assert!(schema.moved("c", Some("b")).is_none());
    assert!(schema.moved("a", Some("a")).is_none());
    let mut pinned = schema.clone();
    pinned.columns[0].pin = Pin::Left;
    pinned.headers.clear();
    assert!(pinned.moved("a", None).is_none());
    assert!(pinned.moved("b", Some("a")).is_none());
    let mut locked = schema;
    locked.columns[0].movable = false;
    assert!(locked.moved("a", Some("b")).is_none());
    let mut config = config();
    config.schema.columns[0].resizable = false;
    assert!(config.allows_request(&Request::Resize(vec![("a".into(), 160.)]), |_| true));
    assert!(!config.allows_request(&Request::Resize(vec![("a".into(), 161.)]), |_| true));
    config.selection_mode = SelectionMode::Rows;
    assert!(!config.allows_request(&Request::Select(Selection::Cell(1, "β".into())), |_| true));
    assert!(config.allows_request(&Request::Select(Selection::Row(1)), |_| true));
    config.column_selection = false;
    assert!(!config.allows_request(&Request::Select(Selection::Column("β".into())), |_| true));
}

#[test]
fn managed_tables_require_a_distinct_negotiated_capability() {
    use gpuio_protocol::v1::*;
    assert_eq!(CAPABILITIES & CAP_MANAGED_TABLES, 1_i64 << 40);
    assert_eq!(
        CAP_MANAGED_TABLES & (CAP_TREE | CAP_MANAGED_TREES | CAP_VIRTUAL_LISTS),
        0
    );
    let hello = Message::Hello(VERSION, CAP_MANAGED_TABLES);
    let mut bytes = Vec::new();
    hello.binprot_write(&mut bytes).unwrap();
    assert_eq!(
        bytes
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect::<String>(),
        "0003fc0000000000010000"
    );
    assert_eq!(gpuio_protocol::decode(&bytes), Ok(hello));
}

#[test]
fn behavior_extension_has_independent_bytes_and_bounded_decoding() {
    use gpuio_protocol::{NodeId, WindowId, v1::*};
    let node = NodeId::from_parts(0, 1).unwrap();
    let behavior = Behavior {
        row_header: false,
        boundary: Boundary::Stop,
        selectable_headers: Some(vec!["a".into(), "β".into()]),
    };
    let op = Op::SetTableBehavior(node, Some(behavior.clone()));
    fixture(
        &op,
        include_str!("../../../test/fixtures/table-behavior-operation.hex"),
    );
    fixture(
        &Op::SetTableBehavior(node, None),
        include_str!("../../../test/fixtures/table-behavior-clear.hex"),
    );
    let message = |op| {
        Message::Apply(Transaction {
            window: WindowId::from_parts(0, 1).unwrap(),
            base: 0,
            revision: 1,
            operations: vec![op],
        })
    };
    let valid = message(op);
    let encoded = bytes(&valid);
    assert_eq!(gpuio_protocol::decode(&encoded), Ok(valid));
    for end in 0..encoded.len() {
        assert!(gpuio_protocol::decode(&encoded[..end]).is_err());
    }
    let mut trailing = encoded.clone();
    trailing.push(0);
    assert!(gpuio_protocol::decode(&trailing).is_err());
    for ids in [
        vec!["a".into(), "a".into()],
        vec!["".into()],
        vec!["x".repeat(257)],
        (0..65).map(|i| i.to_string()).collect(),
    ] {
        let invalid = Behavior {
            selectable_headers: Some(ids),
            ..behavior.clone()
        };
        assert!(!invalid.is_valid());
        assert!(
            gpuio_protocol::decode(&bytes(&message(Op::SetTableBehavior(node, Some(invalid)))))
                .is_err()
        );
    }
    // Invalid boundary discriminant, independently of the typed writer.
    let mut invalid = encoded;
    let boundary = invalid.len() - bytes(&behavior).len() + 1;
    invalid[boundary] = 2;
    assert!(gpuio_protocol::decode(&invalid).is_err());
}

#[test]
fn appearance_extension_has_paired_bytes_and_strict_bounds() {
    use gpuio_protocol::{NodeId, WindowId, v1::*};
    let node = NodeId::from_parts(0, 1).unwrap();
    let padding = Padding {
        top: 1.,
        right: 2.,
        bottom: 3.,
        left: 4.,
    };
    let appearance = Appearance {
        striped: true,
        colors: vec![
            (Part::HeaderBackground, 0x33669980),
            (Part::HeaderForeground, 0xffffffff),
        ],
        padding: Some(padding),
        column_padding: vec![(
            "name".into(),
            Padding {
                top: 0.,
                right: 0.,
                bottom: 0.,
                left: 0.,
            },
        )],
    };
    fixture(
        &Op::SetTableAppearance(node, Some(appearance.clone())),
        include_str!("../../../test/fixtures/table-appearance-operation.hex"),
    );
    fixture(
        &Op::SetTableAppearance(node, None),
        include_str!("../../../test/fixtures/table-appearance-clear.hex"),
    );
    let message = |a| {
        Message::Apply(Transaction {
            window: WindowId::from_parts(0, 1).unwrap(),
            base: 0,
            revision: 1,
            operations: vec![Op::SetTableAppearance(node, Some(a))],
        })
    };
    let valid = message(appearance.clone());
    let encoded = bytes(&valid);
    assert_eq!(gpuio_protocol::decode(&encoded), Ok(valid));
    for end in 0..encoded.len() {
        assert!(gpuio_protocol::decode(&encoded[..end]).is_err());
    }
    let mut trailing = encoded;
    trailing.push(0);
    assert!(gpuio_protocol::decode(&trailing).is_err());
    for invalid in [
        Appearance {
            colors: vec![(Part::RowBorder, -1)],
            ..appearance.clone()
        },
        Appearance {
            colors: vec![(Part::RowBorder, 0x1_0000_0000)],
            ..appearance.clone()
        },
        Appearance {
            colors: vec![(Part::RowBorder, 0); 2],
            ..appearance.clone()
        },
        Appearance {
            column_padding: vec![("name".into(), padding); 2],
            ..appearance.clone()
        },
        Appearance {
            column_padding: (0..65).map(|i| (i.to_string(), padding)).collect(),
            ..appearance.clone()
        },
        Appearance {
            padding: Some(Padding {
                top: f64::NAN,
                ..padding
            }),
            ..appearance.clone()
        },
        Appearance {
            padding: Some(Padding {
                left: 4097.,
                ..padding
            }),
            ..appearance.clone()
        },
    ] {
        assert!(!invalid.is_valid());
        assert!(gpuio_protocol::decode(&bytes(&message(invalid))).is_err());
    }
    let mut invalid = bytes(&message(Appearance {
        colors: vec![(Part::RowBorder, 0)],
        ..Appearance::default()
    }));
    // The first color part follows the outer envelope, op/node/option, stripe and count.
    let part = invalid.len() - 4;
    assert_eq!(invalid[part], 6);
    invalid[part] = 13;
    assert!(gpuio_protocol::decode(&invalid).is_err());
}

#[test]
fn column_viewport_has_independent_event_bytes_and_bounded_schema_identity() {
    use gpuio_protocol::{HandlerId, NodeId, WindowId, v1::Event};
    let viewport = ColumnViewport {
        schema_revision: 7,
        query_generation: 3,
        columns: vec![
            ("a".into(), Pin::Left, true),
            ("β".into(), Pin::Unpinned, false),
        ],
    };
    fixture(
        &Event::TableColumnsObserved(
            WindowId::from_parts(0, 1).unwrap(),
            NodeId::from_parts(0, 1).unwrap(),
            HandlerId::from_parts(0, 1).unwrap(),
            1,
            viewport.clone(),
        ),
        include_str!("../../../test/fixtures/table-columns-event.hex"),
    );
    assert!(viewport.matches_schema(&config()));
    for invalid in [
        ColumnViewport {
            schema_revision: 0,
            ..viewport.clone()
        },
        ColumnViewport {
            query_generation: -1,
            ..viewport.clone()
        },
        ColumnViewport {
            columns: vec![("".into(), Pin::Left, true)],
            ..viewport.clone()
        },
        ColumnViewport {
            columns: vec![("a".into(), Pin::Left, true); 2],
            ..viewport.clone()
        },
        ColumnViewport {
            columns: viewport.columns.iter().cloned().rev().collect(),
            ..viewport.clone()
        },
        ColumnViewport {
            columns: (0..65)
                .map(|i| (i.to_string(), Pin::Unpinned, true))
                .collect(),
            ..viewport.clone()
        },
    ] {
        assert!(!invalid.is_valid());
    }
    for columns in [
        vec![("missing".into(), Pin::Left, true)],
        vec![("a".into(), Pin::Unpinned, true)],
    ] {
        assert!(
            !ColumnViewport {
                columns,
                ..viewport.clone()
            }
            .matches_schema(&config())
        );
    }
    assert!(
        ColumnViewport {
            columns: vec![],
            ..viewport
        }
        .matches_schema(&config())
    );
}
