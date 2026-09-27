//! Retained-canvas geometry, independent of GPUI and runtime ownership.
use binprot::macros::BinProtWrite;

pub const COORDINATE_LIMIT: f64 = 1_000_000.;
pub const HIT_TOLERANCE: f64 = 1e-7;
fn coordinate(value: f64) -> bool {
    value.is_finite() && value.abs() <= COORDINATE_LIMIT
}

pub const MAX_PATH_COMMANDS: usize = 4096;

#[derive(Clone, Debug, PartialEq, BinProtWrite)]
pub enum PathCommand {
    Move(Point),
    Line(Point),
    Quadratic(Point, Point),
    Cubic(Point, Point, Point),
    Close,
}
impl PathCommand {
    fn is_valid(&self) -> bool {
        match self {
            Self::Move(p) | Self::Line(p) => p.is_valid(),
            Self::Quadratic(control, p) => control.is_valid() && p.is_valid(),
            Self::Cubic(first, second, p) => first.is_valid() && second.is_valid() && p.is_valid(),
            Self::Close => true,
        }
    }
}

#[derive(Clone, Debug, PartialEq, BinProtWrite)]
pub struct Path(pub Vec<PathCommand>);
impl Path {
    pub fn is_valid(&self) -> bool {
        self.is_valid_up_to(MAX_PATH_COMMANDS)
    }
    /// Native-generated geometry may have a different admission budget. Wire
    /// readers and canvas scenes always use `is_valid` and its fixed limit.
    pub fn is_valid_up_to(&self, max_commands: usize) -> bool {
        if self.0.len() > max_commands {
            return false;
        }
        let mut active = false;
        let mut segments = 0;
        let mut has_drawing = false;
        for command in &self.0 {
            if !command.is_valid() {
                return false;
            }
            match command {
                PathCommand::Move(_) => {
                    if active && segments == 0 {
                        return false;
                    }
                    active = true;
                    segments = 0;
                }
                PathCommand::Line(_)
                | PathCommand::Quadratic(_, _)
                | PathCommand::Cubic(_, _, _) => {
                    if !active {
                        return false;
                    }
                    segments += 1;
                    has_drawing = true;
                }
                PathCommand::Close => {
                    if !active || segments == 0 {
                        return false;
                    }
                    active = false;
                    segments = 0;
                }
            }
        }
        has_drawing && (!active || segments > 0)
    }
    pub fn is_closed(&self) -> bool {
        self.is_closed_up_to(MAX_PATH_COMMANDS)
    }
    /// Closed-contour validation for the same native-generated path boundary.
    pub fn is_closed_up_to(&self, max_commands: usize) -> bool {
        if !self.is_valid_up_to(max_commands) {
            return false;
        }
        let mut active = false;
        for command in &self.0 {
            match command {
                PathCommand::Move(_) => {
                    if active {
                        return false;
                    }
                    active = true;
                }
                PathCommand::Close => active = false,
                PathCommand::Line(_)
                | PathCommand::Quadratic(_, _)
                | PathCommand::Cubic(_, _, _) => {}
            }
        }
        !active
    }
}

#[derive(Clone, Copy, Debug, PartialEq, BinProtWrite)]
pub struct Point {
    pub x: f64,
    pub y: f64,
}
impl Point {
    pub fn is_valid(self) -> bool {
        coordinate(self.x) && coordinate(self.y)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, BinProtWrite)]
pub struct Rect {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}
impl Rect {
    pub fn is_valid(self) -> bool {
        coordinate(self.x)
            && coordinate(self.y)
            && coordinate(self.width)
            && coordinate(self.height)
            && self.width > 0.
            && self.height > 0.
            && coordinate(self.x + self.width)
            && coordinate(self.y + self.height)
    }
    pub fn contains(self, point: Point) -> bool {
        point.x >= self.x - HIT_TOLERANCE
            && point.y >= self.y - HIT_TOLERANCE
            && point.x <= self.x + self.width + HIT_TOLERANCE
            && point.y <= self.y + self.height + HIT_TOLERANCE
    }
    pub fn intersect(self, other: Self) -> Option<Self> {
        let x = self.x.max(other.x);
        let y = self.y.max(other.y);
        let right = (self.x + self.width).min(other.x + other.width);
        let bottom = (self.y + self.height).min(other.y + other.height);
        (right > x && bottom > y).then_some(Self {
            x,
            y,
            width: right - x,
            height: bottom - y,
        })
    }
}

#[derive(Clone, Copy, Debug, PartialEq, BinProtWrite)]
pub struct Transform {
    pub a: f64,
    pub b: f64,
    pub c: f64,
    pub d: f64,
    pub tx: f64,
    pub ty: f64,
}
impl Transform {
    pub const IDENTITY: Self = Self {
        a: 1.,
        b: 0.,
        c: 0.,
        d: 1.,
        tx: 0.,
        ty: 0.,
    };
    pub fn determinant(self) -> f64 {
        self.a * self.d - self.b * self.c
    }
    pub fn is_valid(self) -> bool {
        [self.a, self.b, self.c, self.d]
            .iter()
            .all(|v| v.is_finite() && v.abs() <= 1000.)
            && coordinate(self.tx)
            && coordinate(self.ty)
            && self.determinant().abs() >= 1e-8
    }
    pub fn apply(self, point: Point) -> Point {
        Point {
            x: self.a * point.x + self.c * point.y + self.tx,
            y: self.b * point.x + self.d * point.y + self.ty,
        }
    }
    /// Local is applied first, then self (the parent).
    pub fn compose(self, local: Self) -> Self {
        Self {
            a: self.a * local.a + self.c * local.b,
            b: self.b * local.a + self.d * local.b,
            c: self.a * local.c + self.c * local.d,
            d: self.b * local.c + self.d * local.d,
            tx: self.a * local.tx + self.c * local.ty + self.tx,
            ty: self.b * local.tx + self.d * local.ty + self.ty,
        }
    }
    pub fn inverse(self) -> Option<Self> {
        if !self.is_valid() {
            return None;
        }
        let det = self.determinant();
        Some(Self {
            a: self.d / det,
            b: -self.b / det,
            c: -self.c / det,
            d: self.a / det,
            tx: (self.c * self.ty - self.d * self.tx) / det,
            ty: (self.b * self.tx - self.a * self.ty) / det,
        })
    }
    pub fn unapply(self, point: Point) -> Option<Point> {
        if !self.is_valid() {
            return None;
        }
        let x = point.x - self.tx;
        let y = point.y - self.ty;
        let determinant = self.determinant();
        Some(Point {
            x: (self.d * x - self.c * y) / determinant,
            y: (self.a * y - self.b * x) / determinant,
        })
    }
}

#[derive(Clone, Debug, PartialEq, BinProtWrite)]
pub enum HitRegion {
    Rectangle(Rect),
    Ellipse(Rect),
    Polygon(Vec<Point>),
}
impl HitRegion {
    pub fn is_valid(&self) -> bool {
        match self {
            Self::Rectangle(rect) | Self::Ellipse(rect) => rect.is_valid(),
            Self::Polygon(points) => {
                (3..=256).contains(&points.len())
                    && points.iter().all(|p| p.is_valid())
                    && points.first().is_some_and(|first| {
                        points.iter().find(|p| *p != first).is_some_and(|second| {
                            points.iter().any(|p| {
                                ((second.x - first.x) * (p.y - first.y)
                                    - (second.y - first.y) * (p.x - first.x))
                                    .abs()
                                    > 1e-8
                            })
                        })
                    })
            }
        }
    }
    pub fn contains(&self, point: Point) -> bool {
        match self {
            Self::Rectangle(rect) => rect.contains(point),
            Self::Ellipse(rect) => {
                let x = (point.x - rect.x - rect.width / 2.) / (rect.width / 2. + HIT_TOLERANCE);
                let y = (point.y - rect.y - rect.height / 2.) / (rect.height / 2. + HIT_TOLERANCE);
                x * x + y * y <= 1.
            }
            Self::Polygon(points) => {
                let Some(mut a) = points.last().copied() else {
                    return false;
                };
                let mut inside = false;
                for &b in points {
                    if on_edge(point, a, b) {
                        return true;
                    }
                    if (a.y > point.y) != (b.y > point.y)
                        && point.x < (b.x - a.x) * (point.y - a.y) / (b.y - a.y) + a.x
                    {
                        inside = !inside;
                    }
                    a = b;
                }
                inside
            }
        }
    }
    pub fn hit(&self, transform: Transform, clips: &[Rect], point: Point) -> bool {
        let in_clip = match clips.split_first() {
            None => true,
            Some((first, rest)) => rest
                .iter()
                .try_fold(*first, |current, next| current.intersect(*next))
                .is_some_and(|clip| clip.contains(point)),
        };
        in_clip
            && transform
                .unapply(point)
                .is_some_and(|local| self.contains(local))
    }
}
fn on_edge(point: Point, a: Point, b: Point) -> bool {
    let dx = b.x - a.x;
    let dy = b.y - a.y;
    let cross = (point.x - a.x) * dy - (point.y - a.y) * dx;
    let epsilon = HIT_TOLERANCE;
    cross.abs() <= epsilon * dx.abs().max(dy.abs())
        && point.x >= a.x.min(b.x) - epsilon
        && point.x <= a.x.max(b.x) + epsilon
        && point.y >= a.y.min(b.y) - epsilon
        && point.y <= a.y.max(b.y) + epsilon
}

#[cfg(test)]
mod tests {
    use super::*;
    use binprot::BinProtWrite;
    fn point(x: f64, y: f64) -> Point {
        Point { x, y }
    }
    fn rect(x: f64, y: f64, width: f64, height: f64) -> Rect {
        Rect {
            x,
            y,
            width,
            height,
        }
    }
    fn scale(x: f64, y: f64) -> Transform {
        Transform {
            a: x,
            d: y,
            ..Transform::IDENTITY
        }
    }
    fn translate(x: f64, y: f64) -> Transform {
        Transform {
            tx: x,
            ty: y,
            ..Transform::IDENTITY
        }
    }

    #[test]
    fn admission_rejects_nonfinite_singular_and_out_of_domain_geometry() {
        for p in [
            point(f64::NAN, 0.),
            point(f64::INFINITY, 0.),
            point(1_000_001., 0.),
        ] {
            assert!(!p.is_valid());
        }
        assert!(!rect(0., 0., 0., 1.).is_valid());
        assert!(!rect(999_999., 0., 2., 1.).is_valid());
        assert!(!scale(0., 1.).is_valid());
        assert!(!scale(1e-5, 1e-5).is_valid());
        assert!(!scale(1000., 1.).compose(scale(2., 1.)).is_valid());
        assert!(!translate(1_000_000., 0.).apply(point(1., 0.)).is_valid());
        assert!(scale(0.0001, 1.).inverse().is_some());
        for points in [
            vec![],
            vec![point(0., 0.); 3],
            vec![point(0., 0.), point(1., 1.), point(2., 2.)],
            vec![point(0., 0.); 257],
        ] {
            assert!(!HitRegion::Polygon(points).is_valid());
        }
    }

    #[test]
    fn reflection_composition_and_world_clipping_match_local_containment() {
        let transform = translate(100., 50.).compose(scale(-2., 3.));
        assert_eq!(transform.apply(point(4., 5.)), point(92., 65.));
        assert_eq!(transform.unapply(point(92., 65.)), Some(point(4., 5.)));
        let region = HitRegion::Rectangle(rect(0., 0., 10., 10.));
        let points = [
            point(80., 50.),
            point(100., 80.),
            point(90., 65.),
            point(79., 65.),
            point(101., 65.),
        ];
        assert_eq!(
            points.map(|p| region.hit(transform, &[], p)),
            [true, true, true, false, false]
        );
        let clips = [rect(85., 55., 10., 20.), rect(88., 60., 5., 10.)];
        assert_eq!(
            points.map(|p| region.hit(transform, &clips, p)),
            [false, false, true, false, false]
        );
        assert!(
            rect(0., 0., 10., 10.)
                .intersect(rect(10., 0., 10., 10.))
                .is_none()
        );
        assert_eq!(
            rect(0., 0., 10., 10.).intersect(rect(5., 6., 10., 10.)),
            Some(rect(5., 6., 5., 4.))
        );
        assert!(!region.hit(
            Transform::IDENTITY,
            &[rect(0., 0., 5., 5.), rect(5., 0., 5., 5.)],
            point(5., 2.)
        ));
    }

    #[test]
    fn ellipse_and_concave_polygon_boundaries_do_not_depend_on_winding() {
        let ellipse = HitRegion::Ellipse(rect(0., 0., 20., 10.));
        assert_eq!(
            [
                point(10., 5.),
                point(20., 5.),
                point(0., 0.),
                point(20.0001, 5.)
            ]
            .map(|p| ellipse.contains(p)),
            [true, true, false, false]
        );
        let points = vec![
            point(0., 0.),
            point(10., 0.),
            point(10., 4.),
            point(4., 4.),
            point(4., 10.),
            point(0., 10.),
        ];
        let polygon = HitRegion::Polygon(points.clone());
        let reversed = HitRegion::Polygon(points.into_iter().rev().collect());
        let probes = [
            point(1., 1.),
            point(8., 2.),
            point(2., 8.),
            point(8., 8.),
            point(4., 6.),
            point(4.001, 6.),
        ];
        assert_eq!(
            probes.map(|p| polygon.contains(p)),
            [true, true, true, false, true, false]
        );
        assert_eq!(
            probes.map(|p| polygon.contains(p)),
            probes.map(|p| reversed.contains(p))
        );
    }

    #[test]
    fn inverse_round_trips_rotated_sheared_and_reflected_points() {
        let angle = 0.7_f64;
        let transforms = [
            Transform::IDENTITY,
            scale(-2., 3.),
            Transform {
                a: 1.,
                b: 0.5,
                c: 0.25,
                d: 1.,
                tx: 500.,
                ty: -300.,
            },
            Transform {
                a: angle.cos(),
                b: angle.sin(),
                c: -angle.sin(),
                d: angle.cos(),
                tx: 0.,
                ty: 0.,
            },
        ];
        for transform in transforms {
            for x in -10..=10 {
                for y in -10..=10 {
                    let p = point(f64::from(x) * 1234., f64::from(y) * 789.);
                    let recovered = transform.inverse().unwrap().apply(transform.apply(p));
                    assert!((p.x - recovered.x).abs() < 1e-8);
                    assert!((p.y - recovered.y).abs() < 1e-8);
                }
            }
        }
    }

    #[test]
    fn contour_topology_distinguishes_open_strokes_from_closed_fills() {
        use PathCommand::*;
        let p = point(0., 0.);
        let q = point(10., 10.);
        let cases = [
            (vec![Move(p), Line(q)], false),
            (vec![Move(p), Line(q), Close], true),
            (vec![Move(p), Quadratic(p, q), Close], true),
            (vec![Move(p), Cubic(p, q, p), Close], true),
            (vec![Move(p), Line(q), Move(q), Line(p), Close], false),
            (vec![Move(p), Line(q), Close, Move(q), Line(p), Close], true),
        ];
        for (commands, closed) in cases {
            let path = Path(commands);
            assert!(path.is_valid());
            assert_eq!(path.is_closed(), closed);
        }
        for commands in [
            vec![],
            vec![Move(p)],
            vec![Line(q)],
            vec![Close],
            vec![Move(p), Close],
            vec![Move(p), Move(q), Line(p)],
            vec![Move(p), Line(q), Close, Line(p)],
            vec![Move(p), Line(q), Close, Close],
            vec![Move(p), Line(q), Move(p)],
            vec![Move(p), Cubic(p, point(f64::NAN, 0.), q)],
        ] {
            assert!(!Path(commands).is_valid());
        }
        let mut maximum = Path(vec![Line(q); MAX_PATH_COMMANDS]);
        maximum.0[0] = Move(p);
        assert!(maximum.is_valid());
        maximum.0.push(Line(q));
        assert!(!maximum.is_valid());
    }

    #[test]
    fn boundary_tolerance_does_not_fill_nearby_outside_points() {
        let region = HitRegion::Rectangle(rect(0., 0., 10., 10.));
        assert_eq!(
            [
                point(-5e-8, 5.),
                point(-2e-7, 5.),
                point(10.00000005, 5.),
                point(10.0000002, 5.)
            ]
            .map(|p| region.contains(p)),
            [true, false, true, false]
        );
        let polygon =
            HitRegion::Polygon(vec![point(0., 0.), point(0.001, 0.), point(0.001, 0.001)]);
        assert!(polygon.contains(point(0.0005, 0.00050005)));
        assert!(!polygon.contains(point(0.0005, 0.0005002)));
    }

    #[test]
    fn path_encoding_matches_all_ocaml_command_tags_and_control_point_order() {
        use PathCommand::*;
        let path = Path(vec![
            Move(point(0., 0.)),
            Line(point(1., 2.)),
            Quadratic(point(3., 4.), point(5., 6.)),
            Cubic(point(7., 8.), point(9., 10.), point(11., 12.)),
            Close,
        ]);
        let mut bytes = Vec::new();
        path.binprot_write(&mut bytes).unwrap();
        let encoded = bytes
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect::<String>();
        assert_eq!(
            encoded,
            include_str!("../../../test/fixtures/canvas-v1-path.hex").trim()
        );
    }
}
