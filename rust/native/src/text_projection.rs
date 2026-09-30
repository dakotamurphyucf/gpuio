//! Mapping between source UTF-8 bytes and retained, shaped text after truncation.
use gpui::TextOverflow;
use std::ops::Range;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Mapping {
    pub(crate) source: Range<usize>,
    pub(crate) displayed: usize,
}
/// Never search for the query in displayed text. Map only retained source slices;
/// synthetic ellipsis text cannot acquire a match or contribute an ordinal.
pub(crate) fn mapping(
    source: &str,
    displayed: &str,
    overflow: Option<&TextOverflow>,
) -> Vec<Mapping> {
    if source == displayed {
        return vec![Mapping {
            source: 0..source.len(),
            displayed: 0,
        }];
    }
    match overflow {
        Some(TextOverflow::Truncate(affix)) => displayed
            .strip_suffix(affix.as_ref())
            .filter(|s| source.starts_with(s))
            .map(|s| {
                vec![Mapping {
                    source: 0..s.len(),
                    displayed: 0,
                }]
            })
            .unwrap_or_default(),
        Some(TextOverflow::TruncateStart(affix)) => displayed
            .strip_prefix(affix.as_ref())
            .filter(|s| source.ends_with(s))
            .map(|s| {
                vec![Mapping {
                    source: source.len() - s.len()..source.len(),
                    displayed: affix.len(),
                }]
            })
            .unwrap_or_default(),
        Some(TextOverflow::TruncateMiddle(affix)) if !affix.is_empty() => displayed
            .match_indices(affix.as_ref())
            .find_map(|(index, _)| {
                let prefix = &displayed[..index];
                let suffix = &displayed[index + affix.len()..];
                (prefix.len() + suffix.len() <= source.len()
                    && source.starts_with(prefix)
                    && source.ends_with(suffix))
                .then(|| {
                    vec![
                        Mapping {
                            source: 0..prefix.len(),
                            displayed: 0,
                        },
                        Mapping {
                            source: source.len() - suffix.len()..source.len(),
                            displayed: index + affix.len(),
                        },
                    ]
                })
            })
            .unwrap_or_default(),
        None | Some(TextOverflow::TruncateMiddle(_)) => {
            if source.starts_with(displayed) {
                vec![Mapping {
                    source: 0..displayed.len(),
                    displayed: 0,
                }]
            } else {
                vec![]
            }
        }
    }
}
/// A geometric range covers only real retained glyphs. If it spans both retained
/// pieces, its contiguous source range includes the omitted middle bytes.
/// Selecting only synthetic ellipsis glyphs yields no source selection.
pub(crate) fn source_range(parts: &[Mapping], displayed: Range<usize>) -> Option<Range<usize>> {
    let mut range: Option<Range<usize>> = None;
    for part in parts {
        let start = displayed.start.max(part.displayed);
        let end = displayed.end.min(part.displayed + part.source.len());
        if start < end {
            let source = part.source.start + start - part.displayed
                ..part.source.start + end - part.displayed;
            match &mut range {
                Some(range) => range.end = source.end,
                None => range = Some(source),
            }
        }
    }
    range
}

/// Map a caret in either coordinate space. Hidden source and synthetic display
/// positions snap to the nearest retained edge; endpoints stay UTF-8 boundaries.
fn offset(parts: &[Mapping], index: usize, to_source: bool) -> Option<usize> {
    let mut nearest = None;
    for part in parts {
        let (from, to) = if to_source {
            (
                part.displayed..part.displayed + part.source.len(),
                part.source.start,
            )
        } else {
            (part.source.clone(), part.displayed)
        };
        if from.start <= index && index <= from.end {
            return Some(to + index - from.start);
        }
        for (edge, mapped) in [(from.start, to), (from.end, to + from.len())] {
            let distance = edge.abs_diff(index);
            if nearest.is_none_or(|(best, _)| distance < best) {
                nearest = Some((distance, mapped));
            }
        }
    }
    nearest.map(|(_, mapped)| mapped)
}
pub(crate) fn source_offset(parts: &[Mapping], displayed: usize) -> Option<usize> {
    offset(parts, displayed, true)
}
pub(crate) fn displayed_offset(parts: &[Mapping], source: usize) -> Option<usize> {
    offset(parts, source, false)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ellipsis_mapping_preserves_source_coordinates_and_excludes_synthetic_text() {
        let source = "α beta γ";
        let parts = mapping(
            source,
            "α…γ",
            Some(&TextOverflow::TruncateMiddle("…".into())),
        );
        assert_eq!(source_range(&parts, 0..2), Some(0..2));
        assert_eq!(source_range(&parts, 2..5), None);
        assert_eq!(source_range(&parts, 5..7), Some(8..10));
        assert_eq!(source_range(&parts, 0..7), Some(0..10));
        assert_eq!(source_offset(&parts, 5), Some(8));
        assert_eq!(displayed_offset(&parts, 8), Some(5));
        assert_eq!(displayed_offset(&parts, 4), Some(2));
        assert_eq!(displayed_offset(&parts, 7), Some(5));
        let start = mapping(source, "…γ", Some(&TextOverflow::TruncateStart("…".into())));
        assert_eq!(source_offset(&start, 0), Some(8));
        assert_eq!(source_range(&start, 0..5), Some(8..10));
        let end = mapping(source, "α…", Some(&TextOverflow::Truncate("…".into())));
        assert_eq!(source_offset(&end, 5), Some(2));
        assert_eq!(source_range(&end, 0..5), Some(0..2));
        let empty = mapping(source, "…", Some(&TextOverflow::Truncate("…".into())));
        assert_eq!(source_range(&empty, 0..3), None);
        assert_eq!(source_offset(&empty, 3), Some(0));
        assert_eq!(source_range(&[], 0..7), None);
        assert_eq!(source_offset(&[], 0), None);
        // A same-length replacement is still synthetic, not an identity map.
        let equal = mapping("café", "ca…", Some(&TextOverflow::Truncate("…".into())));
        assert_eq!(source_range(&equal, 0..5), Some(0..2));
    }
}
