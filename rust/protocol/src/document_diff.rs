//! Diff control values and installed-snapshot observations. Transport attachment
//! and mounted capability advertisement are separate acceptance steps.
use binprot::macros::BinProtWrite;
use std::collections::BTreeSet;

pub const MAX_KEYS: usize = 8192;
pub const MAX_PATH_BYTES: usize = 4096;
pub const MAX_CONFIG_BYTES: usize = 262144;
pub const MAX_EVENT_BYTES: usize = 32768;
pub const MAX_BODY_LINES: i64 = 8192;
pub const MAX_SOURCE_BYTES: i64 = 262144;

fn path_valid(path: &str) -> bool {
    !path.is_empty() && path.len() <= MAX_PATH_BYTES && !path.contains('\0')
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, BinProtWrite)]
pub enum FileKey {
    Path(String),
    Unnamed,
}
impl FileKey {
    pub fn is_valid(&self) -> bool {
        match self {
            Self::Path(path) => path_valid(path),
            Self::Unnamed => true,
        }
    }
    fn encoded_size(&self) -> usize {
        match self {
            Self::Path(path) => 1 + small_int_size(path.len() as i64) + path.len(),
            Self::Unnamed => 1,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, BinProtWrite)]
pub enum Collapse {
    Managed(Vec<FileKey>),
    Controlled(Vec<FileKey>),
}
impl Collapse {
    pub fn keys(&self) -> &[FileKey] {
        match self {
            Self::Managed(keys) | Self::Controlled(keys) => keys,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, BinProtWrite)]
pub enum LineLimit {
    Managed { initial: Option<i64>, step: i64 },
    Controlled(Option<i64>),
}
impl LineLimit {
    pub fn is_valid(&self) -> bool {
        let limit = |value: &Option<i64>| value.is_none_or(|n| (0..=MAX_BODY_LINES).contains(&n));
        match self {
            Self::Managed { initial, step } => {
                limit(initial) && (1..=MAX_BODY_LINES).contains(step)
            }
            Self::Controlled(value) => limit(value),
        }
    }
    fn encoded_size(&self) -> usize {
        let option = |value: &Option<i64>| 1 + value.map_or(0, small_int_size);
        1 + match self {
            Self::Managed { initial, step } => option(initial) + small_int_size(*step),
            Self::Controlled(value) => option(value),
        }
    }
}
// All validated counts here are 0..8192, path lengths 1..4096.
fn small_int_size(value: i64) -> usize {
    if value < 128 { 1 } else { 3 }
}

#[derive(Clone, Debug, PartialEq, Eq, BinProtWrite)]
pub struct Config {
    pub collapse: Collapse,
    pub line_limit: LineLimit,
    pub word_diff: bool,
}
impl Default for Config {
    fn default() -> Self {
        Self {
            collapse: Collapse::Managed(Vec::new()),
            line_limit: LineLimit::Managed {
                initial: None,
                step: 200,
            },
            word_diff: true,
        }
    }
}
impl Config {
    pub fn is_valid(&self) -> bool {
        let keys = self.collapse.keys();
        keys.len() <= MAX_KEYS
            && keys.iter().all(FileKey::is_valid)
            && keys.iter().collect::<BTreeSet<_>>().len() == keys.len()
            && self.line_limit.is_valid()
            && {
                let encoded_bytes = 1
                    + small_int_size(keys.len() as i64)
                    + keys.iter().map(FileKey::encoded_size).sum::<usize>()
                    + self.line_limit.encoded_size()
                    + 1;
                encoded_bytes <= MAX_CONFIG_BYTES
            }
    }
    pub fn retained_bytes(&self) -> usize {
        std::mem::size_of::<Self>()
            + std::mem::size_of_val(self.collapse.keys())
            + self
                .collapse
                .keys()
                .iter()
                .map(|key| match key {
                    FileKey::Path(path) => path.len(),
                    FileKey::Unnamed => 0,
                })
                .sum::<usize>()
    }
}

#[derive(Clone, Debug, PartialEq, Eq, BinProtWrite)]
pub struct File {
    pub index: i64,
    pub key: FileKey,
    pub before_path: Option<String>,
    pub after_path: Option<String>,
}
impl File {
    pub fn is_valid(&self) -> bool {
        (0..MAX_BODY_LINES).contains(&self.index)
            && self.before_path.as_deref().is_none_or(path_valid)
            && self.after_path.as_deref().is_none_or(path_valid)
            && match (
                &self.key,
                self.after_path.as_ref().or(self.before_path.as_ref()),
            ) {
                (FileKey::Path(key), Some(path)) => key == path,
                (FileKey::Unnamed, None) => true,
                _ => false,
            }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, BinProtWrite)]
pub struct Line {
    pub file: File,
    pub before: Option<i64>,
    pub after: Option<i64>,
    pub start_byte: i64,
    pub end_byte: i64,
    pub text: String,
}
impl Line {
    pub fn is_valid(&self) -> bool {
        let coordinate = |n| (1..=i32::MAX as i64).contains(&n);
        self.file.is_valid()
            && self.before.is_none_or(coordinate)
            && self.after.is_none_or(coordinate)
            && self.start_byte >= 0
            && self.end_byte >= self.start_byte
            && self.end_byte <= MAX_SOURCE_BYTES
            && self.end_byte - self.start_byte == self.text.len() as i64
            && self.text.len() <= 16384
            && !self.text.contains('\n')
    }
}

#[derive(Clone, Debug, PartialEq, Eq, BinProtWrite)]
pub enum Observation {
    ToggleFile {
        file: File,
        collapsed: bool,
        applied: bool,
    },
    ShowMore {
        visible: i64,
        hidden: i64,
        applied_limit: Option<i64>,
    },
    Line(Line),
}
impl Observation {
    pub fn is_valid(&self) -> bool {
        match self {
            Self::ToggleFile { file, .. } => file.is_valid(),
            Self::ShowMore {
                visible,
                hidden,
                applied_limit,
            } => {
                (0..MAX_BODY_LINES).contains(visible)
                    && (1..=MAX_BODY_LINES).contains(hidden)
                    && visible + hidden <= MAX_BODY_LINES
                    && applied_limit.is_none_or(|n| n > *visible && n <= MAX_BODY_LINES)
            }
            Self::Line(line) => line.is_valid(),
        }
    }
    pub fn valid_for(&self, config: &Config) -> bool {
        self.is_valid()
            && config.is_valid()
            && match self {
                Self::ToggleFile {
                    file,
                    collapsed,
                    applied,
                } => match &config.collapse {
                    Collapse::Managed(_) => *applied,
                    Collapse::Controlled(keys) => {
                        !applied && *collapsed != keys.contains(&file.key)
                    }
                },
                Self::ShowMore {
                    visible,
                    applied_limit,
                    ..
                } => match config.line_limit {
                    LineLimit::Managed { step, .. } => {
                        *applied_limit == Some((visible + step).min(MAX_BODY_LINES))
                    }
                    LineLimit::Controlled(limit) => {
                        limit == Some(*visible) && applied_limit.is_none()
                    }
                },
                Self::Line(_) => true,
            }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, BinProtWrite)]
pub struct Event {
    pub config_epoch: i64,
    pub source_revision: i64,
    pub source_generation: i64,
    pub observation: Observation,
}
impl Event {
    /// Variable payload bytes, excluding the envelope's separately charged
    /// fixed fields. Every repeated path label is charged independently.
    pub fn payload_bytes(&self) -> usize {
        let file = |file: &File| {
            (match &file.key {
                FileKey::Path(path) => path.len(),
                FileKey::Unnamed => 0,
            }) + file.before_path.as_ref().map_or(0, String::len)
                + file.after_path.as_ref().map_or(0, String::len)
        };
        match &self.observation {
            Observation::ToggleFile { file: value, .. } => file(value),
            Observation::ShowMore { .. } => 0,
            Observation::Line(line) => file(&line.file) + line.text.len(),
        }
    }
    pub fn is_valid(&self) -> bool {
        self.config_epoch > 0
            && self.source_revision > 0
            && self.source_generation > 0
            && self.observation.is_valid()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{decode_document_diff_config, decode_document_diff_event};
    use binprot::BinProtWrite;

    fn encode(value: &impl BinProtWrite) -> Vec<u8> {
        let mut bytes = Vec::new();
        value.binprot_write(&mut bytes).unwrap();
        bytes
    }
    fn hex(text: &str) -> Vec<u8> {
        (0..text.len())
            .step_by(2)
            .map(|i| u8::from_str_radix(&text[i..i + 2], 16).unwrap())
            .collect()
    }
    fn file() -> File {
        File {
            index: 1,
            key: FileKey::Path("λ".into()),
            before_path: Some("old".into()),
            after_path: Some("λ".into()),
        }
    }
    fn line() -> Line {
        Line {
            file: file(),
            before: Some(10),
            after: Some(20),
            start_byte: 50,
            end_byte: 52,
            text: "λ".into(),
        }
    }
    fn event(observation: Observation) -> Event {
        Event {
            config_epoch: 2,
            source_revision: 7,
            source_generation: 3,
            observation,
        }
    }
    #[test]
    fn independent_ocaml_config_fixtures() {
        let fixtures = [
            (Config::default(), "00000000fec80001"),
            (
                Config {
                    collapse: Collapse::Controlled(vec![
                        FileKey::Path("src/λ.ml".into()),
                        FileKey::Unnamed,
                    ]),
                    line_limit: LineLimit::Controlled(Some(0)),
                    word_diff: false,
                },
                "010200097372632fcebb2e6d6c0101010000",
            ),
            (
                Config {
                    collapse: Collapse::Managed(vec![FileKey::Path("a".into())]),
                    line_limit: LineLimit::Managed {
                        initial: Some(130),
                        step: 129,
                    },
                    word_diff: true,
                },
                "00010001610001fe8200fe810001",
            ),
        ];
        for (config, fixture) in fixtures {
            let bytes = hex(fixture);
            assert_eq!(encode(&config), bytes);
            assert_eq!(decode_document_diff_config(&bytes).unwrap(), config);
            for end in 0..bytes.len() {
                assert!(decode_document_diff_config(&bytes[..end]).is_err());
            }
            let mut trailing = bytes;
            trailing.push(0);
            assert!(decode_document_diff_config(&trailing).is_err());
        }
    }
    #[test]
    fn independent_ocaml_revisioned_event_fixtures() {
        let fixtures = [
            (
                Observation::ToggleFile {
                    file: file(),
                    collapsed: true,
                    applied: false,
                },
                "02070300010002cebb01036f6c640102cebb0100",
            ),
            (
                Observation::ShowMore {
                    visible: 0,
                    hidden: 7,
                    applied_limit: None,
                },
                "02070301000700",
            ),
            (
                Observation::ShowMore {
                    visible: 130,
                    hidden: 9,
                    applied_limit: Some(330),
                },
                "02070301fe82000901fe4a01",
            ),
            (
                Observation::Line(line()),
                "02070302010002cebb01036f6c640102cebb010a0114323402cebb",
            ),
            (
                Observation::Line(Line {
                    file: File {
                        index: 0,
                        key: FileKey::Unnamed,
                        before_path: None,
                        after_path: None,
                    },
                    before: None,
                    after: None,
                    start_byte: 1,
                    end_byte: 1,
                    text: String::new(),
                }),
                "02070302000100000000010100",
            ),
        ];
        for (observation, fixture) in fixtures {
            let event = event(observation);
            let bytes = hex(fixture);
            assert_eq!(encode(&event), bytes);
            assert_eq!(decode_document_diff_event(&bytes).unwrap(), event);
            for end in 0..bytes.len() {
                assert!(decode_document_diff_event(&bytes[..end]).is_err());
            }
            let mut trailing = bytes;
            trailing.push(0);
            assert!(decode_document_diff_event(&trailing).is_err());
        }
    }
    #[test]
    fn key_count_path_bounds_and_exact_encoded_limit() {
        let config = |keys| Config {
            collapse: Collapse::Controlled(keys),
            line_limit: LineLimit::Controlled(None),
            word_diff: true,
        };
        for keys in [
            vec![FileKey::Path(String::new())],
            vec![FileKey::Path("bad\0path".into())],
            vec![FileKey::Path("x".repeat(4097))],
            vec![FileKey::Unnamed, FileKey::Unnamed],
            (0..8193).map(|i| FileKey::Path(i.to_string())).collect(),
        ] {
            let value = config(keys);
            assert!(!value.is_valid());
            assert!(decode_document_diff_config(&encode(&value)).is_err());
        }
        assert!(config((0..8192).map(|i| FileKey::Path(i.to_string())).collect()).is_valid());
        let mut keys: Vec<_> = (0..63)
            .map(|i| FileKey::Path(format!("{i:04}{}", "x".repeat(4092))))
            .collect();
        keys.push(FileKey::Path("y".repeat(3835)));
        let exact = config(keys.clone());
        assert!(exact.is_valid());
        assert_eq!(encode(&exact).len(), MAX_CONFIG_BYTES);
        assert_eq!(decode_document_diff_config(&encode(&exact)).unwrap(), exact);
        keys[63] = FileKey::Path("y".repeat(3836));
        assert!(!config(keys.clone()).is_valid());
        assert!(decode_document_diff_config(&encode(&config(keys))).is_err());
        assert!(
            decode_document_diff_config(&[0, 0xfc, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0x7f])
                .is_err()
        );
        // Invalid UTF-8 is rejected before a String can enter the domain.
        assert!(decode_document_diff_config(&[0, 1, 0, 1, 0xff, 1, 0, 1]).is_err());
        for limit in [-1, 8193] {
            let value = Config {
                line_limit: LineLimit::Controlled(Some(limit)),
                ..Config::default()
            };
            assert!(!value.is_valid());
            assert!(decode_document_diff_config(&encode(&value)).is_err());
        }
        for step in [0, 8193] {
            let value = Config {
                line_limit: LineLimit::Managed {
                    initial: None,
                    step,
                },
                ..Config::default()
            };
            assert!(!value.is_valid());
        }
    }
    #[test]
    fn event_boundaries_provenance_and_ownership_are_validated() {
        let managed = Config::default();
        let controlled = Config {
            collapse: Collapse::Controlled(vec![]),
            line_limit: LineLimit::Controlled(Some(0)),
            word_diff: true,
        };
        let toggle = |applied, collapsed| Observation::ToggleFile {
            file: file(),
            applied,
            collapsed,
        };
        assert!(toggle(false, true).valid_for(&controlled));
        assert!(!toggle(true, true).valid_for(&controlled));
        assert!(!toggle(false, false).valid_for(&controlled));
        assert!(toggle(true, false).valid_for(&managed));
        assert!(!toggle(false, false).valid_for(&managed));
        let show = |visible, applied_limit| Observation::ShowMore {
            visible,
            hidden: 7,
            applied_limit,
        };
        assert!(show(0, None).valid_for(&controlled));
        assert!(!show(1, None).valid_for(&controlled));
        assert!(show(20, Some(220)).valid_for(&managed));
        assert!(!show(20, Some(21)).valid_for(&managed));
        for value in [
            Line {
                before: Some(0),
                ..line()
            },
            Line {
                after: Some(i32::MAX as i64 + 1),
                ..line()
            },
            Line {
                text: "a".into(),
                ..line()
            },
            Line {
                text: "a\n".into(),
                ..line()
            },
            Line {
                start_byte: -1,
                ..line()
            },
            Line {
                end_byte: MAX_SOURCE_BYTES + 1,
                ..line()
            },
        ] {
            assert!(decode_document_diff_event(&encode(&event(Observation::Line(value)))).is_err());
        }
        let valid = event(Observation::Line(line()));
        for n in [0, -1] {
            for value in [
                Event {
                    config_epoch: n,
                    ..valid.clone()
                },
                Event {
                    source_revision: n,
                    ..valid.clone()
                },
                Event {
                    source_generation: n,
                    ..valid.clone()
                },
            ] {
                assert!(decode_document_diff_event(&encode(&value)).is_err());
            }
        }
        let bad_file = File {
            key: FileKey::Unnamed,
            ..file()
        };
        assert!(!bad_file.is_valid());
        for observation in [
            Observation::ShowMore {
                visible: 0,
                hidden: 0,
                applied_limit: None,
            },
            Observation::ShowMore {
                visible: 8191,
                hidden: 2,
                applied_limit: None,
            },
            Observation::ShowMore {
                visible: 10,
                hidden: 2,
                applied_limit: Some(10),
            },
        ] {
            assert!(decode_document_diff_event(&encode(&event(observation))).is_err());
        }
        // Payload NUL and a standalone CR remain source text, not path labels.
        let payload = Line {
            text: "\0\r".into(),
            ..line()
        };
        assert!(payload.is_valid());
        assert_eq!(
            decode_document_diff_event(&encode(&event(Observation::Line(payload.clone()))))
                .unwrap()
                .observation,
            Observation::Line(payload)
        );
    }
}
