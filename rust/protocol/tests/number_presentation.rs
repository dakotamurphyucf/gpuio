use binprot::BinProtWrite;
use gpuio_protocol::{NodeId, WindowId, decode, number_presentation::Config, v1::*};
fn config() -> Config {
    Config {
        gap: 4.,
        button_width: 30.,
        button_min_height: 22.,
        stacked_button_min_height: 16.,
        editor_padding: 3.,
        border_width: Some(1.),
        frame_style: vec![
            Style::Fields(vec![Field::Foreground(Color::Rgba(0x11223344))]),
            Style::State(1, vec![Field::BorderColor(Color::Rgba(0x66778899))]),
        ],
        editor_style: vec![Style::Fields(vec![Field::FontSize(14.)])],
        decrement_style: vec![Style::State(2, vec![Field::Opacity(0.5)])],
        increment_style: vec![Style::State(
            3,
            vec![Field::Foreground(Color::Rgba(0xaabbccdd))],
        )],
    }
}
fn message(config: Config) -> Message {
    let node = NodeId::from_parts(0, 1).unwrap();
    Message::Apply(Transaction {
        window: WindowId::from_parts(0, 1).unwrap(),
        base: 0,
        revision: 1,
        operations: vec![
            Op::SetNumberPresentation(node, Some(config)),
            Op::SetNumberPresentation(node, None),
        ],
    })
}
fn encode(value: &Message) -> Vec<u8> {
    let mut bytes = vec![];
    value.binprot_write(&mut bytes).unwrap();
    bytes
}
#[test]
fn independent_number_presentation_fixture_and_bounded_decode() {
    let value = message(config());
    let bytes = encode(&value);
    assert_eq!(
        bytes.iter().map(|b| format!("{b:02x}")).collect::<String>(),
        include_str!("../../../test/fixtures/number-presentation.hex").trim()
    );
    assert_eq!(decode(&bytes), Ok(value));
    for end in 0..bytes.len() {
        assert!(decode(&bytes[..end]).is_err());
    }
    let mut trailing = bytes;
    trailing.push(0);
    assert!(decode(&trailing).is_err());
    for n in [f64::NAN, f64::INFINITY, -1., 257.] {
        for c in [
            Config { gap: n, ..config() },
            Config {
                button_width: n,
                ..config()
            },
            Config {
                editor_padding: n,
                ..config()
            },
        ] {
            assert!(decode(&encode(&message(c))).is_err());
        }
    }
    assert!(
        decode(&encode(&message(Config {
            button_width: 0.,
            ..config()
        })))
        .is_err()
    );
    assert!(
        decode(&encode(&message(Config {
            border_width: Some(65.),
            ..config()
        })))
        .is_err()
    );
    assert!(
        decode(&encode(&message(Config {
            frame_style: vec![Style::Fields(vec![]); 17],
            ..config()
        })))
        .is_err()
    );
}

#[test]
fn application_step_mode_has_an_independent_paired_operation() {
    use gpuio_protocol::number_input::StepMode;
    let node = NodeId::from_parts(0, 1).unwrap();
    let value = Message::Apply(Transaction {
        window: WindowId::from_parts(0, 1).unwrap(),
        base: 0,
        revision: 1,
        operations: vec![
            Op::SetNumberStepMode(node, StepMode::Application),
            Op::SetNumberStepMode(node, StepMode::Native),
        ],
    });
    let bytes = encode(&value);
    assert_eq!(
        bytes.iter().map(|b| format!("{b:02x}")).collect::<String>(),
        include_str!("../../../test/fixtures/number-step-mode.hex").trim()
    );
    assert_eq!(decode(&bytes), Ok(value));
    for end in 0..bytes.len() {
        assert!(decode(&bytes[..end]).is_err());
    }
    let mut invalid = bytes.clone();
    invalid[9] = 2;
    assert!(decode(&invalid).is_err());
    let mut trailing = bytes;
    trailing.push(0);
    assert!(decode(&trailing).is_err());
}
