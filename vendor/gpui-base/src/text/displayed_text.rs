//! Immutable text runs prepared with the AST, before virtualization or paint.
//! This is not the copy/Markdown/AX representation: separators, list markers,
//! image descriptions and arbitrary custom renderers do not fabricate glyphs.
use std::{
    collections::BTreeMap,
    sync::{Arc, Mutex},
};

use gpui::SharedString;

use super::{
    MarkdownExtensions, MarkdownNode, MarkdownPresentation,
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
#[derive(Debug, Default)]
pub struct DisplayedText {
    fragments: Vec<DisplayedFragment>,
    bytes: usize,
    opaque_nodes: usize,
    objects: BTreeMap<usize, PreparedObject>,
}

#[derive(Debug)]
struct PreparedObject {
    _key: Arc<()>,
    text: Option<(SharedString, Option<DisplayedFragment>)>,
    non_text: bool,
}

// The snapshot is an identity, not structural equality over mutable inline state.
impl PartialEq for DisplayedText {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}
impl Eq for DisplayedText {}

impl DisplayedText {
    pub(super) fn object_is_non_text(&self, node: &MarkdownNode) -> bool {
        self.objects
            .get(&(Arc::as_ptr(&node.projection_key) as usize))
            .is_some_and(|object| object.non_text)
    }
    pub(super) fn object_text(&self, node: &MarkdownNode) -> Option<Arc<Mutex<InlineState>>> {
        let (text, fragment) = self
            .objects
            .get(&(Arc::as_ptr(&node.projection_key) as usize))?
            .text
            .as_ref()?;
        // Passive glyph elements have no persistent controller state. Keep the
        // shared projection immutable and create only a frame-local reader.
        let mut state = InlineState::default();
        state.set_text(text.clone());
        state.displayed_fragment = fragment.clone();
        Some(Arc::new(Mutex::new(state)))
    }
    pub fn fragments(&self) -> &[DisplayedFragment] {
        &self.fragments
    }
    pub fn text_bytes(&self) -> usize {
        self.bytes
    }
    pub fn opaque_nodes(&self) -> usize {
        self.opaque_nodes
    }

    pub(super) fn prepare(
        document: &ParsedDocument,
        extensions: &MarkdownExtensions,
    ) -> Result<Arc<Self>, SharedString> {
        let mut result = Self::default();
        // parse_bounded has already validated depth and total AST node count.
        for block in document.blocks.iter() {
            result.block(block, extensions)?;
        }
        Ok(Arc::new(result))
    }

    fn push(&mut self, state: &Arc<Mutex<InlineState>>, text: &str) -> Result<(), SharedString> {
        let fragment = self.fragment(text)?;
        state
            .lock()
            .map_err(|_| SharedString::from("invalid prepared inline state"))?
            .displayed_fragment = fragment;
        Ok(())
    }

    fn fragment(&mut self, text: &str) -> Result<Option<DisplayedFragment>, SharedString> {
        if text.is_empty() {
            return Ok(None);
        }
        if self.fragments.len() >= MAX_FRAGMENTS || text.len() > MAX_BYTES - self.bytes {
            return Err("displayed Markdown text exceeds preparation limits".into());
        }
        let fragment = DisplayedFragment {
            id: self.fragments.len() as u32,
            text: Arc::from(text),
        };
        self.bytes += text.len();
        self.fragments.push(fragment.clone());
        Ok(Some(fragment))
    }

    fn object(
        &mut self,
        node: &MarkdownNode,
        extensions: &MarkdownExtensions,
    ) -> Result<(), SharedString> {
        let presentation = extensions.presentation(node);
        let non_text = presentation == MarkdownPresentation::NonText;
        let text = match presentation {
            MarkdownPresentation::Opaque => {
                self.opaque_nodes += 1;
                None
            }
            MarkdownPresentation::NonText => None,
            MarkdownPresentation::Text(text) => {
                let fragment = self.fragment(&text)?;
                Some((text, fragment))
            }
        };
        self.objects.insert(
            Arc::as_ptr(&node.projection_key) as usize,
            PreparedObject {
                _key: node.projection_key.clone(),
                text,
                non_text,
            },
        );
        Ok(())
    }

    fn paragraph(
        &mut self,
        paragraph: &Paragraph,
        extensions: &MarkdownExtensions,
    ) -> Result<(), SharedString> {
        let mut text = String::new();
        for child in &paragraph.children {
            // This is the same ownership split as Paragraph::inline_flow_items:
            // the object child's state owns the text preceding that object.
            if let Some(node) = &child.custom {
                self.push(&child.state, &text)?;
                text.clear();
                self.object(node, extensions)?;
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

    fn block(
        &mut self,
        block: &BlockNode,
        extensions: &MarkdownExtensions,
    ) -> Result<(), SharedString> {
        match block {
            BlockNode::Root { children, .. } | BlockNode::Blockquote { children, .. } => {
                for child in children {
                    self.block(child, extensions)?;
                }
            }
            BlockNode::List { children, .. } => {
                for child in children {
                    if let BlockNode::ListItem { children, .. } = child {
                        for child in children {
                            self.block(child, extensions)?;
                        }
                    }
                }
            }
            BlockNode::Paragraph(paragraph)
            | BlockNode::Heading {
                children: paragraph,
                ..
            } => {
                self.paragraph(paragraph, extensions)?;
            }
            BlockNode::CodeBlock(code) => self.push(&code.state, &code.code())?,
            BlockNode::Table(table) => {
                for row in &table.children {
                    for cell in &row.children {
                        self.paragraph(&cell.children, extensions)?;
                    }
                }
            }
            BlockNode::Custom(node) => self.object(node, extensions)?,
            BlockNode::Break { .. }
            | BlockNode::HorizontalRule { .. }
            | BlockNode::Definition { .. }
            | BlockNode::ListItem { .. }
            | BlockNode::Unknown => (),
        }
        Ok(())
    }
}
