use binprot::BinProtWrite;
use gpuio_protocol::{
    DecodeError, decode_scrollbar_config,
    scrollbar::*,
    v1::{Color, Fill},
};

fn baseline() -> Config {
    Config {
        label: "Viewport".into(),
        axis: Axis::Both,
        mode: Mode::Scrolling,
        appearance: Appearance::default(),
        motion: Motion::default(),
    }
}
fn custom() -> Config {
    Config {
        label: "Transcript".into(),
        axis: Axis::Both,
        mode: Mode::Hover,
        appearance: Appearance {
            track: Track {
                background: Some(0x10203040),
                border: Some(0xa0b0c0d0),
                width: Some(16.),
            },
            track_hover: Track {
                background: None,
                border: Some(0),
                width: Some(24.),
            },
            track_pressed: Track {
                background: Some(0),
                border: None,
                width: Some(32.),
            },
            thumb: Thumb {
                background: Some(Fill::Solid(Color::Rgba(0x010203ff))),
                width: Some(6.),
                inset: Some(4.),
                radius: Some(3.),
                min_length: Some(48.),
            },
            thumb_hover: Thumb {
                background: Some(Fill::LinearGradient(
                    90.,
                    Color::Rgba(0xffffffff),
                    0.,
                    Color::Rgba(0x112233ff),
                    1.,
                )),
                width: Some(10.),
                inset: None,
                radius: Some(5.),
                min_length: None,
            },
            thumb_pressed: Thumb {
                background: Some(Fill::LinearGradientIn(
                    1,
                    180.,
                    Color::Rgba(0x010203ff),
                    0.25,
                    Color::Rgba(0xabcdef80),
                    0.75,
                )),
                width: Some(12.),
                inset: Some(2.),
                radius: Some(6.),
                min_length: Some(64.),
            },
        },
        motion: Motion {
            idle_ms: 1500,
            enter_ms: 125,
            exit_ms: 250,
            expand_ms: 175,
            entrance: Entrance::SlideAndFade,
            thumb_hover_entrance: Entrance::Fade,
        },
    }
}
fn encode(config: &Config) -> Vec<u8> {
    let mut bytes = vec![];
    config.binprot_write(&mut bytes).unwrap();
    bytes
}
fn rejected(config: Config) {
    assert!(!config.is_valid());
    assert!(decode_scrollbar_config(&encode(&config)).is_err());
}

#[test]
fn independent_default_and_full_state_fixtures_decode_strictly() {
    for (config, expected) in [
        (
            baseline(),
            include_str!("../../../test/fixtures/scrollbar-default.hex"),
        ),
        (
            custom(),
            include_str!("../../../test/fixtures/scrollbar-custom.hex"),
        ),
    ] {
        let bytes = encode(&config);
        assert!(config.is_valid());
        assert_eq!(
            bytes.iter().map(|b| format!("{b:02x}")).collect::<String>(),
            expected.trim()
        );
        assert_eq!(decode_scrollbar_config(&bytes), Ok(config));
        for end in 0..bytes.len() {
            assert!(decode_scrollbar_config(&bytes[..end]).is_err());
        }
        let mut extra = bytes;
        extra.push(0);
        assert!(decode_scrollbar_config(&extra).is_err());
    }
    for axis in [Axis::Horizontal, Axis::Vertical, Axis::Both] {
        for mode in [Mode::Scrolling, Mode::Hover, Mode::Always] {
            for entrance in [Entrance::Fade, Entrance::SlideAndFade] {
                let mut config = custom();
                config.axis = axis;
                config.mode = mode;
                config.motion.thumb_hover_entrance = entrance;
                assert_eq!(decode_scrollbar_config(&encode(&config)), Ok(config));
            }
        }
    }
    // String length and optional/fill tags must be checked before allocation.
    assert_eq!(
        decode_scrollbar_config(&[0xfe, 1, 4]),
        Err(DecodeError::LimitExceeded)
    );
    assert_eq!(
        decode_scrollbar_config(&vec![0; MAX_CONFIG_BYTES + 1]),
        Err(DecodeError::LimitExceeded)
    );
    let bytes = encode(&baseline());
    for index in [9, 10, 11, bytes.len() - 2, bytes.len() - 1] {
        let mut bad = bytes.clone();
        bad[index] = 3; // Axis, Mode, Option or Entrance.
        assert!(decode_scrollbar_config(&bad).is_err());
    }
    let mut invalid_utf8 = bytes;
    invalid_utf8[1] = 0xff;
    assert!(decode_scrollbar_config(&invalid_utf8).is_err());
}

#[test]
fn all_part_dimensions_and_durations_have_checked_boundaries() {
    for value in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY, -0.1, 16384.1] {
        for field in 0..5 {
            for state in 0..3 {
                let mut config = baseline();
                let a = &mut config.appearance;
                if field == 0 {
                    [&mut a.track, &mut a.track_hover, &mut a.track_pressed][state].width =
                        Some(value);
                } else {
                    let thumb = [&mut a.thumb, &mut a.thumb_hover, &mut a.thumb_pressed]
                        .into_iter()
                        .nth(state)
                        .unwrap();
                    match field {
                        1 => thumb.width = Some(value),
                        2 => thumb.inset = Some(value),
                        3 => thumb.radius = Some(value),
                        4 => thumb.min_length = Some(value),
                        _ => unreachable!(),
                    }
                }
                rejected(config);
            }
        }
    }
    for value in [-1, 60_001, i64::MAX] {
        for field in 0..4 {
            let mut config = baseline();
            match field {
                0 => config.motion.idle_ms = value,
                1 => config.motion.enter_ms = value,
                2 => config.motion.exit_ms = value,
                _ => config.motion.expand_ms = value,
            }
            rejected(config);
        }
    }
    for label in [
        "".into(),
        " \t\r\n\x0b\x0c".into(),
        "bad\0label".into(),
        "x".repeat(1025),
    ] {
        rejected(Config {
            label,
            ..baseline()
        });
    }
    let mut max = custom();
    max.label = "x".repeat(1024);
    max.appearance.thumb.width = Some(0.);
    max.appearance.thumb.inset = Some(MAX_DIMENSION);
    max.appearance.thumb.radius = Some(MAX_DIMENSION);
    max.appearance.thumb.min_length = Some(MAX_DIMENSION);
    max.motion.idle_ms = 60_000;
    max.motion.enter_ms = 0;
    assert!(max.is_valid());
    let bytes = encode(&max);
    assert!(bytes.len() < MAX_CONFIG_BYTES);
    assert_eq!(decode_scrollbar_config(&bytes), Ok(max));
}

#[test]
fn resolved_colors_and_gradient_geometry_are_checked_in_every_state() {
    for fill in [
        Fill::Solid(Color::Token(0)),
        Fill::Solid(Color::Rgba(-1)),
        Fill::Solid(Color::Rgba(0x1_0000_0000)),
        Fill::LinearGradient(f64::NAN, Color::Rgba(0), 0., Color::Rgba(0), 1.),
        Fill::LinearGradient(361., Color::Rgba(0), 0., Color::Rgba(0), 1.),
        Fill::LinearGradient(90., Color::Rgba(0), 0.75, Color::Rgba(0), 0.25),
        Fill::LinearGradient(90., Color::Rgba(0), 0., Color::Rgba(0), f64::INFINITY),
        Fill::LinearGradientIn(2, 90., Color::Rgba(0), 0., Color::Rgba(0), 1.),
    ] {
        for state in 0..3 {
            let mut config = baseline();
            let a = &mut config.appearance;
            [&mut a.thumb, &mut a.thumb_hover, &mut a.thumb_pressed][state].background =
                Some(fill.clone());
            rejected(config);
        }
    }
    for color in [-1, 0x1_0000_0000] {
        for state in 0..3 {
            for border in [false, true] {
                let mut config = baseline();
                let a = &mut config.appearance;
                let track = [&mut a.track, &mut a.track_hover, &mut a.track_pressed]
                    .into_iter()
                    .nth(state)
                    .unwrap();
                if border {
                    track.border = Some(color);
                } else {
                    track.background = Some(color);
                }
                rejected(config);
            }
        }
    }
}

#[test]
fn independent_attach_and_clear_operation_fixture() {
    use gpuio_protocol::{
        NodeId, WindowId, decode,
        v1::{Message, Op, Transaction},
    };
    let node = NodeId::from_parts(0, 1).unwrap();
    let message = |config| {
        Message::Apply(Transaction {
            window: WindowId::from_parts(0, 1).unwrap(),
            base: 0,
            revision: 1,
            operations: vec![
                Op::SetScrollbar(node, Some(Box::new(config))),
                Op::SetScrollbar(node, None),
            ],
        })
    };
    let valid = message(baseline());
    let mut bytes = Vec::new();
    valid.binprot_write(&mut bytes).unwrap();
    assert_eq!(
        bytes.iter().map(|b| format!("{b:02x}")).collect::<String>(),
        include_str!("../../../test/fixtures/scrollbar-operation.hex").trim()
    );
    assert_eq!(decode(&bytes), Ok(valid));
    for end in 0..bytes.len() {
        assert!(decode(&bytes[..end]).is_err());
    }
    bytes.push(0);
    assert!(decode(&bytes).is_err());
    let mut invalid = baseline();
    invalid.appearance.thumb_pressed.width = Some(f64::NAN);
    bytes.clear();
    message(invalid).binprot_write(&mut bytes).unwrap();
    assert!(
        decode(&bytes).is_err(),
        "operation uses the checked config decoder"
    );
}
