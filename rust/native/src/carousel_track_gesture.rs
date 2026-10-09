//! Axis lock in pointer coordinates; movement and release in measured track pixels.
use crate::carousel_track_geometry::Geometry;
use gpuio_protocol::carousel::Axis;
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) enum Progress {
    Pending,
    Rejected,
    Dragging(f32),
}
pub(super) struct Drag {
    axis: Axis,
    origin: [f32; 2],
    start_offset: f32,
    start_index: usize,
    geometry: Geometry,
    delta: f64,
    progress: Progress,
}
impl Drag {
    pub fn new(
        axis: Axis,
        origin: [f32; 2],
        offset: f32,
        index: usize,
        geometry: Geometry,
    ) -> Option<Self> {
        (origin.iter().all(|v| v.is_finite())
            && geometry.snap(index).is_some()
            && geometry.bounded_offset(f64::from(offset)).is_some())
        .then_some(Self {
            axis,
            origin,
            start_offset: offset,
            start_index: index,
            geometry,
            delta: 0.,
            progress: Progress::Pending,
        })
    }
    pub fn geometry(&self) -> &Geometry {
        &self.geometry
    }
    pub fn follow_paint(&mut self, offset: f32) {
        if self.progress == Progress::Pending && offset.is_finite() {
            self.start_offset = offset;
        }
    }
    pub fn update(&mut self, point: [f32; 2]) -> Progress {
        if self.progress == Progress::Rejected {
            return self.progress;
        }
        if !point.iter().all(|v| v.is_finite()) {
            self.progress = Progress::Rejected;
            return self.progress;
        }
        let delta = [
            f64::from(point[0]) - f64::from(self.origin[0]),
            f64::from(point[1]) - f64::from(self.origin[1]),
        ];
        let (main, cross) = match self.axis {
            Axis::Horizontal => (delta[0], delta[1]),
            Axis::Vertical => (delta[1], delta[0]),
        };
        if self.progress == Progress::Pending {
            if cross.abs() >= 8. && cross.abs() > main.abs() * 1.25 {
                self.progress = Progress::Rejected;
                return self.progress;
            }
            if main.abs() < 8. || main.abs() < cross.abs() * 1.25 {
                return self.progress;
            }
        }
        self.delta = main;
        self.progress = self
            .geometry
            .bounded_offset(f64::from(self.start_offset) + main)
            .map_or(Progress::Rejected, Progress::Dragging);
        self.progress
    }
    pub fn finish(&self) -> Option<usize> {
        let Progress::Dragging(offset) = self.progress else {
            return None;
        };
        self.geometry
            .release_target(self.start_index, self.delta, offset)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::carousel_track_geometry::Item;
    fn geometry(viewport: f32, looping: bool) -> Geometry {
        Geometry::new(
            viewport,
            (240. - viewport).max(0.),
            vec![
                Item {
                    start: 0.,
                    extent: 40.,
                },
                Item {
                    start: 48.,
                    extent: 120.,
                },
                Item {
                    start: 176.,
                    extent: 64.,
                },
            ],
            looping,
        )
        .unwrap()
    }
    #[test]
    fn unequal_cards_snap_by_pixels_and_axis_lock_preserves_cross_axis_input() {
        for axis in [Axis::Horizontal, Axis::Vertical] {
            let point = |main, cross| {
                if axis == Axis::Horizontal {
                    [main, cross]
                } else {
                    [cross, main]
                }
            };
            let mut drag = Drag::new(axis, [0., 0.], 0., 0, geometry(100., false)).unwrap();
            assert_eq!(drag.update(point(-7., 0.)), Progress::Pending);
            assert_eq!(drag.finish(), None);
            assert_eq!(drag.update(point(-30., 0.)), Progress::Dragging(-30.));
            assert_eq!(drag.finish(), Some(1));
            assert_eq!(drag.update(point(-130., 0.)), Progress::Dragging(-130.));
            assert_eq!(drag.finish(), Some(2));
            assert_eq!(drag.update(point(-1000., 0.)), Progress::Dragging(-140.));
            let mut crossed = Drag::new(axis, [0., 0.], 0., 0, geometry(100., false)).unwrap();
            assert_eq!(crossed.update(point(1., 12.)), Progress::Rejected);
            assert_eq!(crossed.update(point(-100., 0.)), Progress::Rejected);
            assert_eq!(crossed.finish(), None);
        }
    }
    #[test]
    fn boundary_jump_and_continuous_offsets_use_one_canonical_cycle() {
        let g = geometry(150., true);
        let mut jump = Drag::new(Axis::Horizontal, [0., 0.], 0., 0, g).unwrap();
        assert_eq!(jump.update([30., 0.]), Progress::Dragging(0.));
        assert_eq!(jump.finish(), Some(0));
        jump.update([40., 0.]);
        assert_eq!(jump.finish(), Some(2));
        let g = geometry(100., true);
        let start = g.snap(1).unwrap();
        let mut drag = Drag::new(Axis::Horizontal, [0., 0.], start, 1, g.clone()).unwrap();
        for delta in [-10_000., -500., -50., 50., 500., 10_000.] {
            let Progress::Dragging(offset) = drag.update([delta, 0.]) else {
                panic!("drag");
            };
            assert_eq!(g.bounded_offset(f64::from(offset)), Some(offset));
            assert!(drag.finish().is_some());
        }
        assert_eq!(drag.update([f32::NAN, 0.]), Progress::Rejected);
        assert_eq!(drag.finish(), None);
    }
    #[test]
    fn pending_drag_follows_paint_until_claim_then_freezes_origin() {
        let mut drag =
            Drag::new(Axis::Horizontal, [0., 0.], -20., 0, geometry(100., false)).unwrap();
        drag.follow_paint(-30.);
        assert_eq!(drag.update([-10., 0.]), Progress::Dragging(-40.));
        drag.follow_paint(-60.);
        assert_eq!(drag.update([-20., 0.]), Progress::Dragging(-50.));
        assert!(
            Drag::new(
                Axis::Horizontal,
                [0., f32::INFINITY],
                0.,
                0,
                geometry(100., false)
            )
            .is_none()
        );
    }
}
