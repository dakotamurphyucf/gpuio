use binprot::BinProtWrite;
use gpuio_protocol::{
    DecodeError, ResourceId,
    chart_inspection_content::{self as content, Container, Entry, Target},
    chart_selection::{Aggregation, Selection, Span},
    decode_chart_inspection_content,
};
fn source(slot: i64) -> ResourceId {
    ResourceId::from_parts(slot, 2).unwrap()
}
fn span() -> Span {
    Span {
        start_index: 3,
        length: 2,
        first: 9,
        last: 4,
    }
}
fn aggregate() -> Selection {
    Selection::Cartesian {
        series: 7,
        span: span(),
        aggregation: Aggregation::Sum,
    }
}
fn bytes(entries: &Vec<Entry>) -> Vec<u8> {
    let mut bytes = vec![];
    entries.binprot_write(&mut bytes).unwrap();
    bytes
}
fn fixture() -> Vec<Entry> {
    [
        Target::Cartesian(7, 9),
        Target::Slice(9),
        Target::Radar(7, 9),
        Target::Candlestick(9),
        Target::Node(9),
        Target::Edge(9),
        Target::from_selection(aggregate(), source(4), 11, 2).unwrap(),
        Target::from_selection(
            Selection::Candlestick {
                span: Span {
                    start_index: 0,
                    length: 1,
                    first: 9,
                    last: 9,
                },
                aggregated: true,
            },
            source(4),
            11,
            2,
        )
        .unwrap(),
    ]
    .into_iter()
    .enumerate()
    .map(|(i, target)| Entry {
        target: Some(target),
        container: if i % 2 == 0 {
            Container::Card
        } else {
            Container::Overlay
        },
    })
    .collect()
}
#[test]
fn independent_target_metadata_fixture_all_tags_truncation_and_trailing() {
    let hex = "080100070900010109010102070900010309010104090001050901010604020b020007030209040100010604020b0203000109090101";
    let expected = hex
        .as_bytes()
        .chunks_exact(2)
        .map(|s| u8::from_str_radix(std::str::from_utf8(s).unwrap(), 16).unwrap())
        .collect::<Vec<_>>();
    let fixture = fixture();
    assert!(content::is_valid(&fixture));
    assert_eq!(bytes(&fixture), expected);
    assert_eq!(decode_chart_inspection_content(&expected), Ok(fixture));
    for end in 0..expected.len() {
        assert!(decode_chart_inspection_content(&expected[..end]).is_err());
    }
    let mut trailing = expected;
    trailing.push(0);
    assert!(decode_chart_inspection_content(&trailing).is_err());
    assert_eq!(
        decode_chart_inspection_content(&[2, 0, 0, 0, 1]),
        Ok(vec![
            Entry {
                target: None,
                container: Container::Card
            },
            Entry {
                target: None,
                container: Container::Overlay
            }
        ])
    );
}
#[test]
fn stable_targets_ignore_positions_aggregates_fence_source_and_publication() {
    let exact = Selection::Cartesian {
        series: 7,
        span: Span {
            start_index: 0,
            length: 1,
            first: 9,
            last: 9,
        },
        aggregation: Aggregation::Exact,
    };
    let reordered = Selection::Cartesian {
        series: 7,
        span: Span {
            start_index: 99,
            length: 1,
            first: 9,
            last: 9,
        },
        aggregation: Aggregation::Exact,
    };
    assert_eq!(
        Target::from_selection(exact, source(4), 11, 2),
        Target::from_selection(reordered, source(4), 12, 3)
    );
    for aggregation in [Aggregation::Sum, Aggregation::Mean] {
        let one = Selection::Cartesian {
            series: 7,
            span: Span {
                start_index: 0,
                length: 1,
                first: 9,
                last: 9,
            },
            aggregation,
        };
        let target = Target::from_selection(one, source(4), 11, 2).unwrap();
        assert!(matches!(target, Target::Aggregate { .. }));
        for (source, revision, generation) in
            [(source(5), 11, 2), (source(4), 12, 2), (source(4), 11, 3)]
        {
            assert_ne!(
                Some(target),
                Target::from_selection(one, source, revision, generation)
            );
        }
    }
    for (revision, generation) in [(0, 1), (1, 0), (-1, 1), (1, -1)] {
        assert_eq!(
            Target::from_selection(exact, source(4), revision, generation),
            None
        );
    }
    assert_eq!(
        Target::from_selection(Selection::Slice(0), source(4), 1, 1),
        None
    );
}
#[test]
fn admission_rejects_bad_ids_publications_selection_variants_and_duplicates() {
    let present = |target| Entry {
        target: Some(target),
        container: Container::Card,
    };
    let singular_span = Span {
        start_index: 3,
        length: 1,
        first: 9,
        last: 9,
    };
    // These selections are valid by themselves, but cannot be smuggled into
    // the publication-bound aggregate namespace.
    for selection in [
        Selection::Cartesian {
            series: 7,
            span: singular_span,
            aggregation: Aggregation::Exact,
        },
        Selection::Candlestick {
            span: singular_span,
            aggregated: false,
        },
    ] {
        assert!(selection.is_valid());
        let target = Target::Aggregate {
            source: source(4),
            data_revision: 11,
            data_generation: 2,
            selection,
        };
        assert!(!target.is_valid());
        assert!(decode_chart_inspection_content(&bytes(&vec![present(target)])).is_err());
    }
    for target in [
        Target::Cartesian(0, 1),
        Target::Cartesian(1, -1),
        Target::Slice(0),
        Target::Radar(-1, 2),
        Target::Radar(1, 0),
        Target::Candlestick(0),
        Target::Node(-1),
        Target::Edge(0),
        Target::Aggregate {
            source: source(4),
            data_revision: 0,
            data_generation: 2,
            selection: aggregate(),
        },
        Target::Aggregate {
            source: source(4),
            data_revision: 11,
            data_generation: 0,
            selection: aggregate(),
        },
        Target::Aggregate {
            source: source(4),
            data_revision: 11,
            data_generation: 2,
            selection: Selection::Slice(9),
        },
        Target::Aggregate {
            source: source(4),
            data_revision: 11,
            data_generation: 2,
            selection: Selection::Cartesian {
                series: 7,
                span: span(),
                aggregation: Aggregation::Exact,
            },
        },
    ] {
        assert!(!target.is_valid());
        assert!(decode_chart_inspection_content(&bytes(&vec![present(target)])).is_err());
    }
    let duplicate = vec![
        present(Target::Slice(9)),
        Entry {
            target: Some(Target::Slice(9)),
            container: Container::Overlay,
        },
    ];
    assert!(!content::is_valid(&duplicate));
    assert!(decode_chart_inspection_content(&bytes(&duplicate)).is_err());
    for malformed in [
        vec![1, 2],
        vec![1, 1, 7],
        vec![1, 0, 2],
        vec![1, 1, 1, 9, 2],
        vec![1, 1, 6, 0, 0],
    ] {
        assert!(decode_chart_inspection_content(&malformed).is_err());
    }
}
#[test]
fn maximum_metadata_uses_long_integers_and_remains_bounded() {
    let entries = (0..content::MAX_ENTRIES)
        .map(|i| Entry {
            target: Some(Target::Aggregate {
                source: ResourceId::from_parts(i64::from(u32::MAX), i64::from(u32::MAX)).unwrap(),
                data_revision: i64::MAX - i as i64,
                data_generation: i64::MAX,
                selection: Selection::Cartesian {
                    series: i64::MAX,
                    span: Span {
                        start_index: 99998,
                        length: 2,
                        first: i64::MAX,
                        last: i64::MAX - 1,
                    },
                    aggregation: Aggregation::Mean,
                },
            }),
            container: Container::Overlay,
        })
        .collect::<Vec<_>>();
    assert!(content::is_valid(&entries));
    let encoded = bytes(&entries);
    assert!(encoded.len() < content::MAX_BYTES);
    assert_eq!(
        decode_chart_inspection_content(&encoded),
        Ok(entries.clone())
    );
    assert_eq!(
        content::heap_bytes(&entries),
        entries.capacity() * std::mem::size_of::<Entry>()
    );
    let over = vec![
        Entry {
            target: None,
            container: Container::Card
        };
        content::MAX_ENTRIES + 1
    ];
    assert!(!content::is_valid(&over));
    assert!(decode_chart_inspection_content(&bytes(&over)).is_err());
    assert_eq!(
        decode_chart_inspection_content(&vec![0; content::MAX_BYTES + 1]),
        Err(DecodeError::LimitExceeded)
    );
}
