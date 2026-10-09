use super::*;
use gpuio_protocol::choice_picker::{
    Collection, Config, Group, Item, OpenState, Search, Selection,
};

fn config(ids: &[&str]) -> Config {
    Config {
        label: "Pick".into(),
        options: Collection::Flat(
            ids.iter()
                .map(|id| Item {
                    id: (*id).into(),
                    label: (*id).into(),
                    disabled: false,
                })
                .collect(),
        ),
        selected: Selection::Single(None),
        disabled: false,
        search: Search::None,
        clearable: true,
        open_state: OpenState::Managed(false),
        placeholder: String::new(),
        search_placeholder: String::new(),
    }
}
fn projection(config: Config) -> Arc<Projection> {
    Arc::new(Projection::new(Arc::new(config), "").unwrap())
}

#[test]
fn mixed_row_anchor_survives_regrouping_removal_and_content_updates() {
    let mut state = State::new(projection(config(&["a", "b", "c"])), 32., 64.).unwrap();
    state.handle().scroll_to(ListOffset {
        item_ix: 1,
        offset_in_item: px(12.5),
    });
    let mut grouped = config(&[]);
    let Collection::Flat(mut items) = config(&["c", "b", "a"]).options else {
        unreachable!()
    };
    grouped.options = Collection::Grouped(vec![Group {
        id: "b".into(),
        label: "Section".into(),
        items: items.clone(),
    }]);
    state.replace(projection(grouped.clone()));
    assert_offset(
        state.handle().logical_scroll_top(),
        ListOffset {
            item_ix: 2,
            offset_in_item: px(12.5),
        },
    );
    state.invalidate(Key::Group("b"));
    state.invalidate(Key::Item("b"));
    state.invalidate(Key::Item("missing"));
    assert_eq!(state.handle().logical_scroll_top().offset_in_item, px(12.5));
    items[1].label = "new content".into();
    grouped.selected = Selection::Single(Some("b".into()));
    grouped.options = Collection::Grouped(vec![Group {
        id: "b".into(),
        label: "Changed section".into(),
        items,
    }]);
    state.replace(projection(grouped));
    assert_offset(
        state.handle().logical_scroll_top(),
        ListOffset {
            item_ix: 2,
            offset_in_item: px(12.5),
        },
    );
    state.replace(projection(config(&["c", "a"]))); // next survivor after old item b is a
    assert_offset(
        state.handle().logical_scroll_top(),
        ListOffset {
            item_ix: 1,
            offset_in_item: px(0.),
        },
    );
    state.replace(projection(config(&["c"]))); // no successor: previous survivor c
    assert_eq!(state.handle().logical_scroll_top().item_ix, 0);
    state.replace(projection(config(&[])));
    assert_eq!(state.handle().item_count(), 0);
    assert_offset(
        state.handle().logical_scroll_top(),
        ListOffset {
            item_ix: 0,
            offset_in_item: px(0.),
        },
    );
}

#[test]
fn invalid_measurement_policy_is_rejected_and_unchanged_projection_keeps_scroll() {
    let rows = projection(config(&["a", "b", "c"]));
    for (height, overdraw) in [
        (f32::NAN, 0.),
        (0., 0.),
        (1_000_001., 0.),
        (32., -1.),
        (32., f32::INFINITY),
        (32., 4097.),
    ] {
        assert!(matches!(
            State::new(rows.clone(), height, overdraw),
            Err(InvalidGeometry)
        ));
    }
    let mut state = State::new(rows.clone(), 32., 0.).unwrap();
    state.handle().scroll_to(ListOffset {
        item_ix: 1,
        offset_in_item: px(10.),
    });
    state.replace(rows);
    assert_eq!(state.handle().logical_scroll_top().offset_in_item, px(10.));
    state.replace(projection(config(&["a", "b", "c"])));
    assert_offset(
        state.handle().logical_scroll_top(),
        ListOffset {
            item_ix: 1,
            offset_in_item: px(10.),
        },
    );
}

#[cfg(feature = "native-image-tests")]
#[test]
fn gpui_measures_mixed_heights_virtualizes_and_preserves_streaming_anchor() {
    use crate::choice_picker_rows::Row;
    use gpui::{Context, Render, TestAppContext, Window, div, list, point, prelude::*, size};
    use std::{cell::Cell, rc::Rc};
    let mut app = TestAppContext::single();
    let cx = app.add_empty_window();
    let ids: Vec<_> = (0..4096).map(|i| i.to_string()).collect();
    let mut value = config(&[]);
    value.options = Collection::Grouped(vec![Group {
        id: "header".into(),
        label: "Header".into(),
        items: ids
            .iter()
            .enumerate()
            .map(|(i, id)| Item {
                id: id.clone(),
                label: if i % 2 == 0 { "short" } else { "rich" }.into(),
                disabled: false,
            })
            .collect(),
    }]);
    let mut state = State::new(projection(value.clone()), 32., 32.).unwrap();
    let count = Rc::new(Cell::new(0));
    struct TestView {
        handle: ListState,
        rows: Arc<Projection>,
        measured: Rc<Cell<usize>>,
    }
    impl Render for TestView {
        fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
            let rows = self.rows.clone();
            let measured = self.measured.clone();
            list(self.handle.clone(), move |row, _, _| {
                measured.set(measured.get() + 1);
                let height = match rows.row(row).unwrap() {
                    Row::Header(_) => 18.,
                    Row::Item(item) => {
                        if item.label == "short" {
                            28.
                        } else {
                            52.
                        }
                    }
                };
                div().w_full().h(px(height)).into_any_element()
            })
            .w_full()
            .h_full()
        }
    }
    let owner = cx.update(|_, cx| {
        cx.new(|_| TestView {
            handle: state.handle().clone(),
            rows: state.projection().clone(),
            measured: count.clone(),
        })
    });
    let mut draw = |state: &State| {
        count.set(0);
        cx.update(|_, cx| {
            owner.update(cx, |view, cx| {
                view.rows = state.projection().clone();
                cx.notify();
            })
        });
        cx.draw(point(px(0.), px(0.)), size(px(280.), px(160.)), |_, _| {
            owner.clone().into_any_element()
        });
        assert!(
            count.get() > 0 && count.get() < 32,
            "measured {} rows",
            count.get()
        );
    };
    draw(&state);
    assert_eq!(
        state.handle().bounds_for_item(0).unwrap().size.height,
        px(18.)
    );
    assert_eq!(
        state.handle().bounds_for_item(1).unwrap().size.height,
        px(28.)
    );
    assert_eq!(
        state.handle().bounds_for_item(2).unwrap().size.height,
        px(52.)
    );
    state.handle().scroll_to(ListOffset {
        item_ix: 101,
        offset_in_item: px(7.),
    });
    draw(&state);
    let before = state.handle().logical_scroll_top();
    assert_eq!(
        state.projection().row(before.item_ix).unwrap().key(),
        Key::Item("100")
    );
    let Collection::Grouped(groups) = &mut value.options else {
        unreachable!()
    };
    groups[0].items[99].label = "short".into(); // preceding visible row shrinks
    state.replace(projection(value.clone()));
    draw(&state);
    assert_offset(state.handle().logical_scroll_top(), before);
    let Collection::Grouped(groups) = &mut value.options else {
        unreachable!()
    };
    groups[0].items.remove(0);
    state.replace(projection(value));
    draw(&state);
    let after = state.handle().logical_scroll_top();
    assert_eq!(
        state.projection().row(after.item_ix).unwrap().key(),
        Key::Item("100")
    );
    assert_eq!(after.offset_in_item, before.offset_in_item);
}

fn assert_offset(actual: ListOffset, expected: ListOffset) {
    assert_eq!(actual.item_ix, expected.item_ix);
    assert_eq!(actual.offset_in_item, expected.offset_in_item);
}
