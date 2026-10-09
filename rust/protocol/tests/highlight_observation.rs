use binprot::BinProtWrite;
use gpuio_protocol::{
    HandlerId, NodeId, WindowId, decode_highlight_observation, highlight::*, v1::*,
};
fn bytes(value: &impl BinProtWrite) -> Vec<u8> {
    let mut b = Vec::new();
    value.binprot_write(&mut b).unwrap();
    b
}
fn hex(value: &impl BinProtWrite) -> String {
    bytes(value).iter().map(|b| format!("{b:02x}")).collect()
}
#[test]
fn independent_observation_bytes_all_states_and_strict_bounds() {
    let states = [
        State::Pending,
        State::Ready(vec![
            Count {
                total: 5,
                stored: 3,
            },
            Count {
                total: 0,
                stored: 0,
            },
        ]),
        State::InvalidRange(InvalidRange {
            spec_index: 1,
            range_index: 2,
            reason: RangeError::ScalarBoundary,
        }),
        State::Capacity(Limit::Source),
        State::Capacity(Limit::Work),
        State::Capacity(Limit::Admission),
        State::Failed(Failure::SourceUnavailable),
        State::Failed(Failure::WorkerFailed),
        State::Failed(Failure::EpochExhausted),
        State::InvalidRange(InvalidRange {
            spec_index: 0,
            range_index: 0,
            reason: RangeError::OutOfBounds,
        }),
    ];
    let expected = [
        "0100",
        "02010205030000",
        "0302010201",
        "040300",
        "050301",
        "060302",
        "070400",
        "080401",
        "090402",
        "0a02000000",
    ];
    for (i, state) in states.into_iter().enumerate() {
        let observation = Observation {
            epoch: i as i64 + 1,
            state,
        };
        assert_eq!(hex(&observation), expected[i]);
        let b = bytes(&observation);
        assert_eq!(decode_highlight_observation(&b), Ok(observation));
        for n in 0..b.len() {
            assert!(decode_highlight_observation(&b[..n]).is_err());
        }
        let mut extra = b;
        extra.push(0);
        assert!(decode_highlight_observation(&extra).is_err());
    }
    for bad in [
        vec![0, 0],
        vec![1, 5],
        vec![1, 3, 3],
        vec![1, 4, 3],
        vec![1, 2, 0, 0, 2],
        vec![1, 1, 17],
    ] {
        assert!(decode_highlight_observation(&bad).is_err());
    }
    for count in [
        Count {
            total: -1,
            stored: 0,
        },
        Count {
            total: 1,
            stored: 2,
        },
        Count {
            total: 20000,
            stored: 16385,
        },
    ] {
        assert!(
            decode_highlight_observation(&bytes(&Observation {
                epoch: 1,
                state: State::Ready(vec![count])
            }))
            .is_err()
        );
    }
    let mut maximum = Observation {
        epoch: i64::MAX,
        state: State::Ready(vec![
            Count {
                total: i64::MAX,
                stored: 1024
            };
            16
        ]),
    };
    assert!(bytes(&maximum).len() <= MAX_OBSERVATION_BYTES);
    assert_eq!(
        decode_highlight_observation(&bytes(&maximum)),
        Ok(maximum.clone())
    );
    if let State::Ready(counts) = &mut maximum.state {
        counts[0].stored += 1;
    }
    assert!(decode_highlight_observation(&bytes(&maximum)).is_err());
    assert!(decode_highlight_observation(&vec![0; MAX_OBSERVATION_BYTES + 1]).is_err());
}

#[test]
fn scope_envelopes_have_independent_appended_tags() {
    let window = WindowId::from_parts(0, 1).unwrap();
    let node = NodeId::from_parts(0, 1).unwrap();
    let handler = HandlerId::from_parts(0, 1).unwrap();
    let request = Message::Apply(Transaction {
        window,
        base: 0,
        revision: 1,
        operations: vec![
            Op::Create(node, Kind::HighlightScope, "".into(), Some(handler)),
            Op::SetHighlightScope(node, Config(vec![])),
            Op::SetRoot(Some(node)),
        ],
    });
    assert_eq!(
        hex(&request),
        "03000100010300000132000100013900010006010001"
    );
    let b = bytes(&request);
    assert_eq!(gpuio_protocol::decode(&b), Ok(request));
    for n in 0..b.len() {
        assert!(gpuio_protocol::decode(&b[..n]).is_err());
    }
    let mut extra = b;
    extra.push(0);
    assert!(gpuio_protocol::decode(&extra).is_err());
    let event = Event::HighlightObserved(
        window,
        node,
        handler,
        1,
        Observation {
            epoch: 1,
            state: State::Pending,
        },
    );
    assert_eq!(hex(&vec![event]), "0140000100010001010100");
}

#[test]
fn observation_shape_must_match_the_live_scope_configuration() {
    let config = Config(vec![Spec {
        query: None,
        ranges: vec![Range {
            start_byte: 0,
            end_byte: 1,
        }],
        appearance: Appearance {
            color: 1,
            active_color: 2,
            radius: 2.,
        },
        active_index: None,
        match_index_offset: 0,
    }]);
    assert!(
        !Observation {
            epoch: 1,
            state: State::Ready(vec![])
        }
        .valid_for(&config)
    );
    assert!(
        Observation {
            epoch: 1,
            state: State::Ready(vec![Count {
                total: 0,
                stored: 0
            }])
        }
        .valid_for(&config)
    );
    for (spec, range, valid) in [(0, 0, true), (0, 1, false), (1, 0, false)] {
        let observation = Observation {
            epoch: 1,
            state: State::InvalidRange(InvalidRange {
                spec_index: spec,
                range_index: range,
                reason: RangeError::OutOfBounds,
            }),
        };
        assert_eq!(observation.valid_for(&config), valid);
    }
}
