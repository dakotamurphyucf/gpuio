//! Literal UTF-8 byte search over rope chunks. O(source + query), bounded
//! match storage, no flattened full-document allocation or UI-thread scan.
use ropey::Rope;
use std::ops::Range;
#[derive(Default, Debug)]
pub struct Matches {
    pub ranges: Vec<Range<usize>>,
    pub total: usize,
}
pub const MAX_MATCHES: usize = 4096;
pub fn find(text: &Rope, query: &str, cancelled: impl Fn() -> bool) -> Option<Matches> {
    if query.is_empty() {
        return Some(Matches::default());
    }
    if query.len() > 4096 {
        return None;
    }
    let needle = query.as_bytes();
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
    let mut out = Matches::default();
    let mut matched = 0;
    let mut offset = 0;
    for chunk in text.chunks() {
        if cancelled() {
            return None;
        }
        for byte in chunk.bytes() {
            while matched > 0 && byte != needle[matched] {
                matched = prefix[matched - 1];
            }
            if byte == needle[matched] {
                matched += 1;
            }
            offset += 1;
            if matched == needle.len() {
                if out.ranges.len() < MAX_MATCHES {
                    out.ranges.push(offset - needle.len()..offset);
                }
                out.total += 1;
                matched = 0; // Literal editor-compatible non-overlapping matches.
            }
        }
    }
    Some(out)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn chunk_boundaries_unicode_limits_and_cancellation() {
        let text = format!("{}λ 👨‍👩‍👧‍👦{}", "x".repeat(2047), "λ".repeat(5000));
        let rope = Rope::from(text.as_str());
        for query in ["λ", "xλ", "👨‍👩‍👧‍👦", "👦λ", "absent"] {
            let result = find(&rope, query, || false).unwrap();
            let expected: Vec<_> = text
                .match_indices(query)
                .map(|(i, _)| i..i + query.len())
                .collect();
            assert_eq!(result.total, expected.len());
            assert_eq!(
                result.ranges,
                expected.into_iter().take(MAX_MATCHES).collect::<Vec<_>>()
            );
        }
        assert!(find(&rope, "λ", || true).is_none());
    }
}
