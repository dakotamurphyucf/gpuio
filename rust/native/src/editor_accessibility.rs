//! Native text selections for the installed editor page. AccessKit translates
//! run positions to the platform's UTF-16 ranges; bridge offsets remain UTF-8.
use gpui::{App, Context, Div, SharedString, Stateful, StatefulInteractiveElement, accesskit};
use gpui_base::input::{BridgeTextCell, BridgeTextLayoutSnapshot, InputBaseState, InputModeKind};
use std::{cell::RefCell, ops::Range, rc::Rc};

struct Run {
    id: accesskit::NodeId,
    bytes: Range<usize>,
    boundaries: Vec<usize>,
    cells: Vec<BridgeTextCell>,
}

impl Run {
    fn position(&self, offset: usize) -> Option<accesskit::TextPosition> {
        let local = offset.checked_sub(self.bytes.start)?;
        Some(accesskit::TextPosition {
            node: self.id,
            character_index: self.boundaries.binary_search(&local).ok()?,
        })
    }

    fn offset(&self, position: accesskit::TextPosition) -> Option<usize> {
        (position.node == self.id)
            .then(|| self.boundaries.get(position.character_index))
            .flatten()
            .map(|offset| self.bytes.start + offset)
    }
}

struct Snapshot {
    text: SharedString,
    runs: Vec<Run>,
}

impl Snapshot {
    fn new(
        text: SharedString,
        layout: Option<&BridgeTextLayoutSnapshot>,
        id: impl Fn(&Range<usize>) -> accesskit::NodeId,
    ) -> Self {
        let cells = layout.map_or(&[][..], |layout| &layout.cells);
        let mut cell_index = 0;
        let mut start = 0;
        let mut runs = Vec::new();
        // Include the final empty line so the end caret belongs to that line.
        for line in text
            .split_inclusive('\n')
            .chain(text.ends_with('\n').then_some(""))
            .chain(text.is_empty().then_some(""))
        {
            // Bridge commands permit scalar boundaries; CRLF is one native
            // line break, per AccessKit's text-run contract.
            let boundaries = line
                .char_indices()
                .map(|(offset, _)| offset)
                .filter(|offset| {
                    !(*offset > 0 && &line.as_bytes()[offset - 1..=*offset] == b"\r\n")
                })
                .chain(std::iter::once(line.len()))
                .collect::<Vec<_>>();
            let mut current: Option<Run> = None;
            for bytes in boundaries
                .windows(2)
                .map(|pair| start + pair[0]..start + pair[1])
                .chain(line.is_empty().then_some(start..start))
            {
                while cells
                    .get(cell_index)
                    .is_some_and(|cell| cell.bytes.start < bytes.start)
                {
                    cell_index += 1;
                }
                let cell = cells.get(cell_index).filter(|cell| cell.bytes == bytes);
                let can_append =
                    current
                        .as_ref()
                        .is_some_and(|run| match (run.cells.last(), cell) {
                            (None, None) => true,
                            (Some(previous), Some(next)) => contiguous(previous, next),
                            _ => false,
                        });
                if !can_append {
                    if let Some(run) = current.take() {
                        runs.push(run);
                    }
                    current = Some(Run {
                        id: accesskit::NodeId(0),
                        bytes: bytes.start..bytes.start,
                        boundaries: vec![0],
                        cells: Vec::new(),
                    });
                }
                let run = current.as_mut().unwrap();
                run.bytes.end = bytes.end;
                if !bytes.is_empty() {
                    run.boundaries.push(bytes.end - run.bytes.start);
                }
                run.cells.extend(cell.cloned());
            }
            runs.extend(current);
            start += line.len();
        }
        for run in &mut runs {
            run.id = id(&run.bytes);
        }
        Self { text, runs }
    }

    fn position(&self, offset: usize) -> Option<accesskit::TextPosition> {
        // At a line boundary, prefer the following run over the prior newline.
        let index = self.runs.partition_point(|run| run.bytes.start <= offset);
        self.runs.get(index.checked_sub(1)?)?.position(offset)
    }

    fn selection(&self, anchor: usize, head: usize) -> Option<accesskit::TextSelection> {
        Some(accesskit::TextSelection {
            anchor: self.position(anchor)?,
            focus: self.position(head)?,
        })
    }

    fn offsets(&self, selection: &accesskit::TextSelection) -> Option<(usize, usize)> {
        let offset = |position| self.runs.iter().find_map(|run| run.offset(position));
        Some((offset(selection.anchor)?, offset(selection.focus)?))
    }

    fn publish(
        &self,
        selection: (usize, usize),
        scale_factor: f32,
        builder: &mut gpui::A11ySubtreeBuilder,
    ) {
        for (index, run) in self.runs.iter().enumerate() {
            let mut node = accesskit::Node::new(accesskit::Role::TextRun);
            node.set_value(self.text[run.bytes.clone()].to_owned());
            node.set_character_lengths(
                run.boundaries
                    .windows(2)
                    .map(|pair| (pair[1] - pair[0]) as u8)
                    .collect::<Vec<_>>(),
            );
            if let Some(first) = run.cells.first() {
                let bounds = run
                    .cells
                    .iter()
                    .fold(first.bounds, |bounds, cell| bounds.union(&cell.bounds));
                let scale = f64::from(scale_factor);
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
                        .filter(|cell| !cell.bytes.is_empty())
                        .map(|cell| {
                            let x = if first.right_to_left {
                                bounds.right() - cell.bounds.right()
                            } else {
                                cell.bounds.left() - bounds.left()
                            };
                            f32::from(x) * scale_factor
                        })
                        .collect::<Vec<_>>(),
                );
                node.set_character_widths(
                    run.cells
                        .iter()
                        .filter(|cell| !cell.bytes.is_empty())
                        .map(|cell| f32::from(cell.bounds.size.width) * scale_factor)
                        .collect::<Vec<_>>(),
                );
            }
            if let Some(previous) = index.checked_sub(1).and_then(|i| self.runs.get(i))
                && same_visual_line(previous, run)
            {
                node.set_previous_on_line(previous.id);
            }
            if let Some(next) = self.runs.get(index + 1)
                && same_visual_line(run, next)
            {
                node.set_next_on_line(next.id);
            }
            builder.push_child(run.id, node);
        }
        if let Some(selection) = self.selection(selection.0, selection.1) {
            builder.parent_node().set_text_selection(selection);
        }
    }
}

fn same_visual_line(first: &Run, second: &Run) -> bool {
    match (first.cells.last(), second.cells.first()) {
        (Some(first), Some(second)) => {
            first.bounds.top() == second.bounds.top()
                && first.bounds.size.height == second.bounds.size.height
        }
        _ => false,
    }
}

// A run must be representable by AccessKit's first/last-character range
// algorithm. Split visual rows, direction changes and discontinuous extents.
fn contiguous(previous: &BridgeTextCell, next: &BridgeTextCell) -> bool {
    if previous.right_to_left != next.right_to_left
        || previous.bounds.top() != next.bounds.top()
        || previous.bounds.size.height != next.bounds.size.height
    {
        return false;
    }
    previous.bounds == next.bounds
        || if previous.right_to_left {
            previous.bounds.left() == next.bounds.right()
        } else {
            previous.bounds.right() == next.bounds.left()
        }
}

/// The caller excludes private editors and supplies its current ownership and
/// interaction gate. This does not grant editing permission to read-only pages.
pub(crate) fn attach<M: InputModeKind>(
    element: Stateful<Div>,
    state: &InputBaseState<M>,
    cx: &Context<InputBaseState<M>>,
    allows: impl Fn(&App) -> bool + 'static,
) -> Stateful<Div> {
    let text = state.value();
    let revision = state.bridge_revision();
    let selection = state.bridge_selection();
    let editor = cx.weak_entity();
    let layout = state.bridge_text_layout();
    let published = Rc::new(RefCell::new(None::<Snapshot>));
    let action_snapshot = published.clone();
    element
        .aria_value(text.clone())
        .a11y_synthetic_children(move |builder| {
            let layout = layout
                .snapshot()
                .filter(|layout| layout.revision == revision && layout.source_len == text.len());
            let scale = layout.as_ref().map_or(1., |layout| layout.scale_factor);
            let snapshot = Snapshot::new(text, layout.as_deref(), |bytes| {
                builder.synthetic_node_id(("gpuio-editor-text", revision, bytes.start, bytes.end))
            });
            snapshot.publish(selection, scale, builder);
            *published.borrow_mut() = Some(snapshot);
        })
        .on_a11y_action(accesskit::Action::SetTextSelection, move |data, _, cx| {
            if !allows(cx) {
                return;
            }
            let Some(accesskit::ActionData::SetTextSelection(selection)) = data else {
                return;
            };
            let snapshot = action_snapshot.borrow();
            let Some(snapshot) = snapshot.as_ref() else {
                return;
            };
            let Some((anchor, head)) = snapshot.offsets(selection) else {
                return;
            };
            let _ = editor.update(cx, |state, cx| {
                if state.bridge_revision() == revision
                    && state.bridge_composition().is_none()
                    && state.value() == snapshot.text
                {
                    state.bridge_select(anchor, head, cx);
                }
            });
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn geometry_splits_wraps_direction_and_missing_layout_without_losing_offsets() {
        use gpui::{Bounds, point, px, size};
        let cell = |bytes, x, y, width, right_to_left| BridgeTextCell {
            bytes,
            bounds: Bounds::new(point(px(x), px(y)), size(px(width), px(20.))),
            right_to_left,
        };
        let layout = BridgeTextLayoutSnapshot {
            revision: 7,
            source_len: 11,
            scale_factor: 2.,
            cells: vec![
                cell(0..1, 10., 0., 8., false),
                cell(1..2, 18., 0., 8., false),
                cell(2..4, 30., 20., 8., true),
                cell(4..6, 22., 20., 8., true),
                cell(6..8, 22., 20., 0., true), // CRLF
                                                // The following line is not retained: no invented rectangle.
            ],
        };
        let text = "abאב\r\nend";
        let snapshot = Snapshot::new(text.into(), Some(&layout), |bytes| {
            accesskit::NodeId((bytes.start * 100 + bytes.end) as u64)
        });
        assert_eq!(
            snapshot
                .runs
                .iter()
                .map(|run| run.bytes.clone())
                .collect::<Vec<_>>(),
            vec![0..2, 2..8, 8..11]
        );
        assert_eq!(snapshot.runs[0].cells.len(), 2);
        assert_eq!(snapshot.runs[1].cells.len(), 3);
        assert!(snapshot.runs[2].cells.is_empty());
        for offset in [0, 1, 2, 4, 6, 8, 9, 10, 11] {
            assert_eq!(
                snapshot.offsets(&snapshot.selection(11, offset).unwrap()),
                Some((11, offset))
            );
        }
        assert!(snapshot.position(7).is_none());
        // Reflow may regroup source cells. An old interval ID must not be
        // reinterpreted as a different interval with the same ordinal index.
        let old = snapshot.selection(1, 2).unwrap();
        let unlaid = Snapshot::new(text.into(), None, |bytes| {
            accesskit::NodeId((bytes.start * 100 + bytes.end) as u64)
        });
        assert!(unlaid.offsets(&old).is_none());
    }

    #[test]
    fn unicode_line_boundaries_direction_and_foreign_positions() {
        let text = "λ🙂\r\né\n";
        let snapshot = Snapshot::new(text.into(), None, |bytes| {
            accesskit::NodeId(bytes.start as u64 + 1)
        });
        for offset in text
            .char_indices()
            .map(|(offset, _)| offset)
            .chain([text.len()])
        {
            if offset == 7 {
                // Between the CR and LF: not a native character boundary.
                assert!(snapshot.position(offset).is_none());
                continue;
            }
            let selection = snapshot.selection(text.len(), offset).unwrap();
            assert_eq!(snapshot.offsets(&selection), Some((text.len(), offset)));
        }
        assert_eq!(snapshot.position(8).unwrap().node, accesskit::NodeId(9));
        assert_eq!(
            snapshot.position(text.len()).unwrap().node,
            accesskit::NodeId(text.len() as u64 + 1)
        );
        assert!(snapshot.position(1).is_none());
        assert!(snapshot.position(text.len() + 1).is_none());
        let mut selection = snapshot.selection(0, 2).unwrap();
        selection.focus.node = accesskit::NodeId(99);
        assert!(snapshot.offsets(&selection).is_none());
        selection.focus.node = accesskit::NodeId(1);
        selection.focus.character_index = usize::MAX;
        assert!(snapshot.offsets(&selection).is_none());
    }

    #[test]
    fn empty_editor_has_a_caret_run() {
        let snapshot = Snapshot::new("".into(), None, |_| accesskit::NodeId(1));
        assert_eq!(snapshot.runs.len(), 1);
        assert_eq!(
            snapshot.offsets(&snapshot.selection(0, 0).unwrap()),
            Some((0, 0))
        );
    }
}
