//! Paint configuration for native split-button coordination.
//! The standalone value does not add a tree operation or component by itself.
use crate::v1::{Field, Style};
use binprot::macros::BinProtWrite;

pub const MAX_DECLARATIONS: usize = 64;

#[derive(Clone, Copy, Debug, PartialEq, Eq, BinProtWrite)]
pub enum Parts {
    Primary,
    Menu,
    Split,
}

#[derive(Clone, Debug, PartialEq, BinProtWrite)]
pub struct Config {
    pub parts: Parts,
    pub surface: Vec<Style>,
    pub menu_open: Vec<Style>,
}

impl Config {
    /// Shape/budget only. Native admission must also validate field values and
    /// the retained component graph before publishing this presentation.
    pub fn has_valid_shape(&self) -> bool {
        let mut remaining = MAX_DECLARATIONS;
        [&self.surface, &self.menu_open].into_iter().all(|styles| {
            styles.len() <= MAX_DECLARATIONS
                && styles.iter().all(|style| {
                    let Style::Fields(fields) = style else {
                        return false;
                    };
                    let Some(rest) = remaining.checked_sub(fields.len()) else {
                        return false;
                    };
                    remaining = rest;
                    fields.iter().all(|field| {
                        matches!(
                            field,
                            Field::Background(_)
                                | Field::Foreground(_)
                                | Field::BorderColor(_)
                                | Field::Shadows(_)
                                | Field::TextDecoration(_)
                        )
                    })
                })
        })
    }
}
