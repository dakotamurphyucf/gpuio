//! Retained arbitrary inspection Views. Selection and source ownership stay native.
use super::*;
use crate::chart_geometry::Point;
use gpui::{AnyElement, FocusHandle, Pixels};
use gpuio_protocol::{
    chart_data::Contents,
    chart_inspection_content::{Container, Target},
    chart_selection::Selection,
};
use std::cell::Cell;

#[derive(Clone)]
struct Active {
    node: NodeId,
    target: Target,
    selection: Selection,
    index: usize,
    pointer: Option<Point>,
    bounds: Rc<Cell<Option<Bounds<Pixels>>>>,
}

pub(super) struct Content {
    active: RefCell<Option<Active>>,
    pub focus: FocusHandle,
    pub focused: Cell<bool>,
    pub pointer_inside: Cell<bool>,
    subscriptions: RefCell<Vec<gpui::Subscription>>,
}
impl Content {
    pub fn new(cx: &mut App) -> Self {
        Self {
            active: RefCell::new(None),
            focus: cx.focus_handle().tab_stop(false),
            focused: Cell::new(false),
            pointer_inside: Cell::new(false),
            subscriptions: RefCell::new(vec![]),
        }
    }
    pub fn clear(&self) {
        if let Some(active) = self.active.borrow_mut().take() {
            active.bounds.set(None);
        }
        self.focused.set(false);
        self.pointer_inside.set(false);
    }
}

pub(super) struct Position {
    pub node: NodeId,
    pub index: usize,
    pub container: Container,
    pub pointer: Option<Point>,
    pub bounds: Rc<Cell<Option<Bounds<Pixels>>>>,
}

fn present(target: Target, data: &Contents) -> bool {
    match (target, data) {
        (Target::Cartesian(series, datum), Contents::Cartesian(layers)) => layers.iter().any(|l| {
            let s = l.series();
            s.id == series && s.points.iter().any(|p| p.id == datum && p.y.is_some())
        }),
        (Target::Cartesian(series, datum), Contents::Categorical(_, layers)) => {
            layers.iter().any(|l| {
                let s = l.series();
                s.id == series && s.points.iter().any(|p| p.id == datum && p.value.is_some())
            })
        }
        (Target::Slice(id), Contents::Pie(values)) => {
            values.iter().any(|v| v.id == id && v.value > 0.)
        }
        (Target::Radar(series, axis), Contents::Radar(axes, values)) => {
            axes.iter().any(|a| a.id == axis)
                && values
                    .iter()
                    .any(|s| s.id == series && s.values.iter().any(|(id, _)| *id == axis))
        }
        (Target::Candlestick(id), Contents::Candlestick(values)) => {
            values.iter().any(|v| v.id == id)
        }
        (Target::Node(id), Contents::Sankey(values, _)) => values.iter().any(|v| v.id == id),
        (Target::Edge(id), Contents::Sankey(_, values)) => {
            values.iter().any(|v| v.id == id && v.value > 0.)
        }
        (Target::Aggregate { .. }, _) => false,
        _ => false,
    }
}

impl State {
    fn inspected_selection(&self, index: usize) -> Option<Selection> {
        let ready = self.ready.as_ref()?;
        crate::chart_selection::resolve(
            ready.snapshot.data(),
            &ready.config.sampling,
            ready.plan.geometry().marks.get(index)?.source,
        )
    }
    fn held_index(&self, active: &Active) -> Option<usize> {
        let ready = self.ready.as_ref()?;
        match active.target {
            Target::Aggregate {
                source,
                data_revision,
                data_generation,
                ..
            } => {
                if Some(source) != self.config.source
                    || data_revision != ready.snapshot.revision()
                    || data_generation != ready.snapshot.generation()
                {
                    return None;
                }
                if self.inspected_selection(active.index) == Some(active.selection) {
                    return Some(active.index);
                }
                // Only a changed prepared plan can move an aggregate's index.
                (0..ready.plan.geometry().marks.len())
                    .find(|i| self.inspected_selection(*i) == Some(active.selection))
            }
            Target::Cartesian(..)
            | Target::Slice(_)
            | Target::Radar(..)
            | Target::Candlestick(_)
            | Target::Node(_)
            | Target::Edge(_) => ready.plan.selection_index(active.selection),
        }
    }
    fn resolve_inspection(&self) -> Option<(Position, Target, Selection)> {
        if self.closed
            || self.config.inspection_content.is_empty()
            || self.config.disabled
            || self.input.data_cursor.is_some()
            || !self.config.style.inspection.card.visible
        {
            return None;
        }
        let ready = self.ready.as_ref()?;
        let live = self.lease.as_ref()?.snapshot()?;
        let source = self.config.source?;
        if ready.config.source != Some(source) || live.generation() != ready.snapshot.generation() {
            return None;
        }
        let held = self.content.focused.get() || self.content.pointer_inside.get();
        let previous = self.content.active.borrow();
        let index = if held {
            self.held_index(previous.as_ref()?)?
        } else {
            self.input.preview_index()?
        };
        let selection = self.inspected_selection(index)?;
        let target = Target::from_selection(
            selection,
            source,
            ready.snapshot.revision(),
            ready.snapshot.generation(),
        )?;
        if !Arc::ptr_eq(&live, &ready.snapshot) {
            // Aggregates never cross a publication. Exact targets can retain a
            // draft while replacement geometry is prepared, provided they exist.
            if !present(target, &live.data().contents) {
                return None;
            }
        }
        let (slot, entry) = self
            .config
            .inspection_content
            .iter()
            .enumerate()
            .find(|(_, entry)| entry.target == Some(target))?;
        let node = *self
            .label_slots
            .get(self.config.radar_labels.len() + slot)?;
        let same = previous
            .as_ref()
            .filter(|p| p.node == node && p.target == target);
        let pointer = if held {
            same.and_then(|p| p.pointer)
        } else {
            self.input.pointer
        };
        let bounds = same
            .map(|p| p.bounds.clone())
            .unwrap_or_else(|| Rc::new(Cell::new(None)));
        Some((
            Position {
                node,
                index,
                container: entry.container,
                pointer,
                bounds,
            },
            target,
            selection,
        ))
    }
    pub(super) fn inspection_position(&self) -> Option<Position> {
        let Some((position, target, selection)) = self.resolve_inspection() else {
            self.content.clear();
            return None;
        };
        let mut active = self.content.active.borrow_mut();
        if active
            .as_ref()
            .is_some_and(|a| a.node != position.node || a.target != target)
        {
            if let Some(old) = active.take() {
                old.bounds.set(None);
            }
            self.content.focused.set(false);
            self.content.pointer_inside.set(false);
        }
        *active = Some(Active {
            node: position.node,
            target,
            selection,
            index: position.index,
            pointer: position.pointer,
            bounds: position.bounds.clone(),
        });
        Some(position)
    }
    pub(super) fn hold_inspection(
        &self,
        point: gpui::Point<Pixels>,
        window: &Window,
        cx: &App,
    ) -> bool {
        let Some(position) = self.inspection_position() else {
            return false;
        };
        if !self.input.gate.borrow().allows(position.node) {
            return false;
        }
        let inside = position.bounds.get().is_some_and(|b| b.contains(&point));
        let focused = self.content.focus.contains_focused(window, cx);
        self.content.focused.set(focused);
        let capture = window.captured_hitbox().is_some() && self.content.pointer_inside.get();
        self.content.pointer_inside.set(inside || capture);
        inside || focused || capture
    }
}

pub(super) fn install(state: &Rc<RefCell<State>>, window: &mut Window, cx: &mut App) {
    let focus = state.borrow().content.focus.clone();
    let weak = Rc::downgrade(state);
    let enter = window.on_focus_in(&focus, cx, move |window, cx| {
        if let Some(state) = weak.upgrade() {
            let state = state.borrow();
            state.content.focused.set(true);
            state.redraw(window, cx);
        }
    });
    let weak = Rc::downgrade(state);
    let leave = window.on_focus_out(&focus, cx, move |_, window, cx| {
        if let Some(state) = weak.upgrade() {
            let state = state.borrow();
            state.content.focused.set(false);
            state.redraw(window, cx);
        }
    });
    state
        .borrow()
        .content
        .subscriptions
        .borrow_mut()
        .extend([enter, leave]);
}

pub(super) fn element(
    view: &mut View,
    tree: &crate::tree::Tree,
    position: &Position,
    mut interaction: Interaction,
    window: &mut Window,
    cx: &mut Context<View>,
) -> AnyElement {
    interaction.clip_controls = true;
    let element = view.element(tree, position.node, interaction, window, cx);
    table_view::clip_header_control(position.node, element, &view.focus)
}

#[cfg(test)]
mod tests {
    use super::*;
    use gpuio_protocol::chart_data::*;
    #[test]
    fn live_targets_require_their_family_and_present_source_values() {
        let numeric = Contents::Cartesian(vec![Layer::Line(Series {
            id: 3,
            name: "series".into(),
            points: vec![gpuio_protocol::chart_data::Point {
                id: 7,
                x: 0.,
                y: Some(1.),
                label: "".into(),
            }],
        })]);
        let categorical = Contents::Categorical(
            vec![Category {
                id: 1,
                label: "category".into(),
            }],
            vec![CategoricalLayer::Area(CategoricalSeries {
                id: 3,
                name: "series".into(),
                points: vec![CategoricalPoint {
                    id: 7,
                    category: 1,
                    value: Some(1.),
                    label: "".into(),
                }],
            })],
        );
        let families = [
            (Target::Cartesian(3, 7), numeric),
            (Target::Cartesian(3, 7), categorical),
            (
                Target::Slice(7),
                Contents::Pie(vec![Slice {
                    id: 7,
                    label: "slice".into(),
                    value: 1.,
                }]),
            ),
            (
                Target::Radar(3, 7),
                Contents::Radar(
                    [7, 8, 9]
                        .into_iter()
                        .map(|id| RadarAxis {
                            id,
                            label: format!("axis {id}"),
                            maximum: 10.,
                        })
                        .collect(),
                    vec![RadarSeries {
                        id: 3,
                        name: "series".into(),
                        values: vec![(7, 1.), (8, 2.), (9, 3.)],
                    }],
                ),
            ),
            (
                Target::Candlestick(7),
                Contents::Candlestick(vec![Candle {
                    id: 7,
                    x: 0.,
                    label: "candle".into(),
                    open_: 1.,
                    high: 1.,
                    low: 1.,
                    close: 1.,
                }]),
            ),
            (
                Target::Node(7),
                Contents::Sankey(
                    vec![Node {
                        id: 7,
                        label: "node".into(),
                    }],
                    vec![],
                ),
            ),
            (
                Target::Edge(7),
                Contents::Sankey(
                    [1, 2]
                        .into_iter()
                        .map(|id| Node {
                            id,
                            label: format!("node {id}"),
                        })
                        .collect(),
                    vec![Edge {
                        id: 7,
                        source: 1,
                        target: 2,
                        value: 1.,
                    }],
                ),
            ),
        ];
        for (target, contents) in &families {
            Data {
                version: 3,
                bar_baselines: vec![],
                bar_backgrounds: vec![],
                contents: contents.clone(),
            }
            .validate()
            .unwrap();
            for (expected, data) in &families {
                assert_eq!(present(*target, data), target == expected);
            }
        }
        let mut missing = families[0].1.clone();
        let Contents::Cartesian(layers) = &mut missing else {
            unreachable!()
        };
        let Layer::Line(series) = &mut layers[0] else {
            unreachable!()
        };
        series.points[0].y = None;
        assert!(!present(Target::Cartesian(3, 7), &missing));
        let mut missing = families[1].1.clone();
        let Contents::Categorical(_, layers) = &mut missing else {
            unreachable!()
        };
        let CategoricalLayer::Area(series) = &mut layers[0] else {
            unreachable!()
        };
        series.points[0].value = None;
        assert!(!present(Target::Cartesian(3, 7), &missing));
        assert!(!present(
            Target::Slice(7),
            &Contents::Pie(vec![Slice {
                id: 7,
                label: "zero".into(),
                value: 0.
            }])
        ));
        let mut zero_edge = families[6].1.clone();
        let Contents::Sankey(_, edges) = &mut zero_edge else {
            unreachable!()
        };
        edges[0].value = 0.;
        Data {
            version: 3,
            bar_baselines: vec![],
            bar_backgrounds: vec![],
            contents: zero_edge.clone(),
        }
        .validate()
        .unwrap();
        assert!(!present(Target::Edge(7), &zero_edge));
    }
}
