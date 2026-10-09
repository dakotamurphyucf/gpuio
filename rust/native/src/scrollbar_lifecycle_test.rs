use super::*;
fn ms(n: u64) -> Duration {
    Duration::from_millis(n)
}
fn motion() -> Motion {
    Motion {
        idle_ms: 1000,
        enter_ms: 100,
        exit_ms: 200,
        expand_ms: 100,
        entrance: Entrance::SlideAndFade,
        thumb_hover_entrance: Entrance::Fade,
    }
}
fn state(mode: Mode) -> State {
    let mut s = State::new(mode, motion()).unwrap();
    s.set_eligible(true);
    s
}
fn paint(s: &mut State, at: u64, track: f64, thumb: f64) -> Frame {
    let frame = s.sample(ms(at), track, thumb).unwrap();
    assert!(s.painted(&frame));
    frame
}
fn close(a: f64, b: f64) {
    assert!((a - b).abs() < 1e-9, "{a} != {b}");
}

#[test]
fn idle_owners_need_no_frames_and_activity_has_one_finite_deadline() {
    let mut s = state(Mode::Scrolling);
    let hidden = paint(&mut s, 0, 16., 6.);
    assert!(!hidden.needs_frame());
    assert_eq!(hidden.visual().opacity, 0.);
    assert!(s.deadline(ms(0)).is_none());
    assert!(!s.accepts_pointer());
    s.activity(ms(10));
    let deadline = s.deadline(ms(10)).unwrap();
    assert_eq!(deadline.at(), ms(1010));
    let start = paint(&mut s, 10, 16., 6.);
    assert_eq!(start.visual().opacity, 0.);
    assert_eq!(start.visual().slide, 1.);
    assert!(start.needs_frame());
    let mid = paint(&mut s, 60, 16., 6.);
    close(mid.visual().opacity, 0.5);
    close(mid.visual().slide, 0.125);
    assert!(s.accepts_pointer());
    assert!(s.deadline(ms(60)).unwrap().same_schedule(&deadline));
    let end = paint(&mut s, 110, 16., 6.);
    assert_eq!(end.visual().opacity, 1.);
    assert_eq!(end.visual().slide, 0.);
    assert!(!end.needs_frame());
    assert!(!s.wake(&deadline, ms(1009)));
    assert!(s.wake(&deadline, ms(1010)));
    assert!(!s.wake(&deadline, ms(1010)));
    assert!(s.deadline(ms(1010)).is_none());
    assert!(paint(&mut s, 1010, 16., 6.).needs_frame());
    let mid = paint(&mut s, 1110, 16., 6.);
    close(mid.visual().opacity, 0.875);
    close(mid.visual().slide, 0.125);
    let end = paint(&mut s, 1210, 16., 6.);
    assert_eq!(end.visual().opacity, 0.);
    assert_eq!(end.visual().slide, 1.);
    assert!(!end.needs_frame());
    assert!(!s.accepts_pointer());
}

#[test]
fn speculative_layout_does_not_start_motion_and_stale_samples_cannot_commit() {
    let mut s = state(Mode::Hover);
    s.set_interaction(Interaction::TrackHover, false, ms(0));
    let early = s.sample(ms(0), 16., 6.).unwrap();
    let late = s.sample(ms(500), 16., 6.).unwrap();
    assert_eq!(early.visual(), late.visual());
    assert!(s.painted(&late));
    assert!(!s.painted(&early));
    let mid = paint(&mut s, 550, 16., 6.);
    close(mid.visual().opacity, 0.5);
    close(mid.visual().slide, 0.125);
    let replacement = state(Mode::Hover);
    let foreign = replacement.sample(ms(600), 16., 6.).unwrap();
    assert!(!s.painted(&foreign));
    let stale = s.sample(ms(600), 16., 6.).unwrap();
    s.set_reduced(true);
    assert!(!s.painted(&stale));
    assert_eq!(paint(&mut s, 600, 16., 6.).visual().opacity, 1.);
}

#[test]
fn reversal_starts_at_last_paint_not_speculative_elapsed_position() {
    let mut s = state(Mode::Hover);
    let mut m = motion();
    m.idle_ms = 0;
    s.set_policy(Mode::Hover, m).unwrap();
    s.set_interaction(Interaction::TrackHover, false, ms(0));
    paint(&mut s, 0, 16., 6.);
    let mid = paint(&mut s, 50, 16., 6.);
    // A late preview must not become the source for the interrupted exit.
    let preview = s.sample(ms(90), 16., 6.).unwrap();
    s.set_interaction(Interaction::Rest, false, ms(90));
    assert!(!s.painted(&preview));
    let reverse = paint(&mut s, 90, 16., 6.);
    assert_eq!(reverse.visual(), mid.visual());
    // Distance max(opacity .5, slide .875) scales 200ms exit to175ms.
    let end = paint(&mut s, 265, 16., 6.);
    assert_eq!(end.visual().opacity, 0.);
    assert_eq!(end.visual().slide, 1.);
    assert!(!end.needs_frame());
}

#[test]
fn hover_mode_entrances_and_scrolling_hover_obey_actual_visibility() {
    let mut scrolling = state(Mode::Scrolling);
    assert!(!scrolling.set_interaction(Interaction::TrackHover, false, ms(0)));
    assert!(!scrolling.set_interaction(Interaction::Pressed, false, ms(0)));
    assert!(!paint(&mut scrolling, 0, 16., 6.).needs_frame());
    scrolling.activity(ms(0));
    paint(&mut scrolling, 0, 16., 6.);
    paint(&mut scrolling, 100, 16., 6.);
    scrolling.set_interaction(Interaction::TrackHover, false, ms(100));
    assert!(scrolling.deadline(ms(100)).is_none());
    assert_eq!(paint(&mut scrolling, 5000, 16., 6.).visual().opacity, 1.);
    scrolling.set_interaction(Interaction::Rest, false, ms(5000));
    assert_eq!(scrolling.deadline(ms(5000)).unwrap().at(), ms(6000));
    let mut hover = state(Mode::Hover);
    hover.set_interaction(Interaction::ThumbHover, false, ms(0));
    let first = paint(&mut hover, 0, 16., 8.);
    assert_eq!(first.visual().slide, 0.); // Alternate Fade entrance.
    let half = paint(&mut hover, 50, 16., 8.);
    close(half.visual().opacity, 0.5);
    assert_eq!(half.visual().slide, 0.);
    hover.set_interaction(Interaction::TrackHover, false, ms(50));
    assert_eq!(paint(&mut hover, 100, 16., 6.).visual().opacity, 1.);
    assert_eq!(paint(&mut hover, 150, 16., 6.).visual().slide, 0.);
}

#[test]
fn focus_capture_and_reduced_motion_preserve_hold_without_idle_frames() {
    let mut s = state(Mode::Scrolling);
    s.set_reduced(true);
    s.set_interaction(Interaction::Rest, true, ms(0));
    let shown = paint(&mut s, 0, 16., 6.);
    assert_eq!(shown.visual().opacity, 1.);
    assert!(!shown.needs_frame());
    assert!(s.deadline(ms(0)).is_none());
    s.set_interaction(Interaction::Pressed, false, ms(10));
    let pressed = paint(&mut s, 10, 24., 10.);
    assert_eq!(pressed.visual().track_width, 24.);
    assert_eq!(pressed.visual().thumb_width, 10.);
    assert!(!pressed.needs_frame());
    assert!(s.deadline(ms(10)).is_none());
    s.set_interaction(Interaction::Rest, false, ms(20));
    let deadline = s.deadline(ms(20)).unwrap();
    assert_eq!(deadline.at(), ms(1020));
    assert!(!paint(&mut s, 1000, 16., 6.).needs_frame());
    assert!(s.wake(&deadline, ms(1020)));
    let hidden = paint(&mut s, 1020, 16., 6.);
    assert_eq!(hidden.visual().opacity, 0.);
    assert!(!hidden.needs_frame());
}

#[test]
fn always_mode_skips_visibility_motion_but_width_changes_remain_finite() {
    let mut s = state(Mode::Always);
    assert!(!s.set_interaction(Interaction::Pressed, false, ms(0)));
    let first = paint(&mut s, 0, 16., 6.);
    assert_eq!(first.visual().opacity, 1.);
    assert!(!first.needs_frame());
    assert!(s.deadline(ms(0)).is_none());
    s.set_interaction(Interaction::Pressed, false, ms(10));
    assert!(paint(&mut s, 10, 24., 10.).needs_frame());
    let mid = paint(&mut s, 60, 24., 10.);
    close(mid.visual().track_width, 23.);
    close(mid.visual().thumb_width, 9.5);
    // Interrupt from the painted widths, not their speculative value at90ms.
    let reversed = paint(&mut s, 90, 16., 6.);
    assert_eq!(reversed.visual(), mid.visual());
    let end = paint(&mut s, 190, 16., 6.);
    assert_eq!(end.visual().thumb_width, 6.);
    assert!(!end.needs_frame());
    s.activity(ms(200));
    assert!(s.deadline(ms(200)).is_none());
}

#[test]
fn new_activity_policy_and_owner_retirement_reject_stale_deadlines() {
    let mut s = state(Mode::Scrolling);
    s.activity(ms(0));
    let old = s.deadline(ms(0)).unwrap();
    s.activity(ms(500));
    let current = s.deadline(ms(500)).unwrap();
    assert!(!s.wake(&old, ms(1000)));
    assert_eq!(current.at(), ms(1500));
    let mut policy = motion();
    policy.idle_ms = 2000;
    s.set_policy(Mode::Scrolling, policy).unwrap();
    assert!(!s.wake(&current, ms(1500)));
    let current = s.deadline(ms(500)).unwrap();
    assert_eq!(current.at(), ms(2500));
    paint(&mut s, 500, 16., 6.);
    paint(&mut s, 600, 16., 6.);
    s.set_interaction(Interaction::Pressed, false, ms(600));
    let stale = s.sample(ms(650), 16., 8.).unwrap();
    s.set_eligible(false);
    assert_eq!(s.interaction(), Interaction::Rest);
    assert!(!s.accepts_pointer());
    assert!(!s.painted(&stale));
    assert!(!s.wake(&current, ms(2500)));
    assert!(!s.activity(ms(3000)));
    s.set_eligible(true);
    assert_eq!(paint(&mut s, 3000, 16., 6.).visual().opacity, 0.);
    s.activity(ms(3010));
    let deadline = s.deadline(ms(3010)).unwrap();
    assert!(s.close());
    assert!(!s.close());
    assert!(!s.set_eligible(true));
    assert!(!s.wake(&deadline, ms(9999)));
    assert!(!s.sample(ms(9999), 16., 6.).unwrap().needs_frame());
    let mut other = state(Mode::Scrolling);
    other.activity(ms(3010));
    assert!(!other.wake(&deadline, ms(9999)));
}

#[test]
fn boundary_policies_invalid_values_and_backward_time_are_deterministic() {
    let mut m = motion();
    m.enter_ms = -1;
    assert!(State::new(Mode::Always, m).is_err());
    let mut s = state(Mode::Scrolling);
    s.activity(ms(100));
    let deadline = s.deadline(ms(100)).unwrap();
    assert_eq!(s.set_policy(Mode::Always, m), Err(Error::InvalidMotion));
    assert!(s.deadline(ms(100)).unwrap().same_schedule(&deadline));
    for width in [f64::NAN, f64::INFINITY, -1., 16385.] {
        assert!(s.sample(ms(100), width, 6.).is_err());
        assert!(s.sample(ms(100), 16., width).is_err());
    }
    paint(&mut s, 100, 0., 16384.);
    let mid = paint(&mut s, 150, 0., 16384.);
    assert_eq!(paint(&mut s, 140, 0., 16384.).visual(), mid.visual());
    let mut m = motion();
    m.enter_ms = 0;
    m.exit_ms = 0;
    m.expand_ms = 0;
    m.idle_ms = 0;
    s.set_policy(Mode::Scrolling, m).unwrap();
    let hidden = paint(&mut s, 200, 16., 6.);
    assert_eq!(hidden.visual().opacity, 0.);
    assert!(!hidden.needs_frame());
    assert!(s.deadline(ms(200)).is_none());
    s.set_interaction(Interaction::Rest, true, ms(200));
    let shown = paint(&mut s, 200, 16., 6.);
    assert_eq!(shown.visual().opacity, 1.);
    assert!(!shown.needs_frame());
}
