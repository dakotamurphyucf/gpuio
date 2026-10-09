use binprot::macros::BinProtWrite;

#[derive(Clone, Copy, Debug, PartialEq, Eq, BinProtWrite)]
pub enum WrappingIndent {
    FlushLeft,
    MatchFirstLine,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, BinProtWrite)]
pub struct Config {
    pub soft_wrap: bool,
    pub wrapping_indent: WrappingIndent,
    pub show_whitespace: bool,
    pub cursor_margin_lines: Option<i64>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            soft_wrap: true,
            wrapping_indent: WrappingIndent::MatchFirstLine,
            show_whitespace: false,
            cursor_margin_lines: None,
        }
    }
}

impl Config {
    pub fn is_valid(&self) -> bool {
        self.cursor_margin_lines
            .is_none_or(|n| (0..=256).contains(&n))
    }
}
