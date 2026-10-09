use super::{DecodeError, Decoder};
use crate::choice_picker::*;
use std::io::Cursor;
#[derive(Default)]
struct Budget {
    items: usize,
    bytes: usize,
}
impl Decoder<'_> {
    fn picker_text(&mut self, budget: &mut Budget, limit: usize) -> Result<String, DecodeError> {
        let value = self.bounded_text(limit.min(MAX_TEXT_BYTES - budget.bytes))?;
        budget.bytes += value.len();
        Ok(value)
    }
    fn picker_items(&mut self, budget: &mut Budget) -> Result<Vec<Item>, DecodeError> {
        let count = self.count(MAX_ITEMS - budget.items)?;
        budget.items += count;
        let mut items = Vec::with_capacity(count);
        for _ in 0..count {
            items.push(Item {
                id: self.picker_text(budget, 256)?,
                label: self.picker_text(budget, 4096)?,
                disabled: self.boolean()?,
            });
        }
        Ok(items)
    }
    pub(super) fn choice_picker_config(&mut self) -> Result<Config, DecodeError> {
        let label = self.bounded_text(1024)?;
        let mut budget = Budget::default();
        let options = match self.tag()? {
            0 => Collection::Flat(self.picker_items(&mut budget)?),
            1 => {
                let count = self.count(MAX_GROUPS)?;
                let mut groups = Vec::with_capacity(count);
                for _ in 0..count {
                    groups.push(Group {
                        id: self.picker_text(&mut budget, 256)?,
                        label: self.picker_text(&mut budget, 1024)?,
                        items: self.picker_items(&mut budget)?,
                    });
                }
                Collection::Grouped(groups)
            }
            _ => return Err(DecodeError::Malformed),
        };
        let selected = match self.tag()? {
            0 => Selection::Single(self.option(|decoder| decoder.bounded_text(256))?),
            1 => {
                let mut selected_budget = Budget::default();
                Selection::Multiple(self.list(MAX_ITEMS, |decoder| {
                    decoder.picker_text(&mut selected_budget, 256)
                })?)
            }
            _ => return Err(DecodeError::Malformed),
        };
        let disabled = self.boolean()?;
        let search = match self.tag()? {
            0 => Search::None,
            1 => Search::Substring,
            2 => Search::Application,
            _ => return Err(DecodeError::Malformed),
        };
        let clearable = self.boolean()?;
        let open_state = match self.tag()? {
            0 => OpenState::Managed(self.boolean()?),
            1 => OpenState::Controlled(self.boolean()?),
            _ => return Err(DecodeError::Malformed),
        };
        let config = Config {
            label,
            options,
            selected,
            disabled,
            search,
            clearable,
            open_state,
            placeholder: self.bounded_text(1024)?,
            search_placeholder: self.bounded_text(1024)?,
        };
        if config.is_valid() {
            Ok(config)
        } else {
            Err(DecodeError::Malformed)
        }
    }
}
pub fn decode_choice_picker_config(bytes: &[u8]) -> Result<Config, DecodeError> {
    if bytes.len() > MAX_CONFIG_BYTES {
        return Err(DecodeError::LimitExceeded);
    }
    let mut decoder = Decoder(Cursor::new(bytes));
    let value = decoder.choice_picker_config()?;
    if decoder.remaining() == 0 {
        Ok(value)
    } else {
        Err(DecodeError::Malformed)
    }
}

impl Decoder<'_> {
    fn picker_query(&mut self) -> Result<Query, DecodeError> {
        let value = Query {
            node: self.node()?,
            snapshot: crate::v1::EditorSnapshot {
                revision: self.int()?,
                text: self.bounded_text(MAX_QUERY_BYTES)?,
                selection: self.editor_selection()?,
                composition: self.option(Self::editor_selection)?,
                focused: self.boolean()?,
            },
        };
        if value.is_valid() {
            Ok(value)
        } else {
            Err(DecodeError::Malformed)
        }
    }

    fn picker_open_reason(&mut self) -> Result<OpenReason, DecodeError> {
        match self.tag()? {
            0 => Ok(OpenReason::Trigger),
            1 => Ok(OpenReason::Keyboard),
            2 => Ok(OpenReason::Escape),
            3 => Ok(OpenReason::OutsidePointer),
            4 => Ok(OpenReason::FocusLeft),
            5 => Ok(OpenReason::Selection),
            _ => Err(DecodeError::Malformed),
        }
    }
    pub(super) fn choice_picker_event(&mut self) -> Result<Event, DecodeError> {
        let value = match self.tag()? {
            0 => Event::SelectionRequested(
                match self.tag()? {
                    0 => Request::Select(self.bounded_text(256)?),
                    1 => Request::Toggle(self.bounded_text(256)?),
                    2 => Request::Clear,
                    _ => return Err(DecodeError::Malformed),
                },
                self.option(Self::picker_query)?,
            ),
            1 => Event::OpenRequested(self.boolean()?, self.picker_open_reason()?),
            2 => Event::Visibility(match self.tag()? {
                0 => Visibility::Snapshot(self.boolean()?),
                1 => {
                    let open = self.boolean()?;
                    let reason = match self.tag()? {
                        0 => VisibilityReason::Interaction(self.picker_open_reason()?),
                        1 => VisibilityReason::Application,
                        2 => VisibilityReason::Unavailable,
                        _ => return Err(DecodeError::Malformed),
                    };
                    Visibility::Changed(open, reason)
                }
                _ => return Err(DecodeError::Malformed),
            }),
            3 => Event::QueryChanged(self.picker_query()?),
            _ => return Err(DecodeError::Malformed),
        };
        if value.is_valid() {
            Ok(value)
        } else {
            Err(DecodeError::Malformed)
        }
    }
}
pub fn decode_choice_picker_event(bytes: &[u8]) -> Result<Event, DecodeError> {
    if bytes.len() > MAX_EVENT_BYTES {
        return Err(DecodeError::LimitExceeded);
    }
    let mut decoder = Decoder(Cursor::new(bytes));
    let event = decoder.choice_picker_event()?;
    if decoder.remaining() == 0 {
        Ok(event)
    } else {
        Err(DecodeError::Malformed)
    }
}

impl Decoder<'_> {
    fn picker_styles(
        &mut self,
        remaining: &mut usize,
    ) -> Result<Vec<crate::v1::Style>, DecodeError> {
        self.list(128, |decoder| {
            let style = decoder.style()?;
            let fields = match &style {
                crate::v1::Style::Fields(fields) | crate::v1::Style::State(_, fields) => fields,
                _ => return Err(DecodeError::Malformed),
            };
            *remaining = remaining
                .checked_sub(fields.len())
                .ok_or(DecodeError::LimitExceeded)?;
            Ok(style)
        })
    }
    pub(super) fn choice_picker_presentation(&mut self) -> Result<Presentation, DecodeError> {
        let config = self.choice_picker_config()?;
        let mut remaining = 128;
        let mut budget = Budget::default();
        let value = Presentation {
            config,
            popup_width: self.float()?,
            max_height: self.float()?,
            estimated_row_height: self.float()?,
            overscan: self.float()?,
            empty_label: self.bounded_text(1024)?,
            popup_style: self.picker_styles(&mut remaining)?,
            option_style: self.picker_styles(&mut remaining)?,
            header_style: self.picker_styles(&mut remaining)?,
            empty_style: self.picker_styles(&mut remaining)?,
            slots: self.list(MAX_SLOTS, |decoder| {
                Ok(match decoder.tag()? {
                    0 => Slot::Trigger,
                    1 => Slot::Query,
                    2 => Slot::Empty,
                    3 => Slot::Footer,
                    4 => Slot::Group(decoder.picker_text(&mut budget, 256)?),
                    5 => Slot::Option(
                        decoder.picker_text(&mut budget, 256)?,
                        match decoder.tag()? {
                            0 => Checkmark::Native,
                            1 => Checkmark::Custom,
                            _ => return Err(DecodeError::Malformed),
                        },
                    ),
                    _ => return Err(DecodeError::Malformed),
                })
            })?,
        };
        if value.has_valid_shape() {
            Ok(value)
        } else {
            Err(DecodeError::Malformed)
        }
    }
}
pub fn decode_choice_picker_presentation(bytes: &[u8]) -> Result<Presentation, DecodeError> {
    if bytes.len() > MAX_PRESENTATION_BYTES {
        return Err(DecodeError::LimitExceeded);
    }
    let mut decoder = Decoder(Cursor::new(bytes));
    let value = decoder.choice_picker_presentation()?;
    if decoder.remaining() == 0 {
        Ok(value)
    } else {
        Err(DecodeError::Malformed)
    }
}
