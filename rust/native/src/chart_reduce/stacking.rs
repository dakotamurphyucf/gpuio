//! Natural-order cumulative stacks over aligned, immutable source positions.
//! Area envelopes share sample positions across all cumulative boundaries.
use super::*;

fn aligned(layers: Layers<'_>, kind: Kind, cancel: &AtomicBool) -> Result<usize, Error> {
    let mut reference: Option<Points<'_>> = None;
    for layer in layers.iter().filter(|l| l.kind == kind) {
        if let Some(reference) = reference {
            if reference.len() != layer.points.len() {
                return Err(Error::MisalignedStack);
            }
            for (index, (a, b)) in reference.iter().zip(layer.points.iter()).enumerate() {
                checkpoint(index, cancel)?;
                if a.x != b.x {
                    return Err(Error::MisalignedStack);
                }
            }
        } else {
            reference = Some(layer.points);
        }
    }
    Ok(reference.map_or(0, |p| p.len()))
}

pub(super) fn apply(
    layers: Layers<'_>,
    output: &mut [Series],
    policy: policy::Line,
    domain: Domain,
    width: f64,
    cancel: &AtomicBool,
) -> Result<(), Error> {
    let bars = aligned(layers, Kind::Bar, cancel)?;
    let areas = aligned(layers, Kind::Area, cancel)?;
    stack_bars(output, bars, cancel)?;
    stack_areas(layers, output, areas, policy, domain, width, cancel)
}

fn stack_bars(output: &mut [Series], count: usize, cancel: &AtomicBool) -> Result<(), Error> {
    // Aligned inputs and a shared bucket domain guarantee identical intervals.
    // Missing buckets do not shift subsequent series; use the source start.
    let mut cumulative: Vec<Option<(f64, f64)>> = vec![None; count];
    for series in output {
        let Series::Bar(bars) = series else { continue };
        let mut stacked = Vec::with_capacity(bars.len());
        for (index, bar) in bars.iter().copied().enumerate() {
            checkpoint(index, cancel)?;
            let (lower, upper) = match cumulative[bar.source.start()] {
                None => (bar.baseline, bar.value),
                Some((baseline, previous)) => {
                    if baseline != bar.baseline {
                        return Err(Error::IncompatibleBarBaselines);
                    }
                    (previous, previous + (bar.value - bar.baseline))
                }
            };
            cumulative[bar.source.start()] = Some((bar.baseline, upper));
            stacked.push(StackedBar {
                bar,
                bounds: StackBounds { lower, upper },
            });
        }
        *series = Series::StackedBar(stacked);
    }
    Ok(())
}

fn stack_areas(
    layers: Layers<'_>,
    output: &mut [Series],
    count: usize,
    policy: policy::Line,
    domain: Domain,
    width: f64,
    cancel: &AtomicBool,
) -> Result<(), Error> {
    if count == 0 {
        return Ok(());
    }
    let mut cumulative = vec![0.; count];
    let mut bounds = Vec::new();
    let mut selected = vec![matches!(policy, policy::Line::Exact); count];
    for (index, layer) in layers
        .iter()
        .enumerate()
        .filter(|(_, l)| l.kind == Kind::Area)
    {
        let mut boundary = Vec::with_capacity(count);
        for (source, point) in layer.points.iter().enumerate() {
            checkpoint(source, cancel)?;
            let lower = cumulative[source];
            let upper = lower + point.y.unwrap_or(0.);
            cumulative[source] = upper;
            boundary.push(StackBounds { lower, upper });
            // Retain both sides of every missing-value transition. Later curve
            // clipping therefore cannot bridge even a gap within one bucket.
            if source > 0 && point.y.is_some() != layer.points.get(source - 1).unwrap().y.is_some()
            {
                selected[source - 1] = true;
                selected[source] = true;
            }
        }
        if let policy::Line::Envelope(maximum) = policy {
            select_envelope(
                layer.points,
                &boundary,
                domain,
                bucket_count(maximum, width),
                &mut selected,
                cancel,
            )?;
        }
        bounds.push((index, boundary));
    }
    let kept = selected.iter().filter(|keep| **keep).count();
    for (index, boundary) in bounds {
        let layer = layers.get(index).expect("stack layer");
        let mut points = Vec::with_capacity(kept);
        for (source, bounds) in boundary.into_iter().enumerate() {
            checkpoint(source, cancel)?;
            if selected[source] {
                points.push(StackedAreaPoint {
                    source,
                    defined: layer.points.get(source).unwrap().y.is_some(),
                    bounds,
                });
            }
        }
        output[index] = Series::StackedArea(points);
    }
    Ok(())
}

fn select_envelope(
    points: Points<'_>,
    bounds: &[StackBounds],
    domain: Domain,
    buckets: usize,
    selected: &mut [bool],
    cancel: &AtomicBool,
) -> Result<(), Error> {
    let mut start = 0;
    while start < bounds.len() {
        checkpoint(start, cancel)?;
        let bucket = domain.bucket(points.get(start).unwrap().x, buckets);
        let mut end = start + 1;
        let (mut lower_min, mut lower_max, mut upper_min, mut upper_max) =
            (start, start, start, start);
        while end < bounds.len() && domain.bucket(points.get(end).unwrap().x, buckets) == bucket {
            checkpoint(end, cancel)?;
            if bounds[end].lower < bounds[lower_min].lower {
                lower_min = end;
            }
            if bounds[end].lower > bounds[lower_max].lower {
                lower_max = end;
            }
            if bounds[end].upper < bounds[upper_min].upper {
                upper_min = end;
            }
            if bounds[end].upper > bounds[upper_max].upper {
                upper_max = end;
            }
            end += 1;
        }
        for index in [start, end - 1, lower_min, lower_max, upper_min, upper_max] {
            selected[index] = true;
        }
        start = end;
    }
    Ok(())
}
