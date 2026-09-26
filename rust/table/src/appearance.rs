use gpui::{Hsla, Pixels, px, rgb};

/// Resolved per-instance colors, supplied by the host's style/token adapter.
#[derive(Clone, Copy, Debug)]
pub struct Colors {
    pub table: Hsla,
    pub table_head: Hsla,
    pub table_even: Hsla,
    pub table_hover: Hsla,
    pub table_active: Hsla,
    pub secondary: Hsla,
    pub secondary_active: Hsla,
    pub accent: Hsla,
}

#[derive(Clone, Copy, Debug)]
pub struct SelectionAppearance {
    pub active_highlight: bool,
}

#[derive(Clone, Copy, Debug)]
pub struct Appearance {
    pub tokens: Colors,
    pub foreground: Hsla,
    pub table_head_foreground: Hsla,
    pub table_row_border: Hsla,
    pub table_active_border: Hsla,
    pub border: Hsla,
    pub drag_border: Hsla,
    pub secondary_foreground: Hsla,
    pub muted_foreground: Hsla,
    pub selection: Hsla,
    pub radius: Pixels,
    pub list: SelectionAppearance,
}

impl Default for Appearance {
    fn default() -> Self {
        Self {
            tokens: Colors {
                table: rgb(0x17191f).into(),
                table_head: rgb(0x20232c).into(),
                table_even: rgb(0x1c1f27).into(),
                table_hover: rgb(0x262b38).into(),
                table_active: rgb(0x303f65).into(),
                secondary: rgb(0x343c50).into(),
                secondary_active: rgb(0x465578).into(),
                accent: rgb(0x7499f2).into(),
            },
            foreground: rgb(0xe0e5f0).into(),
            table_head_foreground: rgb(0xc0cbdf).into(),
            table_row_border: rgb(0x2a2f3d).into(),
            table_active_border: rgb(0x7499f2).into(),
            border: rgb(0x3b4355).into(),
            drag_border: rgb(0x7499f2).into(),
            secondary_foreground: rgb(0xc0cbdf).into(),
            muted_foreground: rgb(0x9ba7be).into(),
            selection: rgb(0x7499f2).into(),
            radius: px(5.),
            list: SelectionAppearance {
                active_highlight: true,
            },
        }
    }
}
