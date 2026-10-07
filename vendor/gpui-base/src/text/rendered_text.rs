//! Immutable logical copy text with native-owner provenance. Geometry and AX
//! node identities are separate: virtualizing a block must not change offsets.
use super::{
    document::ParsedDocument,
    inline::InlineState,
    node::{BlockNode, Paragraph},
};
use gpui::SharedString;
use std::{
    ops::Range,
    sync::{Arc, Mutex, Weak},
};

// Source and decoration text each have a 64 KiB preparation limit. Allow a
// second 64 KiB for structural separators and explicit object alternatives.
const MAX_BYTES: usize = 128 * 1024;
const MAX_PARTS: usize = 16_384;

#[derive(Clone, Debug)]
enum Owner {
    Text(Weak<Mutex<InlineState>>),
    Object(Weak<Mutex<bool>>),
    Separator,
}

/// A contiguous native text owner, atomic alternative, or structural separator.
#[derive(Debug)]
pub struct RenderedTextPart {
    bytes: Range<usize>,
    owner: Owner,
}
impl RenderedTextPart {
    pub fn bytes(&self) -> Range<usize> {
        self.bytes.clone()
    }
    pub fn is_atomic(&self) -> bool {
        matches!(self.owner, Owner::Object(_))
    }
    pub fn is_separator(&self) -> bool {
        matches!(self.owner, Owner::Separator)
    }
}

/// Checked UTF-8 position belonging to one exact prepared projection. It cannot
/// be reused in an equal-text document or after a replacement. This is not an
/// OS index or an AccessKit node ID.
#[derive(Clone, Debug)]
pub struct RenderedTextPosition {
    identity: Arc<()>,
    byte: usize,
}

/// Bounded rendered plain text, including structural separators and object copy
/// alternatives. It is independent of Markdown clipboard formatting and does
/// not itself publish accessibility nodes or authorize a selection mutation.
#[derive(Debug)]
pub struct RenderedText {
    identity: Arc<()>,
    text: SharedString,
    parts: Vec<RenderedTextPart>,
}
impl PartialEq for RenderedText {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.identity, &other.identity)
    }
}
impl RenderedText {
    pub(super) fn prepare(document: &ParsedDocument) -> Result<Arc<Self>, SharedString> {
        let mut builder = Builder::default();
        for block in document.blocks.iter() {
            builder.block(block)?;
        }
        Ok(Arc::new(Self {
            identity: Arc::new(()),
            text: builder.text.into(),
            parts: builder.parts,
        }))
    }
    /// Logical document text, before the window Copy adapter trims outer
    /// paragraph separators. It equals PreparedText::plain_text().
    pub fn text(&self) -> &str {
        &self.text
    }
    pub fn parts(&self) -> &[RenderedTextPart] {
        &self.parts
    }
    /// Conservative allocation admission units, not process RSS. Weak owner
    /// references retain no AST or native view. Includes vector capacity.
    pub fn retained_units(&self) -> usize {
        256 + self.text.len() * 2 + self.parts.capacity() * std::mem::size_of::<RenderedTextPart>()
    }
    /// Reserve before a bounded background preparation, then reduce to the
    /// retained units after installation. At most two GPUIO workers run at once.
    pub fn max_preparation_units() -> usize {
        256 + MAX_BYTES * 2 + MAX_PARTS * std::mem::size_of::<RenderedTextPart>()
    }
    pub fn position(&self, byte: usize) -> Option<RenderedTextPosition> {
        (self.text.is_char_boundary(byte)
            && !(byte > 0 && self.text.as_bytes().get(byte - 1..=byte) == Some(b"\r\n")))
        .then(|| RenderedTextPosition {
            identity: self.identity.clone(),
            byte,
        })
    }
    pub fn offset(&self, position: &RenderedTextPosition) -> Option<usize> {
        Arc::ptr_eq(&self.identity, &position.identity).then_some(position.byte)
    }
    /// Current local fragment selections only. Select-all, cross-view coverage,
    /// direction and virtual endpoint retention belong to TextViewState; this
    /// method must not be treated as the document's complete selection snapshot.
    pub fn selected_fragment_ranges(&self) -> Vec<Range<usize>> {
        self.parts
            .iter()
            .filter_map(|part| match &part.owner {
                Owner::Text(owner) => {
                    let owner = owner.upgrade()?;
                    let state = owner.lock().ok()?;
                    if state.text.as_ref() != &self.text[part.bytes.clone()] {
                        return None;
                    }
                    let selection = state.selection?;
                    let range = selection.start..selection.end;
                    if range.is_empty() || state.text.get(range.clone()).is_none() {
                        return None;
                    }
                    Some(part.bytes.start + range.start..part.bytes.start + range.end)
                }
                Owner::Object(owner) => {
                    let owner = owner.upgrade()?;
                    let selected = *owner.lock().ok()?;
                    selected.then(|| part.bytes.clone())
                }
                Owner::Separator => None,
            })
            .collect()
    }
}

#[derive(Default)]
struct Builder {
    text: String,
    parts: Vec<RenderedTextPart>,
}
impl Builder {
    fn push(&mut self, text: &str, owner: Owner) -> Result<(), SharedString> {
        if text.is_empty() {
            return Ok(());
        }
        if text.len() > MAX_BYTES - self.text.len() || self.parts.len() >= MAX_PARTS {
            return Err("rendered selection text exceeds preparation limits".into());
        }
        let start = self.text.len();
        self.text.push_str(text);
        self.parts.push(RenderedTextPart {
            bytes: start..self.text.len(),
            owner,
        });
        Ok(())
    }
    fn separator(&mut self, text: &str) -> Result<(), SharedString> {
        self.push(text, Owner::Separator)
    }
    fn text(&mut self, text: &str, state: &Arc<Mutex<InlineState>>) -> Result<(), SharedString> {
        self.push(text, Owner::Text(Arc::downgrade(state)))
    }
    fn paragraph(&mut self, paragraph: &Paragraph) -> Result<(), SharedString> {
        let mut pending = String::new();
        for child in &paragraph.children {
            if child.custom.is_some() {
                self.text(&pending, &child.state)?;
                pending.clear();
                self.push(
                    &child.text,
                    Owner::Object(Arc::downgrade(&child.custom_selection)),
                )?;
            } else {
                // Accumulate only bounded text; never allocate an unbounded
                // temporary before the projection's admission check.
                if child.text.len() > MAX_BYTES - self.text.len() - pending.len() {
                    return Err("rendered selection text exceeds preparation limits".into());
                }
                pending.push_str(&child.text);
                if child.image.is_some() {
                    self.text(&pending, &child.state)?;
                    pending.clear();
                }
            }
        }
        self.text(&pending, &paragraph.state)
    }
    fn children(&mut self, children: &[BlockNode]) -> Result<(), SharedString> {
        for child in children {
            self.block(child)?;
        }
        Ok(())
    }
    fn block(&mut self, block: &BlockNode) -> Result<(), SharedString> {
        let start = self.text.len();
        match block {
            BlockNode::Root { children, .. } | BlockNode::Blockquote { children, .. } => {
                self.children(children)?;
                if self.text.len() > start {
                    self.separator("\n")?;
                }
            }
            BlockNode::List { children, .. } | BlockNode::ListItem { children, .. } => {
                self.children(children)?
            }
            BlockNode::Paragraph(paragraph)
            | BlockNode::Heading {
                children: paragraph,
                ..
            } => {
                self.paragraph(paragraph)?;
                if self.text.len() > start {
                    self.separator("\n")?;
                }
            }
            BlockNode::CodeBlock(code) => {
                self.text(&code.code(), &code.state)?;
                if self.text.len() > start {
                    self.separator("\n")?;
                }
            }
            BlockNode::Custom(node) => {
                self.text(node.as_text(), &node.block_text)?;
                if self.text.len() > start {
                    self.separator("\n")?;
                }
            }
            BlockNode::Table(table) => {
                for row in &table.children {
                    for (index, cell) in row.children.iter().enumerate() {
                        if index > 0 {
                            self.separator(" ")?;
                        }
                        self.paragraph(&cell.children)?;
                    }
                    if !row.children.is_empty() {
                        self.separator("\n")?;
                    }
                }
                if self.text.len() > start {
                    self.separator("\n")?;
                }
            }
            BlockNode::DescriptionList(list) => {
                for entry in &list.entries {
                    let row_start = self.text.len();
                    self.paragraph(&entry.label)?;
                    let has_label = self.text.len() > row_start;
                    // Metadata prepared by the parser, not a render callback.
                    if has_label
                        && entry
                            .value
                            .children
                            .iter()
                            .any(|node| !node.text.is_empty())
                    {
                        self.separator(" ")?;
                    }
                    self.paragraph(&entry.value)?;
                    if self.text.len() > row_start {
                        self.separator("\n")?;
                    }
                }
            }
            BlockNode::Break { .. }
            | BlockNode::HorizontalRule { .. }
            | BlockNode::Definition { .. }
            | BlockNode::Unknown => {}
        }
        Ok(())
    }
}
