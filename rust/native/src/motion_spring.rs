//! Immutable analytic trajectory. Sampling never commits a painted position.
//! The owner constructs a replacement from its last painted position/velocity.
use gpui::{SpringConfig, SpringState};
use gpuio_protocol::animation::{Property, Spring};
use std::time::Duration;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Sample {
    pub position: f64,
    /// Animated units per second. Outward velocity is zero at a clipped bound.
    pub velocity: f64,
    pub finished: bool,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    InvalidParameters,
    InvalidState,
}

pub struct Trajectory {
    solver: SpringConfig,
    initial: SpringState,
    property: Property,
    from: f64,
    velocity: f64,
    target: f64,
    finish: Duration,
}
impl Trajectory {
    pub fn new(
        parameters: Spring,
        property: Property,
        from: f64,
        velocity: f64,
        target: f64,
    ) -> Result<Self, Error> {
        if !parameters.is_valid() {
            return Err(Error::InvalidParameters);
        }
        if !property.accepts(from)
            || !property.accepts(target)
            || !velocity.is_finite()
            || velocity.abs() > 1e12
        {
            return Err(Error::InvalidState);
        }
        let solver = SpringConfig::new(
            parameters.stiffness as f32,
            parameters.damping as f32,
            parameters.mass as f32,
        );
        // Small motion around a large target keeps its precision. GPUI's native
        // rendering uses f32, but the exact declared f64 endpoint is restored.
        let initial = SpringState {
            position: (from - target) as f32,
            velocity: velocity as f32,
        };
        let finish = solver
            .settle_time(initial, 0., parameters.epsilon as f32)
            .min(Duration::from_millis(parameters.max_duration_ms as u64));
        Ok(Self {
            solver,
            initial,
            property,
            from,
            velocity,
            target,
            finish,
        })
    }
    pub fn duration(&self) -> Duration {
        self.finish
    }
    pub fn sample(&self, elapsed: Duration) -> Sample {
        if elapsed >= self.finish {
            return Sample {
                position: self.target,
                velocity: 0.,
                finished: true,
            };
        }
        let (position, velocity) = if elapsed.is_zero() {
            (self.from, self.velocity)
        } else {
            let state = self.solver.step(self.initial, 0., elapsed.as_secs_f32());
            (
                self.target + f64::from(state.position),
                f64::from(state.velocity),
            )
        };
        let clamped = self.property.clamp(position);
        // Projection affects rendering and subsequent retargeting, not the
        // analytic trajectory; an overshoot can return into the legal range.
        let outward = (position > clamped && velocity > 0.)
            || (position < clamped && velocity < 0.)
            || self.property.clamp(clamped + velocity.signum()) == clamped;
        Sample {
            position: clamped,
            velocity: if outward { 0. } else { velocity },
            finished: false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn parameters(damping: f64) -> Spring {
        Spring {
            stiffness: 100.,
            damping,
            mass: 1.,
            epsilon: 0.001,
            max_duration_ms: 10_000,
        }
    }
    fn trajectory(damping: f64) -> Trajectory {
        Trajectory::new(parameters(damping), Property::Left, 0., 0., 100.).unwrap()
    }
    #[test]
    fn every_damping_regime_settles_with_exact_endpoint_and_zero_velocity() {
        for damping in [4., 20., 40.] {
            let path = trajectory(damping);
            assert!(path.duration() > Duration::ZERO);
            assert!(path.duration() < Duration::from_secs(10));
            let before = path.sample(path.duration() - Duration::from_nanos(1));
            assert!((before.position - 100.).abs() < 0.002, "{before:?}");
            assert!(before.velocity.abs() < 0.011, "{before:?}");
            assert_eq!(
                path.sample(path.duration()),
                Sample {
                    position: 100.,
                    velocity: 0.,
                    finished: true
                }
            );
            assert!(path.sample(Duration::MAX).finished);
        }
    }
    #[test]
    fn undamped_motion_overshoots_and_stops_at_its_explicit_deadline() {
        let path = trajectory(0.);
        assert_eq!(path.duration(), Duration::from_secs(10));
        let overshoot = path.sample(Duration::from_millis(300));
        assert!(overshoot.position > 190.);
        assert!(!overshoot.finished);
        assert_eq!(path.sample(Duration::from_secs(10)).position, 100.);
        let mut params = parameters(0.0001);
        params.max_duration_ms = 1;
        let short = Trajectory::new(params, Property::Left, 0., 0., 100.).unwrap();
        assert_eq!(short.duration(), Duration::from_millis(1));
        assert!(short.sample(short.duration()).finished);
    }
    #[test]
    fn previews_do_not_change_retarget_position_or_velocity() {
        let path = trajectory(4.);
        let painted = path.sample(Duration::from_millis(100));
        let _discarded = path.sample(Duration::from_millis(500));
        let next = Trajectory::new(
            parameters(4.),
            Property::Left,
            painted.position,
            painted.velocity,
            -50.,
        )
        .unwrap();
        assert_eq!(next.sample(Duration::ZERO), painted);
        let later = next.sample(Duration::from_millis(1));
        assert!(
            later.position > painted.position,
            "momentum initially continues away from new target"
        );
        assert!(later.velocity > 0.);
    }
    #[test]
    fn elapsed_sampling_is_independent_of_frame_partition_and_absolute_offset() {
        let direct = trajectory(20.);
        let half = direct.sample(Duration::from_millis(100));
        let remainder = Trajectory::new(
            parameters(20.),
            Property::Left,
            half.position,
            half.velocity,
            100.,
        )
        .unwrap();
        let whole = direct.sample(Duration::from_millis(200));
        let split = remainder.sample(Duration::from_millis(100));
        assert!((whole.position - split.position).abs() < 0.0001);
        assert!((whole.velocity - split.velocity).abs() < 0.0001);
        let large =
            Trajectory::new(parameters(20.), Property::Left, 999_999., 0., 999_999.01).unwrap();
        let small = Trajectory::new(parameters(20.), Property::Left, 0., 0., 0.01).unwrap();
        assert!(
            (large.sample(Duration::from_millis(100)).position
                - 999_999.
                - small.sample(Duration::from_millis(100)).position)
                .abs()
                < 1e-9
        );
    }
    #[test]
    fn projection_clips_opacity_and_retargeting_cannot_preserve_outward_velocity() {
        let path = Trajectory::new(parameters(0.), Property::Opacity, 0., 0., 1.).unwrap();
        let outward = path.sample(Duration::from_millis(200));
        assert_eq!(outward.position, 1.);
        assert_eq!(outward.velocity, 0.);
        let returning = path.sample(Duration::from_millis(400));
        assert_eq!(returning.position, 1.);
        assert!(returning.velocity < 0.);
        let restored = path.sample(Duration::from_millis(500));
        assert!(restored.position < 1.);
    }
    #[test]
    fn invalid_parameters_and_state_are_rejected_before_solver_use() {
        let mut bad = parameters(1.);
        bad.mass = f64::NAN;
        assert!(matches!(
            Trajectory::new(bad, Property::Width, 0., 0., 1.),
            Err(Error::InvalidParameters)
        ));
        for (from, velocity, target) in [
            (f64::NAN, 0., 1.),
            (0., f64::INFINITY, 1.),
            (0., 1e13, 1.),
            (0., 0., -1.),
        ] {
            assert!(matches!(
                Trajectory::new(parameters(1.), Property::Width, from, velocity, target),
                Err(Error::InvalidState)
            ));
        }
    }
    #[test]
    fn admitted_parameter_extremes_stay_finite_and_have_bounded_lifetimes() {
        for stiffness in [0.01, 100., 10_000.] {
            for damping in [0., 0.01, 20., 1_000.] {
                for mass in [0.01, 1., 1_000.] {
                    for velocity in [-1e12, 0., 1e12] {
                        let params = Spring {
                            stiffness,
                            damping,
                            mass,
                            ..parameters(damping)
                        };
                        let path = Trajectory::new(
                            params,
                            Property::Left,
                            -1_000_000.,
                            velocity,
                            1_000_000.,
                        )
                        .unwrap();
                        assert!(path.duration() <= Duration::from_secs(10));
                        for millis in [0, 1, 100, 1_000, 9_999, 10_000] {
                            let value = path.sample(Duration::from_millis(millis));
                            assert!(
                                Property::Left.accepts(value.position),
                                "{params:?}: {value:?}"
                            );
                            assert!(value.velocity.is_finite(), "{params:?}: {value:?}");
                        }
                    }
                }
            }
        }
    }
}
