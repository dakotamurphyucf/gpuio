//! Paint-only thumb interaction springs. One weak frame request per slider.
use super::*;
use crate::motion::spring::Trajectory;
use gpuio_protocol::animation::{Property, Spring};
use std::time::Instant;

#[derive(Default)]
struct Ring {
    value: f64,
    velocity: f64,
    target: f64,
    path: Option<(Trajectory, Instant)>,
    painted: bool,
}
impl Ring {
    fn sample(&mut self, target: bool, reduced: bool, now: Instant) -> f64 {
        let target = if target { 1. } else { 0. };
        if reduced {
            self.value = target;
            self.velocity = 0.;
            self.target = target;
            self.path = None;
            return target;
        }
        if let Some((path, start)) = &self.path {
            let sample = path.sample(now.saturating_duration_since(*start));
            self.value = sample.position;
            self.velocity = sample.velocity;
            if sample.finished {
                self.path = None;
            }
        }
        if target != self.target {
            // Match the pinned control spring's 180ms response and critical damping.
            let frequency = std::f64::consts::TAU / 0.180;
            let parameters = Spring {
                stiffness: frequency * frequency,
                damping: 2. * frequency,
                mass: 1.,
                epsilon: 0.001,
                max_duration_ms: 2000,
            };
            self.path = Some((
                Trajectory::new(
                    parameters,
                    Property::Opacity,
                    self.value,
                    self.velocity,
                    target,
                )
                .expect("bounded interaction spring"),
                now,
            ));
            self.target = target;
        }
        self.value
    }
}
#[derive(Default)]
pub(super) struct Rings {
    items: [Ring; 2],
    pending: Option<Rc<()>>,
}
impl Rings {
    pub(super) fn prepare(&mut self) {
        for ring in &mut self.items {
            ring.painted = false;
        }
    }
    pub(super) fn reset(&mut self) {
        *self = Self::default();
    }
    fn running(&self) -> bool {
        self.items
            .iter()
            .any(|ring| ring.painted && ring.path.is_some())
    }
}
impl State {
    fn ring_allowed(&self, window: &Window) -> bool {
        window.is_window_active() && self.allowed(true) && !self.model.config().read_only
    }
}
fn schedule(shared: &Shared, window: &mut Window) {
    let mut state = shared.borrow_mut();
    if !state.rings.running() || state.rings.pending.is_some() {
        return;
    }
    let token = Rc::new(());
    let lease = Rc::downgrade(&token);
    state.rings.pending = Some(token);
    let weak = Rc::downgrade(shared);
    window.on_next_frame(move |window, _cx| {
        let (Some(state), Some(token)) = (weak.upgrade(), lease.upgrade()) else {
            return;
        };
        let mut state = state.borrow_mut();
        if !state
            .rings
            .pending
            .as_ref()
            .is_some_and(|current| Rc::ptr_eq(current, &token))
        {
            return;
        }
        state.rings.pending = None;
        if !state.ring_allowed(window) || !state.rings.running() {
            state.rings.reset();
            return;
        }
        // Reduced motion still receives this final paint to settle to its target.
        window.refresh();
    });
}

pub(super) fn paint(
    shared: &Shared,
    part: usize,
    hitbox: &Hitbox,
    bounds: Bounds<Pixels>,
    window: &mut Window,
    cx: &mut App,
) {
    let mut state = shared.borrow_mut();
    let clip = bounds.intersect(&window.content_mask().bounds);
    let color = state
        .appearance
        .ring_color
        .map_or(window.text_style().color, |c| rgba(c as u32).into());
    if !state.ring_allowed(window) {
        state.rings.reset();
        return;
    }
    if clip.size.width <= px(0.)
        || clip.size.height <= px(0.)
        || window.element_opacity() <= 0.
        || color.a <= 0.
    {
        state.rings.items[part] = Ring::default();
        return;
    }
    let thumb = state.focus[part].0;
    let pressed = state.capture.is_some() && state.model.snapshot().dragging == Some(thumb);
    let target = hitbox.is_hovered(window) || pressed;
    let ring = &mut state.rings.items[part];
    ring.painted = true;
    let progress = ring.sample(target, cx.reduce_motion(), cx.background_executor().now()) as f32;
    let thumb_size = px(state.appearance.thumb_size as f32);
    drop(state);
    if progress > 0. {
        let width = px(3. * progress);
        let inner = Bounds::new(
            bounds.center() - point(thumb_size / 2., thumb_size / 2.),
            size(thumb_size, thumb_size),
        );
        let mut ring = outline(
            inner.dilate(width),
            color.opacity(0.5 * progress),
            BorderStyle::Solid,
        );
        ring.corner_radii = (thumb_size / 2. + width).into();
        ring.border_widths = width.into();
        window.paint_quad(ring);
    }
    schedule(shared, window);
}

#[cfg(test)]
mod model_tests {
    use super::*;
    use std::time::Duration;
    #[::core::prelude::v1::test]
    fn spring_retargets_continuously_and_settles_without_idle_work() {
        let now = Instant::now();
        let mut ring = Ring::default();
        assert_eq!(ring.sample(false, false, now), 0.);
        assert!(ring.path.is_none());
        assert_eq!(ring.sample(true, false, now), 0.);
        let mid = ring.sample(true, false, now + Duration::from_millis(45));
        assert!(mid > 0. && mid < 1.);
        assert_eq!(
            ring.sample(false, false, now + Duration::from_millis(45)),
            mid
        );
        assert_eq!(ring.sample(false, false, now + Duration::from_secs(3)), 0.);
        assert!(ring.path.is_none());
        ring.sample(true, false, now + Duration::from_secs(4));
        assert_eq!(ring.sample(true, false, now + Duration::from_secs(7)), 1.);
        assert!(ring.path.is_none());
        assert_eq!(ring.sample(true, false, now + Duration::from_secs(900)), 1.);
        assert!(ring.path.is_none());
    }
    #[::core::prelude::v1::test]
    fn reduced_motion_and_unpainted_frames_do_not_keep_motion_alive() {
        let now = Instant::now();
        let mut rings = Rings::default();
        rings.items[0].painted = true;
        rings.items[0].sample(true, false, now);
        assert!(rings.running());
        rings.prepare();
        assert!(!rings.running());
        rings.items[0].painted = true;
        assert_eq!(rings.items[0].sample(true, true, now), 1.);
        assert!(!rings.running());
        assert_eq!(rings.items[0].sample(false, true, now), 0.);
        rings.pending = Some(Rc::new(()));
        let lease = Rc::downgrade(rings.pending.as_ref().unwrap());
        rings.reset();
        assert!(lease.upgrade().is_none());
    }
}

#[cfg(all(test, feature = "native-image-tests"))]
#[path = "slider_ring_test.rs"]
mod native_tests;
