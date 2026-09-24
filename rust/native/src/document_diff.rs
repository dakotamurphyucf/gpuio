//! Bounded unified-diff metadata. Preserve source verbatim, including partial
//! trailing hunks. Only well-formed hunk coordinates produce navigation targets.
use gpuio_protocol::document::{Navigation, Side};
use std::ops::Range;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Header,
    Hunk,
    Context,
    Added,
    Removed,
    Other,
}
#[derive(Clone, Debug)]
pub struct Line {
    pub bytes: Range<usize>,
    pub kind: Kind,
    pub before: Option<usize>,
    pub after: Option<usize>,
    pub before_path: Option<String>,
    pub after_path: Option<String>,
    pub changed: Option<Range<usize>>,
}
impl Line {
    pub fn navigation(&self) -> Option<Navigation> {
        if let Some(line) = self.after {
            Some(Navigation::Line(
                self.after_path.clone(),
                Side::After,
                line as i64,
            ))
        } else {
            self.before
                .map(|line| Navigation::Line(self.before_path.clone(), Side::Before, line as i64))
        }
    }
}
#[derive(Clone, Debug, Default)]
pub struct Diff {
    pub lines: Vec<Line>,
    pub hunks: Vec<Range<usize>>,
}
fn coordinate(value: &str, prefix: char) -> Option<(usize, usize)> {
    let value = value.strip_prefix(prefix)?;
    let (start, count) = value.split_once(',').unwrap_or((value, "1"));
    let start = start.parse::<usize>().ok()?;
    let count = count.parse::<usize>().ok()?;
    (start <= i32::MAX as usize && count <= i32::MAX as usize && (start > 0 || count == 0))
        .then_some((start, count))
}
fn hunk(line: &str) -> Option<((usize, usize), (usize, usize))> {
    let mut parts = line.strip_prefix("@@ ")?.split_whitespace();
    let before = coordinate(parts.next()?, '-')?;
    let after = coordinate(parts.next()?, '+')?;
    (parts.next()? == "@@").then_some((before, after))
}
fn path(value: &str) -> Option<String> {
    let path = value.split('\t').next()?.trim_end_matches('\r');
    if path == "/dev/null" || path.len() > 4096 {
        return None;
    }
    // Quoted Git names remain literal labels; they are never opened as paths.
    Some(
        path.strip_prefix("a/")
            .or_else(|| path.strip_prefix("b/"))
            .unwrap_or(path)
            .to_string(),
    )
}
pub fn parse(text: &str, cancelled: impl Fn() -> bool) -> Option<Diff> {
    if text.len() > 262144 {
        return None;
    }
    let mut diff = Diff::default();
    let mut offset = 0;
    let (mut before_path, mut after_path) = (None, None);
    let mut coordinates: Option<((usize, usize), (usize, usize))> = None;
    let mut previous_removed: Option<usize> = None;
    for raw in text.split_inclusive('\n') {
        if cancelled() || diff.lines.len() >= 8192 || raw.len() > 16384 {
            return None;
        }
        let value = raw.trim_end_matches(['\n', '\r']);
        let (mut before, mut after) = (None, None);
        // File headers are recognized outside a hunk. A removed payload may
        // itself start with "---", so prefix checks alone would corrupt paths.
        let kind = if let Some(new) = hunk(value) {
            if let Some(last) = diff.hunks.last_mut() {
                last.end = diff.lines.len();
            }
            diff.hunks.push(diff.lines.len()..diff.lines.len() + 1);
            coordinates = Some(new);
            Kind::Hunk
        } else if value.starts_with("diff --git ") {
            coordinates = None;
            before_path = None;
            after_path = None;
            Kind::Header
        } else if coordinates.is_none() && value.starts_with("--- ") {
            before_path = path(&value[4..]);
            Kind::Header
        } else if coordinates.is_none() && value.starts_with("+++ ") {
            after_path = path(&value[4..]);
            Kind::Header
        } else if let Some((old, new)) = coordinates.as_mut() {
            match value.as_bytes().first() {
                Some(b'-') if old.1 > 0 => {
                    before = Some(old.0);
                    old.0 += 1;
                    old.1 -= 1;
                    Kind::Removed
                }
                Some(b'+') if new.1 > 0 => {
                    after = Some(new.0);
                    new.0 += 1;
                    new.1 -= 1;
                    Kind::Added
                }
                Some(b' ') if old.1 > 0 && new.1 > 0 => {
                    before = Some(old.0);
                    after = Some(new.0);
                    old.0 += 1;
                    new.0 += 1;
                    old.1 -= 1;
                    new.1 -= 1;
                    Kind::Context
                }
                Some(b'\\') => Kind::Other,
                _ => {
                    coordinates = None;
                    Kind::Other
                }
            }
        } else {
            Kind::Other
        };
        let index = diff.lines.len();
        diff.lines.push(Line {
            bytes: offset..offset + raw.len(),
            kind,
            before,
            after,
            before_path: before_path.clone(),
            after_path: after_path.clone(),
            changed: None,
        });
        if kind == Kind::Added
            && let Some(previous) = previous_removed
        {
            let old: &Line = &diff.lines[previous];
            let a = text[old.bytes.start + 1..old.bytes.end].trim_end_matches(['\r', '\n']);
            let b = value.get(1..).unwrap_or("");
            let prefix = a
                .chars()
                .zip(b.chars())
                .take_while(|(a, b)| a == b)
                .map(|(a, _)| a.len_utf8())
                .sum::<usize>();
            let suffix = a[prefix..]
                .chars()
                .rev()
                .zip(b[prefix..].chars().rev())
                .take_while(|(a, b)| a == b)
                .map(|(a, _)| a.len_utf8())
                .sum::<usize>();
            let old_start = old.bytes.start + 1;
            let a_len = a.len();
            diff.lines[previous].changed = Some(old_start + prefix..old_start + a_len - suffix);
            diff.lines[index].changed = Some(offset + 1 + prefix..offset + 1 + b.len() - suffix);
        }
        previous_removed = (kind == Kind::Removed).then_some(index);
        offset += raw.len();
        if coordinates.is_some_and(|(old, new)| old.1 == 0 && new.1 == 0) {
            coordinates = None;
        }
    }
    if let Some(last) = diff.hunks.last_mut() {
        last.end = diff.lines.len();
    }
    Some(diff)
}
pub fn highlights(diff: &Diff, dark: bool) -> Vec<crate::document_highlight::Run> {
    let mut runs = Vec::new();
    for line in &diff.lines {
        let foreground = match (line.kind, dark) {
            (Kind::Added, true) => [80, 190, 110],
            (Kind::Added, false) => [20, 100, 45],
            (Kind::Removed, true) => [240, 110, 110],
            (Kind::Removed, false) => [150, 35, 35],
            (Kind::Hunk, true) => [100, 170, 240],
            (Kind::Hunk, false) => [30, 80, 160],
            (_, true) => [190, 195, 205],
            (_, false) => [40, 45, 55],
        };
        let background = match (line.kind, dark) {
            (Kind::Added, true) => Some([25, 55, 35]),
            (Kind::Added, false) => Some([225, 250, 230]),
            (Kind::Removed, true) => Some([65, 30, 30]),
            (Kind::Removed, false) => Some([255, 230, 230]),
            _ => None,
        };
        let mut push = |bytes: Range<usize>, emphasis: bool| {
            if !bytes.is_empty() {
                runs.push(crate::document_highlight::Run {
                    bytes,
                    foreground,
                    background,
                    bold: emphasis,
                    italic: false,
                    underline: emphasis,
                });
            }
        };
        if let Some(changed) = &line.changed {
            push(line.bytes.start..changed.start, false);
            push(changed.clone(), true);
            push(changed.end..line.bytes.end, false);
        } else {
            push(line.bytes.clone(), false);
        }
    }
    runs
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn streaming_unicode_and_payload_header_are_not_lost() {
        let source = "--- a/a.ml\n+++ b/a.ml\n@@ -9,2 +9,2 @@\n--- λ old\n+-- λ new\n context\n";
        let diff = parse(source, || false).unwrap();
        assert_eq!(
            diff.lines
                .iter()
                .map(|l| &source[l.bytes.clone()])
                .collect::<String>(),
            source
        );
        assert_eq!(diff.lines[3].kind, Kind::Removed);
        assert_eq!(diff.lines[3].before, Some(9));
        assert_eq!(diff.lines[4].after, Some(9));
        assert_eq!(diff.lines[4].after_path.as_deref(), Some("a.ml"));
        assert_eq!(&source[diff.lines[4].changed.clone().unwrap()], "new");
        for end in source.char_indices().map(|(i, _)| i) {
            let prefix = &source[..end];
            let partial = parse(prefix, || false).unwrap();
            assert_eq!(
                partial
                    .lines
                    .iter()
                    .map(|l| &prefix[l.bytes.clone()])
                    .collect::<String>(),
                prefix
            );
        }
        assert!(parse(source, || true).is_none());
        assert!(hunk("@@ -99999999999999999999999 +1 @@").is_none());
        assert!(hunk("@@ -0 +1 @@").is_none());
    }
}
