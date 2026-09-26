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
