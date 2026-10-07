use super::*;
use crate::{chart_node_labels, chart_presentation::Frame};
use gpuio_protocol::{
    chart_node_labels::{Line, Node as LabelNode},
    chart_style::Style,
    chart_view::Config,
};

fn data(reverse: bool) -> data::Data {
    let mut nodes: Vec<_> = [10, 20, 30]
        .into_iter()
        .map(|id| data::Node {
            id,
            label: format!("Stage {id} λ 👨‍👩‍👧‍👦"),
        })
        .collect();
    if reverse {
        nodes.reverse();
    }
    data::Data {
        version: 2,
        bar_backgrounds: vec![],
        contents: data::Contents::Sankey(
            nodes,
            vec![
                data::Edge {
                    id: 1,
                    source: 10,
                    target: 20,
                    value: 100.,
                },
                data::Edge {
                    id: 2,
                    source: 20,
                    target: 30,
                    value: 100.,
                },
                data::Edge {
                    id: 3,
                    source: 10,
                    target: 30,
                    value: 0.,
                },
            ],
        ),
    }
}
fn metrics(data: &data::Data) -> Vec<Option<LabelMetrics>> {
    let data::Contents::Sankey(nodes, _) = &data.contents else {
        unreachable!()
    };
    nodes
        .iter()
        .map(|n| {
            Some(match n.id {
                10 => LabelMetrics {
                    width: 50.,
                    height: 18.,
                },
                20 => LabelMetrics {
                    width: 90.,
                    height: 54.,
                },
                _ => LabelMetrics {
                    width: 900.,
                    height: 18.,
                },
            })
        })
        .collect()
}
fn prepared(
    data: &data::Data,
    options: &options::Options,
    size: (f64, f64),
    labels: &[Option<LabelMetrics>],
) -> Plan {
    prepare_with_labels(
        data,
        Default::default(),
        options,
        size,
        Some(labels),
        &AtomicBool::new(false),
    )
    .unwrap()
}
fn node(plan: &Plan, index: usize) -> Rect {
    match plan
        .marks
        .iter()
        .find(|m| m.source == Source::Node(index))
        .unwrap()
        .shape
    {
        Shape::Node(r) => r,
        _ => unreachable!(),
    }
}
// The pinned layout computes in f32; compare logical coordinates within its
// rounding error, while keeping exact source identity and label policies.
fn close(actual: f64, expected: f64) {
    assert!(
        (actual - expected).abs() <= 0.0002,
        "{actual} != {expected}"
    );
}
#[test]
fn three_columns_use_measured_margins_and_preserve_source_indices() {
    for reverse in [false, true] {
        let data = data(reverse);
        let data::Contents::Sankey(nodes, _) = &data.contents else {
            unreachable!()
        };
        for alignment in [
            options::Alignment::Left,
            options::Alignment::Right,
            options::Alignment::Center,
            options::Alignment::Justify,
        ] {
            let mut options = options::Options::default();
            options.sankey.alignment = alignment;
            for scale in [options::FlowScale::Linear, options::FlowScale::Sqrt] {
                options.sankey.scale = scale;
                let plan = prepared(&data, &options, (500., 200.), &metrics(&data));
                let hits = crate::chart_hit::Index::prepare(
                    &plan,
                    options.cartesian.orientation,
                    3.,
                    &AtomicBool::new(false),
                )
                .unwrap();
                for index in 0..3 {
                    let r = node(&plan, index);
                    let selected = hits
                        .query(
                            &plan,
                            Point::new((r.left + r.right) / 2., (r.top + r.bottom) / 2.),
                            false,
                        )
                        .unwrap();
                    assert_eq!(plan.marks[selected].source, Source::Node(index));
                }
                assert_eq!(plan.labels.len(), 3);
                assert_eq!(plan.marks.len(), 5, "zero edge remains absent");
                for (i, n) in nodes.iter().enumerate() {
                    let bounds = node(&plan, i);
                    close(bounds.top, 60.);
                    close(bounds.bottom, 196.);
                    let label = plan.labels.iter().find(|l| matches!(l.kind, LabelKind::Flow { node_index, .. } if node_index == i)).unwrap();
                    assert_eq!(label.text, n.label);
                    let LabelKind::Flow {
                        placement: Some(p), ..
                    } = label.kind
                    else {
                        panic!("measured placement")
                    };
                    match n.id {
                        10 => {
                            close(bounds.left, 56.);
                            close(label.position.x, 50.);
                            assert_eq!(p.align, FlowAlign::Right);
                            assert_eq!(p.width, 50.);
                            assert!(!p.above);
                        }
                        20 => {
                            assert_eq!(p.align, FlowAlign::Center);
                            assert!(p.above);
                            assert_eq!(label.position.y, bounds.top - 6.);
                            assert_eq!(p.block_height, 54.);
                        }
                        30 => {
                            close(bounds.right, 400.);
                            close(label.position.x, 406.);
                            assert_eq!(p.align, FlowAlign::Left);
                            assert_eq!(p.width, 94., "long text has capped margin");
                        }
                        _ => unreachable!(),
                    }
                }
                for mark in &plan.marks {
                    if let Shape::Ribbon {
                        start_top,
                        start_bottom,
                        end_top,
                        end_bottom,
                    } = mark.shape
                    {
                        for pt in [start_top, start_bottom, end_top, end_bottom] {
                            assert!((55.9998..=400.0002).contains(&pt.x));
                            assert!((59.9998..=196.0002).contains(&pt.y));
                        }
                    }
                }
            }
        }
    }
}
#[test]
fn absent_and_disabled_labels_reserve_no_space() {
    let data = data(false);
    let mut options = options::Options::default();
    let hidden = prepared(&data, &options, (500., 200.), &[None; 3]);
    let ordinary = prepare(
        &data,
        Default::default(),
        &options,
        500.,
        200.,
        &AtomicBool::new(false),
    )
    .unwrap();
    assert_eq!(hidden.marks, ordinary.marks);
    assert!(hidden.labels.is_empty());
    options.sankey.labels = false;
    let disabled = prepared(&data, &options, (500., 200.), &metrics(&data));
    assert_eq!(disabled.marks, ordinary.marks);
    assert!(disabled.labels.is_empty());
    options.sankey.labels = true;
    let mut labels = metrics(&data);
    labels[0] = None;
    labels[1] = None;
    let end_only = prepared(&data, &options, (500., 200.), &labels);
    close(node(&end_only, 0).left, 0.);
    close(node(&end_only, 1).top, 0.);
    assert_eq!(end_only.labels.len(), 1);
}
#[test]
fn rich_middle_block_keeps_placement_and_clips_coherently_in_tiny_views() {
    let data = data(false);
    let style = Style {
        node_labels: vec![LabelNode {
            node: 20,
            lines: vec![
                Line {
                    text: "Large λ".into(),
                    font_size: Some(32.),
                    color: None,
                },
                Line {
                    text: "Caption".into(),
                    font_size: None,
                    color: None,
                },
            ],
        }],
        ..Default::default()
    };
    let config = Config {
        version: -2,
        radar_labels: vec![],
        inspection_content: vec![],
        source: None,
        label: "Chart".into(),
        options: Default::default(),
        sampling: Default::default(),
        style: style.clone(),
        legend: false,
        disabled: false,
    };
    for (width, height) in [(500., 200.), (60., 20.), (1., 1.), (0.125, 0.25)] {
        let frame = Frame::new(width, height, &data, &config);
        let mut plan = prepared(
            &data,
            &config.options,
            (frame.plot.width, frame.plot.height),
            &metrics(&data),
        );
        chart_node_labels::apply(&mut plan, &data, &style, &AtomicBool::new(false)).unwrap();
        let lines: Vec<_> = plan
            .labels
            .iter()
            .filter(|l| matches!(l.kind, LabelKind::FlowLine { .. }))
            .map(|l| frame.label(l))
            .collect();
        assert_eq!(lines.len(), 2);
        assert!(lines[0].rect.y + lines[0].rect.height <= lines[1].rect.y + 1e-8);
        for label in &plan.labels {
            let p = frame.label(label);
            let r = p.rect;
            assert!(r.x >= frame.plot.x && r.y >= frame.plot.y);
            assert!(r.width >= 0. && r.height >= 0.);
            assert!(r.x + r.width <= frame.plot.x + frame.plot.width + 1e-8);
            assert!(r.y + r.height <= frame.plot.y + frame.plot.height + 1e-8);
        }
        for mark in &plan.marks {
            if let Shape::Node(r) = mark.shape {
                assert!(r.left >= 0. && r.top >= 0.);
                assert!(r.right <= frame.plot.width + 1e-6);
                assert!(r.bottom <= frame.plot.height + 1e-6);
                assert!(r.right >= r.left && r.bottom >= r.top);
            }
        }
    }
}
#[test]
fn single_column_prefers_first_column_and_rejects_invalid_metrics() {
    let data = data::Data {
        version: 2,
        bar_backgrounds: vec![],
        contents: data::Contents::Sankey(
            vec![data::Node {
                id: 1,
                label: "Only".into(),
            }],
            vec![],
        ),
    };
    let options = options::Options::default();
    let labels = [Some(LabelMetrics {
        width: 40.,
        height: 18.,
    })];
    let plan = prepared(&data, &options, (300., 200.), &labels);
    assert_eq!(node(&plan, 0).left, 46.);
    assert!(matches!(
        plan.labels[0].kind,
        LabelKind::Flow {
            placement: Some(FlowLabelPlacement {
                align: FlowAlign::Right,
                above: false,
                ..
            }),
            ..
        }
    ));
    for labels in [
        vec![],
        vec![Some(LabelMetrics {
            width: f64::NAN,
            height: 18.,
        })],
        vec![Some(LabelMetrics {
            width: 10.,
            height: 0.,
        })],
    ] {
        assert!(matches!(
            prepare_with_labels(
                &data,
                Default::default(),
                &options,
                (300., 200.),
                Some(&labels),
                &AtomicBool::new(false)
            ),
            Err(Error::InvalidInput)
        ));
    }
    assert!(matches!(
        prepare_with_labels(
            &data,
            Default::default(),
            &options,
            (300., 200.),
            Some(&labels),
            &AtomicBool::new(true)
        ),
        Err(Error::Cancelled)
    ));
}
