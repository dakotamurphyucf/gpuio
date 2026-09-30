//! Per-window overlay work, separate from ordinary text layout and retained
//! payload admission. Paint order is deterministic; rejection keeps base text.
use std::{cell::RefCell, rc::Rc};

pub const MAX_CANDIDATES: usize = 64;
pub const MAX_FRAME_GLYPHS: usize = 16_384;
pub type Shared = Rc<RefCell<Frame>>;

#[derive(Default)]
pub struct Frame {
    candidates: usize,
    glyphs: usize,
}
impl Frame {
    /// Bound preflight scans as well as emitted overlays. Inactive, transparent
    /// and fully clipped effects do not call this. Emoji/capacity checks do.
    pub fn candidate(&mut self) -> bool {
        if self.candidates == MAX_CANDIDATES {
            return false;
        }
        self.candidates += 1;
        true
    }

    /// Reserve the entire shaped text before any overlay glyph is emitted.
    /// Charging independently of sweep phase avoids capacity-driven flicker.
    pub fn reserve(&mut self, glyphs: usize) -> bool {
        if glyphs > MAX_FRAME_GLYPHS - self.glyphs {
            return false;
        }
        self.glyphs += glyphs;
        true
    }

    #[cfg(any(test, feature = "native-image-tests"))]
    pub fn usage(&self) -> (usize, usize) {
        (self.candidates, self.glyphs)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn whole_text_admission_is_bounded_and_failed_reservations_are_atomic() {
        let mut frame = Frame::default();
        assert!(frame.reserve(MAX_FRAME_GLYPHS - 1));
        assert!(!frame.reserve(usize::MAX));
        assert!(!frame.reserve(2));
        assert_eq!(frame.usage(), (0, MAX_FRAME_GLYPHS - 1));
        assert!(frame.reserve(1));
        assert!(!frame.reserve(1));
        for _ in 0..MAX_CANDIDATES {
            assert!(frame.candidate());
        }
        assert!(!frame.candidate());
        assert_eq!(frame.usage(), (MAX_CANDIDATES, MAX_FRAME_GLYPHS));
        let other_window = Frame::default();
        assert_eq!(other_window.usage(), (0, 0));
        frame = Frame::default();
        assert!(frame.candidate() && frame.reserve(MAX_FRAME_GLYPHS));
    }
}
