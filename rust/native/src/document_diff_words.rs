//! Linear word-boundary emphasis for equal-sized replacement groups.
use crate::document_diff::{Diff, Kind};
use std::ops::Range;

fn words(text: &str) -> Vec<Range<usize>> {
    let mut tokens: Vec<Range<usize>> = Vec::new();
    let mut previous_word = false;
    for (start, scalar) in text.char_indices() {
        let word = scalar.is_alphanumeric() || scalar == '_';
        let end = start + scalar.len_utf8();
        if word && previous_word {
            tokens.last_mut().unwrap().end = end;
        } else {
            tokens.push(start..end);
        }
        previous_word = word;
    }
    tokens
}

fn changed(old: &str, new: &str) -> (Option<Range<usize>>, Option<Range<usize>>) {
    let old_words = words(old);
    let new_words = words(new);
    let prefix = old_words
        .iter()
        .zip(&new_words)
        .take_while(|(a, b)| old[(*a).clone()] == new[(*b).clone()])
        .count();
    let suffix = old_words[prefix..]
        .iter()
        .rev()
        .zip(new_words[prefix..].iter().rev())
        .take_while(|(a, b)| old[(*a).clone()] == new[(*b).clone()])
        .count();
    let span = |tokens: &[Range<usize>]| {
        let middle = &tokens[prefix..tokens.len() - suffix];
        middle
            .first()
            .zip(middle.last())
            .map(|(first, last)| first.start..last.end)
    };
    (span(&old_words), span(&new_words))
}

pub fn annotate(text: &str, diff: &mut Diff, cancelled: impl Fn() -> bool) -> bool {
    for hunk in &diff.hunks {
        let mut cursor = hunk.start + 1;
        while cursor < hunk.end {
            if cancelled() {
                return false;
            }
            let removed = cursor;
            while cursor < hunk.end && diff.lines[cursor].kind == Kind::Removed {
                cursor += 1;
            }
            let added = cursor;
            while cursor < hunk.end && diff.lines[cursor].kind == Kind::Added {
                cursor += 1;
            }
            let count = added - removed;
            // Unequal groups have no reliable line pairing. Leave their normal
            // add/remove colors intact rather than comparing unrelated lines.
            if count > 0 && count == cursor - added {
                for index in 0..count {
                    if cancelled() {
                        return false;
                    }
                    let old = diff.lines[removed + index].content.clone();
                    let new = diff.lines[added + index].content.clone();
                    let (old_changed, new_changed) =
                        changed(&text[old.clone()], &text[new.clone()]);
                    diff.lines[removed + index].changed =
                        old_changed.map(|span| old.start + span.start..old.start + span.end);
                    diff.lines[added + index].changed =
                        new_changed.map(|span| new.start + span.start..new.start + span.end);
                }
            }
            if cursor == removed {
                cursor += 1;
            }
        }
    }
    !cancelled()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn whole_unicode_words_and_punctuation_have_exact_byte_ranges() {
        for (old, new, a, b) in [
            (
                "let greeting = 1",
                "let greetings = 1",
                Some("greeting"),
                Some("greetings"),
            ),
            ("f(α_β)", "f(α_γ)", Some("α_β"), Some("α_γ")),
            ("f(x)", "f(x, y)", None, Some(", y")),
            ("same", "same", None, None),
            ("", "世界", None, Some("世界")),
            ("🦀", "🌍", Some("🦀"), Some("🌍")),
        ] {
            let (actual_old, actual_new) = changed(old, new);
            assert_eq!(actual_old.map(|r| &old[r]), a);
            assert_eq!(actual_new.map(|r| &new[r]), b);
        }
    }
    #[test]
    fn replacement_groups_pair_by_index_within_each_hunk() {
        let text = "--- a/a.ml\n+++ b/a.ml\n@@ -1,2 +1,2 @@\n-let α = 1\r\n-let β = 2\r\n+let α = 10\r\n+let β = 20\r\n@@ -9,2 +9 @@\n-one\n-two\n+three\n";
        let diff = crate::document_diff::parse(text, || false).unwrap();
        let spans: Vec<_> = diff
            .lines
            .iter()
            .filter_map(|line| line.changed.clone().map(|range| &text[range]))
            .collect();
        assert_eq!(spans, ["1", "2", "10", "20"]);
        assert!(
            diff.lines[8..].iter().all(|line| line.changed.is_none()),
            "unequal groups are not paired"
        );
    }
    #[test]
    fn every_streamed_prefix_has_valid_payload_emphasis_or_none() {
        let source = "--- a/a.ml\n+++ b/a.ml\n@@ -1,2 +1,2 @@\n-let α = 1\n-let β = 2\n+let α = 10\n+let β = 20\n";
        for end in source.char_indices().map(|(i, _)| i).chain([source.len()]) {
            let text = &source[..end];
            let diff = crate::document_diff::parse(text, || false).unwrap();
            for line in diff.lines {
                if let Some(span) = line.changed {
                    assert!(span.start >= line.content.start && span.end <= line.content.end);
                    assert!(text.get(span).is_some());
                }
            }
        }
    }
}
