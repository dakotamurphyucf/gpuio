//! Bounded ordinary-input format data and pure conversion, paired with Core.
//! Attaching this policy to native editors is a separate integration boundary.
use binprot::macros::BinProtWrite;
use unicode_properties::{GeneralCategory, UnicodeGeneralCategory};

pub const MAX_TEXT_BYTES: usize = 262_144;
pub const MAX_PATTERN_BYTES: usize = 1024;
pub const MAX_PATTERN_SCALARS: usize = 256;

#[derive(Clone, Debug, PartialEq, Eq, BinProtWrite)]
pub struct Number {
    pub separator: Option<String>,
    pub fraction_digits: Option<i64>,
}
#[derive(Clone, Debug, PartialEq, Eq, BinProtWrite)]
pub enum Config {
    Pattern(String),
    Number(Number),
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    InvalidConfig,
    InvalidText,
    DoesNotFit,
    LimitExceeded,
}
fn normalize(c: char) -> char {
    match c {
        '\u{ff10}'..='\u{ff19}' => char::from_u32(c as u32 - 0xff10 + u32::from('0')).unwrap(),
        '\u{ff0b}' => '+',
        '\u{ff0d}' | '\u{2212}' => '-',
        '\u{ff0e}' | '\u{3002}' => '.',
        '\u{ff0c}' => ',',
        _ => c,
    }
}
fn control(c: char) -> bool {
    (c as u32) < 32 || (127..=159).contains(&(c as u32))
}
fn valid_separator(text: &str) -> bool {
    if text.len() > 4 {
        return false;
    }
    let mut chars = text.chars();
    let Some(c) = chars.next() else {
        return false;
    };
    chars.next().is_none()
        && !control(c)
        && c.general_category() != GeneralCategory::DecimalNumber
        && !matches!(c, '+' | '-' | '.')
        && normalize(c) == c
}
fn valid_text(text: &str) -> Result<(), Error> {
    if text.len() > MAX_TEXT_BYTES {
        Err(Error::LimitExceeded)
    } else if text.contains(['\0', '\r', '\n']) {
        Err(Error::InvalidText)
    } else {
        Ok(())
    }
}
fn slot_accepts(token: char, value: char) -> Option<bool> {
    match token {
        '9' => Some(value.is_ascii_digit()),
        'A' => Some(value.is_ascii_alphabetic()),
        '#' => Some(value.is_ascii_alphanumeric()),
        '*' => Some(true),
        _ => None,
    }
}
fn format_pattern(pattern: &str, raw: &str) -> Result<String, Error> {
    let mut input = raw.chars().peekable();
    let mut output = String::new();
    for token in pattern.chars() {
        let Some(&c) = input.peek() else {
            return Ok(output);
        };
        match slot_accepts(token, c) {
            Some(false) => return Err(Error::DoesNotFit),
            Some(true) => {
                output.push(c);
                input.next();
            }
            None => output.push(token),
        }
    }
    if input.next().is_some() {
        Err(Error::DoesNotFit)
    } else {
        Ok(output)
    }
}
fn extract_pattern(pattern: &str, formatted: &str) -> Result<String, Error> {
    let mut input = formatted.chars();
    let mut output = String::new();
    for token in pattern.chars() {
        let Some(c) = input.next() else {
            return Ok(output);
        };
        match slot_accepts(token, c) {
            Some(false) => return Err(Error::DoesNotFit),
            Some(true) => output.push(c),
            None if token == c => (),
            None => return Err(Error::DoesNotFit),
        }
    }
    if input.next().is_some() {
        Err(Error::DoesNotFit)
    } else {
        Ok(output)
    }
}
fn format_number(config: &Number, raw: &str) -> Result<String, Error> {
    let text: String = raw.chars().map(normalize).collect();
    let (sign, unsigned) = if text
        .as_bytes()
        .first()
        .is_some_and(|c| matches!(c, b'+' | b'-'))
    {
        (&text[..1], &text[1..])
    } else {
        ("", text.as_str())
    };
    let (integer, fraction) = match unsigned.split_once('.') {
        Some((integer, fraction)) => (integer, Some(fraction)),
        None => (unsigned, None),
    };
    let digits = |text: &str| text.bytes().all(|c| c.is_ascii_digit());
    if !digits(integer) || fraction.is_some_and(|fraction| !digits(fraction)) {
        return Err(Error::DoesNotFit);
    }
    if fraction.is_some_and(|fraction| {
        config
            .fraction_digits
            .is_some_and(|limit| limit == 0 || fraction.len() > limit as usize)
    }) {
        return Err(Error::DoesNotFit);
    }
    let separator = config.separator.as_deref().unwrap_or("");
    let groups = integer.len().saturating_sub(1) / 3;
    let size = text.len() + groups * separator.len();
    if size > MAX_TEXT_BYTES {
        return Err(Error::LimitExceeded);
    }
    let mut output = String::with_capacity(size);
    output.push_str(sign);
    for (i, c) in integer.bytes().enumerate() {
        if i > 0 && (integer.len() - i).is_multiple_of(3) {
            output.push_str(separator);
        }
        output.push(char::from(c));
    }
    if let Some(fraction) = fraction {
        output.push('.');
        output.push_str(fraction);
    }
    Ok(output)
}
impl Config {
    pub fn is_valid(&self) -> bool {
        match self {
            Self::Pattern(source) => {
                !source.is_empty()
                    && source.len() <= MAX_PATTERN_BYTES
                    && source.chars().count() <= MAX_PATTERN_SCALARS
                    && !source.chars().any(control)
            }
            Self::Number(number) => {
                number.separator.as_deref().is_none_or(valid_separator)
                    && number
                        .fraction_digits
                        .is_none_or(|n| (0..=MAX_TEXT_BYTES as i64).contains(&n))
            }
        }
    }
    /// Normalize an interactive insertion without changing its UTF-16 length.
    /// Pattern slots preserve the original characters. This does not validate
    /// the complete candidate or turn an insertion into a raw-value command.
    pub fn normalize_insert<'a>(&self, text: &'a str) -> std::borrow::Cow<'a, str> {
        if matches!(self, Self::Number(_)) && text.chars().any(|c| normalize(c) != c) {
            std::borrow::Cow::Owned(text.chars().map(normalize).collect())
        } else {
            std::borrow::Cow::Borrowed(text)
        }
    }

    pub fn retained_bytes(&self) -> usize {
        std::mem::size_of::<Self>()
            + match self {
                Self::Pattern(source) => source.len(),
                Self::Number(number) => number.separator.as_ref().map_or(0, String::len),
            }
    }
    pub fn format_raw(&self, raw: &str) -> Result<String, Error> {
        if !self.is_valid() {
            return Err(Error::InvalidConfig);
        }
        valid_text(raw)?;
        match self {
            Self::Pattern(pattern) => format_pattern(pattern, raw),
            Self::Number(number) => format_number(number, raw),
        }
    }
    pub fn raw_of_formatted(&self, formatted: &str) -> Result<String, Error> {
        if !self.is_valid() {
            return Err(Error::InvalidConfig);
        }
        valid_text(formatted)?;
        match self {
            Self::Pattern(pattern) => extract_pattern(pattern, formatted),
            Self::Number(number) => {
                let raw = match &number.separator {
                    None => formatted.to_owned(),
                    Some(separator) => formatted.replace(separator, ""),
                };
                if format_number(number, &raw)? == formatted {
                    Ok(raw)
                } else {
                    Err(Error::DoesNotFit)
                }
            }
        }
    }
    pub fn accepts_formatted(&self, formatted: &str) -> bool {
        self.raw_of_formatted(formatted).is_ok()
    }
}
