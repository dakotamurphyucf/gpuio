//! Native text selections for the installed editor page. AccessKit translates
//! run positions to the platform's UTF-16 ranges; bridge offsets remain UTF-8.
use gpui::{App, Context, Div, SharedString, Stateful, StatefulInteractiveElement, accesskit};
use gpui_base::input::{InputBaseState, InputModeKind};
use std::{cell::RefCell, ops::Range, rc::Rc};

struct Run {
    id: accesskit::NodeId,
    bytes: Range<usize>,
    boundaries: Vec<usize>,
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
    fn new(text: SharedString, id: impl Fn(usize) -> accesskit::NodeId) -> Self {
        let mut start = 0;
        // Include a trailing empty line so the end caret belongs to that line.
        let runs = text
            .split_inclusive('\n')
            .chain(text.ends_with('\n').then_some(""))
            .chain(text.is_empty().then_some(""))
            .enumerate()
            .map(|(index, line)| {
                // Bridge commands permit scalar boundaries. CRLF is one native
                // line break, per AccessKit's text-run contract.
                let boundaries = line
                    .char_indices()
                    .map(|(offset, _)| offset)
                    .filter(|offset| {
                        !(*offset > 0 && &line.as_bytes()[offset - 1..=*offset] == b"\r\n")
                    })
                    .chain(std::iter::once(line.len()))
                    .collect();
                let bytes = start..start + line.len();
                start = bytes.end;
                Run {
                    id: id(index),
                    bytes,
                    boundaries,
                }
            })
            .collect();
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

    fn publish(&self, selection: (usize, usize), builder: &mut gpui::A11ySubtreeBuilder) {
        for run in &self.runs {
            let mut node = accesskit::Node::new(accesskit::Role::TextRun);
            node.set_value(self.text[run.bytes.clone()].to_owned());
            node.set_character_lengths(
                run.boundaries
                    .windows(2)
                    .map(|pair| (pair[1] - pair[0]) as u8)
                    .collect::<Vec<_>>(),
            );
            builder.push_child(run.id, node);
        }
        if let Some(selection) = self.selection(selection.0, selection.1) {
            builder.parent_node().set_text_selection(selection);
        }
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
    let published = Rc::new(RefCell::new(None::<Snapshot>));
    let action_snapshot = published.clone();
    element
        .aria_value(text.clone())
        .a11y_synthetic_children(move |builder| {
            let snapshot = Snapshot::new(text, |line| {
                builder.synthetic_node_id(("gpuio-editor-text", revision, line))
            });
            snapshot.publish(selection, builder);
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
    fn unicode_line_boundaries_direction_and_foreign_positions() {
        let text = "λ🙂\r\né\n";
        let snapshot = Snapshot::new(text.into(), |index| accesskit::NodeId(index as u64 + 1));
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
        assert_eq!(snapshot.position(8).unwrap().node, accesskit::NodeId(2));
        assert_eq!(
            snapshot.position(text.len()).unwrap().node,
            accesskit::NodeId(3)
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
        let snapshot = Snapshot::new("".into(), |_| accesskit::NodeId(1));
        assert_eq!(snapshot.runs.len(), 1);
        assert_eq!(
            snapshot.offsets(&snapshot.selection(0, 0).unwrap()),
            Some((0, 0))
        );
    }
}
