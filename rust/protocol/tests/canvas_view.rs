use binprot::BinProtWrite;
use gpuio_protocol::{
    DecodeError, ResourceId, canvas::*, canvas_view::*, decode_canvas_view_config,
};

fn config() -> Config {
    Config {
        source: Some(ResourceId::from_parts(7, 2).unwrap()),
        label: "Canvas 🦀".into(),
        initial_viewport: Viewport {
            origin: Point { x: -2.5, y: 4.25 },
            zoom: 1.5,
        },
        minimum_zoom: 0.25,
        maximum_zoom: 8.,
        selectable: true,
        draggable: true,
        pan_zoom: false,
        disabled: false,
        selection_color: 0x102030ff,
        command: Some(Command {
            sequence: 9,
            action: Action::SetViewport(Viewport {
                origin: Point { x: 5., y: -3. },
                zoom: 2.75,
            }),
        }),
    }
}
fn encode(value: &impl BinProtWrite) -> Vec<u8> {
    let mut bytes = Vec::new();
    value.binprot_write(&mut bytes).unwrap();
    bytes
}

#[test]
fn canvas_view_transaction_and_event_match_independent_ocaml_fixture() {
    use gpuio_protocol::{HandlerId, NodeId, WindowId, decode, v1::*};
    let window = WindowId::from_parts(0, 1).unwrap();
    let node = NodeId::from_parts(0, 1).unwrap();
    let handler = HandlerId::from_parts(0, 1).unwrap();
    let message = Message::Apply(Transaction {
        window,
        base: 0,
        revision: 1,
        operations: vec![
            Op::Create(node, Kind::CanvasView, "".into(), Some(handler)),
            Op::SetCanvas(node, config()),
        ],
    });
    let bytes = encode(&message);
    let hex = |bytes: &[u8]| bytes.iter().map(|b| format!("{b:02x}")).collect::<String>();
    assert_eq!(
        hex(&bytes),
        format!(
            "0300010001020000011f00010001240001{}",
            include_str!("../../../test/fixtures/canvas-v1-view.hex").trim()
        )
    );
    assert_eq!(decode(&bytes), Ok(message));
    for end in 0..bytes.len() {
        assert!(decode(&bytes[..end]).is_err());
    }
    let mut trailing = bytes;
    trailing.push(0);
    assert_eq!(decode(&trailing), Err(DecodeError::Malformed));
    let event = Event::CanvasEvent(
        window,
        node,
        handler,
        1,
        Some(ResourceId::from_parts(7, 2).unwrap()),
        2,
        3,
        Observation::Activated(9),
    );
    assert_eq!(
        hex(&encode(&vec![event])),
        "01280001000100010101070202030109"
    );
}

#[test]
fn config_matches_ocaml_fixture_and_rejects_every_truncation() {
    let config = config();
    let bytes = encode(&config);
    let actual = bytes.iter().map(|b| format!("{b:02x}")).collect::<String>();
    assert_eq!(
        actual,
        include_str!("../../../test/fixtures/canvas-v1-view.hex").trim()
    );
    assert_eq!(decode_canvas_view_config(&bytes), Ok(config));
    for end in 0..bytes.len() {
        assert!(decode_canvas_view_config(&bytes[..end]).is_err());
    }
    let mut trailing = bytes.clone();
    trailing.push(0);
    assert_eq!(
        decode_canvas_view_config(&trailing),
        Err(DecodeError::Malformed)
    );
    assert_eq!(
        decode_canvas_view_config(&vec![0; MAX_CONFIG_BYTES + 1]),
        Err(DecodeError::LimitExceeded)
    );
    // A declared over-limit label is rejected before payload allocation, even
    // when its purported bytes have not arrived.
    assert_eq!(
        decode_canvas_view_config(&[1, 7, 2, 0xfe, 1, 4]),
        Err(DecodeError::LimitExceeded)
    );
}

#[test]
fn config_validation_and_bounded_decoder_agree() {
    let mut invalid = Vec::new();
    for label in ["", "\x0b \t\r\n", "\0"] {
        invalid.push(Config {
            label: label.into(),
            ..config()
        });
    }
    invalid.push(Config {
        label: "x".repeat(1025),
        ..config()
    });
    for (minimum_zoom, maximum_zoom) in [
        (0., 1.),
        (2., 1.),
        (0.05, 65.),
        (f64::NAN, 2.),
        (0.05, f64::INFINITY),
    ] {
        invalid.push(Config {
            minimum_zoom,
            maximum_zoom,
            ..config()
        });
    }
    invalid.push(Config {
        selection_color: -1,
        ..config()
    });
    invalid.push(Config {
        command: Some(Command {
            sequence: 0,
            action: Action::ResetPositions,
        }),
        ..config()
    });
    invalid.push(Config {
        command: Some(Command {
            sequence: 1,
            action: Action::Select(Some(0)),
        }),
        ..config()
    });
    for value in invalid {
        assert!(!value.is_valid());
        assert!(decode_canvas_view_config(&encode(&value)).is_err());
    }
    let mut bytes = encode(&config());
    bytes[4] = 255; // UTF-8 label payload.
    assert_eq!(
        decode_canvas_view_config(&bytes),
        Err(DecodeError::Malformed)
    );
}

#[test]
fn semantic_observations_and_commands_have_bounded_values() {
    for observation in [
        Observation::SelectionChanged(None),
        Observation::Activated(1),
        Observation::Moved(1, Transform::IDENTITY),
        Observation::ViewportChanged(Viewport::default()),
        Observation::CommandCompleted(1),
        Observation::Failed(Error::RenderLimit),
    ] {
        assert!(observation.is_valid());
    }
    for observation in [
        Observation::SelectionChanged(Some(0)),
        Observation::Activated(-1),
        Observation::Moved(
            1,
            Transform {
                a: 0.,
                ..Transform::IDENTITY
            },
        ),
        Observation::ViewportChanged(Viewport {
            zoom: f64::NAN,
            ..Viewport::default()
        }),
        Observation::CommandCompleted(0),
    ] {
        assert!(!observation.is_valid());
    }
    for action in [
        Action::Select(None),
        Action::SetViewport(Viewport::default()),
        Action::ResetViewport,
        Action::ResetPositions,
    ] {
        let value = Config {
            command: Some(Command {
                sequence: 1,
                action,
            }),
            ..config()
        };
        assert_eq!(decode_canvas_view_config(&encode(&value)), Ok(value));
    }
}
