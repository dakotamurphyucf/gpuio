//! Prepared coordinate conversion, independent of platform nodes and frame geometry.
use super::{RenderedText, RenderedTextPosition};
use crate::TextSelectionContentRevision;
use std::ops::Range;

/// Logical part identity. An equal-text replacement never accepts an older ID.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct RenderedAccessiblePartId {
    revision: TextSelectionContentRevision,
    index: usize,
}

#[derive(Clone, Copy, Debug)]
struct Boundary {
    byte: u32,
    utf16: u32,
}

// Bound every lookup to a binary search plus at most 63 character widths.
const CHECKPOINT_STRIDE: usize = 64;

#[derive(Debug)]
enum Characters {
    Ascii(usize),
    Unicode {
        // Low three bits: UTF-8 bytes (1..4); high bits: UTF-16 units (1..2).
        lengths: Box<[u8]>,
        checkpoints: Box<[Boundary]>,
        end: Boundary,
    },
}
impl Characters {
    fn new(text: &str) -> Self {
        if text.is_ascii() && !text.as_bytes().windows(2).any(|pair| pair == b"\r\n") {
            return Self::Ascii(text.len());
        }
        let count = text
            .char_indices()
            .filter(|(byte, ch)| !(*ch == '\n' && *byte > 0 && text.as_bytes()[*byte - 1] == b'\r'))
            .count();
        let mut lengths = Vec::with_capacity(count);
        let mut checkpoints = Vec::with_capacity(count.div_ceil(CHECKPOINT_STRIDE));
        let mut chars = text.chars().peekable();
        let mut end = Boundary { byte: 0, utf16: 0 };
        while let Some(ch) = chars.next() {
            if lengths.len() % CHECKPOINT_STRIDE == 0 {
                checkpoints.push(end);
            }
            let (bytes, utf16) = if ch == '\r' && chars.peek() == Some(&'\n') {
                chars.next();
                (2, 2)
            } else {
                (ch.len_utf8(), ch.len_utf16())
            };
            lengths.push(bytes as u8 | ((utf16 as u8) << 3));
            end.byte += bytes as u32;
            end.utf16 += utf16 as u32;
        }
        Self::Unicode {
            lengths: lengths.into_boxed_slice(),
            checkpoints: checkpoints.into_boxed_slice(),
            end,
        }
    }
    fn len(&self) -> usize {
        match self {
            Self::Ascii(len) => *len,
            Self::Unicode { lengths, .. } => lengths.len(),
        }
    }
    fn boundary(&self, character: usize) -> Option<(usize, usize)> {
        match self {
            Self::Ascii(len) => (character <= *len).then_some((character, character)),
            Self::Unicode {
                lengths,
                checkpoints,
                end,
            } => {
                if character == lengths.len() {
                    return Some((end.byte as usize, end.utf16 as usize));
                }
                if character > lengths.len() {
                    return None;
                }
                let checkpoint = character / CHECKPOINT_STRIDE;
                let mut boundary = *checkpoints.get(checkpoint)?;
                for width in &lengths[checkpoint * CHECKPOINT_STRIDE..character] {
                    boundary.byte += (width & 7) as u32;
                    boundary.utf16 += (width >> 3) as u32;
                }
                Some((boundary.byte as usize, boundary.utf16 as usize))
            }
        }
    }
    fn character(&self, offset: usize, utf16: bool) -> Option<usize> {
        match self {
            Self::Ascii(len) => (offset <= *len).then_some(offset),
            Self::Unicode {
                lengths,
                checkpoints,
                end,
            } => {
                let coordinate = |boundary: &Boundary| {
                    if utf16 {
                        boundary.utf16 as usize
                    } else {
                        boundary.byte as usize
                    }
                };
                let limit = coordinate(end);
                if offset == limit {
                    return Some(lengths.len());
                }
                if offset > limit {
                    return None;
                }
                let checkpoint = checkpoints
                    .partition_point(|boundary| coordinate(boundary) <= offset)
                    .checked_sub(1)?;
                let mut current = coordinate(&checkpoints[checkpoint]);
                let start = checkpoint * CHECKPOINT_STRIDE;
                for (index, width) in lengths
                    .iter()
                    .enumerate()
                    .skip(start)
                    .take(CHECKPOINT_STRIDE)
                {
                    if current == offset {
                        return Some(index);
                    }
                    current += if utf16 { width >> 3 } else { width & 7 } as usize;
                    if current > offset {
                        return None;
                    }
                }
                None
            }
        }
    }
    fn character_length(&self, character: usize) -> Option<u8> {
        match self {
            Self::Ascii(len) => (character < *len).then_some(1),
            Self::Unicode { lengths, .. } => lengths.get(character).map(|width| width & 7),
        }
    }
    fn retained_units(&self) -> usize {
        match self {
            Self::Ascii(_) => 0,
            Self::Unicode {
                lengths,
                checkpoints,
                ..
            } => lengths.len() + std::mem::size_of_val(&**checkpoints),
        }
    }
}

/// Logical accessible text part, not a painted TextRun. A semantic owner may
/// split this part into several runs without changing its logical coordinates.
#[derive(Debug)]
pub struct RenderedAccessiblePart {
    id: RenderedAccessiblePartId,
    bytes: Range<usize>,
    start_slot: usize,
    end_slot: usize,
    atomic: bool,
    replacement: bool,
    utf16_start: usize,
    characters: Characters,
}
impl RenderedAccessiblePart {
    pub fn id(&self) -> RenderedAccessiblePartId {
        self.id
    }
    pub fn character_count(&self) -> usize {
        self.characters.len()
    }
    pub fn utf16_range(&self) -> Range<usize> {
        self.utf16_start
            ..self.utf16_start
                + self
                    .characters
                    .boundary(self.character_count())
                    .expect("end boundary")
                    .1
    }
    /// Scalar UTF-8 lengths for AccessKit, with CRLF treated as one character.
    pub fn character_lengths(&self) -> impl Iterator<Item = u8> + '_ {
        (0..self.character_count()).map(|index| self.characters.character_length(index).unwrap())
    }

    fn position(&self, text: &RenderedText, character: usize) -> Option<RenderedTextPosition> {
        let (byte, _) = self.characters.boundary(character)?;
        if self.atomic && character != 0 && character != self.character_count() {
            return None;
        }
        let at_end = character == self.character_count();
        let byte = if self.replacement {
            self.bytes.start
        } else {
            self.bytes.start + byte
        };
        let slot = if at_end {
            self.end_slot
        } else if character == 0 {
            self.start_slot
        } else {
            0
        };
        text.position_with_slot(byte, slot)
    }
    fn character_for_position(&self, position: &RenderedTextPosition) -> Option<usize> {
        let key = position.order_key();
        if key == (self.bytes.start, self.start_slot) {
            return Some(0);
        }
        if key == (self.bytes.end, self.end_slot) {
            return Some(self.character_count());
        }
        if self.atomic || position.slot != 0 {
            return None;
        }
        self.characters
            .character(position.byte.checked_sub(self.bytes.start)?, false)
    }
}

impl RenderedText {
    pub(super) fn prepare_accessible_parts(&mut self) {
        let mut utf16_start = 0;
        let mut parts = Vec::with_capacity(self.parts.len().max(1));
        for (index, part) in self.parts.iter().enumerate() {
            let replacement = part.is_atomic() && part.bytes.is_empty();
            let text = if replacement {
                "\u{fffc}"
            } else {
                &self.text[part.bytes.clone()]
            };
            let characters = Characters::new(text);
            let count = characters
                .boundary(characters.len())
                .expect("end boundary")
                .1;
            parts.push(RenderedAccessiblePart {
                id: RenderedAccessiblePartId {
                    revision: self.identity,
                    index,
                },
                bytes: part.bytes.clone(),
                start_slot: part.start_slot,
                end_slot: part.end_slot,
                atomic: part.is_atomic(),
                replacement,
                utf16_start,
                characters,
            });
            utf16_start += count;
        }
        if parts.is_empty() {
            parts.push(RenderedAccessiblePart {
                id: RenderedAccessiblePartId {
                    revision: self.identity,
                    index: 0,
                },
                bytes: 0..0,
                start_slot: 0,
                end_slot: 0,
                atomic: false,
                replacement: false,
                utf16_start: 0,
                characters: Characters::Ascii(0),
            });
        }
        self.accessible_parts = parts.into_boxed_slice();
    }
    /// Prepared reading order, including unpainted owners and an empty-document caret.
    pub fn accessible_parts(&self) -> &[RenderedAccessiblePart] {
        &self.accessible_parts
    }
    fn accessible_part(&self, id: RenderedAccessiblePartId) -> Option<&RenderedAccessiblePart> {
        (id.revision == self.identity)
            .then(|| self.accessible_parts.get(id.index))
            .flatten()
    }
    /// An empty atomic owner has an AX-only object-replacement character. This
    /// borrowed value does not modify rendered text, clipboard data or source.
    pub fn accessible_part_text(&self, id: RenderedAccessiblePartId) -> Option<&str> {
        let part = self.accessible_part(id)?;
        Some(if part.replacement {
            "\u{fffc}"
        } else {
            &self.text[part.bytes.clone()]
        })
    }
    /// Checked scalar/CRLF position. Atomic objects accept only their two edges.
    /// The host must additionally validate native window/view/action ownership.
    pub fn accessible_position(
        &self,
        id: RenderedAccessiblePartId,
        character: usize,
    ) -> Option<RenderedTextPosition> {
        self.accessible_part(id)?.position(self, character)
    }
    /// Prefer the following part at a shared boundary; empty objects retain
    /// their ordered edges. Equal text and stale preparations are not interchangeable.
    pub fn accessible_coordinates(
        &self,
        position: &RenderedTextPosition,
    ) -> Option<(RenderedAccessiblePartId, usize)> {
        self.offset(position)?;
        let end = self
            .accessible_parts
            .partition_point(|part| (part.bytes.start, part.start_slot) <= position.order_key());
        let part = self.accessible_parts.get(end.checked_sub(1)?)?;
        Some((part.id, part.character_for_position(position)?))
    }
    /// Global AX UTF-16 coordinate, distinct from bridge UTF-8 offsets.
    pub fn accessible_utf16_offset(&self, position: &RenderedTextPosition) -> Option<usize> {
        let (id, character) = self.accessible_coordinates(position)?;
        let part = self.accessible_part(id)?;
        Some(part.utf16_start + part.characters.boundary(character)?.1)
    }
    /// Reject surrogate interiors, the middle of CRLF and atomic alternatives.
    /// Positions are bound to this exact projection; OS requests must also carry
    /// a prepared identity and pass the host's interaction guard.
    pub fn position_from_accessible_utf16(&self, offset: usize) -> Option<RenderedTextPosition> {
        let end = self
            .accessible_parts
            .partition_point(|part| part.utf16_start <= offset);
        let part = self.accessible_parts.get(end.checked_sub(1)?)?;
        let character = part
            .characters
            .character(offset.checked_sub(part.utf16_start)?, true)?;
        part.position(self, character)
    }
    pub(super) fn accessible_retained_units(&self) -> usize {
        std::mem::size_of_val(&*self.accessible_parts)
            + self
                .accessible_parts
                .iter()
                .map(|part| part.characters.retained_units())
                .sum::<usize>()
    }
    pub(super) fn accessible_max_units() -> usize {
        // Exact-capacity widths and periodic checkpoints. ASCII allocates neither.
        // Each part can add one partial checkpoint block; reserve allocator headroom.
        super::MAX_PARTS.max(1) * std::mem::size_of::<RenderedAccessiblePart>()
            + 2 * (super::MAX_BYTES
                + (super::MAX_BYTES / CHECKPOINT_STRIDE + super::MAX_PARTS)
                    * std::mem::size_of::<Boundary>())
    }
}
