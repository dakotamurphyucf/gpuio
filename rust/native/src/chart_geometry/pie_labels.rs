//! Measured outside captions with deterministic bounded spreading.
use super::{Label, LabelKind, PieLabelPlacement, Point};

pub(super) struct Candidate {
    pub slice_index: usize,
    pub angle: f64,
    pub outer: f64,
    pub width: f64,
    pub text: String,
}
pub(super) fn layout(width: f64, height: f64, gap: f64, candidates: Vec<Candidate>) -> Vec<Label> {
    let capacity = (height / 18.).floor() as usize;
    if capacity == 0 {
        return vec![];
    }
    let center = Point::new(width / 2., height / 2.);
    let mut sides = [vec![], vec![]];
    for c in candidates {
        if c.width <= 0. {
            continue;
        }
        let side = usize::from(c.angle.cos() >= 0.);
        let target = Point::polar(center, c.outer + gap, c.angle);
        sides[side].push((c, target));
    }
    let mut result = vec![];
    for (side, mut candidates) in sides.into_iter().enumerate() {
        candidates.sort_by(|(a, at), (b, bt)| {
            at.y.total_cmp(&bt.y)
                .then(a.slice_index.cmp(&b.slice_index))
        });
        let count = candidates.len();
        // Choose distributed source positions; never let crowded labels overlap.
        let mut candidates: Vec<_> = candidates
            .into_iter()
            .enumerate()
            .filter(|(i, _)| count <= capacity || (0..capacity).any(|k| *i == k * count / capacity))
            .map(|(_, value)| value)
            .collect();
        let mut previous = -9.;
        for (_, p) in &mut candidates {
            p.y = p.y.clamp(9., height - 9.).max(previous + 18.);
            previous = p.y;
        }
        let mut next = height + 9.;
        for (_, p) in candidates.iter_mut().rev() {
            p.y = p.y.min(next - 18.);
            next = p.y;
        }
        let column_width = candidates
            .iter()
            .map(|(c, _)| c.width)
            .fold(0., f64::max)
            .min(width * 0.25);
        let outer = candidates.iter().map(|(c, _)| c.outer).fold(0., f64::max);
        let align_right = side == 0;
        let x = if align_right {
            (center.x - outer - gap - 4.)
                .max(column_width)
                .min(center.x)
        } else {
            (center.x + outer + gap + 4.)
                .min(width - column_width)
                .max(center.x)
        };
        for (c, target) in candidates {
            let text_width = c.width.min(column_width);
            let end = Point::new(x + if align_right { 4. } else { -4. }, target.y);
            result.push(Label {
                position: Point::new(x, target.y),
                text: c.text,
                kind: LabelKind::Pie {
                    slice_index: c.slice_index,
                    placement: Some(PieLabelPlacement {
                        align_right,
                        width: text_width,
                        edge: Point::polar(center, c.outer, c.angle),
                        bend: Point::new(target.x, target.y),
                        end,
                    }),
                },
            });
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chart_geometry::{self as geometry, LabelMetrics, Shape, Source};
    use gpuio_protocol::{chart_data as data, chart_options as options, chart_sampling::Policy};
    use std::sync::atomic::AtomicBool;
    fn source(ids: &[i64]) -> data::Data {
        data::Data {
            version: 1,
            contents: data::Contents::Pie(
                ids.iter()
                    .map(|&id| data::Slice {
                        id,
                        label: format!("Slice {id}"),
                        value: 1.,
                    })
                    .collect(),
            ),
        }
    }
    fn prepare(
        data: &data::Data,
        o: &options::Options,
        size: (f64, f64),
        metrics: &[Option<LabelMetrics>],
    ) -> geometry::Plan {
        geometry::prepare_with_labels(
            data,
            Policy::default(),
            o,
            size,
            Some(metrics),
            &AtomicBool::new(false),
        )
        .unwrap()
    }
    fn options() -> options::Options {
        let mut o = options::Options::default();
        o.pie.label_placement = options::LabelPlacement::Outside;
        o
    }
    #[test]
    fn leader_edges_follow_actual_slice_radii_and_label_identity() {
        let data = source(&[7, 3]);
        let mut o = options();
        o.pie.radius = options::PieRadius::Pixels(60.);
        o.pie.slice_radii = vec![options::SliceRadii {
            slice: 7,
            inner: 20.,
            outer: 40.,
        }];
        let metrics = vec![
            Some(LabelMetrics {
                width: 80.,
                height: 18.
            });
            2
        ];
        let plan = prepare(&data, &o, (400., 240.), &metrics);
        assert_eq!(plan.marks.len(), 2);
        for label in &plan.labels {
            let LabelKind::Pie {
                slice_index,
                placement: Some(p),
            } = label.kind
            else {
                panic!()
            };
            assert_eq!(p.width, 80.);
            assert_eq!(
                label.text,
                format!("Slice {}", if slice_index == 0 { 7 } else { 3 })
            );
            let expected_x = if slice_index == 0 { 240. } else { 140. };
            assert!((p.edge.x - expected_x).abs() < 1e-9);
            assert!((p.edge.y - 120.).abs() < 1e-9);
            assert_eq!(p.end.y, label.position.y);
            assert_eq!(p.align_right, slice_index == 1);
        }
        let moved = prepare(&source(&[3, 7]), &o, (400., 240.), &metrics);
        assert_eq!(moved.marks[1].source, Source::Slice(1));
        let Shape::Wedge { outer, .. } = moved.marks[1].shape else {
            panic!()
        };
        assert_eq!(outer, 40.);
        o.pie.slice_radii[0].inner = 40.;
        let hidden = prepare(&data, &o, (400., 240.), &metrics);
        assert_eq!(hidden.marks.len(), 1);
        assert_eq!(hidden.labels.len(), 1);
    }
    #[test]
    fn crowded_and_tiny_views_have_bounded_nonoverlapping_captions() {
        let data = source(&(1..=256).collect::<Vec<_>>());
        let o = options();
        let metrics = vec![
            Some(LabelMetrics {
                width: 2000.,
                height: 18.
            });
            256
        ];
        for (width, height) in [(400., 240.), (60., 36.), (1., 1.)] {
            let plan = prepare(&data, &o, (width, height), &metrics);
            assert_eq!(plan.marks.len(), 256);
            for side in [false, true] {
                let mut ys = vec![];
                for l in &plan.labels {
                    let LabelKind::Pie {
                        placement: Some(p), ..
                    } = l.kind
                    else {
                        panic!()
                    };
                    let left = l.position.x - if p.align_right { p.width } else { 0. };
                    assert!(left >= 0. && left + p.width <= width);
                    assert!(l.position.y >= 9. && l.position.y + 9. <= height);
                    if p.align_right == side {
                        ys.push(l.position.y);
                    }
                }
                assert!(ys.windows(2).all(|pair| pair[1] - pair[0] >= 18. - 1e-9));
                assert!(ys.len() <= (height / 18.).floor() as usize);
            }
            if height < 18. {
                assert!(plan.labels.is_empty());
            }
        }
    }
    #[test]
    fn measurements_are_required_and_hidden_or_tiny_slices_do_not_get_leaders() {
        let mut data = source(&[7, 3, 9]);
        let data::Contents::Pie(s) = &mut data.contents else {
            panic!()
        };
        s[0].value = 0.;
        s[1].value = 1e-8;
        let o = options();
        assert!(
            geometry::prepare(
                &data,
                Policy::default(),
                &o,
                400.,
                240.,
                &AtomicBool::new(false)
            )
            .is_err()
        );
        let metrics = vec![
            Some(LabelMetrics {
                width: 80.,
                height: 18.
            });
            3
        ];
        let p = prepare(&data, &o, (400., 240.), &metrics);
        assert_eq!(p.marks.len(), 2);
        assert_eq!(p.labels.len(), 1);
        assert!(matches!(
            p.labels[0].kind,
            LabelKind::Pie { slice_index: 2, .. }
        ));
        let hidden = prepare(&data, &o, (400., 240.), &[None, None, None]);
        assert!(hidden.labels.is_empty());
        assert_eq!(hidden.marks.len(), 2);
        for bad in [f64::NAN, f64::INFINITY, -1.] {
            let metrics = [Some(LabelMetrics {
                width: bad,
                height: 18.,
            }); 3];
            assert!(
                geometry::prepare_with_labels(
                    &data,
                    Policy::default(),
                    &o,
                    (400., 240.),
                    Some(&metrics),
                    &AtomicBool::new(false)
                )
                .is_err()
            );
        }
    }
}
