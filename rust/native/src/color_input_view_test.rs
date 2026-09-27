//! Initial mounted channel and palette acceptance; text/IME and OS AX follow.
use super::super::{
    editor_test::{frame, key},
    native_test::{mouse, move_mouse},
    stop_application,
};
use super::*;
#[cfg(feature = "native-image-tests")]
#[path = "color_input_appearance_test.rs"]
mod appearance;
#[path = "color_input_editors_test.rs"]
mod editors_test;
#[path = "color_input_lifecycle_test.rs"]
mod lifecycle;
#[path = "color_input_workload_test.rs"]
mod workload;
use crate::session::Session;
use gpuio_protocol::v1::{
    CAPABILITIES, Color, Event, Field, Fill, Kind, Op, Style, Transaction, VERSION,
};
use std::{
    cell::RefCell,
    os::fd::{AsRawFd, FromRawFd, OwnedFd},
    rc::Rc,
};
fn node() -> NodeId {
    NodeId::from_parts(0, 1).unwrap()
}
fn config() -> c::Config {
    c::Config {
        labels: c::Labels {
            control: "Accent".into(),
            hue: "Hue".into(),
            saturation: "Saturation".into(),
            lightness: "Lightness".into(),
            alpha: "Alpha".into(),
            hex: "Hex".into(),
            clear: "Clear".into(),
        },
        palette: vec![
            c::PaletteEntry {
                color: Rgba::new(255, 0, 0, 128),
                label: "Translucent red".into(),
            },
            c::PaletteEntry {
                color: Rgba::new(0, 255, 0, 255),
                label: "Green".into(),
            },
        ],
        alpha_policy: AlphaPolicy::AllowAlpha,
        allow_empty: true,
        disabled: false,
        read_only: false,
    }
}
fn set(config: c::Config) -> Op {
    Op::SetColorInput(
        node(),
        Box::new(config),
        Value::Color(Rgba::new(0, 255, 0, 255)),
    )
}
fn apply(cx: &mut AsyncApp, handle: WindowHandle<View>, operations: Vec<Op>) {
    handle
        .update(cx, |v, w, cx| {
            let base = v.session.borrow().tree(v.id).unwrap().revision();
            let result = v
                .session
                .borrow_mut()
                .apply(&Transaction {
                    window: v.id,
                    base,
                    revision: base + 1,
                    operations,
                })
                .unwrap();
            v.update_editors(&result.dirty, w, cx);
            cx.notify();
        })
        .unwrap();
}
fn snapshot(cx: &mut AsyncApp, handle: WindowHandle<View>) -> c::Snapshot {
    handle
        .update(cx, |v, _, cx| {
            v.color_inputs[&node()].state.read(cx).model.snapshot()
        })
        .unwrap()
}
fn events(transport: &Transport) -> Vec<c::Event> {
    transport
        .mailbox
        .lock()
        .unwrap()
        .drain(128)
        .into_iter()
        .filter_map(|e| match e {
            Event::ColorInputEvent(_, _, _, _, e) => Some(e),
            Event::Press(..) | Event::Overloaded(_) => panic!("unexpected route: {e:?}"),
            _ => None,
        })
        .collect()
}
fn focus_channel(cx: &mut AsyncApp, handle: WindowHandle<View>, index: usize) {
    handle
        .update(cx, |v, w, cx| {
            let focus = v.color_inputs[&node()].state.read(cx).channel_focus[index].clone();
            w.focus(&focus, cx);
        })
        .unwrap();
}
fn position(
    cx: &mut AsyncApp,
    handle: WindowHandle<View>,
    index: usize,
    fraction: f32,
) -> Point<Pixels> {
    handle
        .update(cx, |v, _, cx| {
            let bounds = v.color_inputs[&node()].state.read(cx).tracks[index];
            assert!(bounds.size.width > px(0.));
            point(
                bounds.left() + bounds.size.width * fraction,
                bounds.center().y,
            )
        })
        .unwrap()
}
#[cfg(feature = "native-image-tests")]
fn capture(cx: &mut AsyncApp, handle: WindowHandle<View>, name: &str) {
    let Ok(directory) = std::env::var("GPUIO_COLOR_SCREENSHOT_DIR") else {
        return;
    };
    let image = cx
        .update_window(handle.into(), |_, w, cx| {
            w.draw(cx).clear(cx);
            w.render_to_image().unwrap()
        })
        .unwrap();
    std::fs::create_dir_all(&directory).unwrap();
    image
        .save(std::path::Path::new(&directory).join(format!("{name}.png")))
        .unwrap();
}
#[cfg(not(feature = "native-image-tests"))]
fn capture(_: &mut AsyncApp, _: WindowHandle<View>, _: &str) {}
async fn exercise(cx: &mut AsyncApp, handle: WindowHandle<View>, transport: &Transport) {
    apply(
        cx,
        handle,
        vec![
            Op::Create(
                node(),
                Kind::ColorInput,
                "".into(),
                Some(HandlerId::from_parts(0, 1).unwrap()),
            ),
            set(config()),
            Op::SetStyle(
                node(),
                vec![Style::Fields(vec![
                    Field::Background(Fill::Solid(Color::Rgba(0xf4f6faff))),
                    Field::Foreground(Color::Rgba(0x182332ff)),
                ])],
            ),
            Op::SetRoot(Some(node())),
        ],
    );
    frame(cx, handle).await;
    frame(cx, handle).await;
    assert!(matches!(events(transport).as_slice(), [c::Event::Observed(s)] if s.revision==0));
    capture(cx, handle, "color-channels-light");
    apply(
        cx,
        handle,
        vec![Op::SetStyle(
            node(),
            vec![Style::Fields(vec![
                Field::Background(Fill::Solid(Color::Rgba(0x182332ff))),
                Field::Foreground(Color::Rgba(0xe3e9f3ff)),
            ])],
        )],
    );
    frame(cx, handle).await;
    capture(cx, handle, "color-channels-dark");
    assert!(events(transport).is_empty());
    editors_test::exercise(cx, handle, transport).await;
    focus_channel(cx, handle, 0);
    frame(cx, handle).await;
    key(cx, handle, "right");
    frame(cx, handle).await;
    assert_eq!(snapshot(cx, handle).channels.hue_degrees(), 121.);
    assert!(matches!(
        events(transport).as_slice(),
        [c::Event::Committed(c::Source::Keyboard, _)]
    ));
    let baseline = snapshot(cx, handle).committed;
    let start = position(cx, handle, 0, 0.25);
    move_mouse(cx, handle, start, false);
    frame(cx, handle).await;
    mouse(cx, handle, start, true);
    frame(cx, handle).await;
    assert!(
        snapshot(cx, handle).interaction.is_some(),
        "down events: {:?}",
        events(transport)
    );
    let end = position(cx, handle, 0, 0.75);
    move_mouse(cx, handle, end, true);
    frame(cx, handle).await;
    assert!((snapshot(cx, handle).channels.hue_degrees() - 270.).abs() < 0.01);
    events(transport);
    let replacement_handler = HandlerId::from_parts(1, 1).unwrap();
    apply(
        cx,
        handle,
        vec![Op::Bind(node(), Some(replacement_handler))],
    );
    frame(cx, handle).await;
    assert_eq!(snapshot(cx, handle).value, baseline);
    let routed = transport.mailbox.lock().unwrap().drain(128);
    assert!(routed.iter().any(|event| matches!(event,
        Event::ColorInputEvent(_, _, handler, _, c::Event::Cancelled(c::CancelReason::Interrupted, _)) if *handler == replacement_handler)));
    assert!(
        !routed
            .iter()
            .any(|event| matches!(event, Event::Overloaded(_)))
    );
    mouse(cx, handle, end, false);
    move_mouse(cx, handle, start, false);
    frame(cx, handle).await;
    mouse(cx, handle, start, true);
    frame(cx, handle).await;
    move_mouse(cx, handle, end, true);
    frame(cx, handle).await;
    assert!(snapshot(cx, handle).interaction.is_some());
    key(cx, handle, "escape");
    frame(cx, handle).await;
    assert_eq!(snapshot(cx, handle).value, baseline);
    assert!(snapshot(cx, handle).interaction.is_none());
    handle
        .update(cx, |_, w, _| assert!(w.captured_hitbox().is_none()))
        .unwrap();
    assert!(matches!(
        events(transport).last(),
        Some(c::Event::Cancelled(c::CancelReason::Escape, _))
    ));
    mouse(cx, handle, end, false);
    frame(cx, handle).await;
    let start = position(cx, handle, 3, 0.4);
    move_mouse(cx, handle, start, false);
    frame(cx, handle).await;
    mouse(cx, handle, start, true);
    frame(cx, handle).await;
    let end = position(cx, handle, 3, 0.5);
    mouse(cx, handle, end, false);
    frame(cx, handle).await;
    let completed = events(transport);
    assert!(matches!(
        completed.last(),
        Some(c::Event::Committed(c::Source::Pointer, _))
    ));
    assert!(matches!(
        snapshot(cx, handle).value,
        Value::Color(Rgba { alpha: 128, .. })
    ));
    assert!(
        completed
            .windows(2)
            .all(|p| p[0].snapshot().revision < p[1].snapshot().revision)
    );
    handle
        .update(cx, |v, w, cx| {
            let focus = v.color_inputs[&node()].state.read(cx).palette_focus[0].clone();
            w.focus(&focus, cx);
        })
        .unwrap();
    frame(cx, handle).await;
    key(cx, handle, "enter");
    frame(cx, handle).await;
    assert_eq!(
        snapshot(cx, handle).value,
        Value::Color(Rgba::new(255, 0, 0, 128))
    );
    assert!(matches!(
        events(transport).as_slice(),
        [c::Event::Committed(c::Source::Palette, _)]
    ));
    key(cx, handle, "tab");
    frame(cx, handle).await;
    handle
        .update(cx, |v, w, cx| {
            assert!(v.color_inputs[&node()].state.read(cx).palette_focus[1].is_focused(w));
        })
        .unwrap();
    key(cx, handle, "tab");
    frame(cx, handle).await;
    handle
        .update(cx, |v, w, cx| {
            assert!(
                v.color_inputs[&node()]
                    .state
                    .read(cx)
                    .clear_focus
                    .is_focused(w)
            );
        })
        .unwrap();
    key(cx, handle, "enter");
    frame(cx, handle).await;
    assert_eq!(snapshot(cx, handle).value, Value::Empty);
    assert!(matches!(
        events(transport).as_slice(),
        [c::Event::Committed(c::Source::Clear, _)]
    ));
    handle
        .update(cx, |v, w, cx| {
            let focus = v.color_inputs[&node()].state.read(cx).palette_focus[0].clone();
            w.focus(&focus, cx);
        })
        .unwrap();
    frame(cx, handle).await;
    key(cx, handle, "enter");
    frame(cx, handle).await;
    events(transport);
    let start = position(cx, handle, 0, 0.374);
    move_mouse(cx, handle, start, false);
    frame(cx, handle).await;
    mouse(cx, handle, start, true);
    frame(cx, handle).await;
    assert!((snapshot(cx, handle).channels.hue_degrees() - 134.64).abs() < 0.001);
    assert!(snapshot(cx, handle).interaction.is_some());
    events(transport);
    let mut restricted = config();
    restricted.alpha_policy = AlphaPolicy::OpaqueOnly;
    apply(cx, handle, vec![set(restricted.clone())]);
    frame(cx, handle).await;
    assert!(!snapshot(cx, handle).value_allowed);
    assert!(matches!(
        events(transport).as_slice(),
        [
            c::Event::Cancelled(c::CancelReason::ConfigurationChanged, _),
            c::Event::Observed(_)
        ]
    ));
    handle
        .update(cx, |_, w, _| assert!(w.captured_hitbox().is_none()))
        .unwrap();
    mouse(cx, handle, start, false);
    frame(cx, handle).await;
    assert!(events(transport).is_empty());
    focus_channel(cx, handle, 3);
    frame(cx, handle).await;
    key(cx, handle, "home");
    frame(cx, handle).await;
    assert_eq!(
        snapshot(cx, handle).value,
        Value::Color(Rgba::new(255, 0, 0, 128))
    );
    assert!(events(transport).is_empty());
    restricted.disabled = true;
    apply(cx, handle, vec![set(restricted)]);
    frame(cx, handle).await;
    handle
        .update(cx, |v, w, cx| {
            assert!(!v.color_inputs[&node()].focused(w, cx))
        })
        .unwrap();
    let disabled = snapshot(cx, handle);
    events(transport);
    let point = position(cx, handle, 0, 0.25);
    move_mouse(cx, handle, point, false);
    frame(cx, handle).await;
    mouse(cx, handle, point, true);
    mouse(cx, handle, point, false);
    frame(cx, handle).await;
    assert_eq!(snapshot(cx, handle), disabled);
    assert!(events(transport).is_empty());
    handle
        .update(cx, |v, w, cx| {
            assert!(!v.color_inputs[&node()].focused(w, cx))
        })
        .unwrap();
    #[cfg(feature = "native-image-tests")]
    appearance::exercise(cx, handle, transport).await;
    lifecycle::exercise(cx, handle, transport).await;
    let (weak, fields) = handle
        .update(cx, |v, _, cx| {
            let input = &v.color_inputs[&node()].state;
            (
                input.downgrade(),
                input
                    .read(cx)
                    .editors
                    .fields
                    .iter()
                    .map(|field| field.state.downgrade())
                    .collect::<Vec<_>>(),
            )
        })
        .unwrap();
    apply(cx, handle, vec![Op::SetRoot(None), Op::Remove(node())]);
    frame(cx, handle).await;
    frame(cx, handle).await;
    handle
        .update(cx, |v, _, _| {
            assert!(v.color_inputs.is_empty());
            assert!(v.session.borrow().accepts_input(v.id));
        })
        .unwrap();
    assert!(weak.upgrade().is_none());
    assert!(fields.iter().all(|field| field.upgrade().is_none()));
    // A required Started/Preview pair must fault atomically when only one
    // input slot remains, and native fields must stop accepting further edits.
    events(transport);
    let next = NodeId::from_parts(0, 2).unwrap();
    let next_handler = HandlerId::from_parts(2, 1).unwrap();
    apply(
        cx,
        handle,
        vec![
            Op::Create(next, Kind::ColorInput, "".into(), Some(next_handler)),
            Op::SetColorInput(
                next,
                Box::new(config()),
                Value::Color(Rgba::new(0, 255, 0, 255)),
            ),
            Op::SetRoot(Some(next)),
        ],
    );
    frame(cx, handle).await;
    events(transport);
    let field = handle
        .update(cx, |v, w, cx| {
            let input = v.color_inputs[&next].state.read(cx);
            let observed = input.model.snapshot();
            let revision = v.session.borrow().tree(v.id).unwrap().revision();
            for _ in 0..crate::mailbox::MAX_INPUT_EVENTS - 1 {
                assert!(transport.input(Event::ColorInputEvent(
                    v.id,
                    next,
                    next_handler,
                    revision,
                    c::Event::Observed(observed.clone())
                )));
            }
            let field = input.editors.fields[0].state.clone();
            w.focus(&field.read(cx).focus_handle(cx), cx);
            field
        })
        .unwrap();
    frame(cx, handle).await;
    handle
        .update(cx, |_, w, cx| {
            field.update(cx, |s, cx| {
                let length = s.value().encode_utf16().count();
                s.replace_text_in_range(Some(0..length), "#112233", w, cx);
            })
        })
        .unwrap();
    frame(cx, handle).await;
    let drained = transport.mailbox.lock().unwrap().drain(256);
    assert_eq!(
        drained
            .iter()
            .filter(|e| matches!(e, Event::Overloaded(_)))
            .count(),
        1
    );
    assert!(!drained.iter().any(|e| matches!(
        e,
        Event::ColorInputEvent(_, _, _, _, c::Event::Started(_) | c::Event::Preview(_))
    )));
    handle
        .update(cx, |v, w, cx| {
            assert!(!v.session.borrow().accepts_input(v.id));
            assert!(!field.read(cx).is_editable());
            for command in [
                c::Command::ReadSnapshot,
                c::Command::Reset { if_revision: None },
            ] {
                assert_eq!(
                    v.color_inputs[&next].command(&command, w, cx),
                    c::Response::Failed(c::Error::NativeFailure)
                );
            }
        })
        .unwrap();
    eprintln!(
        "GPUIO_COLOR_PRESSURE_OK: required text pair remains atomic and native fields disable after overload"
    );
    eprintln!(
        "GPUIO_COLOR_CHANNELS_OK: native keyboard, captured pointer preview/commit/Escape, palette activation, alpha/history/disabled gates and owner disposal"
    );
}
fn open_test_window(cx: &mut App, transport: &Arc<Transport>) -> WindowHandle<View> {
    let id = WindowId::from_parts(0, 1).unwrap();
    let session = Rc::new(RefCell::new(Session::default()));
    session.borrow_mut().hello(VERSION, CAPABILITIES).unwrap();
    session
        .borrow_mut()
        .open(1, id, "GPUIO Color input test", 340., 560.)
        .unwrap();
    cx.open_window(
        WindowOptions {
            inactive_frame_interval: None,
            window_bounds: Some(WindowBounds::Windowed(Bounds::centered(
                None,
                size(px(340.), px(560.)),
                cx,
            ))),
            ..Default::default()
        },
        |_, cx| cx.new(|_| View::new(id, session.clone(), transport.clone())),
    )
    .unwrap()
}

pub(crate) fn run() {
    let failure = Rc::new(RefCell::new(None));
    let task_failure = failure.clone();
    let mut fds = [0; 2];
    assert_eq!(unsafe { libc::pipe(fds.as_mut_ptr()) }, 0);
    let _reader = unsafe { OwnedFd::from_raw_fd(fds[0]) };
    let writer = unsafe { OwnedFd::from_raw_fd(fds[1]) };
    let transport = Arc::new(Transport::new(writer.as_raw_fd()).unwrap());
    eprintln!("GPUIO_COLOR_STARTING");
    gpui_platform::application().run(move |cx| {
        eprintln!("GPUIO_COLOR_LAUNCHED");
        gpui_base::init(cx);
        cx.set_quit_mode(QuitMode::Explicit);
        let handle = open_test_window(cx, &transport);
        cx.activate(true);
        cx.spawn(async move |cx| {
            eprintln!("GPUIO_COLOR_EXERCISE");
            let result = super::super::native_test::protect(async {
                exercise(cx, handle, &transport).await;
                handle.update(cx, |_, w, _| w.remove_window()).unwrap();
                let next = cx.update(|cx| open_test_window(cx, &transport));
                workload::exercise(cx, next, &transport).await;
            })
            .await;
            *task_failure.borrow_mut() = result.err();
            cx.update(stop_application);
        })
        .detach();
    });
    if let Some(error) = failure.borrow_mut().take() {
        std::panic::resume_unwind(error);
    }
}
