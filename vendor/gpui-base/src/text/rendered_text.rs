//! Immutable logical selection text with native-owner provenance. Geometry and AX
//! node identities are separate: virtualizing a block must not change offsets.
use super::{
    DisplayedText,
    document::ParsedDocument,
    inline::InlineState,
    node::{BlockNode, Paragraph},
};
use crate::{TextSelectionContentPosition, TextSelectionContentRevision};
use gpui::SharedString;
use std::{
    ops::Range,
    sync::{Arc, Mutex, Weak},
};
use unicode_segmentation::UnicodeSegmentation as _;

// Source/decoration admission does not bound custom copy alternatives. GPUIO's
// extension SDK permits 1 MiB of aggregate generated strings; leave another
// 128 KiB for ordinary source text and structural separators. The scheduler
// reserves this maximum before preparation and shrinks it after installation.
const MAX_BYTES: usize = 1024 * 1024 + 128 * 1024;
const MAX_PARTS: usize = 16_384;

/// Owner provenance copied into frame-local rich-flow fragments. No parent,
/// layout or AST reference is retained by an endpoint.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct RenderedFragment {
    identity: TextSelectionContentRevision,
    bytes: Range<usize>,
    start_slot: usize,
    end_slot: usize,
}

impl RenderedFragment {
    pub(super) fn selection(&self, projection: &RenderedText) -> Option<RenderedSelection> {
        let start = projection.captured_position(self.edge(false))?;
        let end = projection.captured_position(self.edge(true))?;
        projection.selection(&start, &end).ok()
    }
    pub(super) fn edge(&self, end: bool) -> TextSelectionContentPosition {
        let (byte, slot) = if end {
            (self.bytes.end, self.end_slot)
        } else {
            (self.bytes.start, self.start_slot)
        };
        self.identity.position(byte).with_object_boundary(slot)
    }
    pub(super) fn slice(&self, range: Range<usize>) -> Option<Self> {
        (range.start <= range.end && range.end <= self.bytes.len()).then(|| Self {
            identity: self.identity,
            bytes: self.bytes.start + range.start..self.bytes.start + range.end,
            start_slot: if range.start == 0 { self.start_slot } else { 0 },
            end_slot: if range.end == self.bytes.len() {
                self.end_slot
            } else {
                0
            },
        })
    }

    pub(super) fn position(&self, byte: usize) -> Option<TextSelectionContentPosition> {
        (byte <= self.bytes.len()).then(|| {
            let slot = if byte == 0 {
                self.start_slot
            } else if byte == self.bytes.len() {
                self.end_slot
            } else {
                0
            };
            self.identity
                .position(self.bytes.start + byte)
                .with_object_boundary(slot)
        })
    }

    pub(super) fn matches(&self, projection: &RenderedText, text: &str) -> bool {
        self.identity == projection.identity
            && projection.text.get(self.bytes.clone()) == Some(text)
    }
}

#[derive(Clone, Debug)]
enum Owner {
    Text(Weak<Mutex<InlineState>>),
    BlockObject(Weak<Mutex<InlineState>>),
    Object(Weak<Mutex<bool>>),
    Separator,
}

// Immutable occurrence metadata, with no parent/native owner retained. Source
// spans are used only after the caller establishes compatible source/AST transfer.
#[derive(Debug)]
struct ZeroSource {
    key: Weak<()>,
    span: Option<Range<usize>>,
    name: SharedString,
    markdown: SharedString,
}
impl ZeroSource {
    fn compatible(&self, old: &Self) -> bool {
        self.key.ptr_eq(&old.key)
            || (self.span.is_some()
                && self.span == old.span
                && self.name == old.name
                && self.markdown == old.markdown)
    }
}

/// A contiguous native text owner, atomic alternative, or structural separator.
#[derive(Debug)]
pub struct RenderedTextPart {
    bytes: Range<usize>,
    owner: Owner,
    start_slot: usize,
    end_slot: usize,
    zero_source: Option<ZeroSource>,
}
impl RenderedTextPart {
    pub fn bytes(&self) -> Range<usize> {
        self.bytes.clone()
    }
    pub fn is_atomic(&self) -> bool {
        matches!(self.owner, Owner::Object(_) | Owner::BlockObject(_))
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
    identity: TextSelectionContentRevision,
    byte: usize,
    slot: usize,
}

impl RenderedTextPosition {
    pub(super) fn order_key(&self) -> (usize, usize) {
        (self.byte, self.slot)
    }

    /// Compact native endpoint; revalidate it with `captured_position` before
    /// addressing an installed document. Retains no old text or native owner.
    pub fn content_position(&self) -> TextSelectionContentPosition {
        self.identity
            .position(self.byte)
            .with_object_boundary(self.slot)
    }
}

/// Directed logical range in one prepared document. Native scalar positions
/// are distinct from grapheme-aware keyboard movement and OS UTF-16 indices.
#[derive(Clone, Debug)]
pub struct RenderedSelection {
    anchor: RenderedTextPosition,
    head: RenderedTextPosition,
}
impl RenderedSelection {
    pub fn anchor(&self) -> &RenderedTextPosition {
        &self.anchor
    }
    pub fn head(&self) -> &RenderedTextPosition {
        &self.head
    }
    pub fn bytes(&self) -> Range<usize> {
        self.anchor.byte.min(self.head.byte)..self.anchor.byte.max(self.head.byte)
    }
    /// A zero-byte object selection is noncollapsed even though bytes() is empty.
    pub fn is_collapsed(&self) -> bool {
        self.anchor.order_key() == self.head.order_key()
    }
    pub fn is_backward(&self) -> bool {
        self.head.order_key() < self.anchor.order_key()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RenderedSelectionError {
    NoPreparedText,
    ForeignPosition,
    AtomicBoundary,
    OwnerUnavailable,
    UnmappedOwner,
    StaleRequest,
    NotSelectable,
}

/// Bounded logical selection text, including declared block glyphs, structural
/// separators and atomic copy alternatives. Whole-document Copy representations
/// can differ. It is independent of Markdown clipboard formatting and does
/// not itself publish accessibility nodes or authorize a selection mutation.
#[derive(Debug)]
pub struct RenderedText {
    identity: TextSelectionContentRevision,
    text: SharedString,
    parts: Vec<RenderedTextPart>,
    // Sorted native-owner address -> part index. Weak owners in `parts` keep
    // these allocation identities alive; lookup never dereferences an address.
    object_parts: Box<[(usize, usize)]>,
}
impl PartialEq for RenderedText {
    fn eq(&self, other: &Self) -> bool {
        self.identity == other.identity
    }
}
impl RenderedText {
    pub(super) fn revision(&self) -> TextSelectionContentRevision {
        self.identity
    }

    /// Rebind a compatible append after native owners have transferred their
    /// selections. Terminal structural separators stay attached to the old
    /// content edge if they moved or became newly appended owned characters.
    pub(super) fn rebind_append_selection(
        &self,
        old: &Self,
        selection: &RenderedSelection,
    ) -> Option<RenderedSelection> {
        let terminal = old
            .parts
            .iter()
            .rev()
            .take_while(|part| part.is_separator())
            .last()
            .map_or(old.text.len(), |part| part.bytes.start);
        let endpoint = |position: &RenderedTextPosition| {
            let byte = old.offset(position)?;
            let prefix = byte.min(terminal);
            if old.text.get(..prefix) != self.text.get(..prefix) {
                return None;
            }
            let byte = if byte > terminal {
                let mut covered = terminal;
                let same_separators = self
                    .parts
                    .iter()
                    .filter(|part| {
                        !part.bytes.is_empty()
                            && part.bytes.end > terminal
                            && part.bytes.start < byte
                    })
                    .all(|part| {
                        if !part.is_separator() || part.bytes.start > covered {
                            return false;
                        }
                        covered = part.bytes.end.min(byte);
                        true
                    });
                if same_separators
                    && covered == byte
                    && old.text.get(..byte) == self.text.get(..byte)
                {
                    byte
                } else {
                    terminal
                }
            } else {
                byte
            };
            // Preserve every zero-width occurrence up to this endpoint. Equal
            // plain text alone cannot establish object identity or edge order.
            let old_zeros = old.parts.iter().filter(|p| {
                p.zero_source.is_some() && (p.bytes.end, p.end_slot) <= position.order_key()
            });
            let new_zeros = self.parts.iter().filter(|p| {
                p.zero_source.is_some() && (p.bytes.end, p.end_slot) <= (byte, position.slot)
            });
            let mut new_zeros = new_zeros;
            for old_part in old_zeros {
                let new_part = new_zeros.next()?;
                if old_part.bytes != new_part.bytes
                    || old_part.end_slot != new_part.end_slot
                    || !new_part
                        .zero_source
                        .as_ref()?
                        .compatible(old_part.zero_source.as_ref()?)
                {
                    return None;
                }
            }
            if new_zeros.next().is_some() {
                return None;
            }
            self.position_with_slot(byte, position.slot)
        };
        self.selection(&endpoint(selection.anchor())?, &endpoint(selection.head())?)
            .ok()
    }

    pub(super) fn prepare(
        document: &ParsedDocument,
        displayed: &DisplayedText,
    ) -> Result<Arc<Self>, SharedString> {
        let identity = TextSelectionContentRevision::new();
        let mut builder = Builder {
            identity,
            text: String::new(),
            copy_bytes: 0,
            slot: 0,
            parts: Vec::new(),
            displayed,
        };
        for block in document.blocks.iter() {
            builder.block(block)?;
        }
        let mut object_parts = builder
            .parts
            .iter()
            .enumerate()
            .filter_map(|(index, part)| {
                if let Owner::Object(owner) = &part.owner {
                    Some((owner.as_ptr() as usize, index))
                } else {
                    None
                }
            })
            .collect::<Vec<_>>();
        object_parts.sort_unstable_by_key(|(owner, _)| *owner);
        Ok(Arc::new(Self {
            identity,
            text: builder.text.into(),
            parts: builder.parts,
            object_parts: object_parts.into_boxed_slice(),
        }))
    }
    /// Logical selection text: declared block glyphs, ordinary text, structural
    /// separators and atomic alternatives. Whole-document Copy can differ:
    /// PreparedText::plain_text() retains declared custom copy representations.
    pub fn text(&self) -> &str {
        &self.text
    }
    pub fn parts(&self) -> &[RenderedTextPart] {
        &self.parts
    }

    pub(super) fn object_fragment(&self, owner: &Arc<Mutex<bool>>) -> Option<RenderedFragment> {
        let index = self
            .object_parts
            .binary_search_by_key(&(Arc::as_ptr(owner) as usize), |(owner, _)| *owner)
            .ok()?;
        let part = &self.parts[self.object_parts[index].1];
        Some(RenderedFragment {
            identity: self.identity,
            bytes: part.bytes.clone(),
            start_slot: part.start_slot,
            end_slot: part.end_slot,
        })
    }

    pub(super) fn multi_click_range(
        &self,
        position: &RenderedTextPosition,
        paragraph: bool,
    ) -> Option<RenderedSelection> {
        let byte = self.offset(position)?;
        let index = self
            .parts
            .iter()
            .position(|part| part.bytes.contains(&byte))?;
        let part = &self.parts[index];
        if part.is_separator() {
            return None;
        }
        if paragraph {
            let start = self.parts[..index]
                .iter()
                .rev()
                .take_while(|part| !part.is_separator())
                .last()
                .unwrap_or(part);
            let end = self.parts[index + 1..]
                .iter()
                .take_while(|part| !part.is_separator())
                .last()
                .unwrap_or(part);
            return self
                .selection(
                    &self.position_with_slot(start.bytes.start, start.start_slot)?,
                    &self.position_with_slot(end.bytes.end, end.end_slot)?,
                )
                .ok();
        }
        if part.is_atomic() {
            return self.selection_for_part(index);
        }
        let text = &self.text[part.bytes.clone()];
        let mut range = crate::text_boundary::word_range_at(text, byte - part.bytes.start)?;
        // Native word policy is scalar based. Never split a grapheme when it
        // treats an emoji/joiner/combining scalar as an individual token.
        for (start, grapheme) in text.grapheme_indices(true) {
            let end = start + grapheme.len();
            if start <= range.start && range.start < end {
                range.start = start;
            }
            if start < range.end && range.end <= end {
                range.end = end;
                break;
            }
        }
        let fragment = RenderedFragment {
            identity: self.identity,
            bytes: part.bytes.clone(),
            start_slot: part.start_slot,
            end_slot: part.end_slot,
        }
        .slice(range)?;
        fragment.selection(self)
    }
    /// Independent bounds for logical selection text and declared whole Copy,
    /// including generated alternatives. Hosts separately enforce source and
    /// plugin-generation limits; empty declared glyphs cannot bypass Copy limits.
    pub const fn max_text_bytes() -> usize {
        MAX_BYTES
    }
    /// Conservative allocation admission units, not process RSS. Weak owner
    /// references retain no AST or native view. Includes vector capacity.
    pub fn retained_units(&self) -> usize {
        256 + self.text.len() * 2
            + self.parts.capacity() * std::mem::size_of::<RenderedTextPart>()
            + self.parts.len() * std::mem::size_of::<Option<RenderedFragment>>()
            + std::mem::size_of_val(&*self.object_parts)
            + self
                .parts
                .iter()
                .filter_map(|p| p.zero_source.as_ref())
                .map(|p| p.name.len() + p.markdown.len())
                .sum::<usize>()
    }
    /// Reserve before a bounded background preparation, then reduce to the
    /// retained units after installation. At most two GPUIO workers run at once.
    pub fn max_preparation_units() -> usize {
        256 + MAX_BYTES * 4
            + MAX_PARTS
                * (std::mem::size_of::<RenderedTextPart>()
                    + std::mem::size_of::<Option<RenderedFragment>>())
            + MAX_PARTS * std::mem::size_of::<(usize, usize)>()
    }
    /// Canonical byte position before any zero-byte objects at this byte.
    /// Use selection_for_part/full_selection when object edges are required.
    pub fn position(&self, byte: usize) -> Option<RenderedTextPosition> {
        self.position_with_slot(byte, 0)
    }
    fn max_slot(&self, byte: usize) -> usize {
        let end = self.parts.partition_point(|p| p.bytes.end <= byte);
        end.checked_sub(1)
            .and_then(|i| self.parts.get(i))
            .filter(|p| p.bytes.end == byte)
            .map_or(0, |p| p.end_slot)
    }
    fn position_with_slot(&self, byte: usize, slot: usize) -> Option<RenderedTextPosition> {
        (slot <= self.max_slot(byte)
            && self.text.is_char_boundary(byte)
            && !(byte > 0 && self.text.as_bytes().get(byte - 1..=byte) == Some(b"\r\n")))
        .then(|| RenderedTextPosition {
            identity: self.identity,
            byte,
            slot,
        })
    }
    pub fn offset(&self, position: &RenderedTextPosition) -> Option<usize> {
        (self.identity == position.identity).then_some(position.byte)
    }
    /// Revalidate a captured native endpoint against this exact preparation.
    pub fn captured_position(
        &self,
        position: TextSelectionContentPosition,
    ) -> Option<RenderedTextPosition> {
        (self.identity == position.revision())
            .then(|| self.position_with_slot(position.byte_offset(), position.object_boundary()))
            .flatten()
    }
    /// Whole logical content, including objects with an empty copy alternative.
    pub fn full_selection(&self) -> RenderedSelection {
        RenderedSelection {
            anchor: self.position(0).expect("document start"),
            head: self
                .position_with_slot(self.text.len(), self.max_slot(self.text.len()))
                .expect("document end"),
        }
    }
    /// Checked owner edges; an empty atomic part still has two distinct edges.
    pub fn selection_for_part(&self, index: usize) -> Option<RenderedSelection> {
        let part = self.parts.get(index)?;
        self.selection(
            &self.position_with_slot(part.bytes.start, part.start_slot)?,
            &self.position_with_slot(part.bytes.end, part.end_slot)?,
        )
        .ok()
    }
    pub(super) fn covers_all(&self, selection: &RenderedSelection) -> bool {
        let full = self.full_selection();
        !selection.is_collapsed()
            && ((selection.anchor.order_key() == full.anchor.order_key()
                && selection.head.order_key() == full.head.order_key())
                || (selection.head.order_key() == full.anchor.order_key()
                    && selection.anchor.order_key() == full.head.order_key()))
    }
    pub fn selection(
        &self,
        anchor: &RenderedTextPosition,
        head: &RenderedTextPosition,
    ) -> Result<RenderedSelection, RenderedSelectionError> {
        let anchor_byte = self
            .offset(anchor)
            .ok_or(RenderedSelectionError::ForeignPosition)?;
        let head_byte = self
            .offset(head)
            .ok_or(RenderedSelectionError::ForeignPosition)?;
        if self.parts.iter().any(|part| {
            part.is_atomic()
                && [anchor_byte, head_byte]
                    .into_iter()
                    .any(|byte| part.bytes.start < byte && byte < part.bytes.end)
        }) {
            return Err(RenderedSelectionError::AtomicBoundary);
        }
        Ok(RenderedSelection {
            anchor: anchor.clone(),
            head: head.clone(),
        })
    }
    pub fn selected_text(&self, selection: &RenderedSelection) -> Option<&str> {
        self.offset(&selection.anchor)?;
        self.offset(&selection.head)?;
        self.text.get(selection.bytes())
    }

    /// Validate and lock every native owner before modifying any of them. The
    /// caller must additionally validate its view/interaction and input policy.
    pub(super) fn apply_selection(
        &self,
        selection: &RenderedSelection,
    ) -> Result<(), RenderedSelectionError> {
        use crate::input::Selection;
        use std::sync::MutexGuard;
        enum Strong {
            Text(Arc<Mutex<InlineState>>),
            Object(Arc<Mutex<bool>>),
            Separator,
        }
        enum Locked<'a> {
            Text(MutexGuard<'a, InlineState>, &'a str, Option<Selection>),
            Object(MutexGuard<'a, bool>, bool),
            Separator,
        }
        let selection = self.selection(&selection.anchor, &selection.head)?;
        let range = selection.bytes();
        let owners = self
            .parts
            .iter()
            .map(|part| {
                match &part.owner {
                    Owner::Text(owner) | Owner::BlockObject(owner) => {
                        owner.upgrade().map(Strong::Text)
                    }
                    Owner::Object(owner) => owner.upgrade().map(Strong::Object),
                    Owner::Separator => Some(Strong::Separator),
                }
                .ok_or(RenderedSelectionError::OwnerUnavailable)
            })
            .collect::<Result<Vec<_>, _>>()?;
        let mut locks = Vec::with_capacity(owners.len());
        for (owner, part) in owners.iter().zip(&self.parts) {
            let atomic_selected = !selection.is_collapsed()
                && selection.anchor.order_key().min(selection.head.order_key())
                    <= (part.bytes.start, part.start_slot)
                && (part.bytes.end, part.end_slot)
                    <= selection.anchor.order_key().max(selection.head.order_key());
            // Opaque block widgets still need a native whole-object wrapper.
            // Do not acknowledge a zero-width selection that cannot paint it.
            if atomic_selected
                && part.bytes.is_empty()
                && matches!(part.owner, Owner::BlockObject(_))
            {
                return Err(RenderedSelectionError::UnmappedOwner);
            }
            let start = range.start.max(part.bytes.start);
            let end = range.end.min(part.bytes.end);
            let selected = (start < end)
                .then(|| Selection::new(start - part.bytes.start, end - part.bytes.start));
            locks.push(match owner {
                Strong::Text(owner) => {
                    // try_lock also rejects accidentally aliased owners without
                    // deadlocking or partially updating a malformed snapshot.
                    let state = owner
                        .try_lock()
                        .map_err(|_| RenderedSelectionError::OwnerUnavailable)?;
                    let text = &self.text[part.bytes.clone()];
                    if selected.is_some() && !state.text.is_empty() && state.text.as_ref() != text {
                        return Err(RenderedSelectionError::UnmappedOwner);
                    }
                    Locked::Text(state, text, selected)
                }
                Strong::Object(owner) => Locked::Object(
                    owner
                        .try_lock()
                        .map_err(|_| RenderedSelectionError::OwnerUnavailable)?,
                    atomic_selected,
                ),
                Strong::Separator => Locked::Separator,
            });
        }
        for lock in locks {
            match lock {
                Locked::Text(mut state, text, selected) => {
                    // Unpainted virtual blocks still need their canonical text
                    // for native plain/source Copy. This does not shape text.
                    if selected.is_some() && state.text.is_empty() {
                        state.text = text.to_owned().into();
                    }
                    state.selection = selected;
                }
                Locked::Object(mut state, selected) => *state = selected,
                Locked::Separator => {}
            }
        }
        Ok(())
    }
    /// Current local fragment selections only. Select-all, cross-view coverage,
    /// direction and virtual endpoint retention belong to TextViewState; this
    /// method must not be treated as the document's complete selection snapshot.
    pub fn selected_fragment_ranges(&self) -> Vec<Range<usize>> {
        self.parts
            .iter()
            .filter_map(|part| match &part.owner {
                Owner::Text(owner) | Owner::BlockObject(owner) => {
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

struct Builder<'a> {
    identity: TextSelectionContentRevision,
    text: String,
    copy_bytes: usize,
    slot: usize,
    parts: Vec<RenderedTextPart>,
    displayed: &'a DisplayedText,
}
impl Builder<'_> {
    fn push(&mut self, text: &str, owner: Owner) -> Result<(), SharedString> {
        self.push_with_copy_len(text, text.len(), owner)
    }
    fn push_with_copy_len(
        &mut self,
        text: &str,
        copy_len: usize,
        owner: Owner,
    ) -> Result<(), SharedString> {
        self.charge_copy(copy_len)?;
        // Empty alternatives still own native selection state that a later
        // request must clear, even though they occupy no logical copy bytes.
        if text.len() > MAX_BYTES - self.text.len() || self.parts.len() >= MAX_PARTS {
            return Err("rendered selection text exceeds preparation limits".into());
        }
        let start = self.text.len();
        let start_slot = self.slot;
        self.slot = if !text.is_empty() {
            0
        } else if matches!(owner, Owner::Object(_) | Owner::BlockObject(_)) {
            self.slot + 1
        } else {
            self.slot
        };
        self.text.push_str(text);
        self.parts.push(RenderedTextPart {
            bytes: start..self.text.len(),
            owner,
            start_slot,
            end_slot: self.slot,
            zero_source: None,
        });
        Ok(())
    }
    fn separator(&mut self, text: &str) -> Result<(), SharedString> {
        self.push(text, Owner::Separator)
    }
    fn charge_copy(&mut self, len: usize) -> Result<(), SharedString> {
        if len > MAX_BYTES - self.copy_bytes {
            return Err("rendered copy text exceeds preparation limits".into());
        }
        self.copy_bytes += len;
        Ok(())
    }
    fn block_separator(&mut self, start: usize, copy_start: usize) -> Result<(), SharedString> {
        let copy_len = usize::from(self.copy_bytes > copy_start);
        if self.text.len() > start {
            self.push_with_copy_len("\n", copy_len, Owner::Separator)
        } else {
            // Keep paragraph boundaries even when their objects contribute no
            // Copy bytes; paragraph multi-click must not cross that boundary.
            self.push_with_copy_len("", copy_len, Owner::Separator)
        }
    }
    fn text(&mut self, text: &str, state: &Arc<Mutex<InlineState>>) -> Result<(), SharedString> {
        self.text_with_copy_len(text, text.len(), state)
    }
    fn text_with_copy_len(
        &mut self,
        text: &str,
        copy_len: usize,
        state: &Arc<Mutex<InlineState>>,
    ) -> Result<(), SharedString> {
        let start = self.text.len();
        let start_slot = self.slot;
        self.push_with_copy_len(text, copy_len, Owner::Text(Arc::downgrade(state)))?;
        state
            .lock()
            .map_err(|_| SharedString::from("invalid prepared inline state"))?
            .rendered_fragment = Some(RenderedFragment {
            identity: self.identity,
            bytes: start..self.text.len(),
            start_slot,
            end_slot: self.slot,
        });
        Ok(())
    }
    fn paragraph(&mut self, paragraph: &Paragraph) -> Result<(), SharedString> {
        let mut pending = String::new();
        for child in &paragraph.children {
            if let Some(custom) = &child.custom {
                self.text(&pending, &child.state)?;
                pending.clear();
                self.push(
                    &child.text,
                    Owner::Object(Arc::downgrade(&child.custom_selection)),
                )?;
                if child.text.is_empty() {
                    self.parts.last_mut().unwrap().zero_source = Some(ZeroSource {
                        key: Arc::downgrade(&custom.projection_key),
                        span: custom.source_range(),
                        name: custom.shared_name(),
                        markdown: custom.shared_markdown(),
                    });
                }
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
        let copy_start = self.copy_bytes;
        match block {
            BlockNode::Root { children, .. } | BlockNode::Blockquote { children, .. } => {
                self.children(children)?;
                self.block_separator(start, copy_start)?;
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
                self.block_separator(start, copy_start)?;
            }
            BlockNode::CodeBlock(code) => {
                self.text(&code.code(), &code.state)?;
                self.block_separator(start, copy_start)?;
            }
            BlockNode::Custom(node) => {
                if let Some(state) = self.displayed.object_block_text(node) {
                    // A declared Text block belongs to the reader's glyph
                    // selection. Its whole-document Copy alternative may have
                    // unrelated bytes; those are not character coordinates.
                    let glyphs = state
                        .lock()
                        .map_err(|_| SharedString::from("invalid prepared inline state"))?
                        .text
                        .clone();
                    self.text_with_copy_len(&glyphs, node.as_text().len(), &state)?;
                } else {
                    node.block_text
                        .lock()
                        .map_err(|_| SharedString::from("invalid prepared inline state"))?
                        .rendered_fragment = None;
                    self.push(
                        node.as_text(),
                        Owner::BlockObject(Arc::downgrade(&node.block_text)),
                    )?;
                }
                self.block_separator(start, copy_start)?;
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
                self.block_separator(start, copy_start)?;
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
