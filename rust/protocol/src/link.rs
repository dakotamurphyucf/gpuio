//! Bounded configuration foundation for composed native links.
//! Admitted by SetLink; capability advertisement awaits remaining native validation.
use binprot::macros::BinProtWrite;

pub const MAX_LABEL_BYTES: usize = 4096;
pub const MAX_TAB_INDEX: i64 = 1_000_000;
// Worst-case length prefix, UTF-8 bytes, two booleans and signed integer.
pub const MAX_CONFIG_BYTES: usize = 9 + MAX_LABEL_BYTES + 2 + 9;

#[derive(Clone, Debug, PartialEq, Eq, BinProtWrite)]
pub struct Config {
    pub label: String,
    pub disabled: bool,
    pub tab_stop: bool,
    pub tab_index: i64,
}

impl Config {
    pub fn is_valid(&self) -> bool {
        self.label.len() <= MAX_LABEL_BYTES
            && !self.label.contains('\0')
            // Match Core.Char.is_whitespace, including vertical tab.
            && self.label.bytes().any(|byte| !matches!(byte, b' ' | b'\t'..=b'\r'))
            && (-MAX_TAB_INDEX..=MAX_TAB_INDEX).contains(&self.tab_index)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{DecodeError, decode_link_config};
    use binprot::BinProtWrite;

    fn fixture() -> Config {
        Config {
            label: "Guide 世界".into(),
            disabled: false,
            tab_stop: false,
            tab_index: -2,
        }
    }

    fn encode(config: &Config) -> Vec<u8> {
        let mut bytes = vec![];
        config.binprot_write(&mut bytes).unwrap();
        bytes
    }

    #[test]
    fn link_fixture_and_framing_match_independent_ocaml_bytes() {
        let config = fixture();
        let bytes = encode(&config);
        assert_eq!(
            bytes
                .iter()
                .map(|byte| format!("{byte:02x}"))
                .collect::<String>(),
            "0c477569646520e4b896e7958c0000fffe"
        );
        assert_eq!(decode_link_config(&bytes), Ok(config));
        for length in 0..bytes.len() {
            assert!(decode_link_config(&bytes[..length]).is_err());
        }
        let mut invalid = bytes.clone();
        invalid.push(0);
        assert_eq!(decode_link_config(&invalid), Err(DecodeError::Malformed));
        for offset in [13, 14] {
            let mut invalid = bytes.clone();
            invalid[offset] = 2;
            assert_eq!(decode_link_config(&invalid), Err(DecodeError::Malformed));
        }
        let mut invalid = bytes;
        invalid[1] = 0xff;
        assert_eq!(decode_link_config(&invalid), Err(DecodeError::Malformed));
    }

    #[test]
    fn link_validation_covers_labels_indices_and_all_focus_flags() {
        for disabled in [false, true] {
            for tab_stop in [false, true] {
                for tab_index in [-MAX_TAB_INDEX, -1, 0, MAX_TAB_INDEX] {
                    let config = Config {
                        disabled,
                        tab_stop,
                        tab_index,
                        ..fixture()
                    };
                    assert!(config.is_valid());
                    assert_eq!(decode_link_config(&encode(&config)), Ok(config));
                }
            }
        }
        for label in ["", " \t\n\r\x0b\x0c", "a\0b"] {
            let config = Config {
                label: label.into(),
                ..fixture()
            };
            assert!(!config.is_valid());
            assert_eq!(
                decode_link_config(&encode(&config)),
                Err(DecodeError::Malformed)
            );
        }
        for tab_index in [i64::MIN, -MAX_TAB_INDEX - 1, MAX_TAB_INDEX + 1, i64::MAX] {
            let config = Config {
                tab_index,
                ..fixture()
            };
            assert!(!config.is_valid());
            assert_eq!(
                decode_link_config(&encode(&config)),
                Err(DecodeError::Malformed)
            );
        }
        let config = Config {
            label: "x".repeat(MAX_LABEL_BYTES),
            ..fixture()
        };
        assert!(config.is_valid());
        assert_eq!(decode_link_config(&encode(&config)), Ok(config));
        let config = Config {
            label: "x".repeat(MAX_LABEL_BYTES + 1),
            ..fixture()
        };
        assert!(!config.is_valid());
        assert_eq!(
            decode_link_config(&encode(&config)),
            Err(DecodeError::LimitExceeded)
        );
        let mut declared = vec![];
        binprot::Nat0(MAX_LABEL_BYTES as u64 + 1)
            .binprot_write(&mut declared)
            .unwrap();
        assert_eq!(
            decode_link_config(&declared),
            Err(DecodeError::LimitExceeded)
        );
        assert_eq!(
            decode_link_config(&vec![0; MAX_CONFIG_BYTES + 1]),
            Err(DecodeError::LimitExceeded)
        );
    }
}
