//! Event-driven expiry only. Prefix/search state and labels remain in OCaml.
use gpuio_protocol::{
    HandlerId,
    tree_input::{Request, valid_typeahead_text},
};
use std::time::{Duration, Instant};
use unicode_segmentation::UnicodeSegmentation as _;

const RESET_AFTER: Duration = Duration::from_secs(1);

#[derive(Default)]
pub(super) struct Clock {
    handler: Option<HandlerId>,
    last: Option<Instant>,
}
impl Clock {
    pub(super) fn sync(&mut self, handler: Option<HandlerId>) {
        if self.handler != handler {
            self.clear();
            self.handler = handler;
        }
    }
    pub(super) fn clear(&mut self) {
        self.last = None;
    }
    pub(super) fn input(&mut self, text: &str, now: Instant) -> Option<Request> {
        if !valid_typeahead_text(text) {
            return None;
        }
        let reset = self
            .last
            .is_none_or(|last| now.saturating_duration_since(last) >= RESET_AFTER);
        self.last = Some(now);
        Some(Request::Typeahead {
            text: text.into(),
            reset,
            cycle: text.graphemes(true).count() == 1,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn flags(request: Option<Request>) -> (bool, bool) {
        match request.unwrap() {
            Request::Typeahead { reset, cycle, .. } => (reset, cycle),
            _ => unreachable!(),
        }
    }
    #[test]
    fn expiry_reset_and_grapheme_policy_have_no_background_task() {
        let mut clock = Clock::default();
        let now = Instant::now();
        clock.sync(Some(HandlerId::from_parts(0, 1).unwrap()));
        assert_eq!(flags(clock.input("é", now)), (true, true));
        assert_eq!(flags(clock.input("e\u{301}", now)), (false, true));
        assert_eq!(flags(clock.input("👨‍👩‍👧‍👦", now)), (false, true));
        assert_eq!(flags(clock.input("ab", now)), (false, false));
        assert_eq!(flags(clock.input("x", now + RESET_AFTER)), (true, true));
        clock.clear();
        assert!(clock.input("\0", now).is_none());
        assert!(clock.input(&"x".repeat(257), now).is_none());
        assert_eq!(flags(clock.input("x", now)), (true, true));
        clock.sync(Some(HandlerId::from_parts(0, 2).unwrap()));
        assert_eq!(flags(clock.input("x", now)), (true, true));
    }
}
