//! Shared native plotting algorithms. Chart adapters provide validated bounded
//! data and retain the resulting geometry; no OCaml callbacks enter layout/paint.
pub mod sankey;

use gpui::{Point, point};
use std::{fmt::Debug, ops::Add};

fn origin_point<T>(x: T, y: T, origin: Point<T>) -> Point<T>
where
    T: Default + Clone + Debug + PartialEq + Add<Output = T>,
{
    point(x, y) + origin
}
