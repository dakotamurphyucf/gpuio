//! Shared bounded binary-float rules; these types own no native interaction state.
use binprot::macros::BinProtWrite;

#[derive(Clone, Copy, Debug, PartialEq, Eq, BinProtWrite)]
pub enum Direction {
    Increase,
    Decrease,
}

#[derive(Clone, Copy, Debug, PartialEq, BinProtWrite)]
pub struct Domain {
    min: f64,
    max: f64,
    step: f64,
}

fn zero(value: f64) -> f64 {
    if value == 0. { 0. } else { value }
}

impl Domain {
    pub fn new(min: f64, max: f64, step: f64) -> Option<Self> {
        let span = max - min;
        (min.is_finite()
            && max.is_finite()
            && step.is_finite()
            && min <= max
            && step > 0.
            && (min == max
                || (span.is_finite()
                    && span / step <= 1_099_511_627_776.
                    && step >= (8. * f64::EPSILON) * min.abs().max(max.abs()))))
        .then_some(Self { min, max, step })
    }
    pub fn min(self) -> f64 {
        self.min
    }
    pub fn max(self) -> f64 {
        self.max
    }
    pub fn step(self) -> f64 {
        self.step
    }
    pub fn contains(self, value: f64) -> bool {
        value.is_finite() && value >= self.min && value <= self.max
    }
    fn point(self, index: f64) -> f64 {
        zero(if index <= 0. {
            self.min
        } else if index >= (self.max - self.min) / self.step {
            self.max
        } else {
            (self.min + index * self.step).clamp(self.min, self.max)
        })
    }
    pub fn normalize(self, value: f64) -> Option<f64> {
        if !value.is_finite() {
            return None;
        }
        if value <= self.min {
            return Some(zero(self.min));
        }
        if value >= self.max {
            return Some(zero(self.max));
        }
        let index = ((value - self.min) / self.step).floor();
        let (lower, upper) = (self.point(index), self.point(index + 1.));
        Some(if (value - lower).abs() < (upper - value).abs() {
            lower
        } else {
            upper
        })
    }
    pub fn advance(self, value: f64, direction: Direction) -> Option<f64> {
        let value = self.normalize(value)?;
        let index = (value - self.min) / self.step;
        let (first, delta, endpoint) = match direction {
            Direction::Increase => (index.floor(), 1., self.max),
            Direction::Decrease => (index.ceil(), -1., self.min),
        };
        // Bounded adjustment for quotient/reconstruction rounding. The validated
        // precision guard keeps adjacent regular points distinct.
        for offset in 0..4 {
            let next = self.point(first + f64::from(offset) * delta);
            if match direction {
                Direction::Increase => next > value,
                Direction::Decrease => next < value,
            } {
                return Some(next);
            }
        }
        Some(zero(endpoint))
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DraftError {
    Syntax,
    NonFinite,
    TooLong,
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Draft {
    Empty,
    Incomplete,
    Invalid(DraftError),
    Valid(f64),
    OutOfRange(f64),
}

impl Draft {
    pub fn parse(domain: Domain, source: &str) -> Self {
        if source.len() > 4096 {
            return Self::Invalid(DraftError::TooLong);
        }
        let text = source
            .trim_matches(|c: char| matches!(c, ' ' | '\t' | '\n' | '\r' | '\u{b}' | '\u{c}'));
        let bytes = text.as_bytes();
        if bytes.is_empty() {
            return Self::Empty;
        }
        fn sign(bytes: &[u8], index: &mut usize) {
            if bytes.get(*index).is_some_and(|c| *c == b'+' || *c == b'-') {
                *index += 1;
            }
        }
        fn digits(bytes: &[u8], index: &mut usize) -> usize {
            let start = *index;
            while bytes.get(*index).is_some_and(u8::is_ascii_digit) {
                *index += 1;
            }
            *index - start
        }
        let mut index = 0;
        sign(bytes, &mut index);
        let before = digits(bytes, &mut index);
        let after = if bytes.get(index) == Some(&b'.') {
            index += 1;
            digits(bytes, &mut index)
        } else {
            0
        };
        if before + after == 0 {
            return if index == bytes.len() {
                Self::Incomplete
            } else {
                Self::Invalid(DraftError::Syntax)
            };
        }
        let missing_exponent = if bytes.get(index).is_some_and(|c| *c == b'e' || *c == b'E') {
            index += 1;
            sign(bytes, &mut index);
            digits(bytes, &mut index) == 0
        } else {
            false
        };
        if index != bytes.len() {
            return Self::Invalid(DraftError::Syntax);
        }
        if missing_exponent {
            return Self::Incomplete;
        }
        match text.parse::<f64>() {
            Ok(value) if !value.is_finite() => Self::Invalid(DraftError::NonFinite),
            Ok(value) if domain.contains(value) => Self::Valid(zero(value)),
            Ok(value) => Self::OutOfRange(zero(value)),
            Err(_) => Self::Invalid(DraftError::Syntax),
        }
    }
}
