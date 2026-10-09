use gpuio_native::toast_lifecycle::*;
use gpuio_protocol::v1::ToastDismissal::*;
use std::time::Duration;
fn ms(n: u64) -> Duration {
    Duration::from_millis(n)
}
#[test]
fn entry_starts_at_paint_deadline_survives_frames_and_terminal_fires_once() {
    let mut state = State::default();
    let motion = Some(Motion::default());
    let discarded_preview = state.sample(ms(0), motion, false).unwrap();
    assert!(state.deadline().is_none());
    assert!(!state.allows_timeout());
    let start = state.sample(ms(100), motion, false).unwrap();
    assert_eq!(
        start.visual(),
        Visual {
            opacity: 0.,
            slide: -96.
        }
    );
    assert!(state.painted(&start));
    assert!(!state.painted(&discarded_preview));
    let deadline = state.deadline().unwrap();
    assert_eq!(deadline.at(), ms(500));
    let middle = state.sample(ms(300), motion, false).unwrap();
    assert!(middle.visual().opacity > 0. && middle.visual().opacity < 1.);
    assert!(state.painted(&middle));
    assert!(!state.finish_deadline(&deadline, ms(499)));
    assert!(state.finish_deadline(&deadline, ms(500)));
    assert!(state.allows_timeout());
    assert!(!state.finish_deadline(&deadline, ms(500)));
    let present = state.sample(ms(500), motion, false).unwrap();
    assert!(state.painted(&present));
    assert!(state.dismiss(CloseButton, ms(510), motion, false).unwrap());
    assert!(!state.accepts_input());
    assert!(!state.allows_timeout());
    assert!(!state.dismiss(Timeout, ms(511), motion, false).unwrap());
    assert!(state.take_dismissal().is_none());
    let exit = state.deadline().unwrap();
    assert_eq!(exit.at(), ms(710));
    assert!(!state.finish_deadline(&exit, ms(709)));
    assert!(state.finish_deadline(&exit, ms(710)));
    assert_eq!(state.take_dismissal(), Some(CloseButton));
    assert_eq!(state.take_dismissal(), None);
    assert!(!state.finish_deadline(&exit, ms(800)));
    assert!(state.is_closed());
    assert!(!state.sample(ms(900), motion, false).unwrap().needs_frame());
}
#[test]
fn interrupted_entry_uses_last_paint_and_motion_changes_do_not_restart_phases() {
    let motion = Some(Motion::default());
    let mut state = State::default();
    let start = state.sample(ms(0), motion, true).unwrap();
    state.painted(&start);
    let visible = state.sample(ms(80), motion, true).unwrap();
    state.painted(&visible);
    let unpainted = state.sample(ms(150), motion, true).unwrap();
    assert!(state.dismiss(Escape, ms(90), motion, true).unwrap());
    assert!(!state.painted(&unpainted));
    let exit = state.sample(ms(90), motion, true).unwrap();
    assert_eq!(exit.visual(), visible.visual());
    let changed = Some(Motion {
        enter: ms(5000),
        exit: ms(5000),
        offset: 500.,
    });
    let at_end = state.sample(ms(290), changed, false).unwrap();
    assert_eq!(
        at_end.visual(),
        Visual {
            opacity: 0.,
            slide: 96.
        }
    );
    assert!(!at_end.needs_frame());
    assert!(state.painted(&at_end));
    assert_eq!(state.take_dismissal(), Some(Escape));
    assert!(!state.painted(&at_end));
    assert_eq!(state.take_dismissal(), None);
}
#[test]
fn immediate_hidden_unmount_and_stale_owner_paths_have_no_extra_events() {
    let mut state = State::default();
    let motion = Some(Motion::default());
    assert!(state.dismiss(Overflow, ms(0), motion, false).unwrap());
    assert!(state.is_closed());
    assert_eq!(state.take_dismissal(), Some(Overflow));
    let mut state = State::default();
    state.suspend();
    assert!(state.deadline().is_none());
    let instant = state.sample(ms(0), None, false).unwrap();
    state.painted(&instant);
    assert!(state.allows_timeout());
    let old = state.sample(ms(0), motion, false).unwrap();
    assert_eq!(
        old.visual(),
        Visual {
            opacity: 1.,
            slide: 0.
        }
    );
    state.dismiss(Timeout, ms(1), motion, false).unwrap();
    let deadline = state.deadline().unwrap();
    state.suspend();
    assert!(!state.finish_deadline(&deadline, ms(1000)));
    assert_eq!(state.take_dismissal(), Some(Timeout));
    let mut replacement = State::default();
    assert!(!replacement.painted(&old));
    let start = replacement.sample(ms(2), motion, false).unwrap();
    replacement.painted(&start);
    replacement.suspend();
    assert!(replacement.allows_timeout());
    assert!(replacement.deadline().is_none());
    let visible = replacement.sample(ms(3), motion, false).unwrap();
    replacement.painted(&visible);
    replacement
        .dismiss(CloseButton, ms(4), motion, false)
        .unwrap();
    let wake = replacement.deadline().unwrap();
    replacement.discard();
    assert!(!replacement.finish_deadline(&wake, ms(1000)));
    assert!(replacement.take_dismissal().is_none());
    assert!(replacement.is_closed());
}
#[test]
fn invalid_and_zero_motion_are_checked_without_mutation() {
    let mut state = State::default();
    for bad in [
        Motion {
            offset: f64::NAN,
            ..Default::default()
        },
        Motion {
            offset: -1.,
            ..Default::default()
        },
        Motion {
            exit: ms(60001),
            ..Default::default()
        },
    ] {
        assert!(state.sample(ms(0), Some(bad), false).is_err());
        assert!(state.dismiss(Timeout, ms(0), Some(bad), false).is_err());
        assert!(state.accepts_input());
        assert!(state.deadline().is_none());
    }
    let zero = Some(Motion {
        enter: ms(0),
        exit: ms(0),
        offset: 0.,
    });
    let present = state.sample(ms(0), zero, false).unwrap();
    assert!(!present.needs_frame());
    state.painted(&present);
    state.dismiss(Escape, ms(1), zero, false).unwrap();
    assert_eq!(state.take_dismissal(), Some(Escape));
    assert!(state.deadline().is_none());
}
