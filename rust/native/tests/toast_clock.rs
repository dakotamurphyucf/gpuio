#[path = "../src/toast_clock.rs"]
mod clock;
use clock::{Clock, Plan};
use std::time::{Duration, Instant};
fn seconds(value: u64) -> Duration {
    Duration::from_secs(value)
}
#[test]
fn pauses_preserve_active_time_and_metadata_does_not_renew_it() {
    let now = Instant::now();
    let timeout = Some(seconds(5));
    let mut clock = Clock::new(timeout);
    assert_eq!(clock.advance(now + seconds(10)), Plan::Idle);
    assert_eq!(
        clock.update(timeout, false, now + seconds(10)),
        Plan::After(seconds(5))
    );
    assert_eq!(
        clock.update(timeout, false, now + seconds(11)),
        Plan::After(seconds(4))
    );
    assert_eq!(clock.update(timeout, true, now + seconds(12)), Plan::Idle);
    assert_eq!(clock.advance(now + seconds(40)), Plan::Idle);
    assert_eq!(
        clock.update(timeout, false, now + seconds(50)),
        Plan::After(seconds(3))
    );
    assert_eq!(clock.advance(now + seconds(52)), Plan::After(seconds(1)));
    assert_eq!(clock.advance(now + seconds(53)), Plan::Expired);
    assert!(!clock.is_closed());
    assert!(clock.close());
    assert!(clock.is_closed());
    assert!(!clock.close());
    assert_eq!(
        clock.update(Some(seconds(30)), false, now + seconds(60)),
        Plan::Idle
    );
}
#[test]
fn timeout_changes_reset_the_interval_and_persistent_items_have_no_deadline() {
    let now = Instant::now();
    let mut clock = Clock::new(Some(seconds(5)));
    assert_eq!(
        clock.update(Some(seconds(5)), false, now),
        Plan::After(seconds(5))
    );
    assert_eq!(
        clock.update(Some(seconds(8)), false, now + seconds(3)),
        Plan::After(seconds(8))
    );
    assert_eq!(clock.update(None, false, now + seconds(4)), Plan::Idle);
    assert_eq!(clock.advance(now + seconds(50)), Plan::Idle);
    assert_eq!(
        clock.update(Some(seconds(2)), true, now + seconds(60)),
        Plan::Idle
    );
    assert_eq!(
        clock.update(Some(seconds(2)), false, now + seconds(90)),
        Plan::After(seconds(2))
    );
    assert_eq!(
        clock.update(Some(seconds(2)), true, now + seconds(92)),
        Plan::Expired
    );
}
