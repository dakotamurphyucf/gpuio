use gpuio_native::{
    highlight_collect::{self as collect, DocumentError, Error},
    highlight_projection::{self as projection, Projection, RunKey, Source},
    tree::Tree,
};
use gpuio_protocol::{NodeId, WindowId, document, highlight::*, v1::*};
use std::sync::Arc;

fn id(slot: i64) -> NodeId {
    NodeId::from_parts(slot, 1).unwrap()
}
fn config() -> Config {
    Config(vec![Spec {
        query: Some(Query {
            text: "ab".into(),
            case_sensitive: true,
            whole_word: false,
        }),
        ranges: vec![],
        appearance: Appearance {
            color: 1,
            active_color: 2,
            radius: 2.,
        },
        active_index: None,
        match_index_offset: 0,
    }])
}
fn apply(tree: &mut Tree, operations: Vec<Op>) {
    tree.apply(&Transaction {
        window: WindowId::from_parts(0, 1).unwrap(),
        base: tree.revision(),
        revision: tree.revision() + 1,
        operations,
    })
    .unwrap();
}
fn tree(children: Vec<Op>, order: Vec<NodeId>) -> Tree {
    let mut tree = Tree::new(WindowId::from_parts(0, 1).unwrap());
    let mut operations = vec![
        Op::Create(id(0), Kind::HighlightScope, "".into(), None),
        Op::SetHighlightScope(id(0), config()),
    ];
    operations.extend(children);
    operations.extend([Op::Splice(id(0), 0, 0, order), Op::SetRoot(Some(id(0)))]);
    apply(&mut tree, operations);
    tree
}
fn text(slot: i64, text: &str) -> Op {
    Op::Create(id(slot), Kind::Text, text.into(), None)
}
fn plain(tree: &Tree, scope: NodeId) -> Projection {
    collect::collect(tree, scope, |_| true, |_| panic!("unexpected document")).unwrap()
}
fn totals(projection: &Projection, config: &Config) -> Vec<i64> {
    projection
        .find(config, || false)
        .unwrap()
        .counts
        .iter()
        .map(|c| c.total)
        .collect()
}
fn document(slot: i64) -> Vec<Op> {
    vec![
        Op::Create(id(slot), Kind::DocumentView, "".into(), None),
        Op::SetDocument(
            id(slot),
            document::Config {
                source: None,
                mode: document::Mode::Markdown,
                dark: true,
                layout: document::Layout::Flow,
                label: "ab metadata".into(),
                path: None,
                line_numbers: false,
                initially_collapsed: false,
                search: String::new(),
                images: vec![],
            },
        ),
    ]
}

#[test]
fn adjacent_leaf_runs_cross_empty_text_but_not_structural_or_nested_boundaries() {
    let t = tree(
        vec![
            text(1, "a"),
            text(2, ""),
            text(3, "b"),
            Op::Create(id(4), Kind::Container, "ab".into(), None),
            text(5, "a"),
            text(6, "b"),
            Op::Splice(id(4), 0, 0, vec![id(5), id(6)]),
            Op::Create(id(7), Kind::HighlightScope, "".into(), None),
            Op::SetHighlightScope(id(7), Config(vec![])),
            text(8, "ab"),
            Op::Splice(id(7), 0, 0, vec![id(8)]),
            text(9, "a"),
            Op::Create(id(10), Kind::Container, "".into(), None),
            text(11, "b"),
        ],
        vec![id(1), id(2), id(3), id(4), id(7), id(9), id(10), id(11)],
    );
    let p = plain(&t, id(0));
    assert_eq!(p.ordinary_bytes(), "ab\nab\nab\na\nb".len());
    assert_eq!(totals(&p, &config()), vec![3]);
    let matches = p.find(&config(), || false).unwrap();
    let key = |slot| RunKey {
        node: id(slot),
        fragment: 0,
    };
    assert_eq!(
        matches.spans[&key(1)][0].ordinal,
        matches.spans[&key(3)][0].ordinal
    );
    assert!(!matches.spans.contains_key(&key(8)));
    assert_eq!(plain(&t, id(7)).source_bytes(), 0);
    assert!(matches!(
        collect::collect(&t, id(8), |_| true, |_| Ok(vec![])),
        Err(Error::MissingScope)
    ));
}

#[test]
fn visibility_and_installed_document_text_preserve_structural_order_and_range_offsets() {
    let mut nodes = vec![
        text(1, "ab"),
        Op::Create(id(2), Kind::Container, "".into(), None),
        text(3, "ab hidden"),
        Op::Splice(id(2), 0, 0, vec![id(3)]),
    ];
    nodes.extend(document(4));
    nodes.push(text(5, "ab"));
    let t = tree(nodes, vec![id(1), id(2), id(4), id(5)]);
    let p = collect::collect(
        &t,
        id(0),
        |n| n.id != id(2),
        |n| {
            assert_eq!(n.id, id(4));
            // Rendered fragments, not Markdown source or toolbar metadata.
            Ok(vec![
                vec![Source::Text("a".into()), Source::Text("b".into())],
                vec![Source::Text("ab".into())],
            ])
        },
    )
    .unwrap();
    assert_eq!(p.ordinary_bytes(), 5); // ordinary "ab\nab", no document bytes
    assert_eq!(totals(&p, &config()), vec![4]);
    let m = p.find(&config(), || false).unwrap();
    assert_eq!(
        m.spans[&RunKey {
            node: id(4),
            fragment: 0
        }][0]
            .ordinal,
        1
    );
    assert_eq!(
        m.spans[&RunKey {
            node: id(4),
            fragment: 2
        }][0]
            .ordinal,
        2
    );
    assert_eq!(
        m.spans[&RunKey {
            node: id(5),
            fragment: 0
        }][0]
            .ordinal,
        3
    );
    let mut c = config();
    c.0[0].ranges.push(Range {
        start_byte: 0,
        end_byte: 5,
    });
    let m = p.find(&c, || false).unwrap();
    assert_eq!(m.counts[0].total, 5);
    assert_eq!(
        m.spans[&RunKey {
            node: id(4),
            fragment: 0
        }]
            .len(),
        1
    );
}

#[test]
fn unresolved_documents_never_publish_partial_counts_and_empty_or_range_only_scopes_do_not_wait() {
    let mut nodes = vec![text(1, "ab")];
    nodes.extend(document(2));
    let mut t = tree(nodes, vec![id(1), id(2)]);
    for error in [DocumentError::Pending, DocumentError::Unavailable] {
        assert!(
            matches!(collect::collect(&t,id(0),|_|true,|_|Err(error)),Err(Error::Document(e)) if e==error)
        );
    }
    let mut c = config();
    c.0[0].query = None;
    c.0[0].ranges.push(Range {
        start_byte: 0,
        end_byte: 2,
    });
    apply(&mut t, vec![Op::SetHighlightScope(id(0), c.clone())]);
    assert_eq!(totals(&plain(&t, id(0)), &c), vec![1]);
    apply(&mut t, vec![Op::SetHighlightScope(id(0), Config(vec![]))]);
    assert_eq!(
        collect::collect(
            &t,
            id(0),
            |_| panic!("empty scope walks nothing"),
            |_| panic!("empty scope reads no document")
        )
        .unwrap()
        .source_bytes(),
        0
    );
}

#[test]
fn cosmetic_and_unrelated_tree_updates_reuse_source_but_text_order_and_generations_do_not() {
    let mut t = tree(vec![text(1, "a"), text(2, "b")], vec![id(1), id(2)]);
    let original = Arc::new(plain(&t, id(0)));
    let mut c = config();
    c.0[0].appearance.radius = 3.;
    apply(
        &mut t,
        vec![
            Op::SetHighlightScope(id(0), c),
            Op::SetStyle(id(1), vec![Style::Opacity(0.5)]),
        ],
    );
    assert!(original.same_source(&plain(&t, id(0))));
    apply(&mut t, vec![Op::SetText(id(1), "a".into())]);
    assert!(original.same_source(&plain(&t, id(0))));
    apply(&mut t, vec![Op::Splice(id(0), 0, 2, vec![id(2), id(1)])]);
    assert!(!original.same_source(&plain(&t, id(0))));
    apply(&mut t, vec![Op::Splice(id(0), 0, 2, vec![id(1), id(2)])]);
    let fresh = NodeId::from_parts(1, 2).unwrap();
    apply(
        &mut t,
        vec![
            Op::Remove(id(1)),
            Op::Create(fresh, Kind::Text, "a".into(), None),
            Op::Splice(id(0), 0, 1, vec![fresh]),
        ],
    );
    assert!(!original.same_source(&plain(&t, id(0))));
    apply(&mut t, vec![Op::SetText(fresh, "z".into())]);
    assert_eq!(totals(&plain(&t, id(0)), &config()), vec![0]);
}

#[test]
fn collection_bounds_empty_native_groups_fragments_and_source_bytes_before_append() {
    let t = tree(document(1), vec![id(1)]);
    let run = |groups: collect::DocumentGroups| {
        collect::collect(&t, id(0), |_| true, |_| Ok(groups.clone()))
    };
    assert!(matches!(
        run(vec![vec![]; projection::MAX_GROUPS + 1]),
        Err(Error::Projection(projection::ProjectionError::GroupLimit))
    ));
    assert!(matches!(
        run(vec![vec![
            Source::Text("".into());
            projection::MAX_RUNS + 1
        ]]),
        Err(Error::Projection(projection::ProjectionError::RunLimit))
    ));
    let shared: Arc<str> = "x".repeat(1024 * 1024).into();
    assert!(matches!(
        run(vec![vec![Source::Text(shared); 17]]),
        Err(Error::Projection(projection::ProjectionError::ByteLimit))
    ));
}

#[test]
fn empty_tree_nodes_consume_visit_budget_and_do_not_grow_source_storage() {
    let mut t = tree(vec![], vec![]);
    let mut created = 0;
    while created < collect::MAX_VISITED_NODES {
        let count = (collect::MAX_VISITED_NODES - created).min(1024);
        let nodes: Vec<_> = (created + 1..=created + count)
            .map(|slot| id(slot as i64))
            .collect();
        let mut ops: Vec<_> = nodes
            .iter()
            .map(|node| Op::Create(*node, Kind::Text, "".into(), None))
            .collect();
        ops.push(Op::Splice(id(0), created as i64, 0, nodes));
        apply(&mut t, ops);
        created += count;
    }
    assert!(matches!(
        collect::collect(&t, id(0), |_| true, |_| Ok(vec![])),
        Err(Error::VisitLimit)
    ));
    // Hiding the scope avoids walking descendants; it is not counted as a
    // successful search through the hidden subtree.
    let p = collect::collect(&t, id(0), |_| false, |_| Ok(vec![])).unwrap();
    assert_eq!(p.source_bytes(), 0);
}
