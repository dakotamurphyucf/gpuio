//! Scoped native header/row presentation, independent of content and geometry.
use crate::v1::{Field, Style};

pub const MAX_DECLARATIONS: usize = 128;

pub fn valid_scope(styles: &[Style], row: bool) -> bool {
    if styles.len() > MAX_DECLARATIONS {
        return false;
    }
    let mut count = 0;
    styles.iter().all(|style| {
        let fields = match style {
            Style::Fields(fields) => fields,
            Style::State(state, fields)
                if matches!(*state, 2 | 3 | 6) || (row && matches!(*state, 1 | 7)) =>
            {
                fields
            }
            _ => return false,
        };
        count += fields.len();
        count <= MAX_DECLARATIONS
            && fields.iter().all(|field| {
                matches!(
                    field,
                    Field::Background(_)
                        | Field::Foreground(_)
                        | Field::BorderColor(_)
                        | Field::BorderStyle(_)
                        | Field::TopLeftRadius(_)
                        | Field::TopRightRadius(_)
                        | Field::BottomLeftRadius(_)
                        | Field::BottomRightRadius(_)
                        | Field::Shadows(_)
                        | Field::FontSize(_)
                        | Field::FontFamily(_)
                        | Field::FontWeight(_)
                        | Field::TextDecoration(_)
                )
            })
    })
}
