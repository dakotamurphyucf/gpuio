use super::*;
use crate::chart_geometry::{LabelKind, Point, Shape, Source};
use gpuio_protocol::chart_data::{Contents, Edge, Node};

fn data() -> Data {
    Data {
        version: 1,
        contents: Contents::Sankey(
            vec![
                Node {
                    id: 10,
                    label: "Input".into(),
                },
                Node {
                    id: 20,
                    label: "Output".into(),
                },
            ],
            vec![1., 0.0001, 0.]
                .into_iter()
                .enumerate()
                .map(|(i, value)| Edge {
                    id: i as i64 + 1,
                    source: 10,
                    target: 20,
                    value,
                })
                .collect(),
        ),
    }
}
fn prepared(options: &Options, width: f64, height: f64) -> Prepared {
    prepare(
        &data(),
        Policy::default(),
        options,
        &Style {
            palette: vec![0x12345680],
            ..Default::default()
        },
        Layout::new(width, height, 1.).unwrap(),
        &AtomicBool::new(false),
    )
    .unwrap()
}
#[test]
fn minimum_ribbon_geometry_preserves_zero_flows_and_hit_provenance() {
    let ordinary = prepared(&Options::default(), 400., 200.);
    let mut options = Options::default();
    options.sankey.min_link_width = 12.;
    let widened = prepared(&options, 400., 200.);
    let ribbon = |plan: &Prepared| {
        let mark = plan
            .geometry
            .marks
            .iter()
            .find(|m| m.source == Source::Edge(1))
            .unwrap();
        let Shape::Ribbon {
            start_top,
            start_bottom,
            end_top,
            end_bottom,
        } = mark.shape
        else {
            panic!("ribbon")
        };
        (start_top, start_bottom, end_top, end_bottom)
    };
    let (a, b, _, _) = ribbon(&ordinary);
    assert!(b.y - a.y < 0.1);
    let (a, b, c, d) = ribbon(&widened);
    assert!(
        b.y - a.y >= 6. && d.y - c.y >= 6.,
        "endpoint clipping retains at least half the configured span"
    );
    let point = Point {
        x: (a.x + c.x) / 2.,
        y: (a.y + b.y + c.y + d.y) / 4.,
    };
    assert_eq!(widened.hit_test(point).unwrap().source, Source::Edge(1));
    assert_eq!(
        widened
            .geometry
            .marks
            .iter()
            .filter(|m| matches!(m.source, Source::Edge(_)))
            .count(),
        2
    );
    assert!(
        widened
            .selection_index(gpuio_protocol::chart_selection::Selection::Edge(2))
            .is_some()
    );
    for (width, height) in [(1., 1.), (32., 8.), (400., 200.)] {
        let p = prepared(&options, width, height);
        for mark in &p.geometry.marks {
            if let Shape::Ribbon {
                start_top,
                start_bottom,
                end_top,
                end_bottom,
            } = mark.shape
            {
                for pt in [start_top, start_bottom, end_top, end_bottom] {
                    assert!(pt.y >= 0. && pt.y <= height);
                }
            }
        }
    }
}
#[test]
fn opacity_rounding_node_corners_and_edge_relative_labels_use_configuration() {
    let mut options = Options::default();
    options.sankey.node_corner_radius = 12.;
    options.sankey.link_opacity = 0.25;
    options.sankey.label_gap = 24.;
    let plan = prepared(&options, 400., 200.);
    assert!(
        plan.meshes
            .iter()
            .all(|m| brush_color(m.brush) == 0x12345620)
    );
    assert!(
        plan.quads.iter().all(|q| q.corners
            == Corners {
                top_left: 8.,
                top_right: 8.,
                bottom_left: 8.,
                bottom_right: 8.
            }),
        "radius clamps to half the 16px node width"
    );
    for (index, label) in plan.geometry.labels.iter().enumerate() {
        let mark = plan
            .geometry
            .marks
            .iter()
            .find(|m| m.source == Source::Node(index))
            .unwrap();
        let Shape::Node(bounds) = mark.shape else {
            panic!("node")
        };
        match label.kind {
            LabelKind::Flow {
                align_right: true, ..
            } => {
                assert_eq!(label.position.x, bounds.left - 24.)
            }
            LabelKind::Flow {
                align_right: false, ..
            } => {
                assert_eq!(label.position.x, bounds.right + 24.)
            }
            _ => panic!("flow label"),
        }
    }
    options.sankey.link_opacity = 0.;
    let hidden = prepared(&options, 400., 200.);
    assert!(
        hidden
            .meshes
            .iter()
            .all(|m| brush_color(m.brush) & 255 == 0)
    );
    assert_eq!(
        hidden.geometry.marks.len(),
        plan.geometry.marks.len(),
        "opacity does not change source identity or hit geometry"
    );
}

#[test]
fn link_brushes_follow_ids_and_opacity_without_changing_geometry_or_budget() {
    use gpuio_protocol::chart_style::{Key, Ordinal};
    let style = Style {
        ordinal: Some(Ordinal {
            domain: vec![Key::Node(20), Key::Node(10)],
            range: vec![0x0000ff80, 0xff0000c8],
            unknown: None,
        }),
        ..Default::default()
    };
    for reverse in [false, true] {
        let mut source = data();
        if let Contents::Sankey(nodes, _) = &mut source.contents
            && reverse
        {
            nodes.reverse();
        }
        let make = |mode| {
            let mut options = Options::default();
            options.sankey.link_opacity = 0.25;
            options.sankey.link_color = mode;
            prepare(
                &source,
                Policy::default(),
                &options,
                &style,
                Layout::new(400., 200., 1.).unwrap(),
                &AtomicBool::new(false),
            )
            .unwrap()
        };
        let ordinary = make(LinkColor::Source);
        for (mode, start, end) in [
            (LinkColor::Source, 0xff000032, None),
            (LinkColor::Target, 0x0000ff20, None),
            (LinkColor::Gradient, 0xff000032, Some(0x0000ff20)),
        ] {
            let plan = make(mode);
            assert_eq!(plan.geometry.marks, ordinary.geometry.marks);
            assert_eq!(plan.vertices(), ordinary.vertices());
            assert_eq!(plan.retained_bytes(), ordinary.retained_bytes());
            assert_eq!(plan.meshes.len(), 2, "zero-valued third edge stays absent");
            for draw in &plan.meshes {
                assert_eq!(
                    draw.brush,
                    end.map_or(solid(start), |end| gradient(90., start, end))
                );
            }
            assert!(
                plan.geometry
                    .marks
                    .iter()
                    .any(|m| m.source == Source::Edge(1))
            );
        }
    }
}
