use crate::{Error, MAX_CODE_BYTES, MAX_RUNS, PrepareContext, contain};
use std::ops::Range;

/// Complete fenced-code input; offsets in output count UTF-8 bytes of this text.
pub struct Code<'a> {
    text: &'a str,
    language: Option<&'a str>,
    dark: bool,
}
impl<'a> Code<'a> {
    pub fn new(text: &'a str, language: Option<&'a str>, dark: bool) -> Result<Self, Error> {
        if text.len() > MAX_CODE_BYTES || language.is_some_and(|s| s.len() > 4096) {
            return Err(Error::LimitExceeded);
        }
        Ok(Self {
            text,
            language,
            dark,
        })
    }
    pub fn text(&self) -> &'a str {
        self.text
    }
    pub fn language(&self) -> Option<&'a str> {
        self.language
    }
    pub fn dark(&self) -> bool {
        self.dark
    }
}

/// Concrete finite styling. A host converts these values into native glyph styles.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Highlight {
    pub bytes: Range<usize>,
    pub foreground: [u8; 3],
    pub background: Option<[u8; 3]>,
    pub weight: u16,
    pub italic: bool,
    pub underline: bool,
    pub strikethrough: bool,
}
impl Highlight {
    pub fn new(bytes: Range<usize>, foreground: [u8; 3]) -> Self {
        Self {
            bytes,
            foreground,
            background: None,
            weight: 400,
            italic: false,
            underline: false,
            strikethrough: false,
        }
    }
}

pub trait Highlighter: Send + Sync + 'static {
    /// Run only in background preparation, never layout/paint. Returning an empty
    /// vector intentionally produces plain code; absence of a hook uses the host default.
    fn highlight(&self, code: &Code<'_>, cx: &PrepareContext<'_>) -> Result<Vec<Highlight>, Error>;
}

pub(crate) fn run(
    highlighter: &dyn Highlighter,
    code: &Code<'_>,
    cx: &PrepareContext<'_>,
) -> Result<Vec<Highlight>, Error> {
    cx.check()?;
    let runs = contain(|| highlighter.highlight(code, cx))?;
    cx.check()?;
    if runs.len() > MAX_RUNS {
        return Err(Error::LimitExceeded);
    }
    let mut previous_end = 0;
    for run in &runs {
        if run.bytes.start < previous_end
            || run.bytes.start >= run.bytes.end
            || run.bytes.end > code.text.len()
            || !code.text.is_char_boundary(run.bytes.start)
            || !code.text.is_char_boundary(run.bytes.end)
            || !(1..=1000).contains(&run.weight)
        {
            return Err(Error::InvalidHighlight);
        }
        previous_end = run.bytes.end;
    }
    Ok(runs)
}
