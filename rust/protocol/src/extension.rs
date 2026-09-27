//! Statically registered component schemas and opaque, bounded bin_prot payloads.
use binprot::{BinProtWrite, macros::BinProtWrite};

pub const MAX_PROPERTIES: usize = 65_536;
pub const MAX_MESSAGE: usize = 16_384;
pub const MAX_COMPONENTS: usize = 64;

#[derive(Clone, Debug, PartialEq, Eq, BinProtWrite)]
pub struct Schema {
    pub name: String,
    pub version: i64,
    pub fingerprint: String,
}
impl Schema {
    pub fn is_valid(&self) -> bool {
        self.name.len() <= 128
            && self.name.split('.').count() >= 2
            && self.name.split('.').all(|segment| {
                segment
                    .as_bytes()
                    .first()
                    .is_some_and(u8::is_ascii_lowercase)
                    && segment
                        .bytes()
                        .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == b'_')
            })
            && (1..=65_535).contains(&self.version)
            && self.fingerprint.len() == 64
            && self
                .fingerprint
                .bytes()
                .all(|c| c.is_ascii_digit() || (b'a'..=b'f').contains(&c))
    }
}

/// OCaml string encoding: a byte length followed by raw data, not Vec<u8>'s
/// list-of-integers encoding. Payloads need not be UTF-8 and may contain NUL.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Payload(pub Vec<u8>);
impl BinProtWrite for Payload {
    fn binprot_write<W: std::io::Write>(&self, writer: &mut W) -> std::io::Result<()> {
        binprot::Nat0(self.0.len() as u64).binprot_write(writer)?;
        writer.write_all(&self.0)
    }
}

#[derive(Clone, Debug, PartialEq, Eq, BinProtWrite)]
pub struct Command {
    pub sequence: i64,
    pub payload: Payload,
}
impl Command {
    pub fn is_valid(&self) -> bool {
        self.sequence > 0 && self.payload.0.len() <= MAX_MESSAGE
    }
}

#[derive(Clone, Debug, PartialEq, Eq, BinProtWrite)]
pub struct Config {
    pub schema: Schema,
    pub generation: i64,
    pub label: String,
    pub disabled: bool,
    pub properties: Payload,
    pub command: Option<Command>,
}
impl Config {
    pub fn is_valid(&self) -> bool {
        self.schema.is_valid()
            && self.generation > 0
            && !self.label.trim_ascii().is_empty()
            && self.label.len() <= 1024
            && !self.label.contains('\0')
            && self.properties.0.len() <= MAX_PROPERTIES
            && self.command.as_ref().is_none_or(Command::is_valid)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, BinProtWrite)]
pub enum Error {
    InvalidSchema,
    IncompatibleSdk,
    DuplicateComponent,
    LimitExceeded,
    UnknownComponent,
    IncompatibleSchema,
    InvalidProperties,
    InvalidCommand,
    Closed,
    Hidden,
    Stale,
    Overloaded,
    Panicked,
    InvalidEvent,
}

#[derive(Clone, Debug, PartialEq, Eq, BinProtWrite)]
pub enum Signal {
    Data(Payload),
    Mounted,
    CommandCompleted(i64),
    Failed(Error),
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn opaque_event_fixture_matches_ocaml_strings() {
        let signal = Signal::Data(Payload(vec![0, 255, 128]));
        let mut bytes = Vec::new();
        signal.binprot_write(&mut bytes).unwrap();
        assert_eq!(bytes, [0x00, 0x03, 0x00, 0xff, 0x80]);
    }

    #[test]
    fn schema_and_instance_limits_are_enforced_independently_of_codecs() {
        let schema = Schema {
            name: "example.counter".into(),
            version: 1,
            fingerprint: "a".repeat(64),
        };
        let mut config = Config {
            schema,
            generation: 1,
            label: "Count".into(),
            disabled: false,
            properties: Payload(vec![0; MAX_PROPERTIES]),
            command: Some(Command {
                sequence: 1,
                payload: Payload(vec![255; MAX_MESSAGE]),
            }),
        };
        assert!(config.is_valid());
        config.properties.0.push(0);
        assert!(!config.is_valid());
        config.properties.0.pop();
        config.command.as_mut().unwrap().sequence = 0;
        assert!(!config.is_valid());
        config.command = None;
        config.schema.version = 65536;
        assert!(!config.is_valid());
    }
}
