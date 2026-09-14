//! Background production-window checks for decorative button icon slots.
use super::*;
use crate::host::{editor_test, native_test};
fn id(slot: i64) -> NodeId {
    NodeId::from_parts(slot, 1).unwrap()
}
fn style() -> Vec<Style> {
    vec![
        Style::Fields(vec![
            Field::Display(1),
            Field::Direction(0),
            Field::AlignItems(4),
            Field::Width(Length::Percent(100.)),
            Field::Height(Length::Px(64.)),
            Field::Foreground(Color::Rgba(0x00cc44ff)),
        ]),
        Style::Padding(8.),
        Style::Gap(8.),
    ]
}
fn icon_style() -> Vec<Style> {
    vec![Style::Fields(vec![
        Field::Width(Length::Px(24.)),
        Field::Height(Length::Px(24.)),
        Field::Shrink(0.),
    ])]
}
fn command(label: &str, generation: i64, enabled: bool) -> CommandConfig {
    CommandConfig {
        id: "send".into(),
        generation,
        label: label.into(),
        enabled,
        checked: None,
        shortcuts: vec![],
        target: CommandTarget::Callback,
    }
}
fn drain(transport: &Transport) -> Vec<Event> {
    transport.mailbox.lock().unwrap().drain(128)
}
fn press_count(events: &[Event]) -> usize {
    events
        .iter()
        .filter(|event| matches!(event, Event::Press(_, node, _, _) if *node == id(3)))
        .count()
}
fn center(
    cx: &mut gpui::AsyncApp,
    window: WindowHandle<View>,
    node: NodeId,
) -> gpui::Point<gpui::Pixels> {
    window
        .update(cx, |view, _, _| view.probes.borrow()[&node].bounds.center())
        .unwrap()
}
async fn settled(cx: &mut gpui::AsyncApp, window: WindowHandle<View>) {
    for _ in 0..200 {
        if window
            .update(cx, |view, _, _| {
                [id(6), id(7)].into_iter().all(|id| {
                    let binding = view.images[&id].binding.borrow();
                    binding.pending.is_none()
                        && binding.resize_error.is_none()
                        && matches!(binding.rendered.size, asset_svg::Size::Exact(_))
                        && binding.rendered.tint == Some(0x00cc44ff)
                })
            })
            .unwrap()
        {
            pause(cx).await;
            pause(cx).await;
            return;
        }
        pause(cx).await;
    }
    panic!("button icons did not settle");
}
async fn click(cx: &mut gpui::AsyncApp, window: WindowHandle<View>, node: NodeId) {
    let point = center(cx, window, node);
    native_test::move_mouse(cx, window, point, false);
    native_test::mouse(cx, window, point, true);
    pause(cx).await;
    native_test::mouse(cx, window, point, false);
    pause(cx).await;
}
pub(super) async fn exercise(
    cx: &mut gpui::AsyncApp,
    window: WindowHandle<View>,
    session: &Rc<RefCell<Session>>,
    transport: &Transport,
) {
    window
        .update(cx, |_, window, _| window.resize(size(px(320.), px(96.))))
        .unwrap();
    for _ in 0..200 {
        if window
            .update(cx, |_, window, _| window.viewport_size().width == px(320.))
            .unwrap()
        {
            break;
        }
        pause(cx).await;
    }
    assert_eq!(
        window
            .update(cx, |_, window, _| window.viewport_size().width)
            .unwrap(),
        px(320.)
    );
    let source = {
        let bytes = br#"<svg xmlns="http://www.w3.org/2000/svg" width="24" height="24"><rect width="24" height="24" fill="red"/></svg>"#;
        let mut session = session.borrow_mut();
        let store = session.assets().unwrap();
        let source = store.begin(Format::Svg, bytes.len()).unwrap();
        store.append(source, 0, bytes).unwrap();
        store.finish(source).unwrap();
        source
    };
    let mut operations = vec![
        Op::Create(id(3), Kind::Button, "Send".into(), Some(handler(8))),
        Op::SetControl(id(3), Control::Button(false)),
        Op::SetStyle(id(3), style()),
        Op::Create(id(4), Kind::Container, "".into(), None),
        Op::Create(id(5), Kind::Container, "".into(), None),
    ];
    for node in [id(6), id(7)] {
        operations.extend([
            Op::Create(node, Kind::Icon, "".into(), None),
            Op::SetImage(
                node,
                ImageConfig {
                    source: ImageSource::Reference(source),
                    fit: ImageFit::Contain,
                    label: None,
                },
            ),
            Op::SetStyle(node, icon_style()),
        ]);
    }
    operations.extend([
        Op::Splice(id(4), 0, 0, vec![id(6)]),
        Op::Splice(id(5), 0, 0, vec![id(7)]),
        Op::Splice(id(3), 0, 0, vec![id(4), id(5)]),
        Op::SetRoot(Some(id(3))),
    ]);
    apply(cx, window, operations);
    session
        .borrow_mut()
        .assets()
        .unwrap()
        .release(source)
        .unwrap();
    settled(cx, window).await;
    let leading = center(cx, window, id(6));
    let trailing = center(cx, window, id(7));
    assert!(
        trailing.x > leading.x + px(24.),
        "label separates icon slots"
    );
    window
        .update(cx, |_, window, _| {
            let image = window.render_to_image().unwrap();
            for point in [leading, trailing] {
                let scale = window.scale_factor();
                assert_eq!(
                    image
                        .get_pixel(
                            (f32::from(point.x) * scale) as u32,
                            (f32::from(point.y) * scale) as u32
                        )
                        .0,
                    [0, 204, 68, 255]
                );
            }
        })
        .unwrap();
    #[cfg(target_os = "macos")]
    assert_eq!(
        crate::host::control_test::accessible_role(cx, window, "Send").as_deref(),
        Some("AXButton")
    );
    drain(transport);
    click(cx, window, id(6)).await;
    assert_eq!(
        press_count(&drain(transport)),
        1,
        "clicking leading icon activates button once"
    );
    click(cx, window, id(7)).await;
    assert_eq!(
        press_count(&drain(transport)),
        1,
        "clicking trailing icon activates button once"
    );
    window
        .update(cx, |view, window, cx| {
            window.focus(&view.buttons[&id(3)].focus, cx)
        })
        .unwrap();
    pause(cx).await;
    editor_test::key(cx, window, "enter");
    pause(cx).await;
    assert_eq!(
        press_count(&drain(transport)),
        1,
        "keyboard has one button action"
    );
    apply(
        cx,
        window,
        vec![Op::SetControl(id(3), Control::Button(true))],
    );
    pause(cx).await;
    pause(cx).await;
    click(cx, window, id(6)).await;
    editor_test::key(cx, window, "enter");
    pause(cx).await;
    assert_eq!(
        press_count(&drain(transport)),
        0,
        "disabled decoration cannot activate"
    );
    // Move the mounted slots under a command button: native labels and registry
    // updates must not require copying label text into an OCaml-owned child.
    apply(
        cx,
        window,
        vec![
            Op::Splice(id(3), 0, 2, vec![]),
            Op::SetRoot(None),
            Op::Remove(id(3)),
            Op::Create(id(8), Kind::CommandScope, "".into(), Some(handler(9))),
            Op::SetCommands(id(8), vec![command("Run", 1, true)]),
            Op::Create(id(9), Kind::CommandButton, "".into(), None),
            Op::SetCommandRef(id(9), "send".into()),
            Op::SetStyle(id(9), style()),
            Op::Splice(id(9), 0, 0, vec![id(4), id(5)]),
            Op::Splice(id(8), 0, 0, vec![id(9)]),
            Op::SetRoot(Some(id(8))),
        ],
    );
    settled(cx, window).await;
    let old_trailing = center(cx, window, id(7)).x;
    apply(
        cx,
        window,
        vec![Op::SetCommands(id(8), vec![command("Run again", 2, true)])],
    );
    pause(cx).await;
    pause(cx).await;
    assert!(
        center(cx, window, id(7)).x > old_trailing,
        "new registry label is laid out between slots"
    );
    #[cfg(target_os = "macos")]
    assert_eq!(
        crate::host::control_test::accessible_role(cx, window, "Run again").as_deref(),
        Some("AXButton")
    );
    drain(transport);
    click(cx, window, id(7)).await;
    let invoked: Vec<_> = drain(transport)
        .into_iter()
        .filter(|event| matches!(event, Event::CommandInvoked(..)))
        .collect();
    assert_eq!(invoked.len(), 1);
    assert!(
        matches!(&invoked[0], Event::CommandInvoked(_, scope, _, _, command, 2, CommandSource::Button(button)) if *scope == id(8) && command == "send" && *button == id(9))
    );
    apply(
        cx,
        window,
        vec![Op::SetCommands(id(8), vec![command("Run again", 3, false)])],
    );
    pause(cx).await;
    pause(cx).await;
    click(cx, window, id(6)).await;
    assert!(
        !drain(transport)
            .iter()
            .any(|event| matches!(event, Event::CommandInvoked(..)))
    );
    let mut icon_only_style = style();
    icon_only_style.push(Style::Fields(vec![Field::AccessibleName(
        "Icon action".into(),
    )]));
    apply(
        cx,
        window,
        vec![
            Op::Splice(id(5), 0, 1, vec![]),
            Op::Remove(id(7)),
            Op::Splice(id(9), 0, 2, vec![]),
            Op::SetRoot(None),
            Op::Remove(id(9)),
            Op::Remove(id(8)),
            Op::Create(id(10), Kind::Button, "".into(), Some(handler(10))),
            Op::SetControl(id(10), Control::Button(false)),
            Op::SetStyle(id(10), icon_only_style),
            Op::Splice(id(10), 0, 0, vec![id(4), id(5)]),
            Op::SetRoot(Some(id(10))),
        ],
    );
    pause(cx).await;
    pause(cx).await;
    #[cfg(target_os = "macos")]
    assert_eq!(
        crate::host::control_test::accessible_role(cx, window, "Icon action").as_deref(),
        Some("AXButton")
    );
    drain(transport);
    click(cx, window, id(6)).await;
    assert_eq!(
        drain(transport)
            .iter()
            .filter(|event| matches!(event, Event::Press(_, node, _, _) if *node == id(10)))
            .count(),
        1
    );
    apply(
        cx,
        window,
        vec![
            Op::SetRoot(None),
            Op::Remove(id(6)),
            Op::Remove(id(4)),
            Op::Remove(id(5)),
            Op::Remove(id(10)),
        ],
    );
    assert_eq!(session.borrow_mut().assets().unwrap().stats().retired, 0);
    eprintln!(
        "GPUIO_BUTTON_ICONS_OK: GPU pixels, leading/trailing clicks, keyboard, disabled, AXButton, live command labels and disposal"
    );
}
