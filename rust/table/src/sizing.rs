// Table sizing derived from gpui-kit 84f57fdf; Apache-2.0, see ../UPSTREAM.md.
use gpui::{Edges, Pixels, Styled, px};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Size {
    Size(Pixels),
    XSmall,
    Small,
    #[default]
    Medium,
    Large,
}

impl Size {
    pub fn table_row_height(self) -> Pixels {
        match self {
            Self::Size(size) => size,
            Self::XSmall => px(26.),
            Self::Small => px(30.),
            Self::Medium => px(32.),
            Self::Large => px(40.),
        }
    }

    pub fn table_cell_padding(self) -> Edges<Pixels> {
        let (vertical, horizontal) = match self {
            Self::XSmall => (2., 4.),
            Self::Small => (3., 6.),
            Self::Large => (8., 12.),
            Self::Medium | Self::Size(_) => (4., 8.),
        };
        Edges {
            top: px(vertical),
            bottom: px(vertical),
            left: px(horizontal),
            right: px(horizontal),
        }
    }
}

pub trait Sizable: Sized {
    fn with_size(self, size: impl Into<Size>) -> Self;
}

pub(crate) trait StyleSized: Styled + Sized {
    fn table_cell_size(self, size: Size) -> Self {
        let padding = size.table_cell_padding();
        let this = match size {
            Size::XSmall | Size::Small => self.text_sm(),
            Size::Medium | Size::Large | Size::Size(_) => self,
        };
        this.pl(padding.left)
            .pr(padding.right)
            .pt(padding.top)
            .pb(padding.bottom)
    }
}
impl<T: Styled> StyleSized for T {}
