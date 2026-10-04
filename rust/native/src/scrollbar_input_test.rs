use super::*;
use gpui::{Point, Size, size};
use std::cell::Cell;
struct Handle {
    bounds: Cell<Bounds<Pixels>>,
    content: Cell<Size<Pixels>>,
    offset: Cell<Point<Pixels>>,
    started: Cell<usize>,
    ended: Cell<usize>,
    writes: Cell<usize>,
}
impl ScrollbarHandle for Handle {
    fn viewport_bounds(&self) -> Bounds<Pixels> {
        self.bounds.get()
    }
    fn content_size(&self) -> Size<Pixels> {
        self.content.get()
    }
    fn offset(&self) -> Point<Pixels> {
        self.offset.get()
    }
    fn set_offset(&self, offset: Point<Pixels>) {
        self.offset.set(offset);
        self.writes.set(self.writes.get() + 1);
    }
    fn start_drag(&self) {
        self.started.set(self.started.get() + 1);
    }
    fn end_drag(&self) {
        self.ended.set(self.ended.get() + 1);
    }
}
fn styles() -> [Style; 2] {
    [Style {
        envelope_width: 16.,
        track_width: 16.,
        thumb_width: 8.,
        inset: 2.,
        radius: 3.,
        min_length: 12.,
    }; 2]
}
fn setup() -> (Rc<Handle>, State) {
    let handle = Rc::new(Handle {
        bounds: Cell::new(Bounds {
            origin: point(px(20.), px(30.)),
            size: size(px(200.), px(100.)),
        }),
        content: Cell::new(size(px(800.), px(400.))),
        offset: Cell::new(point(px(0.), px(0.))),
        started: Cell::new(0),
        ended: Cell::new(0),
        writes: Cell::new(0),
    });
    let state = State::new(handle.clone(), Selection::Both, styles()).unwrap();
    (handle, state)
}
fn grip(state: &State, axis: Axis) -> [f64; 2] {
    let m = state.measurement().unwrap();
    let bar = bar(m.bars, axis).unwrap();
    [
        f64::from(f32::from(m.viewport.origin.x)) + bar.thumb.origin[0] + bar.thumb.size[0] / 2.,
        f64::from(f32::from(m.viewport.origin.y)) + bar.thumb.origin[1] + bar.thumb.size[1] / 2.,
    ]
}
fn close(a: f32, b: f32) {
    assert!((a - b).abs() < 0.001, "{a} != {b}");
}

#[test]
fn sibling_overflow_changes_corner_without_changing_range_or_offset() {
    let (handle, mut state) = setup();
    let (peer, _) = setup();
    state
        .reconcile(
            Selection::Vertical,
            styles(),
            None,
            Policy {
                enabled: true,
                pointer: true,
            },
        )
        .unwrap();
    state.set_corner_peer(Some(peer.clone()));
    let bar = state.measurement().unwrap().bars.vertical.unwrap();
    assert_eq!(bar.envelope.size[1], 84.);
    assert_eq!(bar.max_offset, 300.);
    peer.content.set(size(px(200.), px(400.)));
    let bar = state.measurement().unwrap().bars.vertical.unwrap();
    assert_eq!(
        bar.envelope.size[1], 100.,
        "no horizontal overflow, no corner gap"
    );
    assert_eq!(bar.max_offset, 300.);
    assert_eq!(handle.writes.get(), 0);
    peer.content.set(size(px(800.), px(400.)));
    state.set_corner_peer(None);
    assert_eq!(
        state
            .measurement()
            .unwrap()
            .bars
            .vertical
            .unwrap()
            .envelope
            .size[1],
        100.
    );
}

#[test]
fn drag_mutates_the_existing_handle_and_remeasures_resize_with_balanced_hooks() {
    let (h, mut s) = setup();
    h.offset.set(point(px(-40.), px(-50.)));
    let start = grip(&s, Axis::Vertical);
    assert!(s.begin_drag(Axis::Vertical, start));
    assert_eq!(h.started.get(), 1);
    assert!(!s.drag_to(start));
    let mut moved = start;
    moved[1] += 30.;
    assert!(s.drag_to(moved));
    close(f32::from(h.offset.get().x), -40.);
    close(f32::from(h.offset.get().y), -200.);
    // New content/viewport while capture is active: use current travel, same grip.
    h.bounds.set(Bounds {
        origin: point(px(20.), px(30.)),
        size: size(px(200.), px(200.)),
    });
    h.content.set(size(px(800.), px(1000.)));
    assert!(s.drag_to([start[0], 500.]));
    assert_eq!(h.offset.get(), point(px(-40.), px(-800.)));
    assert_eq!(h.started.get(), 1);
    assert_eq!(h.ended.get(), 0);
    assert!(s.cancel());
    assert!(!s.cancel());
    assert_eq!(h.ended.get(), 1);
    assert_eq!(h.offset.get().y, px(-800.));
}

#[test]
fn keyboard_and_accessibility_share_clamped_offsets_preserving_other_axis() {
    let (h, mut s) = setup();
    h.offset.set(point(px(-90.), px(-50.)));
    assert!(s.adjust(
        Axis::Vertical,
        Adjustment::Line {
            direction: Direction::Forward,
            height: 20.
        }
    ));
    assert_eq!(h.offset.get(), point(px(-90.), px(-70.)));
    assert!(s.adjust(Axis::Vertical, Adjustment::Page(Direction::Forward)));
    assert_eq!(h.offset.get().y, px(-170.));
    assert!(s.adjust(Axis::Vertical, Adjustment::Last));
    assert_eq!(h.offset.get().y, px(-300.));
    assert!(!s.adjust(Axis::Vertical, Adjustment::Last));
    assert!(s.adjust(Axis::Horizontal, Adjustment::Set(999999.)));
    assert_eq!(h.offset.get(), point(px(-600.), px(-300.)));
    assert!(s.adjust(Axis::Horizontal, Adjustment::First));
    assert!(s.adjust(
        Axis::Vertical,
        Adjustment::Line {
            direction: Direction::Backward,
            height: f64::MAX
        }
    ));
    assert_eq!(h.offset.get(), point(px(0.), px(0.)));
    let writes = h.writes.get();
    for v in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        assert!(!s.adjust(Axis::Vertical, Adjustment::Set(v)));
        assert!(!s.adjust(
            Axis::Vertical,
            Adjustment::Line {
                direction: Direction::Forward,
                height: v
            }
        ));
    }
    assert_eq!(h.writes.get(), writes);
    assert_eq!(h.started.get(), 0);
    assert_eq!(h.ended.get(), 0);
}

#[test]
fn selection_policy_overflow_and_drop_cancel_capture_without_rewinding_offsets() {
    for cause in 0..5 {
        let (h, mut s) = setup();
        assert!(s.begin_drag(Axis::Vertical, grip(&s, Axis::Vertical)));
        assert!(s.drag_to([215., 90.]));
        let offset = h.offset.get();
        match cause {
            0 => s
                .reconcile(
                    Selection::Horizontal,
                    styles(),
                    None,
                    Policy {
                        enabled: true,
                        pointer: true,
                    },
                )
                .unwrap(),
            1 => s
                .reconcile(
                    Selection::Both,
                    styles(),
                    None,
                    Policy {
                        enabled: false,
                        pointer: true,
                    },
                )
                .unwrap(),
            2 => s
                .reconcile(
                    Selection::Both,
                    styles(),
                    None,
                    Policy {
                        enabled: true,
                        pointer: false,
                    },
                )
                .unwrap(),
            3 => {
                h.content.set(size(px(800.), px(100.)));
                assert!(!s.drag_to([215., 90.]));
            }
            _ => s.close(),
        }
        assert!(!s.is_dragging());
        assert_eq!(h.ended.get(), 1);
        assert_eq!(h.offset.get(), offset);
        drop(s);
        assert_eq!(h.ended.get(), 1);
    }
    let (h, mut s) = setup();
    assert!(s.begin_drag(Axis::Horizontal, grip(&s, Axis::Horizontal)));
    drop(s);
    assert_eq!(h.started.get(), h.ended.get());
}

#[test]
fn layout_viewport_track_click_and_invalid_updates_use_one_coordinate_contract() {
    let (h, mut s) = setup();
    let viewport = Bounds {
        origin: point(px(120.), px(80.)),
        size: size(px(100.), px(100.)),
    };
    s.reconcile(
        Selection::Vertical,
        styles(),
        Some(viewport),
        Policy {
            enabled: true,
            pointer: true,
        },
    )
    .unwrap();
    assert!(!s.track_click(Axis::Vertical, [50., 130.]));
    assert!(s.track_click(Axis::Vertical, [210., 130.]));
    close(f32::from(h.offset.get().y), -150.);
    assert!(s.begin_drag(Axis::Vertical, grip(&s, Axis::Vertical)));
    let invalid = Style {
        inset: f64::NAN,
        ..styles()[0]
    };
    assert!(
        s.reconcile(
            Selection::Horizontal,
            [invalid; 2],
            None,
            Policy {
                enabled: false,
                pointer: false
            }
        )
        .is_err()
    );
    assert!(s.is_dragging());
    assert_eq!(s.measurement().unwrap().viewport, viewport);
    assert!(!s.drag_to([f64::NAN, 120.]));
    assert!(s.is_dragging());
    // Keyboard remains available when only pointer interaction is disabled.
    s.reconcile(
        Selection::Vertical,
        styles(),
        Some(viewport),
        Policy {
            enabled: true,
            pointer: false,
        },
    )
    .unwrap();
    assert_eq!(h.ended.get(), 1);
    assert!(!s.begin_drag(Axis::Vertical, grip(&s, Axis::Vertical)));
    assert!(!s.track_click(Axis::Vertical, [210., 100.]));
    assert!(s.adjust(Axis::Vertical, Adjustment::First));
    s.close();
    s.reconcile(
        Selection::Both,
        styles(),
        None,
        Policy {
            enabled: true,
            pointer: true,
        },
    )
    .unwrap();
    assert!(!s.adjust(Axis::Vertical, Adjustment::Last));
}
