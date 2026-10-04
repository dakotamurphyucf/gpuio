//! Bounded flat-group geometry. Pure calculations; no entities, clocks or input.
use gpuio_protocol::split_group::{MAX_PANELS, Panel};
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    InvalidPanels,
    InvalidGeometry,
    NoBoundary,
    UnknownPanel,
}
#[derive(Clone, Debug, PartialEq)]
pub struct Layout {
    pub sizes: Vec<f64>,
    pub used: f64,
    pub overflow: f64,
    pub unused: f64,
}
fn panels_valid(panels: &[Panel]) -> bool {
    let mut ids = std::collections::BTreeSet::new();
    panels.len() <= MAX_PANELS && panels.iter().all(|p| p.is_valid() && ids.insert(&p.id))
}
fn checked_sizes(panels: &[Panel], sizes: &[f64]) -> Result<(), Error> {
    if !panels_valid(panels) {
        return Err(Error::InvalidPanels);
    }
    if panels.len() != sizes.len()
        || panels
            .iter()
            .zip(sizes)
            .any(|(p, s)| !s.is_finite() || *s < p.minimum_size || *s > p.maximum_size)
    {
        return Err(Error::InvalidGeometry);
    }
    Ok(())
}
/// Fit visible panes, respecting all ranges even when the parent is infeasible.
/// `preferences` contains retained sizes; new/reset panes use their optional seed.
/// Hidden panes retain a bounded preference and consume none of the extent.
pub fn fit(panels: &[Panel], preferences: &[Option<f64>], extent: f64) -> Result<Layout, Error> {
    if !panels_valid(panels) {
        return Err(Error::InvalidPanels);
    }
    if !extent.is_finite()
        || extent < 0.
        || panels.len() != preferences.len()
        || preferences
            .iter()
            .flatten()
            .any(|n| !n.is_finite() || *n < 0.)
    {
        return Err(Error::InvalidGeometry);
    }
    let visible: Vec<_> = panels
        .iter()
        .enumerate()
        .filter_map(|(i, p)| p.visible.then_some(i))
        .collect();
    let seeds: Vec<_> = panels
        .iter()
        .zip(preferences)
        .map(|(p, previous)| {
            previous
                .or(p.initial_size)
                .map(|v| v.clamp(p.minimum_size, p.maximum_size))
        })
        .collect();
    let mut sizes: Vec<_> = panels
        .iter()
        .zip(&seeds)
        .map(|(p, seed)| seed.unwrap_or(p.minimum_size))
        .collect();
    let minimum: f64 = visible.iter().map(|&i| panels[i].minimum_size).sum();
    let maximum: f64 = visible.iter().map(|&i| panels[i].maximum_size).sum();
    let target = extent.clamp(minimum, maximum);
    let fixed: f64 = visible.iter().filter_map(|&i| seeds[i]).sum();
    let flexible = visible.iter().filter(|&&i| seeds[i].is_none()).count();
    let share = if flexible == 0 {
        0.
    } else {
        (target - fixed).max(0.) / flexible as f64
    };
    let mut weights: Vec<_> = seeds.iter().map(|s| s.unwrap_or(share)).collect();
    let total: f64 = visible.iter().map(|&i| weights[i]).sum();
    for &i in &visible {
        if total == 0. {
            weights[i] = 1.;
        }
        let desired = if total == 0. {
            target / visible.len() as f64
        } else {
            weights[i] * target / total
        };
        sizes[i] = desired.clamp(panels[i].minimum_size, panels[i].maximum_size);
    }
    // Every non-final pass saturates at least one eligible range. Fixed bounds
    // and zero preferences therefore cannot cause an unbounded convergence loop.
    for _ in 0..=visible.len() {
        let remaining = target - visible.iter().map(|&i| sizes[i]).sum::<f64>();
        if remaining.abs() <= 1e-8 {
            break;
        }
        let growing = remaining > 0.;
        let eligible: Vec<_> = visible
            .iter()
            .copied()
            .filter(|&i| {
                if growing {
                    sizes[i] < panels[i].maximum_size
                } else {
                    sizes[i] > panels[i].minimum_size
                }
            })
            .collect();
        if eligible.is_empty() {
            break;
        }
        let weight: f64 = eligible.iter().map(|&i| weights[i]).sum();
        for &i in &eligible {
            let portion = if weight > 0. {
                weights[i] / weight
            } else {
                1. / eligible.len() as f64
            };
            sizes[i] = (sizes[i] + remaining * portion)
                .clamp(panels[i].minimum_size, panels[i].maximum_size);
        }
    }
    let used: f64 = visible.iter().map(|&i| sizes[i]).sum();
    Ok(Layout {
        sizes,
        used,
        overflow: (used - extent).max(0.),
        unused: (extent - used).max(0.),
    })
}
fn capacity(panels: &[Panel], sizes: &[f64], indices: &[usize], grow: bool) -> f64 {
    indices
        .iter()
        .map(|&i| {
            if grow {
                panels[i].maximum_size - sizes[i]
            } else {
                sizes[i] - panels[i].minimum_size
            }
        })
        .sum()
}
fn transfer(
    panels: &[Panel],
    sizes: &mut [f64],
    receivers: &[usize],
    donors: &[usize],
    amount: f64,
) {
    let amount = amount
        .min(capacity(panels, sizes, receivers, true))
        .min(capacity(panels, sizes, donors, false));
    for (indices, grow) in [(receivers, true), (donors, false)] {
        let mut remaining = amount;
        for &i in indices {
            let room = if grow {
                panels[i].maximum_size - sizes[i]
            } else {
                sizes[i] - panels[i].minimum_size
            };
            let delta = remaining.min(room);
            sizes[i] = (sizes[i] + if grow { delta } else { -delta })
                .clamp(panels[i].minimum_size, panels[i].maximum_size);
            remaining -= delta;
        }
    }
}
fn partitions(panels: &[Panel], after: &str) -> Result<(Vec<usize>, Vec<usize>), Error> {
    let visible: Vec<_> = panels
        .iter()
        .enumerate()
        .filter_map(|(i, p)| p.visible.then_some(i))
        .collect();
    let Some(index) = visible.iter().position(|&i| panels[i].id == after) else {
        return Err(Error::NoBoundary);
    };
    if index + 1 == visible.len() {
        return Err(Error::NoBoundary);
    }
    Ok((
        visible[..=index].iter().rev().copied().collect(),
        visible[index + 1..].to_vec(),
    ))
}
/// Feasible boundary position measured from the beginning of the group.
pub fn boundary_range(
    panels: &[Panel],
    sizes: &[f64],
    after: &str,
) -> Result<(f64, f64, f64), Error> {
    checked_sizes(panels, sizes)?;
    let (left, right) = partitions(panels, after)?;
    let current: f64 = left.iter().map(|&i| sizes[i]).sum();
    let minimum =
        current - capacity(panels, sizes, &left, false).min(capacity(panels, sizes, &right, true));
    let maximum =
        current + capacity(panels, sizes, &left, true).min(capacity(panels, sizes, &right, false));
    Ok((minimum, current, maximum))
}
/// Shift an actual boundary, nearest panes first on each side. Total size is
/// conserved, including infeasible layouts; hidden sizes never participate.
pub fn move_boundary(
    panels: &[Panel],
    sizes: &[f64],
    after: &str,
    delta: f64,
) -> Result<Vec<f64>, Error> {
    checked_sizes(panels, sizes)?;
    if !delta.is_finite() {
        return Err(Error::InvalidGeometry);
    }
    let (left, right) = partitions(panels, after)?;
    let mut next = sizes.to_vec();
    if delta >= 0. {
        transfer(panels, &mut next, &left, &right, delta);
    } else {
        transfer(panels, &mut next, &right, &left, -delta);
    }
    Ok(next)
}
/// Resize one pane using right siblings first, then left siblings. A hidden
/// target is unchanged; the owner may retain its pending request until visible.
pub fn resize_panel(
    panels: &[Panel],
    sizes: &[f64],
    id: &str,
    target: f64,
) -> Result<Vec<f64>, Error> {
    checked_sizes(panels, sizes)?;
    if !target.is_finite() || target < 0. {
        return Err(Error::InvalidGeometry);
    }
    let i = panels
        .iter()
        .position(|p| p.id == id)
        .ok_or(Error::UnknownPanel)?;
    let mut next = sizes.to_vec();
    if !panels[i].visible {
        return Ok(next);
    }
    let siblings: Vec<_> = (i + 1..panels.len())
        .chain((0..i).rev())
        .filter(|&j| panels[j].visible)
        .collect();
    let delta = target.clamp(panels[i].minimum_size, panels[i].maximum_size) - sizes[i];
    if delta >= 0. {
        transfer(panels, &mut next, &[i], &siblings, delta);
    } else {
        transfer(panels, &mut next, &siblings, &[i], -delta);
    }
    Ok(next)
}
