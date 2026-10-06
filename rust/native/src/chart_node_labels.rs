//! Expand immutable node-label metadata on the chart worker, before admission.
use crate::chart_geometry::{Error, Label, LabelKind, MAX_PLAN_BYTES, Plan};
use gpuio_protocol::{
    chart_data::{Contents, Data},
    chart_style::Style,
};
use std::{
    collections::BTreeMap,
    sync::atomic::{AtomicBool, Ordering},
};

pub(crate) fn apply(
    plan: &mut Plan,
    data: &Data,
    style: &Style,
    cancel: &AtomicBool,
) -> Result<(), Error> {
    let Contents::Sankey(nodes, _) = &data.contents else {
        return Ok(());
    };
    if style.node_labels.is_empty() {
        return Ok(());
    }
    let overrides: BTreeMap<_, _> = style
        .node_labels
        .iter()
        .map(|n| (n.node, &n.lines))
        .collect();
    let original = std::mem::take(&mut plan.labels);
    let mut expanded = Vec::with_capacity(original.len());
    for label in original {
        if cancel.load(Ordering::Relaxed) {
            return Err(Error::Cancelled);
        }
        let LabelKind::Flow {
            align_right,
            node_index,
        } = label.kind
        else {
            expanded.push(label);
            continue;
        };
        let node = nodes.get(node_index).ok_or(Error::InvalidInput)?;
        let Some(lines) = overrides.get(&node.id) else {
            expanded.push(label);
            continue;
        };
        let line_height = |size: Option<f64>| (size.unwrap_or(11.) + 4.).max(18.);
        let block_height = lines.iter().map(|l| line_height(l.font_size)).sum();
        let mut offset = 0.;
        for line in *lines {
            expanded.push(Label {
                position: label.position,
                text: line.text.clone(),
                kind: LabelKind::FlowLine {
                    align_right,
                    font_size: line.font_size.unwrap_or(11.),
                    color: line.color.map(|c| c as u32),
                    block_height,
                    offset,
                },
            });
            offset += line_height(line.font_size);
        }
    }
    plan.labels = expanded;
    if plan.retained_bytes() > MAX_PLAN_BYTES {
        Err(Error::RenderLimit)
    } else {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{chart_geometry::Source, chart_paint, chart_presentation::Frame};
    use gpuio_protocol::{
        chart_data::{Edge, Node as DataNode},
        chart_node_labels::{Line, Node},
        chart_options::Options,
        chart_view::Config,
    };

    fn data(reverse: bool) -> Data {
        let mut nodes = vec![
            DataNode {
                id: 10,
                label: "Original source".into(),
            },
            DataNode {
                id: 20,
                label: "Original target".into(),
            },
        ];
        if reverse {
            nodes.reverse();
        }
        Data {
            version: 1,
            contents: Contents::Sankey(
                nodes,
                vec![Edge {
                    id: 1,
                    source: 10,
                    target: 20,
                    value: 100.,
                }],
            ),
        }
    }
    fn style() -> Style {
        Style {
            node_labels: vec![
                Node {
                    node: 10,
                    lines: vec![
                        Line {
                            text: "100 units".into(),
                            color: Some(0xff0000ff),
                            font_size: Some(32.),
                        },
                        Line {
                            text: "Intake λ".into(),
                            color: None,
                            font_size: None,
                        },
                    ],
                },
                Node {
                    node: 20,
                    lines: vec![],
                },
                Node {
                    node: 99,
                    lines: vec![Line {
                        text: "Absent node".into(),
                        color: None,
                        font_size: None,
                    }],
                },
            ],
            ..Default::default()
        }
    }
    fn prepare(
        data: &Data,
        style: &Style,
        options: &Options,
        width: f64,
        height: f64,
    ) -> chart_paint::Prepared {
        chart_paint::prepare(
            data,
            Default::default(),
            options,
            style,
            chart_paint::Layout::new(width, height, 1.).unwrap(),
            &AtomicBool::new(false),
        )
        .unwrap()
    }
    #[test]
    fn overrides_follow_node_ids_preserve_geometry_and_hide_only_named_labels() {
        for reverse in [false, true] {
            let data = data(reverse);
            let ordinary = prepare(&data, &Style::default(), &Options::default(), 400., 200.);
            let rich = prepare(&data, &style(), &Options::default(), 400., 200.);
            assert_eq!(ordinary.geometry().marks, rich.geometry().marks);
            assert_eq!(
                rich.geometry()
                    .labels
                    .iter()
                    .map(|l| l.text.as_str())
                    .collect::<Vec<_>>(),
                ["100 units", "Intake λ"]
            );
            assert!(matches!(
                rich.geometry().labels[0].kind,
                LabelKind::FlowLine {
                    font_size: 32.,
                    color: Some(0xff0000ff),
                    offset: 0.,
                    block_height: 54.,
                    ..
                }
            ));
            assert!(matches!(
                rich.geometry().labels[1].kind,
                LabelKind::FlowLine {
                    font_size: 11.,
                    color: None,
                    offset: 36.,
                    ..
                }
            ));
            assert!(
                rich.geometry()
                    .marks
                    .iter()
                    .any(|m| matches!(m.source, Source::Edge(0)))
            );
            let mut long = style();
            for line in &mut long.node_labels[0].lines {
                line.text = "x".repeat(256);
            }
            let longer = prepare(&data, &long, &Options::default(), 400., 200.);
            assert!(
                longer.retained_bytes() >= rich.retained_bytes() + 480,
                "prepared label strings must contribute to retained admission"
            );
            let mut disabled = Options::default();
            disabled.sankey.labels = false;
            assert!(
                prepare(&data, &style(), &disabled, 400., 200.)
                    .geometry()
                    .labels
                    .is_empty()
            );
            let mut missing = style();
            missing.node_labels.retain(|n| n.node != 20);
            assert!(
                prepare(&data, &missing, &Options::default(), 400., 200.)
                    .geometry()
                    .labels
                    .iter()
                    .any(|l| l.text == "Original target")
            );
        }
    }
    #[test]
    fn multiline_blocks_clip_without_overlapping_in_tiny_views() {
        let data = data(false);
        for (width, height) in [(400., 200.), (80., 30.), (1., 1.)] {
            let config = Config {
                source: None,
                label: "Chart".into(),
                options: Default::default(),
                sampling: Default::default(),
                style: style(),
                legend: true,
                disabled: false,
            };
            let frame = Frame::new(width, height, &data, &config);
            let plan = prepare(
                &data,
                &config.style,
                &config.options,
                frame.plot.width,
                frame.plot.height,
            );
            let placed = plan
                .geometry()
                .labels
                .iter()
                .map(|l| frame.label(l))
                .collect::<Vec<_>>();
            assert_eq!(placed.len(), 2);
            assert!(placed[0].rect.y + placed[0].rect.height <= placed[1].rect.y);
            for p in placed {
                assert!(p.rect.x >= 0. && p.rect.x + p.rect.width <= width);
                assert!(p.rect.y >= frame.plot.y);
                assert!(p.rect.height >= 0.);
                assert!(p.rect.y + p.rect.height <= frame.plot.y + frame.plot.height);
            }
        }
    }
}
