//! Background-only syntax work. Never invoke a grammar from layout or paint.
use std::{ops::Range, sync::OnceLock};
use syntect::{
    easy::HighlightLines,
    highlighting::{FontStyle, ThemeSet},
    parsing::SyntaxSet,
};

fn syntaxes() -> &'static SyntaxSet {
    static SYNTAXES: OnceLock<SyntaxSet> = OnceLock::new();
    SYNTAXES.get_or_init(two_face::syntax::extra_newlines)
}

pub const MAX_HIGHLIGHT_BYTES: usize = 256 * 1024;
pub const MAX_LINE_BYTES: usize = 16 * 1024;
pub const MAX_RUNS: usize = 32768;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Run {
    pub bytes: Range<usize>,
    pub foreground: [u8; 3],
    pub background: Option<[u8; 3]>,
    pub bold: bool,
    pub italic: bool,
    pub underline: bool,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    Limit,
    Cancelled,
    Grammar,
}

/// Complete bounded input only. A limit/error returns no partially colored
/// prefix; callers render plain text and explain the expansion policy.
pub fn highlight(
    text: &str,
    language: &str,
    dark: bool,
    cancelled: impl Fn() -> bool,
) -> Result<Vec<Run>, Error> {
    if text.len() > MAX_HIGHLIGHT_BYTES {
        return Err(Error::Limit);
    }
    if cancelled() {
        return Err(Error::Cancelled);
    }
    static THEMES: OnceLock<ThemeSet> = OnceLock::new();
    let themes = THEMES.get_or_init(ThemeSet::load_defaults);
    let theme = &themes.themes[if dark {
        "base16-ocean.dark"
    } else {
        "InspiredGitHub"
    }];
    let syntaxes = syntaxes();
    let syntax = syntaxes
        .find_syntax_by_token(language)
        .or_else(|| syntaxes.find_syntax_by_extension(language))
        .unwrap_or_else(|| syntaxes.find_syntax_plain_text());
    let mut highlighter = HighlightLines::new(syntax, theme);
    let mut runs = Vec::new();
    let mut offset = 0;
    for line in text.split_inclusive('\n') {
        if cancelled() {
            return Err(Error::Cancelled);
        }
        if line.len() > MAX_LINE_BYTES {
            return Err(Error::Limit);
        }
        for (style, segment) in highlighter
            .highlight_line(line, syntaxes)
            .map_err(|_| Error::Grammar)?
        {
            let end = offset + segment.len();
            if end != offset {
                if runs.len() >= MAX_RUNS {
                    return Err(Error::Limit);
                }
                runs.push(Run {
                    bytes: offset..end,
                    foreground: [style.foreground.r, style.foreground.g, style.foreground.b],
                    background: None,
                    bold: style.font_style.contains(FontStyle::BOLD),
                    italic: style.font_style.contains(FontStyle::ITALIC),
                    underline: style.font_style.contains(FontStyle::UNDERLINE),
                });
            }
            offset = end;
        }
    }
    if cancelled() {
        return Err(Error::Cancelled);
    }
    Ok(runs)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn full_markdown_parse_revisits_earlier_references_and_incomplete_fences() {
        use gpui_base::text::{MarkdownExtensions, PreparedMarkdown};
        let before =
            PreparedMarkdown::parse("[name][target]\n", MarkdownExtensions::default()).unwrap();
        let after = PreparedMarkdown::parse(
            "[name][target]\n\n[target]: https://example.test\n",
            MarkdownExtensions::default(),
        )
        .unwrap();
        assert!(before.plain_text().contains("[name][target]"));
        assert!(after.plain_text().contains("name"));
        assert!(!after.plain_text().contains("[target]"));
        for source in [
            "```ocaml\nlet λ = 1",
            "```ocaml\nlet λ = 1\n```\n",
            "| a | b |\n|---|---|\n| λ | x |\n",
        ] {
            let prepared = PreparedMarkdown::parse(source, MarkdownExtensions::default()).unwrap();
            assert_eq!(prepared.source().as_ref(), source);
            assert!(prepared.block_count() > 0);
        }
        assert!(
            PreparedMarkdown::parse(&"x".repeat(65537), MarkdownExtensions::default()).is_err()
        );
        assert!(
            PreparedMarkdown::parse(&"a\n\n".repeat(257), MarkdownExtensions::default()).is_err()
        );
        assert!(
            PreparedMarkdown::parse(
                &format!("{}x", "> ".repeat(40)),
                MarkdownExtensions::default()
            )
            .is_err()
        );
    }
    #[test]
    fn required_language_inventory() {
        for token in [
            "ml", "rs", "sh", "json", "py", "js", "ts", "yaml", "toml", "ini",
        ] {
            assert!(
                syntaxes().find_syntax_by_extension(token).is_some(),
                "missing {token}"
            );
        }
    }

    #[test]
    fn ocaml_tokens_cover_unicode_and_multiline_comments() {
        let text = "(* λ\n nested (* comment *) *)\nlet answer = 42\n";
        let runs = highlight(text, "ml", true, || false).unwrap();
        assert_eq!(runs.first().unwrap().bytes.start, 0);
        assert_eq!(runs.last().unwrap().bytes.end, text.len());
        for pair in runs.windows(2) {
            assert_eq!(pair[0].bytes.end, pair[1].bytes.start);
        }
        for run in &runs {
            assert!(text.get(run.bytes.clone()).is_some());
        }
        assert!(runs.iter().any(|run| run.foreground != runs[0].foreground));
        assert_ne!(runs, highlight(text, "ml", false, || false).unwrap());
        assert_eq!(highlight(text, "ml", true, || true), Err(Error::Cancelled));
        assert_eq!(
            highlight(&"x".repeat(MAX_LINE_BYTES + 1), "ml", true, || false),
            Err(Error::Limit)
        );
    }
}
