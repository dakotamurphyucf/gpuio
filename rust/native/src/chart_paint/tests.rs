use super::*;
use gpuio_protocol::chart_data as data;
fn render(contents: data::Contents) -> Result<Prepared, Error> {
    prepare(
        &Data {
            version: 1,
            contents,
        },
        Policy::default(),
        &Options::default(),
        &Style::default(),
        Layout::new(800., 400., 2.).unwrap(),
        &AtomicBool::new(false),
    )
}
#[test]
fn sampled_streams_prepare_bounded_reusable_meshes_and_preserve_gaps() {
    for gaps in [false, true] {
        let points = (0..100_000)
            .map(|i| data::Point {
                id: i + 1,
                x: i as f64,
                y: if gaps && i % 2 == 1 {
                    None
                } else {
                    Some((i % 17) as f64)
                },
                label: String::new(),
            })
            .collect();
        let p = render(data::Contents::Cartesian(vec![data::Layer::Area(
            data::Series {
                id: 1,
                name: "stream".into(),
                points,
            },
        )]))
        .unwrap();
        assert!(p.vertices() <= MAX_VERTICES);
        assert!(p.retained_bytes() < MAX_BYTES);
        if gaps {
            assert_eq!(p.quad_count(), 50_000);
        } else {
            assert!(p.mesh_count() >= 2);
            assert_eq!(p.quad_count(), 0);
        }
    }
}
#[test]
fn wedges_ribbons_and_candle_noncolor_geometry_prepare() {
    let p = render(data::Contents::Pie(vec![data::Slice {
        id: 1,
        label: "whole".into(),
        value: 1.,
    }]))
    .unwrap();
    assert!(p.vertices() > 0);
    let p = render(data::Contents::Sankey(
        vec![
            data::Node {
                id: 1,
                label: "a".into(),
            },
            data::Node {
                id: 2,
                label: "b".into(),
            },
        ],
        vec![data::Edge {
            id: 1,
            source: 1,
            target: 2,
            value: 1e100,
        }],
    ))
    .unwrap();
    assert_eq!(p.mesh_count(), 1);
    assert_eq!(p.quad_count(), 2);
    let p = render(data::Contents::Candlestick(vec![data::Candle {
        id: 1,
        x: 0.,
        label: String::new(),
        open_: 0.,
        high: 3.,
        low: -1.,
        close: 2.,
    }]))
    .unwrap();
    assert_eq!(p.quad_count(), 3);
    assert_eq!(p.quads[2].color, 0);
    assert!(p.quads[2].border.is_some());
    assert!(p.quads[0].rect.bottom <= p.quads[2].rect.top);
    assert!(p.quads[1].rect.top >= p.quads[2].rect.bottom);
}
#[test]
fn invalid_style_cancel_and_unbounded_exact_paths_fail_atomically() {
    let data = Data {
        version: 1,
        contents: data::Contents::Pie(vec![]),
    };
    let style = Style {
        palette: vec![],
        ..Style::default()
    };
    let layout = Layout::new(800., 400., 1.).unwrap();
    assert!(matches!(
        prepare(
            &data,
            Policy::default(),
            &Options::default(),
            &style,
            layout,
            &AtomicBool::new(false)
        ),
        Err(Error::InvalidInput)
    ));
    assert!(matches!(
        prepare(
            &data,
            Policy::default(),
            &Options::default(),
            &Style::default(),
            layout,
            &AtomicBool::new(true)
        ),
        Err(Error::Cancelled)
    ));
    assert!(Layout::new(f64::NAN, 1., 1.).is_err());
    assert!(Layout::new(1., 1., 9.).is_err());
    let points = (0..100_000)
        .map(|i| data::Point {
            id: i + 1,
            x: i as f64,
            y: Some((i % 2) as f64),
            label: String::new(),
        })
        .collect();
    let data = Data {
        version: 1,
        contents: data::Contents::Cartesian(vec![data::Layer::Line(data::Series {
            id: 1,
            name: "exact".into(),
            points,
        })]),
    };
    let policy = Policy {
        line: gpuio_protocol::chart_sampling::Line::Exact,
        ..Policy::default()
    };
    let mut over_budget = Options::default();
    over_budget.cartesian.curve = gpuio_protocol::chart_options::Curve::Natural;
    assert!(matches!(
        prepare(
            &data,
            policy,
            &over_budget,
            &Style::default(),
            Layout::new(32768., 32768., 8.).unwrap(),
            &AtomicBool::new(false)
        ),
        Err(Error::RenderLimit)
    ));
}

#[test]
fn window_frame_admission_is_shared_atomic_and_bounded() {
    let mut budget = FrameBudget::default();
    budget.reserve(MAX_VERTICES - 10, MAX_QUADS - 5).unwrap();
    assert_eq!(budget.reserve(11, 1), Err(Error::RenderLimit));
    assert_eq!(budget.reserve(1, 6), Err(Error::RenderLimit));
    assert_eq!(
        (budget.used_vertices(), budget.used_quads()),
        (MAX_VERTICES - 10, MAX_QUADS - 5)
    );
    budget.reserve(10, 5).unwrap();
    assert_eq!(budget.reserve(1, 0), Err(Error::RenderLimit));
    assert_eq!(budget.reserve(0, 1), Err(Error::RenderLimit));
    assert_eq!(budget.reserve(0, 0), Ok(()));
}

#[test]
fn large_exact_line_and_all_sampled_curve_modes_have_real_meshes() {
    let contents = |exact: bool| {
        data::Contents::Cartesian(vec![data::Layer::Line(data::Series {
            id: 1,
            name: "large".into(),
            points: (0..100_000)
                .map(|i| data::Point {
                    id: i + 1,
                    x: i as f64,
                    y: Some(if exact { i as f64 } else { (i % 17) as f64 }),
                    label: String::new(),
                })
                .collect(),
        })])
    };
    let cancel = AtomicBool::new(false);
    let layout = Layout::new(800., 400., 2.).unwrap();
    let p = prepare(
        &Data {
            version: 1,
            contents: contents(true),
        },
        Policy {
            line: gpuio_protocol::chart_sampling::Line::Exact,
            ..Policy::default()
        },
        &Options::default(),
        &Style::default(),
        layout,
        &cancel,
    )
    .unwrap();
    assert_eq!(p.geometry().marks.len(), 100_000);
    assert!(p.vertices() > 0 && p.vertices() <= MAX_VERTICES);
    drop(p);
    for curve in [
        gpuio_protocol::chart_options::Curve::Linear,
        gpuio_protocol::chart_options::Curve::Natural,
        gpuio_protocol::chart_options::Curve::StepAfter,
    ] {
        let mut options = Options::default();
        options.cartesian.curve = curve;
        let p = prepare(
            &Data {
                version: 1,
                contents: contents(false),
            },
            Policy::default(),
            &options,
            &Style::default(),
            layout,
            &cancel,
        )
        .unwrap();
        assert!(p.vertices() > 0 && p.vertices() <= MAX_VERTICES);
        assert!(p.retained_bytes() < MAX_BYTES);
    }
}

#[test]
fn chart_allowance_does_not_relax_the_canvas_wire_or_mesh_contract() {
    let mut commands = vec![PathCommand::Move(MeshPoint { x: 0., y: 0. })];
    commands.extend((1..5000).map(|i| PathCommand::Line(MeshPoint { x: i as f64, y: 1. })));
    let path = MeshPath(commands);
    let cancel = AtomicBool::new(false);
    assert!(!path.is_valid());
    assert!(path.is_valid_up_to(5000));
    assert!(matches!(
        mesh::prepare(
            mesh::Geometry::Path(&path),
            mesh::Style::Stroke(1.),
            0.25,
            &cancel
        ),
        Err(mesh::Error::InvalidGeometry)
    ));
    assert!(mesh::prepare_chart(&path, mesh::Style::Stroke(1.), 0.25, &cancel).is_ok());
}
