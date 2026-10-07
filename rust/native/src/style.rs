//! Semantic validation and retained allocation accounting for portable refinements.
use gpuio_protocol::v1::*;
/// Layout participation follows the last base display declaration. Visibility
/// and interaction gating are separate: an invisible/inert pane still has size.
pub(crate) fn display_none(styles: &[Style]) -> bool {
    styles
        .iter()
        .rev()
        .find_map(|style| match style {
            Style::Fields(fields) => fields.iter().rev().find_map(|field| match field {
                Field::Display(value) => Some(*value == 3),
                _ => None,
            }),
            _ => None,
        })
        .unwrap_or(false)
}

/// Interaction gating follows the last base declaration on this node; an
/// ancestor's gate is independently enforced by the focus manager.
pub(crate) fn inert(styles: &[Style]) -> bool {
    styles
        .iter()
        .rev()
        .find_map(|style| match style {
            Style::Fields(fields) => fields.iter().rev().find_map(|field| match field {
                Field::Inert(value) => Some(*value),
                _ => None,
            }),
            _ => None,
        })
        .unwrap_or(false)
}
pub(crate) fn disabled(styles: &[Style]) -> bool {
    styles
        .iter()
        .rev()
        .find_map(|style| match style {
            Style::Fields(fields) => fields.iter().rev().find_map(|field| match field {
                Field::Disabled(value) => Some(*value),
                _ => None,
            }),
            _ => None,
        })
        .unwrap_or(false)
}
/// Last base declaration wins on this element; pointer eligibility/inheritance
/// remains separate. No color or position heuristic silently changes hit testing.
pub(crate) fn pointer_occlusion(styles: &[Style]) -> i64 {
    styles
        .iter()
        .rev()
        .find_map(|style| match style {
            Style::Fields(fields) => fields.iter().rev().find_map(|field| match field {
                Field::PointerOcclusion(value) => Some(*value),
                _ => None,
            }),
            _ => None,
        })
        .unwrap_or(0)
}
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
        Fill::PatternSlash(c, width, interval) => {
            color(c)?;
            if [width, interval]
                .into_iter()
                .all(|n| n.is_finite() && (0.5..=64.).contains(n))
            {
                Ok(())
            } else {
                Err(ErrorCode::Malformed)
            }
        }
        Fill::Checkerboard(c, size) => {
            color(c)?;
            if size.is_finite() && (0.5..=64.).contains(size) {
                Ok(())
            } else {
                Err(ErrorCode::Malformed)
            }
        }

        Fill::LinearGradientIn(space, ..) if !(0..=1).contains(space) => Err(ErrorCode::Malformed),
        Fill::LinearGradient(angle, from, start, to, end)
        | Fill::LinearGradientIn(_, angle, from, start, to, end) => {
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
            | Field::BorderStyle(v) => (0..=1).contains(v),
            Field::Direction(v)
            | Field::TextDecoration(v)
            | Field::OverflowX(v)
            | Field::OverflowY(v) => (0..=3).contains(v),
            Field::PointerOcclusion(v)
            | Field::Wrap(v)
            | Field::GridColumnMinimum(v)
            | Field::GridRowMinimum(v)
            | Field::TextAlign(v)
            | Field::TextOverflow(v) => (0..=2).contains(v),
            Field::AlignItems(v) | Field::AlignSelf(v) => (0..=6).contains(v),
            Field::AlignContent(v) | Field::JustifyContent(v) => (0..=8).contains(v),
            Field::Cursor(v) => (0..=21).contains(v),
            Field::GridColumns(v) | Field::GridRows(v) | Field::LineClamp(v) => {
                (1..=1024).contains(v)
            }
            Field::FontWeight(v) => (1..=1000).contains(v),
            Field::GridLocation(v) => v.valid(),
            Field::AspectRatio(v) => v.is_finite() && (0.000001..=1_000_000.).contains(v),
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
            Field::PointerEvents(_)
            | Field::UserSelect(_)
            | Field::Inert(_)
            | Field::Disabled(_) => true,
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
fn grid_edge(value: gpuio_protocol::grid_location::Edge) -> gpui::GridPlacement {
    use gpuio_protocol::grid_location::Edge;
    match value {
        Edge::Auto => gpui::GridPlacement::Auto,
        Edge::Line(line) => gpui::GridPlacement::Line(line as i16),
        Edge::Span(span) => gpui::GridPlacement::Span(span as u16),
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
                    Fill::PatternSlash(c, width, interval) => {
                        gpui::pattern_slash(gpui_color(c), *width as f32, *interval as f32).into()
                    }
                    Fill::Checkerboard(c, size) => {
                        gpui::checkerboard(gpui_color(c), *size as f32).into()
                    }
                    Fill::LinearGradient(angle, from, start, to, end) => gpui::linear_gradient(
                        *angle as f32,
                        gpui::linear_color_stop(gpui_color(from), *start as f32),
                        gpui::linear_color_stop(gpui_color(to), *end as f32),
                    )
                    .into(),
                    Fill::LinearGradientIn(space, angle, from, start, to, end) => {
                        gpui::linear_gradient(
                            *angle as f32,
                            gpui::linear_color_stop(gpui_color(from), *start as f32),
                            gpui::linear_color_stop(gpui_color(to), *end as f32),
                        )
                        .color_space(match space {
                            0 => gpui::ColorSpace::Srgb,
                            1 => gpui::ColorSpace::Oklab,
                            _ => unreachable!("validated interpolation space"),
                        })
                        .into()
                    }
                })
            }
            Field::GridLocation(v) => {
                style.grid_location = Some(gpui::GridLocation {
                    column: grid_edge(v.column.start)..grid_edge(v.column.end),
                    row: grid_edge(v.row.start)..grid_edge(v.row.end),
                });
            }
            Field::AspectRatio(v) => style.aspect_ratio = Some(*v as f32),
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
            Field::BorderStyle(v) => {
                style.border_style = Some(match v {
                    0 => gpui::BorderStyle::Solid,
                    1 => gpui::BorderStyle::Dashed,
                    _ => unreachable!("validated border style"),
                });
            }
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
                style.text.text_overflow = Some(match v {
                    0 => gpui::TextOverflow::Truncate("".into()),
                    1 => gpui::TextOverflow::Truncate("…".into()),
                    _ => gpui::TextOverflow::TruncateStart("…".into()),
                })
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
                    9 => gpui::CursorStyle::ClosedHand,
                    10 => gpui::CursorStyle::IBeamCursorForVerticalLayout,
                    11 => gpui::CursorStyle::ResizeColumn,
                    12 => gpui::CursorStyle::ResizeRow,
                    // Physical axes, not the swapped CSS comments in this GPUI pin.
                    13 => gpui::CursorStyle::ResizeUpLeftDownRight,
                    14 => gpui::CursorStyle::ResizeUpRightDownLeft,
                    15 => gpui::CursorStyle::ResizeLeft,
                    16 => gpui::CursorStyle::ResizeRight,
                    17 => gpui::CursorStyle::ResizeUp,
                    18 => gpui::CursorStyle::ResizeDown,
                    19 => gpui::CursorStyle::DragLink,
                    20 => gpui::CursorStyle::DragCopy,
                    _ => gpui::CursorStyle::ContextualMenu,
                })
            }
            Field::PointerEvents(_)
            | Field::UserSelect(_)
            | Field::SelectionColor(_)
            | Field::AccessibleName(_)
            | Field::Inert(_)
            | Field::Disabled(_)
            | Field::PointerOcclusion(_) => (),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn percentage_points_above_one_hundred_are_not_clamped_to_full_size() {
        let fields = [
            Field::Width(Length::Percent(200.)),
            Field::RowGap(Length::Percent(25.)),
        ];
        validate_fields(&fields).unwrap();
        let mut style = gpui::StyleRefinement::default();
        refine(&mut style, &fields);
        assert_eq!(style.size.width, Some(gpui::relative(2.).into()));
        assert_eq!(style.gap.height, Some(gpui::relative(0.25)));
    }

    #[test]
    fn alignment_aliases_match_the_pinned_gpui_helpers() {
        use gpui::Styled;
        // GPUIX calls different helpers for items/content versus justification.
        // In particular, justify_start is Start, not FlexStart.
        for (field, expected) in [
            (
                Field::AlignItems(2),
                gpui::StyleRefinement::default().items_start(),
            ),
            (
                Field::AlignItems(3),
                gpui::StyleRefinement::default().items_end(),
            ),
            (
                Field::AlignSelf(2),
                gpui::StyleRefinement::default().self_flex_start(),
            ),
            (
                Field::AlignSelf(3),
                gpui::StyleRefinement::default().self_flex_end(),
            ),
            (
                Field::AlignContent(2),
                gpui::StyleRefinement::default().content_start(),
            ),
            (
                Field::AlignContent(3),
                gpui::StyleRefinement::default().content_end(),
            ),
            (
                Field::JustifyContent(0),
                gpui::StyleRefinement::default().justify_start(),
            ),
            (
                Field::JustifyContent(1),
                gpui::StyleRefinement::default().justify_end(),
            ),
        ] {
            validate_fields(std::slice::from_ref(&field)).unwrap();
            let mut actual = gpui::StyleRefinement::default();
            refine(&mut actual, &[field]);
            assert_eq!(actual.align_items, expected.align_items);
            assert_eq!(actual.align_self, expected.align_self);
            assert_eq!(actual.align_content, expected.align_content);
            assert_eq!(actual.justify_content, expected.justify_content);
        }
        assert_ne!(gpui::AlignItems::Start, gpui::AlignItems::FlexStart);
        assert_ne!(gpui::JustifyContent::Start, gpui::JustifyContent::FlexStart);
    }

    #[test]
    fn empty_state_refinement_preserves_base_alignment() {
        use gpui::{Refineable, Styled};
        let base = crate::appearance::refinement(&[Style::Fields(vec![Field::AlignContent(4)])], 0);
        let hover = crate::appearance::refinement(&[Style::State(2, vec![])], 2);
        let mut resolved = gpui::Style::default();
        resolved.refine(&base);
        resolved.refine(&hover);
        assert_eq!(resolved.align_content, Some(gpui::AlignContent::Center));

        // This is exactly the effect of GPUIX applying content_normal to the
        // hover refinement: absence leaves the base declaration authoritative.
        let normal = gpui::StyleRefinement::default()
            .content_end()
            .content_normal();
        assert!(normal.align_content.is_none());
        resolved.refine(&normal);
        assert_eq!(resolved.align_content, Some(gpui::AlignContent::Center));
    }

    #[test]
    fn grid_minimum_replacement_preserves_track_counts_and_other_axis() {
        use gpui::GridTemplateMinSize::{MaxContent, MinContent, Zero};
        let mut style = gpui::StyleRefinement::default();
        refine(&mut style, &[Field::GridColumns(3), Field::GridRows(5)]);
        for (column, expected_column) in [(0, Zero), (1, MinContent), (2, MaxContent)] {
            for (row, expected_row) in [(0, Zero), (1, MinContent), (2, MaxContent)] {
                refine(&mut style, &[Field::GridColumnMinimum(column)]);
                refine(&mut style, &[Field::GridRowMinimum(row)]);
                assert_eq!(style.grid_cols.as_ref().unwrap().repeat, 3);
                assert_eq!(style.grid_rows.as_ref().unwrap().repeat, 5);
                assert_eq!(style.grid_cols.as_ref().unwrap().min_size, expected_column);
                assert_eq!(style.grid_rows.as_ref().unwrap().min_size, expected_row);
            }
        }
        // A minimum alone creates one track; supplying a later count retains it.
        let mut style = gpui::StyleRefinement::default();
        refine(&mut style, &[Field::GridColumnMinimum(1)]);
        assert_eq!(style.grid_cols.as_ref().unwrap().repeat, 1);
        assert!(style.grid_rows.is_none());
        refine(&mut style, &[Field::GridColumns(7)]);
        assert_eq!(style.grid_cols.as_ref().unwrap().repeat, 7);
        assert_eq!(style.grid_cols.as_ref().unwrap().min_size, MinContent);
    }

    #[test]
    fn text_decoration_replaces_both_lines_and_white_space_can_be_restored() {
        use gpui::Styled;
        for (value, underline, strike) in [(0, 0., 0.), (1, 1., 0.), (2, 0., 1.), (3, 1., 1.)] {
            let mut style = gpui::StyleRefinement::default().underline().line_through();
            refine(&mut style, &[Field::TextDecoration(value)]);
            // Explicit zero thickness suppresses inherited or component defaults.
            assert_eq!(style.text.underline.unwrap().thickness, gpui::px(underline));
            assert_eq!(
                style.text.strikethrough.unwrap().thickness,
                gpui::px(strike)
            );
        }
        let mut style = gpui::StyleRefinement::default();
        for (value, expected) in [(1, gpui::WhiteSpace::Nowrap), (0, gpui::WhiteSpace::Normal)] {
            refine(&mut style, &[Field::WhiteSpace(value)]);
            assert_eq!(style.text.white_space, Some(expected));
        }
    }

    #[test]
    fn explicit_gradient_spaces_validate_before_native_refinement() {
        for space in [0, 1] {
            for (angle, start, end) in [(0., 0., 1.), (360., 0.5, 0.5)] {
                let fields = [Field::Background(Fill::LinearGradientIn(
                    space,
                    angle,
                    Color::Rgba(0xff0000ff),
                    start,
                    Color::Rgba(0xffff),
                    end,
                ))];
                validate_fields(&fields).unwrap();
                let mut style = gpui::StyleRefinement::default();
                refine(&mut style, &fields);
                let expected = gpui::linear_gradient(
                    angle as f32,
                    gpui::linear_color_stop(gpui::rgb(0xff0000), start as f32),
                    gpui::linear_color_stop(gpui::rgb(0xff), end as f32),
                )
                .color_space(if space == 0 {
                    gpui::ColorSpace::Srgb
                } else {
                    gpui::ColorSpace::Oklab
                });
                assert_eq!(style.background, Some(expected.into()));
            }
        }
        for (space, angle, start, end) in [
            (-1, 90., 0., 1.),
            (2, 90., 0., 1.),
            (1, f64::NAN, 0., 1.),
            (1, f64::INFINITY, 0., 1.),
            (1, 361., 0., 1.),
            (1, 90., f64::NAN, 1.),
            (1, 90., 0., f64::INFINITY),
            (1, 90., 0.8, 0.2),
        ] {
            assert_eq!(
                validate_fields(&[Field::Background(Fill::LinearGradientIn(
                    space,
                    angle,
                    Color::Rgba(0),
                    start,
                    Color::Rgba(0),
                    end
                ))]),
                Err(ErrorCode::Malformed)
            );
        }
        assert_eq!(
            validate_fields(&[Field::Background(Fill::LinearGradientIn(
                1,
                90.,
                Color::Rgba(-1),
                0.,
                Color::Rgba(0),
                1.
            ))]),
            Err(ErrorCode::Malformed)
        );
    }

    #[test]
    fn aspect_ratio_reaches_native_layout_and_survives_other_refinements() {
        let mut style = gpui::StyleRefinement::default();
        assert_eq!(style.aspect_ratio, None);
        for value in [0.000001, 0.5, 1., 2., 1_000_000.] {
            let fields = [Field::AspectRatio(value)];
            validate_fields(&fields).unwrap();
            refine(&mut style, &fields);
            refine(&mut style, &[Field::Width(Length::Px(120.))]);
            assert_eq!(style.aspect_ratio, Some(value as f32));
        }
        assert_eq!(gpui::Style::default().aspect_ratio, None);
    }

    #[test]
    fn grid_location_refines_both_axes_as_one_value() {
        use gpuio_protocol::grid_location::{Axis, Edge, Location};
        let first = Location {
            column: Axis {
                start: Edge::Line(1),
                end: Edge::Line(-1),
            },
            row: Axis {
                start: Edge::Span(3),
                end: Edge::Span(3),
            },
        };
        let second = Location {
            column: Axis {
                start: Edge::Span(2),
                end: Edge::Span(2),
            },
            row: Axis {
                start: Edge::Auto,
                end: Edge::Auto,
            },
        };
        let mut style = gpui::StyleRefinement::default();
        refine(
            &mut style,
            &[Field::GridColumns(4), Field::GridLocation(first)],
        );
        assert_eq!(
            style.grid_location.as_ref().unwrap().column,
            gpui::GridPlacement::Line(1)..gpui::GridPlacement::Line(-1)
        );
        assert_eq!(
            style.grid_location.as_ref().unwrap().row,
            gpui::GridPlacement::Span(3)..gpui::GridPlacement::Span(3)
        );
        refine(&mut style, &[Field::GridLocation(second)]);
        assert_eq!(
            style.grid_location.as_ref().unwrap().column,
            gpui::GridPlacement::Span(2)..gpui::GridPlacement::Span(2)
        );
        assert_eq!(
            style.grid_location.as_ref().unwrap().row,
            gpui::GridPlacement::Auto..gpui::GridPlacement::Auto
        );
        assert_eq!(style.grid_cols.unwrap().repeat, 4);
    }

    #[test]
    fn public_cursor_values_reach_the_corresponding_gpui_refinement() {
        use gpui::CursorStyle::*;
        let expected = [
            Arrow,
            IBeam,
            PointingHand,
            Crosshair,
            ClosedHand,
            OperationNotAllowed,
            ResizeLeftRight,
            ResizeUpDown,
            OpenHand,
            ClosedHand,
            IBeamCursorForVerticalLayout,
            ResizeColumn,
            ResizeRow,
            ResizeUpLeftDownRight,
            ResizeUpRightDownLeft,
            ResizeLeft,
            ResizeRight,
            ResizeUp,
            ResizeDown,
            DragLink,
            DragCopy,
            ContextualMenu,
        ];
        for (value, expected) in expected.into_iter().enumerate() {
            let fields = [Field::Cursor(value as i64)];
            validate_fields(&fields).unwrap();
            let mut style = gpui::StyleRefinement::default();
            refine(&mut style, &fields);
            assert_eq!(style.mouse_cursor, Some(expected), "cursor {value}");
        }
    }

    #[test]
    fn border_pattern_replacement_keeps_widths_and_color() {
        let mut style = gpui::StyleRefinement::default();
        assert_eq!(style.border_style, None);
        refine(
            &mut style,
            &[
                Field::BorderTopWidth(3.),
                Field::BorderColor(Color::Rgba(0x00ff00ff)),
            ],
        );
        let widths = style.border_widths.clone();
        let color = style.border_color;
        for (value, expected) in [
            (1, gpui::BorderStyle::Dashed),
            (0, gpui::BorderStyle::Solid),
            (1, gpui::BorderStyle::Dashed),
        ] {
            let fields = [Field::BorderStyle(value)];
            validate_fields(&fields).unwrap();
            refine(&mut style, &fields);
            assert_eq!(style.border_style, Some(expected));
            assert_eq!(style.border_widths, widths);
            assert_eq!(style.border_color, color);
        }
        assert_eq!(
            gpui::Style::default().border_style,
            gpui::BorderStyle::Solid
        );
    }

    #[test]
    fn truncation_direction_survives_refinement_and_replacement() {
        use gpui::TextOverflow::*;
        let mut style = gpui::StyleRefinement::default();
        for (value, expected) in [
            (2, TruncateStart("…".into())),
            (1, Truncate("…".into())),
            (0, Truncate("".into())),
        ] {
            let fields = [Field::TextOverflow(value)];
            validate_fields(&fields).unwrap();
            refine(&mut style, &fields);
            assert_eq!(style.text.text_overflow, Some(expected));
        }
    }
}

#[cfg(test)]
mod pattern_validation_tests {
    use super::*;
    #[test]
    fn pattern_dimensions_are_checked_before_native_brush_creation() {
        for n in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY, 0., 0.49, 64.01] {
            for value in [
                Fill::PatternSlash(Color::Rgba(0), n, 4.),
                Fill::PatternSlash(Color::Rgba(0), 2., n),
                Fill::Checkerboard(Color::Rgba(0), n),
            ] {
                assert!(validate_fields(&[Field::Background(value)]).is_err());
            }
        }
        for n in [0.5, 64.] {
            for value in [
                Fill::PatternSlash(Color::Rgba(0), n, n),
                Fill::Checkerboard(Color::Rgba(0), n),
            ] {
                let fields = [Field::Background(value)];
                assert!(validate_fields(&fields).is_ok());
                let mut style = gpui::StyleRefinement::default();
                refine(&mut style, &fields);
                assert!(style.background.is_some());
            }
        }
    }
}
