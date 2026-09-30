//! Bounded syntax for visible patch content; never read the named files.
use crate::{
    document_diff::{Diff, Kind},
    document_highlight::{self as syntax, Error, Run},
};
use std::ops::Range;

struct Segment {
    buffer: Range<usize>,
    source: Range<usize>,
    paint: bool,
}

/// Admission units for source/side buffers, parser rows/files/mappings, and
/// temporary plus final run vectors (including vector growth). Shared immutable
/// grammars are process resources, not charged anew per patch.
pub fn work_units(source_bytes: usize) -> usize {
    let bytes = source_bytes.min(syntax::MAX_HIGHLIGHT_BYTES);
    4096 + 4 * bytes
        + 2 * bytes.min(8192)
            * (std::mem::size_of::<crate::document_diff::Line>()
                + std::mem::size_of::<crate::document_diff::File>()
                + std::mem::size_of::<Range<usize>>()
                + std::mem::size_of::<Segment>())
        + 8 * bytes.min(syntax::MAX_RUNS) * std::mem::size_of::<Run>()
}

/// Reset grammar state at each hunk: omitted source cannot establish lexical
/// state, and a large source line number must not cause padding or allocation.
pub fn highlight(
    text: &str,
    diff: &Diff,
    dark: bool,
    cancelled: impl Fn() -> bool,
) -> Result<Vec<Run>, Error> {
    let mut colored = Vec::new();
    for file in &diff.files {
        if cancelled() {
            return Err(Error::Cancelled);
        }
        if file.binary {
            continue;
        }
        let old = file
            .before_path
            .as_deref()
            .and_then(syntax::language_for_path);
        let new = file
            .after_path
            .as_deref()
            .and_then(syntax::language_for_path);
        for hunk in &diff.hunks[file.hunks.clone()] {
            for (before, language) in [(true, old), (false, new)] {
                let Some(language) = language else {
                    continue;
                };
                let mut buffer = String::new();
                let mut segments = Vec::new();
                for line in &diff.lines[hunk.clone()] {
                    if cancelled() {
                        return Err(Error::Cancelled);
                    }
                    if !matches!(
                        (line.kind, before),
                        (Kind::Context, _) | (Kind::Removed, true) | (Kind::Added, false)
                    ) {
                        continue;
                    }
                    let start = buffer.len();
                    buffer.push_str(&text[line.content.clone()]);
                    segments.push(Segment {
                        buffer: start..buffer.len(),
                        source: line.content.clone(),
                        paint: line.kind != Kind::Context || !before || new.is_none(),
                    });
                    buffer.push('\n');
                }
                let runs = syntax::highlight(&buffer, language, dark, &cancelled)?;
                let mut cursor = 0;
                for segment in segments {
                    while cursor < runs.len() && runs[cursor].bytes.end <= segment.buffer.start {
                        cursor += 1;
                    }
                    if !segment.paint {
                        continue;
                    }
                    for run in &runs[cursor..] {
                        if run.bytes.start >= segment.buffer.end {
                            break;
                        }
                        let start = run.bytes.start.max(segment.buffer.start);
                        let end = run.bytes.end.min(segment.buffer.end);
                        if start >= end {
                            continue;
                        }
                        if colored.len() == syntax::MAX_RUNS {
                            return Err(Error::Limit);
                        }
                        let mut mapped = run.clone();
                        mapped.bytes = segment.source.start + start - segment.buffer.start
                            ..segment.source.start + end - segment.buffer.start;
                        colored.push(mapped);
                    }
                }
            }
        }
    }
    if cancelled() {
        return Err(Error::Cancelled);
    }
    colored.sort_unstable_by_key(|run| run.bytes.start);
    let mut output = Vec::new();
    let mut cursor = 0;
    for base in crate::document_diff::highlights(diff, dark) {
        if cancelled() {
            return Err(Error::Cancelled);
        }
        while cursor < colored.len() && colored[cursor].bytes.end <= base.bytes.start {
            cursor += 1;
        }
        let mut start = base.bytes.start;
        for token in &colored[cursor..] {
            if token.bytes.start >= base.bytes.end {
                break;
            }
            let a = token.bytes.start.max(base.bytes.start);
            let b = token.bytes.end.min(base.bytes.end);
            if start < a {
                push(&mut output, &base, start..a)?;
            }
            let mut styled = base.clone();
            styled.foreground = token.foreground;
            styled.bold = token.bold;
            styled.italic = token.italic;
            styled.underline = token.underline;
            push(&mut output, &styled, a..b)?;
            start = b;
        }
        if start < base.bytes.end {
            push(&mut output, &base, start..base.bytes.end)?;
        }
    }
    Ok(output)
}

fn push(output: &mut Vec<Run>, style: &Run, bytes: Range<usize>) -> Result<(), Error> {
    if output.len() == syntax::MAX_RUNS {
        return Err(Error::Limit);
    }
    let mut run = style.clone();
    run.bytes = bytes;
    output.push(run);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    fn color(runs: &[Run], offset: usize) -> [u8; 3] {
        runs.iter()
            .find(|run| run.bytes.contains(&offset))
            .unwrap()
            .foreground
    }
    fn check_coverage(text: &str, runs: &[Run]) {
        let mut offset = 0;
        for run in runs {
            assert_eq!(run.bytes.start, offset);
            assert!(text.get(run.bytes.clone()).is_some());
            assert!(!run.bytes.is_empty());
            offset = run.bytes.end;
        }
        assert_eq!(offset, text.len());
        assert!(runs.len() <= syntax::MAX_RUNS);
    }
    #[test]
    fn renamed_languages_preserve_markers_backgrounds_and_exact_unicode_bytes() {
        let text = "--- a/old.ml\r\n+++ b/new.json\r\n@@ -1 +1 @@\r\n-let answer = \"世界\"\r\n+{\"answer\": 42}\r\n";
        let diff = crate::document_diff::parse(text, || false).unwrap();
        for dark in [true, false] {
            let runs = highlight(text, &diff, dark, || false).unwrap();
            check_coverage(text, &runs);
            let old = syntax::highlight("let answer = \"世界\"\n", "ml", dark, || false).unwrap();
            let new = syntax::highlight("{\"answer\": 42}\n", "json", dark, || false).unwrap();
            assert_eq!(
                color(&runs, text.find("let answer").unwrap()),
                color(&old, 0)
            );
            assert_eq!(color(&runs, text.find("42").unwrap()), color(&new, 11));
            let base = crate::document_diff::highlights(&diff, dark);
            for run in &runs {
                let original = base
                    .iter()
                    .find(|r| r.bytes.contains(&run.bytes.start))
                    .unwrap();
                assert_eq!(run.background, original.background);
                assert_eq!(run.diff_emphasis, original.diff_emphasis);
            }
            assert_eq!(
                color(&runs, text.find("-let").unwrap()),
                color(&base, text.find("-let").unwrap())
            );
            assert!(runs.iter().any(|run| run.diff_emphasis));
        }
    }
    #[test]
    fn old_and_new_lexical_states_are_independent_and_context_uses_new_side() {
        let text = "--- a/a.ml\n+++ b/a.ml\n@@ -1,3 +1,3 @@\n-(* old\n+let value = 1\n context\n-*)\n+let other = 2\n";
        let diff = crate::document_diff::parse(text, || false).unwrap();
        let runs = highlight(text, &diff, true, || false).unwrap();
        check_coverage(text, &runs);
        let old_source = "(* old\ncontext\n*)\n";
        let new_source = "let value = 1\ncontext\nlet other = 2\n";
        let old = syntax::highlight(old_source, "ml", true, || false).unwrap();
        let new = syntax::highlight(new_source, "ml", true, || false).unwrap();
        assert_eq!(
            color(&runs, text.find("*)").unwrap()),
            color(&old, old_source.find("*)").unwrap())
        );
        assert_eq!(
            color(&runs, text.find("context").unwrap()),
            color(&new, new_source.find("context").unwrap())
        );
        assert_eq!(
            color(&runs, text.find("let other").unwrap()),
            color(&new, new_source.find("let other").unwrap())
        );
        assert_ne!(
            color(&old, old_source.find("context").unwrap()),
            color(&new, new_source.find("context").unwrap())
        );
    }
    #[test]
    fn hunk_gaps_reset_state_without_padding_large_coordinates() {
        let text = "--- a/a.ml\n+++ b/a.ml\n@@ -1 +1 @@\n (* unclosed\n@@ -600000 +900000 @@\n let value = 42\n";
        let diff = crate::document_diff::parse(text, || false).unwrap();
        let runs = highlight(text, &diff, true, || false).unwrap();
        check_coverage(text, &runs);
        let expected = syntax::highlight("let value = 42\n", "ml", true, || false).unwrap();
        assert_eq!(
            color(&runs, text.find("let value").unwrap()),
            color(&expected, 0)
        );
        assert!(runs.len() < 40);
        let calls = std::cell::Cell::new(0);
        assert_eq!(
            highlight(text, &diff, true, || {
                calls.set(calls.get() + 1);
                calls.get() > 4
            }),
            Err(Error::Cancelled)
        );
    }
    #[test]
    fn unknown_and_quoted_display_paths_never_require_file_access() {
        assert_eq!(
            syntax::language_for_path("/does/not/exist/世界.ml"),
            syntax::language_for_path("file.ml")
        );
        assert!(syntax::language_for_path("\"a/space name.ML\"").is_some());
        assert!(syntax::language_for_path("file.gpuio_unknown_extension").is_none());
        let text = "--- a/a.unknown\n+++ b/a.unknown\n@@ -1 +1 @@\n-old\n+new\n";
        let diff = crate::document_diff::parse(text, || false).unwrap();
        assert_eq!(
            highlight(text, &diff, true, || false).unwrap(),
            crate::document_diff::highlights(&diff, true)
        );
    }
    #[test]
    fn total_run_budget_rejects_a_partial_syntax_result() {
        let text = format!(
            "--- /dev/null\n+++ b/many.ml\n@@ -0,0 +1,3000 @@\n{}",
            "+let value = 42 (* note *)\n".repeat(3000)
        );
        let diff = crate::document_diff::parse(&text, || false).unwrap();
        assert_eq!(highlight(&text, &diff, true, || false), Err(Error::Limit));
    }
}
