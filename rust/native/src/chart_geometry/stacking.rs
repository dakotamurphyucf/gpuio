use super::*;

/// Reverse the actual lower boundary; recomputing Step_after on reversed
/// points would put its vertical step on the wrong end of each interval.
fn reverse(commands: &[Command]) -> Vec<Command> {
    let mut previous = Point::new(0., 0.);
    let mut reversed = Vec::with_capacity(commands.len());
    for command in commands {
        match *command {
            Command::Move(p) => previous = p,
            Command::Line(p) => {
                reversed.push(Command::Line(previous));
                previous = p;
            }
            Command::Cubic(a, b, p) => {
                reversed.push(Command::Cubic(b, a, previous));
                previous = p;
            }
            Command::Close => unreachable!("open boundary curve"),
        }
    }
    reversed.push(Command::Move(previous));
    reversed.reverse();
    reversed
}

/// Curve construction uses every shared retained position, even if the owning
/// series is absent there. Clip the resulting segments into defined runs after
/// interpolation: adjacent cumulative boundaries then have identical tangents,
/// including when neighboring layers have different missing-value runs.
fn section(
    commands: &[Command],
    start: usize,
    end: usize,
    first: Point,
    stride: usize,
) -> Vec<Command> {
    let mut result = Vec::with_capacity(1 + (end - start - 1) * stride);
    result.push(Command::Move(first));
    result.extend_from_slice(&commands[1 + start * stride..1 + (end - 1) * stride]);
    result
}

pub(super) fn area(
    plan: &mut Plan,
    series: usize,
    source: crate::chart_cartesian::Points<'_>,
    points: &[reduce::StackedAreaPoint],
    c: Coordinates,
    options: options::Cartesian,
    cancel: &AtomicBool,
) -> Result<(), Error> {
    let mut lower = Vec::with_capacity(points.len());
    let mut upper = Vec::with_capacity(points.len());
    for (index, point) in points.iter().enumerate() {
        checkpoint(index, cancel)?;
        let datum = source.get(point.source).expect("stack source");
        lower.push(c.point(datum.x, point.bounds.lower));
        upper.push(c.point(datum.x, point.bounds.upper));
    }
    let upper_curve = curve(&upper, options.curve, c.horizontal);
    let lower_curve = curve(&lower, options.curve, c.horizontal);
    let stride = if options.curve == options::Curve::StepAfter {
        2
    } else {
        1
    };
    let mut start = 0;
    while start < points.len() {
        checkpoint(start, cancel)?;
        if !points[start].defined {
            start += 1;
            continue;
        }
        let mut end = start + 1;
        while end < points.len() && points[end].defined {
            checkpoint(end, cancel)?;
            end += 1;
        }
        for i in start..end {
            checkpoint(i, cancel)?;
            let p = points[i];
            let value = source
                .get(p.source)
                .unwrap()
                .y
                .expect("defined stack point");
            plan.summaries.push((
                plan.marks.len(),
                Summary::Stacked {
                    baseline: 0.,
                    value,
                    lower: p.bounds.lower + c.value_offset,
                    upper: p.bounds.upper + c.value_offset,
                },
            ));
            plan.marks.push(Mark {
                layer: series,
                source: Source::Cartesian {
                    series,
                    start: p.source,
                    end: p.source + 1,
                },
                shape: Shape::Dot {
                    center: upper[i],
                    visible: options.dots || end - start == 1,
                },
            });
        }
        if end - start > 1 {
            let commands = section(&upper_curve, start, end, upper[start], stride);
            let back = reverse(&section(&lower_curve, start, end, lower[start], stride));
            let mut fill = commands.clone();
            fill.push(Command::Line(lower[end - 1]));
            fill.extend_from_slice(&back[1..]);
            fill.push(Command::Close);
            plan.paths.push(Path {
                layer: series,
                fill: true,
                commands: fill,
            });
            plan.paths.push(Path {
                layer: series,
                fill: false,
                commands,
            });
        }
        start = end;
    }
    Ok(())
}

#[cfg(test)]
mod tests;
