use super::{DecodeError, Decoder};
use crate::table::*;
use std::io::Cursor;

impl Decoder<'_> {
    fn table_padding(&mut self) -> Result<Padding, DecodeError> {
        let p = Padding {
            top: self.float()?,
            right: self.float()?,
            bottom: self.float()?,
            left: self.float()?,
        };
        if p.is_valid() {
            Ok(p)
        } else {
            Err(DecodeError::Malformed)
        }
    }
    pub(super) fn table_appearance(&mut self) -> Result<Appearance, DecodeError> {
        let striped = self.boolean()?;
        let count = self.count(13)?;
        let colors = (0..count)
            .map(|_| {
                let part = match self.tag()? {
                    0 => Part::HeaderBackground,
                    1 => Part::HeaderForeground,
                    2 => Part::StripeBackground,
                    3 => Part::HoverBackground,
                    4 => Part::SelectedBackground,
                    5 => Part::SelectedBorder,
                    6 => Part::RowBorder,
                    7 => Part::ColumnBorder,
                    8 => Part::SortHoverBackground,
                    9 => Part::SortPressedBackground,
                    10 => Part::SortForeground,
                    11 => Part::DragBorder,
                    12 => Part::ContextBorder,
                    _ => return Err(DecodeError::Malformed),
                };
                Ok((part, self.int()?))
            })
            .collect::<Result<Vec<_>, DecodeError>>()?;
        let padding = self.option(Self::table_padding)?;
        let count = self.count(MAX_COLUMNS)?;
        let column_padding = (0..count)
            .map(|_| Ok((self.bounded_text(256)?, self.table_padding()?)))
            .collect::<Result<Vec<_>, DecodeError>>()?;
        let value = Appearance {
            striped,
            colors,
            padding,
            column_padding,
        };
        if value.is_valid() {
            Ok(value)
        } else {
            Err(DecodeError::Malformed)
        }
    }

    pub(super) fn table_behavior(&mut self) -> Result<Behavior, DecodeError> {
        let row_header = self.boolean()?;
        let boundary = match self.tag()? {
            0 => Boundary::Stop,
            1 => Boundary::Wrap,
            _ => return Err(DecodeError::Malformed),
        };
        let selectable_headers = self.option(|d| {
            let count = d.count(MAX_COLUMNS)?;
            (0..count)
                .map(|_| d.bounded_text(256))
                .collect::<Result<Vec<_>, _>>()
        })?;
        let value = Behavior {
            row_header,
            boundary,
            selectable_headers,
        };
        if !value.is_valid() {
            return Err(DecodeError::Malformed);
        }
        Ok(value)
    }
    fn table_text(&mut self, budget: &mut usize, limit: usize) -> Result<String, DecodeError> {
        let value = self.bounded_text(limit.min(*budget))?;
        *budget -= value.len();
        Ok(value)
    }
    fn table_direction(&mut self) -> Result<Direction, DecodeError> {
        match self.tag()? {
            0 => Ok(Direction::Ascending),
            1 => Ok(Direction::Descending),
            _ => Err(DecodeError::Malformed),
        }
    }
    fn table_schema(&mut self) -> Result<Schema, DecodeError> {
        let mut budget = MAX_SCHEMA_TEXT_BYTES;
        let count = self.count(MAX_COLUMNS)?;
        let mut columns = Vec::with_capacity(count);
        for _ in 0..count {
            columns.push(Column {
                id: self.table_text(&mut budget, 256)?,
                label: self.table_text(&mut budget, 4096)?,
                width: self.float()?,
                min_width: self.float()?,
                max_width: self.float()?,
                pin: match self.tag()? {
                    0 => Pin::Unpinned,
                    1 => Pin::Left,
                    _ => return Err(DecodeError::Malformed),
                },
                alignment: match self.tag()? {
                    0 => Alignment::Left,
                    1 => Alignment::Center,
                    2 => Alignment::Right,
                    _ => return Err(DecodeError::Malformed),
                },
                resizable: self.boolean()?,
                movable: self.boolean()?,
                sortable: self.boolean()?,
            });
        }
        let count = self.count(MAX_HEADER_LEVELS)?;
        let mut headers = Vec::with_capacity(count);
        for _ in 0..count {
            let count = self.count(MAX_COLUMNS)?;
            let mut groups = Vec::with_capacity(count);
            for _ in 0..count {
                let label = self.table_text(&mut budget, 4096)?;
                let count = self.count(MAX_COLUMNS)?;
                let mut columns = Vec::with_capacity(count);
                for _ in 0..count {
                    columns.push(self.table_text(&mut budget, 256)?);
                }
                groups.push(Group { label, columns });
            }
            headers.push(groups);
        }
        let schema = Schema { columns, headers };
        if schema.is_valid() {
            Ok(schema)
        } else {
            Err(DecodeError::Malformed)
        }
    }
    pub(super) fn table_config(&mut self) -> Result<Config, DecodeError> {
        let config = Config {
            schema_revision: self.int()?,
            query_generation: self.int()?,
            schema: self.table_schema()?,
            sort: self.option(|decoder| {
                Ok(Sort {
                    column: decoder.bounded_text(256)?,
                    direction: decoder.table_direction()?,
                })
            })?,
            row_height: self.float()?,
            overscan: self.float()?,
            max_active_rows: self.int()?,
            max_active_cells: self.int()?,
            selection_mode: match self.tag()? {
                0 => SelectionMode::Rows,
                1 => SelectionMode::Cells,
                2 => SelectionMode::RowsAndCells,
                _ => return Err(DecodeError::Malformed),
            },
            column_selection: self.boolean()?,
            disabled: self.boolean()?,
            scrollbar: self.boolean()?,
            label: self.bounded_text(1024)?,
        };
        if config.is_valid() {
            Ok(config)
        } else {
            Err(DecodeError::Malformed)
        }
    }
    pub(super) fn table_cell(&mut self) -> Result<Cell, DecodeError> {
        let cell = Cell {
            column: self.bounded_text(256)?,
            copy_text: self.bounded_text(MAX_COPY_BYTES)?,
        };
        if cell.is_valid() {
            Ok(cell)
        } else {
            Err(DecodeError::Malformed)
        }
    }
    fn table_selection(&mut self) -> Result<Selection, DecodeError> {
        let selection = match self.tag()? {
            0 => Selection::Empty,
            1 => Selection::Row(self.int()?),
            2 => Selection::Column(self.bounded_text(256)?),
            3 => Selection::Cell(self.int()?, self.bounded_text(256)?),
            _ => return Err(DecodeError::Malformed),
        };
        if selection.is_valid() {
            Ok(selection)
        } else {
            Err(DecodeError::Malformed)
        }
    }
    pub(super) fn table_command(&mut self) -> Result<Command, DecodeError> {
        let serial = self.int()?;
        let query_generation = self.int()?;
        let target = match self.tag()? {
            0 => Target::SetSelection(self.table_selection()?),
            1 => Target::Reveal(self.int()?, self.option(|d| d.bounded_text(256))?),
            2 => Target::ScrollTo(self.int()?, self.float()?),
            3 => Target::ScrollToColumn(self.bounded_text(256)?),
            4 => Target::ScrollToEnd,
            5 => Target::ResetColumns,
            _ => return Err(DecodeError::Malformed),
        };
        let command = Command {
            serial,
            query_generation,
            target,
        };
        if command.is_valid() {
            Ok(command)
        } else {
            Err(DecodeError::Malformed)
        }
    }
    pub(super) fn table_request(&mut self) -> Result<Request, DecodeError> {
        let request = match self.tag()? {
            0 => Request::Select(self.table_selection()?),
            1 => Request::Activate(self.int()?, self.option(|d| d.bounded_text(256))?),
            2 => Request::Context(self.table_selection()?),
            3 => {
                let count = self.count(MAX_COLUMNS)?;
                let mut widths = Vec::with_capacity(count);
                for _ in 0..count {
                    widths.push((self.bounded_text(256)?, self.float()?));
                }
                Request::Resize(widths)
            }
            4 => Request::Move(
                self.bounded_text(256)?,
                self.option(|d| d.bounded_text(256))?,
            ),
            5 => Request::Sort(self.bounded_text(256)?, self.option(Self::table_direction)?),
            6 => Request::Copy(self.table_selection()?),
            _ => return Err(DecodeError::Malformed),
        };
        if request.is_valid() {
            Ok(request)
        } else {
            Err(DecodeError::Malformed)
        }
    }
}
macro_rules! entry {
    ($name:ident, $ty:ty, $limit:expr, $method:ident) => {
        pub fn $name(bytes: &[u8]) -> Result<$ty, DecodeError> {
            if bytes.len() > $limit {
                return Err(DecodeError::LimitExceeded);
            }
            let mut decoder = Decoder(Cursor::new(bytes));
            let value = decoder.$method()?;
            if decoder.remaining() != 0 {
                return Err(DecodeError::Malformed);
            }
            Ok(value)
        }
    };
}
entry!(
    decode_table_config,
    Config,
    MAX_SCHEMA_TEXT_BYTES + 16_384,
    table_config
);
entry!(decode_table_cell, Cell, MAX_COPY_BYTES + 272, table_cell);
entry!(decode_table_command, Command, 1024, table_command);
entry!(
    decode_table_request,
    Request,
    MAX_COLUMNS * 272 + 32,
    table_request
);
