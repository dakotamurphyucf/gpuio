use gpuio_native::split_group_appearance::{refinement, retained_bytes, validate};
use gpuio_protocol::{split_group_appearance::Config, v1::*};
#[test]
fn paint_cannot_change_geometry_visibility_or_input_and_has_a_total_budget() {
    for field in [
        Field::Width(Length::Px(50.)),
        Field::Opacity(f64::NAN),
        Field::Opacity(1.1),
        Field::FontSize(-1.),
        Field::BorderTopWidth(-1.),
    ] {
        let c = Config {
            handle_style: vec![Style::Fields(vec![field])],
            ..Config::default()
        };
        assert!(validate(&c).is_err());
    }
    for state in [0, 4, 5, 7, 8] {
        assert!(
            validate(&Config {
                handle_style: vec![Style::State(state, vec![])],
                ..Config::default()
            })
            .is_err()
        );
    }
    let styles = vec![Style::Fields(vec![Field::Opacity(0.5); 5])];
    let c = Config {
        item_styles: (0..64).map(|i| (i.to_string(), styles.clone())).collect(),
        ..Config::default()
    };
    assert_eq!(validate(&c), Err(ErrorCode::LimitExceeded));
    assert!(retained_bytes(&c) > std::mem::size_of::<Config>());
}
#[test]
fn per_id_state_precedence_and_disabled_paint_are_deterministic() {
    let c = Config {
        handle_style: vec![
            Style::Fields(vec![Field::Opacity(0.9)]),
            Style::State(2, vec![Field::Opacity(0.8)]),
            Style::State(1, vec![Field::Opacity(0.7)]),
            Style::State(3, vec![Field::Opacity(0.6)]),
            Style::State(6, vec![Field::Opacity(0.5)]),
        ],
        item_styles: vec![(
            "a".into(),
            vec![
                Style::Fields(vec![Field::Opacity(0.4)]),
                Style::State(3, vec![Field::Opacity(0.3)]),
                Style::State(6, vec![Field::Opacity(0.2)]),
            ],
        )],
        ..Config::default()
    };
    validate(&c).unwrap();
    for (id, hover, focus, pressed, disabled, want) in [
        ("b", false, false, false, false, 0.9),
        ("a", false, false, false, false, 0.4),
        ("a", true, false, false, false, 0.8),
        ("a", true, true, false, false, 0.7),
        ("a", true, true, true, false, 0.3),
        ("a", true, true, true, true, 0.2),
        ("b", true, true, true, true, 0.5),
    ] {
        assert_eq!(
            refinement(&c, id, hover, focus, pressed, disabled).opacity,
            Some(want)
        );
    }
}
