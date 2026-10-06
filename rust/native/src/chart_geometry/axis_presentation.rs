//! Pure projection of bounded axis/grid presentation into the source snapshot.
use super::*;
use gpuio_protocol::{
    chart_axis::{Axis, LabelAlign, LabelSide, Tick, TickPosition},
    chart_grid::Grid,
    chart_style::Style,
};
use std::collections::BTreeSet;

#[derive(Clone, Copy)]
pub(crate) struct Styles<'a> {
    pub x: &'a Axis,
    pub y: &'a Axis,
    pub grid: &'a Grid,
}
impl<'a> Styles<'a> {
    pub fn of(style: &'a Style) -> Self {
        Self {
            x: &style.x_axis,
            y: &style.y_axis,
            grid: &style.grid,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AxisLabel {
    pub horizontal: bool,
    pub before: bool,
    pub gap: f64,
    pub width: Option<f64>,
    pub font_size: f64,
    pub color: Option<u32>,
    pub align: LabelAlign,
}
pub(crate) fn position(axis: &Axis, horizontal: bool) -> f64 {
    axis.position.unwrap_or(if horizontal { 1. } else { 0. })
}
pub(crate) fn before(axis: &Axis, horizontal: bool) -> bool {
    match axis.label_side {
        LabelSide::Auto => position(axis, horizontal) <= 0.5,
        LabelSide::Before => true,
        LabelSide::After => false,
    }
}
pub(crate) fn gap(axis: &Axis, horizontal: bool) -> f64 {
    axis.label_gap.unwrap_or(if horizontal { 6. } else { 8. })
}
pub(crate) fn line(axis: &Axis, horizontal: bool, width: f64, height: f64) -> (Point, Point) {
    let extent = if horizontal { height } else { width };
    let inset = (axis.line_width / 2.).min(extent / 2.);
    let position = (position(axis, horizontal) * extent).clamp(inset, extent - inset);
    if horizontal {
        (Point::new(0., position), Point::new(width, position))
    } else {
        (Point::new(position, 0.), Point::new(position, height))
    }
}
fn ticks(
    c: Coordinates,
    axis: &Axis,
    x: bool,
    axes: options::Axes,
    categories: Option<&[data::Category]>,
) -> Vec<Tick> {
    if let Some(ticks) = &axis.ticks {
        return ticks.clone();
    }
    let count = axis.tick_count.unwrap_or(axes.ticks);
    let values: Vec<_> = if x {
        if let Some(categories) = categories {
            let count = categories.len().min(count as usize);
            (0..count)
                .map(|i| {
                    let index = if count <= 1 {
                        0
                    } else {
                        i * (categories.len() - 1) / (count - 1)
                    };
                    (
                        TickPosition::Category(categories[index].id),
                        categories[index].label.clone(),
                    )
                })
                .collect()
        } else {
            c.x.ticks(count)
                .into_iter()
                .map(|v| (TickPosition::Value(v), format_number(v, axes.x_format)))
                .collect()
        }
    } else {
        c.y.ticks(count)
            .into_iter()
            .map(|v| (TickPosition::Value(v), format_number(v, axes.y_format)))
            .collect()
    };
    values
        .into_iter()
        .map(|(position, text)| Tick {
            position,
            text,
            color: None,
            font_size: None,
            align: LabelAlign::Auto,
        })
        .collect()
}
fn coordinate(
    c: Coordinates,
    x: bool,
    position: TickPosition,
    categories: &BTreeMap<i64, usize>,
) -> Option<f64> {
    match position {
        TickPosition::Fraction(n) => Some(n * if x { c.width } else { c.height }),
        TickPosition::Category(id) if x && c.categorical.is_some() => {
            categories.get(&id).map(|i| c.category(*i as f64))
        }
        TickPosition::Category(_) => None,
        TickPosition::Value(_) if x && c.categorical.is_some() => None,
        TickPosition::Value(v) => {
            let domain = if x { c.x } else { c.y };
            if !(domain.min..=domain.max).contains(&v) {
                return None;
            }
            Some(if x {
                c.category(v)
            } else if c.horizontal {
                c.value(v)
            } else {
                c.height - c.value(v)
            })
        }
    }
}
pub(super) fn prepare(
    c: Coordinates,
    plan: &mut Plan,
    axes: options::Axes,
    categories: Option<&[data::Category]>,
    styles: Styles<'_>,
    cancel: &AtomicBool,
) -> Result<(), Error> {
    plan.x_domain = categories.is_none().then_some(c.x);
    plan.y_domain = Some(c.y);
    let xt = ticks(c, styles.x, true, axes, categories);
    let yt = ticks(c, styles.y, false, axes, categories);
    let requested: BTreeSet<_> = xt
        .iter()
        .chain(&yt)
        .map(|t| &t.position)
        .chain(styles.grid.x.iter().flatten())
        .chain(styles.grid.y.iter().flatten())
        .filter_map(|p| {
            if let TickPosition::Category(id) = p {
                Some(*id)
            } else {
                None
            }
        })
        .collect();
    let mut ranks = BTreeMap::new();
    if !requested.is_empty() {
        for (i, category) in categories.unwrap_or(&[]).iter().enumerate() {
            checkpoint(i, cancel)?;
            if requested.contains(&category.id) {
                ranks.insert(category.id, i);
            }
        }
    }
    let default = Axis::default();
    for (x, axis, ticks, grid, visible) in [
        (true, styles.x, xt, &styles.grid.x, axes.x),
        (false, styles.y, yt, &styles.grid.y, axes.y),
    ] {
        check(cancel)?;
        let horizontal = x != c.horizontal;
        if axes.grid {
            let positions: Vec<_> = grid
                .as_ref()
                .map_or_else(|| ticks.iter().map(|t| t.position).collect(), Clone::clone);
            for p in positions {
                if let Some(n) = coordinate(c, x, p, &ranks) {
                    plan.grid.push(if horizontal {
                        (Point::new(n, 0.), Point::new(n, plan.height))
                    } else {
                        (Point::new(0., n), Point::new(plan.width, n))
                    });
                }
            }
        }
        if !visible || !axis.labels {
            continue;
        }
        for tick in ticks {
            if tick.text.is_empty() {
                continue;
            }
            let Some(n) = coordinate(c, x, tick.position, &ranks) else {
                continue;
            };
            let p = position(axis, horizontal) * if horizontal { plan.height } else { plan.width };
            let kind = if axis == &default {
                if x { LabelKind::X } else { LabelKind::Y }
            } else {
                LabelKind::Axis(AxisLabel {
                    horizontal,
                    before: before(axis, horizontal),
                    gap: gap(axis, horizontal),
                    width: axis.label_width,
                    font_size: tick.font_size.unwrap_or(axis.font_size),
                    color: tick.color.or(axis.label_color).map(|c| c as u32),
                    align: if tick.align == LabelAlign::Auto {
                        axis.label_align
                    } else {
                        tick.align
                    },
                })
            };
            plan.labels.push(Label {
                position: if horizontal {
                    Point::new(n, p)
                } else {
                    Point::new(p, n)
                },
                text: tick.text,
                kind,
            });
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests;
