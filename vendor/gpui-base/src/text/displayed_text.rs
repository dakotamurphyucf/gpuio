//! Immutable text runs prepared with the AST, before virtualization or paint.
//! This is not the copy/Markdown/AX representation: separators, list markers,
//! image descriptions and arbitrary custom renderers do not fabricate glyphs.
use std::sync::{Arc, Mutex};

use gpui::SharedString;

use super::{
    document::ParsedDocument,
    inline::InlineState,
    node::{BlockNode, Paragraph},
};

const MAX_FRAGMENTS: usize = 4096;
const MAX_BYTES: usize = 65536;

/// One contiguous rendered text run. Inline formatting does not split a run;
/// an image/custom object does. IDs are consecutive in structural display order
/// and are local to a single [`DisplayedText`] snapshot.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DisplayedFragment {
    id: u32,
    text: Arc<str>,
}
impl DisplayedFragment {
    pub fn id(&self) -> u32 {
        self.id
    }
    pub fn text(&self) -> &Arc<str> {
        &self.text
    }
}

/// Bounded immutable source for native text decorations. Each fragment is a
/// separate matching group: queries cannot cross cells, paragraphs or objects.
/// Retaining this value retains its text, but no mutable selection/layout state.
/// A nonzero opaque count means custom/image content needs a separate renderer
/// contract; its copy or accessibility label is not searchable displayed text.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct DisplayedText {
    fragments: Vec<DisplayedFragment>,
    bytes: usize,
    opaque_nodes: usize,
}
impl DisplayedText {
    pub fn fragments(&self) -> &[DisplayedFragment] {
        &self.fragments
    }
    pub fn text_bytes(&self) -> usize {
        self.bytes
    }
    pub fn opaque_nodes(&self) -> usize {
        self.opaque_nodes
    }

    pub(super) fn prepare(document: &ParsedDocument) -> Result<Arc<Self>, SharedString> {
        let mut result = Self::default();
        // parse_bounded has already validated depth and total AST node count.
        for block in document.blocks.iter() {
            result.block(block)?;
        }
        Ok(Arc::new(result))
    }

    fn push(&mut self, state: &Arc<Mutex<InlineState>>, text: &str) -> Result<(), SharedString> {
        if text.is_empty() {
            return Ok(());
        }
        if self.fragments.len() >= MAX_FRAGMENTS || text.len() > MAX_BYTES - self.bytes {
            return Err("displayed Markdown text exceeds preparation limits".into());
        }
        let fragment = DisplayedFragment {
            id: self.fragments.len() as u32,
            text: Arc::from(text),
        };
        state
            .lock()
            .map_err(|_| SharedString::from("invalid prepared inline state"))?
            .displayed_fragment = Some(fragment.clone());
        self.bytes += text.len();
        self.fragments.push(fragment);
        Ok(())
    }

    fn paragraph(&mut self, paragraph: &Paragraph) -> Result<(), SharedString> {
        let mut text = String::new();
        for child in &paragraph.children {
            // This is the same ownership split as Paragraph::inline_flow_items:
            // the object child's state owns the text preceding that object.
            if child.custom.is_some() {
                self.push(&child.state, &text)?;
                text.clear();
                self.opaque_nodes += 1;
                continue;
            }
            if child.text.len() > MAX_BYTES - self.bytes - text.len() {
                return Err("displayed Markdown text exceeds preparation limits".into());
            }
            text.push_str(&child.text);
            if child.image.is_some() {
                self.push(&child.state, &text)?;
                text.clear();
                self.opaque_nodes += 1;
            }
        }
        self.push(&paragraph.state, &text)
    }

    fn block(&mut self, block: &BlockNode) -> Result<(), SharedString> {
        match block {
            BlockNode::Root { children, .. } | BlockNode::Blockquote { children, .. } => {
                for child in children {
                    self.block(child)?;
                }
            }
            BlockNode::List { children, .. } => {
                for child in children {
                    if let BlockNode::ListItem { children, .. } = child {
                        for child in children {
                            self.block(child)?;
                        }
                    }
                }
            }
            BlockNode::Paragraph(paragraph)
            | BlockNode::Heading {
                children: paragraph,
                ..
            } => {
                self.paragraph(paragraph)?;
            }
            BlockNode::CodeBlock(code) => self.push(&code.state, &code.code())?,
            BlockNode::Table(table) => {
                for row in &table.children {
                    for cell in &row.children {
                        self.paragraph(&cell.children)?;
                    }
                }
            }
            BlockNode::Custom(_) => self.opaque_nodes += 1,
            BlockNode::Break { .. }
            | BlockNode::HorizontalRule { .. }
            | BlockNode::Definition { .. }
            | BlockNode::ListItem { .. }
            | BlockNode::Unknown => (),
        }
        Ok(())
    }
}
