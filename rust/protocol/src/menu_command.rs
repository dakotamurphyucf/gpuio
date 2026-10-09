//! Bounded logical-window positioning against an exact menu subscription.
use binprot::macros::BinProtWrite;

#[derive(Clone, Debug, PartialEq, BinProtWrite)]
pub struct Position {
    pub x: f64,
    pub y: f64,
}
impl Position {
    pub fn is_valid(&self) -> bool {
        [self.x, self.y]
            .into_iter()
            .all(|v| v.is_finite() && v.abs() <= 1_000_000.)
    }
}
#[derive(Clone, Debug, PartialEq, BinProtWrite)]
pub enum Command {
    Show(Position),
    Close,
}
#[derive(Clone, Debug, PartialEq, Eq, BinProtWrite)]
pub enum Error {
    NotMounted,
    Closed,
    StaleMenu,
    Unavailable,
    InvalidPosition,
    Busy,
    NativeFailure,
}
#[derive(Clone, Debug, PartialEq, Eq, BinProtWrite)]
pub enum Response {
    Applied,
    Failed(Error),
}
