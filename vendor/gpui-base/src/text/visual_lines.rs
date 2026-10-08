//! Join prepared runs on the same painted line without merging semantic flows.
use std::collections::HashMap;

use gpui::{
    A11ySubtreeBuilder,
    accesskit::{NodeId, Rect, Role},
};

use super::{RenderedText, TextRun};

struct Geometry {
    flow: NodeId,
    bounds: Option<Rect>,
    hard_end: bool,
    newline: bool,
}

pub(super) fn publish(
    projection: &RenderedText,
    runs: &[TextRun],
    builder: &mut A11ySubtreeBuilder,
) {
    let mut flows: HashMap<_, _> = builder
        .parent_node()
        .children()
        .iter()
        .map(|id| (*id, *id))
        .collect();
    let mut geometry = HashMap::new();
    builder.visit_descendants(|id, node| {
        let inherited = flows.remove(&id).unwrap_or(id);
        let flow = if matches!(
            node.role(),
            Role::Paragraph
                | Role::Heading
                | Role::Cell
                | Role::ColumnHeader
                | Role::RowHeader
                | Role::Term
                | Role::Definition
        ) {
            id
        } else {
            inherited
        };
        flows.extend(node.children().iter().map(|child| (*child, flow)));
        if node.role() == Role::TextRun {
            let value = node.value().unwrap_or("");
            geometry.insert(
                id,
                Geometry {
                    flow,
                    bounds: node.bounds(),
                    hard_end: value.ends_with('\n'),
                    newline: value == "\n" || value == "\r\n",
                },
            );
        }
    });
    let mut lines: Vec<Vec<NodeId>> = Vec::new();
    let mut previous: Option<(&TextRun, &Geometry)> = None;
    for run in runs {
        let Some(current) = geometry.get(&run.node) else {
            previous = None;
            continue;
        };
        let join = previous.is_some_and(|(run_before, before)| {
            if before.hard_end
                || projection
                    .accessible_character_utf16(run_before.part, run_before.first + run_before.len)
                    != Some(run.source_start)
            {
                return false;
            }
            // A logical hard break terminates the preceding painted line even
            // when its structural separator has no glyph or independent bounds.
            if current.newline && current.bounds.is_none() {
                return true;
            }
            match (before.bounds, current.bounds) {
                (Some(a), Some(b)) => {
                    before.flow == current.flow && a.y0 == b.y0 && a.y1 == b.y1 && a.y1 > a.y0
                }
                _ => false,
            }
        });
        if join {
            lines
                .last_mut()
                .expect("preceding run has a line")
                .push(run.node);
        } else {
            lines.push(vec![run.node]);
        }
        previous = Some((run, current));
    }
    // Every ID came from these completed descendants, once in prepared order.
    // Failure leaves the frame's previous links untouched, not a partial chain.
    assert!(
        builder.set_text_run_lines(&lines),
        "owned rendered text runs"
    );
}
