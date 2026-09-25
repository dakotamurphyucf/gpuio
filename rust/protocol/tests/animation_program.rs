use binprot::BinProtWrite;
use gpuio_protocol::{
    DecodeError,
    animation::{Easing, Property, Repeat, Spring, Target},
    animation_program::*,
    decode_animation_program,
};
fn targets(width: f64, opacity: f64) -> Vec<Target> {
    vec![
        Target {
            property: Property::Width,
            value: width,
        },
        Target {
            property: Property::Opacity,
            value: opacity,
        },
    ]
}
fn example() -> Config {
    Config {
        generation: 42,
        program: Program {
            initial: Some(targets(0., 0.)),
            stages: vec![
                Stage {
                    targets: targets(120., 0.5),
                    timing: Timing::Tween(100, Easing::EaseOut),
                    delay_ms: 10,
                },
                Stage {
                    targets: targets(240., 1.),
                    timing: Timing::Spring(Spring {
                        stiffness: 100.,
                        damping: 10.,
                        mass: 1.,
                        epsilon: 0.001,
                        max_duration_ms: 10_000,
                    }),
                    delay_ms: 20,
                },
            ],
            delay_ms: 30,
            repeat: Repeat::Once,
            clock: Clock::Independent,
        },
        playback: Playback::Paused,
        restart: 2,
    }
}
fn encode(config: &Config) -> Vec<u8> {
    let mut bytes = vec![];
    config.binprot_write(&mut bytes).unwrap();
    bytes
}
#[test]
fn mixed_program_matches_independent_ocaml_fixture_and_round_trips() {
    let config = example();
    let bytes = encode(&config);
    let hex: String = bytes.iter().map(|byte| format!("{byte:02x}")).collect();
    assert_eq!(
        hex,
        include_str!("../../../test/fixtures/animation-program.hex").trim()
    );
    assert_eq!(decode_animation_program(&bytes), Ok(config));
    for end in 0..bytes.len() {
        assert!(decode_animation_program(&bytes[..end]).is_err());
    }
    let mut trailing = bytes;
    trailing.push(0);
    assert_eq!(
        decode_animation_program(&trailing),
        Err(DecodeError::Malformed)
    );
}
#[test]
fn stage_properties_timing_initial_values_and_physical_parameters_are_validated() {
    fn reject(f: impl FnOnce(&mut Config)) {
        let mut config = example();
        f(&mut config);
        assert!(!config.is_valid());
        assert!(decode_animation_program(&encode(&config)).is_err());
    }
    reject(|c| c.generation = 0);
    reject(|c| c.restart = -1);
    reject(|c| c.program.stages.clear());
    reject(|c| c.program.initial = None);
    reject(|c| c.program.stages[1].targets[0].property = Property::Height);
    reject(|c| c.program.stages[1].targets[1].value = 1.1);
    reject(|c| c.program.stages[0].targets[0].value = f64::NAN);
    reject(|c| c.program.stages[0].targets.swap(0, 1));
    reject(|c| c.program.stages[0].delay_ms = -1);
    reject(|c| c.program.delay_ms = 86_400_001);
    reject(|c| c.program.stages[0].timing = Timing::Tween(86_400_000, Easing::Linear));
    reject(|c| {
        c.program.stages[0].timing = Timing::Tween(100, Easing::CubicBezier(2., 0., 1., 1.))
    });
    reject(|c| {
        if let Timing::Spring(ref mut spring) = c.program.stages[1].timing {
            spring.mass = 0.;
        }
    });
    let mut maximum = example();
    maximum.program.stages = vec![maximum.program.stages[0].clone(); MAX_STAGES];
    assert!(maximum.is_valid());
    assert_eq!(
        decode_animation_program(&encode(&maximum)),
        Ok(maximum.clone())
    );
    maximum
        .program
        .stages
        .push(maximum.program.stages[0].clone());
    assert_eq!(
        decode_animation_program(&encode(&maximum)),
        Err(DecodeError::LimitExceeded)
    );
    let mut many_properties = example();
    many_properties.program.stages[0].targets = vec![
        Target {
            property: Property::Width,
            value: 0.
        };
        12
    ];
    assert_eq!(
        decode_animation_program(&encode(&many_properties)),
        Err(DecodeError::LimitExceeded)
    );
    assert_eq!(
        decode_animation_program(&vec![0; MAX_CONFIG_BYTES + 1]),
        Err(DecodeError::LimitExceeded)
    );
}
#[test]
fn shared_clocks_admit_only_fixed_positive_repeating_cycles() {
    let mut config = example();
    config.program.stages.truncate(1);
    config.program.delay_ms = 0;
    config.program.repeat = Repeat::Alternate;
    config.program.clock = Clock::Group("chat-pulse".into());
    assert!(config.is_valid());
    assert_eq!(
        decode_animation_program(&encode(&config)),
        Ok(config.clone())
    );
    let schedule = config.program.clone();
    config.program.stages[0].targets = targets(400., 0.75);
    config.program.stages[0].timing = Timing::Tween(100, Easing::EaseIn);
    assert!(schedule.same_clock_schedule(&config.program));
    config.program.stages[0].delay_ms = 11;
    assert!(!schedule.same_clock_schedule(&config.program));
    config.program = schedule;
    for clock in [
        Clock::Group(String::new()),
        Clock::Group("bad\0name".into()),
        Clock::Group("x".repeat(129)),
    ] {
        config.program.clock = clock;
        assert!(!config.is_valid());
        assert!(decode_animation_program(&encode(&config)).is_err());
    }
    config.program.clock = Clock::Application;
    config.program.repeat = Repeat::Once;
    assert!(!config.is_valid());
    config.program.repeat = Repeat::Loop;
    config.program.delay_ms = 1;
    assert!(!config.is_valid());
    config.program.delay_ms = 0;
    config.program.initial = None;
    assert!(!config.is_valid());
    config.program.initial = Some(targets(0., 0.));
    config.program.stages[0].timing = example().program.stages[1].timing;
    assert!(!config.is_valid());
    config.program.clock = Clock::Independent;
    assert!(config.is_valid());
    config.program.stages[0].timing = Timing::Tween(0, Easing::Linear);
    config.program.stages[0].delay_ms = 0;
    assert!(!config.is_valid());
}
#[test]
fn configuration_generation_and_playback_do_not_retarget_a_run() {
    let original = example();
    let mut next = original.clone();
    next.generation += 1;
    next.playback = Playback::Running;
    assert!(next.same_run(&original));
    next.playback = Playback::Cancelled;
    assert!(next.same_run(&original));
    next.restart += 1;
    assert!(!next.same_run(&original));
    next.restart = original.restart;
    next.program.stages[0].targets[0].value += 1.;
    assert!(!next.same_run(&original));
    assert!(
        original.retained_bytes()
            >= std::mem::size_of::<Config>()
                + 2 * std::mem::size_of::<Stage>()
                + 6 * std::mem::size_of::<Target>()
    );
}
#[test]
fn baseline_conversion_preserves_the_separate_delay_and_maximum_duration() {
    let legacy = gpuio_protocol::animation::Config {
        generation: 1,
        targets: targets(100., 1.),
        initial: Some(targets(0., 0.)),
        duration_ms: 86_400_000,
        delay_ms: 86_400_000,
        easing: Easing::Linear,
        repeat: Repeat::Loop,
    };
    let program = Program::from_legacy(&legacy).unwrap();
    assert!(program.is_valid());
    assert_eq!(program.delay_ms, legacy.delay_ms);
    assert_eq!(program.stages[0].delay_ms, 0);
    assert_eq!(
        program.stages[0].timing,
        Timing::Tween(legacy.duration_ms, legacy.easing)
    );
}
#[test]
fn signal_indices_bound_ordered_stages_and_terminal_delivery() {
    for stage in 0..32 {
        assert!(
            Signal {
                generation: 1,
                index: stage + 1,
                observation: Observation::StageCompleted(stage, StageResult::Played)
            }
            .is_valid()
        );
    }
    for stage in [-1, 32, i64::MAX] {
        assert!(
            !Signal {
                generation: 1,
                index: 1,
                observation: Observation::StageCompleted(stage, StageResult::ReducedMotion)
            }
            .is_valid()
        );
    }
    for observation in [
        Observation::Finished,
        Observation::Cancelled(CancelReason::Requested),
    ] {
        assert!(
            Signal {
                generation: 1,
                index: TERMINAL_INDEX,
                observation
            }
            .is_valid()
        );
        assert!(
            !Signal {
                generation: 1,
                index: 1,
                observation
            }
            .is_valid()
        );
    }
    assert!(
        !Signal {
            generation: 0,
            index: TERMINAL_INDEX,
            observation: Observation::Finished
        }
        .is_valid()
    );
}

#[test]
fn program_operation_and_batched_events_match_independent_wire_fixtures() {
    use gpuio_protocol::{HandlerId, NodeId, WindowId, v1::*};
    let w = WindowId::from_parts(0, 1).unwrap();
    let n = NodeId::from_parts(0, 1).unwrap();
    let h = HandlerId::from_parts(0, 1).unwrap();
    let request = Message::Apply(Transaction {
        window: w,
        base: 0,
        revision: 1,
        operations: vec![
            Op::Create(n, Kind::AnimationProgram, "".into(), Some(h)),
            Op::SetAnimationProgram(n, example()),
            Op::SetRoot(Some(n)),
        ],
    });
    let mut bytes = vec![];
    request.binprot_write(&mut bytes).unwrap();
    let hex: String = bytes.iter().map(|byte| format!("{byte:02x}")).collect();
    assert_eq!(
        hex,
        include_str!("../../../test/fixtures/animation-program-request.hex").trim()
    );
    assert_eq!(gpuio_protocol::decode(&bytes), Ok(request));
    for end in 0..bytes.len() {
        assert!(gpuio_protocol::decode(&bytes[..end]).is_err());
    }
    let signals = vec![
        Signal {
            generation: 42,
            index: 1,
            observation: Observation::StageCompleted(0, StageResult::Played),
        },
        Signal {
            generation: 42,
            index: 2,
            observation: Observation::StageCompleted(1, StageResult::ReducedMotion),
        },
        Signal {
            generation: 42,
            index: 33,
            observation: Observation::Finished,
        },
    ];
    assert!(Signal::valid_batch(&signals));
    assert!(!Signal::valid_batch(&[signals[0]; 34]));
    let mut bytes = vec![];
    vec![Event::AnimationProgramEvent(w, n, h, 1, signals)]
        .binprot_write(&mut bytes)
        .unwrap();
    let hex: String = bytes.iter().map(|byte| format!("{byte:02x}")).collect();
    assert_eq!(
        hex,
        include_str!("../../../test/fixtures/animation-program-events.hex").trim()
    );
}

#[test]
fn advanced_capability_handshake_matches_ocaml_above_32_bits() {
    use gpuio_protocol::{decode, v1::*};
    assert_eq!(CAPABILITIES & CAP_ANIMATION_PROGRAMS, 1_i64 << 32);
    let message = Message::Hello(VERSION, CAPABILITIES);
    let mut bytes = Vec::new();
    message.binprot_write(&mut bytes).unwrap();
    let hex: String = bytes.iter().map(|byte| format!("{byte:02x}")).collect();
    assert_eq!(hex, "0001fcffffffff07000000");
    assert_eq!(decode(&bytes), Ok(message));
}
