//! Compiled finite stage traversal. The retained owner supplies active elapsed
//! time and owns paint confirmation, observations, pause state and repeat phase.
use crate::motion::{Values, spring::Trajectory};
use gpuio_protocol::{
    animation::{Easing, PROPERTY_COUNT},
    animation_program::{Program, Timing},
};
use std::time::Duration;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Frame {
    pub values: Values,
    pub velocity: Values,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Next {
    Idle,
    /// Remaining active time until a stage boundary; no frame polling needed.
    Wait(Duration),
    Frame,
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Sample {
    pub frame: Frame,
    /// The completed stage prefix. It may advance by multiple stages after a
    /// slow frame; the owner publishes each newly painted stage at most once.
    pub completed: usize,
    pub finished: bool,
    pub next: Next,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    InvalidProgram,
    InvalidInitialState,
}

enum Curve {
    Tween(Easing),
    Spring(Box<[Option<Trajectory>; PROPERTY_COUNT]>),
}
struct Segment {
    from: Frame,
    target: Frame,
    begin: Duration,
    end: Duration,
    curve: Curve,
}
pub struct Timeline {
    initial: Frame,
    final_frame: Frame,
    segments: Vec<Segment>,
    duration: Duration,
}
pub(crate) fn rest(values: Values) -> Frame {
    let mut velocity = Values::empty();
    // Property indices are stable protocol tags; no unchecked enum casts.
    for property in PROPERTIES {
        if values.get(property).is_some() {
            velocity.set(property, 0.);
        }
    }
    Frame { values, velocity }
}
use gpuio_protocol::animation::Property;
const PROPERTIES: [Property; PROPERTY_COUNT] = [
    Property::Width,
    Property::Height,
    Property::Top,
    Property::Right,
    Property::Bottom,
    Property::Left,
    Property::Opacity,
    Property::TopLeftRadius,
    Property::TopRightRadius,
    Property::BottomLeftRadius,
    Property::BottomRightRadius,
];
impl Timeline {
    /// New mounts use the declared initial values. With no initial values a
    /// single-stage first mount is placed immediately. Retargets pass the last
    /// painted frame; missing/new properties use the new declaration.
    pub fn compile(program: &Program, painted: Option<Frame>) -> Result<Self, Error> {
        if !program.is_valid() {
            return Err(Error::InvalidProgram);
        }
        let immediate = program.initial.is_none() && painted.is_none();
        let declared = program
            .initial
            .as_deref()
            .unwrap_or(&program.stages[0].targets);
        let mut from = rest(Values::from_targets(declared));
        if let Some(painted) = painted {
            for target in declared {
                if let Some(value) = painted.values.get(target.property) {
                    let velocity = painted.velocity.get(target.property).unwrap_or(0.);
                    if !target.property.accepts(value)
                        || !velocity.is_finite()
                        || velocity.abs() > 1e12
                    {
                        return Err(Error::InvalidInitialState);
                    }
                    from.values.set(target.property, value);
                    from.velocity.set(target.property, velocity);
                }
            }
        }
        let mut cursor = if immediate {
            Duration::ZERO
        } else {
            Duration::from_millis(program.delay_ms as u64)
        };
        let mut segments = Vec::with_capacity(program.stages.len());
        for (index, stage) in program.stages.iter().enumerate() {
            let target = rest(Values::from_targets(&stage.targets));
            let begin = cursor
                + if immediate {
                    Duration::ZERO
                } else {
                    Duration::from_millis(stage.delay_ms as u64)
                };
            let (curve, duration) = match stage.timing {
                Timing::Tween(duration, easing) => {
                    (Curve::Tween(easing), Duration::from_millis(duration as u64))
                }
                Timing::Spring(parameters) => {
                    let mut paths = Box::new(std::array::from_fn(|_| None));
                    let mut duration = Duration::ZERO;
                    for item in &stage.targets {
                        let path = Trajectory::new(
                            parameters,
                            item.property,
                            from.values.get(item.property).unwrap(),
                            from.velocity.get(item.property).unwrap(),
                            item.value,
                        )
                        .map_err(|_| Error::InvalidInitialState)?;
                        duration = duration.max(path.duration());
                        paths[item.property as usize] = Some(path);
                    }
                    (Curve::Spring(paths), duration)
                }
            };
            // Timed stages deliberately do not inherit physical velocity.
            if matches!(curve, Curve::Tween(_)) {
                from = rest(from.values);
            }
            if index == 0 && immediate {
                from = target;
            }
            let end = begin + if immediate { Duration::ZERO } else { duration };
            segments.push(Segment {
                from,
                target,
                begin,
                end,
                curve,
            });
            from = target;
            cursor = end;
        }
        let initial = segments[0].from;
        Ok(Self {
            initial,
            final_frame: from,
            segments,
            duration: cursor,
        })
    }
    pub fn is_static(&self) -> bool {
        self.segments.iter().all(|segment| {
            segment.from.values == segment.target.values
                && segment.from.velocity == rest(segment.from.values).velocity
        })
    }
    pub fn duration(&self) -> Duration {
        self.duration
    }
    pub fn retained_bytes(&self) -> usize {
        std::mem::size_of::<Self>()
            + self.segments.capacity() * std::mem::size_of::<Segment>()
            + self
                .segments
                .iter()
                .map(|s| {
                    if matches!(s.curve, Curve::Spring(_)) {
                        std::mem::size_of::<[Option<Trajectory>; PROPERTY_COUNT]>()
                    } else {
                        0
                    }
                })
                .sum::<usize>()
    }
    pub fn initial(&self) -> Frame {
        self.initial
    }
    pub fn final_frame(&self) -> Frame {
        self.final_frame
    }
    pub fn sample(&self, elapsed: Duration) -> Sample {
        for (index, segment) in self.segments.iter().enumerate() {
            if elapsed < segment.begin {
                return Sample {
                    frame: segment.from,
                    completed: index,
                    finished: false,
                    next: Next::Wait(segment.begin - elapsed),
                };
            }
            if elapsed < segment.end {
                let mut frame = segment.from;
                let delta = elapsed - segment.begin;
                let next = match &segment.curve {
                    Curve::Tween(easing) => {
                        let phase = easing.sample(
                            delta.as_secs_f64() / (segment.end - segment.begin).as_secs_f64(),
                        );
                        for property in PROPERTIES {
                            if let Some(target) = segment.target.values.get(property) {
                                let from = segment.from.values.get(property).unwrap();
                                frame
                                    .values
                                    .set(property, property.clamp(from + (target - from) * phase));
                            }
                        }
                        if segment.from.values == segment.target.values {
                            Next::Wait(segment.end - elapsed)
                        } else {
                            Next::Frame
                        }
                    }
                    Curve::Spring(paths) => {
                        for property in PROPERTIES {
                            if let Some(path) = &paths[property as usize] {
                                let sample = path.sample(delta);
                                frame.values.set(property, sample.position);
                                frame.velocity.set(property, sample.velocity);
                            }
                        }
                        Next::Frame
                    }
                };
                return Sample {
                    frame,
                    completed: index,
                    finished: false,
                    next,
                };
            }
        }
        Sample {
            frame: self.final_frame,
            completed: self.segments.len(),
            finished: true,
            next: Next::Idle,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use gpuio_protocol::{
        animation::{Repeat, Spring, Target},
        animation_program::{Clock, Stage},
    };
    fn targets(value: f64) -> Vec<Target> {
        vec![Target {
            property: Property::Left,
            value,
        }]
    }
    fn spring() -> Timing {
        Timing::Spring(Spring {
            stiffness: 100.,
            damping: 10.,
            mass: 1.,
            epsilon: 0.001,
            max_duration_ms: 10_000,
        })
    }
    fn stage(value: f64, timing: Timing, delay_ms: i64) -> Stage {
        Stage {
            targets: targets(value),
            timing,
            delay_ms,
        }
    }
    fn program(stages: Vec<Stage>) -> Program {
        Program {
            initial: Some(targets(0.)),
            stages,
            delay_ms: 30,
            repeat: Repeat::Once,
            clock: Clock::Independent,
        }
    }
    fn value(sample: Sample) -> f64 {
        sample.frame.values.get(Property::Left).unwrap()
    }
    #[test]
    fn tween_spring_sequence_accounts_for_both_delays_and_skips_late_frames_in_order() {
        let program = program(vec![
            stage(100., Timing::Tween(100, Easing::Linear), 10),
            stage(200., spring(), 20),
        ]);
        let track = Timeline::compile(&program, None).unwrap();
        assert_eq!(
            track.sample(Duration::ZERO).next,
            Next::Wait(Duration::from_millis(40))
        );
        assert_eq!(value(track.sample(Duration::from_millis(90))), 50.);
        let between = track.sample(Duration::from_millis(140));
        assert_eq!(value(between), 100.);
        assert_eq!(between.completed, 1);
        assert_eq!(between.next, Next::Wait(Duration::from_millis(20)));
        let moving = track.sample(Duration::from_millis(260));
        assert!(value(moving) > 100.);
        assert!(moving.frame.velocity.get(Property::Left).unwrap() > 0.);
        let final_sample = track.sample(Duration::MAX);
        assert_eq!(value(final_sample), 200.);
        assert_eq!(final_sample.completed, 2);
        assert!(final_sample.finished);
        assert_eq!(final_sample.next, Next::Idle);
        assert!(track.duration() > Duration::from_millis(160));
        assert_eq!(
            track.sample(Duration::from_millis(90)).completed,
            0,
            "sampling is immutable"
        );
    }
    #[test]
    fn retarget_uses_painted_spring_velocity_and_new_target_properties() {
        let first = Timeline::compile(&program(vec![stage(100., spring(), 0)]), None).unwrap();
        let painted = first.sample(Duration::from_millis(130)).frame;
        let _discarded = first.sample(Duration::from_millis(500));
        let mut replacement = program(vec![stage(-100., spring(), 0)]);
        replacement.delay_ms = 0;
        let next = Timeline::compile(&replacement, Some(painted)).unwrap();
        assert_eq!(next.sample(Duration::ZERO).frame, painted);
        assert!(
            value(next.sample(Duration::from_millis(1)))
                > painted.values.get(Property::Left).unwrap()
        );
        let mut adding = replacement;
        adding.initial.as_mut().unwrap().push(Target {
            property: Property::Opacity,
            value: 0.,
        });
        adding.stages[0].targets.push(Target {
            property: Property::Opacity,
            value: 1.,
        });
        let new = Timeline::compile(&adding, Some(painted))
            .unwrap()
            .sample(Duration::ZERO);
        assert_eq!(new.frame.values.get(Property::Opacity), Some(0.));
        assert_eq!(new.frame.velocity.get(Property::Opacity), Some(0.));
    }
    #[test]
    fn timed_stages_clear_inherited_velocity_and_constant_intervals_wait_without_redraw() {
        let moving = Timeline::compile(&program(vec![stage(100., spring(), 0)]), None)
            .unwrap()
            .sample(Duration::from_millis(130))
            .frame;
        let position = moving.values.get(Property::Left).unwrap();
        let mut hold = program(vec![stage(position, Timing::Tween(100, Easing::Linear), 0)]);
        hold.delay_ms = 0;
        let track = Timeline::compile(&hold, Some(moving)).unwrap();
        let sample = track.sample(Duration::from_millis(50));
        assert_eq!(sample.frame.velocity.get(Property::Left), Some(0.));
        assert_eq!(sample.next, Next::Wait(Duration::from_millis(50)));
    }
    #[test]
    fn zero_duration_stages_advance_as_one_bounded_prefix() {
        let mut p = program(
            (0..32)
                .map(|i| stage(f64::from(i), Timing::Tween(0, Easing::Linear), 0))
                .collect(),
        );
        p.delay_ms = 0;
        let track = Timeline::compile(&p, None).unwrap();
        let result = track.sample(Duration::ZERO);
        assert_eq!(result.completed, 32);
        assert_eq!(value(result), 31.);
        assert!(result.finished);
        assert_eq!(result.next, Next::Idle);
        assert!(track.retained_bytes() < 100_000);
    }
    #[test]
    fn missing_initial_values_place_only_first_mount_immediately() {
        let mut p = program(vec![stage(100., Timing::Tween(100, Easing::Linear), 10)]);
        p.initial = None;
        let first = Timeline::compile(&p, None).unwrap();
        assert_eq!(first.duration(), Duration::ZERO);
        assert_eq!(value(first.sample(Duration::ZERO)), 100.);
        let painted = rest(Values::from_targets(&targets(50.)));
        let next = Timeline::compile(&p, Some(painted)).unwrap();
        assert_eq!(next.duration(), Duration::from_millis(140));
        assert_eq!(value(next.sample(Duration::ZERO)), 50.);
    }
    #[test]
    fn invalid_programs_and_painted_state_cannot_reach_the_solver() {
        let mut p = program(vec![]);
        assert!(matches!(
            Timeline::compile(&p, None),
            Err(Error::InvalidProgram)
        ));
        p.stages.push(stage(100., spring(), 0));
        let mut painted = rest(Values::from_targets(&targets(50.)));
        painted.velocity.set(Property::Left, f64::NAN);
        assert!(matches!(
            Timeline::compile(&p, Some(painted)),
            Err(Error::InvalidInitialState)
        ));
    }
}
