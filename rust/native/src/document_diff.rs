//! Bounded unified-diff metadata. Preserve source verbatim, including partial
//! trailing hunks. Only well-formed hunk coordinates produce navigation targets.
use gpuio_protocol::document::{Navigation, Side};
use std::{ops::Range, sync::Arc};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Header,
    Hunk,
    Context,
    Added,
    Removed,
    /// A hunk annotation such as the no-final-newline marker; no side number.
    Meta,
    Other,
}
#[derive(Clone, Debug)]
pub struct Line {
    pub bytes: Range<usize>,
    /// UTF-8 payload bytes, excluding the diff marker and one line ending.
    pub content: Range<usize>,
    pub kind: Kind,
    pub before: Option<usize>,
    pub after: Option<usize>,
    pub before_path: Option<Arc<str>>,
    pub after_path: Option<Arc<str>>,
    pub changed: Option<Range<usize>>,
    /// Source-local file index; preamble outside any file has no owner.
    pub file: Option<usize>,
}
impl Line {
    pub fn content<'a>(&self, source: &'a str) -> Option<&'a str> {
        source.get(self.content.clone())
    }
    pub fn navigation(&self) -> Option<Navigation> {
        if let Some(line) = self.after {
            Some(Navigation::Line(
                self.after_path.as_deref().map(str::to_owned),
                Side::After,
                line as i64,
            ))
        } else {
            self.before.map(|line| {
                Navigation::Line(
                    self.before_path.as_deref().map(str::to_owned),
                    Side::Before,
                    line as i64,
                )
            })
        }
    }
}
#[derive(Clone, Debug)]
pub struct File {
    pub before_path: Option<Arc<str>>,
    pub after_path: Option<Arc<str>>,
    /// Contiguous source line indices, including file metadata.
    pub lines: Range<usize>,
    /// Indices into Diff.hunks; every hunk stays inside this file.
    pub hunks: Range<usize>,
    pub added: usize,
    pub removed: usize,
    pub body_lines: usize,
    pub binary: bool,
}
impl File {
    /// Labels are never resolved as filesystem paths. Deleted files use the
    /// before label; additions and renames use the after label.
    pub fn path(&self) -> Option<&str> {
        self.after_path.as_deref().or(self.before_path.as_deref())
    }
}
#[derive(Clone, Debug, Default)]
pub struct Diff {
    pub lines: Vec<Line>,
    pub hunks: Vec<Range<usize>>,
    pub files: Vec<File>,
}
impl Diff {
    fn finish_file(&mut self, end: usize) {
        if let Some(file) = self.files.last_mut() {
            file.lines.end = end;
            if !file.hunks.is_empty() {
                self.hunks[file.hunks.end - 1].end = end;
            }
        }
    }
    fn start_file(&mut self, line: usize) {
        self.finish_file(line);
        let hunk = self.hunks.len();
        self.files.push(File {
            before_path: None,
            after_path: None,
            lines: line..line,
            hunks: hunk..hunk,
            added: 0,
            removed: 0,
            body_lines: 0,
            binary: false,
        });
    }
}
fn coordinate(value: &str, prefix: char) -> Option<(usize, usize)> {
    let value = value.strip_prefix(prefix)?;
    let (start, count) = value.split_once(',').unwrap_or((value, "1"));
    let start = start.parse::<usize>().ok()?;
    let count = count.parse::<usize>().ok()?;
    (start <= i32::MAX as usize
        && count <= i32::MAX as usize
        && (start > 0 || count == 0)
        && count.saturating_sub(1) <= (i32::MAX as usize).saturating_sub(start))
    .then_some((start, count))
}
fn hunk(line: &str) -> Option<((usize, usize), (usize, usize))> {
    let mut parts = line.strip_prefix("@@ ")?.split_whitespace();
    let before = coordinate(parts.next()?, '-')?;
    let after = coordinate(parts.next()?, '+')?;
    (parts.next()? == "@@").then_some((before, after))
}
fn label(value: &str) -> Option<Arc<str>> {
    (!value.is_empty() && value.len() <= 4096 && !value.contains('\0')).then(|| Arc::from(value))
}
fn path(value: &str) -> Option<Arc<str>> {
    let path = value.split('\t').next()?.trim_end_matches('\r');
    if path == "/dev/null" {
        return None;
    }
    // Quoted Git names remain literal labels; they are never opened as paths.
    label(
        path.strip_prefix("a/")
            .or_else(|| path.strip_prefix("b/"))
            .unwrap_or(path),
    )
}
fn git_paths(value: &str) -> (Option<Arc<str>>, Option<Arc<str>>) {
    // Git leaves ordinary spaces unquoted. The common unchanged-path form can
    // be split without guessing where a filename's own spaces belong.
    if value.len() >= 5 && (value.len() - 5).is_multiple_of(2) {
        let length = (value.len() - 5) / 2;
        if let (Some(before), Some(after)) = (value.get(..2 + length), value.get(3 + length..))
            && before
                .strip_prefix("a/")
                .is_some_and(|before| Some(before) == after.strip_prefix("b/"))
            && value.as_bytes()[2 + length] == b' '
        {
            let name = path(before);
            return (name.clone(), name);
        }
    }
    fn token(value: &str) -> Option<(&str, &str)> {
        if value.starts_with('"') {
            let mut escaped = false;
            for (index, byte) in value.bytes().enumerate().skip(1) {
                if escaped {
                    escaped = false;
                } else if byte == b'\\' {
                    escaped = true;
                } else if byte == b'"' {
                    return Some((&value[..index + 1], &value[index + 1..]));
                }
            }
            None
        } else {
            let end = value.find(char::is_whitespace).unwrap_or(value.len());
            (end > 0).then_some((&value[..end], &value[end..]))
        }
    }
    let Some((before, rest)) = token(value) else {
        return (None, None);
    };
    let Some((after, rest)) = token(rest.trim_start()) else {
        return (None, None);
    };
    if !rest.trim().is_empty() {
        return (None, None);
    }
    (path(before), path(after))
}
fn without_line_ending(value: &str) -> &str {
    value
        .strip_suffix('\n')
        .map_or(value, |line| line.strip_suffix('\r').unwrap_or(line))
}

pub fn parse(text: &str, cancelled: impl Fn() -> bool) -> Option<Diff> {
    if text.len() > 262144 {
        return None;
    }
    let mut diff = Diff::default();
    let mut offset = 0;
    let mut unified_header = false;
    let mut coordinates: Option<((usize, usize), (usize, usize))> = None;
    let mut previous_removed: Option<usize> = None;
    for raw in text.split_inclusive('\n') {
        if cancelled() || diff.lines.len() >= 8192 || raw.len() > 16384 {
            return None;
        }
        let value = without_line_ending(raw);
        let (mut before, mut after) = (None, None);
        // File headers are recognized outside a hunk. Removed payloads can
        // themselves start with "---" and must retain their source coordinates.
        let kind = if let Some(new) = hunk(value) {
            if diff.files.is_empty() {
                diff.start_file(diff.lines.len());
            }
            let file = diff.files.last_mut().unwrap();
            if !file.hunks.is_empty() {
                diff.hunks[file.hunks.end - 1].end = diff.lines.len();
            }
            diff.hunks.push(diff.lines.len()..diff.lines.len() + 1);
            file.hunks.end = diff.hunks.len();
            coordinates = Some(new);
            Kind::Hunk
        } else if let Some(names) = value.strip_prefix("diff --git ") {
            coordinates = None;
            unified_header = false;
            diff.start_file(diff.lines.len());
            let file = diff.files.last_mut().unwrap();
            (file.before_path, file.after_path) = git_paths(names);
            Kind::Header
        } else if coordinates.is_none() && value.starts_with("--- ") {
            if diff.files.is_empty() || unified_header {
                diff.start_file(diff.lines.len());
            }
            unified_header = true;
            diff.files.last_mut().unwrap().before_path = path(&value[4..]);
            Kind::Header
        } else if coordinates.is_none() && value.starts_with("+++ ") {
            if diff.files.is_empty() {
                diff.start_file(diff.lines.len());
            }
            diff.files.last_mut().unwrap().after_path = path(&value[4..]);
            Kind::Header
        } else if value.starts_with('\\')
            && diff.files.last().is_some_and(|file| !file.hunks.is_empty())
        {
            Kind::Meta
        } else if coordinates.is_none()
            && !diff.files.is_empty()
            && (value.starts_with("rename from ")
                || value.starts_with("rename to ")
                || value.starts_with("copy from ")
                || value.starts_with("copy to ")
                || value.starts_with("new file mode ")
                || value.starts_with("deleted file mode ")
                || value.starts_with("Binary files ")
                || value == "GIT binary patch")
        {
            let file = diff.files.last_mut().unwrap();
            if let Some(name) = value
                .strip_prefix("rename from ")
                .or_else(|| value.strip_prefix("copy from "))
            {
                file.before_path = label(name);
            } else if let Some(name) = value
                .strip_prefix("rename to ")
                .or_else(|| value.strip_prefix("copy to "))
            {
                file.after_path = label(name);
            } else if value.starts_with("new file mode ") {
                file.before_path = None;
            } else if value.starts_with("deleted file mode ") {
                file.after_path = None;
            } else {
                file.binary = true;
            }
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
                Some(b'\\') => Kind::Meta,
                _ => {
                    coordinates = None;
                    Kind::Other
                }
            }
        } else {
            Kind::Other
        };
        let index = diff.lines.len();
        let marker = match kind {
            Kind::Context | Kind::Added | Kind::Removed => 1,
            Kind::Meta => {
                if value.starts_with("\\ ") {
                    2
                } else {
                    1
                }
            }
            Kind::Header | Kind::Hunk | Kind::Other => 0,
        };
        diff.lines.push(Line {
            bytes: offset..offset + raw.len(),
            content: offset + marker..offset + value.len(),
            kind,
            before,
            after,
            before_path: diff.files.last().and_then(|file| file.before_path.clone()),
            after_path: diff.files.last().and_then(|file| file.after_path.clone()),
            changed: None,
            file: diff.files.len().checked_sub(1),
        });
        if let Some(file) = diff.files.last_mut() {
            file.lines.end = index + 1;
            file.added += usize::from(kind == Kind::Added);
            file.removed += usize::from(kind == Kind::Removed);
            file.body_lines += usize::from(matches!(
                kind,
                Kind::Added | Kind::Removed | Kind::Context | Kind::Meta
            ));
        }
        if kind == Kind::Added
            && let Some(previous) = previous_removed
        {
            let old: &Line = &diff.lines[previous];
            let a = without_line_ending(&text[old.bytes.start + 1..old.bytes.end]);
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
    diff.finish_file(diff.lines.len());
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
    fn invariant(source: &str, diff: &Diff) {
        assert_eq!(
            diff.lines
                .iter()
                .map(|line| &source[line.bytes.clone()])
                .collect::<String>(),
            source
        );
        for (index, file) in diff.files.iter().enumerate() {
            assert!(file.lines.start < file.lines.end && file.lines.end <= diff.lines.len());
            assert!(file.hunks.end <= diff.hunks.len());
            if index > 0 {
                assert_eq!(diff.files[index - 1].lines.end, file.lines.start);
            }
            for line in &diff.lines[file.lines.clone()] {
                assert_eq!(line.file, Some(index));
            }
            for hunk in &diff.hunks[file.hunks.clone()] {
                assert!(file.lines.contains(&hunk.start) && hunk.end <= file.lines.end);
                assert_eq!(diff.lines[hunk.start].kind, Kind::Hunk);
            }
            assert_eq!(
                file.body_lines,
                diff.lines[file.lines.clone()]
                    .iter()
                    .filter(|line| matches!(
                        line.kind,
                        Kind::Context | Kind::Added | Kind::Removed | Kind::Meta
                    ))
                    .count()
            );
        }
        for (line_index, line) in diff.lines.iter().enumerate() {
            assert!(line.bytes.start <= line.content.start && line.content.end <= line.bytes.end);
            assert!(line.content(source).is_some());
            if let Some(index) = line.file {
                assert!(diff.files[index].lines.contains(&line_index));
            }
            if let Some(navigation) = line.navigation() {
                assert!(navigation.is_valid());
            }
        }
    }
    #[test]
    fn files_close_hunks_before_unrelated_headers_and_share_path_storage() {
        let source = "preamble\ndiff --git a/one b/one\n--- a/one\n+++ b/one\n@@ -1 +1 @@\n-old\n+new\ndiff --git a/a/old name b/a/new name\nsimilarity index 100%\nrename from a/old name\nrename to a/new name\ndiff --git a/image one.png b/image one.png\nnew file mode 100644\nBinary files /dev/null and b/image one.png differ\n";
        let diff = parse(source, || false).unwrap();
        invariant(source, &diff);
        assert_eq!(diff.files.len(), 3);
        assert_eq!(diff.lines[0].file, None);
        assert_eq!(
            diff.files.iter().map(File::path).collect::<Vec<_>>(),
            [Some("one"), Some("a/new name"), Some("image one.png")]
        );
        assert_eq!(diff.files[1].before_path.as_deref(), Some("a/old name"));
        assert_eq!(diff.hunks, vec![4..7]);
        assert_eq!(diff.files[0].hunks, 0..1);
        assert_eq!(diff.files[1].hunks, 1..1);
        assert_eq!(
            (
                diff.files[0].added,
                diff.files[0].removed,
                diff.files[0].body_lines
            ),
            (1, 1, 2)
        );
        assert!(diff.files[2].binary);
        assert!(diff.files[2].before_path.is_none());
        assert_eq!(diff.files[2].body_lines, 0);
        for end in source.char_indices().map(|(index, _)| index) {
            invariant(&source[..end], &parse(&source[..end], || false).unwrap());
        }
    }
    #[test]
    fn unified_files_deletions_duplicate_paths_and_hunk_annotations() {
        let source = "--- a/repeated\n+++ /dev/null\n@@ -1 +0,0 @@\n-gone\n\\ No newline at end of file\n--- /dev/null\n+++ b/repeated\n@@ -0,0 +1 @@\n+new\n\\ No newline at end of file\n";
        let diff = parse(source, || false).unwrap();
        invariant(source, &diff);
        assert_eq!(diff.files.len(), 2);
        assert_eq!(diff.files[0].path(), Some("repeated"));
        assert_eq!(diff.files[1].path(), Some("repeated"));
        assert_eq!(diff.files[0].hunks, 0..1);
        assert_eq!(diff.files[1].hunks, 1..2);
        assert_eq!(diff.hunks, vec![2..5, 7..10]);
        assert_eq!((diff.files[0].body_lines, diff.files[1].body_lines), (2, 2));
        assert!(diff.files[0].after_path.is_none());
        assert!(diff.files[1].before_path.is_none());
        assert_eq!(diff.lines[4].kind, Kind::Meta);
        assert!(diff.lines[4].navigation().is_none());
        assert_ne!(diff.lines[3].file, diff.lines[8].file);
        for end in source.char_indices().map(|(index, _)| index) {
            invariant(&source[..end], &parse(&source[..end], || false).unwrap());
        }
    }
    #[test]
    fn payloads_preserve_unicode_carriage_returns_and_both_line_numbers() {
        let source = "--- a/old\r\n+++ b/new\r\n@@ -10,3 +20,3 @@\r\n shared λ\r\n-old\r\r\n+new\r\r\n\\ No newline at end of file\r\n last 🌍\r";
        let diff = parse(source, || false).unwrap();
        invariant(source, &diff);
        let rows = diff.lines[3..]
            .iter()
            .map(|line| {
                (
                    line.kind,
                    line.content(source).unwrap(),
                    line.before,
                    line.after,
                )
            })
            .collect::<Vec<_>>();
        assert_eq!(
            rows,
            vec![
                (Kind::Context, "shared λ", Some(10), Some(20)),
                (Kind::Removed, "old\r", Some(11), None),
                (Kind::Added, "new\r", None, Some(21)),
                (Kind::Meta, "No newline at end of file", None, None),
                (Kind::Context, "last 🌍\r", Some(12), Some(22)),
            ]
        );
        assert_eq!(&source[diff.lines[4].changed.clone().unwrap()], "old");
        assert_eq!(&source[diff.lines[5].changed.clone().unwrap()], "new");
        assert_eq!(diff.files[0].body_lines, 5);
        assert_eq!(diff.files[0].before_path.as_deref(), Some("old"));
        assert_eq!(diff.files[0].after_path.as_deref(), Some("new"));
        for end in source.char_indices().map(|(index, _)| index) {
            invariant(&source[..end], &parse(&source[..end], || false).unwrap());
        }
    }
    #[test]
    fn path_labels_are_bounded_literal_and_never_implicitly_read() {
        for (value, expected) in [
            ("a/dir name b/dir name", Some("dir name")),
            ("a/世界 b/世界", Some("世界")),
            ("a/path b/path", Some("path")),
            ("a/old b/new", Some("new")),
            ("a/ambiguous old b/ambiguous new", None),
            (
                "\"a/quoted name\" \"b/quoted name\"",
                Some("\"b/quoted name\""),
            ),
            (
                "\"a/escaped\\\"name\" \"b/escaped\\\"name\"",
                Some("\"b/escaped\\\"name\""),
            ),
            ("\"unterminated", None),
        ] {
            assert_eq!(git_paths(value).1.as_deref(), expected, "{value}");
        }
        assert!(path("").is_none());
        assert!(path("b/\0bad").is_none());
        assert!(path(&format!("b/{}", "x".repeat(4097))).is_none());
        assert_eq!(path("a/../label-only").as_deref(), Some("../label-only"));
        assert_eq!(path("b/file\t2026-09-29").as_deref(), Some("file"));
    }
    #[test]
    fn long_path_is_shared_across_thousands_of_rows_and_limits_are_enforced() {
        let name = "x".repeat(4096);
        let source = format!(
            "--- /dev/null\n+++ b/{name}\n@@ -0,0 +1,6000 @@\n{}",
            "+x\n".repeat(6000)
        );
        let diff = parse(&source, || false).unwrap();
        assert_eq!(diff.files[0].added, 6000);
        let path = diff.files[0].after_path.as_ref().unwrap();
        assert!(
            diff.lines[3..]
                .iter()
                .all(|line| Arc::ptr_eq(line.after_path.as_ref().unwrap(), path))
        );
        assert!(parse(&"\n".repeat(8192), || false).is_some());
        assert!(parse(&"\n".repeat(8193), || false).is_none());
        assert!(parse(&"x".repeat(16385), || false).is_none());
        assert!(parse(&"x\n".repeat(131073), || false).is_none());
        assert!(hunk("@@ -2147483647,2 +1,2 @@").is_none());
        assert!(hunk("@@ -1,2 +2147483647,2 @@").is_none());
        assert!(hunk("@@ -2147483647 +2147483647 @@").is_some());
        let fragment = "@@ -1 +1 @@\n-a\n+b";
        let parsed = parse(fragment, || false).unwrap();
        invariant(fragment, &parsed);
        assert_eq!(parsed.files.len(), 1);
        assert!(parsed.files[0].path().is_none());
    }
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
