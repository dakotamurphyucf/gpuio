use super::*;
use gpuio_protocol::choice_picker::{MAX_GROUPS, MAX_ITEMS, OpenState};

fn item(id: &str, label: &str, disabled: bool) -> Item {
    Item {
        id: id.into(),
        label: label.into(),
        disabled,
    }
}
fn group(id: &str, items: Vec<Item>) -> Group {
    Group {
        id: id.into(),
        label: id.into(),
        items,
    }
}
fn config() -> Config {
    Config {
        label: "Pick".into(),
        options: Collection::Grouped(vec![
            group("empty", vec![]),
            group(
                "a",
                vec![
                    item("a", "Alpha", false),
                    item("locked", "ALPHA locked", true),
                ],
            ),
            group(
                "other",
                vec![
                    item("b", "Beta", false),
                    item("c", "same label", false),
                    item("d", "same label", false),
                ],
            ),
        ]),
        selected: Selection::Multiple(vec!["locked".into(), "b".into()]),
        disabled: false,
        search: Search::Substring,
        clearable: true,
        open_state: OpenState::Managed(false),
        placeholder: String::new(),
        search_placeholder: String::new(),
    }
}
fn keys(projection: &Projection) -> Vec<Key<'_>> {
    (0..projection.len())
        .map(|i| projection.row(i).unwrap().key())
        .collect()
}

#[test]
fn grouped_projection_preserves_identity_order_and_hidden_committed_selection() {
    let config = Arc::new(config());
    let all = Projection::new(config.clone(), "").unwrap();
    assert_eq!(
        keys(&all),
        [
            Key::Group("empty"),
            Key::Group("a"),
            Key::Item("a"),
            Key::Item("locked"),
            Key::Group("other"),
            Key::Item("b"),
            Key::Item("c"),
            Key::Item("d")
        ]
    );
    assert_eq!(all.item_count(), 5);
    assert!(!all.is_selected(1)); // group "a" is distinct from item "a"
    assert!(all.is_selected(3)); // disabled selection remains visible
    assert!(!all.is_enabled(3));
    assert!(all.is_selected(5));
    assert!(!all.is_selected(usize::MAX));
    assert!(all.row(all.len()).is_none());
    let filtered = Projection::new(config.clone(), "aLpHa").unwrap();
    assert_eq!(
        keys(&filtered),
        [Key::Group("a"), Key::Item("a"), Key::Item("locked")]
    );
    assert!(filtered.is_selected(2));
    assert_eq!(
        filtered.config().selected,
        Selection::Multiple(vec!["locked".into(), "b".into()])
    );
    assert!(Arc::ptr_eq(filtered.config(), &config));
    let none = Projection::new(config.clone(), "does not match").unwrap();
    assert!(none.is_empty());
    assert_eq!(none.item_count(), 0);
    assert_eq!(none.config().selected, config.selected);
    let same = Projection::new(config, "same label").unwrap();
    assert_eq!(
        keys(&same),
        [Key::Group("other"), Key::Item("c"), Key::Item("d")]
    );
}

#[test]
fn cursor_skips_headers_and_disabled_choices_and_preserves_ids_across_reorder() {
    let config = Arc::new(config());
    let all = Projection::new(config.clone(), "").unwrap();
    let mut cursor = Cursor::default();
    assert!(cursor.reconcile(&all, true));
    assert_eq!(cursor.active_id(), Some("b")); // skips first, disabled selected ID
    assert_eq!(cursor.active_row(&all), Some(5));
    for invalid in [0, 1, 3, 4, usize::MAX] {
        assert!(!cursor.highlight(&all, invalid));
    }
    assert_eq!(cursor.active_id(), Some("b"));
    for (direction, expected) in [
        (Navigation::First, "a"),
        (Navigation::Previous, "d"),
        (Navigation::Next, "a"),
        (Navigation::Last, "d"),
        (Navigation::Previous, "c"),
    ] {
        assert!(cursor.navigate(&all, direction));
        assert_eq!(cursor.active_id(), Some(expected));
    }
    let mut reordered = (*config).clone();
    let Collection::Grouped(groups) = &mut reordered.options else {
        unreachable!()
    };
    groups.reverse();
    groups[0].items.reverse();
    let reordered = Projection::new(Arc::new(reordered), "").unwrap();
    assert!(!cursor.reconcile(&reordered, false));
    assert_eq!(cursor.active_id(), Some("c"));
    assert_eq!(cursor.active_row(&reordered), Some(2));
    let filtered = Projection::new(config.clone(), "alpha").unwrap();
    assert!(cursor.reconcile(&filtered, false));
    assert_eq!(cursor.active_id(), Some("a"));
    let empty = Projection::new(config, "no results").unwrap();
    assert!(cursor.reconcile(&empty, false));
    assert_eq!(cursor.active_id(), None);
    assert!(!cursor.navigate(&empty, Navigation::Next));
    assert!(cursor.navigate(&all, Navigation::Previous));
    assert_eq!(cursor.active_id(), Some("d"));
}

#[test]
fn query_modes_unicode_and_empty_group_semantics_are_explicit() {
    let mut config = config();
    config.options = Collection::Flat(vec![item("lambda", "Λάμδα", false), item("e", "é", false)]);
    config.selected = Selection::Single(Some("lambda".into()));
    let unicode = Projection::new(Arc::new(config.clone()), "λά").unwrap();
    assert_eq!(keys(&unicode), [Key::Item("lambda")]);
    let unnormalized = Projection::new(Arc::new(config.clone()), "e\u{301}").unwrap();
    assert!(unnormalized.is_empty()); // no accent folding or normalization
    for search in [Search::None, Search::Application] {
        config.search = search;
        let projection = Projection::new(Arc::new(config.clone()), "unmatched query").unwrap();
        assert_eq!(projection.item_count(), 2); // application supplied catalog is authoritative
    }
    config.options = Collection::Grouped(vec![group("empty", vec![])]);
    config.selected = Selection::Single(None);
    let projection = Projection::new(Arc::new(config), "").unwrap();
    assert_eq!(projection.len(), 1);
    assert_eq!(projection.item_count(), 0); // renderer must show the empty slot
    let mut cursor = Cursor::default();
    assert!(!cursor.reconcile(&projection, true));
    assert!(!cursor.navigate(&projection, Navigation::Last));
}

#[test]
fn availability_and_selection_order_do_not_depend_on_positions() {
    let mut config = config();
    config.selected = Selection::Multiple(vec!["d".into(), "c".into()]);
    let all = Projection::new(Arc::new(config.clone()), "").unwrap();
    let mut cursor = Cursor::default();
    cursor.reconcile(&all, true);
    assert_eq!(cursor.active_id(), Some("d"));
    config.disabled = true;
    let disabled = Projection::new(Arc::new(config.clone()), "").unwrap();
    assert!(cursor.reconcile(&disabled, false));
    assert_eq!(cursor.active_id(), None);
    assert!(!cursor.navigate(&disabled, Navigation::First));
    assert!(disabled.is_selected(disabled.item_row("d").unwrap()));
    config.disabled = false;
    config.selected = Selection::Single(None);
    let enabled = Projection::new(Arc::new(config), "").unwrap();
    cursor.reconcile(&enabled, true);
    assert_eq!(cursor.active_id(), Some("a"));
}

#[test]
fn maximum_catalog_has_bounded_projection_and_invalid_inputs_are_rejected() {
    let mut config = config();
    config.options = Collection::Grouped(
        (0..MAX_GROUPS)
            .map(|g| {
                group(
                    &format!("g{g}"),
                    (0..MAX_ITEMS / MAX_GROUPS)
                        .map(|i| item(&format!("{g}-{i}"), "item", false))
                        .collect(),
                )
            })
            .collect(),
    );
    config.selected = Selection::Single(None);
    let config = Arc::new(config);
    let projection = Projection::new(config.clone(), "").unwrap();
    assert_eq!(projection.len(), MAX_GROUPS + MAX_ITEMS);
    assert_eq!(projection.item_count(), MAX_ITEMS);
    assert_eq!(projection.enabled_rows.len(), MAX_ITEMS);
    assert_eq!(projection.selected_rows.len(), MAX_GROUPS + MAX_ITEMS);
    let mut cursor = Cursor::default();
    for _ in 0..MAX_ITEMS + 1 {
        cursor.navigate(&projection, Navigation::Next);
    }
    assert_eq!(cursor.active_id(), Some("0-0"));
    for query in [
        "x\0y".into(),
        "x\ny".into(),
        "x\ry".into(),
        "x".repeat(MAX_QUERY_BYTES + 1),
    ] {
        assert!(matches!(
            Projection::new(config.clone(), &query),
            Err(Error::InvalidQuery)
        ));
    }
    assert!(
        Projection::new(config.clone(), &"x".repeat(MAX_QUERY_BYTES))
            .unwrap()
            .is_empty()
    );
    let mut invalid = (*config).clone();
    invalid.label.clear();
    assert!(matches!(
        Projection::new(Arc::new(invalid), ""),
        Err(Error::InvalidConfig)
    ));
}

#[test]
fn membership_counts_filtered_logical_groups_including_disabled_options() {
    let original = config();
    let collect = |config: Config, query: &str| {
        let projection = Projection::new(Arc::new(config), query).unwrap();
        assert!(projection.membership(usize::MAX).is_none());
        (0..projection.len())
            .filter_map(|index| {
                let membership = projection.membership(index)?;
                let Key::Item(id) = projection.row(index).unwrap().key() else {
                    panic!("header membership")
                };
                Some((
                    id.to_string(),
                    membership.index,
                    membership.count,
                    membership.group.map(|g| g.id.clone()),
                ))
            })
            .collect::<Vec<_>>()
    };
    assert_eq!(
        collect(original.clone(), "alpha"),
        vec![
            ("a".into(), 0, 2, Some("a".into())),
            ("locked".into(), 1, 2, Some("a".into())),
        ]
    );
    assert_eq!(
        collect(original.clone(), "same"),
        vec![
            ("c".into(), 0, 2, Some("other".into())),
            ("d".into(), 1, 2, Some("other".into())),
        ]
    );
    let mut reordered = original.clone();
    let Collection::Grouped(groups) = &mut reordered.options else {
        unreachable!()
    };
    groups.reverse();
    groups[0].items.reverse();
    assert_eq!(
        collect(reordered, "same"),
        vec![
            ("d".into(), 0, 2, Some("other".into())),
            ("c".into(), 1, 2, Some("other".into())),
        ]
    );
    let mut flat = original.clone();
    flat.options = Collection::Flat(vec![
        item("locked", "match disabled", true),
        item("b", "nonmatching", false),
        item("c", "match enabled", false),
    ]);
    assert_eq!(
        collect(flat, "match "),
        vec![("locked".into(), 0, 2, None), ("c".into(), 1, 2, None)]
    );
    assert!(collect(original, "absent").is_empty());
}
