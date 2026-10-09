//! Current-frame measured notification layout. The Host supplies retained child
//! owners and input eligibility; this element never calls the OCaml bridge.
use crate::toast_geometry::{self as geometry, Card, Layering};
use crate::toast_reflow as reflow;
use gpui::{
    AnyElement, App, AvailableSpace, Bounds, ContentMask, Element, ElementId, GlobalElementId,
    Hitbox, HitboxBehavior, InspectorElementId, IntoElement, LayoutId, Pixels, Point, Window, div,
    point, prelude::*, px, size,
};
use gpuio_protocol::{NodeId, toast_placement::Anchor};
use std::{
    cell::RefCell,
    collections::BTreeSet,
    rc::{Rc, Weak},
    time::Duration,
};

pub type Shared = Rc<RefCell<State>>;
/// Native-only observation. The Host applies focus/IME/timer eligibility from
/// these current painted bounds; no transported event is implied by a frame.
pub type Observer = Rc<dyn Fn(&Frame, &mut Window, &mut App)>;
/// Runs after child paint. Return true only when an accepted lifecycle sample
/// needs another frame. It must not synchronously deliver application events.
pub type PaintObserver = Rc<dyn Fn(&Frame, &mut Window, &mut App) -> bool>;
pub type HoverObserver = Rc<dyn Fn(bool, &mut Window, &mut App)>;

#[derive(Clone, Debug)]
pub struct Item {
    pub id: NodeId,
    /// Current transformed bounds. [painted] also covers an eligible first
    /// entry sample with zero opacity or a slide beyond the clip rectangle.
    pub bounds: Bounds<Pixels>,
    pub interactive: bool,
    pub painted: bool,
}
#[derive(Clone, Debug)]
pub struct Frame {
    pub viewport: Bounds<Pixels>,
    pub items: Vec<Item>,
    pub scroll: Pixels,
    pub max_scroll: Pixels,
    /// Reflow demand for eligible cards. Lifecycle demand is accepted separately
    /// by [Render::after_paint] after the children have actually painted.
    pub needs_frame: bool,
}
/// Contains no child elements, timers or external callbacks. Frame listeners
/// borrow this owner weakly; dropping it makes old-frame input inert.
#[derive(Default)]
pub struct State {
    scroll: Pixels,
    max_scroll: Pixels,
    expanded: bool,
    bottom: bool,
    hovered: bool,
    epoch: u64,
    reflow: reflow::State,
}
impl State {
    pub fn new() -> Shared {
        Rc::new(RefCell::new(Self::default()))
    }
    pub fn scroll(&self) -> Pixels {
        self.scroll
    }
    pub fn max_scroll(&self) -> Pixels {
        self.max_scroll
    }
    pub fn retain(&mut self, ids: &[NodeId]) -> Result<(), reflow::Error> {
        self.reflow.retain(ids)
    }
    pub fn settle_motion(&mut self) {
        self.reflow.suspend();
    }
    pub fn retained_items(&self) -> usize {
        self.reflow.retained_items()
    }
    pub fn scroll_by(&mut self, delta: Pixels) -> bool {
        if !self.expanded || !f32::from(delta).is_finite() {
            return false;
        }
        let next = (self.scroll + delta).clamp(px(0.), self.max_scroll);
        let changed = next != self.scroll;
        self.scroll = next;
        if changed {
            self.reflow.suspend();
        }
        changed
    }
    /// The parent calls this when not rendered. Old-frame listeners become
    /// invalid without retaining the parent, content, or a recurring task.
    pub fn suspend(&mut self) {
        self.epoch = self.epoch.wrapping_add(1);
        self.hovered = false;
        self.expanded = false;
        self.scroll = px(0.);
        self.max_scroll = px(0.);
        self.reflow = reflow::State::default();
    }
    fn measure(&mut self, expanded: bool, bottom: bool, maximum: Pixels) {
        if !expanded
            || !self.expanded
            || self.bottom != bottom
            || (bottom && self.scroll == self.max_scroll)
        {
            self.scroll = if bottom { maximum } else { px(0.) };
        }
        self.expanded = expanded;
        self.bottom = bottom;
        self.max_scroll = maximum;
        self.scroll = self.scroll.clamp(px(0.), maximum);
        self.epoch = self.epoch.wrapping_add(1);
    }
}
pub struct Content {
    pub id: NodeId,
    pub ending: bool,
    /// Preview only. The owner commits this token in [after_paint], never layout.
    pub presentation: Option<crate::toast_lifecycle::Frame>,
    pub element: AnyElement,
}
pub struct Render {
    pub state: Shared,
    pub width: Pixels,
    pub anchor: Anchor,
    pub layering: Layering,
    pub expanded: bool,
    pub oldest_first: bool,
    pub enabled: bool,
    pub pointer: bool,
    pub spring: Option<gpuio_protocol::animation::Spring>,
    pub now: Duration,
    /// Oldest to newest. The caller retains any resource-owning children which
    /// are omitted from painting by collapsed geometry.
    pub contents: Vec<Content>,
    pub observe: Observer,
    pub after_paint: PaintObserver,
    pub hover: HoverObserver,
}
pub fn element(render: Render) -> Result<AnyElement, geometry::Error> {
    if !render.layering.is_valid() || render.spring.is_some_and(|spring| !spring.is_valid()) {
        return Err(geometry::Error::InvalidParameters);
    }
    let mut ids = BTreeSet::new();
    if !f32::from(render.width).is_finite()
        || render.width < px(0.)
        || render.width > px(1_000_000.)
        || render.contents.len() > 32
        || render.contents.iter().any(|c| !ids.insert(c.id))
    {
        return Err(geometry::Error::InvalidMeasurements);
    }
    Ok(Stack {
        state: Rc::downgrade(&render.state),
        width: render.width,
        anchor: render.anchor,
        layering: render.layering,
        expanded: render.expanded,
        oldest_first: render.oldest_first,
        enabled: render.enabled,
        pointer: render.pointer,
        spring: render.spring,
        now: render.now,
        contents: Some(render.contents),
        observe: render.observe,
        after_paint: render.after_paint,
        hover: render.hover,
    }
    .into_any_element())
}
struct Stack {
    state: Weak<RefCell<State>>,
    width: Pixels,
    anchor: Anchor,
    layering: Layering,
    expanded: bool,
    oldest_first: bool,
    enabled: bool,
    pointer: bool,
    spring: Option<gpuio_protocol::animation::Spring>,
    now: Duration,
    contents: Option<Vec<Content>>,
    observe: Observer,
    after_paint: PaintObserver,
    hover: HoverObserver,
}
struct Painted {
    frame: Frame,
    children: Vec<AnyElement>,
    hitbox: Hitbox,
    epoch: u64,
    reflow: reflow::Frame,
    mask: ContentMask<Pixels>,
}
impl IntoElement for Stack {
    type Element = Self;
    fn into_element(self) -> Self {
        self
    }
}
fn bottom(anchor: Anchor) -> bool {
    matches!(
        anchor,
        Anchor::BottomLeft | Anchor::BottomRight | Anchor::BottomCenter
    )
}
fn aligned(anchor: Anchor, bounds: Bounds<Pixels>, extent: gpui::Size<Pixels>) -> Point<Pixels> {
    let x = match anchor {
        Anchor::TopLeft | Anchor::BottomLeft | Anchor::LeftCenter => bounds.left(),
        Anchor::TopRight | Anchor::BottomRight | Anchor::RightCenter => {
            bounds.right() - extent.width
        }
        _ => bounds.left() + (bounds.size.width - extent.width) / 2.,
    };
    let y = match anchor {
        Anchor::TopLeft | Anchor::TopRight | Anchor::TopCenter => bounds.top(),
        Anchor::BottomLeft | Anchor::BottomRight | Anchor::BottomCenter => {
            bounds.bottom() - extent.height
        }
        _ => bounds.top() + (bounds.size.height - extent.height) / 2.,
    };
    point(x, y)
}
impl Element for Stack {
    type RequestLayoutState = ();
    type PrepaintState = Option<Painted>;
    fn id(&self) -> Option<ElementId> {
        None
    }
    fn source_location(&self) -> Option<&'static std::panic::Location<'static>> {
        None
    }
    fn request_layout(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        window: &mut Window,
        cx: &mut App,
    ) -> (LayoutId, ()) {
        (
            window.request_layout(
                gpui::Style {
                    size: size(gpui::relative(1.).into(), gpui::relative(1.).into()),
                    ..Default::default()
                },
                [],
                cx,
            ),
            (),
        )
    }
    fn prepaint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>,
        _: &mut (),
        window: &mut Window,
        cx: &mut App,
    ) -> Option<Painted> {
        let state = self.state.upgrade()?;
        let width = self.width.min(bounds.size.width).max(px(0.));
        let contents = self.contents.take()?;
        if width <= px(0.) || bounds.size.height <= px(0.) {
            state.borrow_mut().suspend();
            return None;
        }
        let count = contents.len();
        let spring = self
            .spring
            .filter(|_| self.enabled && window.is_window_active() && !cx.reduce_motion());
        let desired_widths: Vec<_> = contents
            .iter()
            .enumerate()
            .map(|(index, content)| {
                let rank = count - index - 1;
                let value = if self.expanded {
                    width
                } else {
                    width
                        * (1.
                            - self.layering.width_step as f32
                                * rank.min(self.layering.visible - 1) as f32)
                };
                (content.id, f64::from(f32::from(value)))
            })
            .collect();
        let width_frame = state
            .borrow_mut()
            .reflow
            .measure_widths(&desired_widths, spring, self.now)
            .ok()?;
        let mut cards = Vec::with_capacity(count);
        let mut measured = Vec::with_capacity(count);
        let mut visuals = Vec::with_capacity(count);
        // Sample width before measuring: wrapping uses the width actually painted,
        // not a future target or a cached height from the previous frame.
        for (index, (content, preview)) in contents.into_iter().zip(width_frame.items()).enumerate()
        {
            let rank = count - index - 1;
            let item_width = px(preview.rect.width as f32);
            let identity: ElementId = (
                "toast-measured-card",
                ((content.id.generation() as u64) << 32) | content.id.slot() as u64,
            )
                .into();
            let visual = content.presentation.as_ref().map_or(
                crate::toast_lifecycle::Visual {
                    opacity: 1.,
                    slide: 0.,
                },
                |frame| frame.visual(),
            );
            visuals.push(visual);
            let interactive =
                !content.ending && visual.opacity > 0. && (self.expanded || rank == 0);
            let body = div()
                .id(identity.clone())
                .w(item_width)
                .min_w_0()
                .opacity(visual.opacity as f32)
                .child(content.element);
            let mut child = if interactive && self.enabled {
                body.into_any_element()
            } else if interactive {
                crate::semantics::InteractionShield::disabled(body).into_any_element()
            } else {
                crate::semantics::InteractionShield::inert(body, identity).into_any_element()
            };
            let measured_size = child.layout_as_root(
                size(
                    AvailableSpace::Definite(item_width),
                    AvailableSpace::MaxContent,
                ),
                window,
                cx,
            );
            cards.push(Card {
                id: content.id,
                height: f64::from(f32::from(measured_size.height)),
                ending: content.ending,
            });
            measured.push(child);
        }
        let layout = geometry::layout(
            &cards,
            f64::from(f32::from(width)),
            self.layering,
            self.expanded,
            (self.expanded && self.oldest_first) || bottom(self.anchor),
        )
        .ok()?;
        let height = px(layout.height as f32);
        let extent = size(width, height.min(bounds.size.height));
        let viewport = Bounds::new(aligned(self.anchor, bounds, extent), extent);
        state.borrow_mut().measure(
            self.expanded,
            bottom(self.anchor),
            (height - extent.height).max(px(0.)),
        );
        let (scroll, max_scroll, epoch) = {
            let s = state.borrow();
            (s.scroll, s.max_scroll, s.epoch)
        };
        let vertical_anchor = match self.anchor {
            Anchor::TopLeft | Anchor::TopRight | Anchor::TopCenter => 0.,
            Anchor::BottomLeft | Anchor::BottomRight | Anchor::BottomCenter => 1.,
            Anchor::LeftCenter | Anchor::RightCenter => 0.5,
        };
        // Interpolate a zero-height rectangle's center/vertical anchor and width.
        // Its height is always measured native content, not an independent spring.
        let targets: Vec<_> = layout
            .items
            .iter()
            .map(|item| reflow::Target {
                id: item.id,
                rect: reflow::Rect {
                    x: f64::from(f32::from(viewport.origin.x)) + item.x + item.width / 2.,
                    y: f64::from(f32::from(viewport.origin.y - scroll))
                        + item.y
                        + item.height * vertical_anchor,
                    width: desired_widths
                        .iter()
                        .find(|(id, _)| *id == item.id)
                        .unwrap()
                        .1,
                    height: 0.,
                },
            })
            .collect();
        state
            .borrow_mut()
            .reflow
            .configure_frame(&targets, spring, self.now)
            .ok()?;
        let mut reflow = state.borrow().reflow.sample(self.now);
        // Do not spend frames travelling from a position that a resize has put
        // completely outside the usable surface. Width remains measurable now.
        let outside: Vec<_> = layout
            .items
            .iter()
            .zip(reflow.items())
            .filter_map(|(item, sample)| {
                let current = Bounds::new(
                    point(
                        px((sample.rect.x - sample.rect.width / 2.) as f32),
                        px((sample.rect.y - item.height * vertical_anchor) as f32),
                    ),
                    size(px(sample.rect.width as f32), px(item.height as f32)),
                )
                .intersect(&bounds);
                let target = Bounds::new(
                    viewport.origin + point(px(item.x as f32), px(item.y as f32) - scroll),
                    size(px(item.width as f32), px(item.height as f32)),
                )
                .intersect(&viewport);
                (item.visible
                    && sample.rect.width > 0.
                    && item.height > 0.
                    && (current.size.width <= px(0.) || current.size.height <= px(0.))
                    && target.size.width > px(0.)
                    && target.size.height > px(0.))
                .then_some(item.id)
            })
            .collect();
        if !outside.is_empty() {
            state.borrow_mut().reflow.settle_positions(&outside).ok()?;
            reflow = state.borrow().reflow.sample(self.now);
        }
        let mask = ContentMask {
            bounds: (if reflow.needs_frame() {
                bounds
            } else {
                viewport
            })
            .intersect(&window.content_mask().bounds),
        };
        window.with_content_mask(Some(mask), |w| {
            w.insert_hitbox(viewport, HitboxBehavior::BlockMouseExceptScroll)
        });
        let mut children = Vec::new();
        let mut items = Vec::with_capacity(count);
        window.with_content_mask(Some(mask), |window| {
            for (((item, mut child), sample), visual) in layout
                .items
                .into_iter()
                .zip(measured)
                .zip(reflow.items())
                .zip(visuals)
            {
                let bounds = Bounds::new(
                    point(
                        px((sample.rect.x - sample.rect.width / 2.) as f32),
                        px((sample.rect.y - item.height * vertical_anchor) as f32),
                    ),
                    size(px(sample.rect.width as f32), px(item.height as f32)),
                );
                // Eligibility uses unslid layout bounds. Otherwise the initial
                // fully transparent/outside entry would never commit and start.
                let clipped = bounds.intersect(&mask.bounds);
                let bounds = Bounds::new(
                    bounds.origin + point(px(0.), px(visual.slide as f32)),
                    bounds.size,
                );
                let visible = bounds.intersect(&mask.bounds);
                let painted =
                    item.visible && clipped.size.width > px(0.) && clipped.size.height > px(0.);
                items.push(Item {
                    id: item.id,
                    bounds,
                    interactive: painted
                        && item.interactive
                        && self.enabled
                        && visual.opacity > 0.
                        && visible.size.width > px(0.)
                        && visible.size.height > px(0.),
                    painted,
                });
                if painted {
                    child.prepaint_at(bounds.origin, window, cx);
                    children.push(child);
                }
            }
        });
        // A nonblocking region above the cards observes their entire footprint,
        // including cards that occlude content behind themselves. Child scroll
        // listeners still bubble first because they are registered after ours.
        let hitbox = window.with_content_mask(Some(mask), |w| {
            w.insert_hitbox(viewport, HitboxBehavior::Normal)
        });
        let visible: Vec<_> = items
            .iter()
            .filter(|item| item.painted)
            .map(|item| item.id)
            .collect();
        let needs_frame = reflow.needs_frame_for(&visible);
        Some(Painted {
            frame: Frame {
                viewport,
                items,
                scroll,
                max_scroll,
                needs_frame,
            },
            children,
            hitbox,
            epoch,
            reflow,
            mask,
        })
    }
    fn paint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        _: Bounds<Pixels>,
        _: &mut (),
        painted: &mut Option<Painted>,
        window: &mut Window,
        cx: &mut App,
    ) {
        let Some(painted) = painted else {
            return;
        };
        let Some(state) = self.state.upgrade() else {
            return;
        };
        if state.borrow().epoch != painted.epoch {
            return;
        }
        if window.last_input_was_keyboard() && state.borrow().hovered {
            state.borrow_mut().hovered = false;
            (self.hover)(false, window, cx);
        }
        // Host eligibility must precede child focus recording and input binding.
        (self.observe)(&painted.frame, window, cx);
        let epoch = painted.epoch;
        let weak = self.state.clone();
        let hitbox = painted.hitbox.clone();
        let enabled = self.enabled && self.pointer && self.expanded;
        window.on_mouse_event(move |event: &gpui::ScrollWheelEvent, phase, window, cx| {
            if !enabled || !phase.bubble() || !hitbox.should_handle_scroll(window) {
                return;
            }
            let Some(state) = weak.upgrade() else {
                return;
            };
            let mut state = state.borrow_mut();
            if state.epoch != epoch {
                return;
            }
            let delta = event.delta.pixel_delta(px(16.)).y;
            if state.scroll_by(-delta) {
                window.refresh();
                cx.stop_propagation();
            }
        });
        let weak = self.state.clone();
        let hitbox = painted.hitbox.clone();
        let hover = self.hover.clone();
        let enabled = self.enabled && self.pointer;
        window.on_mouse_event(move |_: &gpui::MouseMoveEvent, phase, window, cx| {
            if !phase.bubble() {
                return;
            }
            let Some(state) = weak.upgrade() else {
                return;
            };
            let changed = {
                let mut state = state.borrow_mut();
                if state.epoch != epoch {
                    return;
                }
                let hovered = enabled && hitbox.is_hovered(window);
                if state.hovered == hovered {
                    None
                } else {
                    state.hovered = hovered;
                    Some(hovered)
                }
            };
            if let Some(value) = changed {
                hover(value, window, cx);
            }
        });
        let weak = self.state.clone();
        let hover = self.hover.clone();
        window.on_mouse_event(move |_: &gpui::MouseExitEvent, phase, window, cx| {
            if !phase.bubble() {
                return;
            }
            let Some(state) = weak.upgrade() else {
                return;
            };
            let changed = {
                let mut state = state.borrow_mut();
                if state.epoch != epoch {
                    return;
                }
                std::mem::replace(&mut state.hovered, false)
            };
            if changed {
                hover(false, window, cx);
            }
        });
        window.with_content_mask(Some(painted.mask), |window| {
            for child in &mut painted.children {
                child.paint(window, cx);
            }
        });
        let visible: Vec<_> = painted
            .frame
            .items
            .iter()
            .filter(|i| i.painted)
            .map(|i| i.id)
            .collect();
        let reflow_pending = state
            .borrow_mut()
            .reflow
            .painted_subset(&painted.reflow, &visible)
            && painted.reflow.needs_frame_for(&visible);
        let lifecycle_pending = (self.after_paint)(&painted.frame, window, cx);
        if !visible.is_empty()
            && (reflow_pending || lifecycle_pending)
            && window.is_window_active()
            && !cx.reduce_motion()
            && self.enabled
        {
            window.request_animation_frame();
        }
    }
}
