use super::*;

fn tick(position: TickPosition, text: &str) -> Tick {
    Tick {
        position,
        text: text.into(),
        color: None,
        font_size: None,
        align: LabelAlign::Auto,
    }
}
fn data() -> data::Data {
    data::Data {
        version: 2,
        bar_backgrounds: vec![],
        contents: data::Contents::Cartesian(vec![data::Layer::Line(data::Series {
            id: 1,
            name: "original".into(),
            points: vec![
                data::Point {
                    id: 11,
                    x: 0.,
                    y: Some(-2.),
                    label: "raw first".into(),
                },
                data::Point {
                    id: 12,
                    x: 10.,
                    y: Some(6.),
                    label: "raw last".into(),
                },
            ],
        })]),
    }
}
fn prepare(data: &data::Data, options: &options::Options, style: &Style) -> Plan {
    super::super::prepare_with_axes(
        data,
        Policy::default(),
        options,
        (200., 100.),
        None,
        Some(Styles::of(style)),
        &AtomicBool::new(false),
    )
    .unwrap()
}
#[test]
fn explicit_numeric_ticks_transpose_and_reverse_without_mutating_source_geometry() {
    for orientation in [
        options::Orientation::Vertical,
        options::Orientation::Horizontal,
        options::Orientation::VerticalReversed,
        options::Orientation::HorizontalReversed,
    ] {
        let mut options = options::Options::default();
        options.cartesian.orientation = orientation;
        let source = data();
        let plain = prepare(&source, &options, &Style::default());
        let mut style = Style::default();
        style.x_axis.position = Some(0.25);
        style.x_axis.ticks = Some(vec![
            tick(TickPosition::Value(5.), "mid x"),
            tick(TickPosition::Value(11.), "outside"),
            tick(TickPosition::Category(1), "wrong kind"),
        ]);
        style.y_axis.position = Some(0.75);
        style.y_axis.label_color = Some(8);
        style.y_axis.font_size = 16.;
        let mut custom = tick(TickPosition::Value(0.), "zero y");
        custom.color = Some(7);
        custom.font_size = Some(20.);
        custom.align = LabelAlign::Left;
        style.y_axis.ticks = Some(vec![custom, tick(TickPosition::Fraction(0.1), "physical")]);
        let plan = prepare(&source, &options, &style);
        assert_eq!(plan.x_domain, plain.x_domain);
        assert_eq!(plan.y_domain, plain.y_domain);
        assert_eq!(plan.marks, plain.marks);
        assert_eq!(plan.paths, plain.paths);
        let captions: Vec<_> = plan
            .labels
            .iter()
            .filter(|l| matches!(l.kind, LabelKind::Axis(_)))
            .collect();
        assert_eq!(
            captions.iter().map(|l| l.text.as_str()).collect::<Vec<_>>(),
            ["mid x", "zero y", "physical"]
        );
        let horizontal = orientation.is_horizontal();
        let reversed = matches!(
            orientation,
            options::Orientation::VerticalReversed | options::Orientation::HorizontalReversed
        );
        assert_eq!(
            captions[0].position,
            if horizontal {
                Point::new(50., 50.)
            } else {
                Point::new(100., 25.)
            }
        );
        let value = if reversed { 0.75 } else { 0.25 };
        assert_eq!(
            captions[1].position,
            if horizontal {
                Point::new(value * 200., 75.)
            } else {
                Point::new(150., (1. - value) * 100.)
            }
        );
        assert_eq!(
            captions[2].position,
            if horizontal {
                Point::new(20., 75.)
            } else {
                Point::new(150., 10.)
            }
        );
        let LabelKind::Axis(first) = captions[1].kind else {
            unreachable!()
        };
        assert_eq!(
            (first.color, first.font_size, first.align),
            (Some(7), 20., LabelAlign::Left)
        );
        let LabelKind::Axis(second) = captions[2].kind else {
            unreachable!()
        };
        assert_eq!((second.color, second.font_size), (Some(8), 16.));
        assert_eq!(plan.grid.len(), 3);
    }
}
#[test]
fn category_targets_follow_ids_across_reordering_and_empty_caption_keeps_grid() {
    let category = |id| data::Category {
        id,
        label: format!("original {id}"),
    };
    let mut source = data::Data {
        version: 2,
        bar_backgrounds: vec![],
        contents: data::Contents::Categorical(vec![category(7), category(9)], vec![]),
    };
    let mut style = Style::default();
    style.x_axis.ticks = Some(vec![
        tick(TickPosition::Category(9), ""),
        tick(TickPosition::Category(7), "custom"),
        tick(TickPosition::Category(42), "unknown"),
        tick(TickPosition::Value(0.), "wrong kind"),
    ]);
    style.y_axis.ticks = Some(vec![]);
    let options = options::Options::default();
    let first = prepare(&source, &options, &style);
    assert_eq!(first.labels.len(), 1);
    assert_eq!(first.labels[0].text, "custom");
    assert_eq!(first.grid.len(), 2);
    assert_eq!(first.grid[1].0.x, first.labels[0].position.x);
    let data::Contents::Categorical(categories, _) = &mut source.contents else {
        unreachable!()
    };
    categories.reverse();
    let next = prepare(&source, &options, &style);
    assert_eq!(next.labels[0].position.x, first.grid[0].0.x);
    style.grid.x = Some(vec![]);
    style.grid.y = Some(vec![TickPosition::Fraction(0.4)]);
    let next = prepare(&source, &options, &style);
    assert_eq!(next.grid, [(Point::new(0., 40.), Point::new(200., 40.))]);
    let mut hidden = options.clone();
    hidden.axes.x = false;
    hidden.axes.y = false;
    let next = prepare(&source, &hidden, &style);
    assert!(next.labels.is_empty());
    assert_eq!(next.grid.len(), 1);
    hidden.axes.grid = false;
    assert!(prepare(&source, &hidden, &style).grid.is_empty());
}
#[test]
fn per_axis_counts_and_constant_domain_reject_absent_values() {
    let mut style = Style::default();
    style.x_axis.tick_count = Some(3);
    style.y_axis.tick_count = Some(7);
    let source = data();
    let options = options::Options::default();
    let result = prepare(&source, &options, &style);
    assert_eq!(
        result
            .labels
            .iter()
            .filter(|l| matches!(l.kind, LabelKind::Axis(_)))
            .count(),
        10
    );
    let mut source = source;
    let data::Contents::Cartesian(layers) = &mut source.contents else {
        unreachable!()
    };
    let data::Layer::Line(series) = &mut layers[0] else {
        unreachable!()
    };
    series.points.truncate(1);
    for p in &mut series.points {
        p.x = 5.;
        p.y = Some(3.);
    }
    style.x_axis.ticks = Some(vec![
        tick(TickPosition::Value(5.), "actual"),
        tick(TickPosition::Value(4.), "absent"),
    ]);
    style.y_axis.labels = false;
    let result = prepare(&source, &options, &style);
    assert_eq!(
        result
            .labels
            .iter()
            .filter(|l| matches!(l.kind, LabelKind::Axis(_)))
            .map(|l| l.text.as_str())
            .collect::<Vec<_>>(),
        ["actual"]
    );
}
