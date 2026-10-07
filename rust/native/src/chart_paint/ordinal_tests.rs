use super::*;
use crate::{chart_details, chart_presentation};
use gpuio_protocol::{
    chart_data as data,
    chart_style::{Key, Ordinal},
    chart_view::Config,
};
const RED: i64 = 0xff0000ff;
const BLUE: i64 = 0x0000ffff;
const YELLOW: i64 = 0xffff00ff;
fn style(keys: Vec<Key>) -> Style {
    Style {
        ordinal: Some(Ordinal {
            domain: keys,
            range: vec![RED, BLUE],
            unknown: Some(YELLOW),
        }),
        ..Style::default()
    }
}
fn data(contents: data::Contents) -> Data {
    Data {
        version: 2,
        bar_backgrounds: vec![],
        contents,
    }
}
fn options() -> Options {
    let mut o = Options::default();
    o.axes.x = false;
    o.axes.y = false;
    o.axes.grid = false;
    o
}
fn plan(data: &Data, style: &Style) -> Prepared {
    prepare(
        data,
        Policy::default(),
        &options(),
        style,
        Layout::new(600., 300., 1.).unwrap(),
        &AtomicBool::new(false),
    )
    .unwrap()
}
fn series(id: i64) -> data::Series {
    data::Series {
        id,
        name: format!("Series {id}"),
        points: vec![data::Point {
            id: 1,
            x: 0.,
            y: Some(1.),
            label: String::new(),
        }],
    }
}

#[test]
fn cartesian_categorical_and_radar_colors_follow_series_identity_after_reorder() {
    let style = style(vec![Key::Series(2), Key::Series(9)]);
    for ids in [[9, 2, 7], [7, 9, 2]] {
        let values = data(data::Contents::Cartesian(
            ids.map(|id| data::Layer::Bar(series(id))).to_vec(),
        ));
        let categories = data(data::Contents::Categorical(
            vec![data::Category {
                id: 11,
                label: "One".into(),
            }],
            ids.map(|id| {
                data::CategoricalLayer::Bar(data::CategoricalSeries {
                    id,
                    name: format!("Series {id}"),
                    points: vec![data::CategoricalPoint {
                        id: 1,
                        category: 11,
                        value: Some(1.),
                        label: String::new(),
                    }],
                })
            })
            .to_vec(),
        ));
        let radar = data(data::Contents::Radar(
            (1..=3)
                .map(|id| data::RadarAxis {
                    id,
                    label: format!("Axis {id}"),
                    maximum: 10.,
                })
                .collect(),
            ids.map(|id| data::RadarSeries {
                id,
                name: format!("Series {id}"),
                values: vec![(1, 1.), (2, 2.), (3, 3.)],
            })
            .to_vec(),
        ));
        for source in [&values, &categories, &radar] {
            let prepared = plan(source, &style);
            let names = chart_presentation::legend(source);
            for (index, id) in ids.iter().copied().enumerate() {
                let expected = match id {
                    2 => RED,
                    9 => BLUE,
                    _ => YELLOW,
                } as u32;
                assert_eq!(prepared.series_color(index), expected);
                assert_eq!(names[index], format!("Series {id}"));
                if !matches!(source.contents, data::Contents::Radar(..)) {
                    assert_eq!(brush_color(prepared.quads[index].brush), expected);
                    let details = chart_details::describe(
                        source,
                        &Policy::default(),
                        &options(),
                        prepared.geometry(),
                        index,
                    )
                    .unwrap();
                    assert_eq!(details.title, format!("Series {id}"));
                }
            }
        }
    }
}

#[test]
fn pie_reorder_cycles_domain_colors_and_unknown_falls_back_only_when_requested() {
    let mut style = style(vec![Key::Slice(2), Key::Slice(9), Key::Slice(7)]);
    for ids in [[9, 2, 7, 10], [10, 7, 9, 2]] {
        let source = data(data::Contents::Pie(
            ids.map(|id| data::Slice {
                id,
                label: format!("Slice {id}"),
                value: 1.,
            })
            .to_vec(),
        ));
        for explicit_unknown in [true, false] {
            style.ordinal.as_mut().unwrap().unknown = explicit_unknown.then_some(YELLOW);
            let p = plan(&source, &style);
            for (index, id) in ids.iter().copied().enumerate() {
                let expected = match id {
                    2 | 7 => RED as u32,
                    9 => BLUE as u32,
                    _ if explicit_unknown => YELLOW as u32,
                    _ => style.color(index),
                };
                assert_eq!(p.series_color(index), expected);
                assert_eq!(brush_color(p.meshes[index].brush), expected);
            }
        }
    }
}

#[test]
fn sankey_ribbons_and_nodes_use_the_same_stable_source_node_color() {
    let style = style(vec![Key::Node(2), Key::Node(9)]);
    for ids in [[9, 2, 7], [7, 9, 2]] {
        let source = data(data::Contents::Sankey(
            ids.map(|id| data::Node {
                id,
                label: format!("Node {id}"),
            })
            .to_vec(),
            vec![data::Edge {
                id: 1,
                source: 9,
                target: 2,
                value: 1.,
            }],
        ));
        let p = plan(&source, &style);
        assert_eq!(brush_color(p.meshes[0].brush), alpha(BLUE as u32, 0.5));
        for (index, id) in ids.iter().copied().enumerate() {
            let expected = match id {
                2 => RED,
                9 => BLUE,
                _ => YELLOW,
            } as u32;
            assert_eq!(p.series_color(index), expected);
        }
        let visible = ids
            .into_iter()
            .filter(|id| *id != 7)
            .map(|id| if id == 2 { RED as u32 } else { BLUE as u32 })
            .collect::<Vec<_>>();
        assert_eq!(
            p.quads
                .iter()
                .map(|q| brush_color(q.brush))
                .collect::<Vec<_>>(),
            visible,
            "the isolated zero-flow node keeps its legend color but has no rectangle"
        );
    }
}

#[test]
fn candle_movement_and_key_namespaces_do_not_depend_on_raw_id_collisions() {
    let style = style(vec![Key::Series(9), Key::Rising, Key::Falling]);
    let source = data(data::Contents::Candlestick(vec![
        data::Candle {
            id: 9,
            x: 0.,
            open_: 1.,
            high: 3.,
            low: 0.,
            close: 2.,
            label: "Rise".into(),
        },
        data::Candle {
            id: 2,
            x: 1.,
            open_: 2.,
            high: 3.,
            low: 0.,
            close: 1.,
            label: "Fall".into(),
        },
    ]));
    let p = plan(&source, &style);
    assert_eq!(p.colors, vec![BLUE as u32, RED as u32]);
    assert!(
        p.quads
            .iter()
            .any(|q| q.border.is_some_and(|(_, c)| c == BLUE as u32))
    );
    assert!(p.quads.iter().any(|q| brush_color(q.brush) == RED as u32));
    let wrong_namespace = data(data::Contents::Pie(vec![data::Slice {
        id: 9,
        label: "Nine".into(),
        value: 1.,
    }]));
    assert_eq!(plan(&wrong_namespace, &style).colors, vec![YELLOW as u32]);
}

#[test]
fn expanded_style_storage_is_charged_and_cancelled_before_rendering() {
    let style = style((1..=1024).map(Key::Series).collect());
    let config = Config {
        version: -2,
        radar_labels: vec![],
        inspection_content: vec![],
        source: None,
        label: "Color storage".into(),
        options: options(),
        sampling: Policy::default(),
        style: style.clone(),
        legend: true,
        disabled: false,
    };
    assert!(config.retained_bytes() >= 1024 * size_of::<Key>());
    let source = data(data::Contents::Cartesian(vec![data::Layer::Bar(series(2))]));
    let p = plan(&source, &style);
    assert_eq!(p.colors.len(), 1);
    assert!(p.retained_bytes() >= size_of::<Prepared>() + size_of::<u32>());
    assert!(matches!(
        prepare(
            &source,
            Policy::default(),
            &options(),
            &style,
            Layout::new(600., 300., 1.).unwrap(),
            &AtomicBool::new(true)
        ),
        Err(Error::Cancelled)
    ));
}
