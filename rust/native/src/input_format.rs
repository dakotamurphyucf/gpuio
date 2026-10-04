//! Rust-only ordinary-input formatting. Exact commands and interactive edits
//! deliberately use different entry points; neither converts through a float.
use gpui_base::input::{BridgeInputFormat, MaskPattern};
use gpuio_protocol::input_format::{Config, MAX_PATTERN_SCALARS, MAX_TEXT_BYTES};
use std::{borrow::Cow, rc::Rc, sync::Arc};

struct Policy {
    config: Arc<Config>,
    mask: Option<MaskPattern>,
}

/// Conservative retained budget includes shared config, the compiled bounded
/// pattern, duplicate pattern source and Rc/Arc bookkeeping. No text is retained.
pub(crate) fn retained_bytes(config: &Config) -> usize {
    config.retained_bytes()
        + std::mem::size_of::<Policy>()
        + 64
        + match config {
            Config::Pattern(source) => source.len() + source.chars().count() * 16,
            Config::Number(_) => 0,
        }
}

pub(crate) fn policy(config: Arc<Config>) -> Rc<dyn BridgeInputFormat> {
    let mask = match config.as_ref() {
        Config::Pattern(source) => Some(MaskPattern::new(source)),
        Config::Number(_) => None,
    };
    Rc::new(Policy { config, mask })
}

struct FilteredPolicy {
    format: Option<Rc<dyn BridgeInputFormat>>,
    validation: Arc<crate::input_validation::Policy>,
}

/// Compose native formatting and filtering without a second editing hook or
/// foreign callback. Existing Base draft recovery, IME and history rules apply.
pub(crate) fn editing_policy(
    format: Option<Arc<Config>>,
    validation: Option<Arc<crate::input_validation::Policy>>,
) -> Option<Rc<dyn BridgeInputFormat>> {
    let format = format.map(policy);
    match validation {
        None => format,
        Some(validation) => Some(Rc::new(FilteredPolicy { format, validation })),
    }
}

impl BridgeInputFormat for FilteredPolicy {
    fn accepts(&self, text: &str) -> bool {
        self.format
            .as_ref()
            .is_none_or(|format| format.accepts(text))
            && self.validation.accepts(text)
    }

    fn normalize<'a>(&self, text: &'a str) -> Cow<'a, str> {
        self.format
            .as_ref()
            .map_or(Cow::Borrowed(text), |format| format.normalize(text))
    }

    fn format(&self, text: &str, caret: usize) -> Option<(String, usize)> {
        if text.len() > MAX_TEXT_BYTES
            || !text.is_char_boundary(caret)
            || text.contains(['\0', '\n', '\r'])
        {
            return None;
        }
        let (text, caret) = match &self.format {
            Some(format) => format.format(text, caret)?,
            None => (text.to_owned(), caret),
        };
        self.validation.accepts(&text).then_some((text, caret))
    }
}

impl BridgeInputFormat for Policy {
    fn accepts(&self, text: &str) -> bool {
        self.config.accepts_formatted(text)
    }
    fn normalize<'a>(&self, text: &'a str) -> Cow<'a, str> {
        self.config.normalize_insert(text)
    }
    fn format(&self, text: &str, caret: usize) -> Option<(String, usize)> {
        if text.len() > MAX_TEXT_BYTES || text.contains(['\0', '\n', '\r']) {
            return None;
        }
        let prefix = text.get(..caret)?;
        if let Some(mask) = &self.mask {
            // Avoid allocating a scalar vector proportional to a large invalid
            // retained draft: a bounded pattern cannot consume more characters.
            if text.chars().take(MAX_PATTERN_SCALARS + 1).count() > MAX_PATTERN_SCALARS
                || !mask.is_valid(text)
            {
                return None;
            }
            let formatted = mask.mask(text).to_string();
            let caret = mask.mask(prefix).len();
            return self.accepts(&formatted).then_some((formatted, caret));
        }
        let Config::Number(number) = self.config.as_ref() else {
            unreachable!()
        };
        let normalized = self.normalize(text);
        let prefix = self.normalize(prefix);
        let (raw, before) = match &number.separator {
            Some(separator) => (
                normalized.replace(separator, ""),
                prefix.replace(separator, ""),
            ),
            None => (normalized.into_owned(), prefix.into_owned()),
        };
        let formatted = self.config.format_raw(&raw).ok()?;
        // Track the raw characters before the caret through grouping of the
        // entire value. Formatting only the prefix gives the wrong position
        // when the first integer group changes width after a middle edit.
        let separator = number.separator.as_ref().and_then(|s| s.chars().next());
        let mut consumed = 0;
        let mut offset = 0;
        for (index, c) in formatted.char_indices() {
            if consumed == before.len() {
                break;
            }
            if Some(c) != separator {
                consumed += c.len_utf8();
            }
            offset = index + c.len_utf8();
        }
        Some((formatted, offset))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use gpuio_protocol::input_format::Number;

    #[test]
    fn decimal_grouping_tracks_caret_through_the_whole_value_and_preserves_precision() {
        for separator in [",", "界"] {
            let p = policy(Arc::new(Config::Number(Number {
                separator: Some(separator.into()),
                fraction_digits: Some(3),
            })));
            for (text, caret, formatted, offset) in [
                ("1234", 0, format!("1{separator}234"), 0),
                ("1234", 2, format!("1{separator}234"), 2 + separator.len()),
                ("1234", 4, format!("1{separator}234"), 4 + separator.len()),
                (
                    "１２３４",
                    6,
                    format!("1{separator}234"),
                    2 + separator.len(),
                ),
                (
                    "-1234.500",
                    9,
                    format!("-1{separator}234.500"),
                    9 + separator.len(),
                ),
            ] {
                assert_eq!(p.format(text, caret), Some((formatted, offset)));
            }
            assert_eq!(p.format("1.2345", 6), None);
            assert!(!p.accepts("1234"));
            assert!(p.accepts(&format!("1{separator}234.500")));
        }
        let p = policy(Arc::new(Config::Number(Number {
            separator: None,
            fraction_digits: Some(2),
        })));
        assert_eq!(p.format("1.234", 5), None);
        assert_eq!(p.format("-.50", 4), Some(("-.50".into(), 4)));
        assert_eq!(
            p.format("00012345678901234567890.00", 24),
            Some(("00012345678901234567890.00".into(), 24))
        );
    }

    #[test]
    fn pattern_interactive_candidates_are_distinct_from_exact_formatted_drafts() {
        let p = policy(Arc::new(Config::Pattern("*–99".into())));
        assert_eq!(p.format("界12", 4), Some(("界–12".into(), 7)));
        assert!(!p.accepts("界12"));
        assert!(p.accepts("界–12"));
        assert_eq!(p.format("界xy", 5), None);
        assert_eq!(
            p.format("a".repeat(MAX_TEXT_BYTES).as_str(), MAX_TEXT_BYTES),
            None
        );
        assert_eq!(p.format("界12", 1), None);
        let p = policy(Arc::new(Config::Pattern("-*".into())));
        assert_eq!(p.format("--", 2), Some(("--".into(), 2)));
        assert_eq!(p.format("-", 1), Some(("-".into(), 1)));
    }
}
