use super::*;
use crate::{chart_hit::Index, chart_selection};
use gpuio_protocol::chart_selection::Selection;

fn dataset(ids: &[i64]) -> data::Data {
    data::Data {
        version: 2,
        bar_backgrounds: vec![],
        contents: data::Contents::Pie(
            ids.iter()
                .map(|&id| data::Slice {
                    id,
                    label: format!("Slice {id}"),
                    value: if id == 9 { 0. } else { 1. },
                })
                .collect(),
        ),
    }
}
fn plan(data: &data::Data, options: &options::Options) -> Plan {
    prepare(
        data,
        Policy::default(),
        options,
        200.,
        160.,
        &AtomicBool::new(false),
    )
    .unwrap()
}
fn ring(mark: &Mark) -> (f64, f64, f64, f64) {
    let Shape::Wedge {
        inner,
        outer,
        start,
        end,
        ..
    } = mark.shape
    else {
        panic!("expected wedge")
    };
    (inner, outer, start, end)
}

#[test]
fn fixed_and_per_slice_radii_preserve_angles_raw_values_and_identity() {
    let mut o = options::Options::default();
    o.pie.inner_radius = 0.5;
    let source = dataset(&[7, 3, 9]);
    let original = source.clone();
    let fit = plan(&source, &o);
    assert_eq!((ring(&fit.marks[0]).0, ring(&fit.marks[0]).1), (40., 80.));
    o.pie.radius = options::PieRadius::Pixels(60.);
    let fixed = plan(&source, &o);
    assert_eq!(
        (ring(&fixed.marks[0]).0, ring(&fixed.marks[0]).1),
        (30., 60.)
    );
    o.pie.slice_radii = vec![
        options::SliceRadii {
            slice: 7,
            inner: 10.,
            outer: 40.,
        },
        options::SliceRadii {
            slice: 9,
            inner: 0.,
            outer: 100.,
        },
        options::SliceRadii {
            slice: 77,
            inner: 0.,
            outer: 20.,
        },
    ];
    let custom = plan(&source, &o);
    assert_eq!(custom.marks.len(), 2); // The zero-valued source stays absent.
    assert_eq!(
        (ring(&custom.marks[0]).0, ring(&custom.marks[0]).1),
        (10., 40.)
    );
    assert_eq!(
        (ring(&custom.marks[1]).0, ring(&custom.marks[1]).1),
        (30., 60.)
    );
    for (a, b) in fixed.marks.iter().zip(&custom.marks) {
        assert_eq!((ring(a).2, ring(a).3), (ring(b).2, ring(b).3));
        assert_eq!(a.source, b.source);
    }
    assert_eq!(custom.labels[0].position, Point::new(129.5, 80.));
    assert_eq!(source, original);
    let mut reordered = dataset(&[3, 9, 7]);
    let data::Contents::Pie(slices) = &mut reordered.contents else {
        panic!()
    };
    slices[2].label = "Renamed".into();
    let moved = plan(&reordered, &o);
    assert_eq!(
        (ring(&moved.marks[1]).0, ring(&moved.marks[1]).1),
        (10., 40.)
    );
    assert_eq!(moved.marks[1].source, Source::Slice(2));
    assert_eq!(moved.labels[1].text, "Renamed");
    assert_eq!(
        chart_selection::resolve(&reordered, &Policy::default(), moved.marks[1].source),
        Some(Selection::Slice(7))
    );
}

#[test]
fn equal_radii_keep_angular_weight_and_unknown_ids_activate_on_return() {
    let mut o = options::Options::default();
    o.pie.slice_radii = vec![options::SliceRadii {
        slice: 7,
        inner: 0.,
        outer: 0.,
    }];
    let source = dataset(&[7, 3]);
    let p = plan(&source, &o);
    assert_eq!(p.marks.len(), 1);
    assert_eq!(p.labels.len(), 1);
    assert_eq!(p.marks[0].source, Source::Slice(1));
    assert!((ring(&p.marks[0]).2 - PI / 2.).abs() < 1e-12);
    assert!((ring(&p.marks[0]).3 - 3. * PI / 2.).abs() < 1e-12);
    o.pie.slice_radii[0].inner = 30.;
    o.pie.slice_radii[0].outer = 30.;
    assert_eq!(plan(&source, &o).marks.len(), 1);
    let absent = plan(&dataset(&[3]), &o);
    assert_eq!(
        (ring(&absent.marks[0]).0, ring(&absent.marks[0]).1),
        (0., 80.)
    );
    assert_eq!(plan(&dataset(&[7, 3]), &o).marks.len(), 1);
    o.pie.slice_radii[0].outer = 50.;
    assert_eq!(plan(&source, &o).marks.len(), 2);
}

#[test]
fn hit_testing_uses_override_holes_and_outer_boundaries() {
    let mut o = options::Options::default();
    o.pie.radius = options::PieRadius::Pixels(60.);
    o.pie.slice_radii = vec![options::SliceRadii {
        slice: 7,
        inner: 20.,
        outer: 40.,
    }];
    let source = dataset(&[7, 3]);
    let p = plan(&source, &o);
    let index = Index::prepare(
        &p,
        options::Orientation::Vertical,
        3.,
        &AtomicBool::new(false),
    )
    .unwrap();
    for x in [110., 150.] {
        assert!(index.query(&p, Point::new(x, 80.), false).is_none());
    }
    let hit = index.query(&p, Point::new(130., 80.), false).unwrap();
    assert_eq!(
        chart_selection::resolve(&source, &Policy::default(), p.marks[hit].source),
        Some(Selection::Slice(7))
    );
    let hit = index.query(&p, Point::new(50., 80.), false).unwrap();
    assert_eq!(
        chart_selection::resolve(&source, &Policy::default(), p.marks[hit].source),
        Some(Selection::Slice(3))
    );
}
