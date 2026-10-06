//! Bounded, source-independent axis strokes and grid dash preparation.
use super::*;
use geometry::axis_presentation;
use gpuio_protocol::chart_data::Contents;

const MAX_GRID_FRAGMENTS: usize = 16_384;

pub(super) fn segments(lines: &[(geometry::Point, geometry::Point)]) -> MeshPath {
    MeshPath(
        lines
            .iter()
            .flat_map(|(a, b)| {
                [
                    PathCommand::Move(point_wire(*a)),
                    PathCommand::Line(point_wire(*b)),
                ]
            })
            .collect(),
    )
}

fn grid_path(
    lines: &[(geometry::Point, geometry::Point)],
    dashes: &[f64],
    cancel: &AtomicBool,
) -> Result<MeshPath, Error> {
    if dashes.is_empty() {
        check(cancel)?;
        if lines.len() > MAX_GRID_FRAGMENTS {
            return Err(Error::RenderLimit);
        }
        return Ok(segments(lines));
    }
    let mut out = Vec::new();
    for (a, b) in lines {
        check(cancel)?;
        let dx = b.x - a.x;
        let dy = b.y - a.y;
        let length = dx.hypot(dy);
        let mut distance = 0.;
        let mut index = 0;
        let mut painted = true;
        while distance < length {
            check(cancel)?;
            let end = (distance + dashes[index]).min(length);
            if painted {
                if out.len() / 2 == MAX_GRID_FRAGMENTS {
                    return Err(Error::RenderLimit);
                }
                let at = |n: f64| MeshPoint {
                    x: a.x + dx * n / length,
                    y: a.y + dy * n / length,
                };
                out.push(PathCommand::Move(at(distance)));
                out.push(PathCommand::Line(at(end)));
            }
            distance = end;
            index = (index + 1) % dashes.len();
            painted = !painted;
        }
    }
    Ok(MeshPath(out))
}

pub(super) fn prepare(
    build: &mut Build<'_>,
    geometry: &geometry::Plan,
    data: &Data,
    options: &Options,
    style: &Style,
) -> Result<(), Error> {
    if !geometry.grid.is_empty() {
        let path = grid_path(&geometry.grid, &style.grid.dashes, build.cancel)?;
        if !path.0.is_empty() {
            build.mesh(
                &path,
                mesh::Style::Stroke(style.grid.width),
                style.grid.color.unwrap_or(style.grid_color) as u32,
            )?;
        }
    }
    if !matches!(
        data.contents,
        Contents::Cartesian(_) | Contents::Categorical(..) | Contents::Candlestick(_)
    ) {
        return Ok(());
    }
    let horizontal = options.cartesian.orientation.is_horizontal()
        && !matches!(data.contents, Contents::Candlestick(_));
    let mut strokes = Vec::with_capacity(2);
    for (axis, visible, horizontal) in [
        (&style.x_axis, options.axes.x, !horizontal),
        (&style.y_axis, options.axes.y, horizontal),
    ] {
        if visible && axis.line {
            strokes.push((
                axis_presentation::line(axis, horizontal, geometry.width, geometry.height),
                axis.line_width,
                axis.line_color.unwrap_or(style.axis_color) as u32,
            ));
        }
    }
    // Preserve the single retained axis mesh when both strokes share a brush.
    if strokes.len() == 2 && strokes[0].1 == strokes[1].1 && strokes[0].2 == strokes[1].2 {
        build.mesh(
            &segments(&[strokes[0].0, strokes[1].0]),
            mesh::Style::Stroke(strokes[0].1),
            strokes[0].2,
        )?;
    } else {
        for (line, width, color) in strokes {
            build.mesh(&segments(&[line]), mesh::Style::Stroke(width), color)?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    fn line(length: f64) -> (geometry::Point, geometry::Point) {
        (
            geometry::Point { x: 0., y: 0. },
            geometry::Point { x: length, y: 0. },
        )
    }
    #[test]
    fn odd_dashes_repeat_over_two_cycles_and_restart_per_line() {
        let path = grid_path(
            &[line(16.), line(16.)],
            &[3., 2., 1.],
            &AtomicBool::new(false),
        )
        .unwrap();
        let spans: Vec<_> = path
            .0
            .chunks_exact(2)
            .map(|pair| match pair {
                [PathCommand::Move(a), PathCommand::Line(b)] => (a.x, b.x),
                _ => panic!("dash must be an independent line"),
            })
            .collect();
        let expected = [(0., 3.), (5., 6.), (9., 11.), (12., 15.)];
        assert_eq!(spans, expected.repeat(2));
    }
    #[test]
    fn dash_preparation_rejects_excess_and_cancels_without_partial_output() {
        assert!(matches!(
            grid_path(&[line(32768.)], &[0.5], &AtomicBool::new(false)),
            Err(Error::RenderLimit)
        ));
        assert!(matches!(
            grid_path(&[line(20.)], &[1.], &AtomicBool::new(true)),
            Err(Error::Cancelled)
        ));
        assert!(
            grid_path(&[line(0.)], &[1.], &AtomicBool::new(false))
                .unwrap()
                .0
                .is_empty()
        );
        assert_eq!(
            grid_path(&[line(16384.)], &[0.5], &AtomicBool::new(false))
                .unwrap()
                .0
                .len(),
            32768
        );
    }
}
