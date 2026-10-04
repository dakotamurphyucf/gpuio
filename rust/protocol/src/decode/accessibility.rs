use super::{DecodeError, Decoder};
use crate::accessibility::*;
use std::io::Cursor;

impl Decoder<'_> {
    fn table_cell_semantics(&mut self) -> Result<TableCell, DecodeError> {
        Ok(TableCell {
            row: self.int()?,
            column: self.int()?,
            column_span: self.int()?,
        })
    }
    pub(super) fn accessibility_config(&mut self) -> Result<Config, DecodeError> {
        let config = Config {
            role: self.option(|d| {
                Ok(match d.tag()? {
                    0 => Role::Group,
                    1 => Role::Label,
                    2 => Role::Link,
                    3 => Role::Separator,
                    4 => Role::DescriptionList,
                    5 => Role::Term,
                    6 => Role::Definition,
                    7 => Role::Status,
                    8 => Role::Alert,
                    9 => Role::Image,
                    10 => Role::Heading(d.int()?),
                    11 => Role::Navigation,
                    12 => Role::Tree(d.boolean()?),
                    13 => Role::TreeItem(TreeItem {
                        level: d.int()?,
                        index: d.int()?,
                        count: d.option(|d| d.int())?,
                        expanded: d.option(|d| d.boolean())?,
                        selected: d.boolean()?,
                        disabled: d.boolean()?,
                        busy: d.boolean()?,
                    }),
                    14 => Role::Toolbar(match d.tag()? {
                        0 => Orientation::Horizontal,
                        1 => Orientation::Vertical,
                        _ => return Err(DecodeError::Malformed),
                    }),
                    15 => Role::RadioGroup(match d.tag()? {
                        0 => Orientation::Horizontal,
                        1 => Orientation::Vertical,
                        _ => return Err(DecodeError::Malformed),
                    }),
                    16 => Role::Log,
                    17 => Role::ListBox(d.boolean()?),
                    18 => Role::OptionItem(OptionItem {
                        index: d.int()?,
                        count: d.option(|d| d.int())?,
                        selected: d.boolean()?,
                        disabled: d.boolean()?,
                    }),
                    19 => Role::Table(TableInfo {
                        rows: d.option(|d| d.int())?,
                        columns: d.option(|d| d.int())?,
                    }),
                    20 => Role::RowGroup,
                    21 => Role::TableRow(d.int()?),
                    22 => Role::TableCell(d.table_cell_semantics()?),
                    23 => Role::ColumnHeader(d.table_cell_semantics()?),
                    24 => Role::RowHeader(d.table_cell_semantics()?),
                    25 => Role::Caption,
                    _ => return Err(DecodeError::Malformed),
                })
            })?,
            label: self.option(|d| d.bounded_text(MAX_TEXT_BYTES))?,
            description: self.option(|d| d.bounded_text(MAX_TEXT_BYTES))?,
            live: match self.tag()? {
                0 => Live::Off,
                1 => Live::Polite,
                2 => Live::Assertive,
                _ => return Err(DecodeError::Malformed),
            },
            field: self.option(|d| {
                Ok(Field {
                    label: d.bounded_text(MAX_TEXT_BYTES)?,
                    help: d.option(|d| d.bounded_text(MAX_TEXT_BYTES))?,
                    error: d.option(|d| d.bounded_text(MAX_TEXT_BYTES))?,
                    required: d.boolean()?,
                })
            })?,
            current: self.option(|d| {
                Ok(match d.tag()? {
                    0 => Current::True,
                    1 => Current::Page,
                    2 => Current::Step,
                    3 => Current::Location,
                    4 => Current::Date,
                    5 => Current::Time,
                    _ => return Err(DecodeError::Malformed),
                })
            })?,
        };
        if config.is_valid() {
            Ok(config)
        } else {
            Err(DecodeError::Malformed)
        }
    }
}
pub fn decode_accessibility(bytes: &[u8]) -> Result<Config, DecodeError> {
    if bytes.len() > MAX_CONFIG_BYTES {
        return Err(DecodeError::LimitExceeded);
    }
    let mut decoder = Decoder(Cursor::new(bytes));
    let config = decoder.accessibility_config()?;
    if decoder.remaining() != 0 {
        return Err(DecodeError::Malformed);
    }
    Ok(config)
}
