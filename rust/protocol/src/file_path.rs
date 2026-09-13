//! Exact Unix path bytes shared by file dialogs and file drag/drop. This module
//! performs no filesystem I/O and does not grant authority to access a path.
use binprot::BinProtWrite;

pub const MAX_PATH_BYTES: usize = 16_384;

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct FilePath(Vec<u8>);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PathError {
    TooLong,
    NotAbsolute,
    ContainsNul,
}

impl FilePath {
    pub fn new(bytes: Vec<u8>) -> Result<Self, PathError> {
        if bytes.len() > MAX_PATH_BYTES {
            Err(PathError::TooLong)
        } else if bytes.first() != Some(&b'/') {
            Err(PathError::NotAbsolute)
        } else if bytes.contains(&0) {
            Err(PathError::ContainsNul)
        } else {
            Ok(Self(bytes))
        }
    }

    pub fn as_bytes(&self) -> &[u8] {
        &self.0
    }

    pub fn into_bytes(self) -> Vec<u8> {
        self.0
    }
}

impl BinProtWrite for FilePath {
    fn binprot_write<W: std::io::Write>(&self, writer: &mut W) -> std::io::Result<()> {
        // OCaml strings use a Nat0 byte length followed by raw bytes, not the
        // integer-per-element encoding of the generic Vec<u8> implementation.
        binprot::Nat0(self.0.len() as u64).binprot_write(writer)?;
        writer.write_all(&self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use binprot::Bytes;

    #[test]
    fn preserves_native_bytes_and_validates_without_filesystem_access() {
        for bytes in [
            b"/".to_vec(),
            b"//a/../b".to_vec(),
            b"/tmp/\xff.txt".to_vec(),
        ] {
            let path = FilePath::new(bytes.clone()).unwrap();
            assert_eq!(path.as_bytes(), bytes);
            assert_eq!(path.into_bytes(), bytes);
        }
        for bytes in [b"".to_vec(), b"a/b".to_vec(), b"~/file".to_vec()] {
            assert_eq!(FilePath::new(bytes), Err(PathError::NotAbsolute));
        }
        assert_eq!(
            FilePath::new(b"/tmp/\0secret".to_vec()),
            Err(PathError::ContainsNul)
        );
        let mut limit = vec![b'a'; MAX_PATH_BYTES];
        limit[0] = b'/';
        assert!(FilePath::new(limit.clone()).is_ok());
        limit.push(b'a');
        assert_eq!(FilePath::new(limit), Err(PathError::TooLong));
    }

    #[test]
    fn encoding_matches_ocaml_string_bytes_including_invalid_utf8() {
        let bytes = b"/tmp/\xff.txt".to_vec();
        let path = FilePath::new(bytes.clone()).unwrap();
        let mut output = vec![];
        path.binprot_write(&mut output).unwrap();
        assert_eq!(output, b"\x0a/tmp/\xff.txt");
        let mut expected = vec![];
        Bytes(bytes).binprot_write(&mut expected).unwrap();
        assert_eq!(output, expected);
    }
}
