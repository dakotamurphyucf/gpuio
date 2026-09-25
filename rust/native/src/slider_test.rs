use super::super::{
    editor_test::key,
    native_test::{mouse, move_mouse},
};
use super::*;
use gpuio_protocol::{numeric::Domain, slider as s};
fn config() -> s::Config {
    s::Config {
        domain: Domain::new(-2., 8., 0.5).unwrap(),
        label: "Range".into(),
        lower_label: "Minimum".into(),
        upper_label: "Maximum".into(),
        axis: s::Axis::Horizontal,
        scale: s::Scale::Linear,
        disabled: false,
        read_only: false,
    }
}
fn initial() -> s::Value {
    s::Value::Range {
        lower: 2.,
        upper: 7.,
    }
}
fn snapshot(cx: &mut gpui::AsyncApp, handle: WindowHandle<View>) -> s::Snapshot {
    handle
        .update(cx, |v, _, _| v.sliders[&node(1)].borrow().model.snapshot())
        .unwrap()
}
fn events(transport: &Transport) -> Vec<s::Event> {
    transport
        .mailbox
        .lock()
        .unwrap()
        .drain(128)
        .into_iter()
        .filter_map(|event| match event {
            Event::SliderEvent(_, id, _, _, event) if id == node(1) => Some(event),
            Event::Press(_, id, ..) if id == node(1) => panic!("slider emitted Press"),
            _ => None,
        })
        .collect()
}
fn position(
    cx: &mut gpui::AsyncApp,
    handle: WindowHandle<View>,
    fraction: f32,
) -> gpui::Point<gpui::Pixels> {
    handle
        .update(cx, |v, _, _| {
            let b = v.probes.borrow()[&node(1)].bounds;
            gpui::point(
                b.left() + px(10.) + (b.size.width - px(20.)) * fraction,
                b.center().y,
            )
        })
        .unwrap()
}
#[cfg(target_os = "macos")]
#[derive(Clone, Copy)]
enum Action {
    Read,
    Increase,
    Decrease,
    Set(f64),
}
#[cfg(target_os = "macos")]
fn accessible(
    cx: &mut gpui::AsyncApp,
    handle: WindowHandle<View>,
    label: &str,
    action: Action,
) -> Option<(f64, f64, f64, bool)> {
    use objc2::{
        class, msg_send,
        runtime::{AnyObject, Bool},
    };
    use objc2_foundation::NSString;
    unsafe fn visit(
        object: *mut AnyObject,
        label: &str,
        action: Action,
        depth: usize,
    ) -> Option<(f64, f64, f64, bool)> {
        if object.is_null() || depth > 20 {
            return None;
        }
        unsafe {
            let role: *mut NSString = msg_send![object, accessibilityRole];
            let title: *mut NSString = msg_send![object, accessibilityTitle];
            if role.as_ref().is_some_and(|s| s.to_string() == "AXSlider")
                && title.as_ref().is_some_and(|s| s.to_string() == label)
            {
                let value: *mut AnyObject = msg_send![object, accessibilityValue];
                let min: *mut AnyObject = msg_send![object, accessibilityMinValue];
                let max: *mut AnyObject = msg_send![object, accessibilityMaxValue];
                assert!(!value.is_null() && !min.is_null() && !max.is_null());
                let value: f64 = msg_send![value, doubleValue];
                let min: f64 = msg_send![min, doubleValue];
                let max: f64 = msg_send![max, doubleValue];
                let enabled: Bool = msg_send![object, isAccessibilityEnabled];
                match action {
                    Action::Read => (),
                    Action::Increase => {
                        let _: Bool = msg_send![object, accessibilityPerformIncrement];
                    }
                    Action::Decrease => {
                        let _: Bool = msg_send![object, accessibilityPerformDecrement];
                    }
                    Action::Set(value) => {
                        let number: *mut AnyObject =
                            msg_send![class!(NSNumber),numberWithDouble:value];
                        let _: () = msg_send![object,setAccessibilityValue:number];
                    }
                }
                return Some((value, min, max, enabled.as_bool()));
            }
            let children: *mut AnyObject = msg_send![object, accessibilityChildren];
            if children.is_null() {
                return None;
            }
            let count: usize = msg_send![children, count];
            assert!(count < 128);
            for i in 0..count {
                let child: *mut AnyObject = msg_send![children,objectAtIndex:i];
                if let Some(found) = visit(child, label, action, depth + 1) {
                    return Some(found);
                }
            }
            None
        }
    }
    let address = super::super::editor_test::native_view(cx, handle) as *mut AnyObject;
    unsafe {
        let window: *mut AnyObject = msg_send![address, window];
        let content: *mut AnyObject = msg_send![window, contentView];
        visit(content, label, action, 0)
    }
}

pub(super) async fn exercise(
    cx: &mut gpui::AsyncApp,
    handle: WindowHandle<View>,
    transport: &Transport,
) {
    apply(
        cx,
        handle,
        vec![
            Op::Create(node(0), Kind::FocusScope, "".into(), None),
            Op::SetFocusScope(
                node(0),
                FocusScopeConfig {
                    trap: true,
                    auto_focus: false,
                    restore_focus: true,
                },
            ),
            Op::Create(node(1), Kind::Slider, "".into(), Some(handler(1))),
            Op::SetSlider(node(1), config(), initial()),
            Op::Create(
                node(2),
                Kind::Button,
                "After slider".into(),
                Some(handler(2)),
            ),
            Op::Splice(node(0), 0, 0, vec![node(1), node(2)]),
            Op::SetRoot(Some(node(0))),
        ],
    );
    frame(cx, handle).await;
    assert_eq!(snapshot(cx, handle).value, initial());
    assert!(matches!(
        events(transport).as_slice(),
        [s::Event::Observed(s::Snapshot { revision: 0, .. })]
    ));
    let owner = handle
        .update(cx, |v, _, _| Rc::downgrade(&v.sliders[&node(1)]))
        .unwrap();
    handle
        .update(cx, |v, w, cx| {
            w.focus(&v.sliders[&node(1)].borrow().focus[0].1, cx)
        })
        .unwrap();
    frame(cx, handle).await;
    assert!(
        handle
            .update(cx, |v, w, _| v.sliders[&node(1)].borrow().focus[0]
                .1
                .is_focused(w))
            .unwrap()
    );
    key(cx, handle, "right");
    assert_eq!(
        snapshot(cx, handle).value,
        s::Value::Range {
            lower: 2.5,
            upper: 7.
        }
    );
    key(cx, handle, "tab");
    assert!(
        handle
            .update(cx, |v, w, _| v.sliders[&node(1)].borrow().focus[1]
                .1
                .is_focused(w))
            .unwrap()
    );
    key(cx, handle, "left");
    assert_eq!(
        snapshot(cx, handle).value,
        s::Value::Range {
            lower: 2.5,
            upper: 6.5
        }
    );
    key(cx, handle, "home");
    assert_eq!(
        snapshot(cx, handle).value,
        s::Value::Range {
            lower: 2.5,
            upper: 2.5
        }
    );
    key(cx, handle, "end");
    assert_eq!(
        snapshot(cx, handle).value,
        s::Value::Range {
            lower: 2.5,
            upper: 8.
        }
    );
    frame(cx, handle).await;
    events(transport);
    #[cfg(target_os = "macos")]
    {
        // The first AppKit query enables GPUI accessibility; paint the requested tree.
        accessible(cx, handle, "Minimum", Action::Read);
        frame(cx, handle).await;
        assert_eq!(
            accessible(cx, handle, "Minimum", Action::Read),
            Some((2.5, -2., 8., true))
        );
        assert_eq!(
            accessible(cx, handle, "Maximum", Action::Read),
            Some((8., 2.5, 8., true))
        );
        accessible(cx, handle, "Minimum", Action::Increase).unwrap();
        frame(cx, handle).await;
        assert_eq!(
            snapshot(cx, handle).value,
            s::Value::Range {
                lower: 3.,
                upper: 8.
            }
        );
        accessible(cx, handle, "Minimum", Action::Decrease).unwrap();
        frame(cx, handle).await;
        accessible(cx, handle, "Maximum", Action::Set(7.)).unwrap();
        frame(cx, handle).await;
        assert_eq!(
            snapshot(cx, handle).value,
            s::Value::Range {
                lower: 2.5,
                upper: 7.
            }
        );
        accessible(cx, handle, "Maximum", Action::Set(8.)).unwrap();
        frame(cx, handle).await;
        assert!(
            events(transport)
                .iter()
                .all(|e| matches!(e, s::Event::Committed(s::Source::Accessibility, _)))
        );
    }
    // Native values survive different same-mode initial values in a rerender.
    apply(
        cx,
        handle,
        vec![Op::SetSlider(
            node(1),
            config(),
            s::Value::Range {
                lower: -2.,
                upper: -1.,
            },
        )],
    );
    frame(cx, handle).await;
    assert_eq!(
        snapshot(cx, handle).value,
        s::Value::Range {
            lower: 2.5,
            upper: 8.
        }
    );
    assert!(events(transport).is_empty());
    let lower = position(cx, handle, 0.45);
    let destination = position(cx, handle, 0.1);
    mouse(cx, handle, lower, true);
    move_mouse(cx, handle, destination, true);
    frame(cx, handle).await;
    assert_eq!(snapshot(cx, handle).dragging, Some(s::Thumb::Lower));
    assert_eq!(
        snapshot(cx, handle).committed,
        s::Value::Range {
            lower: 2.5,
            upper: 8.
        }
    );
    assert_eq!(
        snapshot(cx, handle).value,
        s::Value::Range {
            lower: -1.,
            upper: 8.
        }
    );
    key(cx, handle, "escape");
    assert_eq!(snapshot(cx, handle).value, snapshot(cx, handle).committed);
    assert_eq!(snapshot(cx, handle).dragging, None);
    mouse(cx, handle, destination, false);
    let sequence = events(transport);
    assert!(matches!(sequence.first(), Some(s::Event::DragStarted(_))));
    assert!(matches!(
        sequence.last(),
        Some(s::Event::Cancelled(s::CancelReason::Escape, _))
    ));
    frame(cx, handle).await;
    mouse(cx, handle, lower, true);
    move_mouse(cx, handle, destination, true);
    mouse(cx, handle, destination, false);
    assert_eq!(
        snapshot(cx, handle).value,
        s::Value::Range {
            lower: -1.,
            upper: 8.
        }
    );
    assert_eq!(snapshot(cx, handle).value, snapshot(cx, handle).committed);
    assert!(matches!(
        events(transport).last(),
        Some(s::Event::Committed(s::Source::Pointer, _))
    ));
    // A bound update cancels using the old domain, then publishes its normalization.
    frame(cx, handle).await;
    let lower = position(cx, handle, 0.1);
    mouse(cx, handle, lower, true);
    move_mouse(cx, handle, destination + gpui::point(px(20.), px(0.)), true);
    events(transport);
    let bounded = s::Config {
        domain: Domain::new(0., 4., 0.5).unwrap(),
        ..config()
    };
    apply(
        cx,
        handle,
        vec![Op::SetSlider(node(1), bounded.clone(), initial())],
    );
    let sequence = events(transport);
    assert!(matches!(
        sequence.as_slice(),
        [
            s::Event::Cancelled(s::CancelReason::ConfigurationChanged, _),
            s::Event::Observed(_)
        ]
    ));
    assert_eq!(
        snapshot(cx, handle).value,
        s::Value::Range {
            lower: 0.,
            upper: 4.
        }
    );
    mouse(cx, handle, destination, false);
    frame(cx, handle).await;
    let start = position(cx, handle, 0.);
    mouse(cx, handle, start, true);
    let middle = position(cx, handle, 0.5);
    move_mouse(cx, handle, middle, true);
    events(transport);
    apply(
        cx,
        handle,
        vec![Op::SetStyle(
            node(0),
            vec![Style::Fields(vec![gpuio_protocol::v1::Field::Display(3)])],
        )],
    );
    assert!(matches!(
        events(transport).last(),
        Some(s::Event::Cancelled(s::CancelReason::Hidden, _))
    ));
    mouse(cx, handle, start, false);
    frame(cx, handle).await;
    #[cfg(target_os = "macos")]
    assert!(accessible(cx, handle, "Minimum", Action::Read).is_none());
    apply(cx, handle, vec![Op::SetStyle(node(0), vec![])]);
    frame(cx, handle).await;
    mouse(cx, handle, start, true);
    let middle = position(cx, handle, 0.5);
    move_mouse(cx, handle, middle, true);
    events(transport);
    apply(
        cx,
        handle,
        vec![
            Op::Create(node(3), Kind::FocusScope, "".into(), None),
            Op::SetFocusScope(
                node(3),
                FocusScopeConfig {
                    trap: true,
                    auto_focus: true,
                    restore_focus: true,
                },
            ),
            Op::Create(
                node(4),
                Kind::Button,
                "Modal action".into(),
                Some(handler(4)),
            ),
            Op::Splice(node(3), 0, 0, vec![node(4)]),
            Op::Splice(node(0), 2, 0, vec![node(3)]),
        ],
    );
    assert!(matches!(
        events(transport).last(),
        Some(s::Event::Cancelled(s::CancelReason::Modal, _))
    ));
    mouse(cx, handle, start, false);
    frame(cx, handle).await;
    #[cfg(target_os = "macos")]
    {
        let before = snapshot(cx, handle);
        accessible(cx, handle, "Minimum", Action::Increase);
        assert_eq!(snapshot(cx, handle), before);
    }
    apply(
        cx,
        handle,
        vec![
            Op::Splice(node(0), 2, 1, vec![]),
            Op::Remove(node(4)),
            Op::Remove(node(3)),
        ],
    );
    frame(cx, handle).await;
    // Read-only retains focus but keyboard does not mutate native state.
    apply(
        cx,
        handle,
        vec![Op::SetSlider(
            node(1),
            s::Config {
                read_only: true,
                ..bounded.clone()
            },
            initial(),
        )],
    );
    frame(cx, handle).await;
    handle
        .update(cx, |v, w, cx| {
            w.focus(&v.sliders[&node(1)].borrow().focus[0].1, cx)
        })
        .unwrap();
    let before = snapshot(cx, handle);
    key(cx, handle, "right");
    assert_eq!(snapshot(cx, handle), before);
    apply(
        cx,
        handle,
        vec![Op::SetSlider(
            node(1),
            s::Config {
                disabled: true,
                ..bounded
            },
            initial(),
        )],
    );
    frame(cx, handle).await;
    assert!(
        handle
            .update(cx, |v, w, _| v.sliders[&node(1)]
                .borrow()
                .focus
                .iter()
                .all(|(_, f)| !f.is_focused(w)))
            .unwrap()
    );
    apply(
        cx,
        handle,
        vec![
            Op::SetRoot(None),
            Op::Remove(node(1)),
            Op::Remove(node(2)),
            Op::Remove(node(0)),
        ],
    );
    frame(cx, handle).await;
    assert!(owner.upgrade().is_none());
    assert_eq!(
        handle
            .update(cx, |v, _, _| v.session.borrow().retained_bytes())
            .unwrap(),
        0
    );
    // A fresh single-thumb owner exercises vertical logarithmic geometry.
    let single_config = s::Config {
        domain: Domain::new(1., 1000., 1.).unwrap(),
        label: "Logarithmic".into(),
        axis: s::Axis::Vertical,
        scale: s::Scale::Logarithmic,
        ..config()
    };
    apply(
        cx,
        handle,
        vec![
            Op::Create(node(5), Kind::Container, "".into(), None),
            Op::Create(node(6), Kind::Slider, "".into(), Some(handler(6))),
            Op::SetSlider(node(6), single_config, s::Value::Single(10.)),
            Op::Splice(node(5), 0, 0, vec![node(6)]),
            Op::SetRoot(Some(node(5))),
        ],
    );
    frame(cx, handle).await;
    let value = |cx: &mut gpui::AsyncApp| {
        handle
            .update(cx, |v, _, _| {
                v.sliders[&node(6)].borrow().model.snapshot().value
            })
            .unwrap()
    };
    handle
        .update(cx, |v, w, cx| {
            w.focus(&v.sliders[&node(6)].borrow().focus[0].1, cx)
        })
        .unwrap();
    frame(cx, handle).await;
    key(cx, handle, "up");
    assert_eq!(value(cx), s::Value::Single(11.));
    key(cx, handle, "home");
    assert_eq!(value(cx), s::Value::Single(1.));
    key(cx, handle, "end");
    assert_eq!(value(cx), s::Value::Single(1000.));
    key(cx, handle, "pagedown");
    assert_eq!(value(cx), s::Value::Single(990.));
    frame(cx, handle).await;
    let bounds = handle
        .update(cx, |v, _, _| v.probes.borrow()[&node(6)].bounds)
        .unwrap();
    mouse(cx, handle, bounds.center(), true);
    assert_eq!(value(cx), s::Value::Single(32.));
    mouse(cx, handle, bounds.center(), false);
    frame(cx, handle).await;
    #[cfg(target_os = "macos")]
    assert_eq!(
        accessible(cx, handle, "Logarithmic", Action::Read),
        Some((32., 1., 1000., true))
    );
    let at_thumb = gpui::point(
        bounds.center().x,
        bounds.bottom() - px(10.) - (bounds.size.height - px(20.)) * (32_f32.ln() / 1000_f32.ln()),
    );
    mouse(cx, handle, at_thumb, true);
    let beyond = gpui::point(bounds.center().x, bounds.bottom() + px(100.));
    move_mouse(cx, handle, beyond, true);
    mouse(cx, handle, beyond, false);
    assert_eq!(value(cx), s::Value::Single(1.));
    apply(
        cx,
        handle,
        vec![Op::SetRoot(None), Op::Remove(node(6)), Op::Remove(node(5))],
    );
    frame(cx, handle).await;
    assert!(handle.update(cx, |v, _, _| v.sliders.is_empty()).unwrap());
    #[cfg(target_os = "macos")]
    eprintln!(
        "GPUIO_SLIDER_AX_OK: independent range values/actions/bounds, hidden/modal policy, single logarithmic value"
    );
    eprintln!(
        "GPUIO_SLIDER_NATIVE_OK: two-thumb keyboard, pointer preview/commit/cancel, retained values, domain/policy updates and disposal"
    );
}
