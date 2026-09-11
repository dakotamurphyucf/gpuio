//! Semantic validation and retained allocation accounting for portable refinements.
use gpuio_protocol::v1::*;
fn bounded(v: f64) -> bool {
    v.is_finite() && v.abs() <= 1_000_000.
}
fn nonnegative(v: f64) -> bool {
    bounded(v) && v >= 0.
}
fn length(v: &Length, negative: bool, auto: bool) -> bool {
    match v {
        Length::Auto => auto,
        Length::Px(n) | Length::Percent(n) => bounded(*n) && (negative || *n >= 0.),
    }
}
fn color(v: &Color) -> Result<(), ErrorCode> {
    match v {
        Color::Rgba(n) if (0..=u32::MAX as i64).contains(n) => Ok(()),
        Color::Token(_) => Err(ErrorCode::UnsupportedCapability),
        Color::Rgba(_) => Err(ErrorCode::Malformed),
    }
}
fn fill(v: &Fill) -> Result<(), ErrorCode> {
    match v {
        Fill::Solid(c) => color(c),
        Fill::LinearGradient(angle, from, start, to, end) => {
            color(from)?;
            color(to)?;
            if angle.is_finite()
                && (0.0..=360.0).contains(angle)
                && start.is_finite()
                && end.is_finite()
                && (0.0..=1.0).contains(start)
                && (*start..=1.0).contains(end)
            {
                Ok(())
            } else {
                Err(ErrorCode::Malformed)
            }
        }
    }
}
pub fn validate_fields(fields: &[Field]) -> Result<(), ErrorCode> {
    if fields.len() > MAX_STYLE_FIELDS {
        return Err(ErrorCode::LimitExceeded);
    }
    for field in fields {
        let valid = match field {
            Field::Display(v) => (0..=3).contains(v),
            Field::Visibility(v)
            | Field::Position(v)
            | Field::WhiteSpace(v)
            | Field::TextOverflow(v) => (0..=1).contains(v),
            Field::Direction(v)
            | Field::TextDecoration(v)
            | Field::OverflowX(v)
            | Field::OverflowY(v) => (0..=3).contains(v),
            Field::Wrap(v)
            | Field::GridColumnMinimum(v)
            | Field::GridRowMinimum(v)
            | Field::TextAlign(v) => (0..=2).contains(v),
            Field::AlignItems(v) | Field::AlignSelf(v) => (0..=6).contains(v),
            Field::AlignContent(v) | Field::JustifyContent(v) => (0..=8).contains(v),
            Field::Cursor(v) => (0..=9).contains(v),
            Field::GridColumns(v) | Field::GridRows(v) | Field::LineClamp(v) => {
                (1..=1024).contains(v)
            }
            Field::FontWeight(v) => (1..=1000).contains(v),
            Field::Width(v)
            | Field::Height(v)
            | Field::MinWidth(v)
            | Field::MinHeight(v)
            | Field::MaxWidth(v)
            | Field::MaxHeight(v)
            | Field::Basis(v) => length(v, false, true),
            Field::PaddingTop(v)
            | Field::PaddingRight(v)
            | Field::PaddingBottom(v)
            | Field::PaddingLeft(v)
            | Field::RowGap(v)
            | Field::ColumnGap(v)
            | Field::LineHeight(v) => length(v, false, false),
            Field::MarginTop(v)
            | Field::MarginRight(v)
            | Field::MarginBottom(v)
            | Field::MarginLeft(v)
            | Field::Top(v)
            | Field::Right(v)
            | Field::Bottom(v)
            | Field::Left(v) => length(v, true, true),
            Field::Grow(v)
            | Field::Shrink(v)
            | Field::BorderTopWidth(v)
            | Field::BorderRightWidth(v)
            | Field::BorderBottomWidth(v)
            | Field::BorderLeftWidth(v)
            | Field::TopLeftRadius(v)
            | Field::TopRightRadius(v)
            | Field::BottomLeftRadius(v)
            | Field::BottomRightRadius(v) => nonnegative(*v),
            Field::FontSize(v) => nonnegative(*v) && *v > 0.,
            Field::Opacity(v) => v.is_finite() && (0.0..=1.0).contains(v),
            Field::Background(v) => {
                fill(v)?;
                true
            }
            Field::Foreground(v) | Field::BorderColor(v) | Field::SelectionColor(v) => {
                color(v)?;
                true
            }
            Field::FontFamily(v) => !v.is_empty() && v.len() <= 256,
            Field::Shadows(shadows) => {
                if shadows.len() > 8 {
                    return Err(ErrorCode::LimitExceeded);
                }
                for shadow in shadows {
                    color(&shadow.color)?;
                    if !bounded(shadow.offset_x)
                        || !bounded(shadow.offset_y)
                        || !nonnegative(shadow.blur)
                        || !bounded(shadow.spread)
                    {
                        return Err(ErrorCode::Malformed);
                    }
                }
                true
            }
            Field::PointerEvents(_) | Field::UserSelect(_) => true,
            Field::AccessibleName(v) => !v.is_empty() && v.len() <= 1024,
        };
        if !valid {
            return Err(ErrorCode::Malformed);
        }
    }
    Ok(())
}
pub fn retained_bytes(style: &Style) -> usize {
    match style {
        Style::Fields(fields) | Style::State(_, fields) => {
            std::mem::size_of_val(fields.as_slice())
                + fields
                    .iter()
                    .map(|field| match field {
                        Field::FontFamily(text) | Field::AccessibleName(text) => text.len(),
                        Field::Shadows(shadows) => std::mem::size_of_val(shadows.as_slice()),
                        _ => 0,
                    })
                    .sum::<usize>()
        }
        _ => 0,
    }
}

fn gpui_length(value: &Length) -> gpui::Length {
    match value {
        Length::Auto => gpui::Length::Auto,
        Length::Px(v) => gpui::px(*v as f32).into(),
        Length::Percent(v) => gpui::relative(*v as f32 / 100.).into(),
    }
}
fn definite(value: &Length) -> gpui::DefiniteLength {
    match value {
        Length::Px(v) => gpui::px(*v as f32).into(),
        Length::Percent(v) => gpui::relative(*v as f32 / 100.),
        Length::Auto => unreachable!("validated definite length"),
    }
}
fn gpui_color(value: &Color) -> gpui::Hsla {
    match value {
        Color::Rgba(v) => gpui::rgba(*v as u32).into(),
        Color::Token(_) => unreachable!("unresolved token"),
    }
}
fn align(value: i64) -> gpui::AlignItems {
    use gpui::AlignItems::*;
    match value {
        0 => Start,
        1 => End,
        2 => FlexStart,
        3 => FlexEnd,
        4 => Center,
        5 => Baseline,
        6 => Stretch,
        _ => unreachable!(),
    }
}
fn distribution(value: i64) -> gpui::AlignContent {
    use gpui::AlignContent::*;
    match value {
        0 => Start,
        1 => End,
        2 => FlexStart,
        3 => FlexEnd,
        4 => Center,
        5 => Stretch,
        6 => SpaceBetween,
        7 => SpaceEvenly,
        8 => SpaceAround,
        _ => unreachable!(),
    }
}
fn overflow(value: i64) -> gpui::Overflow {
    match value {
        0 => gpui::Overflow::Visible,
        1 => gpui::Overflow::Clip,
        2 => gpui::Overflow::Hidden,
        3 => gpui::Overflow::Scroll,
        _ => unreachable!(),
    }
}
fn grid_minimum(value: i64) -> gpui::GridTemplateMinSize {
    match value {
        0 => gpui::GridTemplateMinSize::Zero,
        1 => gpui::GridTemplateMinSize::MinContent,
        2 => gpui::GridTemplateMinSize::MaxContent,
        _ => unreachable!(),
    }
}
fn grid(value: &mut Option<gpui::GridTemplate>) -> &mut gpui::GridTemplate {
    value.get_or_insert(gpui::GridTemplate {
        repeat: 1,
        min_size: gpui::GridTemplateMinSize::Zero,
    })
}
/// Refine an already validated list. Pointer policy is applied by the host's
/// interaction binding, not by GPUI's visual StyleRefinement.
pub fn refine(style: &mut gpui::StyleRefinement, fields: &[Field]) {
    use gpui::{Styled, px};
    for field in fields {
        match field {
            Field::Display(v) => {
                style.display = Some(match v {
                    0 => gpui::Display::Block,
                    1 => gpui::Display::Flex,
                    2 => gpui::Display::Grid,
                    _ => gpui::Display::None,
                })
            }
            Field::Visibility(v) => {
                style.visibility = Some(if *v == 0 {
                    gpui::Visibility::Visible
                } else {
                    gpui::Visibility::Hidden
                })
            }
            Field::Direction(v) => {
                style.flex_direction = Some(match v {
                    0 => gpui::FlexDirection::Row,
                    1 => gpui::FlexDirection::Column,
                    2 => gpui::FlexDirection::RowReverse,
                    _ => gpui::FlexDirection::ColumnReverse,
                })
            }
            Field::Wrap(v) => {
                style.flex_wrap = Some(match v {
                    0 => gpui::FlexWrap::NoWrap,
                    1 => gpui::FlexWrap::Wrap,
                    _ => gpui::FlexWrap::WrapReverse,
                })
            }
            Field::Grow(v) => style.flex_grow = Some(*v as f32),
            Field::Shrink(v) => style.flex_shrink = Some(*v as f32),
            Field::Basis(v) => style.flex_basis = Some(gpui_length(v)),
            Field::AlignItems(v) => style.align_items = Some(align(*v)),
            Field::AlignSelf(v) => style.align_self = Some(align(*v)),
            Field::AlignContent(v) => style.align_content = Some(distribution(*v)),
            Field::JustifyContent(v) => style.justify_content = Some(distribution(*v)),
            Field::RowGap(v) => style.gap.height = Some(definite(v)),
            Field::ColumnGap(v) => style.gap.width = Some(definite(v)),
            Field::GridColumns(v) => grid(&mut style.grid_cols).repeat = *v as u16,
            Field::GridRows(v) => grid(&mut style.grid_rows).repeat = *v as u16,
            Field::GridColumnMinimum(v) => grid(&mut style.grid_cols).min_size = grid_minimum(*v),
            Field::GridRowMinimum(v) => grid(&mut style.grid_rows).min_size = grid_minimum(*v),
            Field::Width(v) => style.size.width = Some(gpui_length(v)),
            Field::Height(v) => style.size.height = Some(gpui_length(v)),
            Field::MinWidth(v) => style.min_size.width = Some(gpui_length(v)),
            Field::MinHeight(v) => style.min_size.height = Some(gpui_length(v)),
            Field::MaxWidth(v) => style.max_size.width = Some(gpui_length(v)),
            Field::MaxHeight(v) => style.max_size.height = Some(gpui_length(v)),
            Field::PaddingTop(v) => style.padding.top = Some(definite(v)),
            Field::PaddingRight(v) => style.padding.right = Some(definite(v)),
            Field::PaddingBottom(v) => style.padding.bottom = Some(definite(v)),
            Field::PaddingLeft(v) => style.padding.left = Some(definite(v)),
            Field::MarginTop(v) => style.margin.top = Some(gpui_length(v)),
            Field::MarginRight(v) => style.margin.right = Some(gpui_length(v)),
            Field::MarginBottom(v) => style.margin.bottom = Some(gpui_length(v)),
            Field::MarginLeft(v) => style.margin.left = Some(gpui_length(v)),
            Field::Position(v) => {
                style.position = Some(if *v == 0 {
                    gpui::Position::Relative
                } else {
                    gpui::Position::Absolute
                })
            }
            Field::Top(v) => style.inset.top = Some(gpui_length(v)),
            Field::Right(v) => style.inset.right = Some(gpui_length(v)),
            Field::Bottom(v) => style.inset.bottom = Some(gpui_length(v)),
            Field::Left(v) => style.inset.left = Some(gpui_length(v)),
            Field::Background(v) => {
                style.background = Some(match v {
                    Fill::Solid(c) => gpui::Fill::from(gpui_color(c)),
                    Fill::LinearGradient(angle, from, start, to, end) => gpui::linear_gradient(
                        *angle as f32,
                        gpui::linear_color_stop(gpui_color(from), *start as f32),
                        gpui::linear_color_stop(gpui_color(to), *end as f32),
                    )
                    .into(),
                })
            }
            Field::Foreground(v) => style.text.color = Some(gpui_color(v)),
            Field::Opacity(v) => style.opacity = Some(*v as f32),
            Field::BorderTopWidth(v) => style.border_widths.top = Some(px(*v as f32).into()),
            Field::BorderRightWidth(v) => style.border_widths.right = Some(px(*v as f32).into()),
            Field::BorderBottomWidth(v) => style.border_widths.bottom = Some(px(*v as f32).into()),
            Field::BorderLeftWidth(v) => style.border_widths.left = Some(px(*v as f32).into()),
            Field::TopLeftRadius(v) => style.corner_radii.top_left = Some(px(*v as f32).into()),
            Field::TopRightRadius(v) => style.corner_radii.top_right = Some(px(*v as f32).into()),
            Field::BottomLeftRadius(v) => {
                style.corner_radii.bottom_left = Some(px(*v as f32).into())
            }
            Field::BottomRightRadius(v) => {
                style.corner_radii.bottom_right = Some(px(*v as f32).into())
            }
            Field::BorderColor(v) => style.border_color = Some(gpui_color(v)),
            Field::Shadows(shadows) => {
                style.box_shadow = Some(
                    shadows
                        .iter()
                        .map(|s| {
                            let shadow = gpui::BoxShadow::new(
                                px(s.offset_x as f32),
                                px(s.offset_y as f32),
                                gpui_color(&s.color),
                            )
                            .blur_radius(px(s.blur as f32))
                            .spread_radius(px(s.spread as f32));
                            if s.inset { shadow.inset() } else { shadow }
                        })
                        .collect(),
                )
            }
            Field::FontSize(v) => style.text.font_size = Some(px(*v as f32).into()),
            Field::FontFamily(v) => style.text.font_family = Some(v.clone().into()),
            Field::FontWeight(v) => style.text.font_weight = Some(gpui::FontWeight(*v as f32)),
            Field::TextAlign(v) => {
                style.text.text_align = Some(match v {
                    0 => gpui::TextAlign::Left,
                    1 => gpui::TextAlign::Center,
                    _ => gpui::TextAlign::Right,
                })
            }
            Field::LineHeight(v) => style.text.line_height = Some(definite(v)),
            Field::WhiteSpace(v) => {
                style.text.white_space = Some(if *v == 0 {
                    gpui::WhiteSpace::Normal
                } else {
                    gpui::WhiteSpace::Nowrap
                })
            }
            Field::TextOverflow(v) => {
                style.text.text_overflow = Some(gpui::TextOverflow::Truncate(if *v == 0 {
                    "".into()
                } else {
                    "…".into()
                }))
            }
            Field::LineClamp(v) => style.text.line_clamp = Some(*v as usize),
            Field::TextDecoration(v) => {
                *style = std::mem::take(style).underline().line_through();
                style.text.underline.as_mut().unwrap().thickness =
                    px(if *v & 1 != 0 { 1. } else { 0. });
                style.text.strikethrough.as_mut().unwrap().thickness =
                    px(if *v & 2 != 0 { 1. } else { 0. });
            }
            Field::OverflowX(v) => style.overflow.x = Some(overflow(*v)),
            Field::OverflowY(v) => style.overflow.y = Some(overflow(*v)),
            Field::Cursor(v) => {
                style.mouse_cursor = Some(match v {
                    0 => gpui::CursorStyle::Arrow,
                    1 => gpui::CursorStyle::IBeam,
                    2 => gpui::CursorStyle::PointingHand,
                    3 => gpui::CursorStyle::Crosshair,
                    4 => gpui::CursorStyle::ClosedHand,
                    5 => gpui::CursorStyle::OperationNotAllowed,
                    6 => gpui::CursorStyle::ResizeLeftRight,
                    7 => gpui::CursorStyle::ResizeUpDown,
                    8 => gpui::CursorStyle::OpenHand,
                    _ => gpui::CursorStyle::ClosedHand,
                })
            }
            Field::PointerEvents(_)
            | Field::UserSelect(_)
            | Field::SelectionColor(_)
            | Field::AccessibleName(_) => (),
        }
    }
}
