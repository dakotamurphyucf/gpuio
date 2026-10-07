//! TextRun children of actual native labels/links. Geometry comes from the same
//! cached shaped clusters as pointer selection; missing geometry stays missing.
use std::{
    ops::Range,
    sync::{Arc, Mutex},
};

use gpui::{A11ySubtreeBuilder, SharedString, accesskit};

use super::{
    RenderedAccessiblePartId, RenderedText, rendered_text::RenderedFragment,
    semantic_attachments::Frame,
};
use crate::text_selection::TextSelectionGlyph;

#[derive(Clone)]
struct Run {
    bytes: Range<usize>,
    lengths: Vec<u8>,
    cells: Vec<TextSelectionGlyph>,
}

#[derive(Clone)]
enum Binding {
    Glyphs(Frame, RenderedFragment),
    // Reading coordinates may include atomic interiors. Mapping to native
    // selection still goes through the checked part-edge conversion.
    Reading(Frame, RenderedAccessiblePartId),
}

#[derive(Clone)]
pub(super) struct Snapshot {
    text: SharedString,
    runs: Vec<Run>,
    scale: f32,
    start: usize,
    binding: Option<Binding>,
}

impl Snapshot {
    pub fn new(
        text: SharedString,
        cells: &[TextSelectionGlyph],
        range: Range<usize>,
        scale: f32,
    ) -> Self {
        let start = range.start;
        let mut cell_index = cells.partition_point(|cell| cell.bytes.end <= range.start);
        let mut runs = Vec::new();
        let mut current: Option<Run> = None;
        let mut previous_newline = false;
        let mut chars = text[range.clone()].char_indices().peekable();
        while let Some((local, character)) = chars.next() {
            let start = range.start + local;
            let len = if character == '\r' && chars.peek().is_some_and(|(_, c)| *c == '\n') {
                chars.next();
                2
            } else {
                character.len_utf8()
            };
            let bytes = start..start + len;
            while cells
                .get(cell_index)
                .is_some_and(|cell| cell.bytes.end <= start)
            {
                cell_index += 1;
            }
            // The cached geometry adapter supplies disjoint source intervals;
            // absent or ambiguous clusters retain text without a rectangle.
            let cell = cells
                .get(cell_index)
                .filter(|cell| cell.bytes.start <= start && cell.bytes.end >= bytes.end);
            let append = !previous_newline
                && current
                    .as_ref()
                    .is_some_and(|run| match (run.cells.last(), cell) {
                        (None, None) => true,
                        (Some(previous), Some(next)) => contiguous(previous, next),
                        _ => false,
                    });
            if !append {
                runs.extend(current.take());
                current = Some(Run {
                    bytes: start..start,
                    lengths: Vec::new(),
                    cells: Vec::new(),
                });
            }
            let run = current.as_mut().expect("character starts a run");
            run.bytes.end = bytes.end;
            run.lengths.push(len as u8);
            run.cells.extend(cell.cloned());
            previous_newline = character == '\n' || (character == '\r' && len == 2);
        }
        runs.extend(current);
        if range.is_empty() {
            runs.push(Run {
                bytes: range,
                lengths: Vec::new(),
                cells: Vec::new(),
            });
        }
        Self {
            text,
            runs,
            scale,
            start,
            binding: None,
        }
    }

    pub fn with_binding(mut self, binding: Option<(Frame, RenderedFragment)>) -> Self {
        self.binding = binding.map(|(frame, fragment)| Binding::Glyphs(frame, fragment));
        self
    }

    pub fn for_object<T>(
        projection: &RenderedText,
        frame: Frame,
        owner: &Arc<Mutex<T>>,
        text: SharedString,
    ) -> Option<Self> {
        let fragment = projection.object_fragment(owner)?;
        let (part, characters) = projection.accessible_fragment(&fragment)?;
        let reading = projection.accessible_part_text(part)?;
        let text = if text.as_ref() == reading {
            text
        } else {
            reading.to_owned().into()
        };
        let len = text.len();
        let mut snapshot = Self::new(text, &[], 0..len, 1.);
        debug_assert_eq!(characters.start, 0);
        debug_assert_eq!(
            characters.len(),
            snapshot
                .runs
                .iter()
                .map(|run| run.lengths.len())
                .sum::<usize>()
        );
        snapshot.binding = Some(Binding::Reading(frame, part));
        Some(snapshot)
    }

    pub fn publish(&self, key: usize, builder: &mut A11ySubtreeBuilder) {
        let mut first = 0;
        for run in &self.runs {
            let characters = first..first + run.lengths.len();
            first = characters.end;
            let binding = self.binding.as_ref().and_then(|binding| {
                let Binding::Glyphs(frame, fragment) = binding else {
                    return None;
                };
                Some((
                    frame,
                    fragment.slice(run.bytes.start - self.start..run.bytes.end - self.start)?,
                ))
            });
            let provenance = binding
                .as_ref()
                .map(|(_, fragment)| (fragment.edge(false), fragment.edge(true)));
            let id = if let Some(Binding::Reading(_, part)) = &self.binding {
                builder.synthetic_node_id((
                    "rendered-object-run",
                    part,
                    key,
                    characters.start,
                    characters.end,
                ))
            } else {
                builder.synthetic_node_id((
                    "rendered-text-run",
                    provenance,
                    key,
                    run.bytes.start,
                    run.bytes.end,
                ))
            };
            let mut node = accesskit::Node::new(accesskit::Role::TextRun);
            node.set_value(self.text[run.bytes.clone()].to_owned());
            node.set_character_lengths(run.lengths.clone());
            if let Some(first) = run.cells.first() {
                let bounds = run
                    .cells
                    .iter()
                    .fold(first.bounds, |area, cell| area.union(&cell.bounds));
                let scale = f64::from(self.scale);
                node.set_bounds(accesskit::Rect {
                    x0: f64::from(bounds.left()) * scale,
                    y0: f64::from(bounds.top()) * scale,
                    x1: f64::from(bounds.right()) * scale,
                    y1: f64::from(bounds.bottom()) * scale,
                });
                node.set_text_direction(if first.right_to_left {
                    accesskit::TextDirection::RightToLeft
                } else {
                    accesskit::TextDirection::LeftToRight
                });
                node.set_character_positions(
                    run.cells
                        .iter()
                        .map(|cell| {
                            f32::from(if first.right_to_left {
                                bounds.right() - cell.bounds.right()
                            } else {
                                cell.bounds.left() - bounds.left()
                            }) * self.scale
                        })
                        .collect::<Vec<_>>(),
                );
                node.set_character_widths(
                    run.cells
                        .iter()
                        .map(|cell| f32::from(cell.bounds.size.width) * self.scale)
                        .collect::<Vec<_>>(),
                );
            }
            if builder.push_child(id, node) {
                if let Some((frame, fragment)) = binding {
                    frame.record_run(id, &fragment, run.lengths.len());
                } else if let Some(Binding::Reading(frame, part)) = &self.binding {
                    frame.record_reading_run(id, *part, characters);
                }
            }
        }
    }
}

fn contiguous(previous: &TextSelectionGlyph, next: &TextSelectionGlyph) -> bool {
    previous.right_to_left == next.right_to_left
        && previous.bounds.top() == next.bounds.top()
        && previous.bounds.size.height == next.bounds.size.height
        && (previous.bounds == next.bounds
            || if previous.right_to_left {
                previous.bounds.left() == next.bounds.right()
            } else {
                previous.bounds.right() == next.bounds.left()
            })
}
