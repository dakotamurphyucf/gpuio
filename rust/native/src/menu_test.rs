//! Real menu key/pointer/accessibility and native macOS menu-bar routing.
use super::*;
fn handler(slot: i64) -> gpuio_protocol::HandlerId {
    gpuio_protocol::HandlerId::from_parts(slot, 1).unwrap()
}
fn definition() -> MenuDefinition {
    MenuDefinition {
        label: "Actions".into(),
        disabled: false,
        items: vec![
            MenuItem::Command("run".into()),
            MenuItem::Command("disabled".into()),
            MenuItem::Separator,
            MenuItem::Submenu(MenuDefinition {
                label: "More".into(),
                disabled: false,
                items: vec![
                    MenuItem::Command("deep".into()),
                    MenuItem::Command("copy".into()),
                ],
            }),
        ],
    }
}
fn config(presentation: MenuPresentation) -> MenuConfig {
    MenuConfig {
        presentation,
        menus: vec![definition()],
    }
}
fn commands() -> Vec<CommandConfig> {
    ["run", "disabled", "deep", "copy"]
        .iter()
        .enumerate()
        .map(|(index, id)| CommandConfig {
            id: (*id).into(),
            label: format!("Menu {id}"),
            generation: index as i64 + 1,
            enabled: *id != "disabled",
            checked: (*id == "run").then_some(true),
            shortcuts: vec![],
            target: if *id == "copy" {
                CommandTarget::Native(NativeCommand::Copy)
            } else {
                CommandTarget::Callback
            },
        })
        .collect()
}
fn emitted(transport: &Transport) -> Vec<(String, CommandSource)> {
    transport
        .mailbox
        .lock()
        .unwrap()
        .drain(128)
        .into_iter()
        .filter_map(|event| match event {
            Event::CommandInvoked(_, _, _, _, id, _, source) => Some((id, source)),
            _ => None,
        })
        .collect()
}
fn focus_menu(cx: &mut gpui::AsyncApp, handle: WindowHandle<View>, slot: i64) {
    handle
        .update(cx, |view, window, cx| {
            window.focus(&view.menus[&node(slot)].borrow().focus, cx)
        })
        .unwrap();
}
fn state(
    cx: &mut gpui::AsyncApp,
    handle: WindowHandle<View>,
    slot: i64,
) -> (Vec<usize>, Vec<Option<usize>>) {
    handle
        .update(cx, |view, _, _| {
            view.menus[&node(slot)].borrow().test_state()
        })
        .unwrap()
}
fn panel(
    cx: &mut gpui::AsyncApp,
    handle: WindowHandle<View>,
    slot: i64,
    depth: usize,
) -> Bounds<gpui::Pixels> {
    handle
        .update(cx, |view, _, _| {
            view.menus[&node(slot)].borrow().test_panel(depth)
        })
        .unwrap()
}
#[cfg(target_os = "macos")]
fn native_menu_press(name: &str, index: usize) {
    use objc2::{
        class, msg_send,
        runtime::{AnyObject, Bool},
    };
    use objc2_foundation::NSString;
    unsafe {
        let app: *mut AnyObject = msg_send![class!(NSApplication), sharedApplication];
        let main: *mut AnyObject = msg_send![app, mainMenu];
        assert!(!main.is_null());
        let count: usize = msg_send![main, numberOfItems];
        for item_index in 0..count {
            let item: *mut AnyObject = msg_send![main, itemAtIndex: item_index];
            let title: *mut NSString = msg_send![item, title];
            if title
                .as_ref()
                .is_some_and(|title| title.to_string() == name)
            {
                let submenu: *mut AnyObject = msg_send![item, submenu];
                assert!(!submenu.is_null());
                let item: *mut AnyObject = msg_send![submenu, itemAtIndex: index];
                let enabled: Bool = msg_send![item, isEnabled];
                assert!(enabled.as_bool());
                let _: () = msg_send![submenu, performActionForItemAtIndex: index];
                return;
            }
        }
        panic!("native NSMenu missing {name}");
    }
}
#[cfg(target_os = "macos")]
fn platform_action(cx: &mut gpui::AsyncApp, label: &str) -> Box<dyn gpui::Action> {
    cx.update(|cx| {
        let menus = cx.get_menus().unwrap();
        let menu = menus.iter().find(|menu| menu.name == "Actions").unwrap();
        let gpui::OwnedMenuItem::Action {
            name,
            action,
            disabled,
            ..
        } = &menu.items[0]
        else {
            panic!("platform command item")
        };
        assert_eq!(name.as_str(), label);
        assert!(!disabled);
        action.boxed_clone()
    })
}
#[cfg(target_os = "macos")]
fn routed(transport: &Transport) -> Vec<(WindowId, NodeId, i64, CommandSource)> {
    transport
        .mailbox
        .lock()
        .unwrap()
        .drain(128)
        .into_iter()
        .filter_map(|event| match event {
            Event::CommandInvoked(window, scope, _, _, id, generation, source) => {
                assert_eq!(id, "run");
                Some((window, scope, generation, source))
            }
            _ => None,
        })
        .collect()
}
#[cfg(target_os = "macos")]
async fn platform_contexts(
    cx: &mut gpui::AsyncApp,
    primary: WindowHandle<View>,
    transport: &Transport,
) {
    let mut scoped = commands()[0].clone();
    scoped.label = "Scoped run".into();
    scoped.generation = 3000;
    apply(
        cx,
        primary,
        vec![
            Op::Create(node(45), Kind::CommandScope, "".into(), Some(handler(45))),
            Op::SetCommands(node(45), vec![scoped]),
            Op::Create(node(46), Kind::CommandButton, "".into(), None),
            Op::SetCommandRef(node(46), "run".into()),
            Op::Splice(node(45), 0, 0, vec![node(46)]),
            Op::Splice(node(0), 7, 0, vec![node(45)]),
        ],
    );
    frame(cx, primary).await;
    primary
        .update(cx, |view, window, cx| {
            window.focus(&view.buttons[&node(46)].focus, cx)
        })
        .unwrap();
    frame(cx, primary).await;
    let scoped_action = platform_action(cx, "Scoped run");
    emitted(transport);
    let primary_id = WindowId::from_parts(0, 1).unwrap();
    native_menu_press("Actions", 0);
    frame(cx, primary).await;
    assert_eq!(
        routed(transport),
        [(primary_id, node(45), 3000, CommandSource::Menu(node(42)))]
    );
    primary
        .update(cx, |view, window, cx| {
            window.focus(&view.editors[&node(4)].focus_handle(cx), cx)
        })
        .unwrap();
    frame(cx, primary).await;
    cx.update(|cx| cx.dispatch_action(scoped_action.as_ref()));
    frame(cx, primary).await;
    assert!(
        emitted(transport).is_empty(),
        "old action cannot target a scope that lost focus"
    );
    let primary_action = platform_action(cx, "Menu run");
    native_menu_press("Actions", 0);
    frame(cx, primary).await;
    assert_eq!(
        routed(transport),
        [(primary_id, node(38), 2001, CommandSource::Menu(node(42)))]
    );

    // Display/visibility changes must clear native menus and reject queued actions.
    apply(
        cx,
        primary,
        vec![Op::SetStyle(
            node(42),
            vec![Style::Fields(vec![Field::Visibility(1)])],
        )],
    );
    frame(cx, primary).await;
    cx.update(|cx| {
        assert!(cx.get_menus().unwrap().is_empty());
        cx.dispatch_action(primary_action.as_ref());
    });
    frame(cx, primary).await;
    assert!(emitted(transport).is_empty());
    apply(cx, primary, vec![Op::SetStyle(node(42), vec![])]);
    frame(cx, primary).await;
    let _ = platform_action(cx, "Menu run");

    let (session, native_transport) = primary
        .update(cx, |view, _, _| {
            (view.session.clone(), view.transport.clone())
        })
        .unwrap();
    let id = [1, 2]
        .into_iter()
        .map(|generation| WindowId::from_parts(1, generation).unwrap())
        .find(|id| {
            session
                .borrow()
                .validate_open(*id, "Menu isolation", 400., 280.)
                .is_ok()
        })
        .unwrap();
    let secondary = cx.update(|cx| {
        cx.open_window(
            WindowOptions {
                focus: false,
                inactive_frame_interval: Some(std::time::Duration::from_millis(16)),
                ..Default::default()
            },
            |_, cx| cx.new(|_| View::new(id, session.clone(), native_transport)),
        )
        .unwrap()
    });
    session
        .borrow_mut()
        .open(3, id, "Menu isolation", 400., 280.)
        .unwrap();
    let mut other = commands();
    other[0].label = "Other run".into();
    for command in &mut other {
        command.generation += 3999;
    }
    apply(
        cx,
        secondary,
        vec![
            Op::Create(node(0), Kind::CommandScope, "".into(), Some(handler(0))),
            Op::SetCommands(node(0), other),
            Op::Create(node(1), Kind::Menu, "".into(), None),
            Op::SetMenu(node(1), config(MenuPresentation::PlatformBar)),
            Op::Create(node(2), Kind::CommandButton, "".into(), None),
            Op::SetCommandRef(node(2), "run".into()),
            Op::Splice(node(0), 0, 0, vec![node(1), node(2)]),
            Op::SetRoot(Some(node(0))),
        ],
    );
    frame(cx, secondary).await;
    assert!(primary.update(cx, |_, w, _| w.is_window_active()).unwrap());
    assert!(
        !secondary
            .update(cx, |_, w, _| w.is_window_active())
            .unwrap()
    );
    let _ = platform_action(cx, "Menu run");
    secondary
        .update(cx, |view, window, cx| {
            window.activate_window();
            window.focus(&view.buttons[&node(2)].focus, cx);
        })
        .unwrap();
    frame(cx, secondary).await;
    let secondary_action = platform_action(cx, "Other run");
    emitted(transport);
    cx.update(|cx| cx.dispatch_action(primary_action.as_ref()));
    frame(cx, secondary).await;
    assert!(
        emitted(transport).is_empty(),
        "primary action rejected in another active window"
    );
    native_menu_press("Actions", 0);
    frame(cx, secondary).await;
    assert_eq!(
        routed(transport),
        [(id, node(0), 4000, CommandSource::Menu(node(1)))]
    );
    secondary
        .update(cx, |_, window, _| window.remove_window())
        .unwrap();
    session.borrow_mut().close(id).unwrap();
    primary
        .update(cx, |_, window, _| window.activate_window())
        .unwrap();
    frame(cx, primary).await;
    assert!(primary.update(cx, |_, w, _| w.is_window_active()).unwrap());
    let _ = platform_action(cx, "Menu run");
    cx.update(|cx| cx.dispatch_action(secondary_action.as_ref()));
    frame(cx, primary).await;
    assert!(
        emitted(transport).is_empty(),
        "closed window action rejected"
    );
    native_menu_press("Actions", 0);
    frame(cx, primary).await;
    assert_eq!(
        routed(transport),
        [(primary_id, node(38), 2001, CommandSource::Menu(node(42)))]
    );
    apply(
        cx,
        primary,
        vec![
            Op::Splice(node(0), 7, 1, vec![]),
            Op::Remove(node(46)),
            Op::Remove(node(45)),
        ],
    );
    frame(cx, primary).await;
    println!(
        "GPUIO_MENUS_WINDOWS_OK: focused scope routing, hidden menu rejection, active-window ownership, stale actions and surviving-window restoration"
    );
}
async fn menu_availability(
    cx: &mut gpui::AsyncApp,
    handle: WindowHandle<View>,
    transport: &Transport,
) {
    let style = handle
        .update(cx, |view, _, _| {
            view.session
                .borrow()
                .tree(view.id)
                .unwrap()
                .get(node(39))
                .unwrap()
                .style
                .to_vec()
        })
        .unwrap();
    for field in [Field::Visibility(1), Field::Display(3)] {
        focus_menu(cx, handle, 39);
        key(cx, handle, "down");
        frame(cx, handle).await;
        assert!(!state(cx, handle, 39).0.is_empty());
        let mut hidden = style.clone();
        hidden.push(Style::Fields(vec![field]));
        apply(cx, handle, vec![Op::SetStyle(node(39), hidden)]);
        frame(cx, handle).await;
        assert!(
            state(cx, handle, 39).0.is_empty(),
            "hiding a trigger closes its detached panels"
        );
        assert!(
            !handle
                .update(cx, |view, window, _| view.menus[&node(39)]
                    .borrow()
                    .focus
                    .is_focused(window))
                .unwrap(),
            "hidden menu cannot retain focus"
        );
        emitted(transport);
        key(cx, handle, "enter");
        frame(cx, handle).await;
        assert!(emitted(transport).is_empty());
        apply(cx, handle, vec![Op::SetStyle(node(39), style.clone())]);
        frame(cx, handle).await;
    }
    focus_menu(cx, handle, 39);
    key(cx, handle, "down");
    frame(cx, handle).await;
    let mut disabled = config(MenuPresentation::Button);
    disabled.menus[0].disabled = true;
    apply(cx, handle, vec![Op::SetMenu(node(39), disabled)]);
    frame(cx, handle).await;
    assert!(state(cx, handle, 39).0.is_empty());
    assert!(
        !handle
            .update(cx, |view, window, _| view.menus[&node(39)]
                .borrow()
                .focus
                .is_focused(window))
            .unwrap()
    );
    apply(
        cx,
        handle,
        vec![Op::SetMenu(node(39), config(MenuPresentation::Button))],
    );
    frame(cx, handle).await;
    #[cfg(not(target_os = "macos"))]
    {
        focus_menu(cx, handle, 42);
        key(cx, handle, "down");
        frame(cx, handle).await;
        emitted(transport);
        key(cx, handle, "enter");
        frame(cx, handle).await;
        assert_eq!(
            emitted(transport),
            [("run".into(), CommandSource::Menu(node(42)))]
        );
    }
    println!(
        "GPUIO_MENU_AVAILABILITY_OK: hidden/disabled triggers release panels and focus without activation"
    );
}
async fn context_copy(cx: &mut gpui::AsyncApp, handle: WindowHandle<View>, transport: &Transport) {
    handle
        .update(cx, |view, window, cx| {
            let result = view.editors.get_mut(&node(4)).unwrap().command(
                &EditorCommand::Replace(
                    "Context copy".into(),
                    EditorSelectionPolicy::Select(EditorSelection {
                        anchor: 0,
                        head: 12,
                    }),
                    EditorUndoPolicy::Reset,
                    None,
                ),
                window,
                cx,
            );
            assert!(matches!(result, EditorResult::Applied(_)));
            window.focus(&view.editors[&node(4)].focus_handle(cx), cx);
        })
        .unwrap();
    frame(cx, handle).await;
    let bounds = handle
        .update(cx, |view, _, _| view.probes.borrow()[&node(4)].bounds)
        .unwrap();
    let position = gpui::point(bounds.left() + px(8.), bounds.center().y);
    super::super::native_test::move_mouse(cx, handle, position, false);
    cx.update_window(handle.into(), |_, window, cx| {
        window.dispatch_event(
            gpui::PlatformInput::MouseDown(gpui::MouseDownEvent {
                position,
                button: gpui::MouseButton::Right,
                modifiers: Default::default(),
                click_count: 1,
                first_mouse: false,
            }),
            cx,
        );
        window.dispatch_event(
            gpui::PlatformInput::MouseUp(gpui::MouseUpEvent {
                position,
                button: gpui::MouseButton::Right,
                modifiers: Default::default(),
                click_count: 1,
            }),
            cx,
        );
    })
    .unwrap();
    frame(cx, handle).await;
    assert_eq!(state(cx, handle, 40).0, [0]);
    let popup = panel(cx, handle, 40, 0);
    assert!(popup.left() >= px(0.) && popup.right() <= px(400.));
    emitted(transport);
    key(cx, handle, "end");
    key(cx, handle, "right");
    frame(cx, handle).await;
    key(cx, handle, "down");
    frame(cx, handle).await;
    assert_eq!(state(cx, handle, 40).1.last(), Some(&Some(1)));
    key(cx, handle, "enter");
    frame(cx, handle).await;
    assert!(
        emitted(transport).is_empty(),
        "native edit menu actions stay on GPUI thread"
    );
    assert!(focused(cx, handle, node(4)));
    handle
        .update(cx, |_, _, cx| {
            assert_eq!(
                cx.read_from_clipboard()
                    .and_then(|item| item.text())
                    .as_deref(),
                Some("Context copy")
            )
        })
        .unwrap();
    println!(
        "GPUIO_CONTEXT_MENU_NATIVE_OK: right-click placement, native selection and Copy with editing focus restoration"
    );
}
async fn large_menu(cx: &mut gpui::AsyncApp, handle: WindowHandle<View>, transport: &Transport) {
    let mut registry = commands();
    for command in &mut registry {
        command.generation += 100;
    }
    registry.extend((0..1000).map(|index| CommandConfig {
        id: format!("long-{index}"),
        label: format!("Command {index:04}"),
        generation: index + 200,
        enabled: true,
        checked: None,
        shortcuts: vec![],
        target: CommandTarget::Callback,
    }));
    let large = MenuConfig {
        presentation: MenuPresentation::Button,
        menus: vec![MenuDefinition {
            label: "Long menu".into(),
            disabled: false,
            items: (0..1000)
                .map(|index| MenuItem::Command(format!("long-{index}")))
                .collect(),
        }],
    };
    apply(
        cx,
        handle,
        vec![
            Op::SetCommands(node(38), registry),
            Op::SetMenu(node(39), large),
        ],
    );
    frame(cx, handle).await;
    focus_menu(cx, handle, 39);
    key(cx, handle, "down");
    frame(cx, handle).await;
    let point = panel(cx, handle, 39, 0).center();
    super::super::native_test::move_mouse(cx, handle, point, false);
    frame(cx, handle).await;
    handle
        .update(cx, |_, window, cx| {
            window.dispatch_event(
                gpui::PlatformInput::ScrollWheel(gpui::ScrollWheelEvent {
                    position: point,
                    delta: gpui::ScrollDelta::Pixels(gpui::point(px(0.), px(-280.))),
                    modifiers: Default::default(),
                    touch_phase: gpui::TouchPhase::Moved,
                }),
                cx,
            );
        })
        .unwrap();
    frame(cx, handle).await;
    let (top, rendered) = handle
        .update(cx, |view, _, _| {
            view.menus[&node(39)].borrow().test_scroll(0)
        })
        .unwrap();
    assert!(
        top > 0,
        "wheel changes menu viewport without snapping back to highlight: {}",
        handle
            .update(cx, |view, _, _| view.menus[&node(39)]
                .borrow()
                .test_scroll_diagnostics(0))
            .unwrap()
    );
    assert!(rendered < 32, "only visible uniform menu rows are built");
    key(cx, handle, "end");
    frame(cx, handle).await;
    let (top, rendered) = handle
        .update(cx, |view, _, _| {
            view.menus[&node(39)].borrow().test_scroll(0)
        })
        .unwrap();
    assert!(top >= 994 && rendered < 32);
    key(cx, handle, "enter");
    frame(cx, handle).await;
    assert_eq!(
        emitted(transport),
        [("long-999".into(), CommandSource::Menu(node(39)))]
    );
    let mut registry = commands();
    for command in &mut registry {
        command.generation += 2000;
    }
    apply(
        cx,
        handle,
        vec![
            Op::SetCommands(node(38), registry),
            Op::SetMenu(node(39), config(MenuPresentation::Button)),
        ],
    );
    frame(cx, handle).await;
    println!(
        "GPUIO_MENUS_VIRTUAL_OK: 1000 commands, bounded visible rows, wheel routing and keyboard reveal"
    );
}
async fn inside_popover(
    cx: &mut gpui::AsyncApp,
    handle: WindowHandle<View>,
    transport: &Transport,
) {
    apply(
        cx,
        handle,
        vec![
            Op::Create(node(43), Kind::FocusScope, "".into(), Some(handler(43))),
            Op::SetFocusScope(
                node(43),
                FocusScopeConfig {
                    trap: false,
                    auto_focus: true,
                    restore_focus: true,
                },
            ),
            Op::SetOverlay(
                node(43),
                Some(OverlayConfig {
                    kind: OverlayKind::Popover,
                    label: "Menu host".into(),
                    width: 180.,
                    dismiss_on_escape: true,
                    dismiss_on_outside_pointer: true,
                }),
            ),
            Op::Create(
                node(44),
                Kind::Button,
                "Popover anchor".into(),
                Some(handler(44)),
            ),
            Op::SetControl(node(44), Control::Button(false)),
            Op::SetStyle(
                node(44),
                vec![Style::Fields(vec![
                    Field::Position(1),
                    Field::Left(Length::Px(10.)),
                    Field::Top(Length::Px(8.)),
                    Field::Width(Length::Px(160.)),
                    Field::Height(Length::Px(28.)),
                ])],
            ),
            Op::SetStyle(
                node(39),
                vec![Style::Fields(vec![
                    Field::Width(Length::Px(140.)),
                    Field::Height(Length::Px(28.)),
                ])],
            ),
            Op::Splice(node(43), 0, 0, vec![node(39)]),
            Op::Splice(node(0), 4, 3, vec![node(41), node(42), node(44), node(43)]),
        ],
    );
    frame(cx, handle).await;
    focus_menu(cx, handle, 39);
    key(cx, handle, "down");
    frame(cx, handle).await;
    key(cx, handle, "end");
    key(cx, handle, "right");
    frame(cx, handle).await;
    let child = panel(cx, handle, 39, 1);
    let point = child.origin + gpui::point(px(20.), px(4.));
    let parent = handle
        .update(cx, |view, _, _| view.probes.borrow()[&node(43)].bounds)
        .unwrap();
    assert!(
        !parent.contains(&point),
        "submenu interaction really is outside containing popover bounds: parent={parent:?}, child={child:?}, point={point:?}"
    );
    emitted(transport);
    super::super::native_test::move_mouse(cx, handle, point, false);
    super::super::native_test::mouse(cx, handle, point, true);
    super::super::native_test::mouse(cx, handle, point, false);
    frame(cx, handle).await;
    let events = transport.mailbox.lock().unwrap().drain(128);
    assert!(
        !events
            .iter()
            .any(|event| matches!(event, Event::OverlayDismissed(..)))
    );
    assert_eq!(events.iter().filter(|event| matches!(event, Event::CommandInvoked(_, _, _, _, command, _, CommandSource::Menu(source)) if command == "deep" && *source == node(39))).count(), 1);
    key(cx, handle, "down");
    frame(cx, handle).await;
    key(cx, handle, "escape");
    frame(cx, handle).await;
    assert!(
        !transport
            .mailbox
            .lock()
            .unwrap()
            .drain(128)
            .iter()
            .any(|event| matches!(event, Event::OverlayDismissed(..)))
    );
    key(cx, handle, "escape");
    frame(cx, handle).await;
    assert!(transport.mailbox.lock().unwrap().drain(128).iter().any(|event| matches!(event, Event::OverlayDismissed(_, source, _, _, Dismissal::Escape) if *source == node(43))));
    apply(
        cx,
        handle,
        vec![
            Op::Splice(node(43), 0, 1, vec![]),
            Op::Splice(node(0), 4, 4, vec![node(39), node(41), node(42)]),
            Op::Remove(node(44)),
            Op::Remove(node(43)),
            Op::SetStyle(
                node(39),
                vec![Style::Fields(vec![
                    Field::Position(1),
                    Field::Left(Length::Px(8.)),
                    Field::Top(Length::Px(200.)),
                    Field::Width(Length::Px(140.)),
                    Field::Height(Length::Px(28.)),
                ])],
            ),
        ],
    );
    frame(cx, handle).await;
    println!(
        "GPUIO_MENU_POPOVER_OK: submenu panels participate in parent hit routing and Escape dismisses innermost surface first"
    );
}
pub(super) async fn exercise(
    cx: &mut gpui::AsyncApp,
    handle: WindowHandle<View>,
    transport: &Transport,
) {
    let mut operations = vec![
        Op::Create(node(38), Kind::CommandScope, "".into(), Some(handler(38))),
        Op::SetCommands(node(38), commands()),
        Op::Splice(node(38), 0, 0, vec![node(0)]),
        Op::SetRoot(Some(node(38))),
    ];
    for (slot, presentation) in [
        (39, MenuPresentation::Button),
        (40, MenuPresentation::Context),
        (41, MenuPresentation::Bar),
        (42, MenuPresentation::PlatformBar),
    ] {
        operations.extend([
            Op::Create(node(slot), Kind::Menu, "".into(), None),
            Op::SetMenu(node(slot), config(presentation)),
            Op::SetChoiceAppearance(
                node(slot),
                ChoiceAppearance {
                    popup_width: 150.,
                    row_height: 28.,
                    max_visible_rows: 6,
                    ..Default::default()
                },
            ),
        ]);
    }
    operations.extend([
        Op::SetStyle(
            node(39),
            vec![Style::Fields(vec![
                Field::Position(1),
                Field::Left(Length::Px(8.)),
                Field::Top(Length::Px(200.)),
                Field::Width(Length::Px(140.)),
                Field::Height(Length::Px(28.)),
            ])],
        ),
        Op::SetStyle(
            node(41),
            vec![Style::Fields(vec![
                Field::Position(1),
                Field::Left(Length::Px(170.)),
                Field::Top(Length::Px(200.)),
            ])],
        ),
        Op::Splice(node(40), 0, 0, vec![node(4)]),
        Op::Splice(node(0), 3, 1, vec![node(40), node(39), node(41), node(42)]),
    ]);
    apply(cx, handle, operations);
    frame(cx, handle).await;
    #[cfg(target_os = "macos")]
    {
        // macOS activates AccessKit lazily on the first accessibility query.
        // Allow its initial tree request to paint before testing popup updates.
        let _ = accessible(cx, handle, "Actions", false);
        frame(cx, handle).await;
    }
    focus_menu(cx, handle, 39);
    frame(cx, handle).await;
    emitted(transport);
    key(cx, handle, "down");
    frame(cx, handle).await;
    assert_eq!(state(cx, handle, 39), (vec![0], vec![Some(0)]));
    #[cfg(target_os = "macos")]
    {
        let command = accessible(cx, handle, "Menu run", false).expect("menu accessibility item");
        assert!(command.enabled);
        assert_eq!(command.role, "AXMenuItem");
        assert_eq!(command.value, 1);
        assert!(
            !accessible(cx, handle, "Menu disabled", false)
                .unwrap()
                .enabled
        );
        accessible(cx, handle, "Menu run", true).unwrap();
        frame(cx, handle).await;
        assert_eq!(
            emitted(transport),
            [("run".into(), CommandSource::Menu(node(39)))]
        );
        key(cx, handle, "down");
        frame(cx, handle).await;
    }
    key(cx, handle, "down");
    frame(cx, handle).await;
    assert_eq!(
        state(cx, handle, 39),
        (vec![0], vec![Some(3)]),
        "skip separators and disabled commands"
    );
    key(cx, handle, "right");
    frame(cx, handle).await;
    assert_eq!(state(cx, handle, 39), (vec![0, 3], vec![Some(3), Some(0)]));
    let submenu = panel(cx, handle, 39, 1);
    assert!(submenu.left() >= px(0.) && submenu.right() <= px(400.));
    key(cx, handle, "enter");
    frame(cx, handle).await;
    assert_eq!(
        emitted(transport),
        [("deep".into(), CommandSource::Menu(node(39)))]
    );
    assert!(state(cx, handle, 39).0.is_empty());
    key(cx, handle, "enter");
    frame(cx, handle).await;
    key(cx, handle, "end");
    key(cx, handle, "right");
    frame(cx, handle).await;
    key(cx, handle, "left");
    frame(cx, handle).await;
    assert_eq!(state(cx, handle, 39).0, [0]);
    key(cx, handle, "escape");
    frame(cx, handle).await;
    assert!(state(cx, handle, 39).0.is_empty());
    // Context keyboard opening restores the actual native editor on Escape.
    handle
        .update(cx, |view, window, cx| {
            window.focus(&view.editors[&node(4)].focus_handle(cx), cx)
        })
        .unwrap();
    frame(cx, handle).await;
    key(cx, handle, "shift-f10");
    frame(cx, handle).await;
    assert_eq!(state(cx, handle, 40).0, [0]);
    key(cx, handle, "escape");
    frame(cx, handle).await;
    assert!(focused(cx, handle, node(4)));
    // Pointer activation goes through a rendered row after current-frame flipping.
    focus_menu(cx, handle, 39);
    key(cx, handle, "down");
    frame(cx, handle).await;
    let bounds = panel(cx, handle, 39, 0);
    let point = bounds.origin + gpui::point(px(20.), px(14.));
    super::super::native_test::move_mouse(cx, handle, point, false);
    super::super::native_test::mouse(cx, handle, point, true);
    super::super::native_test::mouse(cx, handle, point, false);
    frame(cx, handle).await;
    assert_eq!(
        emitted(transport),
        [("run".into(), CommandSource::Menu(node(39)))]
    );
    // The in-window menu bar uses the same registry and keyboard path.
    focus_menu(cx, handle, 41);
    key(cx, handle, "down");
    frame(cx, handle).await;
    key(cx, handle, "enter");
    frame(cx, handle).await;
    assert_eq!(
        emitted(transport),
        [("run".into(), CommandSource::Menu(node(41)))]
    );
    #[cfg(target_os = "macos")]
    {
        let action = handle
            .update(cx, |_, _, cx| {
                let menus = cx.get_menus().expect("platform menus");
                let menu = menus.iter().find(|menu| menu.name == "Actions").unwrap();
                let gpui::OwnedMenuItem::Action {
                    action,
                    checked,
                    disabled,
                    ..
                } = &menu.items[0]
                else {
                    panic!("command item")
                };
                assert!(*checked && !disabled);
                let gpui::OwnedMenuItem::Action { disabled, .. } = &menu.items[1] else {
                    panic!("disabled item")
                };
                assert!(*disabled);
                action.boxed_clone()
            })
            .unwrap();
        native_menu_press("Actions", 0);
        frame(cx, handle).await;
        assert_eq!(
            emitted(transport),
            [("run".into(), CommandSource::Menu(node(42)))]
        );
        let mut disabled = commands();
        disabled[0].enabled = false;
        disabled[0].generation = 5;
        apply(
            cx,
            handle,
            vec![Op::SetCommands(node(38), disabled.clone())],
        );
        frame(cx, handle).await;
        cx.update(|cx| cx.dispatch_action(action.as_ref()));
        frame(cx, handle).await;
        assert!(emitted(transport).is_empty());
        disabled[0].enabled = true;
        disabled[0].generation = 6;
        apply(cx, handle, vec![Op::SetCommands(node(38), disabled)]);
        frame(cx, handle).await;
        cx.update(|cx| cx.dispatch_action(action.as_ref()));
        frame(cx, handle).await;
        assert!(
            emitted(transport).is_empty(),
            "old OS menu action cannot cross disable/re-enable"
        );
        native_menu_press("Actions", 0);
        frame(cx, handle).await;
        assert_eq!(
            emitted(transport),
            [("run".into(), CommandSource::Menu(node(42)))]
        );
        println!(
            "GPUIO_MENUS_MACOS_OK: real NSMenu activation and stale action rejection across disable/re-enable"
        );
    }
    menu_availability(cx, handle, transport).await;
    context_copy(cx, handle, transport).await;
    large_menu(cx, handle, transport).await;
    inside_popover(cx, handle, transport).await;
    #[cfg(target_os = "macos")]
    platform_contexts(cx, handle, transport).await;
    apply(
        cx,
        handle,
        vec![
            Op::Remove(node(39)),
            Op::Remove(node(41)),
            Op::Remove(node(42)),
            Op::Splice(node(40), 0, 1, vec![]),
            Op::Splice(node(0), 3, 4, vec![node(4)]),
            Op::Remove(node(40)),
            Op::Splice(node(38), 0, 1, vec![]),
            Op::SetRoot(Some(node(0))),
            Op::Remove(node(38)),
        ],
    );
    frame(cx, handle).await;
    handle
        .update(cx, |view, _, cx| {
            assert!(view.menus.is_empty());
            #[cfg(target_os = "macos")]
            assert!(cx.get_menus().unwrap().is_empty());
            #[cfg(not(target_os = "macos"))]
            let _ = cx;
        })
        .unwrap();
    println!(
        "GPUIO_MENUS_NATIVE_OK: nested navigation, disabled/separator skipping, placement, pointer activation, context focus restoration, in-window/platform routing and disposal"
    );
}
