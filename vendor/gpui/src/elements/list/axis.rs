//! The measured-list algorithm uses x/width for the cross axis and y/height for
//! the main axis. Only boundaries map these coordinates; child elements keep
//! their physical layout, paint, text, input and accessibility orientation.
use crate::{AnyElement, App, AvailableSpace, Axis, Bounds, Edges, Pixels, Point, Size, Window};
use std::fmt::Debug;

#[derive(Clone, Copy)]
pub(super) struct Mapping(pub Axis);
impl Mapping {
    pub fn size<T: Clone + Debug + Default + PartialEq>(self, value: Size<T>) -> Size<T> {
        match self.0 {
            Axis::Vertical => value,
            Axis::Horizontal => Size {
                width: value.height,
                height: value.width,
            },
        }
    }
    pub fn point(self, value: Point<Pixels>) -> Point<Pixels> {
        match self.0 {
            Axis::Vertical => value,
            Axis::Horizontal => Point {
                x: value.y,
                y: value.x,
            },
        }
    }
    pub fn bounds(self, value: Bounds<Pixels>) -> Bounds<Pixels> {
        Bounds {
            origin: self.point(value.origin),
            size: self.size(value.size),
        }
    }
    pub fn edges(self, value: Edges<Pixels>) -> Edges<Pixels> {
        match self.0 {
            Axis::Vertical => value,
            Axis::Horizontal => Edges {
                top: value.left,
                right: value.bottom,
                bottom: value.right,
                left: value.top,
            },
        }
    }
    pub fn layout(
        self,
        element: &mut AnyElement,
        logical_space: Size<AvailableSpace>,
        window: &mut Window,
        cx: &mut App,
    ) -> Size<Pixels> {
        self.size(element.layout_as_root(self.size(logical_space), window, cx))
    }
}
