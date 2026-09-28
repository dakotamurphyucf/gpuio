use binprot::BinProtWrite;
use gpuio_protocol::{
    DecodeError,
    chart_selection::{Aggregation, Selection, Span},
    decode_chart_selection,
};
fn span() -> Span {
    Span {
        start_index: 1,
        length: 3,
        first: 42,
        last: 7,
    }
}
fn cases() -> Vec<Selection> {
    let single = Span {
        start_index: 2,
        length: 1,
        first: 17,
        last: 17,
    };
    vec![
        Selection::Cartesian {
            series: 9,
            span: single,
            aggregation: Aggregation::Exact,
        },
        Selection::Cartesian {
            series: 9,
            span: span(),
            aggregation: Aggregation::Sum,
        },
        Selection::Cartesian {
            series: 9,
            span: span(),
            aggregation: Aggregation::Mean,
        },
        Selection::Slice(7),
        Selection::Radar { series: 9, axis: 3 },
        Selection::Candlestick {
            span: single,
            aggregated: false,
        },
        Selection::Candlestick {
            span: span(),
            aggregated: true,
        },
        Selection::Node(1),
        Selection::Edge(5),
    ]
}
fn encode(value: &impl BinProtWrite) -> Vec<u8> {
    let mut bytes = vec![];
    value.binprot_write(&mut bytes).unwrap();
    bytes
}
fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}
#[test]
fn all_families_and_committed_events_match_independent_fixture() {
    use gpuio_protocol::{
        HandlerId, NodeId, ResourceId, WindowId, chart_view::Observation, v1::Event,
    };
    let fixtures = include_str!("../../../test/fixtures/chart-v1-selection.hex")
        .lines()
        .collect::<Vec<_>>();
    assert_eq!(fixtures.len(), cases().len());
    for (case, fixture) in cases().into_iter().zip(fixtures) {
        let (_, fixture) = fixture.split_once(' ').unwrap();
        let bytes = encode(&case);
        assert_eq!(hex(&bytes), fixture);
        assert_eq!(decode_chart_selection(&bytes), Ok(case));
        for end in 0..bytes.len() {
            assert!(decode_chart_selection(&bytes[..end]).is_err());
        }
        let mut trailing = bytes;
        trailing.push(0);
        assert_eq!(
            decode_chart_selection(&trailing),
            Err(DecodeError::Malformed)
        );
        let event = Event::ChartEvent(
            WindowId::from_parts(0, 1).unwrap(),
            NodeId::from_parts(0, 1).unwrap(),
            HandlerId::from_parts(0, 1).unwrap(),
            1,
            Some(ResourceId::from_parts(7, 2).unwrap()),
            2,
            3,
            Observation::SelectionChanged(Some(case)),
        );
        assert_eq!(
            hex(&encode(&vec![event])),
            format!("013e0001000100010101070202030201{fixture}")
        );
    }
}
#[test]
fn invalid_spans_ids_tags_and_exact_aggregates_are_rejected() {
    for span in [
        Span {
            start_index: -1,
            ..span()
        },
        Span {
            start_index: 100_000,
            ..span()
        },
        Span {
            start_index: 99_999,
            ..span()
        },
        Span {
            length: 0,
            ..span()
        },
        Span {
            length: i64::MAX,
            ..span()
        },
        Span { first: 0, ..span() },
        Span { last: -1, ..span() },
        Span { last: 42, ..span() },
        Span {
            length: 1,
            ..span()
        },
    ] {
        let value = Selection::Candlestick {
            span,
            aggregated: true,
        };
        assert!(!value.is_valid());
        assert_eq!(
            decode_chart_selection(&encode(&value)),
            Err(DecodeError::Malformed)
        );
    }
    for value in [
        Selection::Cartesian {
            series: 9,
            span: span(),
            aggregation: Aggregation::Exact,
        },
        Selection::Candlestick {
            span: span(),
            aggregated: false,
        },
        Selection::Slice(0),
        Selection::Node(-1),
        Selection::Edge(0),
        Selection::Radar { series: 1, axis: 0 },
    ] {
        assert!(decode_chart_selection(&encode(&value)).is_err());
    }
    assert_eq!(decode_chart_selection(&[6]), Err(DecodeError::Malformed));
    for mut bytes in [encode(&cases()[0]), encode(&cases()[5])] {
        *bytes.last_mut().unwrap() = 3;
        assert_eq!(decode_chart_selection(&bytes), Err(DecodeError::Malformed));
    }
    assert_eq!(
        decode_chart_selection(&[0; 65]),
        Err(DecodeError::LimitExceeded)
    );
    // Exact valid upper source bound and arbitrary positive ID order.
    let boundary = Selection::Candlestick {
        span: Span {
            start_index: 99_997,
            ..span()
        },
        aggregated: true,
    };
    assert_eq!(decode_chart_selection(&encode(&boundary)), Ok(boundary));
}
