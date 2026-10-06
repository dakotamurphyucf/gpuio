//! Worker-prepared broad phase and native analytic hit tests. No UI callbacks.
use crate::chart_geometry::{Mark, Plan, Point, Rect, Shape, Source};
use gpuio_protocol::chart_options::Orientation;
use std::{
    collections::BTreeMap,
    mem::size_of,
    sync::atomic::{AtomicBool, Ordering},
};

const LEAF_SIZE: usize = 8;
const NONE: usize = usize::MAX;
const MAX_MARKS: usize = 100_000;
const MAX_BYTES: usize = 16 * 1024 * 1024;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    Cancelled,
    InvalidGeometry,
    LimitExceeded,
}
#[derive(Clone, Copy)]
struct Entry {
    bounds: Rect,
    mark: usize,
}
struct Node {
    bounds: Rect,
    start: usize,
    end: usize,
    left: usize,
    right: usize,
}
#[derive(Default)]
pub struct Index {
    entries: Vec<Entry>,
    nodes: Vec<Node>,
    columns: Vec<Vec<usize>>,
    horizontal: bool,
    point_radius: f64,
}
fn contains(r: Rect, p: Point) -> bool {
    p.x >= r.left && p.x <= r.right && p.y >= r.top && p.y <= r.bottom
}
fn expand(r: Rect, amount: f64) -> Rect {
    Rect {
        left: r.left - amount,
        top: r.top - amount,
        right: r.right + amount,
        bottom: r.bottom + amount,
    }
}
fn union(a: Rect, b: Rect) -> Rect {
    Rect {
        left: a.left.min(b.left),
        top: a.top.min(b.top),
        right: a.right.max(b.right),
        bottom: a.bottom.max(b.bottom),
    }
}
fn bounds(shape: Shape, point_radius: f64) -> Rect {
    match shape {
        Shape::Dot { center, .. } => expand(
            Rect {
                left: center.x,
                right: center.x,
                top: center.y,
                bottom: center.y,
            },
            point_radius,
        ),
        Shape::Bar(r) | Shape::Node(r) => expand(r, 2.),
        Shape::Candle {
            left,
            right,
            high,
            low,
            ..
        } => expand(
            Rect {
                left,
                right,
                top: high.min(low),
                bottom: high.max(low),
            },
            3.,
        ),
        Shape::Wedge { center, outer, .. } => expand(
            Rect {
                left: center.x,
                right: center.x,
                top: center.y,
                bottom: center.y,
            },
            outer,
        ),
        Shape::Ribbon {
            start_top: a,
            start_bottom: b,
            end_top: c,
            end_bottom: d,
        } => Rect {
            left: a.x.min(b.x).min(c.x).min(d.x),
            right: a.x.max(b.x).max(c.x).max(d.x),
            top: a.y.min(b.y).min(c.y).min(d.y) - 2.,
            bottom: a.y.max(b.y).max(c.y).max(d.y) + 2.,
        },
    }
}
fn distance_squared(a: Point, b: Point) -> f64 {
    (a.x - b.x).powi(2) + (a.y - b.y).powi(2)
}
fn hit(shape: Shape, p: Point, point_radius: f64) -> bool {
    match shape {
        Shape::Dot { center, .. } => distance_squared(center, p) <= point_radius.powi(2),
        Shape::Bar(r) | Shape::Node(r) => contains(expand(r, 2.), p),
        Shape::Candle {
            center,
            left,
            right,
            open,
            high,
            low,
            close,
        } => {
            contains(
                expand(
                    Rect {
                        left,
                        right,
                        top: open.min(close),
                        bottom: open.max(close),
                    },
                    2.,
                ),
                p,
            ) || ((p.x - center).abs() <= 3.
                && p.y >= high.min(low) - 2.
                && p.y <= high.max(low) + 2.)
        }
        Shape::Wedge {
            center,
            inner,
            outer,
            start,
            end,
        } => {
            let radius = distance_squared(center, p).sqrt();
            let angle = (p.y - center.y).atan2(p.x - center.x);
            let offset = (angle - start).rem_euclid(std::f64::consts::TAU);
            radius >= inner && radius <= outer && offset <= end - start
        }
        Shape::Ribbon {
            start_top: a,
            start_bottom: b,
            end_top: c,
            end_bottom: d,
        } => {
            if p.x < a.x || p.x > c.x || c.x <= a.x {
                return false;
            }
            // Both ribbon boundaries use control x=(source.x+target.x)/2.
            let x = (p.x - a.x) / (c.x - a.x);
            let (mut lo, mut hi) = (0., 1.);
            for _ in 0..24 {
                let t = (lo + hi) * 0.5;
                let curve = 1.5 * t - 1.5 * t * t + t * t * t;
                if curve < x {
                    lo = t;
                } else {
                    hi = t;
                }
            }
            let t = (lo + hi) * 0.5;
            let y = 3. * t * t - 2. * t * t * t;
            let top = a.y + (c.y - a.y) * y;
            let bottom = b.y + (d.y - b.y) * y;
            p.y >= top.min(bottom) - 2. && p.y <= top.max(bottom) + 2.
        }
    }
}
fn coordinate(mark: &Mark, horizontal: bool) -> f64 {
    let Shape::Dot { center, .. } = mark.shape else {
        unreachable!("only point columns");
    };
    if horizontal { center.y } else { center.x }
}
fn build(
    entries: &mut [Entry],
    base: usize,
    nodes: &mut Vec<Node>,
    cancel: &AtomicBool,
) -> Result<usize, Error> {
    if cancel.load(Ordering::Relaxed) {
        return Err(Error::Cancelled);
    }
    let bounds = entries
        .iter()
        .skip(1)
        .fold(entries[0].bounds, |b, e| union(b, e.bounds));
    let index = nodes.len();
    nodes.push(Node {
        bounds,
        start: base,
        end: base + entries.len(),
        left: NONE,
        right: NONE,
    });
    if entries.len() > LEAF_SIZE {
        let horizontal = bounds.right - bounds.left >= bounds.bottom - bounds.top;
        let middle = entries.len() / 2;
        entries.select_nth_unstable_by(middle, |a, b| {
            let center = |e: &Entry| {
                if horizontal {
                    e.bounds.left + e.bounds.right
                } else {
                    e.bounds.top + e.bounds.bottom
                }
            };
            center(a).total_cmp(&center(b)).then(a.mark.cmp(&b.mark))
        });
        let (left, right) = entries.split_at_mut(middle);
        let left = build(left, base, nodes, cancel)?;
        let right = build(right, base + middle, nodes, cancel)?;
        nodes[index].left = left;
        nodes[index].right = right;
    }
    Ok(index)
}
impl Index {
    /// The validated geometry contains at most 100,000 marks. Cancellation can
    /// interrupt construction between bounded partitions; none of this runs in paint.
    pub fn prepare(
        plan: &Plan,
        orientation: Orientation,
        point_radius: f64,
        cancel: &AtomicBool,
    ) -> Result<Self, Error> {
        if plan.marks.len() > MAX_MARKS {
            return Err(Error::LimitExceeded);
        }
        if !plan.width.is_finite()
            || !plan.height.is_finite()
            || plan.width <= 0.
            || plan.height <= 0.
            || plan.width > 32768.
            || plan.height > 32768.
            || !point_radius.is_finite()
            || !(1. ..=12.).contains(&point_radius)
        {
            return Err(Error::InvalidGeometry);
        }
        let mut index = Self {
            horizontal: orientation.is_horizontal(),
            point_radius: (point_radius + 4.).max(8.),
            ..Self::default()
        };
        let mut columns = BTreeMap::<usize, Vec<usize>>::new();
        for (i, mark) in plan.marks.iter().enumerate() {
            if i % 256 == 0 && cancel.load(Ordering::Relaxed) {
                return Err(Error::Cancelled);
            }
            let bounds = bounds(mark.shape, index.point_radius);
            if ![bounds.left, bounds.top, bounds.right, bounds.bottom]
                .into_iter()
                .all(f64::is_finite)
                || bounds.left > bounds.right
                || bounds.top > bounds.bottom
            {
                return Err(Error::InvalidGeometry);
            }
            index.entries.push(Entry { bounds, mark: i });
            if let (Source::Cartesian { series, .. }, Shape::Dot { .. }) = (mark.source, mark.shape)
            {
                columns.entry(series).or_default().push(i);
            }
        }
        if !index.entries.is_empty() {
            build(&mut index.entries, 0, &mut index.nodes, cancel)?;
        }
        for mut column in columns.into_values() {
            if cancel.load(Ordering::Relaxed) {
                return Err(Error::Cancelled);
            }
            column.sort_unstable_by(|a, b| {
                coordinate(&plan.marks[*a], index.horizontal)
                    .total_cmp(&coordinate(&plan.marks[*b], index.horizontal))
                    .then(a.cmp(b))
            });
            index.columns.push(column);
        }
        if cancel.load(Ordering::Relaxed) {
            return Err(Error::Cancelled);
        }
        if index.retained_bytes() > MAX_BYTES {
            return Err(Error::LimitExceeded);
        }
        Ok(index)
    }
    pub fn retained_bytes(&self) -> usize {
        self.entries.capacity() * size_of::<Entry>()
            + self.nodes.capacity() * size_of::<Node>()
            + self.columns.capacity() * size_of::<Vec<usize>>()
            + self
                .columns
                .iter()
                .map(|c| c.capacity() * size_of::<usize>())
                .sum::<usize>()
    }
    fn visit(&self, node: usize, plan: &Plan, p: Point, best: &mut Option<usize>) {
        let node = &self.nodes[node];
        if !contains(node.bounds, p) {
            return;
        }
        if node.left != NONE {
            self.visit(node.left, plan, p, best);
            self.visit(node.right, plan, p, best);
            return;
        }
        for entry in &self.entries[node.start..node.end] {
            let mark = &plan.marks[entry.mark];
            if contains(entry.bounds, p)
                && hit(mark.shape, p, self.point_radius)
                && best.is_none_or(|old| prefer(plan, entry.mark, old, p))
            {
                *best = Some(entry.mark);
            }
        }
    }
    /// Exact mark hits first. Between line/area samples, optionally return the
    /// nearest plotted sample around the pointer's data-axis coordinate. This
    /// never invents an interpolated value or claims that a gap contains data.
    pub fn query(&self, plan: &Plan, p: Point, nearest_sample: bool) -> Option<usize> {
        if !p.x.is_finite()
            || !p.y.is_finite()
            || p.x < 0.
            || p.y < 0.
            || p.x > plan.width
            || p.y > plan.height
        {
            return None;
        }
        let mut best = None;
        if !self.nodes.is_empty() {
            self.visit(0, plan, p, &mut best);
        }
        if best.is_some() || !nearest_sample {
            return best;
        }
        let axis = if self.horizontal { p.y } else { p.x };
        let mut distance = f64::INFINITY;
        for column in &self.columns {
            let at = |i| coordinate(&plan.marks[column[i]], self.horizontal);
            if axis < at(0) || axis > at(column.len() - 1) {
                continue;
            }
            let right =
                column.partition_point(|i| coordinate(&plan.marks[*i], self.horizontal) < axis);
            for i in [
                right.checked_sub(1),
                (right < column.len()).then_some(right),
            ]
            .into_iter()
            .flatten()
            {
                let mark = column[i];
                let Shape::Dot { center, .. } = plan.marks[mark].shape else {
                    unreachable!()
                };
                let d = distance_squared(center, p);
                if d < distance || (d == distance && best.is_none_or(|old| mark > old)) {
                    best = Some(mark);
                    distance = d;
                }
            }
        }
        best
    }
}
fn prefer(plan: &Plan, new: usize, old: usize, p: Point) -> bool {
    let priority = |s: Shape| match s {
        Shape::Node(_) => 2,
        Shape::Ribbon { .. } => 0,
        _ => 1,
    };
    let a = plan.marks[new].shape;
    let b = plan.marks[old].shape;
    if priority(a) != priority(b) {
        return priority(a) > priority(b);
    }
    if let (Shape::Dot { center: a, .. }, Shape::Dot { center: b, .. }) = (a, b) {
        let a = distance_squared(a, p);
        let b = distance_squared(b, p);
        if a != b {
            return a < b;
        }
    }
    new > old
}

#[cfg(test)]
mod tests {
    use super::*;
    use gpuio_protocol::{
        chart_data::{Contents, Data, Layer, Series},
        chart_options::Options,
        chart_sampling::Policy,
    };
    fn prepared(data: &Data, policy: Policy, orientation: Orientation) -> (Plan, Index) {
        let options = Options {
            cartesian: gpuio_protocol::chart_options::Cartesian {
                orientation,
                ..Options::default().cartesian
            },
            ..Options::default()
        };
        let plan = crate::chart_geometry::prepare(
            data,
            policy,
            &options,
            640.,
            320.,
            &AtomicBool::new(false),
        )
        .unwrap();
        let index = Index::prepare(&plan, orientation, 3., &AtomicBool::new(false)).unwrap();
        (plan, index)
    }
    #[test]
    fn analytic_shapes_exclude_holes_and_match_curved_ribbons() {
        let wedge = Shape::Wedge {
            center: Point { x: 50., y: 50. },
            inner: 10.,
            outer: 40.,
            start: 0.,
            end: std::f64::consts::FRAC_PI_2,
        };
        assert!(!hit(wedge, Point { x: 50., y: 50. }, 8.));
        assert!(hit(wedge, Point { x: 70., y: 70. }, 8.));
        assert!(!hit(wedge, Point { x: 30., y: 70. }, 8.));
        let ribbon = Shape::Ribbon {
            start_top: Point { x: 0., y: 20. },
            start_bottom: Point { x: 0., y: 40. },
            end_top: Point { x: 100., y: 80. },
            end_bottom: Point { x: 100., y: 100. },
        };
        // t=.25 gives x=29.6875, upper y=29.375; not a linear trapezoid.
        assert!(hit(ribbon, Point { x: 29.6875, y: 30. }, 8.));
        assert!(!hit(ribbon, Point { x: 29.6875, y: 20. }, 8.));
        let candle = Shape::Candle {
            center: 20.,
            left: 19.9,
            right: 20.1,
            open: 30.,
            close: 40.,
            high: 10.,
            low: 60.,
        };
        let near_wick = Point { x: 22.9, y: 12. };
        assert!(hit(candle, near_wick, 8.));
        assert!(contains(bounds(candle, 8.), near_wick));
        assert!(!hit(candle, Point { x: 25., y: 12. }, 8.));
    }
    #[test]
    fn spatial_pruning_agrees_with_exhaustive_marks_for_every_family() {
        for fixture in include_str!("../../../test/fixtures/chart-v1-data.hex").lines() {
            let (_, hex) = fixture.split_once(' ').unwrap();
            let bytes = (0..hex.len())
                .step_by(2)
                .map(|i| u8::from_str_radix(&hex[i..i + 2], 16).unwrap())
                .collect::<Vec<_>>();
            let data = gpuio_protocol::decode_chart_data(&bytes).unwrap();
            for orientation in [
                Orientation::Vertical,
                Orientation::Horizontal,
                Orientation::VerticalReversed,
                Orientation::HorizontalReversed,
            ] {
                let (plan, index) = prepared(&data, Policy::default(), orientation);
                for x in 0..=40 {
                    for y in 0..=20 {
                        let p = Point {
                            x: x as f64 * 16.,
                            y: y as f64 * 16.,
                        };
                        let expected = plan
                            .marks
                            .iter()
                            .enumerate()
                            .filter(|(_, m)| hit(m.shape, p, index.point_radius))
                            .fold(None, |best, (i, _)| {
                                if best.is_none_or(|old| prefer(&plan, i, old, p)) {
                                    Some(i)
                                } else {
                                    best
                                }
                            });
                        assert_eq!(index.query(&plan, p, false), expected, "{fixture} at {p:?}");
                    }
                }
            }
        }
    }
    fn candidates(index: &Index, node: usize, p: Point) -> usize {
        let node = &index.nodes[node];
        if !contains(node.bounds, p) {
            0
        } else if node.left == NONE {
            node.end - node.start
        } else {
            candidates(index, node.left, p) + candidates(index, node.right, p)
        }
    }
    #[test]
    fn hundred_thousand_points_stay_bounded_and_queries_prune_work() {
        let data = Data {
            version: 1,
            contents: Contents::Cartesian(vec![Layer::Line(Series {
                id: 1,
                name: "100k".into(),
                points: (0..100_000)
                    .map(|i| gpuio_protocol::chart_data::Point {
                        id: i + 1,
                        x: i as f64,
                        y: Some((i % 17) as f64),
                        label: String::new(),
                    })
                    .collect(),
            })]),
        };
        let policy = Policy {
            line: gpuio_protocol::chart_sampling::Line::Exact,
            ..Default::default()
        };
        for orientation in [
            Orientation::Vertical,
            Orientation::Horizontal,
            Orientation::VerticalReversed,
            Orientation::HorizontalReversed,
        ] {
            let (plan, index) = prepared(&data, policy, orientation);
            assert_eq!(plan.marks.len(), 100_000);
            assert!(index.retained_bytes() < MAX_BYTES);
            for i in [0, 11111, 49999, 88888, 99999] {
                let Shape::Dot { center, .. } = plan.marks[i].shape else {
                    panic!("line point")
                };
                assert_eq!(index.query(&plan, center, true), Some(i));
                assert!(candidates(&index, 0, center) < 10_000);
            }
            assert_eq!(index.query(&plan, Point { x: f64::NAN, y: 0. }, true), None);
            assert_eq!(index.query(&plan, Point { x: -1., y: 0. }, true), None);
            assert!(matches!(
                Index::prepare(&plan, orientation, 3., &AtomicBool::new(true)),
                Err(Error::Cancelled)
            ));
        }
    }
    #[test]
    fn empty_and_nearest_sample_queries_do_not_invent_data() {
        let empty = Data {
            version: 1,
            contents: Contents::Cartesian(vec![]),
        };
        let (plan, index) = prepared(&empty, Policy::default(), Orientation::Vertical);
        assert_eq!(index.query(&plan, Point { x: 50., y: 50. }, true), None);
        let data = Data {
            version: 1,
            contents: Contents::Cartesian(vec![Layer::Line(Series {
                id: 1,
                name: "Line".into(),
                points: vec![
                    gpuio_protocol::chart_data::Point {
                        id: 9,
                        x: 0.,
                        y: Some(1.),
                        label: String::new(),
                    },
                    gpuio_protocol::chart_data::Point {
                        id: 3,
                        x: 1.,
                        y: Some(3.),
                        label: String::new(),
                    },
                ],
            })]),
        };
        let (plan, index) = prepared(&data, Policy::default(), Orientation::Vertical);
        let p = Point { x: 150., y: 160. };
        assert_eq!(index.query(&plan, p, false), None);
        assert_eq!(index.query(&plan, p, true), Some(0));
        assert!(matches!(
            Index::prepare(
                &plan,
                Orientation::Vertical,
                f64::NAN,
                &AtomicBool::new(false)
            ),
            Err(Error::InvalidGeometry)
        ));
    }
}
