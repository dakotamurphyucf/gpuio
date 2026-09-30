use gpuio_native::session::Session;
use gpuio_protocol::{NodeId, WindowId, animation::*, animation_program, v1::*};

fn config(value: f64) -> Config {
    Config {
        generation: 1,
        targets: vec![Target {
            property: Property::OpacityFactor,
            value,
        }],
        initial: None,
        duration_ms: 0,
        delay_ms: 0,
        easing: Easing::Linear,
        repeat: Repeat::Once,
    }
}
#[test]
fn invalid_factors_reject_the_whole_transaction_in_both_native_apis() {
    for advanced in [false, true] {
        let window = WindowId::from_parts(0, 1).unwrap();
        let node = NodeId::from_parts(0, 1).unwrap();
        let text = NodeId::from_parts(1, 1).unwrap();
        let operation = |cfg: Config| {
            if advanced {
                let mut program = animation_program::Program::from_legacy(&config(1.)).unwrap();
                program.stages[0].targets = cfg.targets;
                Op::SetAnimationProgram(
                    node,
                    animation_program::Config {
                        generation: cfg.generation,
                        program,
                        playback: animation_program::Playback::Running,
                        restart: 0,
                    },
                )
            } else {
                Op::SetAnimation(node, cfg)
            }
        };
        let mut session = Session::default();
        session.hello(VERSION, CAPABILITIES).unwrap();
        session
            .open(1, window, "factor admission", 200., 200.)
            .unwrap();
        session
            .apply(&Transaction {
                window,
                base: 0,
                revision: 1,
                operations: vec![
                    Op::Create(
                        node,
                        if advanced {
                            Kind::AnimationProgram
                        } else {
                            Kind::Animated
                        },
                        String::new(),
                        None,
                    ),
                    operation(config(1.)),
                    Op::Create(text, Kind::Text, "original".into(), None),
                    Op::Splice(node, 0, 0, vec![text]),
                    Op::SetRoot(Some(node)),
                ],
            })
            .unwrap();
        let retained = session.retained_bytes();
        let mut invalids = [f64::NAN, f64::INFINITY, -0.1, 1.1].map(config).to_vec();
        let mut conflict = config(0.5);
        conflict.targets.insert(
            0,
            Target {
                property: Property::Opacity,
                value: 0.5,
            },
        );
        invalids.push(conflict);
        for mut invalid in invalids {
            invalid.generation = 2;
            assert!(
                session
                    .apply(&Transaction {
                        window,
                        base: 1,
                        revision: 2,
                        operations: vec![
                            Op::SetText(text, "must roll back".into()),
                            operation(invalid)
                        ]
                    })
                    .is_err()
            );
            assert_eq!(session.tree(window).unwrap().revision(), 1);
            assert_eq!(session.retained_bytes(), retained);
            assert_eq!(
                session
                    .tree(window)
                    .unwrap()
                    .get(text)
                    .unwrap()
                    .text
                    .as_ref(),
                "original"
            );
        }
    }
}
