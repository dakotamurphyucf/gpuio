use gpuio_native::{document_store::Store, highlight_projection::*};
use gpuio_protocol::{
    NodeId,
    highlight::{self, Appearance, Config, Query, Spec},
};
use std::{cell::Cell, sync::Arc};

fn key(slot: i64) -> RunKey {
    RunKey {
        node: NodeId::from_parts(slot, 1).unwrap(),
        fragment: 0,
    }
}
fn run(slot: i64, text: &str) -> Run {
    Run {
        key: key(slot),
        source: Source::Text(text.into()),
    }
}
fn group(kind: Kind, runs: Vec<Run>) -> Group {
    Group { kind, runs }
}
fn ordinary(runs: Vec<Run>) -> Group {
    group(Kind::Ordinary, runs)
}
fn spec(query: Option<&str>, ranges: &[(i64, i64)]) -> Spec {
    Spec {
        query: query.map(|s| Query {
            text: s.into(),
            case_sensitive: false,
            whole_word: false,
        }),
        ranges: ranges
            .iter()
            .map(|(start, end)| highlight::Range {
                start_byte: *start,
                end_byte: *end,
            })
            .collect(),
        appearance: Appearance {
            color: 1,
            active_color: 2,
            radius: 2.,
        },
        active_index: None,
        match_index_offset: 0,
    }
}
fn spans(matches: &Matches, slot: i64) -> Vec<(usize, i64, usize, usize)> {
    matches
        .spans
        .get(&key(slot))
        .into_iter()
        .flatten()
        .map(|s| (s.spec_index, s.ordinal, s.bytes.start, s.bytes.end))
        .collect()
}

#[test]
fn cross_run_matches_count_once_and_query_ordinals_precede_ranges() {
    let p = Projection::new(vec![ordinary(vec![
        run(0, "Hello "),
        run(1, ""),
        run(2, "café"),
    ])])
    .unwrap();
    assert_eq!(
        (
            p.source_bytes(),
            p.ordinary_bytes(),
            p.run_count(),
            p.group_count()
        ),
        (11, 11, 2, 1)
    );
    let config = Config(vec![spec(Some("hello café"), &[(0, 11), (6, 11)])]);
    let result = p.find(&config, || false).unwrap();
    assert_eq!(
        result.counts,
        vec![Count {
            total: 3,
            stored: 3
        }]
    );
    assert_eq!(spans(&result, 0), vec![(0, 0, 0, 6), (0, 1, 0, 6)]);
    assert_eq!(
        spans(&result, 2),
        vec![(0, 0, 0, 5), (0, 1, 0, 5), (0, 2, 0, 5)]
    );
    assert!(!result.spans.contains_key(&key(1)));
}

#[test]
fn native_groups_are_query_only_and_separators_do_not_paint_or_count() {
    let p = Projection::new(vec![
        ordinary(vec![run(0, "α")]),
        ordinary(vec![]),
        group(Kind::NativeDocument, vec![run(1, "α native")]),
        ordinary(vec![run(2, "β")]),
    ])
    .unwrap();
    assert_eq!(p.ordinary_bytes(), 5); // "α\nβ", excludes native source
    let result = p
        .find(&Config(vec![spec(Some("α"), &[(2, 3), (0, 5)])]), || false)
        .unwrap();
    assert_eq!(
        result.counts,
        vec![Count {
            total: 3,
            stored: 3
        }]
    );
    assert_eq!(spans(&result, 0), vec![(0, 0, 0, 2), (0, 2, 0, 2)]);
    assert_eq!(spans(&result, 1), vec![(0, 1, 0, 2)]);
    assert_eq!(spans(&result, 2), vec![(0, 2, 0, 2)]);
    assert_eq!(
        p.find(&Config(vec![spec(Some("αβ"), &[])]), || false)
            .unwrap()
            .counts[0]
            .total,
        0
    );
}

#[test]
fn invalid_ranges_identify_the_first_spec_and_range_without_partial_output() {
    let p = Projection::new(vec![ordinary(vec![run(0, "λ")])]).unwrap();
    for (range, reason) in [
        ((1, 2), RangeError::ScalarBoundary),
        ((0, 1), RangeError::ScalarBoundary),
        ((0, 3), RangeError::OutOfBounds),
        ((i64::MAX - 1, i64::MAX), RangeError::OutOfBounds),
    ] {
        let config = Config(vec![spec(Some("λ"), &[]), spec(None, &[(0, 2), range])]);
        assert_eq!(
            p.find(&config, || false),
            Err(Error::InvalidRange {
                spec_index: 1,
                range_index: 1,
                reason
            })
        );
    }
    assert_eq!(
        p.find(&Config(vec![spec(None, &[])]), || false),
        Err(Error::InvalidConfig)
    );
    let empty = Projection::new(vec![]).unwrap();
    assert_eq!(
        empty.find(&Config(vec![spec(None, &[(0, 1)])]), || false),
        Err(Error::InvalidRange {
            spec_index: 0,
            range_index: 0,
            reason: RangeError::OutOfBounds
        })
    );
}

#[test]
fn multiple_specs_overlap_and_cosmetic_updates_preserve_exact_match_mapping() {
    let p = Projection::new(vec![ordinary(vec![run(0, "ababa")])]).unwrap();
    let config = Config(vec![spec(Some("aba"), &[(2, 5)]), spec(Some("ba"), &[])]);
    let result = p.find(&config, || false).unwrap();
    assert_eq!(
        result.counts,
        vec![
            Count {
                total: 2,
                stored: 2
            };
            2
        ]
    );
    assert_eq!(
        spans(&result, 0),
        vec![(0, 0, 0, 3), (0, 1, 2, 5), (1, 0, 1, 3), (1, 1, 3, 5)]
    );
    let mut changed = config.clone();
    changed.0[0].appearance.color = 3;
    changed.0[0].active_index = Some(i64::MAX);
    changed.0[0].match_index_offset = i64::MAX - 1;
    assert_eq!(p.find(&changed, || false), Ok(result));
    assert!(changed.0[0].is_active(1));
    let empty = p.find(&Config(vec![]), || false).unwrap();
    assert!(empty.counts.is_empty() && empty.spans.is_empty());
}

#[test]
fn stored_matches_are_a_prefix_and_counts_remain_exact() {
    let p = Projection::new(vec![ordinary(vec![run(0, &"a".repeat(100000))])]).unwrap();
    let result = p
        .find(
            &Config(vec![spec(Some("a"), &[(0, 1)]), spec(Some("a"), &[])]),
            || false,
        )
        .unwrap();
    assert_eq!(
        result.counts,
        vec![
            Count {
                total: 100001,
                stored: 16384
            },
            Count {
                total: 100000,
                stored: 0
            }
        ]
    );
    assert_eq!(result.spans[&key(0)].len(), 16384);
    assert_eq!(result.spans[&key(0)].last().unwrap().ordinal, 16383);
}

#[test]
fn a_cross_run_match_is_never_partially_stored_when_span_limit_is_reached() {
    let runs = (0..MAX_RUNS).map(|i| run(i as i64, "a")).collect();
    let p = Projection::new(vec![ordinary(runs)]).unwrap();
    let result = p
        .find(
            &Config(vec![spec(None, &[(0, MAX_RUNS as i64); 3])]),
            || false,
        )
        .unwrap();
    assert_eq!(
        result.counts,
        vec![Count {
            total: 3,
            stored: 2
        }]
    );
    assert_eq!(
        result.spans.values().map(Vec::len).sum::<usize>(),
        MAX_PAINT_SPANS
    );
    for spans in result.spans.values() {
        assert_eq!(spans.len(), 2);
        assert_eq!(spans[0].ordinal, 0);
        assert_eq!(spans[1].ordinal, 1);
    }
}

#[test]
fn projections_bound_empty_inputs_shared_source_bytes_and_duplicate_keys() {
    assert!(matches!(
        Projection::new((0..=MAX_GROUPS).map(|_| ordinary(vec![])).collect()),
        Err(ProjectionError::GroupLimit)
    ));
    assert!(matches!(
        Projection::new(vec![ordinary(
            (0..=MAX_RUNS).map(|i| run(i as i64, "")).collect()
        )]),
        Err(ProjectionError::RunLimit)
    ));
    assert!(matches!(
        Projection::new(vec![ordinary(vec![run(0, "a"), run(0, "b")])]),
        Err(ProjectionError::DuplicateRun)
    ));
    let text: Arc<str> = "a".repeat(MAX_SOURCE_BYTES / 2 + 1).into();
    assert!(matches!(
        Projection::new(vec![ordinary(vec![
            Run {
                key: key(0),
                source: Source::Text(text.clone())
            },
            Run {
                key: key(1),
                source: Source::Text(text)
            }
        ])]),
        Err(ProjectionError::ByteLimit)
    ));
}

#[test]
fn rope_snapshot_survives_retirement_and_keeps_original_byte_offsets() {
    use gpuio_protocol::document::{Status, Update};
    let text = format!("{}λfind λfind", "x".repeat(8191));
    let mut store = Store::default();
    let id = store.create().unwrap();
    store
        .begin(Update {
            id,
            base: 0,
            revision: 1,
            generation: 1,
            from_byte: 0,
            suffix_bytes: text.len() as i64,
            status: Status::Complete,
        })
        .unwrap();
    store.chunk(id, 1, 0, text.as_bytes()).unwrap();
    store.publish(id, 1).unwrap();
    let snapshot = store.acquire(id).unwrap().snapshot();
    let p = Projection::new(vec![group(
        Kind::NativeDocument,
        vec![Run {
            key: key(0),
            source: Source::Document(snapshot),
        }],
    )])
    .unwrap();
    drop(store);
    let result = p
        .find(&Config(vec![spec(Some("λfind"), &[])]), || false)
        .unwrap();
    assert_eq!(
        result.counts,
        vec![Count {
            total: 2,
            stored: 2
        }]
    );
    assert_eq!(
        spans(&result, 0),
        vec![(0, 0, 8191, 8197), (0, 1, 8198, 8204)]
    );
    assert_eq!(p.ordinary_bytes(), 0);
    let calls = Cell::new(0);
    assert_eq!(
        p.find(&Config(vec![spec(Some("x"), &[])]), || {
            calls.set(calls.get() + 1);
            calls.get() > 5
        }),
        Err(Error::Cancelled)
    );
}

fn published(
    store: &mut Store,
    text: &str,
) -> (
    gpuio_protocol::ResourceId,
    Arc<gpuio_native::document_store::Snapshot>,
) {
    use gpuio_protocol::document::{Status, Update};
    let id = store.create().unwrap();
    store
        .begin(Update {
            id,
            base: 0,
            revision: 1,
            generation: 1,
            from_byte: 0,
            suffix_bytes: text.len() as i64,
            status: Status::Complete,
        })
        .unwrap();
    store.chunk(id, 1, 0, text.as_bytes()).unwrap();
    store.publish(id, 1).unwrap();
    (id, store.acquire(id).unwrap().snapshot())
}

fn page_projection(
    snapshot: Arc<gpuio_native::document_store::Snapshot>,
    bytes: std::ops::Range<usize>,
) -> Projection {
    Projection::new(vec![group(
        Kind::NativeDocument,
        vec![Run {
            key: key(0),
            source: Source::document_slice(snapshot, bytes).unwrap(),
        }],
    )])
    .unwrap()
}

#[test]
fn installed_pages_match_only_their_slice_with_local_byte_offsets() {
    let text = format!(
        "excluded λfind\n{}λfind λfind\nexcluded λfind",
        "x".repeat(8191)
    );
    let mut store = Store::default();
    let (id, snapshot) = published(&mut store, &text);
    let start = "excluded λfind\n".len();
    let end = start + 8191 + "λfind λfind".len();
    let page = page_projection(snapshot.clone(), start..end);
    assert_eq!(page.source_bytes(), end - start);
    assert_eq!(page.ordinary_bytes(), 0);
    let matches = page
        .find(&Config(vec![spec(Some("λfind"), &[])]), || false)
        .unwrap();
    assert_eq!(
        matches.counts,
        vec![Count {
            total: 2,
            stored: 2
        }]
    );
    assert_eq!(
        spans(&matches, 0),
        vec![(0, 0, 8191, 8197), (0, 1, 8198, 8204)]
    );
    // Full-document match ordinals must not leak into a mounted native page.
    assert!(page.same_source(&page_projection(snapshot.clone(), start..end)));
    assert!(!page.same_source(&page_projection(snapshot.clone(), 0..end)));
    store
        .begin(gpuio_protocol::document::Update {
            id,
            base: 1,
            revision: 2,
            generation: 1,
            from_byte: text.len() as i64,
            suffix_bytes: 0,
            status: gpuio_protocol::document::Status::Complete,
        })
        .unwrap();
    store.publish(id, 2).unwrap();
    let other_revision_identity = store.acquire(id).unwrap().snapshot();
    assert!(
        !page.same_source(&page_projection(other_revision_identity, start..end)),
        "identical page bytes from another installed snapshot must invalidate paints"
    );
}

#[test]
fn native_slices_validate_endpoints_and_keep_retired_snapshot_charge() {
    let mut store = Store::default();
    let (id, snapshot) = published(&mut store, "prefix é🙂 suffix");
    for bytes in [7..8, 0..8, 10..13] {
        assert!(matches!(
            Source::document_slice(snapshot.clone(), bytes),
            Err(RangeError::ScalarBoundary)
        ));
    }
    for (start, end) in [(9, 8), (0, usize::MAX), (100, 100)] {
        assert!(matches!(
            Source::document_slice(snapshot.clone(), start..end),
            Err(RangeError::OutOfBounds)
        ));
    }
    let end = snapshot.text.len();
    let empty = page_projection(snapshot.clone(), end..end);
    assert_eq!(empty.source_bytes(), 0);
    let page = page_projection(snapshot.clone(), 7..13);
    let weak = Arc::downgrade(&snapshot);
    drop(snapshot);
    store.release(id).unwrap();
    assert!(weak.upgrade().is_some());
    assert!(
        store.reserved_bytes() > 0,
        "page retains the original store reservation"
    );
    let found = page
        .find(&Config(vec![spec(Some("é🙂"), &[])]), || false)
        .unwrap();
    assert_eq!(spans(&found, 0), vec![(0, 0, 0, 6)]);
    drop(page);
    assert!(weak.upgrade().is_none());
    assert_eq!(store.reserved_bytes(), 0);
}
