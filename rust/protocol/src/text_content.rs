//! Bounded foreground runs over one logical ordinary-text source.
//! Data/codec foundation only; mounted view integration is not yet available.
use binprot::macros::BinProtWrite;

pub const MAX_TEXT_BYTES: usize = 262144;
pub const MAX_SPANS: usize = 4096;
// Three worst-case bin_prot integers per span and two length prefixes.
pub const MAX_CONFIG_BYTES: usize = MAX_TEXT_BYTES + MAX_SPANS * 27 + 18;

#[derive(Clone, Debug, PartialEq, Eq, BinProtWrite)]
pub struct Span {
    pub start_byte: i64,
    pub end_byte: i64,
    pub foreground: i64,
}

impl Span {
    pub fn is_valid(&self) -> bool {
        self.start_byte >= 0
            && self.end_byte > self.start_byte
            && self.end_byte <= MAX_TEXT_BYTES as i64
            && (0..=0xffff_ffff).contains(&self.foreground)
    }
}

#[derive(Clone, Debug, PartialEq, Eq, BinProtWrite)]
pub struct Content {
    pub text: String,
    pub spans: Vec<Span>,
}

impl Content {
    pub fn is_valid(&self) -> bool {
        if self.text.len() > MAX_TEXT_BYTES || self.spans.len() > MAX_SPANS {
            return false;
        }
        let mut previous_end = 0;
        for span in &self.spans {
            if !span.is_valid()
                || span.start_byte < previous_end
                || span.end_byte > self.text.len() as i64
                || !self.text.is_char_boundary(span.start_byte as usize)
                || !self.text.is_char_boundary(span.end_byte as usize)
            {
                return false;
            }
            previous_end = span.end_byte;
        }
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{DecodeError, decode_text_content};
    use binprot::BinProtWrite;

    fn fixture() -> Content {
        Content {
            text: "Aé世界".into(),
            spans: vec![
                Span {
                    start_byte: 1,
                    end_byte: 3,
                    foreground: 0x11223344,
                },
                Span {
                    start_byte: 3,
                    end_byte: 9,
                    foreground: 0xaabbccdd,
                },
            ],
        }
    }

    fn encode(content: &Content) -> Vec<u8> {
        let mut bytes = vec![];
        content.binprot_write(&mut bytes).unwrap();
        bytes
    }

    #[test]
    fn text_content_fixture_agrees_with_ocaml_and_rejects_framing_errors() {
        let content = fixture();
        assert!(content.is_valid());
        let bytes = encode(&content);
        assert_eq!(
            bytes
                .iter()
                .map(|byte| format!("{byte:02x}"))
                .collect::<String>(),
            "0941c3a9e4b896e7958c020103fd443322110309fcddccbbaa00000000"
        );
        assert_eq!(decode_text_content(&bytes), Ok(content));
        for length in 0..bytes.len() {
            assert!(
                decode_text_content(&bytes[..length]).is_err(),
                "prefix {length}"
            );
        }
        let mut trailing = bytes.clone();
        trailing.push(0);
        assert_eq!(decode_text_content(&trailing), Err(DecodeError::Malformed));
        let mut invalid_utf8 = bytes;
        invalid_utf8[1] = 0xff;
        assert_eq!(
            decode_text_content(&invalid_utf8),
            Err(DecodeError::Malformed)
        );
    }

    #[test]
    fn text_content_rejects_unordered_overlapping_and_non_scalar_ranges() {
        for (start, end) in [
            (-1, 3),
            (1, 1),
            (2, 3),
            (1, 2),
            (1, 10),
            (i64::MAX, i64::MAX),
        ] {
            let mut content = fixture();
            content.spans[0].start_byte = start;
            content.spans[0].end_byte = end;
            assert!(!content.is_valid());
            assert!(decode_text_content(&encode(&content)).is_err());
        }
        for foreground in [-1, 0x1_0000_0000] {
            let mut content = fixture();
            content.spans[0].foreground = foreground;
            assert!(!content.is_valid());
            assert!(decode_text_content(&encode(&content)).is_err());
        }
        let mut content = fixture();
        content.spans.swap(0, 1);
        assert!(!content.is_valid());
        content.spans.swap(0, 1);
        content.spans[1].start_byte = 1;
        assert!(!content.is_valid());
        assert!(decode_text_content(&encode(&content)).is_err());
    }

    #[test]
    fn text_content_limits_bound_declared_allocations_and_allow_exact_edges() {
        let content = Content {
            text: "x".repeat(MAX_TEXT_BYTES),
            spans: (0..MAX_SPANS)
                .map(|offset| Span {
                    start_byte: offset as i64,
                    end_byte: offset as i64 + 1,
                    foreground: 0xffff_ffff,
                })
                .collect(),
        };
        assert!(content.is_valid());
        assert_eq!(decode_text_content(&encode(&content)), Ok(content.clone()));
        let mut too_many = content;
        too_many.spans.push(Span {
            start_byte: MAX_SPANS as i64,
            end_byte: MAX_SPANS as i64 + 1,
            foreground: 0,
        });
        assert!(!too_many.is_valid());
        assert_eq!(
            decode_text_content(&encode(&too_many)),
            Err(DecodeError::LimitExceeded)
        );
        let mut declared_text = vec![];
        binprot::Nat0(MAX_TEXT_BYTES as u64 + 1)
            .binprot_write(&mut declared_text)
            .unwrap();
        assert_eq!(
            decode_text_content(&declared_text),
            Err(DecodeError::LimitExceeded)
        );
        assert_eq!(
            decode_text_content(&vec![0; MAX_CONFIG_BYTES + 1]),
            Err(DecodeError::LimitExceeded)
        );
        assert_eq!(
            decode_text_content(&[0, 0]),
            Ok(Content {
                text: String::new(),
                spans: vec![]
            })
        );
    }
}
