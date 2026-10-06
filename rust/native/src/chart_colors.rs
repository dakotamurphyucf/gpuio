//! Resolve ordinal identities once per immutable prepared publication. Rendering
//! and legend construction share the same source-order color vector.
use crate::chart_paint::Error;
use gpuio_protocol::{
    chart_data::{Contents, Data},
    chart_style::{Key, Style},
};
use std::{
    collections::BTreeMap,
    sync::atomic::{AtomicBool, Ordering},
};

pub(crate) fn resolve(data: &Data, style: &Style, cancel: &AtomicBool) -> Result<Vec<u32>, Error> {
    if cancel.load(Ordering::Relaxed) {
        return Err(Error::Cancelled);
    }
    let mut lookup = BTreeMap::new();
    if let Some(ordinal) = &style.ordinal {
        for (index, key) in ordinal.domain.iter().copied().enumerate() {
            if index & 255 == 0 && cancel.load(Ordering::Relaxed) {
                return Err(Error::Cancelled);
            }
            lookup.insert(key, ordinal.range[index % ordinal.range.len()] as u32);
        }
    }
    let keys: Vec<Key> = match &data.contents {
        Contents::Cartesian(layers) => layers.iter().map(|l| Key::Series(l.series().id)).collect(),
        Contents::Categorical(_, layers) => {
            layers.iter().map(|l| Key::Series(l.series().id)).collect()
        }
        Contents::Radar(_, series) => series.iter().map(|s| Key::Series(s.id)).collect(),
        Contents::Pie(slices) => slices.iter().map(|s| Key::Slice(s.id)).collect(),
        Contents::Sankey(nodes, _) => nodes.iter().map(|n| Key::Node(n.id)).collect(),
        Contents::Candlestick(_) => vec![Key::Rising, Key::Falling],
    };
    let unknown = style
        .ordinal
        .as_ref()
        .and_then(|o| o.unknown)
        .map(|c| c as u32);
    let mut result = Vec::with_capacity(keys.len());
    for (index, key) in keys.into_iter().enumerate() {
        if cancel.load(Ordering::Relaxed) {
            return Err(Error::Cancelled);
        }
        result.push(
            lookup
                .get(&key)
                .copied()
                .or(unknown)
                .unwrap_or_else(|| style.color(index)),
        );
    }
    Ok(result)
}

/// Resolve both endpoints by identity before painting. This temporary lookup is
/// bounded by the already validated dataset and never runs per frame.
pub(crate) fn link_colors(
    data: &Data,
    colors: &[u32],
    cancel: &AtomicBool,
) -> Result<Vec<(u32, u32)>, Error> {
    let Contents::Sankey(nodes, edges) = &data.contents else {
        return Ok(vec![]);
    };
    if nodes.len() != colors.len() {
        return Err(Error::InvalidInput);
    }
    let by_id: BTreeMap<_, _> = nodes.iter().zip(colors).map(|(n, c)| (n.id, *c)).collect();
    edges
        .iter()
        .map(|edge| {
            if cancel.load(Ordering::Relaxed) {
                return Err(Error::Cancelled);
            }
            Ok((
                *by_id.get(&edge.source).ok_or(Error::InvalidInput)?,
                *by_id.get(&edge.target).ok_or(Error::InvalidInput)?,
            ))
        })
        .collect()
}
