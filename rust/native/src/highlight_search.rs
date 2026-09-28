//! Streaming literal matching for one logical highlight group. Adjacent chunks
//! share matches; callers start a new search at each structural group boundary.
//! No flattened/lowercased source or source-sized origin map is allocated.
use gpuio_protocol::highlight::Query;
use std::{cell::Cell, collections::VecDeque, ops::Range};

pub const MAX_WORK_BYTES: usize = 64 * 1024 * 1024;
pub const MAX_STORED_MATCHES: usize = 16384;
pub const MAX_GROUP_VISITS: usize = 65536;
pub const MAX_CHUNK_VISITS: usize = 262144;

/// One worker request shares these limits across every group and spec. Work
/// counts original UTF-8 bytes inspected, including repeated scans for specs.
#[derive(Debug)]
pub struct Budget {
    remaining_bytes: usize,
    remaining_matches: usize,
    remaining_groups: usize,
    remaining_chunks: usize,
}
impl Default for Budget {
    fn default() -> Self {
        Self {
            remaining_bytes: MAX_WORK_BYTES,
            remaining_matches: MAX_STORED_MATCHES,
            remaining_groups: MAX_GROUP_VISITS,
            remaining_chunks: MAX_CHUNK_VISITS,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    InvalidQuery,
    Cancelled,
    WorkLimit,
}

#[derive(Debug, Default, PartialEq, Eq)]
pub struct Matches {
    /// Retained prefix of matches. `total` may exceed this length.
    pub ranges: Vec<Range<usize>>,
    pub total: usize,
}
impl Matches {
    pub fn has_all_ranges(&self) -> bool {
        self.total == self.ranges.len()
    }
}

#[derive(Clone, Copy)]
struct Origin {
    start_byte: usize,
    left_word: bool,
}
fn word(c: char) -> bool {
    c.is_alphanumeric() || c == '_'
}

/// Compile once per spec, then reuse across groups. Cosmetic configuration
/// changes do not require either recompilation or a new source scan.
pub struct Matcher {
    needle: Vec<u8>,
    prefix: Vec<usize>,
    case_sensitive: bool,
    whole_word: bool,
}
impl Matcher {
    pub fn new(query: &Query) -> Result<Self, Error> {
        if !query.is_valid() {
            return Err(Error::InvalidQuery);
        }
        let needle: Vec<u8> = if query.case_sensitive {
            query.text.bytes().collect()
        } else {
            query
                .text
                .chars()
                .flat_map(char::to_lowercase)
                .collect::<String>()
                .into_bytes()
        };
        let mut prefix = vec![0; needle.len()];
        for index in 1..needle.len() {
            let mut matched = prefix[index - 1];
            while matched > 0 && needle[index] != needle[matched] {
                matched = prefix[matched - 1];
            }
            if needle[index] == needle[matched] {
                matched += 1;
            }
            prefix[index] = matched;
        }
        Ok(Self {
            needle,
            prefix,
            case_sensitive: query.case_sensitive,
            whole_word: query.whole_word,
        })
    }

    /// Input chunks must form a single logical group and contain complete UTF-8
    /// scalars (as Rust strings always do). Newline/CR also break query matching.
    /// Returned offsets index the concatenated original bytes, never folded bytes.
    /// On cancellation/work exhaustion the caller must discard the whole request's
    /// partial results. Storage exhaustion keeps exact counts but marks truncation.
    pub fn find<'a>(
        &self,
        chunks: impl IntoIterator<Item = &'a str>,
        budget: &mut Budget,
        cancelled: impl Fn() -> bool,
    ) -> Result<Matches, Error> {
        if cancelled() {
            return Err(Error::Cancelled);
        }
        budget.remaining_groups = budget
            .remaining_groups
            .checked_sub(1)
            .ok_or(Error::WorkLimit)?;
        let needle = &self.needle;
        let prefix = &self.prefix;
        let remaining_chunks = &mut budget.remaining_chunks;
        let chunk_error = Cell::new(None);
        // Empty fragments consume no byte budget, but must still be cancellable
        // and finite. Do not hide an unbounded empty-chunk loop inside flat_map.
        let chunks = chunks.into_iter().take_while(|_| {
            if cancelled() {
                chunk_error.set(Some(Error::Cancelled));
                false
            } else if let Some(remaining) = remaining_chunks.checked_sub(1) {
                *remaining_chunks = remaining;
                true
            } else {
                chunk_error.set(Some(Error::WorkLimit));
                false
            }
        });
        let mut chars = chunks.flat_map(str::chars).peekable();
        let mut origins = VecDeque::with_capacity(needle.len());
        let mut out = Matches::default();
        let mut matched = 0;
        let mut offset = 0;
        let mut previous_word = false;
        let mut last_match_end = 0;
        let mut until_cancel_check = 0;
        while let Some(c) = chars.next() {
            if until_cancel_check == 0 {
                if cancelled() {
                    return Err(Error::Cancelled);
                }
                until_cancel_check = 1024;
            }
            until_cancel_check -= 1;
            budget.remaining_bytes = budget
                .remaining_bytes
                .checked_sub(c.len_utf8())
                .ok_or(Error::WorkLimit)?;
            let origin = Origin {
                start_byte: offset,
                left_word: previous_word,
            };
            offset += c.len_utf8();
            previous_word = word(c);
            if c == '\n' || c == '\r' {
                origins.clear();
                matched = 0;
                continue;
            }
            let right_word = chars.peek().is_some_and(|c| word(*c));
            // Both arms produce an iterator of scalars without a per-source-scalar
            // heap allocation. The sensitive case must not lowercase even once.
            let mut lower = c.to_lowercase();
            let mut sensitive = Some(c);
            loop {
                let next = if self.case_sensitive {
                    sensitive.take()
                } else {
                    lower.next()
                };
                let Some(folded) = next else { break };
                let mut bytes = [0; 4];
                for byte in folded.encode_utf8(&mut bytes).bytes() {
                    if origins.len() == needle.len() {
                        origins.pop_front();
                    }
                    origins.push_back(origin);
                    while matched > 0 && byte != needle[matched] {
                        matched = prefix[matched - 1];
                    }
                    if byte == needle[matched] {
                        matched += 1;
                    }
                    if matched == needle.len() {
                        let first = origins.front().expect("nonempty validated query");
                        if first.start_byte >= last_match_end
                            && (!self.whole_word || (!first.left_word && !right_word))
                        {
                            if budget.remaining_matches > 0 {
                                out.ranges.push(first.start_byte..offset);
                                budget.remaining_matches -= 1;
                            }
                            out.total += 1;
                            last_match_end = offset;
                            matched = 0;
                        } else {
                            // An invalid boundary is not a match: retain overlaps
                            // that may have a valid start (e.g. '..' in 'x... ').
                            matched = prefix[matched - 1];
                        }
                    }
                }
            }
        }
        if let Some(error) = chunk_error.get() {
            Err(error)
        } else if cancelled() {
            Err(Error::Cancelled)
        } else {
            Ok(out)
        }
    }
}

#[cfg(test)]
// Fixtures compare vectors of byte intervals, including a single interval.
#[allow(clippy::single_range_in_vec_init)]
mod tests {
    use super::*;
    use std::cell::Cell;

    fn find<'a>(
        query: &Query,
        chunks: impl IntoIterator<Item = &'a str>,
        budget: &mut Budget,
        cancelled: impl Fn() -> bool,
    ) -> Result<Matches, Error> {
        Matcher::new(query)?.find(chunks, budget, cancelled)
    }

    fn query(text: &str, case_sensitive: bool, whole_word: bool) -> Query {
        Query {
            text: text.into(),
            case_sensitive,
            whole_word,
        }
    }
    fn ranges(
        text: &str,
        needle: &str,
        case_sensitive: bool,
        whole_word: bool,
    ) -> Vec<Range<usize>> {
        find(
            &query(needle, case_sensitive, whole_word),
            [text],
            &mut Budget::default(),
            || false,
        )
        .unwrap()
        .ranges
    }

    #[test]
    fn lowercase_expansion_original_offsets_and_literal_policy() {
        assert_eq!(ranges("İ i I", "i", false, false), [0..2, 3..4, 5..6]);
        assert_eq!(ranges("İ", "i\u{307}", false, false), [0..2]);
        assert_eq!(ranges("İ", "\u{307}", false, false), [0..2]);
        assert_eq!(ranges("CAFÉ café", "café", false, false), [0..5, 6..11]);
        assert_eq!(ranges("CAFÉ café", "café", true, false), [6..11]);
        assert!(ranges("ﬀ", "ff", false, false).is_empty());
        assert!(ranges("e\u{301}", "é", false, false).is_empty());
        assert_eq!(ranges("ΟΣ", "οσ", false, false), [0..4]); // scalar, not contextual lowercasing
        assert_eq!(ranges("Σςσ", "σ", false, false), [0..2, 4..6]);
        assert_eq!(ranges("aaa", "aa", true, false), [0..2]);
        assert!(ranges("a\nb", "a\nb", true, false).is_empty());
        assert!(ranges("a\rb", "a\rb", false, false).is_empty());
    }

    #[test]
    fn whole_word_unicode_underscore_and_rejected_overlap() {
        assert_eq!(
            ranges("cat cat_ λcat catλ (cat)", "cat", true, true),
            [0..3, 22..25]
        );
        assert_eq!(ranges("x... ", "..", true, true), [2..4]);
        assert_eq!(ranges("e\u{301}", "e", false, true), [0..1]);
        assert_eq!(ranges("İ ", "i", false, true), [0..2]);
        assert!(ranges("_İ", "i", false, true).is_empty());
    }

    #[test]
    fn every_scalar_chunk_boundary_preserves_results() {
        let text = "Hello café! İ i Σσ aaaa 👨‍👩‍👧‍👦\nHello world";
        let chunks: Vec<_> = text
            .char_indices()
            .map(|(i, c)| &text[i..i + c.len_utf8()])
            .collect();
        for needle in ["Hello café", "i", "σ", "aa", "👨‍👩‍👧‍👦", "world", "absent"]
        {
            for sensitive in [false, true] {
                for whole_word in [false, true] {
                    let q = query(needle, sensitive, whole_word);
                    let expected = find(&q, [text], &mut Budget::default(), || false).unwrap();
                    let actual =
                        find(&q, chunks.iter().copied(), &mut Budget::default(), || false).unwrap();
                    assert_eq!(actual, expected, "{q:?}");
                }
            }
        }
    }

    #[test]
    fn shared_work_storage_limits_and_cancellation_are_explicit() {
        let q = query("a", false, false);
        let mut budget = Budget {
            remaining_bytes: 8,
            remaining_matches: 2,
            remaining_groups: MAX_GROUP_VISITS,
            remaining_chunks: MAX_CHUNK_VISITS,
        };
        let first = find(&q, ["aaa"], &mut budget, || false).unwrap();
        assert_eq!(
            first,
            Matches {
                ranges: vec![0..1, 1..2],
                total: 3
            }
        );
        assert!(!first.has_all_ranges());
        let second = find(&q, ["aa"], &mut budget, || false).unwrap();
        assert_eq!(second.total, 2);
        assert!(second.ranges.is_empty());
        assert_eq!(
            find(&q, ["aaaa"], &mut budget, || false),
            Err(Error::WorkLimit)
        );
        assert_eq!(
            find(&q, ["a"], &mut Budget::default(), || true),
            Err(Error::Cancelled)
        );
        let calls = Cell::new(0);
        assert_eq!(
            find(
                &q,
                ["a".repeat(5000).as_str()],
                &mut Budget::default(),
                || {
                    calls.set(calls.get() + 1);
                    calls.get() > 3
                }
            ),
            Err(Error::Cancelled)
        );
        assert_eq!(
            find(
                &query("", false, false),
                ["a"],
                &mut Budget::default(),
                || false
            ),
            Err(Error::InvalidQuery)
        );
    }

    /// Deliberately slow, allocating oracle: inspect every possible folded
    /// scalar start, translate to source, then apply boundaries/nonoverlap.
    fn reference(text: &str, query: &Query) -> Vec<Range<usize>> {
        let transform = |text: &str| -> String {
            if query.case_sensitive {
                text.to_owned()
            } else {
                text.chars().flat_map(char::to_lowercase).collect()
            }
        };
        let needle = transform(&query.text);
        let mut folded = String::new();
        let mut origins = Vec::new();
        for (offset, c) in text.char_indices() {
            let transformed = transform(&c.to_string());
            folded.push_str(&transformed);
            origins.extend(std::iter::repeat_n(
                offset..offset + c.len_utf8(),
                transformed.len(),
            ));
        }
        let mut result: Vec<Range<usize>> = Vec::new();
        for (start, _) in folded.char_indices() {
            if !folded[start..].starts_with(&needle) {
                continue;
            }
            let range = origins[start].start..origins[start + needle.len() - 1].end;
            if text[range.clone()].contains(['\r', '\n'])
                || result.last().is_some_and(|r| r.end > range.start)
            {
                continue;
            }
            if query.whole_word
                && (text[..range.start].chars().next_back().is_some_and(word)
                    || text[range.end..].chars().next().is_some_and(word))
            {
                continue;
            }
            result.push(range);
        }
        result
    }

    #[test]
    fn exhaustive_short_inputs_match_independent_reference() {
        let alphabet = ['a', '.', 'İ', 'Σ', '_', ' ', '\n'];
        let mut sources = vec![String::new()];
        let mut level = vec![String::new()];
        for _ in 0..4 {
            level = level
                .iter()
                .flat_map(|s| alphabet.map(|c| format!("{s}{c}")))
                .collect();
            sources.extend(level.iter().cloned());
        }
        for text in sources {
            for needle in [
                "a", "aa", ".", "..", "a.a", "i", "i\u{307}", "\u{307}", "σ", "_", " ",
            ] {
                for case_sensitive in [false, true] {
                    for whole_word in [false, true] {
                        let q = query(needle, case_sensitive, whole_word);
                        let actual = find(&q, [&*text], &mut Budget::default(), || false).unwrap();
                        assert_eq!(
                            actual.ranges,
                            reference(&text, &q),
                            "source{text:?}, query{q:?}"
                        );
                        assert_eq!(actual.total, actual.ranges.len());
                    }
                }
            }
        }
    }

    #[test]
    fn large_stream_keeps_exact_count_after_shared_storage_limit() {
        let mut budget = Budget::default();
        let matches = find(
            &query("a", true, true),
            std::iter::repeat_n("a ", 100000),
            &mut budget,
            || false,
        )
        .unwrap();
        assert_eq!(matches.total, 100000);
        assert_eq!(matches.ranges.len(), MAX_STORED_MATCHES);
        assert_eq!(matches.ranges.last(), Some(&(32766..32767)));
        assert!(!matches.has_all_ranges());
        let later = find(&query("a", true, true), ["a"], &mut budget, || false).unwrap();
        assert_eq!(later.total, 1);
        assert!(later.ranges.is_empty());
    }

    #[test]
    fn compiled_matcher_reuse_and_empty_group_visits_are_bounded() {
        let matcher = Matcher::new(&query("İ", false, false)).unwrap();
        let mut budget = Budget {
            remaining_bytes: 100,
            remaining_matches: 10,
            remaining_groups: 3,
            remaining_chunks: MAX_CHUNK_VISITS,
        };
        assert_eq!(
            matcher
                .find(["i\u{307}"], &mut budget, || false)
                .unwrap()
                .ranges,
            [0..3]
        );
        assert_eq!(
            matcher.find(["İ"], &mut budget, || false).unwrap().ranges,
            [0..2]
        );
        assert_eq!(
            matcher.find([""], &mut budget, || false),
            Ok(Matches::default())
        );
        assert_eq!(
            matcher.find([""], &mut budget, || false),
            Err(Error::WorkLimit)
        );
    }

    #[test]
    fn empty_chunks_cannot_bypass_limits_or_cancellation() {
        let matcher = Matcher::new(&query("a", false, false)).unwrap();
        let mut budget = Budget {
            remaining_chunks: 3,
            ..Budget::default()
        };
        assert_eq!(
            matcher.find(std::iter::repeat(""), &mut budget, || false),
            Err(Error::WorkLimit)
        );
        let calls = Cell::new(0);
        assert_eq!(
            matcher.find(std::iter::repeat(""), &mut Budget::default(), || {
                calls.set(calls.get() + 1);
                calls.get() > 4
            }),
            Err(Error::Cancelled)
        );
        // A peek across exhausted chunks must discard the partial result, not
        // mistake the artificial end for a valid whole-word boundary.
        let matcher = Matcher::new(&query("a", true, true)).unwrap();
        let mut budget = Budget {
            remaining_chunks: 1,
            ..Budget::default()
        };
        assert_eq!(
            matcher.find(["a", "b"], &mut budget, || false),
            Err(Error::WorkLimit)
        );
    }
}
