//! Concrete straight-alpha encoded-sRGB values. Theme references are not colors
//! selected by a picker. All conversion arithmetic is binary64, independent of GPUI.

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Hsla {
    hue_degrees: f64,
    saturation: f64,
    lightness: f64,
    alpha: f64,
}

fn zero(value: f64) -> f64 {
    if value == 0. { 0. } else { value }
}

impl Hsla {
    /// Hue is finite 0..=360 (360 becomes 0); all other channels are finite 0..=1.
    pub fn new(hue_degrees: f64, saturation: f64, lightness: f64, alpha: f64) -> Option<Self> {
        (hue_degrees.is_finite()
            && (0. ..=360.).contains(&hue_degrees)
            && [saturation, lightness, alpha]
                .iter()
                .all(|v| v.is_finite() && (0. ..=1.).contains(v)))
        .then_some(Self {
            hue_degrees: if hue_degrees == 360. {
                0.
            } else {
                zero(hue_degrees)
            },
            saturation: zero(saturation),
            lightness: zero(lightness),
            alpha: zero(alpha),
        })
    }
    pub fn hue_degrees(self) -> f64 {
        self.hue_degrees
    }
    pub fn saturation(self) -> f64 {
        self.saturation
    }
    pub fn lightness(self) -> f64 {
        self.lightness
    }
    pub fn alpha(self) -> f64 {
        self.alpha
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Rgba {
    pub red: u8,
    pub green: u8,
    pub blue: u8,
    pub alpha: u8,
}

fn byte(value: f64) -> u8 {
    // Only internal conversion roundoff is clamped. Public HSLA is validated.
    (value.clamp(0., 1.) * 255. + 0.5).floor() as u8
}

impl Rgba {
    pub const fn new(red: u8, green: u8, blue: u8, alpha: u8) -> Self {
        Self {
            red,
            green,
            blue,
            alpha,
        }
    }
    pub fn of_hsla(value: Hsla) -> Self {
        let hue = value.hue_degrees / 60.;
        let chroma = (1. - (2. * value.lightness - 1.).abs()) * value.saturation;
        let x = chroma * (1. - (hue % 2. - 1.).abs());
        let (red, green, blue) = match hue as u8 {
            0 => (chroma, x, 0.),
            1 => (x, chroma, 0.),
            2 => (0., chroma, x),
            3 => (0., x, chroma),
            4 => (x, 0., chroma),
            _ => (chroma, 0., x),
        };
        let offset = value.lightness - chroma / 2.;
        Self::new(
            byte(red + offset),
            byte(green + offset),
            byte(blue + offset),
            byte(value.alpha),
        )
    }
    pub fn to_hsla(self) -> Hsla {
        let red = f64::from(self.red) / 255.;
        let green = f64::from(self.green) / 255.;
        let blue = f64::from(self.blue) / 255.;
        let maximum = red.max(green.max(blue));
        let minimum = red.min(green.min(blue));
        let chroma = maximum - minimum;
        let lightness = (maximum + minimum) / 2.;
        let (hue_degrees, saturation) = if chroma == 0. {
            (0., 0.)
        } else {
            let sector = if maximum == red {
                (green - blue) / chroma
            } else if maximum == green {
                (blue - red) / chroma + 2.
            } else {
                (red - green) / chroma + 4.
            };
            let sector = if sector < 0. { sector + 6. } else { sector };
            (
                sector * 60.,
                (chroma / (1. - (2. * lightness - 1.).abs())).min(1.),
            )
        };
        Hsla::new(
            hue_degrees,
            saturation,
            lightness,
            f64::from(self.alpha) / 255.,
        )
        .expect("RGBA conversion produces validated HSLA")
    }
    /// Strict ASCII 3/4/6/8 hex digits, optional leading '#', no whitespace.
    pub fn of_hex(text: &str) -> Option<Self> {
        if text.len() > 9 {
            return None;
        }
        let digits = text.strip_prefix('#').unwrap_or(text).as_bytes();
        let length = digits.len();
        if !matches!(length, 3 | 4 | 6 | 8) || !digits.iter().all(u8::is_ascii_hexdigit) {
            return None;
        }
        let at = |index| hex_digit(digits[index]);
        let channel = |index| {
            if length <= 4 {
                at(index) * 17
            } else {
                at(index * 2) * 16 + at(index * 2 + 1)
            }
        };
        Some(Self::new(
            channel(0),
            channel(1),
            channel(2),
            if matches!(length, 4 | 8) {
                channel(3)
            } else {
                255
            },
        ))
    }
    pub fn to_hex(self) -> String {
        let rgb = format!("#{:02X}{:02X}{:02X}", self.red, self.green, self.blue);
        if self.alpha == 255 {
            rgb
        } else {
            format!("{rgb}{:02X}", self.alpha)
        }
    }
}

// Called only after ASCII validation.
fn hex_digit(value: u8) -> u8 {
    match value {
        b'0'..=b'9' => value - b'0',
        b'a'..=b'f' => value - b'a' + 10,
        b'A'..=b'F' => value - b'A' + 10,
        _ => unreachable!("validated hex digit"),
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Value {
    Empty,
    Color(Rgba),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AlphaPolicy {
    AllowAlpha,
    OpaqueOnly,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DraftError {
    Syntax,
    TooLong,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HexDraft {
    Empty,
    Incomplete,
    Invalid(DraftError),
    Valid(Rgba),
}

impl HexDraft {
    /// Classification preserves the caller's text. No allocation for invalid input.
    pub fn parse(text: &str) -> Self {
        if text.is_empty() {
            return Self::Empty;
        }
        if text.len() > 9 {
            return Self::Invalid(DraftError::TooLong);
        }
        let digits = text.strip_prefix('#').unwrap_or(text).as_bytes();
        if !digits.iter().all(u8::is_ascii_hexdigit) {
            return Self::Invalid(DraftError::Syntax);
        }
        match digits.len() {
            0 | 1 | 2 | 5 | 7 => Self::Incomplete,
            3 | 4 | 6 | 8 => Self::Valid(Rgba::of_hex(text).expect("validated hex color")),
            _ => Self::Invalid(DraftError::TooLong),
        }
    }
}
