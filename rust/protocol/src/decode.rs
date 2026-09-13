use crate::{HandlerId, NodeId, WindowId, v1::*};
use binprot::BinProtRead;
use std::io::{Cursor, Read};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DecodeError {
    Malformed,
    LimitExceeded,
}

struct Decoder<'a>(Cursor<&'a [u8]>);

impl Decoder<'_> {
    fn remaining(&self) -> usize {
        self.0.get_ref().len() - self.0.position() as usize
    }

    fn int(&mut self) -> Result<i64, DecodeError> {
        i64::binprot_read(&mut self.0).map_err(|_| DecodeError::Malformed)
    }

    fn float(&mut self) -> Result<f64, DecodeError> {
        let value = f64::binprot_read(&mut self.0).map_err(|_| DecodeError::Malformed)?;
        if value.is_finite() {
            Ok(value)
        } else {
            Err(DecodeError::Malformed)
        }
    }

    fn tag(&mut self) -> Result<u8, DecodeError> {
        let mut byte = [0];
        self.0
            .read_exact(&mut byte)
            .map_err(|_| DecodeError::Malformed)?;
        Ok(byte[0])
    }

    fn count(&mut self, maximum: usize) -> Result<usize, DecodeError> {
        let value = binprot::Nat0::binprot_read(&mut self.0)
            .map_err(|_| DecodeError::Malformed)?
            .0;
        if value > maximum as u64 {
            Err(DecodeError::LimitExceeded)
        } else if value > self.remaining() as u64 {
            // Every supported list element occupies at least one byte. Bound
            // allocations by the actual input as well as the declared limit.
            Err(DecodeError::Malformed)
        } else {
            Ok(value as usize)
        }
    }

    fn text(&mut self) -> Result<String, DecodeError> {
        let count = self.count(MAX_TEXT_BYTES)?;
        let start = self.0.position() as usize;
        let value = std::str::from_utf8(&self.0.get_ref()[start..start + count])
            .map_err(|_| DecodeError::Malformed)?
            .to_owned();
        self.0.set_position((start + count) as u64);
        Ok(value)
    }

    fn window(&mut self) -> Result<WindowId, DecodeError> {
        WindowId::from_parts(self.int()?, self.int()?).ok_or(DecodeError::Malformed)
    }

    fn node(&mut self) -> Result<NodeId, DecodeError> {
        NodeId::from_parts(self.int()?, self.int()?).ok_or(DecodeError::Malformed)
    }

    fn option<T>(
        &mut self,
        f: impl FnOnce(&mut Self) -> Result<T, DecodeError>,
    ) -> Result<Option<T>, DecodeError> {
        match self.tag()? {
            0 => Ok(None),
            1 => f(self).map(Some),
            _ => Err(DecodeError::Malformed),
        }
    }

    fn handler(&mut self) -> Result<Option<HandlerId>, DecodeError> {
        self.option(|d| HandlerId::from_parts(d.int()?, d.int()?).ok_or(DecodeError::Malformed))
    }

    fn length(&mut self) -> Result<Length, DecodeError> {
        match self.tag()? {
            0 => Ok(Length::Px(self.float()?)),
            1 => Ok(Length::Percent(self.float()?)),
            2 => Ok(Length::Auto),
            _ => Err(DecodeError::Malformed),
        }
    }

    fn color(&mut self) -> Result<Color, DecodeError> {
        match self.tag()? {
            0 => Ok(Color::Rgba(self.int()?)),
            1 => Ok(Color::Token(self.int()?)),
            _ => Err(DecodeError::Malformed),
        }
    }

    fn fill(&mut self) -> Result<Fill, DecodeError> {
        match self.tag()? {
            0 => Ok(Fill::Solid(self.color()?)),
            1 => Ok(Fill::LinearGradient(
                self.float()?,
                self.color()?,
                self.float()?,
                self.color()?,
                self.float()?,
            )),
            _ => Err(DecodeError::Malformed),
        }
    }
    fn boolean(&mut self) -> Result<bool, DecodeError> {
        match self.tag()? {
            0 => Ok(false),
            1 => Ok(true),
            _ => Err(DecodeError::Malformed),
        }
    }
    fn shadow(&mut self) -> Result<Shadow, DecodeError> {
        Ok(Shadow {
            color: self.color()?,
            offset_x: self.float()?,
            offset_y: self.float()?,
            blur: self.float()?,
            spread: self.float()?,
            inset: self.boolean()?,
        })
    }
    fn field(&mut self) -> Result<Field, DecodeError> {
        Ok(match self.tag()? {
            0 => Field::Display(self.int()?),
            1 => Field::Visibility(self.int()?),
            2 => Field::Direction(self.int()?),
            3 => Field::Wrap(self.int()?),
            4 => Field::Grow(self.float()?),
            5 => Field::Shrink(self.float()?),
            6 => Field::Basis(self.length()?),
            7 => Field::AlignItems(self.int()?),
            8 => Field::AlignSelf(self.int()?),
            9 => Field::AlignContent(self.int()?),
            10 => Field::JustifyContent(self.int()?),
            11 => Field::RowGap(self.length()?),
            12 => Field::ColumnGap(self.length()?),
            13 => Field::GridColumns(self.int()?),
            14 => Field::GridRows(self.int()?),
            15 => Field::GridColumnMinimum(self.int()?),
            16 => Field::GridRowMinimum(self.int()?),
            17 => Field::Width(self.length()?),
            18 => Field::Height(self.length()?),
            19 => Field::MinWidth(self.length()?),
            20 => Field::MinHeight(self.length()?),
            21 => Field::MaxWidth(self.length()?),
            22 => Field::MaxHeight(self.length()?),
            23 => Field::PaddingTop(self.length()?),
            24 => Field::PaddingRight(self.length()?),
            25 => Field::PaddingBottom(self.length()?),
            26 => Field::PaddingLeft(self.length()?),
            27 => Field::MarginTop(self.length()?),
            28 => Field::MarginRight(self.length()?),
            29 => Field::MarginBottom(self.length()?),
            30 => Field::MarginLeft(self.length()?),
            31 => Field::Position(self.int()?),
            32 => Field::Top(self.length()?),
            33 => Field::Right(self.length()?),
            34 => Field::Bottom(self.length()?),
            35 => Field::Left(self.length()?),
            36 => Field::Background(self.fill()?),
            37 => Field::Foreground(self.color()?),
            38 => Field::Opacity(self.float()?),
            39 => Field::BorderTopWidth(self.float()?),
            40 => Field::BorderRightWidth(self.float()?),
            41 => Field::BorderBottomWidth(self.float()?),
            42 => Field::BorderLeftWidth(self.float()?),
            43 => Field::TopLeftRadius(self.float()?),
            44 => Field::TopRightRadius(self.float()?),
            45 => Field::BottomLeftRadius(self.float()?),
            46 => Field::BottomRightRadius(self.float()?),
            47 => Field::BorderColor(self.color()?),
            48 => Field::Shadows(self.list(8, Self::shadow)?),
            49 => Field::FontSize(self.float()?),
            50 => Field::FontFamily(self.text()?),
            51 => Field::FontWeight(self.int()?),
            52 => Field::TextAlign(self.int()?),
            53 => Field::LineHeight(self.length()?),
            54 => Field::WhiteSpace(self.int()?),
            55 => Field::TextOverflow(self.int()?),
            56 => Field::LineClamp(self.int()?),
            57 => Field::TextDecoration(self.int()?),
            58 => Field::OverflowX(self.int()?),
            59 => Field::OverflowY(self.int()?),
            60 => Field::Cursor(self.int()?),
            61 => Field::PointerEvents(self.boolean()?),
            62 => Field::UserSelect(self.boolean()?),
            63 => Field::SelectionColor(self.color()?),
            64 => Field::AccessibleName(self.text()?),
            _ => return Err(DecodeError::Malformed),
        })
    }

    fn style(&mut self) -> Result<Style, DecodeError> {
        Ok(match self.tag()? {
            0 => Style::Width(self.length()?),
            1 => Style::Height(self.length()?),
            2 => Style::MinWidth(self.length()?),
            3 => Style::MinHeight(self.length()?),
            4 => Style::MaxWidth(self.length()?),
            5 => Style::MaxHeight(self.length()?),
            6 => Style::Padding(self.float()?),
            7 => Style::Gap(self.float()?),
            8 => Style::Grow(self.float()?),
            9 => Style::Shrink(self.float()?),
            10 => Style::Direction(self.int()?),
            11 => Style::Background(self.color()?),
            12 => Style::Foreground(self.color()?),
            13 => Style::FontSize(self.float()?),
            14 => Style::Radius(self.float()?),
            15 => Style::Opacity(self.float()?),
            16 => Style::HoverBackground(self.color()?),
            17 => Style::PressedBackground(self.color()?),
            18 => Style::FocusBackground(self.color()?),
            19 => Style::Fields(self.list(MAX_STYLE_FIELDS, Self::field)?),
            20 => Style::State(self.int()?, self.list(MAX_STYLE_FIELDS, Self::field)?),
            _ => return Err(DecodeError::Malformed),
        })
    }

    fn list<T>(
        &mut self,
        max: usize,
        mut f: impl FnMut(&mut Self) -> Result<T, DecodeError>,
    ) -> Result<Vec<T>, DecodeError> {
        let count = self.count(max)?;
        (0..count).map(|_| f(self)).collect()
    }

    fn op(&mut self) -> Result<Op, DecodeError> {
        Ok(match self.tag()? {
            0 => {
                let id = self.node()?;
                let kind = match self.tag()? {
                    0 => Kind::Container,
                    1 => Kind::Text,
                    2 => Kind::Button,
                    3 => Kind::Input,
                    4 => Kind::Textarea,
                    5 => Kind::Checkbox,
                    6 => Kind::Switch,
                    7 => Kind::RadioGroup,
                    8 => Kind::Select,
                    _ => return Err(DecodeError::Malformed),
                };
                Op::Create(id, kind, self.text()?, self.handler()?)
            }
            1 => Op::Remove(self.node()?),
            2 => Op::SetText(self.node()?, self.text()?),
            3 => Op::SetStyle(self.node()?, self.list(MAX_STYLE_FIELDS, Self::style)?),
            4 => Op::Bind(self.node()?, self.handler()?),
            5 => Op::Splice(
                self.node()?,
                self.int()?,
                self.int()?,
                self.list(MAX_NODES, Self::node)?,
            ),
            6 => Op::SetRoot(self.option(Self::node)?),
            7 => Op::SetEditor(self.node()?, self.editor_config()?),
            8 => Op::SetControl(self.node()?, self.control()?),
            9 => Op::SetChoice(self.node()?, self.choice_config()?),
            _ => return Err(DecodeError::Malformed),
        })
    }

    fn choice_config(&mut self) -> Result<ChoiceConfig, DecodeError> {
        let config = ChoiceConfig {
            label: self.text()?,
            items: self.list(4096, |decoder| {
                Ok(ChoiceItem {
                    id: decoder.text()?,
                    label: decoder.text()?,
                    disabled: decoder.boolean()?,
                })
            })?,
            selected: self.option(Self::text)?,
            disabled: self.boolean()?,
        };
        if !config.is_valid() {
            return Err(DecodeError::Malformed);
        }
        Ok(config)
    }

    fn control(&mut self) -> Result<Control, DecodeError> {
        Ok(match self.tag()? {
            0 => Control::Button(self.boolean()?),
            1 => {
                let state = match self.tag()? {
                    0 => CheckState::Unchecked,
                    1 => CheckState::Checked,
                    2 => CheckState::Indeterminate,
                    _ => return Err(DecodeError::Malformed),
                };
                Control::Checkbox(state, self.boolean()?)
            }
            2 => Control::Switch(self.boolean()?, self.boolean()?),
            _ => return Err(DecodeError::Malformed),
        })
    }

    fn editor_selection(&mut self) -> Result<EditorSelection, DecodeError> {
        let selection = EditorSelection {
            anchor: self.int()?,
            head: self.int()?,
        };
        if !(0..=MAX_TEXT_BYTES as i64).contains(&selection.anchor)
            || !(0..=MAX_TEXT_BYTES as i64).contains(&selection.head)
        {
            return Err(DecodeError::Malformed);
        }
        Ok(selection)
    }

    fn editor_config(&mut self) -> Result<EditorConfig, DecodeError> {
        let config = EditorConfig {
            label: self.text()?,
            placeholder: self.text()?,
            read_only: self.boolean()?,
            disabled: self.boolean()?,
            submit_on_enter: self.boolean()?,
            auto_focus: self.boolean()?,
            min_rows: self.int()?,
            max_rows: self.int()?,
        };
        if !config.is_valid() {
            return Err(DecodeError::Malformed);
        }
        Ok(config)
    }

    fn editor_command(&mut self) -> Result<EditorCommand, DecodeError> {
        Ok(match self.tag()? {
            0 => {
                let text = self.text()?;
                let selection = match self.tag()? {
                    0 => EditorSelectionPolicy::Start,
                    1 => EditorSelectionPolicy::End,
                    2 => EditorSelectionPolicy::Preserve,
                    3 => EditorSelectionPolicy::Select(self.editor_selection()?),
                    _ => return Err(DecodeError::Malformed),
                };
                let undo = match self.tag()? {
                    0 => EditorUndoPolicy::Record,
                    1 => EditorUndoPolicy::Reset,
                    _ => return Err(DecodeError::Malformed),
                };
                let revision = self.option(Self::int)?;
                if revision.is_some_and(|revision| revision < 0) {
                    return Err(DecodeError::Malformed);
                }
                EditorCommand::Replace(text, selection, undo, revision)
            }
            1 => EditorCommand::Select(self.editor_selection()?),
            2 => EditorCommand::Focus,
            3 => EditorCommand::Undo,
            4 => EditorCommand::Redo,
            _ => return Err(DecodeError::Malformed),
        })
    }
}

/// Decode one message, rejecting trailing data, non-finite numbers and oversized
/// containers before allocating their declared capacity. UTF-8 text is required.
pub fn decode(bytes: &[u8]) -> Result<Message, DecodeError> {
    if bytes.len() > MAX_MESSAGE_BYTES {
        return Err(DecodeError::LimitExceeded);
    }
    let d = &mut Decoder(Cursor::new(bytes));
    let value = match d.tag()? {
        0 => Message::Hello(d.int()?, d.int()?),
        1 => Message::Open(d.int()?, d.window()?, d.text()?, d.float()?, d.float()?),
        2 => Message::Close(d.int()?, d.window()?),
        3 => Message::Apply(Transaction {
            window: d.window()?,
            base: d.int()?,
            revision: d.int()?,
            operations: d.list(MAX_OPERATIONS, Decoder::op)?,
        }),
        4 => Message::RequestFrame(d.int()?, d.window()?),
        5 => Message::Shutdown,
        6 => Message::EditorCommand(d.int()?, d.window()?, d.node()?, d.editor_command()?),
        _ => return Err(DecodeError::Malformed),
    };
    if d.remaining() != 0 {
        return Err(DecodeError::Malformed);
    }
    Ok(value)
}
