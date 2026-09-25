//! Canonical OTP text policy. No native editing state or authentication semantics.
use binprot::macros::BinProtWrite;

pub const MAX_INPUT_BYTES: usize = 4096;
#[derive(Clone, Copy, Debug, PartialEq, Eq, BinProtWrite)]
pub enum Alphabet {
    Digits,
    AsciiAlphanumeric,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, BinProtWrite)]
pub struct Policy {
    length: i64,
    alphabet: Alphabet,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InputError {
    InvalidPolicy,
    InputTooLarge,
    InvalidUtf8,
    UnexpectedCharacter { byte_offset: usize },
    TooLong,
    InvalidValue,
    InvalidSelection,
}
impl InputError {
    pub fn is_valid(self) -> bool {
        match self {
            Self::UnexpectedCharacter { byte_offset } => byte_offset < MAX_INPUT_BYTES,
            _ => true,
        }
    }
}
impl binprot::BinProtWrite for InputError {
    fn binprot_write<W: std::io::Write>(&self, writer: &mut W) -> std::io::Result<()> {
        let tag: u8 = match self {
            Self::InvalidPolicy => 0,
            Self::InputTooLarge => 1,
            Self::InvalidUtf8 => 2,
            Self::UnexpectedCharacter { .. } => 3,
            Self::TooLong => 4,
            Self::InvalidValue => 5,
            Self::InvalidSelection => 6,
        };
        writer.write_all(&[tag])?;
        if let Self::UnexpectedCharacter { byte_offset } = self {
            let offset = i64::try_from(*byte_offset)
                .map_err(|_| std::io::Error::other("OTP offset overflow"))?;
            offset.binprot_write(writer)?;
        }
        Ok(())
    }
}
impl Policy {
    pub fn new(length: i64, alphabet: Alphabet) -> Option<Self> {
        (1..=32)
            .contains(&length)
            .then_some(Self { length, alphabet })
    }
    pub fn length(self) -> usize {
        self.length as usize
    }
    pub fn alphabet(self) -> Alphabet {
        self.alphabet
    }
    fn allowed(self, ch: char) -> bool {
        ch.is_ascii_digit()
            || self.alphabet == Alphabet::AsciiAlphanumeric && ch.is_ascii_alphabetic()
    }
    pub fn canonical(self, text: &str) -> bool {
        text.len() <= self.length() && text.chars().all(|ch| self.allowed(ch))
    }
    /// Full-width digits/Latin letters map to ASCII. Paste additionally removes
    /// six ASCII whitespace characters and ASCII hyphens; never truncate.
    pub fn normalize(self, text: &str, paste: bool) -> Result<String, InputError> {
        if text.len() > MAX_INPUT_BYTES {
            return Err(InputError::InputTooLarge);
        }
        let mut result = String::with_capacity(self.length());
        for (byte_offset, ch) in text.char_indices() {
            if paste && matches!(ch, '\t' | '\n' | '\u{000b}' | '\u{000c}' | '\r' | ' ' | '-') {
                continue;
            }
            let code = ch as u32;
            let ch = if matches!(code, 0xff10..=0xff19 | 0xff21..=0xff3a | 0xff41..=0xff5a) {
                char::from_u32(code - 0xfee0).expect("ASCII mapping")
            } else {
                ch
            };
            if !self.allowed(ch) {
                return Err(InputError::UnexpectedCharacter { byte_offset });
            }
            if result.len() == self.length() {
                return Err(InputError::TooLong);
            }
            result.push(ch);
        }
        Ok(result)
    }
    pub fn normalize_bytes(self, text: &[u8], paste: bool) -> Result<String, InputError> {
        if text.len() > MAX_INPUT_BYTES {
            return Err(InputError::InputTooLarge);
        }
        let text = std::str::from_utf8(text).map_err(|_| InputError::InvalidUtf8)?;
        self.normalize(text, paste)
    }
    pub fn replace(
        self,
        value: &str,
        anchor: usize,
        head: usize,
        text: &str,
        paste: bool,
    ) -> Result<(String, usize, usize), InputError> {
        if !self.canonical(value) {
            return Err(InputError::InvalidValue);
        }
        if anchor > value.len() || head > value.len() {
            return Err(InputError::InvalidSelection);
        }
        let inserted = self.normalize(text, paste)?;
        if paste && inserted.is_empty() {
            return Ok((value.to_owned(), anchor, head));
        }
        let (start, stop) = (anchor.min(head), anchor.max(head));
        if value.len() - (stop - start) + inserted.len() > self.length() {
            return Err(InputError::TooLong);
        }
        let result = format!("{}{}{}", &value[..start], inserted, &value[stop..]);
        Ok((result, start + inserted.len(), start + inserted.len()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn policy_encoding_matches_the_ocaml_fixture() {
        use binprot::BinProtWrite;
        for (policy, expected) in [
            (Policy::new(6, Alphabet::Digits).unwrap(), [6, 0]),
            (
                Policy::new(32, Alphabet::AsciiAlphanumeric).unwrap(),
                [32, 1],
            ),
        ] {
            let mut bytes = Vec::new();
            policy.binprot_write(&mut bytes).unwrap();
            assert_eq!(bytes, expected);
        }
    }
    #[test]
    fn policies_normalize_only_allowed_ascii_and_full_width_characters() {
        assert!(Policy::new(0, Alphabet::Digits).is_none());
        assert!(Policy::new(33, Alphabet::Digits).is_none());
        let digits = Policy::new(6, Alphabet::Digits).unwrap();
        assert_eq!(digits.normalize("１２3４", false), Ok("1234".into()));
        assert_eq!(digits.normalize("１２-3 \t4\r\n", true), Ok("1234".into()));
        assert_eq!(
            digits.normalize("１２-3", false),
            Err(InputError::UnexpectedCharacter { byte_offset: 6 })
        );
        for text in ["١", "１\u{00a0}2", "１－2", "a", "\0", "🙂"] {
            assert!(matches!(
                digits.normalize(text, true),
                Err(InputError::UnexpectedCharacter { .. })
            ));
        }
        let letters = Policy::new(32, Alphabet::AsciiAlphanumeric).unwrap();
        assert_eq!(letters.normalize("ａＡzＺ０9", false), Ok("aAzZ09".into()));
        for code in (0xff10..=0xff19)
            .chain(0xff21..=0xff3a)
            .chain(0xff41..=0xff5a)
        {
            let text = char::from_u32(code).unwrap().to_string();
            let expected = char::from_u32(code - 0xfee0).unwrap().to_string();
            assert_eq!(letters.normalize(&text, false), Ok(expected));
        }
        assert_eq!(
            digits.normalize_bytes(&[0xff], false),
            Err(InputError::InvalidUtf8)
        );
        assert_eq!(
            digits.normalize(&" ".repeat(MAX_INPUT_BYTES), true),
            Ok("".into())
        );
        assert_eq!(
            digits.normalize(&" ".repeat(MAX_INPUT_BYTES + 1), true),
            Err(InputError::InputTooLarge)
        );
        assert_eq!(digits.normalize("1234567", false), Err(InputError::TooLong));
    }
    #[test]
    fn replacements_validate_the_entire_edit_and_preserve_empty_paste_selection() {
        let policy = Policy::new(6, Alphabet::Digits).unwrap();
        assert_eq!(
            policy.replace("123456", 4, 2, "９-８", true),
            Ok(("129856".into(), 4, 4))
        );
        assert_eq!(
            policy.replace("123456", 4, 2, " - ", true),
            Ok(("123456".into(), 4, 2))
        );
        assert_eq!(
            policy.replace("123456", 4, 2, "", false),
            Ok(("1256".into(), 2, 2))
        );
        assert_eq!(
            policy.replace("123456", 3, 3, "7", false),
            Err(InputError::TooLong)
        );
        assert_eq!(
            policy.replace("123456", 4, 2, "7x", true),
            Err(InputError::UnexpectedCharacter { byte_offset: 1 })
        );
        assert_eq!(
            policy.replace("123456", 7, 0, "1", false),
            Err(InputError::InvalidSelection)
        );
        assert_eq!(
            policy.replace("A", 0, 1, "1", false),
            Err(InputError::InvalidValue)
        );
        for length in 1..=32 {
            let policy = Policy::new(length, Alphabet::Digits).unwrap();
            let value = "1".repeat(length as usize);
            assert!(policy.canonical(&value));
            assert!(!policy.canonical(&(value.clone() + "2")));
            for start in 0..value.len() {
                let (next, anchor, head) = policy
                    .replace(&value, start + 1, start, "９", false)
                    .unwrap();
                assert!(policy.canonical(&next));
                assert_eq!(&next[start..start + 1], "9");
                assert_eq!((anchor, head), (start + 1, start + 1));
            }
        }
    }
}
