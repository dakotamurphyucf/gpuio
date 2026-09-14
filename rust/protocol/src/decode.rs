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

    fn shortcut(&mut self) -> Result<Shortcut, DecodeError> {
        Ok(Shortcut {
            key: self.text()?,
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
    fn menu_definition(
        &mut self,
        depth: usize,
        count: &mut usize,
    ) -> Result<MenuDefinition, DecodeError> {
        if depth > 8 {
            return Err(DecodeError::LimitExceeded);
        }
        let label = self.text()?;
        let disabled = self.boolean()?;
        let size = self.count(1024usize.saturating_sub(*count))?;
        *count += size;
        let mut items = Vec::with_capacity(size);
        for _ in 0..size {
            items.push(match self.tag()? {
                0 => MenuItem::Command(self.text()?),
                1 => MenuItem::Separator,
                2 => MenuItem::Submenu(self.menu_definition(depth + 1, count)?),
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
            _ => return Err(DecodeError::Malformed),
        };
        let size = self.count(32)?;
        let mut count = 0;
        let mut menus = Vec::with_capacity(size);
        for _ in 0..size {
            menus.push(self.menu_definition(1, &mut count)?);
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
/// containers before allocating their declared capacity. UTF-8 text is required except for validated native path bytes.
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
