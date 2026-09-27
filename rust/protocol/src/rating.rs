use binprot::macros::BinProtWrite;
pub const MAX_STARS: i64 = 32;

#[derive(Clone, Copy, Debug, PartialEq, Eq, BinProtWrite)]
pub enum Request {
    Set(i64),
    Toggle(i64),
    Increase,
    Decrease,
}
impl Request {
    pub fn is_valid(self) -> bool {
        match self {
            Self::Set(value) => (0..=MAX_STARS).contains(&value),
            Self::Toggle(value) => (1..=MAX_STARS).contains(&value),
            Self::Increase | Self::Decrease => true,
        }
    }
}

#[derive(Clone, Debug, PartialEq, BinProtWrite)]
pub struct Config {
    pub label: String,
    pub value: i64,
    pub maximum: i64,
    pub star_size: f64,
    pub disabled: bool,
    pub read_only: bool,
}
impl Config {
    pub fn is_valid(&self) -> bool {
        (1..=MAX_STARS).contains(&self.maximum)
            && (0..=self.maximum).contains(&self.value)
            && self.star_size.is_finite()
            && (8. ..=128.).contains(&self.star_size)
            && crate::v1::CommandConfig::valid_text(&self.label, 4096)
    }
    pub fn can_apply(&self, request: Request) -> bool {
        !self.disabled
            && !self.read_only
            && request.is_valid()
            && match request {
                Request::Set(value) | Request::Toggle(value) => value <= self.maximum,
                Request::Increase | Request::Decrease => true,
            }
    }
    pub fn retained_bytes(&self) -> usize {
        std::mem::size_of::<Self>() + self.label.capacity()
    }
}
