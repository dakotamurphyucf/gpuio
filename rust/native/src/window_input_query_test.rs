//! Metadata queries on retained production widgets; no OS desktop claim.
use super::*;
use crate::session::Session;
use gpui::{Entity, TestAppContext, VisualTestContext};
use gpuio_protocol::{HandlerId, number_input as n, otp_input as o};
use std::os::{fd::AsRawFd, unix::net::UnixStream};
fn id(slot: i64) -> NodeId {
    NodeId::from_parts(slot, 1).unwrap()
}
fn editor_config(read_only: bool, disabled: bool) -> EditorConfig {
    EditorConfig {
        label: "Private input".into(),
        placeholder: "".into(),
        read_only,
        disabled,
        submit_on_enter: false,
        auto_focus: false,
        min_rows: 1,
        max_rows: 1,
    }
}
pub(super) fn mount(app: &mut TestAppContext) -> (Entity<View>, VisualTestContext, UnixStream) {
    app.update(gpui_base::init);
    let (reader, writer) = UnixStream::pair().unwrap();
    let transport = Arc::new(Transport::new(writer.as_raw_fd()).unwrap());
    let session = Rc::new(RefCell::new(Session::default()));
    let window = WindowId::from_parts(0, 1).unwrap();
    session.borrow_mut().hello(VERSION, CAPABILITIES).unwrap();
    session
        .borrow_mut()
        .open(1, window, "Query", 800., 600.)
        .unwrap();
    let native = app.update(|cx| {
        cx.open_window(WindowOptions::default(), |_, cx| {
            cx.new(|_| View::new(window, session.clone(), transport.clone()))
        })
        .unwrap()
    });
    let owner = native.root(app).unwrap();
    let cx = VisualTestContext::from_window(native.into(), app);
    cx.simulate_resize(size(px(800.), px(600.)));
    (owner, cx, reader)
}
pub(super) fn draw(cx: &mut VisualTestContext) {
    cx.run_until_parked();
    cx.update(|w, cx| w.draw(cx).clear(cx));
    cx.run_until_parked();
}
pub(super) fn apply(owner: &Entity<View>, cx: &mut VisualTestContext, operations: Vec<Op>) {
    cx.update(|w, cx| {
        owner.update(cx, |view, cx| {
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
                .unwrap_or_else(|error| panic!("{error:?} at {transaction:?}"));
            view.update_editors(&applied.dirty, w, cx);
            cx.notify();
        })
    });
    draw(cx);
}
fn query(owner: &Entity<View>, cx: &mut VisualTestContext) -> Option<wire::Input> {
    cx.update(|w, cx| {
        owner.update(cx, |view, cx| {
            let wire::Response::FocusedInput(input) =
                request(view, &wire::Command::FocusedInput, w, cx)
            else {
                panic!("wrong response")
            };
            input
        })
    })
}
fn install(kind: Kind) -> Vec<Op> {
    let mut operations = vec![Op::Create(
        id(0),
        kind,
        if matches!(kind, Kind::Input | Kind::Textarea | Kind::Combobox) {
            "private λ".into()
        } else {
            "".into()
        },
        Some(HandlerId::from_parts(0, 1).unwrap()),
    )];
    match kind {
        Kind::Input | Kind::Textarea | Kind::Combobox => {
            operations.push(Op::SetEditor(id(0), editor_config(false, false)))
        }
        Kind::OtpInput => operations.push(Op::SetOtpInput(
            id(0),
            o::Config {
                policy: o::Policy::new(6, o::Alphabet::Digits).unwrap(),
                label: "OTP".into(),
                masked: true,
                disabled: false,
                read_only: false,
                auto_focus: false,
            },
            "123456".into(),
        )),
        Kind::NumberInput => operations.push(Op::SetNumberInput(
            id(0),
            n::Config {
                domain: gpuio_protocol::numeric::Domain::new(0., 10., 1.).unwrap(),
                label: "Number".into(),
                placeholder: "".into(),
                increment_label: "Increase".into(),
                decrement_label: "Decrease".into(),
                step_controls: n::StepControls::Sides,
                allow_empty: false,
                disabled: false,
                read_only: false,
                auto_focus: false,
            },
            n::Value::Number(2.),
        )),
        Kind::CommandPalette => operations.push(Op::SetPalette(
            id(0),
            PaletteConfig {
                label: "Commands".into(),
                placeholder: "Search".into(),
                commands: vec![],
                dismiss_on_outside_pointer: true,
            },
        )),
        Kind::ColorInput => operations.push(Op::SetColorInput(
            id(0),
            Box::new(gpuio_protocol::color_input::Config {
                labels: gpuio_protocol::color_input::Labels {
                    control: "Color".into(),
                    hue: "Hue".into(),
                    saturation: "Saturation".into(),
                    lightness: "Lightness".into(),
                    alpha: "Alpha".into(),
                    hex: "Hex".into(),
                    clear: "Clear".into(),
                },
                palette: vec![],
                alpha_policy: gpuio_protocol::color_value::AlphaPolicy::AllowAlpha,
                allow_empty: true,
                disabled: false,
                read_only: false,
            }),
            gpuio_protocol::color_value::Value::Color(gpuio_protocol::color_value::Rgba::new(
                10, 20, 30, 255,
            )),
        )),
        _ => unreachable!(),
    }
    if kind == Kind::Combobox {
        operations.push(Op::SetComboboxFilter(id(0), ComboboxFilter::Substring));
        operations.push(Op::SetChoice(
            id(0),
            ChoiceConfig {
                label: "Private input".into(),
                items: vec![],
                selected: None,
                disabled: false,
            },
        ));
    }
    if kind == Kind::Input {
        operations.push(Op::SetEditorPrivacy(id(0), EditorPrivacy::PasswordHidden));
    }
    operations.extend([
        Op::SetStyle(
            id(0),
            vec![
                Style::Width(Length::Px(400.)),
                Style::Height(Length::Px(120.)),
            ],
        ),
        Op::SetRoot(Some(id(0))),
    ]);
    operations
}
#[test]
fn eligible_native_inputs_are_identified_without_copying_values() {
    use binprot::BinProtWrite;
    for (kind, expected) in [
        (Kind::Input, wire::InputKind::Input),
        (Kind::Textarea, wire::InputKind::Textarea),
        (Kind::Combobox, wire::InputKind::Combobox),
        (Kind::OtpInput, wire::InputKind::Otp),
        (Kind::NumberInput, wire::InputKind::Number),
        (Kind::CommandPalette, wire::InputKind::CommandPalette),
    ] {
        let mut app = TestAppContext::single();
        let (owner, mut cx, _reader) = mount(&mut app);
        apply(&owner, &mut cx, install(kind));
        cx.update(|window, cx| {
            let view = owner.read(cx);
            let handle = match kind {
                Kind::OtpInput => view.otps[&id(0)].focus_handle(cx),
                Kind::NumberInput => view.numbers[&id(0)].focus_handle(cx),
                Kind::CommandPalette => view.palettes[&id(0)].query.read(cx).focus_handle(cx),
                _ => view.editors[&id(0)].focus_handle(cx),
            };
            window.focus(&handle, cx);
        });
        draw(&mut cx);
        let result = query(&owner, &mut cx).expect("focused native input");
        assert_eq!(
            result,
            wire::Input {
                node: id(0),
                kind: expected
            }
        );
        let mut bytes = vec![];
        result.binprot_write(&mut bytes).unwrap();
        assert_eq!(
            bytes.len(),
            3,
            "metadata only, regardless of private value length"
        );
        cx.update(|w, cx| w.blur(cx));
        draw(&mut cx);
        assert_eq!(query(&owner, &mut cx), None);
    }
}
#[test]
fn disabled_hidden_and_removed_owners_do_not_report_stale_focus() {
    let mut app = TestAppContext::single();
    let (owner, mut cx, _reader) = mount(&mut app);
    apply(&owner, &mut cx, install(Kind::Input));
    let focus = cx.update(|_, cx| owner.read(cx).editors[&id(0)].focus_handle(cx));
    cx.update(|w, cx| w.focus(&focus, cx));
    draw(&mut cx);
    apply(
        &owner,
        &mut cx,
        vec![Op::SetEditor(id(0), editor_config(true, false))],
    );
    assert!(
        query(&owner, &mut cx).is_some(),
        "read-only remains a text input"
    );
    apply(
        &owner,
        &mut cx,
        vec![Op::SetEditor(id(0), editor_config(false, true))],
    );
    cx.update(|w, cx| w.focus(&focus, cx));
    assert_eq!(query(&owner, &mut cx), None);
    apply(
        &owner,
        &mut cx,
        vec![
            Op::SetEditor(id(0), editor_config(false, false)),
            Op::SetStyle(id(0), vec![Style::Fields(vec![Field::Display(3)])]),
        ],
    );
    cx.update(|w, cx| w.focus(&focus, cx));
    assert_eq!(query(&owner, &mut cx), None);
    apply(&owner, &mut cx, vec![Op::SetRoot(None), Op::Remove(id(0))]);
    cx.update(|w, cx| w.focus(&focus, cx));
    assert_eq!(query(&owner, &mut cx), None);
    let replacement = NodeId::from_parts(0, 2).unwrap();
    apply(
        &owner,
        &mut cx,
        vec![
            Op::Create(
                replacement,
                Kind::Input,
                "replacement".into(),
                Some(HandlerId::from_parts(0, 2).unwrap()),
            ),
            Op::SetEditor(replacement, editor_config(false, false)),
            Op::SetRoot(Some(replacement)),
        ],
    );
    cx.update(|w, cx| w.focus(&focus, cx));
    assert_eq!(
        query(&owner, &mut cx),
        None,
        "old native handle cannot alias the reused node slot"
    );
    cx.update(|w, cx| w.focus(&owner.read(cx).editors[&replacement].focus_handle(cx), cx));
    draw(&mut cx);
    assert_eq!(
        query(&owner, &mut cx),
        Some(wire::Input {
            node: replacement,
            kind: wire::InputKind::Input
        })
    );
}

#[test]
fn composite_color_text_fields_are_inputs_but_sliders_and_buttons_are_not() {
    let mut app = TestAppContext::single();
    let (owner, mut cx, _reader) = mount(&mut app);
    let mut operations = install(Kind::ColorInput);
    operations.push(Op::SetStyle(
        id(0),
        vec![
            Style::Width(Length::Px(700.)),
            Style::Height(Length::Px(500.)),
        ],
    ));
    apply(&owner, &mut cx, operations);
    let mut inputs = 0;
    let mut other = 0;
    for _ in 0..24 {
        cx.simulate_keystrokes("tab");
        draw(&mut cx);
        match query(&owner, &mut cx) {
            Some(input) => {
                assert_eq!(
                    input,
                    wire::Input {
                        node: id(0),
                        kind: wire::InputKind::Color
                    }
                );
                inputs += 1;
            }
            None => other += 1,
        }
    }
    assert!(
        inputs > 0 && other > 0,
        "must traverse both text fields and non-text color controls: {inputs}/{other}"
    );
}
#[test]
fn modal_scope_blocks_old_input_even_if_its_native_focus_handle_is_retained() {
    let mut app = TestAppContext::single();
    let (owner, mut cx, _reader) = mount(&mut app);
    apply(&owner, &mut cx, install(Kind::Input));
    let focus = cx.update(|_, cx| owner.read(cx).editors[&id(0)].focus_handle(cx));
    apply(
        &owner,
        &mut cx,
        vec![
            Op::Create(id(1), Kind::Container, "".into(), None),
            Op::Create(id(2), Kind::FocusScope, "".into(), None),
            Op::SetFocusScope(
                id(2),
                FocusScopeConfig {
                    trap: true,
                    auto_focus: true,
                    restore_focus: true,
                },
            ),
            Op::Create(
                id(3),
                Kind::Button,
                "Modal button".into(),
                Some(HandlerId::from_parts(2, 1).unwrap()),
            ),
            Op::SetControl(id(3), Control::Button(false)),
            Op::Splice(id(2), 0, 0, vec![id(3)]),
            Op::Splice(id(1), 0, 0, vec![id(0), id(2)]),
            Op::SetRoot(Some(id(1))),
        ],
    );
    cx.update(|w, cx| w.focus(&focus, cx));
    assert_eq!(query(&owner, &mut cx), None);
}
