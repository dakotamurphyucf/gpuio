use crate::{HandlerId, NodeId, WindowId, v1::*};
use binprot::BinProtRead;
use std::io::{Cursor, Read};

mod button;
pub use button::decode_button_config;
mod split_group;
mod split_group_appearance;
pub use split_group::{decode_split_group_config, decode_split_group_snapshot};
pub use split_group_appearance::decode_split_group_appearance;
mod split_button;
pub use split_button::decode_split_button_config;
mod checkable;
mod command_binding;
pub use checkable::{decode_radio_position, decode_tab_order};
pub use command_binding::{decode_command_binding_config, decode_command_binding_observation};
mod choice_picker;
mod input;
mod input_format;
mod input_validation;
mod text_area_layout;
pub use input_format::decode_input_format;
pub use input_validation::decode_input_validation_source;
mod link;
pub use choice_picker::{
    decode_choice_picker_config, decode_choice_picker_event, decode_choice_picker_presentation,
};
mod text_content;
mod text_shimmer;
pub use input::{decode_input_config, decode_input_event};
pub use link::decode_link_config;
pub use text_content::decode_text_content;
pub use text_shimmer::decode_text_shimmer_config;
mod highlight;
pub use highlight::{decode_highlight_config, decode_highlight_observation};
mod document_diff;
pub use document_diff::{decode_document_diff_config, decode_document_diff_event};
mod accessibility;
mod chart_data;
mod chart_options;
mod chart_style;
mod chart_view;
mod list_input;
pub use chart_style::decode_chart_style;
pub use chart_view::decode_chart_view_config;
mod chart_resource;
mod chart_sampling;
mod chart_selection;
pub use chart_options::decode_chart_options;
pub use chart_resource::{decode_chart_request, decode_chart_response};
pub use chart_sampling::decode_chart_sampling;
pub use chart_selection::decode_chart_selection;
mod desktop;
mod notification;
pub use chart_data::decode_chart_data;
pub use desktop::{decode_desktop_launch, decode_desktop_request};
pub use notification::{
    decode_notification_event, decode_notification_request, decode_notification_response,
};
mod avatar;
mod calendar;
mod carousel;
mod carousel_track;
pub use carousel_track::{decode_carousel_track_config, decode_carousel_track_request};
mod document_actions;
mod document_profile;
mod table;
mod table_header;
pub use carousel::{decode_carousel_config, decode_carousel_request};
pub use table::{
    decode_table_cell, decode_table_command, decode_table_config, decode_table_request,
};
pub use table_header::decode_table_header_target;
mod color_input;
pub use calendar::{
    decode_calendar_command, decode_calendar_config, decode_calendar_constraints,
    decode_calendar_event, decode_calendar_response, decode_calendar_selection,
};
pub use color_input::{
    decode_color_command, decode_color_config, decode_color_event, decode_color_response,
};
mod loading;
mod spinner;
pub use spinner::decode_spinner_config;
mod progress_presentation;
pub use progress_presentation::decode_progress_presentation;
mod navigation_stack;
pub use navigation_stack::decode_navigation_stack;
mod number_input;
mod otp_input;
pub use otp_input::{
    decode_otp_input_command, decode_otp_input_config, decode_otp_input_event,
    decode_otp_input_response,
};
mod numeric;
pub use number_input::{
    decode_number_input_command, decode_number_input_config, decode_number_input_event,
    decode_number_input_response,
};
mod calendar_content;
mod calendar_presentation;
pub use calendar_content::decode_calendar_content;
mod color_presentation;
mod number_presentation;
mod otp_presentation;
mod rating;
mod reveal;
mod scrollbar;
pub use scrollbar::decode_scrollbar_config;
mod slider;
mod slider_presentation;
pub use accessibility::decode_accessibility;
pub use numeric::decode_numeric_domain;
pub use slider::{decode_slider_command, decode_slider_config, decode_slider_event};
mod container_query;
pub use container_query::decode_container_query;
mod animation_program;
pub use animation_program::decode_animation_program;
mod canvas;
mod canvas_view;
pub use canvas::decode_canvas_scene;
pub use canvas_view::decode_canvas_view_config;

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
        self.bounded_text(MAX_TEXT_BYTES)
    }

    fn bounded_text(&mut self, maximum: usize) -> Result<String, DecodeError> {
        let count = self.count(maximum)?;
        let start = self.0.position() as usize;
        let value = std::str::from_utf8(&self.0.get_ref()[start..start + count])
            .map_err(|_| DecodeError::Malformed)?
            .to_owned();
        self.0.set_position((start + count) as u64);
        Ok(value)
    }

    fn extension_payload(
        &mut self,
        maximum: usize,
    ) -> Result<crate::extension::Payload, DecodeError> {
        let count = self.count(maximum)?;
        let start = self.0.position() as usize;
        let payload = self.0.get_ref()[start..start + count].to_vec();
        self.0.set_position((start + count) as u64);
        Ok(crate::extension::Payload(payload))
    }

    fn extension_config(&mut self) -> Result<crate::extension::Config, DecodeError> {
        use crate::extension::*;
        let config = Config {
            schema: Schema {
                name: self.bounded_text(128)?,
                version: self.int()?,
                fingerprint: self.bounded_text(64)?,
            },
            generation: self.int()?,
            label: self.bounded_text(1024)?,
            disabled: self.boolean()?,
            properties: self.extension_payload(MAX_PROPERTIES)?,
            command: self.option(|decoder| {
                Ok(Command {
                    sequence: decoder.int()?,
                    payload: decoder.extension_payload(MAX_MESSAGE)?,
                })
            })?,
        };
        if config.is_valid() {
            Ok(config)
        } else {
            Err(DecodeError::Malformed)
        }
    }

    fn file_path(&mut self) -> Result<crate::file_path::FilePath, DecodeError> {
        let count = self.count(crate::file_path::MAX_PATH_BYTES)?;
        let start = self.0.position() as usize;
        let bytes = self.0.get_ref()[start..start + count].to_vec();
        self.0.set_position((start + count) as u64);
        crate::file_path::FilePath::new(bytes).map_err(|_| DecodeError::Malformed)
    }

    fn resource(&mut self) -> Result<crate::ResourceId, DecodeError> {
        crate::ResourceId::from_parts(self.int()?, self.int()?).ok_or(DecodeError::Malformed)
    }
    fn animation_targets(&mut self) -> Result<Vec<crate::animation::Target>, DecodeError> {
        use crate::animation::{MAX_TARGETS, Property, Target};
        let count = self.count(MAX_TARGETS)?;
        (0..count)
            .map(|_| {
                let property = match self.tag()? {
                    0 => Property::Width,
                    1 => Property::Height,
                    2 => Property::Top,
                    3 => Property::Right,
                    4 => Property::Bottom,
                    5 => Property::Left,
                    6 => Property::Opacity,
                    7 => Property::TopLeftRadius,
                    8 => Property::TopRightRadius,
                    9 => Property::BottomLeftRadius,
                    10 => Property::BottomRightRadius,
                    11 => Property::OpacityFactor,
                    _ => return Err(DecodeError::Malformed),
                };
                Ok(Target {
                    property,
                    value: self.float()?,
                })
            })
            .collect()
    }
    fn animation_config(&mut self) -> Result<crate::animation::Config, DecodeError> {
        use crate::animation::Config;
        let generation = self.int()?;
        let targets = self.animation_targets()?;
        let initial = match self.tag()? {
            0 => None,
            1 => Some(self.animation_targets()?),
            _ => return Err(DecodeError::Malformed),
        };
        let duration_ms = self.int()?;
        let delay_ms = self.int()?;
        let easing = self.animation_easing()?;
        let repeat = self.animation_repeat()?;
        let config = Config {
            generation,
            targets,
            initial,
            duration_ms,
            delay_ms,
            easing,
            repeat,
        };
        if config.is_valid() {
            Ok(config)
        } else {
            Err(DecodeError::Malformed)
        }
    }
    fn image_error(&mut self) -> Result<ImageError, DecodeError> {
        Ok(match self.tag()? {
            0 => ImageError::WrongApplication,
            1 => ImageError::Released,
            2 => ImageError::InvalidData,
            3 => ImageError::Unsupported,
            4 => ImageError::ResourceLimit,
            5 => ImageError::NativeFailure,
            _ => return Err(DecodeError::Malformed),
        })
    }
    fn image_config(&mut self) -> Result<ImageConfig, DecodeError> {
        let source = match self.tag()? {
            0 => ImageSource::Reference(self.resource()?),
            1 => ImageSource::Unavailable(self.image_error()?),
            _ => return Err(DecodeError::Malformed),
        };
        let fit = match self.tag()? {
            0 => ImageFit::Fill,
            1 => ImageFit::Contain,
            2 => ImageFit::Cover,
            3 => ImageFit::ScaleDown,
            4 => ImageFit::None,
            _ => return Err(DecodeError::Malformed),
        };
        let label = self.option(|decoder| decoder.bounded_text(4096))?;
        let config = ImageConfig { source, fit, label };
        if !config.is_valid() {
            return Err(DecodeError::Malformed);
        }
        Ok(config)
    }
    fn document_config(&mut self) -> Result<crate::document::Config, DecodeError> {
        use crate::document::{Config, Layout, Mode};
        let config = Config {
            source: self.option(Self::resource)?,
            mode: match self.tag()? {
                0 => Mode::Markdown,
                1 => Mode::Code(self.bounded_text(64)?),
                2 => Mode::Diff,
                3 => Mode::Html,
                _ => return Err(DecodeError::Malformed),
            },
            dark: self.boolean()?,
            layout: match self.tag()? {
                0 => Layout::Flow,
                1 => Layout::Viewport(self.float()?),
                _ => return Err(DecodeError::Malformed),
            },
            label: self.bounded_text(1024)?,
            path: self.option(|d| d.bounded_text(4096))?,
            line_numbers: self.boolean()?,
            initially_collapsed: self.boolean()?,
            search: self.bounded_text(4096)?,
            images: self.list(128, |d| {
                let url = d.bounded_text(4096)?;
                let source = match d.tag()? {
                    0 => ImageSource::Reference(d.resource()?),
                    1 => ImageSource::Unavailable(d.image_error()?),
                    _ => return Err(DecodeError::Malformed),
                };
                Ok((url, source))
            })?,
        };
        if !config.is_valid() {
            return Err(DecodeError::Malformed);
        }
        Ok(config)
    }
    fn document(&mut self) -> Result<crate::document::Request, DecodeError> {
        use crate::document::{MAX_CHUNK_BYTES, Request, Status, Update};
        Ok(match self.tag()? {
            0 => Request::Create,
            1 => Request::Begin(Update {
                id: self.resource()?,
                base: self.int()?,
                revision: self.int()?,
                generation: self.int()?,
                from_byte: self.int()?,
                suffix_bytes: self.int()?,
                status: match self.tag()? {
                    0 => Status::Streaming,
                    1 => Status::Complete,
                    2 => Status::Cancelled,
                    _ => return Err(DecodeError::Malformed),
                },
            }),
            2 => {
                let id = self.resource()?;
                let revision = self.int()?;
                let offset = self.int()?;
                let length = self.count(MAX_CHUNK_BYTES)?;
                let start = self.0.position() as usize;
                let data = self.0.get_ref()[start..start + length].to_vec();
                self.0.set_position((start + length) as u64);
                Request::Chunk(
                    id,
                    revision,
                    offset,
                    crate::asset::Chunk::new(data).map_err(|_| DecodeError::LimitExceeded)?,
                )
            }
            3 => Request::Publish(self.resource()?, self.int()?),
            4 => Request::Abort(self.resource()?, self.int()?),
            5 => Request::Release(self.resource()?),
            _ => return Err(DecodeError::Malformed),
        })
    }
    fn asset(&mut self) -> Result<crate::asset::Request, DecodeError> {
        use crate::asset::{Chunk, Format, MAX_CHUNK_BYTES, Request};
        Ok(match self.tag()? {
            0 => {
                let format = match self.tag()? {
                    0 => Format::Png,
                    1 => Format::Jpeg,
                    2 => Format::Webp,
                    3 => Format::Gif,
                    4 => Format::Svg,
                    5 => Format::Bmp,
                    6 => Format::Tiff,
                    7 => Format::Ico,
                    8 => Format::Pnm,
                    _ => return Err(DecodeError::Malformed),
                };
                Request::Begin(format, self.int()?)
            }
            1 => {
                let id = self.resource()?;
                let offset = self.int()?;
                let length = self.count(MAX_CHUNK_BYTES)?;
                let start = self.0.position() as usize;
                let data = self.0.get_ref()[start..start + length].to_vec();
                self.0.set_position((start + length) as u64);
                Request::Append(
                    id,
                    offset,
                    Chunk::new(data).map_err(|_| DecodeError::LimitExceeded)?,
                )
            }
            2 => Request::Finish(self.resource()?),
            3 => Request::Release(self.resource()?),
            _ => return Err(DecodeError::Malformed),
        })
    }

    fn file_dialog(&mut self) -> Result<FileDialogConfig, DecodeError> {
        let config = match self.tag()? {
            0 => FileDialogConfig::Open(OpenFileConfig {
                selection: match self.tag()? {
                    0 => FileSelection::Files,
                    1 => FileSelection::Directories,
                    2 => FileSelection::FilesAndDirectories,
                    _ => return Err(DecodeError::Malformed),
                },
                multiple: self.boolean()?,
                title: self.bounded_text(4096)?,
                accept_label: self.bounded_text(4096)?,
                directory: self.option(Self::file_path)?,
            }),
            1 => FileDialogConfig::Save(SaveFileConfig {
                directory: self.file_path()?,
                suggested_name: self.bounded_text(255)?,
                title: self.bounded_text(4096)?,
                accept_label: self.bounded_text(4096)?,
            }),
            2 => FileDialogConfig::Capabilities,
            _ => return Err(DecodeError::Malformed),
        };
        if config.is_valid() {
            Ok(config)
        } else {
            Err(DecodeError::Malformed)
        }
    }

    fn drag_kind(&mut self) -> Result<crate::drag_drop::CustomKind, DecodeError> {
        crate::drag_drop::CustomKind::new(self.bounded_text(128)?)
            .map_err(|_| DecodeError::Malformed)
    }

    fn drag_payload(&mut self) -> Result<crate::drag_drop::Payload, DecodeError> {
        use crate::drag_drop::{File, MAX_DATA_BYTES, MAX_FILES, Payload};
        let payload = match self.tag()? {
            0 => Payload::text(self.bounded_text(MAX_DATA_BYTES)?),
            1 => {
                let count = self.count(MAX_FILES)?;
                if count == 0 {
                    return Err(DecodeError::Malformed);
                }
                let mut files = Vec::with_capacity(count);
                let mut remaining_bytes = MAX_DATA_BYTES;
                for _ in 0..count {
                    let length =
                        self.count(remaining_bytes.min(crate::file_path::MAX_PATH_BYTES))?;
                    let start = self.0.position() as usize;
                    let bytes = self.0.get_ref()[start..start + length].to_vec();
                    self.0.set_position((start + length) as u64);
                    let path = crate::file_path::FilePath::new(bytes)
                        .map_err(|_| DecodeError::Malformed)?;
                    files.push(File {
                        path,
                        is_directory: self.option(Self::boolean)?,
                    });
                    remaining_bytes -= length;
                }
                Payload::files(files)
            }
            2 => {
                let kind = self.drag_kind()?;
                let length = self.count(MAX_DATA_BYTES)?;
                let start = self.0.position() as usize;
                let data = self.0.get_ref()[start..start + length].to_vec();
                self.0.set_position((start + length) as u64);
                Payload::custom(kind, data)
            }
            _ => return Err(DecodeError::Malformed),
        };
        payload.map_err(|_| DecodeError::Malformed)
    }

    fn drag_source(&mut self) -> Result<crate::drag_drop::Source, DecodeError> {
        crate::drag_drop::Source::new(
            self.bounded_text(4096)?,
            self.drag_payload()?,
            self.boolean()?,
            self.boolean()?,
        )
        .map_err(|_| DecodeError::Malformed)
    }

    fn drag_target(&mut self) -> Result<crate::drag_drop::Target, DecodeError> {
        use crate::drag_drop::{Format, MAX_FORMATS, Target};
        let label = self.bounded_text(4096)?;
        let count = self.count(MAX_FORMATS)?;
        let mut formats = Vec::with_capacity(count);
        for _ in 0..count {
            formats.push(match self.tag()? {
                0 => Format::Text,
                1 => Format::Files,
                2 => Format::Custom(self.drag_kind()?),
                _ => return Err(DecodeError::Malformed),
            });
        }
        Target::new(label, formats, self.boolean()?).map_err(|_| DecodeError::Malformed)
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
            2 => Ok(Fill::LinearGradientIn(
                self.int()?,
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
    fn grid_edge(&mut self) -> Result<crate::grid_location::Edge, DecodeError> {
        use crate::grid_location::Edge;
        let edge = match self.tag()? {
            0 => Edge::Auto,
            1 => Edge::Line(self.int()?),
            2 => Edge::Span(self.int()?),
            _ => return Err(DecodeError::Malformed),
        };
        if edge.valid() {
            Ok(edge)
        } else {
            Err(DecodeError::Malformed)
        }
    }
    fn grid_axis(&mut self) -> Result<crate::grid_location::Axis, DecodeError> {
        Ok(crate::grid_location::Axis {
            start: self.grid_edge()?,
            end: self.grid_edge()?,
        })
    }
    fn grid_location(&mut self) -> Result<crate::grid_location::Location, DecodeError> {
        Ok(crate::grid_location::Location {
            column: self.grid_axis()?,
            row: self.grid_axis()?,
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
            65 => Field::Inert(self.boolean()?),
            66 => Field::PointerOcclusion(self.int()?),
            67 => Field::BorderStyle(self.int()?),
            68 => Field::AspectRatio(self.float()?),
            69 => Field::Disabled(self.boolean()?),
            70 => Field::GridLocation(self.grid_location()?),
            _ => return Err(DecodeError::Malformed),
        })
    }

    fn table_presentation(&mut self, row: bool) -> Result<Vec<Style>, DecodeError> {
        let mut remaining = crate::table_presentation::MAX_DECLARATIONS;
        let styles = self.list(remaining, |decoder| {
            let style = decoder.style()?;
            let count = match &style {
                Style::Fields(fields) | Style::State(_, fields) => fields.len(),
                _ => return Err(DecodeError::Malformed),
            };
            remaining = remaining
                .checked_sub(count)
                .ok_or(DecodeError::LimitExceeded)?;
            Ok(style)
        })?;
        if crate::table_presentation::valid_scope(&styles, row) {
            Ok(styles)
        } else {
            Err(DecodeError::Malformed)
        }
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

    fn shortcut(&mut self) -> Result<Shortcut, DecodeError> {
        Ok(Shortcut {
            key: self.bounded_text(256)?,
            modifiers: self.list(5, |this| {
                Ok(match this.tag()? {
                    0 => ShortcutModifier::Primary,
                    1 => ShortcutModifier::Control,
                    2 => ShortcutModifier::Alt,
                    3 => ShortcutModifier::Shift,
                    4 => ShortcutModifier::Super,
                    _ => return Err(DecodeError::Malformed),
                })
            })?,
            priority: match self.tag()? {
                0 => ShortcutPriority::NativeFirst,
                1 => ShortcutPriority::Override,
                _ => return Err(DecodeError::Malformed),
            },
            text_input: match self.tag()? {
                0 => ShortcutTextInput::ModifiedOnly,
                1 => ShortcutTextInput::Always,
                2 => ShortcutTextInput::Never,
                _ => return Err(DecodeError::Malformed),
            },
            during_composition: self.boolean()?,
        })
    }
    fn bounded_metadata_text(
        &mut self,
        limit: usize,
        bytes: &mut usize,
    ) -> Result<String, DecodeError> {
        let value = self.bounded_text(limit.min(262144usize.saturating_sub(*bytes)))?;
        *bytes += value.len();
        Ok(value)
    }
    fn palette_layout(&mut self) -> Result<crate::palette_layout::Config, DecodeError> {
        use crate::palette_layout::{Config, Entry};
        let size = self.count(1024)?;
        let mut entries = Vec::with_capacity(size);
        let mut remaining = 1024usize;
        let mut bytes = 0;
        for _ in 0..size {
            entries.push(match self.tag()? {
                0 => {
                    remaining = remaining.checked_sub(1).ok_or(DecodeError::Malformed)?;
                    Entry::Command(self.int()?)
                }
                1 => {
                    let id = self.bounded_metadata_text(256, &mut bytes)?;
                    let label = self.option(|this| this.bounded_metadata_text(4096, &mut bytes))?;
                    let count = self.count(remaining)?;
                    remaining -= count;
                    let mut commands = Vec::with_capacity(count);
                    for _ in 0..count {
                        commands.push(self.int()?);
                    }
                    Entry::Group(id, label, commands)
                }
                2 => Entry::Separator,
                _ => return Err(DecodeError::Malformed),
            });
        }
        let config = Config(entries);
        if !config.is_valid() {
            return Err(DecodeError::Malformed);
        }
        Ok(config)
    }
    fn palette_options(&mut self) -> Result<crate::palette_options::Config, DecodeError> {
        use crate::palette_options::{Config, Escape, Keywords, Presentation, Search};
        let search = match self.tag()? {
            0 => Search::AllTerms,
            1 => Search::Substring,
            2 => Search::Unfiltered,
            3 => Search::External,
            _ => return Err(DecodeError::Malformed),
        };
        let searchable = self.boolean()?;
        let escape = match self.tag()? {
            0 => Escape::Dismiss,
            1 => Escape::ClearQueryFirst,
            _ => return Err(DecodeError::Malformed),
        };
        let size = self.count(1024)?;
        let mut bytes = 0;
        let mut keywords = Vec::with_capacity(size);
        for _ in 0..size {
            let command = self.bounded_metadata_text(256, &mut bytes)?;
            let size = self.count(64)?;
            let mut words = Vec::with_capacity(size);
            for _ in 0..size {
                words.push(self.bounded_metadata_text(4096, &mut bytes)?);
            }
            keywords.push(Keywords { command, words });
        }
        let presentation = match self.tag()? {
            0 => Presentation::Modal,
            1 => Presentation::Embedded,
            _ => return Err(DecodeError::Malformed),
        };
        let config = Config {
            search,
            searchable,
            escape,
            keywords,
            presentation,
        };
        if !config.is_valid() {
            return Err(DecodeError::Malformed);
        }
        Ok(config)
    }
    fn menu_definition(
        &mut self,
        depth: usize,
        count: &mut usize,
        bytes: &mut usize,
    ) -> Result<MenuDefinition, DecodeError> {
        if depth > 8 {
            return Err(DecodeError::LimitExceeded);
        }
        let label = self.bounded_metadata_text(4096, bytes)?;
        let disabled = self.boolean()?;
        let size = self.count(1024usize.saturating_sub(*count))?;
        *count += size;
        let mut items = Vec::with_capacity(size);
        for _ in 0..size {
            items.push(match self.tag()? {
                0 => MenuItem::Command(self.bounded_metadata_text(256, bytes)?),
                1 => MenuItem::Separator,
                2 => MenuItem::Submenu(self.menu_definition(depth + 1, count, bytes)?),
                3 => MenuItem::Label(self.bounded_metadata_text(4096, bytes)?),
                _ => return Err(DecodeError::Malformed),
            });
        }
        Ok(MenuDefinition {
            label,
            disabled,
            items,
        })
    }
    fn menu_config(&mut self) -> Result<MenuConfig, DecodeError> {
        let presentation = match self.tag()? {
            0 => MenuPresentation::Button,
            1 => MenuPresentation::Context,
            2 => MenuPresentation::Bar,
            3 => MenuPresentation::PlatformBar,
            4 => MenuPresentation::EditorContext,
            _ => return Err(DecodeError::Malformed),
        };
        let size = self.count(32)?;
        let mut count = 0;
        let mut bytes = 0;
        let mut menus = Vec::with_capacity(size);
        for _ in 0..size {
            menus.push(self.menu_definition(1, &mut count, &mut bytes)?);
        }
        let config = MenuConfig {
            presentation,
            menus,
        };
        if !config.is_valid() {
            return Err(DecodeError::Malformed);
        }
        Ok(config)
    }

    fn list_config(&mut self) -> Result<crate::list::Config, DecodeError> {
        let config = crate::list::Config {
            estimated_height: self.float()?,
            overscan: self.float()?,
            max_active: self.int()?,
            scroll_policy: match self.tag()? {
                0 => crate::list::ScrollPolicy::KeepPosition,
                1 => crate::list::ScrollPolicy::FollowTailWhenAtEnd,
                _ => return Err(DecodeError::Malformed),
            },
            scrollbar: self.boolean()?,
            managed: self.boolean()?,
        };
        if !config.is_valid() {
            return Err(DecodeError::Malformed);
        }
        Ok(config)
    }

    fn list_order(&mut self) -> Result<crate::list::Order, DecodeError> {
        let order = crate::list::Order {
            revision: self.int()?,
            runs: self.list(crate::list::MAX_ID_RUNS, |decoder| {
                Ok(crate::list::IdRun {
                    first: decoder.int()?,
                    count: decoder.int()?,
                })
            })?,
        };
        if !order.is_valid() {
            return Err(DecodeError::Malformed);
        }
        Ok(order)
    }

    fn list_scroll(&mut self) -> Result<crate::list::ScrollRequest, DecodeError> {
        let request = crate::list::ScrollRequest {
            serial: self.int()?,
            target: match self.tag()? {
                0 => crate::list::ScrollTarget::Offset(self.int()?, self.float()?),
                1 => crate::list::ScrollTarget::Reveal(self.int()?),
                2 => crate::list::ScrollTarget::End,
                3 => crate::list::ScrollTarget::FocusTreeRow(self.int()?),
                _ => return Err(DecodeError::Malformed),
            },
        };
        let valid_target = match request.target {
            crate::list::ScrollTarget::Offset(row, offset) => {
                row > 0 && (0.0..=1_000_000.0).contains(&offset)
            }
            crate::list::ScrollTarget::Reveal(row)
            | crate::list::ScrollTarget::FocusTreeRow(row) => row > 0,
            crate::list::ScrollTarget::End => true,
        };
        if request.serial < 1 || !valid_target {
            return Err(DecodeError::Malformed);
        }
        Ok(request)
    }

    fn command_config(&mut self) -> Result<CommandConfig, DecodeError> {
        Ok(CommandConfig {
            id: self.text()?,
            generation: self.int()?,
            label: self.text()?,
            enabled: self.boolean()?,
            checked: self.option(Self::boolean)?,
            shortcuts: self.list(4, Self::shortcut)?,
            target: match self.tag()? {
                0 => CommandTarget::Callback,
                1 => CommandTarget::Native(match self.tag()? {
                    0 => NativeCommand::Copy,
                    1 => NativeCommand::Cut,
                    2 => NativeCommand::Paste,
                    3 => NativeCommand::SelectAll,
                    4 => NativeCommand::Undo,
                    5 => NativeCommand::Redo,
                    _ => return Err(DecodeError::Malformed),
                }),
                _ => return Err(DecodeError::Malformed),
            },
        })
    }

    fn toast_motion(&mut self) -> Result<crate::toast_motion::Config, DecodeError> {
        let config = crate::toast_motion::Config {
            spring: crate::animation::Spring {
                stiffness: self.float()?,
                damping: self.float()?,
                mass: self.float()?,
                epsilon: self.float()?,
                max_duration_ms: self.int()?,
            },
            enter_ms: self.int()?,
            exit_ms: self.int()?,
            offset: self.float()?,
        };
        if config.is_valid() {
            Ok(config)
        } else {
            Err(DecodeError::Malformed)
        }
    }
    fn toast_layering(&mut self) -> Result<crate::toast_layering::Layering, DecodeError> {
        let config = crate::toast_layering::Layering {
            peek: self.float()?,
            gap: self.float()?,
            width_step: self.float()?,
            visible: self.int()?,
        };
        if !config.is_valid() {
            return Err(DecodeError::Malformed);
        }
        Ok(config)
    }
    fn toast_placement(&mut self) -> Result<crate::toast_placement::Placement, DecodeError> {
        use crate::toast_placement::{Anchor, Placement};
        let anchor = match self.tag()? {
            0 => Anchor::TopLeft,
            1 => Anchor::TopRight,
            2 => Anchor::BottomLeft,
            3 => Anchor::BottomRight,
            4 => Anchor::TopCenter,
            5 => Anchor::BottomCenter,
            6 => Anchor::LeftCenter,
            7 => Anchor::RightCenter,
            _ => return Err(DecodeError::Malformed),
        };
        let p = Placement {
            anchor,
            top: self.float()?,
            right: self.float()?,
            bottom: self.float()?,
            left: self.float()?,
        };
        if !p.is_valid() {
            return Err(DecodeError::Malformed);
        }
        Ok(p)
    }
    fn sheet_insets(&mut self) -> Result<crate::sheet_insets::Insets, DecodeError> {
        let insets = crate::sheet_insets::Insets {
            top: self.float()?,
            right: self.float()?,
            bottom: self.float()?,
            left: self.float()?,
        };
        if !insets.is_valid() {
            return Err(DecodeError::Malformed);
        }
        Ok(insets)
    }
    fn placement_geometry(&mut self) -> Result<crate::placement_geometry::Config, DecodeError> {
        use crate::placement_geometry::{Config, Corner, Point};
        let config = Config {
            viewport_margin: self.float()?,
            point: self.option(|r| {
                let corner = match r.tag()? {
                    0 => Corner::TopLeft,
                    1 => Corner::TopRight,
                    2 => Corner::BottomLeft,
                    3 => Corner::BottomRight,
                    _ => return Err(DecodeError::Malformed),
                };
                Ok(Point {
                    corner,
                    x: r.float()?,
                    y: r.float()?,
                })
            })?,
        };
        if !config.is_valid() {
            return Err(DecodeError::Malformed);
        }
        Ok(config)
    }
    fn placement(&mut self) -> Result<Placement, DecodeError> {
        Ok(Placement {
            side: match self.tag()? {
                0 => Side::Top,
                1 => Side::Right,
                2 => Side::Bottom,
                3 => Side::Left,
                _ => return Err(DecodeError::Malformed),
            },
            align: match self.tag()? {
                0 => Align::Start,
                1 => Align::Center,
                2 => Align::End,
                _ => return Err(DecodeError::Malformed),
            },
            offset: self.float()?,
        })
    }
    fn overlay_config(&mut self) -> Result<OverlayConfig, DecodeError> {
        Ok(OverlayConfig {
            kind: match self.tag()? {
                0 => OverlayKind::Dialog,
                1 => OverlayKind::Popover,
                2 => OverlayKind::SheetLeft,
                3 => OverlayKind::SheetRight,
                4 => OverlayKind::SheetTop,
                5 => OverlayKind::SheetBottom,
                6 => OverlayKind::AlertDialog,
                _ => return Err(DecodeError::Malformed),
            },
            label: self.text()?,
            width: self.float()?,
            dismiss_on_escape: self.boolean()?,
            dismiss_on_outside_pointer: self.boolean()?,
        })
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
                    9 => Kind::Combobox,
                    10 => Kind::FocusScope,
                    11 => Kind::Tooltip,
                    12 => Kind::CommandScope,
                    13 => Kind::CommandButton,
                    14 => Kind::Menu,
                    15 => Kind::CommandPalette,
                    16 => Kind::Progress,
                    17 => Kind::Toast,
                    18 => Kind::ToastStack,
                    19 => Kind::PointerArea,
                    20 => Kind::DragSource,
                    21 => Kind::DropTarget,
                    22 => Kind::Image,
                    23 => Kind::Icon,
                    24 => Kind::Animated,
                    25 => Kind::VirtualList,
                    26 => Kind::DocumentView,
                    27 => Kind::TabBar,
                    28 => Kind::TabPanel,
                    29 => Kind::SplitPane,
                    30 => Kind::Extension,
                    31 => Kind::CanvasView,
                    32 => Kind::AnimationProgram,
                    33 => Kind::ContainerQuery,
                    34 => Kind::Loading,
                    35 => Kind::Avatar,
                    36 => Kind::Rating,
                    37 => Kind::Slider,
                    38 => Kind::NumberInput,
                    39 => Kind::OtpInput,
                    40 => Kind::Calendar,
                    41 => Kind::ColorInput,
                    42 => Kind::Panel,
                    43 => Kind::Disclosure,
                    44 => Kind::Accordion,
                    45 => Kind::NavigationStack,
                    46 => Kind::HoverCard,
                    47 => Kind::Carousel,
                    48 => Kind::ChartView,
                    49 => Kind::InputRegion,
                    50 => Kind::HighlightScope,
                    51 => Kind::Link,
                    52 => Kind::Radio,
                    53 => Kind::ChoicePicker,
                    54 => Kind::CarouselTrack,
                    55 => Kind::CarouselTrackGroup,
                    56 => Kind::SplitGroup,
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
            18 => Op::SetMenu(self.node()?, self.menu_config()?),
            24 => Op::SetDragSource(self.node()?, self.drag_source()?),
            25 => Op::SetDropTarget(self.node()?, self.drag_target()?),
            23 => Op::SetPointer(
                self.node()?,
                PointerConfig {
                    label: self.text()?,
                    button: match self.tag()? {
                        0 => PointerButton::Left,
                        1 => PointerButton::Right,
                        2 => PointerButton::Middle,
                        3 => PointerButton::Back,
                        4 => PointerButton::Forward,
                        _ => return Err(DecodeError::Malformed),
                    },
                    disabled: self.boolean()?,
                    prevent_default: self.boolean()?,
                    stop_propagation: self.boolean()?,
                },
            ),
            21 => Op::SetToast(
                self.node()?,
                ToastConfig {
                    label: self.text()?,
                    close_label: self.text()?,
                    timeout_ns: self.option(Self::int)?,
                    politeness: match self.tag()? {
                        0 => ToastPoliteness::Polite,
                        1 => ToastPoliteness::Assertive,
                        _ => return Err(DecodeError::Malformed),
                    },
                },
            ),
            22 => Op::SetToastStack(
                self.node()?,
                ToastStackConfig {
                    label: self.text()?,
                    corner: match self.tag()? {
                        0 => ToastCorner::TopLeft,
                        1 => ToastCorner::TopRight,
                        2 => ToastCorner::BottomLeft,
                        3 => ToastCorner::BottomRight,
                        _ => return Err(DecodeError::Malformed),
                    },
                    width: self.float()?,
                    max_visible: self.int()?,
                },
            ),
            26 => Op::SetImage(self.node()?, self.image_config()?),
            33 => Op::SetDocument(self.node()?, self.document_config()?),
            35 => Op::SetExtension(self.node()?, self.extension_config()?),
            36 => Op::SetCanvas(self.node()?, self.canvas_view_config()?),
            37 => Op::SetAnimationProgram(self.node()?, self.animation_program_config()?),
            38 => Op::SetContainerQuery(self.node()?, self.container_query_config()?),
            39 => Op::SetAccessibility(self.node()?, self.option(|d| d.accessibility_config())?),
            40 => Op::SetLoading(self.node()?, self.loading_config()?),
            41 => Op::SetAvatar(self.node()?, self.avatar_config()?),
            42 => Op::SetRating(self.node()?, self.rating_config()?),
            43 => Op::SetSlider(self.node()?, self.slider_config()?, self.slider_value()?),
            44 => Op::SetNumberInput(self.node()?, self.number_config()?, self.number_value()?),
            45 => {
                let node = self.node()?;
                let config = self.otp_config()?;
                let initial = self.bounded_text(32)?;
                if !config.policy.canonical(&initial) {
                    return Err(DecodeError::Malformed);
                }
                Op::SetOtpInput(node, config, initial)
            }
            48 => Op::SetNavigationStack(self.node()?, self.navigation_stack_config()?),
            49 => Op::SetCarousel(self.node()?, self.carousel_config()?),
            50 => Op::SetTreeInput(self.node()?, self.boolean()?),
            51 => Op::SetTreeMoves(self.node()?, self.boolean()?),
            52 => Op::SetTable(self.node()?, self.table_config()?),
            53 => Op::SetTableCell(self.node()?, self.table_cell()?),
            54 => Op::TableCommand(self.node()?, self.table_command()?),
            55 => Op::SetChart(self.node()?, self.chart_view_config()?),
            56 => Op::SetInputRegion(self.node()?, self.input_config()?),
            57 => Op::SetHighlightScope(self.node()?, self.highlight_config()?),
            59 => Op::SetStyledText(self.node()?, self.text_content()?),
            60 => Op::SetLink(self.node()?, self.link_config()?),
            61 => Op::SetTextShimmer(self.node()?, self.option(Self::text_shimmer_config)?),
            62 => Op::SetCommandBinding(self.node()?, self.option(Self::command_binding_config)?),
            63 => Op::SetNumberInputDraft(self.node()?, self.option(Self::number_initial_draft)?),
            64 => Op::SetRatingAppearance(self.node()?, self.option(Self::rating_appearance)?),
            65 => Op::SetSpinner(self.node()?, self.spinner_config()?),
            66 => Op::SetProgressPresentation(self.node()?, self.progress_presentation()?),
            67 => Op::SetControlAppearance(self.node()?, self.option(Self::control_appearance)?),
            68 => Op::SetTabOrder(self.node()?, self.option(Self::tab_order)?),
            69 => Op::SetButtonPresentation(self.node()?, self.option(Self::button_config)?),
            70 => Op::SetSplitButton(self.node()?, self.option(Self::split_button_config)?),
            71 => Op::SetHoverObserver(self.node()?, self.handler()?),
            72 => Op::SetChoicePicker(self.node()?, Box::new(self.choice_picker_presentation()?)),
            78 => Op::SetTextAreaLayout(self.node()?, self.option(Self::text_area_layout)?),
            79 => Op::SetEditorClearOnEscape(self.node()?, self.boolean()?),
            80 => Op::SetEditorSearchable(self.node()?, self.boolean()?),
            81 => Op::SetOtpAppearance(self.node()?, self.option(Self::otp_appearance)?),
            82 => Op::SetNumberPresentation(self.node()?, self.option(Self::number_presentation)?),
            84 => Op::SetSliderAppearance(self.node()?, self.option(Self::slider_appearance)?),
            85 => Op::SetReveal(self.node()?, self.option(Self::reveal_config)?),
            86 => Op::SetCalendarAppearance(self.node()?, self.option(Self::calendar_appearance)?),
            87 => Op::SetColorPresentation(self.node()?, self.option(Self::color_presentation)?),
            88 => Op::SetPopover(self.node()?, self.boolean()?),
            89 => Op::SetCalendarContent(self.node()?, self.option(Self::calendar_content)?),
            90 => Op::SetOverlayBackdrop(self.node()?, self.option(Self::int)?),
            91 => Op::SetOverlayMotion(self.node()?, self.boolean()?),
            92 => Op::SetTooltipMotion(self.node()?, self.boolean()?),
            93 => Op::SetPlacementGeometry(self.node()?, self.option(Self::placement_geometry)?),
            94 => Op::SetSheetInsets(self.node()?, self.option(Self::sheet_insets)?),
            95 => Op::SetCalendarViewportObserver(self.node()?, self.handler()?),
            96 => Op::SetCarouselTrack(self.node()?, self.carousel_track_config()?),
            97 => {
                Op::SetCarouselTrackMotion(self.node()?, self.option(Self::carousel_track_motion)?)
            }
            98 => Op::SetTabAppearance(self.node()?, self.option(Self::tab_appearance)?),
            99 => Op::SetTabContent(self.node()?, self.option(Self::tab_content)?),
            100 => Op::SetTabViewport(self.node()?, self.option(Self::tab_viewport)?),
            101 => Op::SetTabTrailing(self.node()?, self.boolean()?),
            102 => Op::SetChoiceMenu(self.node()?, self.boolean()?),
            105 => Op::SetToastPlacement(self.node()?, self.option(Self::toast_placement)?),
            106 => Op::SetToastLayering(self.node()?, self.option(Self::toast_layering)?),
            107 => Op::SetToastMotion(self.node()?, self.option(Self::toast_motion)?),
            110 => Op::SetListInput(self.node()?, self.option(Self::list_input_config)?),
            111 => Op::SetTableBehavior(self.node()?, self.option(Self::table_behavior)?),
            112 => Op::SetTableAppearance(self.node()?, self.option(Self::table_appearance)?),
            113 => Op::SetTableHeader(self.node()?, self.option(Self::table_header_target)?),
            114 => Op::SetTableHeaderStyle(self.node()?, self.table_presentation(false)?),
            115 => Op::SetTableRowStyle(self.node()?, self.table_presentation(true)?),
            116 => Op::SetDocumentSelectionFormat(self.node()?, self.boolean()?),
            118 => Op::SetDocumentTextStyle(self.node()?, self.option(Self::document_style)?),
            120 => Op::SetDocumentActions(self.node()?, self.document_actions()?),
            121 => Op::SetDocumentProfile(self.node()?, self.document_profile()?),
            123 => Op::CreateTableText(self.node()?, self.table_cell()?),
            124 => Op::SetTableText(self.node()?, self.table_cell()?),
            125 => Op::SetPaletteOptions(self.node()?, self.option(Self::palette_options)?),
            126 => Op::SetPaletteLayout(self.node()?, self.option(Self::palette_layout)?),
            127 => Op::SetPaletteObserved(self.node()?, self.boolean()?),
            122 => Op::SetWindowRegion(
                self.node()?,
                self.option(|d| {
                    use crate::window_region::{Edge, Region};
                    Ok(match d.tag()? {
                        0 => Region::TitleBar,
                        1 => Region::Exclude,
                        2 => Region::Resize(match d.tag()? {
                            0 => Edge::Top,
                            1 => Edge::Bottom,
                            2 => Edge::Left,
                            3 => Edge::Right,
                            4 => Edge::TopLeft,
                            5 => Edge::TopRight,
                            6 => Edge::BottomLeft,
                            7 => Edge::BottomRight,
                            _ => return Err(DecodeError::Malformed),
                        }),
                        _ => return Err(DecodeError::Malformed),
                    })
                })?,
            ),
            119 => Op::SetDocumentMarkdownOptions(
                self.node()?,
                crate::document::MarkdownOptions {
                    frontmatter: match self.tag()? {
                        0 => crate::document::Frontmatter::Disabled,
                        1 => crate::document::Frontmatter::CodeBlock,
                        2 => crate::document::Frontmatter::DescriptionList,
                        _ => return Err(DecodeError::Malformed),
                    },
                    mdx: self.boolean()?,
                },
            ),
            117 => {
                let node = self.node()?;
                let config = crate::document_preview::Config {
                    epoch: self.int()?,
                    max_lines: self.option(Self::int)?,
                    observe: self.boolean()?,
                };
                if !config.is_valid() {
                    return Err(DecodeError::Malformed);
                }
                Op::SetDocumentPreview(node, config)
            }
            109 => Op::SetListAxis(
                self.node()?,
                match self.tag()? {
                    0 => crate::list::Axis::Vertical,
                    1 => crate::list::Axis::Horizontal,
                    _ => return Err(DecodeError::Malformed),
                },
            ),
            108 => Op::SetScrollbar(
                self.node()?,
                self.option(Self::scrollbar_config)?.map(Box::new),
            ),
            103 => Op::SetTabMotion(self.node()?, self.option(Self::tab_motion)?),
            104 => Op::SetSplitGroup(
                self.node()?,
                self.split_group()?,
                self.split_group_appearance()?,
            ),
            83 => Op::SetNumberStepMode(
                self.node()?,
                match self.tag()? {
                    0 => crate::number_input::StepMode::Native,
                    1 => crate::number_input::StepMode::Application,
                    _ => return Err(DecodeError::Malformed),
                },
            ),
            76 => Op::SetEditorFormat(self.node()?, self.option(Self::input_format_config)?),
            77 => Op::SetEditorValidation(self.node()?, self.option(Self::input_validation_rule)?),
            75 => Op::SetEditorContentHint(
                self.node()?,
                self.option(|decoder| {
                    crate::input_content_hint::Hint::from_tag(decoder.tag()?)
                        .ok_or(DecodeError::Malformed)
                })?,
            ),
            74 => Op::SetEditorFrame(self.node()?, self.option(Self::editor_frame_config)?),
            73 => Op::SetEditorPrivacy(
                self.node()?,
                match self.tag()? {
                    0 => EditorPrivacy::Plain,
                    1 => EditorPrivacy::PasswordHidden,
                    2 => EditorPrivacy::PasswordRevealed,
                    _ => return Err(DecodeError::Malformed),
                },
            ),
            58 => {
                let node = self.node()?;
                let epoch = self.int()?;
                if epoch <= 0 {
                    return Err(DecodeError::Malformed);
                }
                Op::SetDocumentDiff(node, epoch, self.option(Self::document_diff_config)?)
            }
            47 => Op::SetColorInput(
                self.node()?,
                Box::new(self.color_config()?),
                self.color_value()?,
            ),
            46 => Op::SetCalendar(
                self.node()?,
                Box::new(self.calendar_config()?),
                self.calendar_selection()?,
                self.calendar_month()?,
            ),
            34 => {
                let id = self.node()?;
                let config = crate::split::Config {
                    label: self.text()?,
                    axis: match self.tag()? {
                        0 => crate::split::Axis::Horizontal,
                        1 => crate::split::Axis::Vertical,
                        _ => return Err(DecodeError::Malformed),
                    },
                    initial_first: self.float()?,
                    minimum_first: self.float()?,
                    maximum_first: self.float()?,
                    minimum_second: self.float()?,
                    keyboard_step: self.float()?,
                    reset_generation: self.int()?,
                };
                if !config.is_valid() {
                    return Err(DecodeError::Malformed);
                }
                Op::SetSplit(id, config)
            }
            27 => Op::SetAnimation(self.node()?, self.animation_config()?),
            28 => Op::SetListConfig(self.node()?, self.list_config()?),
            29 => Op::SetListOrder(self.node()?, self.list_order()?),
            30 => Op::SetListRows(
                self.node()?,
                self.list(MAX_NODES, |decoder| {
                    Ok(crate::list::Row {
                        id: decoder.int()?,
                        node: decoder.node()?,
                    })
                })?,
            ),
            31 => Op::InvalidateListRows(
                self.node()?,
                self.list(crate::list::MAX_LOGICAL_ROWS, Self::int)?,
            ),
            32 => Op::ScrollList(self.node()?, self.list_scroll()?),
            20 => Op::SetProgress(
                self.node()?,
                ProgressConfig {
                    label: self.text()?,
                    fraction: self.option(Self::float)?,
                },
            ),
            19 => Op::SetPalette(
                self.node()?,
                PaletteConfig {
                    label: self.text()?,
                    placeholder: self.text()?,
                    commands: self.list(1024, Self::text)?,
                    dismiss_on_outside_pointer: self.boolean()?,
                },
            ),
            17 => Op::SetCommandRef(self.node()?, self.text()?),
            16 => Op::SetCommands(self.node()?, self.list(1024, Self::command_config)?),
            15 => Op::SetTooltip(
                self.node()?,
                TooltipConfig {
                    label: self.text()?,
                    width: self.float()?,
                    open_state: match self.tag()? {
                        0 => TooltipOpenState::Managed(self.boolean()?),
                        1 => TooltipOpenState::Controlled(self.boolean()?),
                        _ => return Err(DecodeError::Malformed),
                    },
                    disabled: self.boolean()?,
                    hoverable: self.boolean()?,
                    show_delay_ns: self.int()?,
                    hide_delay_ns: self.int()?,
                    skip_delay_ns: self.int()?,
                },
            ),
            14 => Op::SetPlacement(self.node()?, self.option(Self::placement)?),
            13 => Op::SetOverlay(self.node()?, self.option(Self::overlay_config)?),
            12 => Op::SetFocusScope(
                self.node()?,
                FocusScopeConfig {
                    trap: self.boolean()?,
                    auto_focus: self.boolean()?,
                    restore_focus: self.boolean()?,
                },
            ),
            11 => Op::SetComboboxFilter(
                self.node()?,
                match self.tag()? {
                    0 => ComboboxFilter::Substring,
                    1 => ComboboxFilter::Unfiltered,
                    _ => return Err(DecodeError::Malformed),
                },
            ),
            10 => Op::SetChoiceAppearance(
                self.node()?,
                ChoiceAppearance {
                    popup_width: self.float()?,
                    row_height: self.float()?,
                    max_visible_rows: self.int()?,
                    empty_label: self.text()?,
                    popup_style: self.list(MAX_STYLE_FIELDS, Self::style)?,
                    option_style: self.list(MAX_STYLE_FIELDS, Self::style)?,
                    empty_style: self.list(MAX_STYLE_FIELDS, Self::style)?,
                },
            ),
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
            3 => Control::Radio(
                self.boolean()?,
                self.option(Self::radio_position)?,
                self.boolean()?,
            ),
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

    fn editor_frame_config(&mut self) -> Result<crate::editor_frame::Config, DecodeError> {
        let config = crate::editor_frame::Config {
            clear_label: self.option(Self::text)?,
            loading: self.boolean()?,
            gap: self.float()?,
        };
        if !config.is_valid() {
            return Err(DecodeError::Malformed);
        }
        Ok(config)
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

    fn editor_search_command(&mut self) -> Result<crate::editor_search::Command, DecodeError> {
        use crate::editor_search::{Case, Command, MAX_QUERY_BYTES, Stamp, valid_query};
        Ok(match self.tag()? {
            0 => Command::Read,
            1 => Command::Open(self.boolean()?),
            2 => Command::Close,
            3 => {
                let query = self.bounded_text(MAX_QUERY_BYTES)?;
                if !valid_query(&query) {
                    return Err(DecodeError::Malformed);
                }
                let case = match self.tag()? {
                    0 => Case::Sensitive,
                    1 => Case::AsciiInsensitive,
                    _ => return Err(DecodeError::Malformed),
                };
                Command::SetQuery(query, case)
            }
            tag @ (6 | 7) => {
                let stamp = Stamp {
                    editor_revision: self.int()?,
                    search_revision: self.int()?,
                };
                if !stamp.is_valid() {
                    return Err(DecodeError::Malformed);
                }
                let replacement = self.text()?;
                if replacement.contains('\0') {
                    return Err(DecodeError::Malformed);
                }
                if tag == 6 {
                    Command::ReplaceCurrent(stamp, replacement)
                } else {
                    Command::ReplaceAll(stamp, replacement)
                }
            }
            4 => Command::Next,
            5 => Command::Previous,
            11 => Command::ToggleCase,
            9 => {
                let query = self.bounded_text(MAX_QUERY_BYTES)?;
                if !valid_query(&query) {
                    return Err(DecodeError::Malformed);
                }
                Command::SetQueryText(query)
            }
            10 => Command::SetCase(match self.tag()? {
                0 => Case::Sensitive,
                1 => Case::AsciiInsensitive,
                _ => return Err(DecodeError::Malformed),
            }),
            8 => {
                let activation = self.int()?;
                if activation < 0 {
                    return Err(DecodeError::Malformed);
                }
                Command::CloseAndFocus(activation)
            }
            _ => return Err(DecodeError::Malformed),
        })
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
            5 => EditorCommand::Submit,
            6 => EditorCommand::ReadSnapshot,
            7 => EditorCommand::ReadContentHintStatus,
            8 => EditorCommand::ReadViewport,
            9 => {
                let offset = crate::editor_viewport::Offset {
                    x: self.float()?,
                    y: self.float()?,
                };
                if !offset.is_valid() {
                    return Err(DecodeError::Malformed);
                }
                EditorCommand::ScrollViewport(offset)
            }
            10 => EditorCommand::Search(self.editor_search_command()?),
            11 => {
                let revision = self.int()?;
                if revision < 0 {
                    return Err(DecodeError::Malformed);
                }
                EditorCommand::ReadRangeBounds(revision, self.editor_selection()?)
            }
            _ => return Err(DecodeError::Malformed),
        })
    }
}

/// Decode one message, rejecting trailing data, non-finite numbers and oversized
/// containers before allocating their declared capacity. UTF-8 text is required except for validated native path bytes.
pub fn decode(bytes: &[u8]) -> Result<Message, DecodeError> {
    if bytes.len() > MAX_MESSAGE_BYTES {
        return Err(DecodeError::LimitExceeded);
    }
    let d = &mut Decoder(Cursor::new(bytes));
    let value = match d.tag()? {
        22 => {
            use crate::palette_command::Command;
            let correlation = d.int()?;
            let window = d.window()?;
            let node = d.node()?;
            let handler =
                HandlerId::from_parts(d.int()?, d.int()?).ok_or(DecodeError::Malformed)?;
            let expected = d.option(|d| d.int())?;
            let command = match d.tag()? {
                0 => Command::ReadSnapshot,
                1 => Command::Focus,
                2 => Command::SetQuery(d.text()?),
                3 => Command::Highlight(d.option(|d| d.text())?),
                4 => Command::SetLoading(d.boolean()?),
                5 => {
                    let commands = d.list(1024, |d| d.bounded_text(256))?;
                    let layout = d.option(Decoder::palette_layout)?;
                    if expected.is_none() {
                        return Err(DecodeError::Malformed);
                    }
                    Command::PublishResults(crate::palette_results::Results { commands, layout })
                }
                _ => return Err(DecodeError::Malformed),
            };
            if correlation <= 0 || expected.is_some_and(|r| r <= 0) || !command.is_valid() {
                return Err(DecodeError::Malformed);
            }
            Message::PaletteCommand(correlation, window, node, handler, expected, command)
        }
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
        7 => {
            let correlation = d.int()?;
            if correlation <= 0 {
                return Err(DecodeError::Malformed);
            }
            Message::FileDialog(correlation, d.window()?, d.file_dialog()?)
        }
        8 => {
            let correlation = d.int()?;
            if correlation <= 0 {
                return Err(DecodeError::Malformed);
            }
            Message::Asset(correlation, d.asset()?)
        }
        9 => Message::SetMotion(match d.tag()? {
            0 => crate::animation::Preference::System,
            1 => crate::animation::Preference::Reduce,
            2 => crate::animation::Preference::Full,
            _ => return Err(DecodeError::Malformed),
        }),
        10 => {
            let correlation = d.int()?;
            if correlation <= 0 {
                return Err(DecodeError::Malformed);
            }
            Message::Document(correlation, d.document()?)
        }
        11 => {
            use crate::window::Command;
            let correlation = d.int()?;
            let id = d.window()?;
            let command = match d.tag()? {
                0 => Command::Observe,
                1 => Command::SetTitle(d.text()?),
                2 => Command::Resize(d.float()?, d.float()?),
                3 => Command::Activate,
                4 => Command::Zoom,
                5 => Command::ToggleFullscreen,
                6 => Command::SetEdited(d.boolean()?),
                7 => Command::SetDocument(crate::window::Document {
                    path: d.option(|d| d.file_path())?,
                    edited: d.boolean()?,
                }),
                8 => Command::Minimize,
                9 => Command::FocusedInput,
                10 => Command::HasTextSelection,
                11 => Command::SelectedText(d.int()?),
                12 => Command::ClearTextSelection,
                13 => Command::EndTextSelection,
                _ => return Err(DecodeError::Malformed),
            };
            if correlation <= 0 || !command.is_valid() {
                return Err(DecodeError::Malformed);
            }
            Message::WindowCommand(correlation, id, command)
        }
        12 => {
            let correlation = d.int()?;
            let id = d.window()?;
            let config = crate::window::Config {
                title: d.text()?,
                width: d.float()?,
                height: d.float()?,
                focus: d.boolean()?,
                chrome: match d.tag()? {
                    0 => crate::window::Chrome::Standard,
                    1 => crate::window::Chrome::Hidden,
                    2 => crate::window::Chrome::Custom,
                    _ => return Err(DecodeError::Malformed),
                },
                resizable: d.boolean()?,
                frame: crate::window::Frame {
                    shadow_size: d.float()?,
                    resize_hit_size: d.float()?,
                },
            };
            if correlation <= 0 || !config.is_valid() {
                return Err(DecodeError::Malformed);
            }
            Message::OpenConfigured(correlation, id, config)
        }
        21 => {
            let correlation = d.int()?;
            if correlation <= 0 {
                return Err(DecodeError::Malformed);
            }
            Message::Chart(correlation, d.chart_request()?)
        }
        20 => {
            let correlation = d.int()?;
            if correlation <= 0 {
                return Err(DecodeError::Malformed);
            }
            Message::Notification(correlation, d.notification_request()?)
        }
        19 => {
            let correlation = d.int()?;
            if correlation <= 0 {
                return Err(DecodeError::Malformed);
            }
            Message::Desktop(correlation, d.desktop_request()?)
        }
        18 => {
            let correlation = d.int()?;
            if correlation <= 0 {
                return Err(DecodeError::Malformed);
            }
            Message::ColorInputCommand(correlation, d.window()?, d.node()?, d.color_command()?)
        }
        17 => {
            let correlation = d.int()?;
            if correlation <= 0 {
                return Err(DecodeError::Malformed);
            }
            Message::CalendarCommand(correlation, d.window()?, d.node()?, d.calendar_command()?)
        }
        16 => {
            let correlation = d.int()?;
            if correlation <= 0 {
                return Err(DecodeError::Malformed);
            }
            Message::OtpInputCommand(correlation, d.window()?, d.node()?, d.otp_command()?)
        }
        15 => {
            let correlation = d.int()?;
            if correlation <= 0 {
                return Err(DecodeError::Malformed);
            }
            Message::NumberInputCommand(correlation, d.window()?, d.node()?, d.number_command()?)
        }
        14 => {
            let correlation = d.int()?;
            if correlation <= 0 {
                return Err(DecodeError::Malformed);
            }
            Message::SliderCommand(correlation, d.window()?, d.node()?, d.slider_command()?)
        }
        13 => {
            let correlation = d.int()?;
            if correlation <= 0 {
                return Err(DecodeError::Malformed);
            }
            Message::Canvas(correlation, d.canvas_request()?)
        }
        _ => return Err(DecodeError::Malformed),
    };
    if d.remaining() != 0 {
        return Err(DecodeError::Malformed);
    }
    Ok(value)
}

fn decode_drag_data<T>(
    bytes: &[u8],
    read: impl FnOnce(&mut Decoder<'_>) -> Result<T, DecodeError>,
) -> Result<T, DecodeError> {
    if bytes.len() > MAX_MESSAGE_BYTES {
        return Err(DecodeError::LimitExceeded);
    }
    let mut decoder = Decoder(Cursor::new(bytes));
    let value = read(&mut decoder)?;
    if decoder.remaining() != 0 {
        return Err(DecodeError::Malformed);
    }
    Ok(value)
}

pub(crate) fn decode_drag_source(bytes: &[u8]) -> Result<crate::drag_drop::Source, DecodeError> {
    decode_drag_data(bytes, |decoder| decoder.drag_source())
}

pub(crate) fn decode_drag_target(bytes: &[u8]) -> Result<crate::drag_drop::Target, DecodeError> {
    decode_drag_data(bytes, |decoder| decoder.drag_target())
}

mod control_appearance;
pub use control_appearance::decode_control_appearance;

mod tab_appearance;
mod tab_content;
mod tab_motion;
mod tab_viewport;

mod document_style;
pub use document_style::decode_document_style;
