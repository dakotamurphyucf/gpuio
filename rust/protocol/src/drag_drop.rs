//! Immutable drag payloads and synchronous acceptance policy. These constructors
//! do no I/O; native gesture ownership and OS capabilities belong to the host.
use crate::file_path::FilePath;
use binprot::{BinProtWrite, macros::BinProtWrite};

pub const MAX_DATA_BYTES: usize = 262_144;
pub const MAX_FILES: usize = 128;
pub const MAX_FORMATS: usize = 16;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InvalidDragData {
    Kind,
    Text,
    Files,
    CustomData,
    Label,
    DesktopFiles,
    Formats,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct CustomKind(String);

impl BinProtWrite for CustomKind {
    fn binprot_write<W: std::io::Write>(&self, writer: &mut W) -> std::io::Result<()> {
        self.0.binprot_write(writer)
    }
}

impl CustomKind {
    pub fn new(value: String) -> Result<Self, InvalidDragData> {
        if value.is_empty()
            || value.len() > 128
            || !value
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'.' | b'_' | b'-' | b'/' | b'+'))
        {
            Err(InvalidDragData::Kind)
        } else {
            Ok(Self(value))
        }
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, BinProtWrite)]
pub enum Format {
    Text,
    Files,
    Custom(CustomKind),
}

#[derive(Clone, Debug, PartialEq, Eq, BinProtWrite)]
pub struct File {
    pub path: FilePath,
    /// None for unknown native metadata. No implicit filesystem lookup.
    pub is_directory: Option<bool>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum Data {
    Text(String),
    Files(Vec<File>),
    Custom { kind: CustomKind, data: Vec<u8> },
}

/// Private representation prevents bypassing constructors in the native host.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Payload(Data);

impl BinProtWrite for Payload {
    fn binprot_write<W: std::io::Write>(&self, writer: &mut W) -> std::io::Result<()> {
        match &self.0 {
            Data::Text(text) => {
                writer.write_all(&[0])?;
                text.binprot_write(writer)
            }
            Data::Files(files) => {
                writer.write_all(&[1])?;
                files.binprot_write(writer)
            }
            Data::Custom { kind, data } => {
                writer.write_all(&[2])?;
                kind.binprot_write(writer)?;
                // OCaml string encoding: one byte length followed by raw bytes
                // for short data, Nat0's wider encoding for longer data. Never
                // use Vec<u8>'s integer-per-element representation here.
                binprot::Nat0(data.len() as u64).binprot_write(writer)?;
                writer.write_all(data)
            }
        }
    }
}

pub enum PayloadRef<'a> {
    Text(&'a str),
    Files(&'a [File]),
    Custom {
        kind: &'a CustomKind,
        data: &'a [u8],
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, BinProtWrite)]
pub enum Origin {
    Internal,
    Desktop,
}

#[derive(Clone, Debug, PartialEq, Eq, BinProtWrite)]
pub struct Offer {
    pub format: Format,
    pub data_bytes: i64,
    pub file_count: i64,
    pub origin: Origin,
}

impl Offer {
    pub fn new(payload: &Payload, origin: Origin) -> Self {
        Self {
            format: payload.format(),
            data_bytes: payload.data_bytes() as i64,
            file_count: match payload.as_ref() {
                PayloadRef::Files(files) => files.len() as i64,
                _ => 0,
            },
            origin,
        }
    }

    pub fn is_valid(&self) -> bool {
        (0..=MAX_DATA_BYTES as i64).contains(&self.data_bytes)
            && match self.format {
                Format::Files => {
                    (1..=MAX_FILES as i64).contains(&self.file_count)
                        && self.data_bytes >= self.file_count
                }
                Format::Text | Format::Custom(_) => self.file_count == 0,
            }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, BinProtWrite)]
pub enum CancelReason {
    Escape,
    Hidden,
    Blocked,
    Disabled,
    Removed,
    Reconfigured,
    WindowClosed,
    WindowInactive,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, BinProtWrite)]
pub enum Outcome {
    InternalDrop,
    Cancelled(CancelReason),
    Unconfirmed,
}

#[derive(Clone, Debug, PartialEq, Eq, BinProtWrite)]
pub enum SourcePhase {
    Started(Payload),
    DesktopOffered,
    DesktopUnavailable,
    Ended(Outcome),
}

#[derive(Clone, Debug, PartialEq, Eq, BinProtWrite)]
pub struct SourceSample {
    pub gesture: i64,
    pub phase: SourcePhase,
}

impl SourceSample {
    pub fn is_valid(&self) -> bool {
        self.gesture > 0
    }
    pub fn payload_bytes(&self) -> usize {
        match &self.phase {
            SourcePhase::Started(payload) => payload.retained_bytes(),
            _ => 0,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, BinProtWrite)]
pub enum Rejection {
    InvalidData,
    LimitExceeded,
}

#[derive(Clone, Debug, PartialEq, Eq, BinProtWrite)]
pub enum TargetPhase {
    Entered(Offer),
    Moved,
    Left,
    Dropped(Payload),
    Rejected(Rejection),
}

#[derive(Clone, Debug, PartialEq, BinProtWrite)]
pub struct TargetSample {
    pub gesture: i64,
    pub phase: TargetPhase,
    pub window_x: f64,
    pub window_y: f64,
    pub local_x: f64,
    pub local_y: f64,
    pub modifiers: crate::pointer::PointerModifiers,
}

impl TargetSample {
    pub fn is_valid(&self) -> bool {
        self.gesture > 0
            && [self.window_x, self.window_y, self.local_x, self.local_y]
                .iter()
                .all(|v| v.is_finite())
            && match &self.phase {
                TargetPhase::Entered(offer) => offer.is_valid(),
                _ => true,
            }
    }
    pub fn payload_bytes(&self) -> usize {
        match &self.phase {
            TargetPhase::Dropped(payload) => payload.retained_bytes(),
            TargetPhase::Entered(Offer {
                format: Format::Custom(kind),
                ..
            }) => kind.as_str().len(),
            _ => 0,
        }
    }
}

impl Payload {
    pub fn text(text: String) -> Result<Self, InvalidDragData> {
        if text.len() > MAX_DATA_BYTES || text.contains('\0') {
            Err(InvalidDragData::Text)
        } else {
            Ok(Self(Data::Text(text)))
        }
    }

    pub fn files(files: Vec<File>) -> Result<Self, InvalidDragData> {
        if files.is_empty()
            || files.len() > MAX_FILES
            || files.iter().map(|f| f.path.as_bytes().len()).sum::<usize>() > MAX_DATA_BYTES
        {
            Err(InvalidDragData::Files)
        } else {
            Ok(Self(Data::Files(files)))
        }
    }

    pub fn custom(kind: CustomKind, data: Vec<u8>) -> Result<Self, InvalidDragData> {
        if data.len() > MAX_DATA_BYTES {
            Err(InvalidDragData::CustomData)
        } else {
            Ok(Self(Data::Custom { kind, data }))
        }
    }

    pub fn as_ref(&self) -> PayloadRef<'_> {
        match &self.0 {
            Data::Text(text) => PayloadRef::Text(text),
            Data::Files(files) => PayloadRef::Files(files),
            Data::Custom { kind, data } => PayloadRef::Custom { kind, data },
        }
    }

    pub fn format(&self) -> Format {
        match &self.0 {
            Data::Text(_) => Format::Text,
            Data::Files(_) => Format::Files,
            Data::Custom { kind, .. } => Format::Custom(kind.clone()),
        }
    }

    /// Data bytes only, excluding file metadata, custom kind and wire overhead.
    pub fn data_bytes(&self) -> usize {
        match &self.0 {
            Data::Text(text) => text.len(),
            Data::Files(files) => files.iter().map(|f| f.path.as_bytes().len()).sum(),
            Data::Custom { data, .. } => data.len(),
        }
    }

    /// Conservative retained/encoded variable-data bound, including metadata.
    pub fn retained_bytes(&self) -> usize {
        std::mem::size_of::<Self>()
            + self.data_bytes()
            + match self.as_ref() {
                PayloadRef::Files(files) => files.len() * (std::mem::size_of::<File>() + 16),
                PayloadRef::Custom { kind, .. } => kind.as_str().len() + 32,
                PayloadRef::Text(_) => 16,
            }
    }

    fn can_offer_desktop_files(&self) -> bool {
        matches!(&self.0, Data::Files(files) if files.iter().all(|f| f.is_directory.is_some()))
    }
}

fn valid_label(label: &str) -> bool {
    // Match Core.String.strip's ASCII whitespace vocabulary exactly.
    !label
        .trim_matches([' ', '\t', '\n', '\r', '\x0b', '\x0c'])
        .is_empty()
        && label.len() <= 4096
        && !label.contains('\0')
}

#[derive(Clone, Debug, PartialEq, Eq, BinProtWrite)]
pub struct Source {
    label: String,
    payload: Payload,
    disabled: bool,
    allow_desktop_files: bool,
}

impl Source {
    pub fn decode(bytes: &[u8]) -> Result<Self, crate::DecodeError> {
        crate::decode::decode_drag_source(bytes)
    }

    pub fn new(
        label: String,
        payload: Payload,
        disabled: bool,
        allow_desktop_files: bool,
    ) -> Result<Self, InvalidDragData> {
        if !valid_label(&label) {
            Err(InvalidDragData::Label)
        } else if allow_desktop_files && !payload.can_offer_desktop_files() {
            Err(InvalidDragData::DesktopFiles)
        } else {
            Ok(Self {
                label,
                payload,
                disabled,
                allow_desktop_files,
            })
        }
    }

    pub fn label(&self) -> &str {
        &self.label
    }

    pub fn payload(&self) -> &Payload {
        &self.payload
    }

    pub fn disabled(&self) -> bool {
        self.disabled
    }

    pub fn allow_desktop_files(&self) -> bool {
        self.allow_desktop_files
    }

    pub fn retained_bytes(&self) -> usize {
        std::mem::size_of::<Self>() + self.label.len() + self.payload.retained_bytes()
    }
}

#[derive(Clone, Debug, PartialEq, Eq, BinProtWrite)]
pub struct Target {
    label: String,
    accepted_formats: Vec<Format>,
    disabled: bool,
}

impl Target {
    pub fn decode(bytes: &[u8]) -> Result<Self, crate::DecodeError> {
        crate::decode::decode_drag_target(bytes)
    }

    pub fn new(
        label: String,
        accepted_formats: Vec<Format>,
        disabled: bool,
    ) -> Result<Self, InvalidDragData> {
        if !valid_label(&label) {
            return Err(InvalidDragData::Label);
        }
        if accepted_formats.is_empty()
            || accepted_formats.len() > MAX_FORMATS
            || accepted_formats
                .iter()
                .enumerate()
                .any(|(i, f)| accepted_formats[..i].contains(f))
        {
            return Err(InvalidDragData::Formats);
        }
        Ok(Self {
            label,
            accepted_formats,
            disabled,
        })
    }

    pub fn label(&self) -> &str {
        &self.label
    }

    pub fn accepted_formats(&self) -> &[Format] {
        &self.accepted_formats
    }

    pub fn disabled(&self) -> bool {
        self.disabled
    }

    pub fn retained_bytes(&self) -> usize {
        std::mem::size_of::<Self>()
            + self.label.len()
            + self
                .accepted_formats
                .iter()
                .map(|format| {
                    std::mem::size_of::<Format>()
                        + match format {
                            Format::Custom(kind) => kind.as_str().len(),
                            _ => 0,
                        }
                })
                .sum::<usize>()
    }

    pub fn accepts(&self, payload: &Payload) -> bool {
        // No custom-kind allocation on motion/hover acceptance checks.
        !self.disabled
            && self
                .accepted_formats
                .iter()
                .any(|format| match (format, payload.as_ref()) {
                    (Format::Text, PayloadRef::Text(_)) | (Format::Files, PayloadRef::Files(_)) => {
                        true
                    }
                    (Format::Custom(expected), PayloadRef::Custom { kind, .. }) => expected == kind,
                    _ => false,
                })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn file(bytes: &[u8], is_directory: Option<bool>) -> File {
        File {
            path: FilePath::new(bytes.to_vec()).unwrap(),
            is_directory,
        }
    }

    #[test]
    fn native_paths_and_opaque_bytes_are_preserved_without_normalization() {
        let files = vec![file(b"//tmp/../\xff", None), file(b"/tmp/a", Some(false))];
        let payload = Payload::files(files.clone()).unwrap();
        let PayloadRef::Files(actual) = payload.as_ref() else {
            panic!()
        };
        assert_eq!(actual, files);
        let kind = CustomKind::new("com.example.task/v1".into()).unwrap();
        let bytes = vec![0, 255, 128, 65];
        let payload = Payload::custom(kind.clone(), bytes.clone()).unwrap();
        let PayloadRef::Custom {
            kind: actual_kind,
            data,
        } = payload.as_ref()
        else {
            panic!()
        };
        assert_eq!(actual_kind, &kind);
        assert_eq!(data, bytes);
        assert_eq!(payload.data_bytes(), 4);
    }

    #[test]
    fn payload_limits_are_inclusive_and_reject_whole_values() {
        assert!(Payload::text(String::new()).is_ok());
        assert!(Payload::text("a".repeat(MAX_DATA_BYTES)).is_ok());
        assert_eq!(
            Payload::text("a".repeat(MAX_DATA_BYTES + 1)),
            Err(InvalidDragData::Text)
        );
        assert_eq!(Payload::text("a\0b".into()), Err(InvalidDragData::Text));
        assert_eq!(Payload::files(vec![]), Err(InvalidDragData::Files));
        let short = file(b"/a", None);
        assert!(Payload::files(vec![short.clone(); MAX_FILES]).is_ok());
        assert_eq!(
            Payload::files(vec![short; MAX_FILES + 1]),
            Err(InvalidDragData::Files)
        );
        let long = file(&vec![b'/'; 16_384], Some(true));
        assert_eq!(
            Payload::files(vec![long.clone(); 16]).unwrap().data_bytes(),
            MAX_DATA_BYTES
        );
        assert_eq!(Payload::files(vec![long; 17]), Err(InvalidDragData::Files));
        let kind = CustomKind::new("bytes".into()).unwrap();
        assert!(Payload::custom(kind.clone(), vec![255; MAX_DATA_BYTES]).is_ok());
        assert_eq!(
            Payload::custom(kind, vec![255; MAX_DATA_BYTES + 1]),
            Err(InvalidDragData::CustomData)
        );
    }

    #[test]
    fn desktop_export_requires_metadata_and_never_silently_drops_entries() {
        let known = file(b"/tmp/a", Some(false));
        let unknown = file(b"/tmp/b", None);
        let mixed = Payload::files(vec![known.clone(), unknown]).unwrap();
        assert!(Source::new("Files".into(), mixed.clone(), false, false).is_ok());
        assert_eq!(
            Source::new("Files".into(), mixed, false, true),
            Err(InvalidDragData::DesktopFiles)
        );
        assert!(
            Source::new(
                "Files".into(),
                Payload::files(vec![known]).unwrap(),
                false,
                true
            )
            .is_ok()
        );
        assert_eq!(
            Source::new(
                "Text".into(),
                Payload::text("x".into()).unwrap(),
                false,
                true
            ),
            Err(InvalidDragData::DesktopFiles)
        );
    }

    #[test]
    fn target_acceptance_is_exact_and_disabled_is_authoritative() {
        let a = CustomKind::new("example/a".into()).unwrap();
        let b = CustomKind::new("example/A".into()).unwrap();
        let payload = Payload::custom(a.clone(), vec![]).unwrap();
        let formats = vec![Format::Text, Format::Custom(a.clone())];
        assert!(
            Target::new("Target".into(), formats.clone(), false)
                .unwrap()
                .accepts(&payload)
        );
        assert!(
            !Target::new("Target".into(), formats, true)
                .unwrap()
                .accepts(&payload)
        );
        assert!(
            !Target::new("Target".into(), vec![Format::Custom(b)], false)
                .unwrap()
                .accepts(&payload)
        );
        for formats in [vec![], vec![Format::Files; 2]] {
            assert_eq!(
                Target::new("Target".into(), formats, false),
                Err(InvalidDragData::Formats)
            );
        }
        let formats: Vec<_> = (0..17)
            .map(|i| Format::Custom(CustomKind::new(format!("kind/{i}")).unwrap()))
            .collect();
        assert!(Target::new("Target".into(), formats[..16].to_vec(), false).is_ok());
        assert_eq!(
            Target::new("Target".into(), formats, false),
            Err(InvalidDragData::Formats)
        );
    }

    #[test]
    fn labels_and_identifiers_have_distinct_rules() {
        for value in ["", "with space", "λ", "a\0b", "a:b", &"x".repeat(129)] {
            assert_eq!(CustomKind::new(value.into()), Err(InvalidDragData::Kind));
        }
        for value in ["application/x-a.v1+bytes", "A_b/1", &"x".repeat(128)] {
            assert!(CustomKind::new(value.into()).is_ok());
        }
        for label in ["", " \t\n\r\x0b\x0c", "a\0b", &"x".repeat(4097)] {
            assert_eq!(
                Target::new(label.into(), vec![Format::Text], false),
                Err(InvalidDragData::Label)
            );
        }
        for label in ["λ", "\u{a0}", &"x".repeat(4096)] {
            assert!(Target::new(label.into(), vec![Format::Text], false).is_ok());
        }
    }
}
