//! Real-window control activation, focus traversal and native accessibility.
#[path = "overlay_test.rs"]
mod overlay_test;
#[path = "tooltip_test.rs"]
mod tooltip_test;
use super::editor_test::{frame, key};
use super::*;
use std::os::fd::{AsRawFd, FromRawFd, OwnedFd};

fn node(slot: i64) -> NodeId {
    NodeId::from_parts(slot, 1).unwrap()
}
fn presses(transport: &Transport) -> Vec<NodeId> {
    transport
        .mailbox
        .lock()
        .unwrap()
        .drain(128)
        .into_iter()
        .filter_map(|event| {
            if let Event::Press(_, node, _, _) = event {
                Some(node)
            } else {
                None
            }
        })
        .collect()
}

fn choices(transport: &Transport) -> Vec<String> {
    transport
        .mailbox
        .lock()
        .unwrap()
        .drain(128)
        .into_iter()
        .filter_map(|event| {
            if let Event::Choice(_, _, _, _, selected) = event {
                Some(selected)
            } else {
                None
            }
        })
        .collect()
}

async fn radio(cx: &mut gpui::AsyncApp, handle: WindowHandle<View>, transport: &Transport) {
    let mut config = ChoiceConfig {
        label: "Mode".into(),
        selected: Some("fast".into()),
        disabled: false,
        items: [
            ("fast", "Fast", false),
            ("blocked", "Disabled option", true),
            ("deep", "Deep", false),
        ]
        .into_iter()
        .map(|(id, label, disabled)| ChoiceItem {
            id: id.into(),
            label: label.into(),
            disabled,
        })
        .collect(),
    };
    apply(
        cx,
        handle,
        vec![
            Op::Create(
                node(5),
                Kind::RadioGroup,
                "".into(),
                Some(gpuio_protocol::HandlerId::from_parts(5, 1).unwrap()),
            ),
            Op::SetChoice(node(5), config.clone()),
            Op::SetStyle(
                node(5),
                vec![Style::Fields(vec![Field::Width(Length::Px(300.))])],
            ),
            Op::Splice(node(0), 4, 0, vec![node(5)]),
        ],
    );
    frame(cx, handle).await;
    handle
        .update(cx, |view, window, cx| {
            window.focus(&view.editors[&node(4)].focus_handle(cx), cx)
        })
        .unwrap();
    frame(cx, handle).await;
    key(cx, handle, "tab");
    frame(cx, handle).await;
    assert!(focused(cx, handle, node(5)), "radio group is one Tab stop");
    choices(transport);
    key(cx, handle, "down");
    key(cx, handle, "down");
    assert_eq!(
        choices(transport),
        ["deep", "fast"],
        "skip disabled, wrap, and preserve consecutive navigation before value commits"
    );
    key(cx, handle, "end");
    assert_eq!(choices(transport), ["deep"]);
    // Committed selected value is independent of the active navigation target.
    handle
        .update(cx, |view, _, _| {
            assert_eq!(
                view.session
                    .borrow()
                    .tree(view.id)
                    .unwrap()
                    .get(node(5))
                    .unwrap()
                    .choice
                    .as_ref()
                    .unwrap()
                    .selected
                    .as_deref(),
                Some("fast")
            )
        })
        .unwrap();
    config.items.reverse();
    apply(cx, handle, vec![Op::SetChoice(node(5), config.clone())]);
    frame(cx, handle).await;
    key(cx, handle, "space");
    assert_eq!(
        choices(transport),
        ["deep"],
        "reorder retains active option ID"
    );
    #[cfg(target_os = "macos")]
    {
        let _ = accessible(cx, handle, "Fast", false);
        frame(cx, handle).await;
        assert_eq!(
            accessible(cx, handle, "Fast", false),
            Some(Accessible {
                role: "AXRadioButton".into(),
                value: 1,
                enabled: true
            })
        );
        assert_eq!(
            accessible(cx, handle, "Deep", true),
            Some(Accessible {
                role: "AXRadioButton".into(),
                value: 0,
                enabled: true
            })
        );
        frame(cx, handle).await;
        assert_eq!(choices(transport), ["deep"]);
        assert_eq!(
            accessible(cx, handle, "Disabled option", false),
            Some(Accessible {
                role: "AXRadioButton".into(),
                value: 0,
                enabled: false
            })
        );
    }
    config.items.retain(|item| item.id != "deep");
    apply(cx, handle, vec![Op::SetChoice(node(5), config.clone())]);
    frame(cx, handle).await;
    key(cx, handle, "space");
    assert_eq!(
        choices(transport),
        ["fast"],
        "removed active option falls back to a valid value"
    );
    config.disabled = true;
    apply(cx, handle, vec![Op::SetChoice(node(5), config.clone())]);
    frame(cx, handle).await;
    assert!(
        !focused(cx, handle, node(5)),
        "disabled group releases focus"
    );
    key(cx, handle, "space");
    assert!(choices(transport).is_empty());
    handle
        .update(cx, |view, window, cx| {
            window.focus(&view.editors[&node(4)].focus_handle(cx), cx)
        })
        .unwrap();
    frame(cx, handle).await;
    key(cx, handle, "tab");
    frame(cx, handle).await;
    assert!(!focused(cx, handle, node(5)), "disabled group is skipped");
    config.disabled = false;
    apply(cx, handle, vec![Op::SetChoice(node(5), config)]);
    frame(cx, handle).await;
    handle
        .update(cx, |view, window, cx| {
            window.focus(&view.buttons[&node(5)].focus, cx)
        })
        .unwrap();
    frame(cx, handle).await;
    key(cx, handle, "shift-tab");
    frame(cx, handle).await;
    assert!(
        focused(cx, handle, node(4)),
        "radio options do not add duplicate Tab stops"
    );
    apply(
        cx,
        handle,
        vec![Op::Remove(node(5)), Op::Splice(node(0), 4, 1, vec![])],
    );
    frame(cx, handle).await;
    handle
        .update(cx, |view, _, _| assert!(view.radios.is_empty()))
        .unwrap();
    println!(
        "GPUIO_RADIO_NATIVE_OK: stable choices, arrow navigation, disabled options/groups, reorder, single Tab stop and disposal"
    );
    #[cfg(target_os = "macos")]
    println!(
        "GPUIO_RADIO_MACOS_AX_OK: option roles, checked values, disabled state and activation"
    );
}
async fn select_control(
    cx: &mut gpui::AsyncApp,
    handle: WindowHandle<View>,
    transport: &Transport,
) {
    let mut config = ChoiceConfig {
        label: "Select mode".into(),
        selected: Some("fast".into()),
        disabled: false,
        items: [
            ("fast", "Fast choice", false),
            ("blocked", "Disabled choice", true),
            ("deep", "Deep choice", false),
        ]
        .into_iter()
        .map(|(id, label, disabled)| ChoiceItem {
            id: id.into(),
            label: label.into(),
            disabled,
        })
        .collect(),
    };
    apply(
        cx,
        handle,
        vec![
            Op::Create(
                node(6),
                Kind::Select,
                "".into(),
                Some(gpuio_protocol::HandlerId::from_parts(6, 1).unwrap()),
            ),
            Op::SetChoice(node(6), config.clone()),
            Op::SetStyle(
                node(6),
                vec![Style::Fields(vec![
                    Field::Width(Length::Px(300.)),
                    Field::Height(Length::Px(40.)),
                ])],
            ),
            Op::Splice(node(0), 4, 0, vec![node(6)]),
        ],
    );
    frame(cx, handle).await;
    handle
        .update(cx, |view, window, cx| {
            window.focus(&view.buttons[&node(6)].focus, cx)
        })
        .unwrap();
    frame(cx, handle).await;
    choices(transport);
    key(cx, handle, "space");
    frame(cx, handle).await;
    assert!(select_is_open(cx, handle));
    assert!(focused(cx, handle, node(6)));
    let (trigger, popup, viewport) = handle
        .update(cx, |view, window, _| {
            (
                view.probes.borrow()[&node(6)].bounds,
                view.selects[&node(6)]
                    .borrow()
                    .popup
                    .borrow()
                    .popup_bounds
                    .get(),
                window.viewport_size(),
            )
        })
        .unwrap();
    assert!(
        popup.size.height > px(90.),
        "popup has measured option rows: {popup:?}"
    );
    assert!(
        popup.top() >= px(0.) && popup.bottom() <= viewport.height,
        "popup fits viewport: {popup:?}"
    );
    assert!(popup.left() >= px(0.) && popup.right() <= viewport.width);
    assert!(
        popup.bottom() <= trigger.top() || popup.top() >= trigger.bottom(),
        "popup avoids covering trigger when one side fits: {trigger:?} {popup:?}"
    );
    key(cx, handle, "down");
    assert!(choices(transport).is_empty(), "highlight is not selection");
    let appearance = ChoiceAppearance {
        popup_width: 240.,
        row_height: 48.,
        max_visible_rows: 3,
        empty_label: "Keine Optionen".into(),
        popup_style: vec![Style::Fields(vec![Field::Foreground(Color::Rgba(
            0x445566ff,
        ))])],
        option_style: vec![
            Style::Fields(vec![Field::Foreground(Color::Rgba(0x123456ff))]),
            Style::State(1, vec![Field::Foreground(Color::Rgba(0xabcdefff))]),
            Style::State(7, vec![Field::Foreground(Color::Rgba(0xff0000ff))]),
            Style::State(6, vec![Field::Foreground(Color::Rgba(0x999999ff))]),
        ],
        empty_style: vec![Style::Fields(vec![Field::FontSize(14.)])],
    };
    let retained_state = handle
        .update(cx, |view, _, _| view.selects[&node(6)].clone())
        .unwrap();
    apply(
        cx,
        handle,
        vec![Op::SetChoiceAppearance(node(6), appearance)],
    );
    frame(cx, handle).await;
    assert!(focused(cx, handle, node(6)));
    assert!(select_is_open(cx, handle));
    handle
        .update(cx, |view, _, _| {
            assert!(Rc::ptr_eq(&retained_state, &view.selects[&node(6)]));
            let popup = view.selects[&node(6)].borrow().popup.clone();
            let state = popup.borrow();
            let bounds = state.popup_bounds.get();
            assert!(bounds.size.width >= px(236.) && bounds.size.width <= px(240.));
            let probes = state.option_probes.borrow();
            assert_eq!(probes["deep"].color, rgba(0xabcdefff).into());
            assert_eq!(probes["fast"].color, rgba(0xff0000ff).into());
            assert_eq!(probes["blocked"].color, rgba(0x999999ff).into());
            assert_eq!(probes["deep"].bounds.size.height, px(48.));
        })
        .unwrap();
    key(cx, handle, "escape");
    frame(cx, handle).await;
    assert!(!select_is_open(cx, handle));
    assert!(choices(transport).is_empty(), "Escape cancels");
    key(cx, handle, "enter");
    frame(cx, handle).await;
    key(cx, handle, "down");
    key(cx, handle, "enter");
    assert_eq!(
        choices(transport),
        ["deep"],
        "skip disabled and commit once on Enter release"
    );
    frame(cx, handle).await;
    assert!(!select_is_open(cx, handle));
    key(cx, handle, "space");
    frame(cx, handle).await;
    key(cx, handle, "down");
    config.items.reverse();
    apply(cx, handle, vec![Op::SetChoice(node(6), config.clone())]);
    frame(cx, handle).await;
    key(cx, handle, "enter");
    assert_eq!(
        choices(transport),
        ["deep"],
        "open highlight retains stable identity across reorder"
    );
    frame(cx, handle).await;
    key(cx, handle, "space");
    frame(cx, handle).await;
    key(cx, handle, "shift-tab");
    frame(cx, handle).await;
    assert!(!select_is_open(cx, handle));
    assert!(
        focused(cx, handle, node(4)),
        "Tab leaves composite normally"
    );
    key(cx, handle, "tab");
    frame(cx, handle).await;
    key(cx, handle, "space");
    frame(cx, handle).await;
    let position = handle
        .update(cx, |view, _, _| {
            view.selects[&node(6)]
                .borrow()
                .popup
                .borrow()
                .popup_bounds
                .get()
                .origin
                + gpui::point(px(15.), px(16.))
        })
        .unwrap();
    super::native_test::move_mouse(cx, handle, position, false);
    super::native_test::mouse(cx, handle, position, true);
    super::native_test::mouse(cx, handle, position, false);
    assert_eq!(
        choices(transport),
        ["deep"],
        "pointer option activates without trigger toggle"
    );
    frame(cx, handle).await;
    assert!(!select_is_open(cx, handle));
    assert!(focused(cx, handle, node(6)));
    key(cx, handle, "space");
    frame(cx, handle).await;
    #[cfg(target_os = "macos")]
    {
        let _ = accessible(cx, handle, "Select mode", false);
        frame(cx, handle).await;
        assert_eq!(
            accessible(cx, handle, "Select mode", false).unwrap().role,
            "AXPopUpButton"
        );
        assert!(
            !accessible(cx, handle, "Disabled choice", false)
                .unwrap()
                .enabled
        );
        assert!(accessible(cx, handle, "Deep choice", true).unwrap().enabled);
        frame(cx, handle).await;
        assert_eq!(choices(transport), ["deep"]);
        assert!(!select_is_open(cx, handle));
    }
    if select_is_open(cx, handle) {
        key(cx, handle, "escape");
        frame(cx, handle).await;
    }
    key(cx, handle, "f");
    frame(cx, handle).await;
    assert!(
        select_is_open(cx, handle),
        "printable keys open type-ahead navigation"
    );
    assert!(
        choices(transport).is_empty(),
        "typing does not commit selection"
    );
    key(cx, handle, "enter");
    assert_eq!(choices(transport), ["fast"]);
    frame(cx, handle).await;
    key(cx, handle, "d");
    key(cx, handle, "e");
    frame(cx, handle).await;
    key(cx, handle, "enter");
    assert_eq!(
        choices(transport),
        ["deep"],
        "prefix resets between opens and searches labels"
    );
    frame(cx, handle).await;
    // Large options remain bounded to the popup viewport and navigation reveals
    // offscreen choices. A reorder must also reveal the retained active ID.
    config.items = (0..4096)
        .map(|i| ChoiceItem {
            id: format!("item-{i}"),
            label: format!("Option {i}"),
            disabled: false,
        })
        .collect();
    config.selected = Some("item-0".into());
    apply(cx, handle, vec![Op::SetChoice(node(6), config.clone())]);
    frame(cx, handle).await;
    if select_is_open(cx, handle) {
        key(cx, handle, "escape");
        frame(cx, handle).await;
    }
    key(cx, handle, "space");
    frame(cx, handle).await;
    key(cx, handle, "end");
    frame(cx, handle).await;
    handle
        .update(cx, |view, _, _| {
            assert!(
                view.selects[&node(6)]
                    .borrow()
                    .popup
                    .borrow()
                    .rendered_options
                    .get()
                    < 16
            )
        })
        .unwrap();
    #[cfg(target_os = "macos")]
    assert!(
        accessible(cx, handle, "Option 4095", false).is_some(),
        "last choice is actually rendered after End"
    );
    handle
        .update(cx, |_, window, _| {
            window.resize(gpui::size(px(400.), px(160.)))
        })
        .unwrap();
    frame(cx, handle).await;
    handle
        .update(cx, |view, _, _| {
            let popup = view.selects[&node(6)].borrow().popup.clone();
            let state = popup.borrow();
            let probes = state.option_probes.borrow();
            assert!(
                probes["item-4095"].bounds.bottom() <= state.popup_bounds.get().bottom() + px(1.),
                "resize reveals entire active option"
            );
        })
        .unwrap();
    handle
        .update(cx, |_, window, _| {
            window.resize(gpui::size(px(400.), px(280.)))
        })
        .unwrap();
    frame(cx, handle).await;
    config.items.reverse();
    apply(cx, handle, vec![Op::SetChoice(node(6), config.clone())]);
    frame(cx, handle).await;
    #[cfg(target_os = "macos")]
    assert!(
        accessible(cx, handle, "Option 4095", false).is_some(),
        "reorder reveals retained active choice"
    );
    key(cx, handle, "enter");
    assert_eq!(choices(transport), ["item-4095"]);
    frame(cx, handle).await;
    key(cx, handle, "space");
    frame(cx, handle).await;
    // Clicking outside dismisses without choosing or preventing the other target.
    let outside = gpui::point(px(380.), px(10.));
    super::native_test::move_mouse(cx, handle, outside, false);
    super::native_test::mouse(cx, handle, outside, true);
    super::native_test::mouse(cx, handle, outside, false);
    frame(cx, handle).await;
    assert!(!select_is_open(cx, handle));
    assert!(choices(transport).is_empty());
    handle
        .update(cx, |view, window, cx| {
            window.focus(&view.buttons[&node(6)].focus, cx)
        })
        .unwrap();
    frame(cx, handle).await;
    key(cx, handle, "space");
    frame(cx, handle).await;
    config.items.clear();
    config.selected = None;
    apply(cx, handle, vec![Op::SetChoice(node(6), config.clone())]);
    frame(cx, handle).await;
    #[cfg(target_os = "macos")]
    assert!(accessible(cx, handle, "Keine Optionen", false).is_some());
    key(cx, handle, "enter");
    assert!(
        choices(transport).is_empty(),
        "empty lists never fabricate a selection"
    );
    frame(cx, handle).await;
    key(cx, handle, "space");
    frame(cx, handle).await;
    config.disabled = true;
    apply(cx, handle, vec![Op::SetChoice(node(6), config)]);
    frame(cx, handle).await;
    assert!(!select_is_open(cx, handle));
    assert!(!focused(cx, handle, node(6)));
    apply(
        cx,
        handle,
        vec![Op::Splice(node(0), 4, 1, vec![]), Op::Remove(node(6))],
    );
    frame(cx, handle).await;
    handle
        .update(cx, |view, _, _| assert!(view.selects.is_empty()))
        .unwrap();
    println!(
        "GPUIO_SELECT_TYPEAHEAD_OK: printable prefix navigation, explicit confirmation and accessible localized empty state"
    );
    println!(
        "GPUIO_SELECT_APPEARANCE_OK: identity/focus/open retained, custom geometry and native active/selected/disabled colors"
    );
    println!(
        "GPUIO_SELECT_NATIVE_OK: popup position, keyboard/cancel, stable highlight, pointer, focus, 4096-option virtualization and disposal"
    );
    #[cfg(target_os = "macos")]
    println!(
        "GPUIO_SELECT_MACOS_AX_OK: popup role, disabled options, semantic selection and offscreen navigation"
    );
}
async fn combobox_control(
    cx: &mut gpui::AsyncApp,
    handle: WindowHandle<View>,
    transport: &Transport,
) {
    let config = ChoiceConfig {
        label: "Search model".into(),
        selected: Some("fast".into()),
        disabled: false,
        items: [
            ("fast", "Fast choice", false),
            ("disabled", "Deep disabled", true),
            ("deep", "Deep model", false),
        ]
        .into_iter()
        .map(|(id, label, disabled)| ChoiceItem {
            id: id.into(),
            label: label.into(),
            disabled,
        })
        .collect(),
    };
    let editor = EditorConfig {
        label: config.label.clone(),
        placeholder: "Find a model".into(),
        disabled: false,
        read_only: false,
        submit_on_enter: false,
        auto_focus: true,
        min_rows: 1,
        max_rows: 1,
    };
    apply(
        cx,
        handle,
        vec![
            Op::Create(
                node(7),
                Kind::Combobox,
                "".into(),
                Some(gpuio_protocol::HandlerId::from_parts(7, 1).unwrap()),
            ),
            Op::SetEditor(node(7), editor.clone()),
            Op::SetChoice(node(7), config.clone()),
            Op::SetComboboxFilter(node(7), ComboboxFilter::Substring),
            Op::SetStyle(
                node(7),
                vec![Style::Fields(vec![
                    Field::Width(Length::Px(300.)),
                    Field::Height(Length::Px(36.)),
                ])],
            ),
            Op::Splice(node(0), 4, 0, vec![node(7)]),
        ],
    );
    frame(cx, handle).await;
    assert!(focused(cx, handle, node(7)));
    let get = |cx: &mut gpui::AsyncApp| {
        handle
            .update(cx, |view, window, cx| {
                view.editors[&node(7)].snapshot(window, cx)
            })
            .unwrap()
    };
    let open = |cx: &mut gpui::AsyncApp| {
        handle
            .update(cx, |view, _, _| {
                view.editors[&node(7)]
                    .combobox()
                    .unwrap()
                    .1
                    .borrow()
                    .popup
                    .borrow()
                    .open
            })
            .unwrap()
    };
    assert!(!open(cx));
    key(cx, handle, "down");
    frame(cx, handle).await;
    assert!(open(cx));
    key(cx, handle, "escape");
    frame(cx, handle).await;
    assert!(!open(cx));
    super::editor_test::native_text(cx, handle, "De", false);
    frame(cx, handle).await;
    assert_eq!(get(cx).text, "De");
    assert!(open(cx));
    handle
        .update(cx, |view, _, _| {
            let state = view.editors[&node(7)].combobox().unwrap().1;
            let popup = state.borrow().popup.clone();
            let popup = popup.borrow();
            let probes = popup.option_probes.borrow();
            assert!(!probes.contains_key("fast"));
            assert!(probes.contains_key("disabled") && probes.contains_key("deep"));
            assert_eq!(popup.navigation.active.as_deref(), Some("deep"));
        })
        .unwrap();
    #[cfg(target_os = "macos")]
    {
        let input = accessible(cx, handle, "Search model", false).unwrap();
        assert_eq!(input.role, "AXComboBox");
        assert!(input.enabled);
    }
    choices(transport);
    let before = get(cx);
    key(cx, handle, "enter");
    frame(cx, handle).await;
    assert!(!open(cx));
    assert!(focused(cx, handle, node(7)));
    let selections = transport
        .mailbox
        .lock()
        .unwrap()
        .drain(128)
        .into_iter()
        .filter_map(|event| match event {
            Event::ComboboxSelected(_, _, _, _, id, snapshot) => Some((id, snapshot)),
            Event::EditorEvent(_, _, _, _, EditorEventKind::Submitted, _) => {
                panic!("combobox submitted free text")
            }
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(selections, vec![("deep".into(), before.clone())]);
    assert_eq!(
        get(cx).text,
        "De",
        "choosing does not implicitly replace query"
    );
    assert_eq!(
        handle
            .update(cx, |view, _, _| {
                view.session
                    .borrow()
                    .tree(view.id)
                    .unwrap()
                    .get(node(7))
                    .unwrap()
                    .choice
                    .as_ref()
                    .unwrap()
                    .selected
                    .clone()
            })
            .unwrap(),
        Some("fast".into()),
        "application selection remains controlled"
    );
    super::editor_test::native_text(cx, handle, "x", false);
    frame(cx, handle).await;
    let result = handle
        .update(cx, |view, window, cx| {
            view.editors.get_mut(&node(7)).unwrap().command(
                &EditorCommand::Replace(
                    "Deep model".into(),
                    EditorSelectionPolicy::End,
                    EditorUndoPolicy::Record,
                    Some(before.revision),
                ),
                window,
                cx,
            )
        })
        .unwrap();
    assert_eq!(result, EditorResult::Failed(EditorError::StaleRevision));
    assert_eq!(get(cx).text, "Dex");
    #[cfg(target_os = "macos")]
    {
        key(cx, handle, "cmd-a");
        super::editor_test::native_text(cx, handle, "に", true);
        frame(cx, handle).await;
        assert!(get(cx).composition.is_some());
        assert!(!open(cx), "composition closes suggestions");
        key(cx, handle, "down");
        frame(cx, handle).await;
        assert!(!open(cx), "composing arrow cannot open suggestions");
        super::editor_test::native_text(cx, handle, "日本", false);
        frame(cx, handle).await;
        assert!(get(cx).composition.is_none());
        assert_eq!(get(cx).text, "日本");
    }
    key(cx, handle, "tab");
    frame(cx, handle).await;
    assert!(!open(cx));
    assert!(!focused(cx, handle, node(7)));
    apply(
        cx,
        handle,
        vec![Op::Splice(node(0), 4, 1, vec![]), Op::Remove(node(7))],
    );
    frame(cx, handle).await;
    handle
        .update(cx, |view, _, _| {
            assert!(!view.editors.contains_key(&node(7)))
        })
        .unwrap();
    println!(
        "GPUIO_COMBOBOX_NATIVE_OK: native query/filtering, keyboard/cancel, exact intent snapshot, controlled value, stale replacement, Tab and cleanup"
    );
    #[cfg(target_os = "macos")]
    println!(
        "GPUIO_COMBOBOX_MACOS_OK: editable combo role and native marked/committed text without popup interference"
    );
}

async fn focus_scopes(cx: &mut gpui::AsyncApp, handle: WindowHandle<View>, transport: &Transport) {
    let scope = FocusScopeConfig {
        trap: true,
        auto_focus: true,
        restore_focus: true,
    };
    let outside_config = handle
        .update(cx, |view, window, cx| {
            window.focus(&view.editors[&node(4)].focus_handle(cx), cx);
            view.session
                .borrow()
                .tree(view.id)
                .unwrap()
                .get(node(4))
                .unwrap()
                .editor
                .as_ref()
                .unwrap()
                .as_ref()
                .clone()
        })
        .unwrap();
    frame(cx, handle).await;
    let button = |slot, label: &str, disabled| {
        vec![
            Op::Create(
                node(slot),
                Kind::Button,
                label.into(),
                Some(gpuio_protocol::HandlerId::from_parts(slot, 1).unwrap()),
            ),
            Op::SetControl(node(slot), Control::Button(disabled)),
            Op::SetStyle(
                node(slot),
                vec![Style::Fields(vec![
                    Field::Width(Length::Px(200.)),
                    Field::Height(Length::Px(28.)),
                ])],
            ),
        ]
    };
    let mut operations = vec![
        Op::SetControl(node(1), Control::Checkbox(CheckState::Unchecked, false)),
        Op::Create(node(8), Kind::FocusScope, "".into(), None),
        Op::SetFocusScope(node(8), scope),
        Op::SetStyle(
            node(8),
            vec![Style::Fields(vec![
                Field::Position(1),
                Field::Top(Length::Px(0.)),
                Field::Left(Length::Px(0.)),
                Field::Width(Length::Px(240.)),
                Field::Height(Length::Px(140.)),
            ])],
        ),
    ];
    operations.extend(button(9, "Scope first", false));
    operations.extend([
        Op::Create(
            node(10),
            Kind::Input,
            "inside".into(),
            Some(gpuio_protocol::HandlerId::from_parts(10, 1).unwrap()),
        ),
        Op::SetEditor(
            node(10),
            EditorConfig {
                label: "Scope editor".into(),
                placeholder: "".into(),
                disabled: false,
                read_only: false,
                submit_on_enter: false,
                auto_focus: false,
                min_rows: 1,
                max_rows: 1,
            },
        ),
        Op::SetStyle(
            node(10),
            vec![Style::Fields(vec![
                Field::Width(Length::Px(200.)),
                Field::Height(Length::Px(28.)),
            ])],
        ),
    ]);
    operations.extend(button(11, "Scope hidden", false));
    operations.push(Op::SetStyle(
        node(11),
        vec![Style::Fields(vec![Field::Visibility(1)])],
    ));
    operations.extend(button(12, "Scope disabled", true));
    operations.extend([
        Op::Splice(node(8), 0, 0, vec![node(9), node(10), node(11), node(12)]),
        Op::Splice(node(0), 4, 0, vec![node(8)]),
    ]);
    apply(cx, handle, operations);
    frame(cx, handle).await;
    assert!(
        focused(cx, handle, node(9)),
        "scope enters first visible enabled control"
    );
    key(cx, handle, "tab");
    frame(cx, handle).await;
    assert!(focused(cx, handle, node(10)));
    key(cx, handle, "tab");
    frame(cx, handle).await;
    assert!(
        focused(cx, handle, node(9)),
        "Tab skips hidden/disabled and wraps inside"
    );
    key(cx, handle, "shift-tab");
    frame(cx, handle).await;
    assert!(focused(cx, handle, node(10)));
    let result = handle
        .update(cx, |view, window, cx| {
            view.editors
                .get_mut(&node(4))
                .unwrap()
                .command(&EditorCommand::Focus, window, cx)
        })
        .unwrap();
    assert_eq!(result, EditorResult::Failed(EditorError::FocusBlocked));
    assert!(
        focused(cx, handle, node(10)),
        "blocked command never moves focus outside"
    );
    #[cfg(target_os = "macos")]
    {
        presses(transport);
        assert!(accessible(cx, handle, "Check", true).unwrap().enabled);
        frame(cx, handle).await;
        assert!(
            focused(cx, handle, node(10)),
            "outside accessibility focus is blocked"
        );
        assert!(
            presses(transport).is_empty(),
            "outside accessibility activation is blocked"
        );
    }
    apply(
        cx,
        handle,
        vec![Op::SetStyle(
            node(9),
            vec![Style::Fields(vec![Field::Visibility(1)])],
        )],
    );
    frame(cx, handle).await;
    key(cx, handle, "tab");
    frame(cx, handle).await;
    assert!(
        focused(cx, handle, node(10)),
        "one eligible control wraps to itself"
    );
    apply(
        cx,
        handle,
        vec![
            Op::SetStyle(
                node(9),
                vec![Style::Fields(vec![
                    Field::Width(Length::Px(200.)),
                    Field::Height(Length::Px(28.)),
                ])],
            ),
            Op::Splice(node(8), 0, 2, vec![node(10), node(9)]),
        ],
    );
    frame(cx, handle).await;
    key(cx, handle, "tab");
    frame(cx, handle).await;
    assert!(
        focused(cx, handle, node(9)),
        "native order follows keyed reorder"
    );
    key(cx, handle, "shift-tab");
    frame(cx, handle).await;
    let mut nested = vec![
        Op::Create(node(13), Kind::FocusScope, "".into(), None),
        Op::SetFocusScope(node(13), scope),
    ];
    nested.extend(button(14, "Nested", false));
    nested.extend([
        Op::Splice(node(13), 0, 0, vec![node(14)]),
        Op::Splice(node(8), 4, 0, vec![node(13)]),
    ]);
    apply(cx, handle, nested);
    frame(cx, handle).await;
    assert!(focused(cx, handle, node(14)));
    key(cx, handle, "shift-tab");
    frame(cx, handle).await;
    assert!(focused(cx, handle, node(14)), "nested trap is exclusive");
    apply(
        cx,
        handle,
        vec![
            Op::Splice(node(8), 4, 1, vec![]),
            Op::Remove(node(14)),
            Op::Remove(node(13)),
        ],
    );
    frame(cx, handle).await;
    assert!(
        focused(cx, handle, node(10)),
        "closing nested scope restores parent focus"
    );
    let mut disabled = outside_config.clone();
    disabled.disabled = true;
    apply(cx, handle, vec![Op::SetEditor(node(4), disabled)]);
    frame(cx, handle).await;
    apply(
        cx,
        handle,
        vec![
            Op::Splice(node(0), 4, 1, vec![]),
            Op::Remove(node(9)),
            Op::Remove(node(10)),
            Op::Remove(node(11)),
            Op::Remove(node(12)),
            Op::Remove(node(8)),
        ],
    );
    frame(cx, handle).await;
    handle
        .update(cx, |view, window, _| {
            assert!(
                view.root_focus.as_ref().unwrap().is_focused(window),
                "disabled restoration target falls back without focus probe"
            )
        })
        .unwrap();
    apply(
        cx,
        handle,
        vec![
            Op::SetEditor(node(4), outside_config),
            Op::Create(node(15), Kind::FocusScope, "".into(), None),
            Op::SetFocusScope(node(15), scope),
            Op::Splice(node(0), 4, 0, vec![node(15)]),
        ],
    );
    frame(cx, handle).await;
    key(cx, handle, "tab");
    frame(cx, handle).await;
    handle
        .update(cx, |view, window, _| {
            assert!(
                view.focus
                    .borrow()
                    .handle(node(15))
                    .unwrap()
                    .is_focused(window),
                "empty trap retains its root"
            )
        })
        .unwrap();
    let mut populated = button(16, "Late child", false);
    populated.push(Op::Splice(node(15), 0, 0, vec![node(16)]));
    apply(cx, handle, populated);
    frame(cx, handle).await;
    key(cx, handle, "tab");
    frame(cx, handle).await;
    assert!(focused(cx, handle, node(16)));
    apply(
        cx,
        handle,
        vec![
            Op::Splice(node(0), 4, 1, vec![]),
            Op::Remove(node(16)),
            Op::Remove(node(15)),
        ],
    );
    frame(cx, handle).await;
    handle
        .update(cx, |view, _, _| {
            for id in [8, 13, 15] {
                assert!(view.focus.borrow().handle(node(id)).is_none());
            }
            assert!(
                !view.focus.borrow_mut().take_pending(),
                "focus repair does not permanently poll"
            );
        })
        .unwrap();
    println!(
        "GPUIO_FOCUS_SCOPES_OK: painted traversal, hidden/disabled/reorder, nested traps/restoration, blocked commands/AX, empty fallback and cleanup"
    );
}

fn select_is_open(cx: &mut gpui::AsyncApp, handle: WindowHandle<View>) -> bool {
    handle
        .update(cx, |view, _, _| {
            view.selects[&node(6)].borrow().popup.borrow().open
        })
        .unwrap()
}

fn apply(cx: &mut gpui::AsyncApp, handle: WindowHandle<View>, operations: Vec<Op>) {
    handle
        .update(cx, |view, window, cx| {
            let base = view.session.borrow().tree(view.id).unwrap().revision();
            let transaction = Transaction {
                window: view.id,
                base,
                revision: base + 1,
                operations,
            };
            let applied = view
                .session
                .borrow_mut()
                .apply(&transaction)
                .unwrap_or_else(|error| {
                    panic!("native test transaction rejected: {error:?}: {transaction:?}")
                });
            view.update_editors(&applied.dirty, window, cx);
            cx.notify();
        })
        .unwrap();
}
fn focused(cx: &mut gpui::AsyncApp, handle: WindowHandle<View>, id: NodeId) -> bool {
    handle
        .update(cx, |view, window, cx| {
            if let Some(editor) = view.editors.get(&id) {
                editor.focus_handle(cx).is_focused(window)
            } else {
                view.buttons
                    .get(&id)
                    .is_some_and(|button| button.focus.is_focused(window))
            }
        })
        .unwrap()
}

fn assert_color(cx: &mut gpui::AsyncApp, handle: WindowHandle<View>, id: NodeId, expected: u32) {
    handle
        .update(cx, |view, _, _| {
            assert_eq!(view.probes.borrow()[&id].color, rgba(expected).into());
        })
        .unwrap();
}

#[cfg(target_os = "macos")]
#[derive(Debug, PartialEq)]
struct Accessible {
    role: String,
    value: isize,
    enabled: bool,
}

#[cfg(target_os = "macos")]
fn accessible(
    cx: &mut gpui::AsyncApp,
    handle: WindowHandle<View>,
    label: &str,
    press: bool,
) -> Option<Accessible> {
    use objc2::{
        msg_send,
        runtime::{AnyObject, Bool},
    };
    use objc2_foundation::NSString;
    unsafe fn visit(
        object: *mut AnyObject,
        label: &str,
        press: bool,
        depth: usize,
    ) -> Option<Accessible> {
        if object.is_null() || depth > 16 {
            return None;
        }
        unsafe {
            let title: *mut NSString = msg_send![object, accessibilityTitle];
            if !title.is_null() && (*title).to_string() == label {
                let role: *mut NSString = msg_send![object, accessibilityRole];
                let value: *mut AnyObject = msg_send![object, accessibilityValue];
                let value: isize = if value.is_null() {
                    -1
                } else {
                    msg_send![value, integerValue]
                };
                let enabled: Bool = msg_send![object, isAccessibilityEnabled];
                if press {
                    let _: () = msg_send![object, setAccessibilityFocused: true];
                    let accepted: Bool = msg_send![object, accessibilityPerformPress];
                    assert!(accepted.as_bool());
                }
                return Some(Accessible {
                    role: (*role).to_string(),
                    value,
                    enabled: enabled.as_bool(),
                });
            }
            let children: *mut AnyObject = msg_send![object, accessibilityChildren];
            if children.is_null() {
                return None;
            }
            let count: usize = msg_send![children, count];
            assert!(count < 128);
            for index in 0..count {
                let child: *mut AnyObject = msg_send![children, objectAtIndex: index];
                if let Some(found) = visit(child, label, press, depth + 1) {
                    return Some(found);
                }
            }
            None
        }
    }
    let view = super::editor_test::native_view(cx, handle) as *mut AnyObject;
    unsafe {
        let window: *mut AnyObject = msg_send![view, window];
        let content: *mut AnyObject = msg_send![window, contentView];
        visit(content, label, press, 0)
    }
}

async fn exercise(cx: &mut gpui::AsyncApp, handle: WindowHandle<View>, transport: Arc<Transport>) {
    frame(cx, handle).await;
    key(cx, handle, "tab");
    frame(cx, handle).await;
    assert!(focused(cx, handle, node(1)));
    assert_color(cx, handle, node(1), 0x555555ff);
    presses(&transport);
    key(cx, handle, "space");
    key(cx, handle, "space");
    assert_eq!(
        presses(&transport),
        [node(1), node(1)],
        "two activations before any value commit"
    );
    key(cx, handle, "enter");
    assert_eq!(presses(&transport), [node(1)]);
    let position = handle
        .update(cx, |view, _, _| {
            view.probes.borrow()[&node(2)].bounds.center()
        })
        .unwrap();
    super::native_test::move_mouse(cx, handle, position, false);
    super::native_test::mouse(cx, handle, position, true);
    super::native_test::mouse(cx, handle, position, false);
    assert!(
        focused(cx, handle, node(1)),
        "pointer-disabled control does not steal focus"
    );
    assert!(presses(&transport).is_empty());
    key(cx, handle, "tab");
    frame(cx, handle).await;
    assert!(focused(cx, handle, node(2)));
    key(cx, handle, "space");
    assert_eq!(
        presses(&transport),
        [node(2)],
        "pointer policy does not suppress keyboard"
    );
    key(cx, handle, "tab");
    frame(cx, handle).await;
    assert!(
        focused(cx, handle, node(4)),
        "disabled button skipped; editor owns one Tab stop"
    );
    assert_color(cx, handle, node(4), 0x555555ff);
    key(cx, handle, "shift-tab");
    frame(cx, handle).await;
    assert!(focused(cx, handle, node(2)));

    apply(
        cx,
        handle,
        vec![Op::SetControl(node(2), Control::Switch(true, true))],
    );
    frame(cx, handle).await;
    assert!(!focused(cx, handle, node(2)), "disable blurs focus");
    assert_color(cx, handle, node(2), 0x444444ff);
    presses(&transport);
    key(cx, handle, "space");
    assert!(presses(&transport).is_empty());
    apply(
        cx,
        handle,
        vec![
            Op::SetControl(node(2), Control::Switch(true, false)),
            Op::SetControl(node(1), Control::Checkbox(CheckState::Indeterminate, false)),
        ],
    );
    frame(cx, handle).await;

    #[cfg(target_os = "macos")]
    {
        // First query activates AccessKit, then a frame publishes the tree.
        let _ = accessible(cx, handle, "Check", false);
        frame(cx, handle).await;
        assert_eq!(
            accessible(cx, handle, "Check", false),
            Some(Accessible {
                role: "AXCheckBox".into(),
                value: 2,
                enabled: true
            })
        );
        assert_eq!(
            accessible(cx, handle, "Switch", true),
            Some(Accessible {
                role: "AXCheckBox".into(),
                value: 1,
                enabled: true
            })
        );
        frame(cx, handle).await;
        assert!(focused(cx, handle, node(2)));
        assert_eq!(
            presses(&transport),
            [node(2)],
            "accessibility works when pointer events are disabled"
        );
        apply(
            cx,
            handle,
            vec![Op::SetControl(
                node(1),
                Control::Checkbox(CheckState::Unchecked, true),
            )],
        );
        frame(cx, handle).await;
        assert_eq!(
            accessible(cx, handle, "Check", false),
            Some(Accessible {
                role: "AXCheckBox".into(),
                value: 0,
                enabled: false
            })
        );
    }
    radio(cx, handle, &transport).await;
    select_control(cx, handle, &transport).await;
    combobox_control(cx, handle, &transport).await;
    focus_scopes(cx, handle, &transport).await;
    overlay_test::exercise(cx, handle, &transport).await;
    tooltip_test::exercise(cx, handle, &transport).await;
    // Remove a focused native node and enter the surviving Tab order again.
    handle
        .update(cx, |view, window, cx| {
            window.focus(&view.buttons[&node(2)].focus, cx)
        })
        .unwrap();
    apply(
        cx,
        handle,
        vec![
            Op::SetControl(node(1), Control::Checkbox(CheckState::Indeterminate, false)),
            Op::Remove(node(2)),
            Op::Splice(node(0), 1, 1, vec![]),
        ],
    );
    frame(cx, handle).await;
    assert_color(cx, handle, node(1), 0x333333ff);
    key(cx, handle, "tab");
    frame(cx, handle).await;
    assert!(
        focused(cx, handle, node(1)),
        "removed focus recovers at the window root"
    );
    apply(
        cx,
        handle,
        vec![
            Op::Remove(node(1)),
            Op::Remove(node(3)),
            Op::Remove(node(4)),
            Op::Remove(node(0)),
            Op::SetRoot(None),
        ],
    );
    frame(cx, handle).await;
    handle
        .update(cx, |view, window, _| {
            assert!(view.buttons.is_empty());
            assert!(view.editors.is_empty());
            assert_eq!(
                view.session
                    .borrow()
                    .tree(view.id)
                    .unwrap()
                    .retained_bytes(),
                0
            );
            window.remove_window();
        })
        .unwrap();
}

pub fn run() {
    let failure = Rc::new(RefCell::new(None));
    let task_failure = failure.clone();
    let mut fds = [0; 2];
    assert_eq!(unsafe { libc::pipe(fds.as_mut_ptr()) }, 0);
    let _reader = unsafe { OwnedFd::from_raw_fd(fds[0]) };
    let writer = unsafe { OwnedFd::from_raw_fd(fds[1]) };
    let transport = Arc::new(Transport::new(writer.as_raw_fd()).unwrap());
    gpui_platform::application().run(move |cx| {
        gpui_base::init(cx);
        cx.set_quit_mode(gpui::QuitMode::Explicit);
        let id = WindowId::from_parts(0, 1).unwrap();
        let session = Rc::new(RefCell::new(Session::default()));
        session.borrow_mut().hello(VERSION, CAPABILITIES).unwrap();
        session
            .borrow_mut()
            .open(1, id, "GPUIO controls test", 400., 280.)
            .unwrap();
        let mut operations = vec![Op::Create(node(0), Kind::Container, "".into(), None)];
        for (slot, label, control) in [
            (1, "Check", Control::Checkbox(CheckState::Unchecked, false)),
            (2, "Switch", Control::Switch(false, false)),
            (3, "Disabled", Control::Button(true)),
        ] {
            operations.extend([
                Op::Create(
                    node(slot),
                    control.kind(),
                    label.into(),
                    Some(gpuio_protocol::HandlerId::from_parts(slot, 1).unwrap()),
                ),
                Op::SetControl(node(slot), control),
                Op::SetStyle(
                    node(slot),
                    vec![
                        Style::Fields(vec![
                            Field::Width(Length::Px(300.)),
                            Field::Height(Length::Px(45.)),
                            Field::PointerEvents(slot != 2),
                            Field::Foreground(Color::Rgba(0x111111ff)),
                        ]),
                        Style::State(1, vec![Field::Foreground(Color::Rgba(0x555555ff))]),
                        Style::State(4, vec![Field::Foreground(Color::Rgba(0x222222ff))]),
                        Style::State(5, vec![Field::Foreground(Color::Rgba(0x333333ff))]),
                        Style::State(6, vec![Field::Foreground(Color::Rgba(0x444444ff))]),
                    ],
                ),
            ]);
        }
        operations.extend([
            Op::Create(
                node(4),
                Kind::Input,
                "".into(),
                Some(gpuio_protocol::HandlerId::from_parts(4, 1).unwrap()),
            ),
            Op::SetEditor(
                node(4),
                EditorConfig {
                    label: "Editor".into(),
                    placeholder: "".into(),
                    read_only: false,
                    disabled: false,
                    submit_on_enter: true,
                    auto_focus: false,
                    min_rows: 1,
                    max_rows: 1,
                },
            ),
            Op::SetStyle(
                node(4),
                vec![Style::State(
                    1,
                    vec![Field::Foreground(Color::Rgba(0x555555ff))],
                )],
            ),
            Op::Splice(node(0), 0, 0, vec![node(1), node(2), node(3), node(4)]),
            Op::SetRoot(Some(node(0))),
        ]);
        let applied = session
            .borrow_mut()
            .apply(&Transaction {
                window: id,
                base: 0,
                revision: 1,
                operations,
            })
            .unwrap();
        let handle = cx
            .open_window(
                WindowOptions {
                    inactive_frame_interval: None,
                    window_bounds: Some(WindowBounds::Windowed(Bounds::centered(
                        None,
                        size(px(400.), px(280.)),
                        cx,
                    ))),
                    ..Default::default()
                },
                |_, cx| cx.new(|_| View::new(id, session, transport.clone())),
            )
            .unwrap();
        handle
            .update(cx, |view, window, cx| {
                view.update_editors(&applied.dirty, window, cx);
                window.activate_window();
                cx.notify();
            })
            .unwrap();
        cx.activate(true);
        cx.spawn(async move |cx| {
            let result = super::native_test::protect(exercise(cx, handle, transport)).await;
            *task_failure.borrow_mut() = result.err();
            cx.update(stop_application);
        })
        .detach();
    });
    if let Some(error) = failure.borrow_mut().take() {
        std::panic::resume_unwind(error);
    }
    println!(
        "GPUIO_CONTROLS_NATIVE_OK: activation, keyboard, Tab/Shift-Tab, pointer policy, native state styling, disable/re-enable and disposal"
    );
    #[cfg(target_os = "macos")]
    println!(
        "GPUIO_CONTROLS_MACOS_AX_OK: roles, mixed/checked/unchecked values, disabled state and focus/press actions"
    );
}
