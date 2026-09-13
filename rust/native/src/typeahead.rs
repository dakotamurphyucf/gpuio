//! Bounded native prefix navigation for non-editable choice controls. Callers
//! supply time so expiration and cycling can be tested without sleeps.
use gpuio_protocol::v1::ChoiceConfig;
use std::time::{Duration, Instant};
use unicode_segmentation::UnicodeSegmentation as _;

const MAX_PREFIX_BYTES: usize = 256;
const RESET_AFTER: Duration = Duration::from_secs(1);

#[derive(Default)]
pub(super) struct Search {
    prefix: String,
    last_input: Option<Instant>,
}
impl Search {
    pub(super) fn clear(&mut self) {
        self.prefix.clear();
        self.last_input = None;
    }
    pub(super) fn advance(
        &mut self,
        config: &ChoiceConfig,
        active: Option<&str>,
        text: &str,
        now: Instant,
    ) -> Option<String> {
        if config.disabled
            || text.is_empty()
            || text.len() > MAX_PREFIX_BYTES
            || text.chars().any(char::is_control)
        {
            return None;
        }
        let text = text.to_lowercase();
        if text.len() > MAX_PREFIX_BYTES {
            return None;
        }
        if self
            .last_input
            .is_none_or(|last| now.saturating_duration_since(last) >= RESET_AFTER)
        {
            self.prefix.clear();
        }
        self.last_input = Some(now);
        let cycling = self.prefix == text && text.graphemes(true).count() == 1;
        let extending = !self.prefix.is_empty() && !cycling;
        if !cycling {
            if self.prefix.len() + text.len() > MAX_PREFIX_BYTES {
                self.prefix.clear();
            }
            self.prefix.push_str(&text);
        }
        let current = config
            .items
            .iter()
            .position(|item| Some(item.id.as_str()) == active);
        let start = current.map_or(0, |index| index + usize::from(!extending));
        let find = |prefix: &str| {
            let count = config.items.len();
            (0..count).find_map(|offset| {
                let item = &config.items[(start + offset) % count];
                (!item.disabled && item.label.to_lowercase().starts_with(prefix))
                    .then(|| item.id.clone())
            })
        };
        if let Some(found) = find(&self.prefix) {
            return Some(found);
        }
        // A fresh printable key can start a new search after an unmatched prefix.
        // Preserve the user's current highlight when even this key has no match.
        self.prefix = text;
        find(&self.prefix)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use gpuio_protocol::v1::ChoiceItem;
    fn options() -> ChoiceConfig {
        ChoiceConfig {
            label: "Modes".into(),
            selected: None,
            disabled: false,
            items: [
                ("deep", "Deep", false),
                ("disabled", "Delta", true),
                ("detail", "Detailed", false),
                ("fast", "Fast", false),
                ("accent", "Éclair", false),
            ]
            .into_iter()
            .map(|(id, label, disabled)| ChoiceItem {
                id: id.into(),
                label: label.into(),
                disabled,
            })
            .collect(),
        }
    }
    #[test]
    fn prefixes_cycle_skip_disabled_and_expire_without_a_timer_task() {
        let mut search = Search::default();
        let config = options();
        let now = Instant::now();
        assert_eq!(
            search.advance(&config, None, "D", now).as_deref(),
            Some("deep")
        );
        assert_eq!(
            search.advance(&config, Some("deep"), "d", now).as_deref(),
            Some("detail")
        );
        assert_eq!(
            search.advance(&config, Some("detail"), "d", now).as_deref(),
            Some("deep")
        );
        assert_eq!(
            search.advance(&config, Some("deep"), "e", now).as_deref(),
            Some("deep")
        );
        assert_eq!(
            search.advance(&config, Some("deep"), "t", now).as_deref(),
            Some("detail")
        );
        assert_eq!(
            search
                .advance(&config, Some("detail"), "f", now + RESET_AFTER)
                .as_deref(),
            Some("fast")
        );
        assert_eq!(
            search
                .advance(&config, Some("fast"), "é", now + RESET_AFTER)
                .as_deref(),
            Some("accent")
        );
    }
    #[test]
    fn failed_searches_and_collection_changes_preserve_bounds() {
        let mut search = Search::default();
        let mut config = options();
        let now = Instant::now();
        assert!(search.advance(&config, None, "x", now).is_none());
        assert_eq!(
            search.advance(&config, None, "d", now).as_deref(),
            Some("deep")
        );
        config.items.reverse();
        assert_eq!(
            search.advance(&config, Some("deep"), "d", now).as_deref(),
            Some("detail")
        );
        for _ in 0..1024 {
            search.advance(&config, None, "e", now);
        }
        assert!(search.prefix.len() <= MAX_PREFIX_BYTES);
        assert!(
            search
                .advance(&config, None, &"a".repeat(257), now)
                .is_none()
        );
        assert!(search.advance(&config, None, "\0", now).is_none());
        config.items.clear();
        assert!(search.advance(&config, None, "d", now).is_none());
        config = options();
        config.disabled = true;
        assert!(search.advance(&config, None, "d", now).is_none());
        search.clear();
        assert!(search.prefix.is_empty());
        assert!(search.last_input.is_none());
    }
}
