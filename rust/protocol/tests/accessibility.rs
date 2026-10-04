use binprot::BinProtWrite;
use gpuio_protocol::{DecodeError, accessibility::*, decode_accessibility};

fn config() -> Config {
    Config {
        role: None,
        label: None,
        description: None,
        live: Live::Off,
        current: None,
        field: Some(Field {
            label: "Email".into(),
            help: Some("Private".into()),
            error: Some("Required".into()),
            required: true,
        }),
    }
}
fn encode(value: &Config) -> Vec<u8> {
    let mut bytes = vec![];
    value.binprot_write(&mut bytes).unwrap();
    bytes
}

#[test]
fn transcript_log_fixture_preserves_tags_and_bounds() {
    use gpuio_protocol::v1::Kind;
    let config = Config {
        role: Some(Role::Log),
        label: Some("Transcript".into()),
        description: None,
        live: Live::Polite,
        field: None,
        current: None,
    };
    let bytes = encode(&config);
    assert_eq!(
        bytes.iter().map(|b| format!("{b:02x}")).collect::<String>(),
        include_str!("../../../test/fixtures/accessibility-log.hex").trim()
    );
    assert_eq!(decode_accessibility(&bytes), Ok(config.clone()));
    for length in 0..bytes.len() {
        assert!(decode_accessibility(&bytes[..length]).is_err());
    }
    let mut invalid = bytes.clone();
    invalid[1] = 17;
    assert!(decode_accessibility(&invalid).is_err());
    let mut trailing = bytes;
    trailing.push(0);
    assert!(decode_accessibility(&trailing).is_err());
    for kind in [Kind::Container, Kind::VirtualList] {
        assert!(config.supports(kind));
    }
    for kind in [Kind::Text, Kind::Button, Kind::Input] {
        assert!(!config.supports(kind));
    }
    for live in [Live::Off, Live::Polite, Live::Assertive] {
        let value = Config {
            live,
            ..config.clone()
        };
        assert_eq!(decode_accessibility(&encode(&value)), Ok(value));
    }
}

fn tree_config(role: Role, label: &str) -> Config {
    Config {
        role: Some(role),
        label: Some(label.into()),
        description: None,
        live: Live::Off,
        field: None,
        current: None,
    }
}

#[test]
fn appended_tree_roles_match_the_ocaml_fixture_and_validate_hierarchy() {
    let branch = TreeItem {
        level: 2,
        index: 4,
        count: None,
        expanded: Some(false),
        selected: true,
        disabled: false,
        busy: true,
    };
    let leaf = TreeItem {
        level: 3,
        index: 1,
        count: Some(2),
        expanded: None,
        selected: false,
        disabled: true,
        busy: false,
    };
    let configs = vec![
        tree_config(Role::Tree(true), "Project"),
        tree_config(Role::TreeItem(branch), "Branch"),
        tree_config(Role::TreeItem(leaf), "Leaf"),
    ];
    let mut bytes = vec![];
    configs.binprot_write(&mut bytes).unwrap();
    assert_eq!(
        bytes.iter().map(|b| format!("{b:02x}")).collect::<String>(),
        include_str!("../../../test/fixtures/accessibility-tree.hex").trim()
    );
    for config in &configs {
        let bytes = encode(config);
        assert_eq!(decode_accessibility(&bytes), Ok(config.clone()));
        for length in 0..bytes.len() {
            assert!(decode_accessibility(&bytes[..length]).is_err());
        }
        let mut extra = bytes;
        extra.push(0);
        assert!(decode_accessibility(&extra).is_err());
    }
    for invalid in [
        TreeItem { level: 0, ..branch },
        TreeItem {
            level: 129,
            ..branch
        },
        TreeItem {
            index: -1,
            ..branch
        },
        TreeItem {
            index: 100_000,
            ..branch
        },
        TreeItem {
            count: Some(4),
            ..branch
        },
        TreeItem {
            count: Some(100_001),
            ..branch
        },
        TreeItem {
            count: Some(-1),
            ..branch
        },
    ] {
        assert!(
            decode_accessibility(&encode(&tree_config(Role::TreeItem(invalid), "Bad"))).is_err()
        );
    }
    let maximum = tree_config(
        Role::TreeItem(TreeItem {
            level: 128,
            index: 99_999,
            count: Some(100_000),
            ..leaf
        }),
        "Maximum",
    );
    assert_eq!(decode_accessibility(&encode(&maximum)), Ok(maximum));
    use gpuio_protocol::v1::Kind;
    assert!(configs[0].supports(Kind::VirtualList));
    assert!(!configs[0].supports(Kind::Container));
    assert!(configs[1].supports(Kind::Container));
    assert!(!configs[1].supports(Kind::Button));
    assert!(!configs[1].supports(Kind::VirtualList));
}
#[test]
fn independent_field_fixture_and_strict_decoder() {
    let value = config();
    let bytes = encode(&value);
    assert_eq!(
        bytes.iter().map(|b| format!("{b:02x}")).collect::<String>(),
        include_str!("../../../test/fixtures/accessibility-field.hex").trim()
    );
    assert_eq!(decode_accessibility(&bytes), Ok(value));
    for length in 0..bytes.len() {
        assert!(decode_accessibility(&bytes[..length]).is_err());
    }
    let mut extra = bytes;
    extra.push(0);
    assert!(decode_accessibility(&extra).is_err());
    assert_eq!(
        decode_accessibility(&vec![0; MAX_CONFIG_BYTES + 1]),
        Err(DecodeError::LimitExceeded)
    );
    // A huge declared string length is rejected before allocating its body.
    assert_eq!(
        decode_accessibility(&[0, 1, 0xfe, 0xff, 0xff]),
        Err(DecodeError::LimitExceeded)
    );
    assert!(decode_accessibility(&[0, 1, 1, 0xff]).is_err());
}
#[test]
fn invalid_combinations_and_text_limits() {
    let mut invalid = config();
    invalid.role = Some(Role::Group);
    assert!(decode_accessibility(&encode(&invalid)).is_err());
    invalid = config();
    invalid.live = Live::Assertive;
    assert!(decode_accessibility(&encode(&invalid)).is_err());
    for text in ["".into(), "x\0y".into(), "x".repeat(MAX_TEXT_BYTES + 1)] {
        let mut invalid = config();
        invalid.field.as_mut().unwrap().label = text;
        assert!(decode_accessibility(&encode(&invalid)).is_err());
    }
    let mut maximum = config();
    let field = maximum.field.as_mut().unwrap();
    field.label = "x".repeat(MAX_TEXT_BYTES);
    field.help = Some(field.label.clone());
    field.error = Some(field.label.clone());
    assert!(encode(&maximum).len() <= MAX_CONFIG_BYTES);
    assert_eq!(decode_accessibility(&encode(&maximum)), Ok(maximum));
    for level in [0, 7, i64::MAX] {
        let invalid = Config {
            role: Some(Role::Heading(level)),
            field: None,
            ..config()
        };
        assert!(decode_accessibility(&encode(&invalid)).is_err());
    }
}
#[test]
fn appended_update_and_reset_roundtrip() {
    use gpuio_protocol::{NodeId, WindowId, decode, v1::*};
    let node = NodeId::from_parts(0, 1).unwrap();
    let message = Message::Apply(Transaction {
        window: WindowId::from_parts(0, 1).unwrap(),
        base: 0,
        revision: 1,
        operations: vec![
            Op::SetAccessibility(node, Some(config())),
            Op::SetAccessibility(node, None),
        ],
    });
    let mut bytes = vec![];
    message.binprot_write(&mut bytes).unwrap();
    assert_eq!(
        bytes.iter().map(|b| format!("{b:02x}")).collect::<String>(),
        include_str!("../../../test/fixtures/accessibility-request.hex").trim()
    );
    assert_eq!(decode(&bytes), Ok(message));
    assert!(config().supports(Kind::Input));
    assert!(!config().supports(Kind::Container));
    let link = Config {
        role: Some(Role::Link),
        field: None,
        ..config()
    };
    assert!(link.supports(Kind::Button));
    assert!(!link.supports(Kind::Input));
}

#[test]
fn semantic_role_tags_match_ocaml() {
    let values = [
        (Role::Status, "Saved", None, Live::Polite),
        (Role::Alert, "Error", None, Live::Assertive),
        (Role::Heading(2), "Account", None, Live::Off),
        (Role::Link, "Open", Some("New window"), Live::Off),
    ]
    .into_iter()
    .map(|(role, label, description, live)| Config {
        role: Some(role),
        label: Some(label.into()),
        description: description.map(str::to_owned),
        live,
        current: None,
        field: None,
    })
    .collect::<Vec<_>>();
    let mut bytes = vec![];
    values.binprot_write(&mut bytes).unwrap();
    assert_eq!(
        bytes.iter().map(|b| format!("{b:02x}")).collect::<String>(),
        include_str!("../../../test/fixtures/accessibility-roles.hex").trim()
    );
    for value in values {
        assert_eq!(decode_accessibility(&encode(&value)), Ok(value));
    }
}

#[test]
fn accepted_presentation_capability() {
    use gpuio_protocol::v1::{CAP_PRESENTATION, CAPABILITIES};
    assert_eq!(CAPABILITIES & CAP_PRESENTATION, 1_i64 << 34);
}

#[test]
fn current_item_fixture_validation_and_placement() {
    let value = Config {
        role: Some(Role::Link),
        label: Some("Inbox".into()),
        description: Some("Current page".into()),
        live: Live::Off,
        field: None,
        current: Some(Current::Page),
    };
    assert_eq!(
        encode(&value)
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect::<String>(),
        include_str!("../../../test/fixtures/accessibility-current.hex").trim()
    );
    use gpuio_protocol::v1::Kind;
    assert!(value.supports(Kind::Button));
    assert!(!value.supports(Kind::Container));
    assert!(!value.supports(Kind::Checkbox));
    for (tag, current) in [
        Current::True,
        Current::Page,
        Current::Step,
        Current::Location,
        Current::Date,
        Current::Time,
    ]
    .into_iter()
    .enumerate()
    {
        let config = Config {
            current: Some(current),
            ..value.clone()
        };
        let bytes = encode(&config);
        assert_eq!(bytes.last().copied(), Some(tag as u8));
        assert_eq!(decode_accessibility(&bytes), Ok(config));
        for length in 0..bytes.len() {
            assert!(decode_accessibility(&bytes[..length]).is_err());
        }
    }
    let mut bad = value.clone();
    bad.description = None;
    assert!(decode_accessibility(&encode(&bad)).is_err());
    let mut bytes = encode(&value);
    *bytes.last_mut().unwrap() = 6;
    assert!(decode_accessibility(&bytes).is_err());
    let mut field = config();
    field.current = Some(Current::True);
    assert!(decode_accessibility(&encode(&field)).is_err());
    let nav = Config {
        role: Some(Role::Navigation),
        current: None,
        ..value
    };
    assert!(nav.supports(Kind::Container));
    assert!(!nav.supports(Kind::Text));
    assert_eq!(encode(&nav)[1], 11);
    assert_eq!(decode_accessibility(&encode(&nav)), Ok(nav));
}

#[test]
fn toolbar_orientations_match_independent_bytes_and_reject_invalid_placement() {
    use gpuio_protocol::v1::Kind;
    let configs = vec![
        tree_config(Role::Toolbar(Orientation::Horizontal), "Row"),
        tree_config(Role::Toolbar(Orientation::Vertical), "Column"),
    ];
    let mut bytes = vec![];
    configs.binprot_write(&mut bytes).unwrap();
    assert_eq!(
        bytes.iter().map(|b| format!("{b:02x}")).collect::<String>(),
        include_str!("../../../test/fixtures/accessibility-toolbar.hex").trim()
    );
    for config in configs {
        let bytes = encode(&config);
        assert_eq!(decode_accessibility(&bytes), Ok(config.clone()));
        for n in 0..bytes.len() {
            assert!(decode_accessibility(&bytes[..n]).is_err());
        }
        let mut invalid = bytes.clone();
        invalid[2] = 2;
        assert_eq!(decode_accessibility(&invalid), Err(DecodeError::Malformed));
        let mut trailing = bytes;
        trailing.push(0);
        assert!(decode_accessibility(&trailing).is_err());
        assert!(config.supports(Kind::Container));
        for kind in [
            Kind::Text,
            Kind::Button,
            Kind::Checkbox,
            Kind::RadioGroup,
            Kind::VirtualList,
            Kind::CommandScope,
        ] {
            assert!(!config.supports(kind));
        }
    }
}

#[test]
fn structural_table_roles_have_paired_coordinates_and_strict_bounds() {
    use gpuio_protocol::v1::Kind;
    let roles = [
        Role::Table(TableInfo {
            rows: Some(3),
            columns: Some(4),
        }),
        Role::RowGroup,
        Role::TableRow(1),
        Role::TableCell(TableCell {
            row: 1,
            column: 0,
            column_span: 2,
        }),
        Role::ColumnHeader(TableCell {
            row: 0,
            column: 0,
            column_span: 4,
        }),
        Role::RowHeader(TableCell {
            row: 1,
            column: 0,
            column_span: 1,
        }),
        Role::Caption,
    ];
    let config = |role| Config {
        role: Some(role),
        label: None,
        description: None,
        live: Live::Off,
        field: None,
        current: None,
    };
    for (role, fixture) in roles
        .into_iter()
        .zip(include_str!("../../../test/fixtures/accessibility-structural-table.hex").lines())
    {
        let value = config(role);
        let bytes = encode(&value);
        assert_eq!(
            bytes.iter().map(|b| format!("{b:02x}")).collect::<String>(),
            fixture
        );
        assert_eq!(decode_accessibility(&bytes), Ok(value.clone()));
        assert!(value.supports(Kind::Container));
        for kind in [Kind::Text, Kind::Button, Kind::VirtualList, Kind::Input] {
            assert!(!value.supports(kind));
        }
        for len in 0..bytes.len() {
            assert!(decode_accessibility(&bytes[..len]).is_err());
        }
        let mut trailing = bytes;
        trailing.push(0);
        assert!(decode_accessibility(&trailing).is_err());
    }
    for role in [
        Role::Table(TableInfo {
            rows: Some(-1),
            columns: None,
        }),
        Role::Table(TableInfo {
            rows: None,
            columns: Some(1025),
        }),
        Role::TableRow(-1),
        Role::TableRow(1_000_000),
        Role::TableCell(TableCell {
            row: 1,
            column: 1023,
            column_span: 2,
        }),
        Role::TableCell(TableCell {
            row: 1,
            column: 0,
            column_span: 0,
        }),
    ] {
        assert!(decode_accessibility(&encode(&config(role))).is_err());
    }
}
