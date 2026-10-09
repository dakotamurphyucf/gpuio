//! Paint-committed tab geometry. Two finite springs, no timer or item resources.
use super::View;
use crate::motion::spring::{Sample, Trajectory};
use gpui::{Bounds, Hsla, IntoElement, Pixels, canvas, prelude::*, px};
use gpuio_protocol::{animation::Property, tab_appearance::Variant, tab_motion::Config};
use std::{
    cell::{Cell, RefCell},
    rc::{Rc, Weak},
    time::Instant,
};

#[derive(Clone, Copy, Debug, PartialEq)]
struct Geometry {
    left: f64,
    width: f64,
}
struct Travel {
    left: Trajectory,
    width: Trajectory,
    start: Instant,
}
struct History {
    selected: String,
    variant: Variant,
    config: Config,
    target: Geometry,
    left: Sample,
    width: Sample,
    travel: Option<Travel>,
    fade: Option<Instant>,
}
#[derive(Default)]
pub(super) struct State {
    history: Option<History>,
    painted: bool,
}
fn settled(position: f64) -> Sample {
    Sample {
        position,
        velocity: 0.,
        finished: true,
    }
}
fn cubic(t: f32) -> f32 {
    if t < 0.5 {
        4. * t.powi(3)
    } else {
        1. - (-2. * t + 2.).powi(3) / 2.
    }
}
impl State {
    fn color_progress(
        &self,
        selected: &str,
        variant: Variant,
        config: Config,
        now: Instant,
        immediate: bool,
    ) -> f32 {
        if immediate || variant != Variant::Pill || config.color_duration_ms == 0 {
            return 1.;
        }
        let Some(h) = &self.history else {
            return 1.;
        };
        if h.variant != variant || h.config != config {
            return 1.;
        }
        if h.selected != selected {
            return 0.;
        }
        h.fade.map_or(1., |start| {
            cubic(
                (now.saturating_duration_since(start).as_secs_f32() * 1000.
                    / config.color_duration_ms as f32)
                    .min(1.),
            )
        })
    }
    fn paint(
        &mut self,
        selected: &str,
        variant: Variant,
        config: Config,
        target: Geometry,
        now: Instant,
        immediate: bool,
    ) -> (Geometry, bool) {
        self.painted = true;
        if immediate
            || self
                .history
                .as_ref()
                .is_none_or(|h| h.variant != variant || h.config != config)
        {
            self.history = Some(History {
                selected: selected.into(),
                variant,
                config,
                target,
                left: settled(target.left),
                width: settled(target.width),
                travel: None,
                fade: None,
            });
        }
        let h = self.history.as_mut().expect("painted history");
        if h.selected != selected || h.target != target {
            let left = Trajectory::new(
                config.spring,
                Property::Left,
                h.left.position,
                h.left.velocity,
                target.left,
            );
            let width = Trajectory::new(
                config.spring,
                Property::Width,
                h.width.position,
                h.width.velocity,
                target.width,
            );
            // Valid layout can exceed the generic animation property's coordinate
            // domain. Adopt it exactly instead of clamping or panicking.
            h.travel = match (left, width) {
                (Ok(left), Ok(width)) => Some(Travel {
                    left,
                    width,
                    start: now,
                }),
                _ => {
                    h.left = settled(target.left);
                    h.width = settled(target.width);
                    None
                }
            };
            if h.selected != selected {
                h.selected = selected.into();
                h.fade = (variant == Variant::Pill && config.color_duration_ms > 0 && !immediate)
                    .then_some(now);
            }
            h.target = target;
        }
        if let Some(travel) = &h.travel {
            let elapsed = now.saturating_duration_since(travel.start);
            h.left = travel.left.sample(elapsed);
            h.width = travel.width.sample(elapsed);
            if h.left.finished && h.width.finished {
                h.travel = None;
            }
        }
        if h.fade.is_some_and(|start| {
            now.saturating_duration_since(start).as_millis() >= config.color_duration_ms as u128
        }) {
            h.fade = None;
        }
        (
            Geometry {
                left: h.left.position,
                width: h.width.position,
            },
            h.travel.is_some() || h.fade.is_some(),
        )
    }
}

pub(super) struct Owner {
    pub state: Rc<RefCell<State>>,
    pub config: Config,
}
struct Frame {
    owner: Weak<RefCell<State>>,
    selected: String,
    variant: Variant,
    config: Config,
    first: Cell<Option<Bounds<Pixels>>>,
    selected_bounds: Cell<Option<Bounds<Pixels>>>,
    now: Instant,
    immediate: bool,
    opacity: Cell<f32>,
}
#[derive(Clone)]
pub(super) struct Binding(Rc<Frame>);
impl Binding {
    pub(super) fn new(
        owner: Owner,
        selected: &str,
        variant: Variant,
        immediate: bool,
        now: Instant,
    ) -> Option<Self> {
        if !matches!(
            variant,
            Variant::Pill | Variant::Segmented | Variant::Underline
        ) {
            owner.state.borrow_mut().history = None;
            return None;
        }
        Some(Self(Rc::new(Frame {
            owner: Rc::downgrade(&owner.state),
            selected: selected.into(),
            variant,
            config: owner.config,
            first: Cell::new(None),
            selected_bounds: Cell::new(None),
            now,
            immediate,
            opacity: Cell::new(1.),
        })))
    }
    pub(super) fn record(&self, first: bool, selected: bool, bounds: Bounds<Pixels>) {
        if first {
            self.0.first.set(Some(bounds));
        }
        if selected {
            self.0.selected_bounds.set(Some(bounds));
        }
    }
    pub(super) fn set_opacity(&self, opacity: f32) {
        self.0.opacity.set(opacity);
    }
    pub(super) fn color(&self, normal: Hsla, target: Hsla) -> Hsla {
        let f = &self.0;
        let t = f.owner.upgrade().map_or(1., |owner| {
            owner
                .borrow()
                .color_progress(&f.selected, f.variant, f.config, f.now, f.immediate)
        });
        // Linearly interpolate channels in GPUI's RGBA representation;
        // descendants with an explicit foreground still override inheritance.
        let a = normal.to_rgb();
        let b = target.to_rgb();
        gpui::Rgba {
            r: a.r + (b.r - a.r) * t,
            g: a.g + (b.g - a.g) * t,
            b: a.b + (b.b - a.b) * t,
            a: a.a + (b.a - a.a) * t,
        }
        .into()
    }
    pub(super) fn indicator(&self, accent: Hsla) -> impl IntoElement {
        let frame = self.0.clone();
        canvas(
            |_, _, _| (),
            move |bounds, _, window, _| {
                let Some(owner) = frame.owner.upgrade() else {
                    return;
                };
                let Some((first, selected)) = frame.first.get().zip(frame.selected_bounds.get())
                else {
                    return;
                };
                let visible = bounds.intersect(&window.content_mask().bounds);
                if visible.size.width <= px(0.)
                    || visible.size.height <= px(0.)
                    || selected.size.width <= px(0.)
                    || selected.size.height <= px(0.)
                {
                    return;
                }
                let target = Geometry {
                    left: f32::from(selected.origin.x - first.origin.x).into(),
                    width: f32::from(selected.size.width).into(),
                };
                if !target.left.is_finite() || !target.width.is_finite() {
                    return;
                }
                let (sample, running) = owner.borrow_mut().paint(
                    &frame.selected,
                    frame.variant,
                    frame.config,
                    target,
                    frame.now,
                    frame.immediate,
                );
                let mut rect = selected;
                rect.origin.x = first.origin.x + px(sample.left as f32);
                rect.size.width = px(sample.width as f32);
                let (radius, color) = match frame.variant {
                    Variant::Pill => (rect.size.height / 2., accent.alpha(0.15)),
                    Variant::Segmented => (px(6.), accent.alpha(0.15)),
                    Variant::Underline => {
                        rect.origin.y += rect.size.height - px(2.);
                        rect.size.height = px(2.);
                        (px(0.), accent)
                    }
                    _ => unreachable!(),
                };
                window.paint_quad(
                    gpui::fill(rect, color.alpha(color.a * frame.opacity.get()))
                        .corner_radii(radius),
                );
                if running {
                    window.request_animation_frame();
                }
            },
        )
        .absolute()
        .top_0()
        .left_0()
        .size_full()
    }
}
impl View {
    pub(super) fn begin_tab_motion_paint(&mut self) {
        for owner in self.tab_motions.values() {
            owner.borrow_mut().painted = false;
        }
    }
    pub(super) fn finish_tab_motion_paint(&mut self) {
        for owner in self.tab_motions.values() {
            let mut owner = owner.borrow_mut();
            if !owner.painted {
                owner.history = None;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;
    fn geometry(left: f64, width: f64) -> Geometry {
        Geometry { left, width }
    }
    #[test]
    fn first_paint_interruption_and_geometry_retarget_preserve_painted_velocity() {
        let now = Instant::now();
        let config = Config::default();
        let mut state = State::default();
        let a = geometry(0., 80.);
        let b = geometry(120., 100.);
        let c = geometry(260., 60.);
        assert_eq!(
            state.paint("a", Variant::Pill, config, a, now, false),
            (a, false)
        );
        assert_eq!(
            state.paint("b", Variant::Pill, config, b, now, false),
            (a, true)
        );
        let middle_time = now + Duration::from_millis(50);
        let (middle, running) = state.paint("b", Variant::Pill, config, b, middle_time, false);
        assert!(running && middle.left > 0. && middle.left < b.left);
        let velocity = state.history.as_ref().unwrap().left.velocity;
        assert!(velocity > 0.);
        assert_eq!(
            state.paint("c", Variant::Pill, config, c, middle_time, false),
            (middle, true)
        );
        assert_eq!(state.history.as_ref().unwrap().left.velocity, velocity);
        let (settled, running) = state.paint(
            "c",
            Variant::Pill,
            config,
            c,
            now + Duration::from_secs(3),
            false,
        );
        assert_eq!((settled, running), (c, false));
        let resized = geometry(300., 120.);
        assert_eq!(
            state.paint(
                "c",
                Variant::Pill,
                config,
                resized,
                now + Duration::from_secs(3),
                false
            ),
            (c, true)
        );
        assert_eq!(
            state.paint(
                "c",
                Variant::Pill,
                config,
                resized,
                now + Duration::from_secs(6),
                false
            ),
            (resized, false)
        );
    }
    #[test]
    fn text_preview_does_not_commit_and_reduced_motion_clears_both_clocks() {
        let now = Instant::now();
        let config = Config::default();
        let mut state = State::default();
        assert_eq!(
            state.color_progress("a", Variant::Pill, config, now, false),
            1.
        );
        state.paint("a", Variant::Pill, config, geometry(0., 80.), now, false);
        assert_eq!(
            state.color_progress("b", Variant::Pill, config, now, false),
            0.
        );
        assert_eq!(state.history.as_ref().unwrap().selected, "a");
        state.paint("b", Variant::Pill, config, geometry(100., 80.), now, false);
        assert_eq!(
            state.color_progress(
                "b",
                Variant::Pill,
                config,
                now + Duration::from_millis(100),
                false
            ),
            0.5
        );
        assert_eq!(
            state.color_progress(
                "b",
                Variant::Pill,
                config,
                now + Duration::from_millis(200),
                false
            ),
            1.
        );
        assert_eq!(
            state.color_progress("b", Variant::Segmented, config, now, false),
            1.
        );
        let target = geometry(100., 80.);
        assert_eq!(
            state.paint("b", Variant::Pill, config, target, now, true),
            (target, false)
        );
        assert_eq!(
            state.color_progress("b", Variant::Pill, config, now, false),
            1.
        );
        assert!(
            std::mem::size_of::<State>() + std::mem::size_of::<Frame>() + 512
                < Config::OWNER_RESERVED_BYTES
        );
    }
    #[test]
    fn changed_policy_and_out_of_spring_domain_geometry_adopt_exact_values() {
        let now = Instant::now();
        let mut state = State::default();
        let config = Config::default();
        state.paint(
            "a",
            Variant::Underline,
            config,
            geometry(0., 20.),
            now,
            false,
        );
        let large = geometry(2e6, 3e6);
        assert_eq!(
            state.paint("b", Variant::Underline, config, large, now, false),
            (large, false)
        );
        let changed = Config {
            color_duration_ms: 0,
            ..config
        };
        let target = geometry(500., 20.);
        assert_eq!(
            state.paint("c", Variant::Pill, changed, target, now, false),
            (target, false)
        );
    }
}

#[cfg(all(test, feature = "native-image-tests"))]
#[path = "tab_motion_test.rs"]
mod native_tests;
