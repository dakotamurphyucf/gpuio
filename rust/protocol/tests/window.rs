use binprot::BinProtWrite;
use gpuio_protocol::{WindowId, decode, v1::*, window::*};
fn hex(message: &Message) -> String {
    let mut bytes = Vec::new();
    message.binprot_write(&mut bytes).unwrap();
    assert_eq!(decode(&bytes).unwrap(), *message);
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}
#[test]
fn window_messages_match_ocaml_fixtures() {
    let id = WindowId::from_parts(0, 1).unwrap();
    for (command, expected) in [
        (Command::Observe, "0b07000100"),
        (Command::SetTitle("x".into()), "0b070001010178"),
        (
            Command::Resize(640., 400.),
            "0b0700010200000000000084400000000000007940",
        ),
        (Command::Activate, "0b07000103"),
        (Command::Zoom, "0b07000104"),
        (Command::ToggleFullscreen, "0b07000105"),
        (Command::SetEdited(true), "0b0700010601"),
        (Command::Minimize, "0b07000108"),
        (
            Command::SetDocument(Document {
                path: None,
                edited: false,
            }),
            "0b070001070000",
        ),
        (
            Command::SetDocument(Document {
                path: Some(
                    gpuio_protocol::file_path::FilePath::new(b"/tmp/\xff".to_vec()).unwrap(),
                ),
                edited: true,
            }),
            "0b0700010701062f746d702fff01",
        ),
    ] {
        assert_eq!(hex(&Message::WindowCommand(7, id, command)), expected);
    }
    assert_eq!(
        hex(&Message::OpenConfigured(
            7,
            id,
            Config {
                title: "x".into(),
                width: 640.,
                height: 400.,
                focus: false,
                chrome: Chrome::Hidden,
                resizable: false,
                frame: Frame::default(),
            }
        )),
        "0c07000101780000000000008440000000000000794000010000000000000034400000000000001040"
    );
    let mut bytes = Vec::new();
    vec![
        Event::CloseRequested(id),
        Event::QuitRequested,
        Event::ReopenRequested,
    ]
    .binprot_write(&mut bytes)
    .unwrap();
    assert_eq!(bytes, vec![3, 31, 0, 1, 32, 33]);
}
#[test]
fn native_decoder_rejects_invalid_window_data() {
    let id = WindowId::from_parts(0, 1).unwrap();
    for command in [
        Command::Resize(f64::NAN, 400.),
        Command::Resize(0., 400.),
        Command::SetTitle("bad\0title".into()),
    ] {
        let mut bytes = Vec::new();
        Message::WindowCommand(7, id, command)
            .binprot_write(&mut bytes)
            .unwrap();
        assert!(decode(&bytes).is_err());
    }
    // String bytes are decoded as a validated native path, not UTF-8 text.
    for bytes in [
        b"\x0b\x07\x00\x01\x07\x01\x03rel\x00".as_slice(),
        b"\x0b\x07\x00\x01\x07\x01\x03/a\x00\x00".as_slice(),
    ] {
        assert!(decode(bytes).is_err());
    }
}

#[test]
fn document_observation_matches_independent_ocaml_fixture() {
    let id = WindowId::from_parts(0, 1).unwrap();
    let events = vec![
        Event::WindowResponse(
            7,
            id,
            Response::Observed(Snapshot {
                appearance: Appearance::Light,
                title: "t".into(),
                x: 0.,
                y: 0.,
                width: 0.,
                height: 0.,
                content_width: 0.,
                content_height: 0.,
                active: false,
                fullscreen: false,
                maximized: false,
                presentation: Presentation {
                    decorations: Decorations::Server,
                    controls: Controls {
                        fullscreen: true,
                        maximize: true,
                        minimize: true,
                        window_menu: true,
                    },
                    resizable: true,
                },
                document: Some(Document {
                    path: Some(
                        gpuio_protocol::file_path::FilePath::new(b"/tmp/\xff".to_vec()).unwrap(),
                    ),
                    edited: true,
                }),
            }),
        ),
        Event::WindowResponse(8, id, Response::Failed(Error::Unsupported)),
    ];
    let mut actual = Vec::new();
    events.binprot_write(&mut actual).unwrap();
    let mut expected = b"\x02\x23\x07\x00\x01\x00\x01t".to_vec();
    expected.extend_from_slice(&[0; 48]);
    expected.extend_from_slice(
        b"\x00\x00\x00\x01\x01\x06/tmp/\xff\x01\x00\x01\x01\x01\x01\x01\x00\x23\x08\x00\x01\x01\x05",
    );
    assert_eq!(actual, expected);
}

#[test]
fn custom_chrome_uses_additive_tag_and_unknown_chrome_is_rejected() {
    let message = Message::OpenConfigured(
        7,
        WindowId::from_parts(0, 1).unwrap(),
        Config {
            title: "x".into(),
            width: 640.,
            height: 400.,
            focus: false,
            chrome: Chrome::Custom,
            resizable: false,
            frame: Frame::default(),
        },
    );
    assert_eq!(
        hex(&message),
        "0c07000101780000000000008440000000000000794000020000000000000034400000000000001040"
    );
    let mut bytes = vec![];
    message.binprot_write(&mut bytes).unwrap();
    let index = bytes.len() - 18;
    bytes[index] = 3;
    assert!(decode(&bytes).is_err());
}

#[test]
fn configured_frames_reject_nonfinite_out_of_range_and_truncated_geometry() {
    let message = |frame| {
        Message::OpenConfigured(
            7,
            WindowId::from_parts(0, 1).unwrap(),
            Config {
                title: "frame".into(),
                width: 640.,
                height: 400.,
                focus: false,
                chrome: Chrome::Custom,
                resizable: true,
                frame,
            },
        )
    };
    for frame in [
        Frame::default(),
        Frame {
            shadow_size: 0.,
            resize_hit_size: 0.5,
        },
        Frame {
            shadow_size: 128.,
            resize_hit_size: 32.,
        },
    ] {
        let mut bytes = vec![];
        let valid = message(frame);
        valid.binprot_write(&mut bytes).unwrap();
        assert_eq!(decode(&bytes).unwrap(), valid);
        for missing in 1..=16 {
            assert!(decode(&bytes[..bytes.len() - missing]).is_err());
        }
    }
    for frame in [
        Frame {
            shadow_size: -1.,
            ..Frame::default()
        },
        Frame {
            shadow_size: 129.,
            ..Frame::default()
        },
        Frame {
            shadow_size: f64::NAN,
            ..Frame::default()
        },
        Frame {
            shadow_size: f64::INFINITY,
            ..Frame::default()
        },
        Frame {
            resize_hit_size: 0.,
            ..Frame::default()
        },
        Frame {
            resize_hit_size: 33.,
            ..Frame::default()
        },
        Frame {
            resize_hit_size: f64::NAN,
            ..Frame::default()
        },
        Frame {
            resize_hit_size: f64::NEG_INFINITY,
            ..Frame::default()
        },
    ] {
        let mut bytes = vec![];
        message(frame).binprot_write(&mut bytes).unwrap();
        assert!(decode(&bytes).is_err());
    }
}

#[test]
fn client_presentation_matches_independent_ocaml_observation() {
    let event = Event::WindowChanged(
        WindowId::from_parts(0, 1).unwrap(),
        Snapshot {
            appearance: Appearance::Light,
            title: "t".into(),
            x: 0.,
            y: 0.,
            width: 0.,
            height: 0.,
            content_width: 0.,
            content_height: 0.,
            active: false,
            fullscreen: false,
            maximized: false,
            document: None,
            presentation: Presentation {
                decorations: Decorations::Client(Tiling {
                    top: true,
                    right: false,
                    bottom: true,
                    left: false,
                }),
                controls: Controls {
                    fullscreen: false,
                    maximize: true,
                    minimize: false,
                    window_menu: true,
                },
                resizable: true,
            },
        },
    );
    let mut actual = vec![];
    vec![event].binprot_write(&mut actual).unwrap();
    let mut expected = b"\x01\x22\x00\x01\x01t".to_vec();
    expected.extend_from_slice(&[0; 52]);
    expected.extend_from_slice(b"\x01\x01\x00\x01\x00\x00\x01\x00\x01\x01\x00");
    assert_eq!(actual, expected);
}

#[test]
fn focused_input_query_and_metadata_match_independent_ocaml_bytes() {
    let id = WindowId::from_parts(0, 1).unwrap();
    assert_eq!(
        hex(&Message::WindowCommand(7, id, Command::FocusedInput)),
        "0b07000109"
    );
    for (kind, tag) in [
        (InputKind::Input, 0),
        (InputKind::Textarea, 1),
        (InputKind::Combobox, 2),
        (InputKind::Otp, 3),
        (InputKind::Number, 4),
        (InputKind::Color, 5),
        (InputKind::CommandPalette, 6),
    ] {
        let mut bytes = vec![];
        vec![Event::WindowResponse(
            7,
            id,
            Response::FocusedInput(Some(Input {
                node: gpuio_protocol::NodeId::from_parts(3, 4).unwrap(),
                kind,
            })),
        )]
        .binprot_write(&mut bytes)
        .unwrap();
        assert_eq!(bytes, vec![1, 35, 7, 0, 1, 2, 1, 3, 4, tag]);
    }
    let mut bytes = vec![];
    vec![Event::WindowResponse(7, id, Response::FocusedInput(None))]
        .binprot_write(&mut bytes)
        .unwrap();
    assert_eq!(bytes, vec![1, 35, 7, 0, 1, 2, 0]);
}

#[test]
fn selection_queries_and_replies_match_independent_ocaml_bytes() {
    let id = WindowId::from_parts(0, 1).unwrap();
    for (command, expected) in [
        (Command::HasTextSelection, "0b0700010a"),
        (Command::SelectedText(4), "0b0700010b04"),
        (Command::ClearTextSelection, "0b0700010c"),
        (Command::EndTextSelection, "0b0700010d"),
    ] {
        assert_eq!(hex(&Message::WindowCommand(7, id, command)), expected);
    }
    for limit in [-1, MAX_SELECTION_BYTES as i64 + 1, i64::MAX] {
        let mut bytes = vec![];
        Message::WindowCommand(7, id, Command::SelectedText(limit))
            .binprot_write(&mut bytes)
            .unwrap();
        assert!(decode(&bytes).is_err());
    }
    for limit in [0, MAX_SELECTION_BYTES as i64] {
        hex(&Message::WindowCommand(7, id, Command::SelectedText(limit)));
    }
    for (response, suffix) in [
        (Response::SelectionPresent(true), vec![3, 1]),
        (
            Response::SelectedText("é\n ".into()),
            vec![4, 4, 0xc3, 0xa9, 10, 32],
        ),
        (Response::SelectionUpdated, vec![5]),
        (Response::Failed(Error::LimitExceeded), vec![1, 6]),
    ] {
        let mut bytes = vec![];
        vec![Event::WindowResponse(7, id, response)]
            .binprot_write(&mut bytes)
            .unwrap();
        let mut expected = vec![1, 35, 7, 0, 1];
        expected.extend(suffix);
        assert_eq!(bytes, expected);
    }
}

#[test]
fn native_appearances_use_explicit_snapshot_tags() {
    for (appearance, tag) in [
        (Appearance::Light, 0),
        (Appearance::VibrantLight, 1),
        (Appearance::Dark, 2),
        (Appearance::VibrantDark, 3),
    ] {
        let snapshot = Snapshot {
            title: "t".into(),
            x: 0.,
            y: 0.,
            width: 0.,
            height: 0.,
            content_width: 0.,
            content_height: 0.,
            active: false,
            fullscreen: false,
            maximized: false,
            document: None,
            presentation: Presentation {
                decorations: Decorations::Server,
                controls: Controls {
                    fullscreen: true,
                    maximize: true,
                    minimize: true,
                    window_menu: true,
                },
                resizable: true,
            },
            appearance,
        };
        let mut actual = vec![];
        vec![Event::WindowChanged(
            WindowId::from_parts(0, 1).unwrap(),
            snapshot,
        )]
        .binprot_write(&mut actual)
        .unwrap();
        let mut expected = b"\x01\x22\x00\x01\x01t".to_vec();
        expected.extend_from_slice(&[0; 52]);
        expected.extend_from_slice(&[0, 1, 1, 1, 1, 1, tag]);
        assert_eq!(actual, expected);
    }
}
