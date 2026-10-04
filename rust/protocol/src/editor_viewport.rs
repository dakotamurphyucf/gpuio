use binprot::macros::BinProtWrite;

pub fn valid_pixels(n: f64) -> bool {
    n.is_finite() && (0.0..=1e9).contains(&n)
}

#[derive(Clone, Copy, Debug, PartialEq, BinProtWrite)]
pub struct Offset {
    pub x: f64,
    pub y: f64,
}
impl Offset {
    pub fn is_valid(&self) -> bool {
        valid_pixels(self.x) && valid_pixels(self.y)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, BinProtWrite)]
pub struct Snapshot {
    pub offset: Offset,
    pub width: f64,
    pub height: f64,
    pub line_height: f64,
    pub first_buffer_line: i64,
    pub buffer_line_limit: i64,
}
impl Snapshot {
    pub fn is_valid(&self) -> bool {
        self.offset.is_valid()
            && valid_pixels(self.width)
            && valid_pixels(self.height)
            && valid_pixels(self.line_height)
            && self.line_height > 0.
            && (0..=262145).contains(&self.first_buffer_line)
            && (self.first_buffer_line..=262145).contains(&self.buffer_line_limit)
    }
}
