//! Revisioned display sources. Updates stage a UTF-8 suffix and publish it
//! atomically; the old snapshot stays visible until publication succeeds.
use crate::ResourceId;

pub const MAX_BYTES: usize = 8 * 1024 * 1024;
pub const MAX_CHUNK_BYTES: usize = 256 * 1024;

#[derive(Clone, Copy, Debug, PartialEq, Eq, binprot::macros::BinProtWrite)]
pub enum Status {
    Streaming,
    Complete,
    Cancelled,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, binprot::macros::BinProtWrite)]
pub enum Error {
    Closed,
    ResourceLimit,
    StaleHandle,
    InvalidRevision,
    InvalidRange,
    InvalidUtf8,
    Incomplete,
    Busy,
    NotReady,
    NativeFailure,
}

#[derive(Clone, Debug, PartialEq, Eq, binprot::macros::BinProtWrite)]
pub struct Update {
    pub id: ResourceId,
    pub base: i64,
    pub revision: i64,
    pub generation: i64,
    pub from_byte: i64,
    pub suffix_bytes: i64,
    pub status: Status,
}

#[derive(Clone, Debug, PartialEq, Eq, binprot::macros::BinProtWrite)]
pub enum Request {
    Create,
    Begin(Update),
    Chunk(ResourceId, i64, i64, crate::asset::Chunk),
    Publish(ResourceId, i64),
    Abort(ResourceId, i64),
    Release(ResourceId),
}

#[derive(Clone, Debug, PartialEq, Eq, binprot::macros::BinProtWrite)]
pub enum Response {
    Created(ResourceId),
    Ack,
    Failed(Error),
}

#[derive(Clone, Debug, PartialEq, Eq, binprot::macros::BinProtWrite)]
pub enum Mode {
    Markdown,
    Code(String),
    Diff,
}
#[derive(Clone, Debug, PartialEq, binprot::macros::BinProtWrite)]
pub enum Layout {
    Flow,
    Viewport(f64),
}
#[derive(Clone, Debug, PartialEq, binprot::macros::BinProtWrite)]
pub struct Config {
    pub source: Option<ResourceId>,
    pub mode: Mode,
    pub dark: bool,
    pub layout: Layout,
    pub label: String,
    pub path: Option<String>,
    pub line_numbers: bool,
    pub initially_collapsed: bool,
    pub search: String,
    pub images: Vec<(String, crate::image::ImageSource)>,
}
impl Config {
    pub fn is_valid(&self) -> bool {
        let valid = |text: &str, max| text.len() <= max && !text.contains('\0');
        !self.label.is_empty()
            && valid(&self.label, 1024)
            && self.path.as_ref().is_none_or(|path| valid(path, 4096))
            && valid(&self.search, 4096)
            && match &self.mode {
                Mode::Code(token) => {
                    !token.is_empty()
                        && token.len() <= 64
                        && token
                            .bytes()
                            .all(|b| b.is_ascii_alphanumeric() || b"_+-.#".contains(&b))
                }
                _ => true,
            }
            && match self.layout {
                Layout::Flow => true,
                Layout::Viewport(h) => h.is_finite() && (1.0..=16384.0).contains(&h),
            }
            && self.images.len() <= 128
            && self
                .images
                .iter()
                .all(|(url, _)| !url.is_empty() && valid(url, 4096))
            && self
                .images
                .iter()
                .map(|(url, _)| url)
                .collect::<std::collections::BTreeSet<_>>()
                .len()
                == self.images.len()
    }
    pub fn retained_bytes(&self) -> usize {
        std::mem::size_of::<Self>()
            + self.label.len()
            + self.search.len()
            + self.path.as_ref().map_or(0, String::len)
            + match &self.mode {
                Mode::Code(token) => token.len(),
                _ => 0,
            }
            + self
                .images
                .iter()
                .map(|(url, _)| {
                    url.len() + std::mem::size_of::<(String, crate::image::ImageSource)>()
                })
                .sum::<usize>()
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, binprot::macros::BinProtWrite)]
pub enum Side {
    Before,
    After,
}
#[derive(Clone, Debug, PartialEq, Eq, binprot::macros::BinProtWrite)]
pub enum Navigation {
    Link(String),
    Line(Option<String>, Side, i64),
}

impl Navigation {
    pub fn is_valid(&self) -> bool {
        let valid = |value: &str| value.len() <= 4096 && !value.contains('\0');
        match self {
            Self::Link(url) => !url.is_empty() && valid(url),
            Self::Line(path, _, line) => {
                *line > 0 && *line <= i32::MAX as i64 && path.as_deref().is_none_or(valid)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use binprot::BinProtWrite;
    #[test]
    fn ocaml_document_fixtures_fix_tags_order_and_raw_utf8() {
        let id = ResourceId::from_parts(0, 1).unwrap();
        let cases = [
            (Request::Create, vec![10, 7, 0]),
            (
                Request::Begin(Update {
                    id,
                    base: 0,
                    revision: 1,
                    generation: 1,
                    from_byte: 0,
                    suffix_bytes: 2,
                    status: Status::Streaming,
                }),
                vec![10, 7, 1, 0, 1, 0, 1, 1, 0, 2, 0],
            ),
            (
                Request::Chunk(
                    id,
                    1,
                    0,
                    crate::asset::Chunk::new(vec![0xce, 0xbb]).unwrap(),
                ),
                vec![10, 7, 2, 0, 1, 1, 0, 2, 0xce, 0xbb],
            ),
            (Request::Publish(id, 1), vec![10, 7, 3, 0, 1, 1]),
            (Request::Abort(id, 1), vec![10, 7, 4, 0, 1, 1]),
            (Request::Release(id), vec![10, 7, 5, 0, 1]),
        ];
        for (request, bytes) in cases {
            let message = crate::v1::Message::Document(7, request);
            assert_eq!(crate::decode(&bytes).unwrap(), message);
            let mut actual = Vec::new();
            message.binprot_write(&mut actual).unwrap();
            assert_eq!(actual, bytes);
            let mut trailing = bytes.clone();
            trailing.push(0);
            assert!(crate::decode(&trailing).is_err());
            for end in 0..bytes.len() {
                assert!(crate::decode(&bytes[..end]).is_err());
            }
        }
        assert!(crate::decode(&[10, 0, 0]).is_err());
        assert!(crate::decode(&[10, 7, 6]).is_err());
    }
}

#[cfg(test)]
mod presentation_fixtures {
    use super::*;
    use crate::{
        HandlerId, NodeId, WindowId,
        v1::{Event, Kind, Message, Op},
    };
    use binprot::BinProtWrite;
    fn bytes(hex: &str) -> Vec<u8> {
        (0..hex.len())
            .step_by(2)
            .map(|i| u8::from_str_radix(&hex[i..i + 2], 16).unwrap())
            .collect()
    }
    #[test]
    fn ocaml_presentation_and_generation_navigation() {
        let fixtures = [
            "0300010001030000011a000100012100010100010000000164000100000006010001",
            "0300010001030000011a0001000121000101000101026d6c01010000000000c0724001640104612e6d6c010002cebb0006010001",
            "0300010001030000011a00010001210001000200000164000101000006010001",
        ];
        for (index, hex) in fixtures.iter().enumerate() {
            let raw = bytes(hex);
            let message = crate::decode(&raw).unwrap();
            let Message::Apply(tx) = &message else {
                panic!("apply");
            };
            assert!(matches!(
                tx.operations[0],
                Op::Create(_, Kind::DocumentView, _, _)
            ));
            let Op::SetDocument(_, config) = &tx.operations[1] else {
                panic!("config");
            };
            assert_eq!(
                config.mode,
                match index {
                    0 => Mode::Markdown,
                    1 => Mode::Code("ml".into()),
                    _ => Mode::Diff,
                }
            );
            assert_eq!(config.dark, index == 1);
            assert_eq!(config.search, if index == 1 { "λ" } else { "" });
            assert_eq!(config.initially_collapsed, index == 2);
            let mut out = Vec::new();
            message.binprot_write(&mut out).unwrap();
            assert_eq!(out, raw);
            for end in 0..raw.len() {
                assert!(crate::decode(&raw[..end]).is_err());
            }
        }
        let id = ResourceId::from_parts(0, 1).unwrap();
        for (navigation, hex) in [
            (
                Navigation::Link("https://example.test".into()),
                "011e00010001000101000103001468747470733a2f2f6578616d706c652e74657374",
            ),
            (
                Navigation::Line(Some("a.ml".into()), Side::Before, 9),
                "011e00010001000101000103010104612e6d6c0009",
            ),
        ] {
            let events = vec![Event::DocumentNavigation(
                WindowId::from_parts(0, 1).unwrap(),
                NodeId::from_parts(0, 1).unwrap(),
                HandlerId::from_parts(0, 1).unwrap(),
                1,
                id,
                3,
                navigation,
            )];
            let mut out = Vec::new();
            events.binprot_write(&mut out).unwrap();
            assert_eq!(out, bytes(hex));
        }
    }
}
