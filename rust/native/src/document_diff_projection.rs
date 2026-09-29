//! Visible diff rows with an explicit mapping to the unmodified source.
//! Construction belongs to preparation/configuration, never paint. The mounted
//! owner must retain the exact source snapshot beside this projection.
use crate::document_diff::{Diff, File, Kind};
use std::ops::Range;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Row {
    pub source_line: usize,
    pub source: Range<usize>,
    pub display: Range<usize>,
}

#[derive(Clone, Debug)]
pub struct Projection {
    text: String,
    rows: Vec<Row>,
    source_bytes: usize,
    shown_body_lines: usize,
    hidden_body_lines: usize,
    collapsed_body_lines: usize,
}

fn body(kind: Kind) -> bool {
    matches!(
        kind,
        Kind::Context | Kind::Added | Kind::Removed | Kind::Meta
    )
}

impl Projection {
    /// [diff] must describe [source]. File indices belong to that snapshot;
    /// callers can collapse by index or by matching File::path labels. Sample
    /// each collapse decision once so counts and rows use the same state.
    /// Returns None for inconsistent byte/file ranges rather than slicing them.
    pub fn new(
        source: &str,
        diff: &Diff,
        collapsed: impl Fn(usize, &File) -> bool,
        max_lines: Option<usize>,
    ) -> Option<Self> {
        if source.len() > 262144 || diff.lines.len() > 8192 {
            return None;
        }
        let mut end = 0;
        for line in &diff.lines {
            if line.bytes.start != end || line.bytes.is_empty() {
                return None;
            }
            source.get(line.bytes.clone())?;
            end = line.bytes.end;
        }
        if end != source.len() {
            return None;
        }
        let mut file_end = diff
            .files
            .first()
            .map_or(diff.lines.len(), |f| f.lines.start);
        for file in &diff.files {
            if file.lines.start != file_end
                || file.lines.is_empty()
                || file.lines.end > diff.lines.len()
            {
                return None;
            }
            file_end = file.lines.end;
        }
        if file_end != diff.lines.len() {
            return None;
        }
        let collapsed: Vec<_> = diff
            .files
            .iter()
            .enumerate()
            .map(|(index, file)| collapsed(index, file))
            .collect();
        let mut available = 0;
        let mut collapsed_body_lines = 0;
        for (index, file) in diff.files.iter().enumerate() {
            let count = diff.lines[file.lines.clone()]
                .iter()
                .filter(|line| body(line.kind))
                .count();
            if collapsed[index] {
                collapsed_body_lines += count;
            } else {
                available += count;
            }
        }
        let preamble = diff
            .files
            .first()
            .map_or(diff.lines.len(), |f| f.lines.start);
        let mut visible: Vec<_> = (0..preamble).collect();
        let mut shown_body_lines = 0;
        let at_limit = |count| max_lines.is_some_and(|limit| count >= limit);
        'files: for (index, file) in diff.files.iter().enumerate() {
            // A preview ending at a file boundary must not show the next
            // header. This also applies if that next file is collapsed.
            if index > 0 && at_limit(shown_body_lines) {
                break;
            }
            if collapsed[index] {
                visible.push(file.lines.start);
                continue;
            }
            for line_index in file.lines.clone() {
                let line = &diff.lines[line_index];
                if (body(line.kind) || line.kind == Kind::Hunk) && at_limit(shown_body_lines) {
                    break 'files;
                }
                visible.push(line_index);
                shown_body_lines += usize::from(body(line.kind));
            }
        }
        let bytes = visible.iter().map(|&i| diff.lines[i].bytes.len()).sum();
        let mut text = String::with_capacity(bytes);
        let mut rows = Vec::with_capacity(visible.len());
        for source_line in visible {
            let source_range = diff.lines[source_line].bytes.clone();
            let start = text.len();
            text.push_str(&source[source_range.clone()]);
            rows.push(Row {
                source_line,
                source: source_range,
                display: start..text.len(),
            });
        }
        Some(Self {
            text,
            rows,
            source_bytes: source.len(),
            shown_body_lines,
            hidden_body_lines: available - shown_body_lines,
            collapsed_body_lines,
        })
    }

    pub fn text(&self) -> &str {
        &self.text
    }
    pub fn rows(&self) -> &[Row] {
        &self.rows
    }
    /// Retained allocation units for admission accounting, excluding the
    /// independently owned canonical snapshot and native editor/page buffers.
    pub fn retained_bytes(&self) -> usize {
        std::mem::size_of::<Self>()
            + self.text.capacity()
            + self.rows.capacity() * std::mem::size_of::<Row>()
    }
    pub fn shown_body_lines(&self) -> usize {
        self.shown_body_lines
    }
    /// Only expanded-file rows withheld by the preview, excluding collapsed
    /// bodies. Zero means there is no actionable Show more control.
    pub fn hidden_body_lines(&self) -> usize {
        self.hidden_body_lines
    }
    pub fn collapsed_body_lines(&self) -> usize {
        self.collapsed_body_lines
    }

    /// Project cached syntax runs without parsing or consulting OCaml. Invalid
    /// source ranges fail the complete projection; no partial styling escapes.
    pub fn highlights(
        &self,
        runs: &[crate::document_highlight::Run],
    ) -> Option<Vec<crate::document_highlight::Run>> {
        let mut projected = Vec::new();
        for run in runs {
            for bytes in self.display_ranges(run.bytes.clone())? {
                let mut run = run.clone();
                run.bytes = bytes;
                projected.push(run);
            }
        }
        Some(projected)
    }

    /// Fold candidates are display-row intervals, not original line numbers.
    /// A missing hunk header cannot create a candidate. The native editor can
    /// subsequently clip these to its bounded page, exactly as for raw source.
    pub fn hunks(&self, diff: &Diff) -> Vec<Range<usize>> {
        diff.hunks
            .iter()
            .filter_map(|hunk| {
                let start = self
                    .rows
                    .partition_point(|row| row.source_line < hunk.start);
                let row = self.rows.get(start)?;
                if row.source_line != hunk.start {
                    return None;
                }
                let end = self.rows.partition_point(|row| row.source_line < hunk.end);
                (end > start + 2).then_some(start..end)
            })
            .collect()
    }

    /// A caret uses the following visible row at a join; EOF uses the last
    /// visible row's end, not the canonical source's end. Empty views have no
    /// source caret. Interior UTF-8 byte positions are rejected.
    pub fn source_caret(&self, display: usize) -> Option<usize> {
        if !self.text.is_char_boundary(display) {
            return None;
        }
        if display == self.text.len() {
            return self.rows.last().map(|row| row.source.end);
        }
        let index = self.rows.partition_point(|row| row.display.end <= display);
        let row = self.rows.get(index)?;
        Some(row.source.start + display - row.display.start)
    }

    pub fn display_caret(&self, source: usize) -> Option<usize> {
        if self.rows.last().is_some_and(|row| row.source.end == source) {
            return Some(self.text.len());
        }
        let index = self.rows.partition_point(|row| row.source.end <= source);
        let row = self.rows.get(index)?;
        let offset = source.checked_sub(row.source.start)?;
        let display = row.display.start + offset;
        self.text.is_char_boundary(display).then_some(display)
    }

    /// Preserve a native selection only if the exact source intervals and
    /// selected bytes are still one contiguous display interval. Revealing or
    /// hiding text inside that interval clears it instead of silently selecting
    /// different content. The caller also fences source generation/continuity.
    /// Endpoint order (anchor/caret direction) is preserved.
    pub fn remap_selection(
        &self,
        next: &Self,
        selection: (usize, usize),
    ) -> Option<(usize, usize)> {
        let (anchor, caret) = selection;
        if anchor == caret {
            let display = next.display_caret(self.source_caret(caret)?)?;
            return Some((display, display));
        }
        let old = anchor.min(caret)..anchor.max(caret);
        let source = self.source_ranges(old.clone())?;
        let first = source.first()?;
        let last = source.last()?;
        let start = next.display_caret(first.start)?;
        // An end boundary may precede a hidden gap, so map the final selected
        // scalar interval rather than use the following-row caret affinity.
        let ends = next.display_ranges(last.clone())?;
        let end = ends.last()?.end;
        if next.source_ranges(start..end)? != source
            || next.text.get(start..end)? != self.text.get(old)?
        {
            return None;
        }
        Some(if anchor < caret {
            (start, end)
        } else {
            (end, start)
        })
    }

    /// Copy/selection mapping includes only visible source intervals. It never
    /// spans a hidden gap even when the display selection crosses that join.
    pub fn source_ranges(&self, display: Range<usize>) -> Option<Vec<Range<usize>>> {
        self.text.get(display.clone())?;
        let mut ranges = Vec::new();
        let start = self
            .rows
            .partition_point(|row| row.display.end <= display.start);
        for row in &self.rows[start..] {
            if row.display.start >= display.end {
                break;
            }
            let first = row.display.start.max(display.start);
            let last = row.display.end.min(display.end);
            if first < last {
                push_range(
                    &mut ranges,
                    row.source.start + first - row.display.start
                        ..row.source.start + last - row.display.start,
                );
            }
        }
        Some(ranges)
    }

    /// Intersect a valid canonical byte range with visible rows. Callers must
    /// validate scalar boundaries against their retained canonical snapshot;
    /// hidden text is intentionally absent here. Visible endpoints are checked.
    pub fn display_ranges(&self, source: Range<usize>) -> Option<Vec<Range<usize>>> {
        if source.start > source.end || source.end > self.source_bytes {
            return None;
        }
        let start = self
            .rows
            .partition_point(|row| row.source.end <= source.start);
        let mut ranges = Vec::new();
        for row in &self.rows[start..] {
            if row.source.start >= source.end {
                break;
            }
            let first = row.source.start.max(source.start);
            let last = row.source.end.min(source.end);
            if first < last {
                let range = row.display.start + first - row.source.start
                    ..row.display.start + last - row.source.start;
                self.text.get(range.clone())?;
                push_range(&mut ranges, range);
            }
        }
        Some(ranges)
    }
}

fn push_range(ranges: &mut Vec<Range<usize>>, range: Range<usize>) {
    if let Some(last) = ranges.last_mut()
        && last.end == range.start
    {
        last.end = range.end;
    } else {
        ranges.push(range);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::document_diff::parse;

    const PATCH: &str = concat!(
        "preamble\n",
        "diff --git a/one b/one\n--- a/one\n+++ b/one\n@@ -1,2 +1,2 @@\n",
        " keep λ\n-old\n+new\n\\ No newline at end of file\n",
        "diff --git a/two b/two\n--- a/two\n+++ b/two\n@@ -4 +8 @@\n",
        "-before\n+after 🌍\n",
        "diff --git a/three b/three\n@@ -0,0 +1 @@\n+last\n",
    );

    fn check_mapping(source: &str, p: &Projection) {
        let ranges = p.source_ranges(0..p.text.len()).unwrap();
        assert_eq!(
            ranges
                .iter()
                .map(|r| &source[r.clone()])
                .collect::<String>(),
            p.text()
        );
        for row in p.rows() {
            assert_eq!(&source[row.source.clone()], &p.text()[row.display.clone()]);
            assert_eq!(p.source_caret(row.display.start), Some(row.source.start));
        }
        for (byte, c) in p.text().char_indices() {
            let source_byte = p.source_caret(byte).unwrap();
            assert_eq!(source[source_byte..].chars().next(), Some(c));
            assert_eq!(
                p.display_ranges(source_byte..source_byte + c.len_utf8())
                    .unwrap(),
                vec![byte..byte + c.len_utf8()]
            );
        }
        assert_eq!(p.source_caret(p.text.len() + 1), None);
        assert!(p.source_ranges(0..p.text.len() + 1).is_none());
        assert!(p.display_ranges(0..source.len() + 1).is_none());
    }

    #[test]
    fn identity_and_collapsed_files_keep_original_byte_mapping() {
        let diff = parse(PATCH, || false).unwrap();
        let identity = Projection::new(PATCH, &diff, |_, _| false, None).unwrap();
        assert_eq!(identity.text(), PATCH);
        assert_eq!(identity.shown_body_lines(), 7);
        assert_eq!(identity.hidden_body_lines(), 0);
        check_mapping(PATCH, &identity);

        let p = Projection::new(PATCH, &diff, |_, file| file.path() == Some("two"), None).unwrap();
        assert_eq!(p.shown_body_lines(), 5);
        assert_eq!(p.collapsed_body_lines(), 2);
        assert_eq!(p.hidden_body_lines(), 0);
        assert!(p.text().contains("diff --git a/two b/two\n"));
        assert!(!p.text().contains("--- a/two") && !p.text().contains("after 🌍"));
        let start = PATCH.find("--- a/two").unwrap();
        let end = PATCH.find("diff --git a/three").unwrap();
        assert!(p.display_ranges(start..end).unwrap().is_empty());
        // Selecting across the join cannot accidentally copy the hidden file.
        assert_eq!(
            p.source_ranges(0..p.text.len()).unwrap(),
            vec![0..start, end..PATCH.len()]
        );
        check_mapping(PATCH, &p);
        let utf8 = p.text().find('λ').unwrap();
        assert_eq!(p.source_caret(utf8 + 1), None);
        assert!(p.source_ranges(utf8 + 1..utf8 + 2).is_none());
    }

    #[test]
    fn preview_counts_only_expanded_body_rows_and_stops_before_headers() {
        let diff = parse(PATCH, || false).unwrap();
        let p = Projection::new(PATCH, &diff, |_, _| false, Some(4)).unwrap();
        assert_eq!((p.shown_body_lines(), p.hidden_body_lines()), (4, 3));
        assert!(p.text().ends_with("\\ No newline at end of file\n"));
        assert!(!p.text().contains("diff --git a/two"));
        assert_eq!(
            p.source_caret(p.text.len()),
            Some(PATCH.find("diff --git a/two").unwrap())
        );

        let p = Projection::new(PATCH, &diff, |i, _| i == 0, Some(1)).unwrap();
        assert_eq!(
            (
                p.shown_body_lines(),
                p.hidden_body_lines(),
                p.collapsed_body_lines()
            ),
            (1, 2, 4)
        );
        assert!(p.text().contains("diff --git a/one b/one\n"));
        assert!(!p.text().contains("keep λ"));
        assert!(p.text().ends_with("-before\n"));
        check_mapping(PATCH, &p);

        let zero = Projection::new(PATCH, &diff, |_, _| false, Some(0)).unwrap();
        assert_eq!((zero.shown_body_lines(), zero.hidden_body_lines()), (0, 7));
        assert_eq!(zero.text(), &PATCH[..PATCH.find("@@").unwrap()]);
        let all_collapsed = Projection::new(PATCH, &diff, |_, _| true, None).unwrap();
        assert_eq!(
            (
                all_collapsed.hidden_body_lines(),
                all_collapsed.collapsed_body_lines()
            ),
            (0, 7)
        );
        assert_eq!(all_collapsed.rows().len(), 4); // Preamble plus three headers.
    }

    #[test]
    fn exact_hunk_boundaries_and_duplicate_labels_are_independent() {
        let source = "--- a/x\n+++ b/x\n@@ -1 +1 @@\n first\n@@ -5 +5 @@\n second\n--- a/x\n+++ b/x\n@@ -10 +10 @@\n third\n";
        let diff = parse(source, || false).unwrap();
        let first = Projection::new(source, &diff, |_, _| false, Some(1)).unwrap();
        assert!(first.text().ends_with(" first\n"));
        assert_eq!(first.hidden_body_lines(), 2);
        let both = Projection::new(source, &diff, |_, f| f.path() == Some("x"), None).unwrap();
        assert_eq!(both.text(), "--- a/x\n--- a/x\n");
        let one = Projection::new(source, &diff, |i, _| i == 0, None).unwrap();
        assert!(one.text().ends_with(" third\n"));
        assert_eq!((one.shown_body_lines(), one.collapsed_body_lines()), (1, 2));
        check_mapping(source, &one);
    }

    #[test]
    fn native_selection_moves_with_its_source_and_never_expands_across_hidden_gaps() {
        let diff = parse(PATCH, || false).unwrap();
        let all = Projection::new(PATCH, &diff, |_, _| false, None).unwrap();
        let collapsed = Projection::new(PATCH, &diff, |i, _| i == 0, None).unwrap();
        let start = PATCH.find("after 🌍").unwrap();
        let end = start + "after 🌍".len();
        let moved_start = collapsed.text().find("after 🌍").unwrap();
        let moved_end = moved_start + "after 🌍".len();
        assert_eq!(
            all.remap_selection(&collapsed, (start, end)),
            Some((moved_start, moved_end))
        );
        assert_eq!(
            all.remap_selection(&collapsed, (end, start)),
            Some((moved_end, moved_start))
        );
        assert_eq!(
            all.remap_selection(&collapsed, (start, start)),
            Some((moved_start, moved_start))
        );
        let hidden = PATCH.find("keep λ").unwrap();
        assert_eq!(all.remap_selection(&collapsed, (hidden, hidden + 4)), None);
        assert_eq!(all.remap_selection(&collapsed, (hidden, hidden)), None);
        assert_eq!(all.remap_selection(&collapsed, (0, end)), None);
        assert_eq!(collapsed.remap_selection(&all, (0, moved_end)), None);
        assert_eq!(
            collapsed.remap_selection(&all, (moved_start, moved_end)),
            Some((start, end))
        );
        // The previous projection's EOF is a valid caret before newly shown text.
        let preview = Projection::new(PATCH, &diff, |_, _| false, Some(4)).unwrap();
        assert_eq!(
            preview.remap_selection(&all, (preview.text.len(), preview.text.len())),
            Some((preview.text.len(), preview.text.len()))
        );
    }

    #[test]
    fn syntax_and_native_hunk_candidates_use_visible_offsets() {
        let diff = parse(PATCH, || false).unwrap();
        let p = Projection::new(PATCH, &diff, |i, _| i == 0, Some(2)).unwrap();
        let runs = crate::document_diff::highlights(&diff, false);
        let projected = p.highlights(&runs).unwrap();
        assert_eq!(projected.first().unwrap().bytes.start, 0);
        assert_eq!(projected.last().unwrap().bytes.end, p.text.len());
        assert!(
            projected
                .windows(2)
                .all(|pair| pair[0].bytes.end == pair[1].bytes.start)
        );
        for run in &projected {
            let source = p.source_ranges(run.bytes.clone()).unwrap();
            assert_eq!(source.len(), 1);
            let original = runs
                .iter()
                .find(|original| original.bytes == source[0])
                .unwrap();
            assert_eq!(run.foreground, original.foreground);
            assert_eq!(run.background, original.background);
            assert_eq!(run.bold, original.bold);
        }
        assert_eq!(p.hunks(&diff), vec![5..8]);
        let header = p.rows()[5].source_line;
        assert_eq!(diff.lines[header].kind, Kind::Hunk);
        assert!(
            p.rows()[5..8]
                .iter()
                .all(|row| diff.lines[row.source_line].file == Some(1))
        );
    }

    #[test]
    fn streaming_prefixes_limits_and_partial_unicode_keep_consistent_mapping() {
        for end in PATCH
            .char_indices()
            .map(|(index, _)| index)
            .chain([PATCH.len()])
        {
            let source = &PATCH[..end];
            let diff = parse(source, || false).unwrap();
            for max_lines in [None, Some(0), Some(1), Some(4), Some(8192)] {
                let p = Projection::new(source, &diff, |i, _| i == 1, max_lines).unwrap();
                check_mapping(source, &p);
                assert!(p.rows().len() <= diff.lines.len());
                assert!(p.text.len() <= source.len());
            }
        }
    }

    #[test]
    fn malformed_metadata_is_rejected_and_empty_source_has_no_caret() {
        let diff = parse("", || false).unwrap();
        let p = Projection::new("", &diff, |_, _| false, None).unwrap();
        assert_eq!(p.text(), "");
        assert_eq!(p.source_caret(0), None);
        assert!(p.source_ranges(0..0).unwrap().is_empty());
        let mut diff = parse(PATCH, || false).unwrap();
        assert!(Projection::new("different", &diff, |_, _| false, None).is_none());
        diff.files[0].lines.end = usize::MAX;
        assert!(Projection::new(PATCH, &diff, |_, _| false, None).is_none());
        let mut diff = parse("λ", || false).unwrap();
        diff.lines[0].bytes.end = 1;
        assert!(Projection::new("λ", &diff, |_, _| false, None).is_none());
    }

    #[test]
    fn large_collapsed_body_retains_only_its_header_and_visible_row_map() {
        let source = format!(
            "diff --git a/large b/large\n@@ -0,0 +1,6000 @@\n{}",
            "+λ\n".repeat(6000)
        );
        let diff = parse(&source, || false).unwrap();
        let p = Projection::new(&source, &diff, |_, _| true, None).unwrap();
        assert_eq!(p.rows().len(), 1);
        assert_eq!((p.shown_body_lines(), p.collapsed_body_lines()), (0, 6000));
        assert!(p.retained_bytes() < 1024);
        let preview = Projection::new(&source, &diff, |_, _| false, Some(20)).unwrap();
        assert_eq!(preview.rows().len(), 22);
        assert_eq!(preview.hidden_body_lines(), 5980);
        assert!(preview.retained_bytes() < 4096);
        check_mapping(&source, &preview);
    }
}
