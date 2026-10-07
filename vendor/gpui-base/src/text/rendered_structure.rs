//! Prepared structural ownership for rich accessibility. No platform tree or
//! frame geometry is stored here; native controls remain owned by their renderer.
use super::{RenderedFragment, RenderedSelection, RenderedText, RenderedTextPart};
use crate::TextSelectionContentRevision;
use std::ops::Range;

pub(super) const MAX_NODES: usize = super::MAX_PARTS * 2;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct RenderedSemanticId {
    revision: TextSelectionContentRevision,
    index: usize,
}

/// Structural ownership of the parsed document. Native control subtrees still
/// need their own publication/attachment mapping.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RenderedSemanticKind {
    Group,
    Paragraph,
    Heading {
        level: u8,
    },
    Blockquote,
    List,
    ListItem,
    Code,
    NativeObject,
    Link {
        source_start: Option<usize>,
        url: gpui::SharedString,
        title: Option<gpui::SharedString>,
    },
    Table {
        rows: usize,
        columns: usize,
    },
    Row {
        index: usize,
    },
    Cell {
        row: usize,
        column: usize,
        header: bool,
    },
    DescriptionList,
    Term,
    Definition,
}

/// One structural owner in preorder. Its contiguous part range includes logical
/// separators and unpainted owners. It holds no source, AST or native view.
#[derive(Debug)]
pub struct RenderedSemanticNode {
    pub(super) id: RenderedSemanticId,
    pub(super) kind: RenderedSemanticKind,
    pub(super) parent: Option<usize>,
    pub(super) parts: Range<usize>,
    pub(super) subtree_end: usize,
    pub(super) fragment: Option<RenderedFragment>,
}
impl RenderedSemanticNode {
    pub fn id(&self) -> RenderedSemanticId {
        self.id
    }
    pub fn kind(&self) -> &RenderedSemanticKind {
        &self.kind
    }
    pub fn parts(&self) -> Range<usize> {
        self.parts.clone()
    }
}

impl RenderedText {
    /// Prepared owner for an original top-level block slot. Non-text definitions
    /// and rules retain their slot but have no logical text owner.
    pub fn semantic_block(&self, block: usize) -> Option<RenderedSemanticId> {
        self.semantic_blocks.get(block).copied().flatten()
    }
    /// Complete prepared structure, independent of block realization. This is
    /// metadata for native publication, not a duplicate hidden accessibility tree.
    pub fn semantic_nodes(&self) -> &[RenderedSemanticNode] {
        &self.semantic_nodes
    }
    pub fn semantic_node(&self, id: RenderedSemanticId) -> Option<&RenderedSemanticNode> {
        (id.revision == self.identity)
            .then(|| self.semantic_nodes.get(id.index))
            .flatten()
    }
    pub fn semantic_parent(&self, id: RenderedSemanticId) -> Option<RenderedSemanticId> {
        let index = self.semantic_node(id)?.parent?;
        Some(self.semantic_nodes[index].id)
    }
    /// Direct children (or top-level blocks for None). Skips subtrees rather
    /// than rescanning the arena per parent. Foreign IDs yield no children.
    pub fn semantic_children(
        &self,
        parent: Option<RenderedSemanticId>,
    ) -> impl Iterator<Item = &RenderedSemanticNode> {
        let range = match parent {
            None => 0..self.semantic_nodes.len(),
            Some(id) => self
                .semantic_node(id)
                .map_or(0..0, |node| id.index + 1..node.subtree_end),
        };
        std::iter::successors(
            (range.start < range.end).then_some(range.start),
            move |index| {
                let next = self.semantic_nodes[*index].subtree_end;
                (next < range.end).then_some(next)
            },
        )
        .map(|index| &self.semantic_nodes[index])
    }
    /// Exact logical link extent; structural nodes cover their full part range.
    /// Authorization and current native owner checks still belong to the host.
    pub fn semantic_selection(&self, id: RenderedSemanticId) -> Option<RenderedSelection> {
        let node = self.semantic_node(id)?;
        if let Some(fragment) = &node.fragment {
            return fragment.selection(self);
        }
        if node.parts.is_empty() {
            return None;
        }
        let first = self.selection_for_part(node.parts.start)?;
        let last = self.selection_for_part(node.parts.end.checked_sub(1)?)?;
        self.selection(first.anchor(), last.head()).ok()
    }
    pub fn semantic_parts(&self, id: RenderedSemanticId) -> Option<&[RenderedTextPart]> {
        self.parts.get(self.semantic_node(id)?.parts.clone())
    }
    pub(super) fn semantic_string_bytes(&self) -> usize {
        let mut strings = std::collections::BTreeMap::new();
        for node in &self.semantic_nodes {
            if let RenderedSemanticKind::Link { url, title, .. } = &node.kind {
                strings.insert(url.as_ptr() as usize, url.len());
                if let Some(title) = title {
                    strings.insert(title.as_ptr() as usize, title.len());
                }
            }
        }
        strings.values().sum()
    }
    pub(super) fn semantic_retained_units(&self) -> usize {
        self.semantic_nodes.capacity() * std::mem::size_of::<RenderedSemanticNode>()
            + std::mem::size_of_val(&*self.semantic_blocks)
            + self.semantic_string_bytes
    }
    pub(super) fn semantic_max_units() -> usize {
        // Growth capacity is bounded separately from text. AST admission remains
        // 4096 nodes; extra row/cell/description wrappers fit this conservative cap.
        MAX_NODES * std::mem::size_of::<RenderedSemanticNode>() * 2
            + MAX_NODES * std::mem::size_of::<Option<RenderedSemanticId>>()
            + super::MAX_BYTES * 2
    }
}

impl super::Builder<'_> {
    pub(super) fn semantic_link(
        &mut self,
        mark: super::super::node::LinkMark,
        fragment: RenderedFragment,
    ) -> Result<(), gpui::SharedString> {
        let index = self.semantic_nodes.len();
        self.semantic_scope(
            RenderedSemanticKind::Link {
                source_start: mark.source_start,
                url: mark.url,
                title: mark.title,
            },
            |_| Ok(()),
        )?;
        let start = self.parts.partition_point(|part| {
            (part.bytes.end, part.end_slot) <= (fragment.bytes.start, fragment.start_slot)
        });
        let end = self.parts.partition_point(|part| {
            (part.bytes.start, part.start_slot) < (fragment.bytes.end, fragment.end_slot)
        });
        self.semantic_nodes[index].parts = start.min(end)..end;
        self.semantic_nodes[index].fragment = Some(fragment);
        Ok(())
    }
    pub(super) fn semantic_scope(
        &mut self,
        kind: RenderedSemanticKind,
        f: impl FnOnce(&mut Self) -> Result<(), gpui::SharedString>,
    ) -> Result<(), gpui::SharedString> {
        if self.semantic_nodes.len() >= MAX_NODES {
            return Err("rendered semantic structure exceeds preparation limits".into());
        }
        let index = self.semantic_nodes.len();
        let parent = self.semantic_parent;
        self.semantic_nodes.push(RenderedSemanticNode {
            id: RenderedSemanticId {
                revision: self.identity,
                index,
            },
            kind,
            parent,
            parts: self.parts.len()..self.parts.len(),
            subtree_end: index + 1,
            fragment: None,
        });
        self.semantic_parent = Some(index);
        let result = f(self);
        self.semantic_parent = parent;
        let end = self.semantic_nodes.len();
        self.semantic_nodes[index].subtree_end = end;
        self.semantic_nodes[index].parts.end = self.parts.len();
        result
    }
}
