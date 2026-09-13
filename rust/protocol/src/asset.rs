//! Immutable encoded sources. Format declaration and byte bounds do not decode
//! content, perform I/O, or establish native registration/lifetime.
use std::fmt;

pub const MAX_ENCODED_BYTES: usize = 16 * 1024 * 1024;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
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
