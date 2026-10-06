//! Ordinary retained View children, measured by GPUI without entering OCaml.
use super::*;
use crate::chart_geometry::{LabelKind, Point};
use gpui::{AnyElement, AvailableSpace};
use gpuio_protocol::chart_data::Contents;
use std::f64::consts::{PI, TAU};

pub(super) struct Position {
    pub node: NodeId,
    pub axis: i64,
    anchor: Point,
    direction: Point,
}

impl State {
    pub(super) fn label_positions(&self) -> Vec<Position> {
        if self.label_slots.is_empty()
            || self.closed
            || self.input.data_cursor.is_some()
            || !self.config.options.radar.labels
        {
            return vec![];
        }
        let Some(ready) = &self.ready else {
            return vec![];
        };
        let Some(frame) = self.ready_frame else {
            return vec![];
        };
        let Some(live) = self.lease.as_ref().and_then(Lease::snapshot) else {
            return vec![];
        };
        if live.generation() != ready.snapshot.generation() {
            return vec![];
        }
        let (Contents::Radar(axes, _), Contents::Radar(current, _)) =
            (&ready.snapshot.data().contents, &live.data().contents)
        else {
            return vec![];
        };
        self.config
            .radar_labels
            .iter()
            .zip(self.label_slots.iter())
            .filter_map(|(axis, node)| {
                if !current.iter().any(|a| a.id == *axis) {
                    return None;
                }
                let index = axes.iter().position(|a| a.id == *axis)?;
                let label = ready
                    .plan
                    .geometry()
                    .labels
                    .iter()
                    .find(|label| label.kind == LabelKind::RadarAxis(*axis))?;
                let angle = index as f64 * TAU / axes.len() as f64 - PI / 2.;
                Some(Position {
                    node: *node,
                    axis: *axis,
                    anchor: Point {
                        x: frame.plot.x + label.position.x,
                        y: frame.plot.y + label.position.y,
                    },
                    direction: Point {
                        x: angle.cos(),
                        y: angle.sin(),
                    },
                })
            })
            .collect()
    }

    pub(super) fn hidden_labels(&self) -> Vec<NodeId> {
        let shown: std::collections::BTreeSet<_> =
            self.label_positions().iter().map(|p| p.node).collect();
        self.label_slots
            .iter()
            .copied()
            .filter(|id| !shown.contains(id))
            .collect()
    }

    pub(super) fn sync_label_visibility(&self) -> bool {
        self.input
            .gate
            .borrow_mut()
            .replace_chart_hidden(&self.label_slots, &self.hidden_labels())
    }
}

pub(super) fn element(
    labels: Vec<(Position, AnyElement)>,
    gate: super::super::focus::Shared,
) -> Option<AnyElement> {
    if labels.is_empty() {
        return None;
    }
    Some(
        canvas(
            move |bounds, window, cx| {
                let mut visible = Vec::with_capacity(labels.len());
                let mut clipped = vec![];
                for (position, mut element) in labels {
                    let size = element.layout_as_root(AvailableSpace::min_size(), window, cx);
                    let width = f32::from(size.width);
                    let height = f32::from(size.height);
                    if !width.is_finite()
                        || !height.is_finite()
                        || width <= 0.
                        || height <= 0.
                        || f64::from(width) > gpuio_protocol::canvas::COORDINATE_LIMIT
                        || f64::from(height) > gpuio_protocol::canvas::COORDINATE_LIMIT
                    {
                        clipped.push(position.node);
                        continue;
                    }
                    let origin = bounds.origin
                        + gpui::point(
                            px((position.anchor.x
                                + (position.direction.x - 1.) * f64::from(width) / 2.)
                                as f32),
                            px((position.anchor.y
                                + (position.direction.y - 1.) * f64::from(height) / 2.)
                                as f32),
                        );
                    element.prepaint_at(origin, window, cx);
                    visible.push(element);
                }
                (visible, clipped)
            },
            move |_, (labels, clipped), window, cx| {
                for node in clipped {
                    gate.borrow_mut().track_card_clipped(node, true);
                }
                for mut label in labels {
                    label.paint(window, cx);
                }
            },
        )
        .absolute()
        .top_0()
        .left_0()
        .size_full()
        .into_any_element(),
    )
}
