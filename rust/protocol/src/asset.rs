//! Immutable encoded sources. Format declaration and byte bounds do not decode
//! content, perform I/O, or establish native registration/lifetime.
use std::fmt;

pub const MAX_ENCODED_BYTES: usize = 16 * 1024 * 1024;

#[derive(
    Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, binprot::macros::BinProtWrite,
)]
pub enum Format {
    Png,
    Jpeg,
    Webp,
    Gif,
    Svg,
    Bmp,
    Tiff,
    Ico,
    Pnm,
}
impl Format {
    pub const fn mime_type(self) -> &'static str {
        match self {
            Self::Png => "image/png",
            Self::Jpeg => "image/jpeg",
            Self::Webp => "image/webp",
            Self::Gif => "image/gif",
            Self::Svg => "image/svg+xml",
            Self::Bmp => "image/bmp",
            Self::Tiff => "image/tiff",
            Self::Ico => "image/ico",
            Self::Pnm => "image/x-portable-anymap",
        }
    }
}

#[derive(Clone, PartialEq, Eq)]
pub struct Source {
    format: Format,
    data: Vec<u8>,
}
impl Source {
    pub fn new(format: Format, data: Vec<u8>) -> Result<Self, &'static str> {
        if data.is_empty() || data.len() > MAX_ENCODED_BYTES {
            return Err("encoded asset must contain 1..16777216 bytes");
        }
        Ok(Self { format, data })
    }
    pub fn format(&self) -> Format {
        self.format
    }
    pub fn as_bytes(&self) -> &[u8] {
        &self.data
    }
    pub fn byte_length(&self) -> usize {
        self.data.len()
    }
}
impl fmt::Debug for Source {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("Source")
            .field("format", &self.format)
            .field("byte_length", &self.data.len())
            .finish()
    }
}

/// One transport chunk uses bin_prot string encoding (raw bytes, not integers).
pub const MAX_CHUNK_BYTES: usize = 256 * 1024;

#[derive(Clone, Copy, Debug, PartialEq, Eq, binprot::macros::BinProtWrite)]
pub enum Error {
    Closed,
    InvalidSize,
    ResourceLimit,
    StaleHandle,
    NotUploading,
    InvalidChunk,
    Incomplete,
    NotReady,
    NativeFailure,
}

#[derive(Clone, PartialEq, Eq)]
pub struct Chunk(Vec<u8>);
impl Chunk {
    pub fn new(bytes: Vec<u8>) -> Result<Self, Error> {
        if bytes.len() > MAX_CHUNK_BYTES {
            Err(Error::InvalidChunk)
        } else {
            Ok(Self(bytes))
        }
    }
    pub fn as_bytes(&self) -> &[u8] {
        &self.0
    }
}
impl fmt::Debug for Chunk {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("Chunk")
            .field("byte_length", &self.0.len())
            .finish()
    }
}
impl binprot::BinProtWrite for Chunk {
    fn binprot_write<W: std::io::Write>(&self, writer: &mut W) -> std::io::Result<()> {
        binprot::Nat0(self.0.len() as u64).binprot_write(writer)?;
        writer.write_all(&self.0)
    }
}

#[derive(Clone, Debug, PartialEq, Eq, binprot::macros::BinProtWrite)]
pub enum Request {
    Begin(Format, i64),
    Append(crate::ResourceId, i64, Chunk),
    Finish(crate::ResourceId),
    Release(crate::ResourceId),
}
#[derive(Clone, Debug, PartialEq, Eq, binprot::macros::BinProtWrite)]
pub enum Response {
    Begun(crate::ResourceId),
    Ack,
    Failed(Error),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn opaque_sources_validate_bounds_without_decoding() {
        let bytes = vec![0, 255, 128, b'X'];
        let source = Source::new(Format::Png, bytes.clone()).unwrap();
        assert_eq!(source.as_bytes(), bytes);
        assert_eq!(source.format(), Format::Png);
        assert_eq!(source.byte_length(), 4);
        assert_eq!(
            format!("{source:?}"),
            "Source { format: Png, byte_length: 4 }"
        );
        assert_ne!(source, Source::new(Format::Svg, bytes).unwrap());
        assert!(Source::new(Format::Png, vec![]).is_err());
        assert!(Source::new(Format::Png, vec![0; MAX_ENCODED_BYTES]).is_ok());
        assert!(Source::new(Format::Png, vec![0; MAX_ENCODED_BYTES + 1]).is_err());
    }
}
