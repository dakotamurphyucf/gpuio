//! Optional measured disclosure motion. Native visibility remains semantic.
use crate::animation::Spring;
use binprot::macros::BinProtWrite;

#[derive(Clone, Copy, Debug, PartialEq, BinProtWrite)]
pub struct Config {
    pub expanded: bool,
    pub retain: bool,
    pub spring: Spring,
}
impl Config {
    pub fn is_valid(self) -> bool {
        self.spring.is_valid()
    }
}
