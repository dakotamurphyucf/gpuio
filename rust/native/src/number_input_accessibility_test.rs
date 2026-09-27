//! Query and invoke the real AppKit accessibility objects for a mounted field.
use super::*;
use objc2::{
    class, msg_send,
    runtime::{AnyObject, Bool},
    sel,
};
use objc2_foundation::NSString;

#[derive(Debug, PartialEq)]
enum Value {
    Absent,
    Text(String),
    Number(f64),
}
#[derive(Debug)]
struct Accessible {
    value: Value,
    min: Option<f64>,
    max: Option<f64>,
    help: Option<String>,
    enabled: bool,
    required: bool,
    focused: bool,
    editable: bool,
    increment: bool,
    decrement: bool,
}
#[derive(Clone, Copy)]
enum Action<'a> {
    Read,
    Focus,
    Set(&'a str),
    Increment,
    Decrement,
    Press,
}

fn accessible(
    cx: &mut AsyncApp,
    handle: WindowHandle<View>,
    role: &str,
    label: &str,
    action: Action<'_>,
) -> Option<Accessible> {
    unsafe fn string(value: *mut NSString) -> Option<String> {
        unsafe { value.as_ref().map(|s| s.to_string()) }
    }
    unsafe fn number(value: *mut AnyObject) -> Option<f64> {
        if value.is_null() {
            None
        } else {
            Some(unsafe { msg_send![value, doubleValue] })
        }
    }
    unsafe fn visit(
        object: *mut AnyObject,
        role: &str,
        label: &str,
        action: Action<'_>,
        depth: usize,
    ) -> Option<Accessible> {
        if object.is_null() || depth > 24 {
            return None;
        }
        unsafe {
            let found_role: *mut NSString = msg_send![object, accessibilityRole];
            let title: *mut NSString = msg_send![object, accessibilityTitle];
            if string(found_role).as_deref() == Some(role)
                && string(title).as_deref() == Some(label)
            {
                let value: *mut AnyObject = msg_send![object, accessibilityValue];
                let value = if value.is_null() {
                    Value::Absent
                } else {
                    let is_text: Bool = msg_send![value, isKindOfClass: class!(NSString)];
                    if is_text.as_bool() {
                        Value::Text(string(value.cast()).unwrap())
                    } else {
                        Value::Number(number(value).unwrap())
                    }
                };
                let min: *mut AnyObject = msg_send![object, accessibilityMinValue];
                let max: *mut AnyObject = msg_send![object, accessibilityMaxValue];
                let help: *mut NSString = msg_send![object, accessibilityHelp];
                let enabled: Bool = msg_send![object, isAccessibilityEnabled];
                let required: Bool = msg_send![object, isAccessibilityRequired];
                let focused: Bool = msg_send![object, isAccessibilityFocused];
                let editable: Bool =
                    msg_send![object, isAccessibilitySelectorAllowed: sel!(setAccessibilityValue:)];
                let increment: Bool = msg_send![object, isAccessibilitySelectorAllowed: sel!(accessibilityPerformIncrement)];
                let decrement: Bool = msg_send![object, isAccessibilitySelectorAllowed: sel!(accessibilityPerformDecrement)];
                let result = Accessible {
                    value,
                    min: number(min),
                    max: number(max),
                    help: string(help),
                    enabled: enabled.as_bool(),
                    required: required.as_bool(),
                    focused: focused.as_bool(),
                    editable: editable.as_bool(),
                    increment: increment.as_bool(),
                    decrement: decrement.as_bool(),
                };
                match action {
                    Action::Read => (),
                    Action::Focus => {
                        let _: () = msg_send![object, setAccessibilityFocused: true];
                    }
                    Action::Set(text) => {
                        let text = NSString::from_str(text);
                        let _: () = msg_send![object, setAccessibilityValue: &*text];
                    }
                    Action::Increment => {
                        let _: Bool = msg_send![object, accessibilityPerformIncrement];
                    }
                    Action::Decrement => {
                        let _: Bool = msg_send![object, accessibilityPerformDecrement];
                    }
                    Action::Press => {
                        let _: Bool = msg_send![object, accessibilityPerformPress];
                    }
                }
                return Some(result);
            }
            let children: *mut AnyObject = msg_send![object, accessibilityChildren];
            if children.is_null() {
                return None;
            }
            let count: usize = msg_send![children, count];
            assert!(count < 512);
            for index in 0..count {
                let child: *mut AnyObject = msg_send![children, objectAtIndex: index];
                if let Some(found) = visit(child, role, label, action, depth + 1) {
                    return Some(found);
                }
            }
        }
        None
    }
    let view = super::super::super::editor_test::native_view(cx, handle) as *mut AnyObject;
    unsafe {
        let window: *mut AnyObject = msg_send![view, window];
        let content: *mut AnyObject = msg_send![window, contentView];
        visit(content, role, label, action, 0)
    }
}
async fn set(cx: &mut AsyncApp, handle: WindowHandle<View>, text: &str) {
    assert!(accessible(cx, handle, "AXTextField", "Temperature", Action::Set(text)).is_some());
    frame(cx, handle).await;
    assert_eq!(snapshot(cx, handle).draft, text);
}
fn reset(cx: &mut AsyncApp, handle: WindowHandle<View>, config: n::Config) {
    apply(
        cx,
        handle,
        vec![Op::SetNumberInput(node(1), config, n::Value::Empty)],
    );
    assert!(matches!(
        command(
            cx,
            handle,
            n::Command::ReplaceValue {
                value: n::Value::Number(1.5),
                selection: n::SelectionPolicy::End,
                undo: n::UndoPolicy::Reset,
                if_revision: None,
            }
        ),
        n::Response::Applied(_)
    ));
}

pub(super) async fn exercise(cx: &mut AsyncApp, handle: WindowHandle<View>, transport: &Transport) {
    reset(cx, handle, config());
    frame(cx, handle).await;
    // Reading the native tree activates AccessKit if this window was not queried yet.
    accessible(cx, handle, "AXTextField", "Temperature", Action::Read);
    frame(cx, handle).await;
    let field = accessible(cx, handle, "AXTextField", "Temperature", Action::Read)
        .expect("native numeric text field");
    assert_eq!(field.value, Value::Text("1.5".into()));
    assert!(field.enabled && field.required && field.editable);
    let stepper = accessible(cx, handle, "AXIncrementor", "Temperature", Action::Read)
        .expect("native numeric stepper");
    assert_eq!(stepper.value, Value::Number(1.5));
    assert_eq!((stepper.min, stepper.max), (Some(-2.), Some(8.)));
    assert!(stepper.increment && stepper.decrement && stepper.enabled);
    handle.update(cx, |_, w, cx| w.blur(cx)).unwrap();
    frame(cx, handle).await;
    accessible(cx, handle, "AXTextField", "Temperature", Action::Focus).unwrap();
    frame(cx, handle).await;
    assert!(
        accessible(cx, handle, "AXTextField", "Temperature", Action::Read)
            .unwrap()
            .focused
    );
    events(transport);
    for controls in [
        n::StepControls::Sides,
        n::StepControls::Stacked,
        n::StepControls::Hidden,
    ] {
        reset(
            cx,
            handle,
            n::Config {
                step_controls: controls,
                ..config()
            },
        );
        frame(cx, handle).await;
        accessible(
            cx,
            handle,
            "AXIncrementor",
            "Temperature",
            Action::Increment,
        )
        .unwrap();
        frame(cx, handle).await;
        assert_eq!(snapshot(cx, handle).committed, n::Value::Number(2.));
        accessible(
            cx,
            handle,
            "AXIncrementor",
            "Temperature",
            Action::Decrement,
        )
        .unwrap();
        frame(cx, handle).await;
        assert_eq!(snapshot(cx, handle).committed, n::Value::Number(1.5));
        if controls == n::StepControls::Hidden {
            assert!(
                accessible(cx, handle, "AXButton", "Increase temperature", Action::Read).is_none()
            );
        } else {
            accessible(
                cx,
                handle,
                "AXButton",
                "Increase temperature",
                Action::Press,
            )
            .unwrap();
            frame(cx, handle).await;
            assert_eq!(snapshot(cx, handle).committed, n::Value::Number(2.));
            accessible(
                cx,
                handle,
                "AXButton",
                "Decrease temperature",
                Action::Press,
            )
            .unwrap();
            frame(cx, handle).await;
            assert_eq!(snapshot(cx, handle).committed, n::Value::Number(1.5));
        }
        assert!(
            events(transport)
                .iter()
                .filter(|e| matches!(e, n::Event::Committed(n::Source::Accessibility, _)))
                .count()
                >= 2
        );
    }
    reset(cx, handle, config());
    frame(cx, handle).await;
    for (text, feedback) in [
        ("-", "Complete the number"),
        ("é", "Enter a number"),
        ("1e999", "Enter a finite number"),
        ("99", "limited to -2 through 8"),
    ] {
        set(cx, handle, text).await;
        let field = accessible(cx, handle, "AXTextField", "Temperature", Action::Read).unwrap();
        assert!(
            field
                .help
                .as_deref()
                .is_some_and(|help| help.contains(feedback)),
            "missing validation feedback: {field:?}"
        );
        assert_eq!(
            accessible(cx, handle, "AXIncrementor", "Temperature", Action::Read)
                .unwrap()
                .value,
            Value::Text(text.into())
        );
        assert_eq!(snapshot(cx, handle).committed, n::Value::Number(1.5));
    }
    accessible(
        cx,
        handle,
        "AXIncrementor",
        "Temperature",
        Action::Increment,
    )
    .unwrap();
    frame(cx, handle).await;
    assert_eq!(snapshot(cx, handle).committed, n::Value::Number(8.));
    assert!(
        accessible(cx, handle, "AXTextField", "Temperature", Action::Read)
            .unwrap()
            .help
            .is_none()
    );
    set(cx, handle, "").await;
    assert_eq!(
        accessible(cx, handle, "AXIncrementor", "Temperature", Action::Read)
            .unwrap()
            .value,
        Value::Text("".into())
    );
    events(transport);

    // Marked text is preserved and not announced as a syntax error mid-composition.
    native_text(cx, handle, "に", true);
    frame(cx, handle).await;
    let before = snapshot(cx, handle);
    assert!(before.composition.is_some());
    assert!(
        accessible(cx, handle, "AXTextField", "Temperature", Action::Read)
            .unwrap()
            .help
            .is_none()
    );
    accessible(cx, handle, "AXTextField", "Temperature", Action::Set("2")).unwrap();
    accessible(
        cx,
        handle,
        "AXIncrementor",
        "Temperature",
        Action::Increment,
    )
    .unwrap();
    frame(cx, handle).await;
    assert_eq!(snapshot(cx, handle).draft, before.draft);
    assert_eq!(snapshot(cx, handle).composition, before.composition);
    assert!(
        events(transport)
            .iter()
            .any(|e| matches!(e, n::Event::Rejected(n::Rejection::Composing, _)))
    );

    use gpuio_protocol::accessibility as a;
    apply(
        cx,
        handle,
        vec![Op::SetAccessibility(
            node(1),
            Some(a::Config {
                role: None,
                label: None,
                description: None,
                live: a::Live::Off,
                current: None,
                field: Some(a::Field {
                    label: "Sampling temperature".into(),
                    help: Some("Workspace setting\nAdjust generation".into()),
                    error: Some("Server unavailable".into()),
                    required: true,
                }),
            }),
        )],
    );
    frame(cx, handle).await;
    assert_eq!(snapshot(cx, handle).composition, before.composition);
    let field = accessible(
        cx,
        handle,
        "AXTextField",
        "Sampling temperature",
        Action::Read,
    )
    .unwrap();
    assert_eq!(
        field.help.as_deref(),
        Some("Workspace setting\nAdjust generation\nServer unavailable")
    );
    assert!(field.required);
    assert!(
        accessible(
            cx,
            handle,
            "AXIncrementor",
            "Sampling temperature",
            Action::Read
        )
        .is_some()
    );
    key(cx, handle, "escape");
    replace(cx, handle, "1e-");
    frame(cx, handle).await;
    assert!(
        accessible(
            cx,
            handle,
            "AXTextField",
            "Sampling temperature",
            Action::Read
        )
        .unwrap()
        .help
        .unwrap()
        .contains("Server unavailable\nComplete the number.")
    );
    apply(cx, handle, vec![Op::SetAccessibility(node(1), None)]);
    events(transport);

    for (disabled, read_only) in [(false, true), (true, false)] {
        reset(
            cx,
            handle,
            n::Config {
                disabled,
                read_only,
                allow_empty: true,
                ..config()
            },
        );
        frame(cx, handle).await;
        let field = accessible(cx, handle, "AXTextField", "Temperature", Action::Read).unwrap();
        assert!(!field.required && !field.editable);
        assert_eq!(field.enabled, !disabled);
        let stepper = accessible(
            cx,
            handle,
            "AXIncrementor",
            "Temperature",
            Action::Increment,
        )
        .unwrap();
        assert!(!stepper.increment && !stepper.decrement);
        accessible(cx, handle, "AXTextField", "Temperature", Action::Set("7")).unwrap();
        frame(cx, handle).await;
        assert_eq!(snapshot(cx, handle).draft, "1.5");
        assert_eq!(snapshot(cx, handle).committed, n::Value::Number(1.5));
    }
    apply(
        cx,
        handle,
        vec![Op::SetStyle(
            node(1),
            vec![Style::Fields(vec![Field::Display(3)])],
        )],
    );
    frame(cx, handle).await;
    assert!(accessible(cx, handle, "AXTextField", "Temperature", Action::Read).is_none());
    assert!(accessible(cx, handle, "AXIncrementor", "Temperature", Action::Read).is_none());
    events(transport);
    eprintln!(
        "GPUIO_NUMBER_AX_OK: actual AppKit roles/value types/ranges, focus/text/step/button actions, draft feedback, metadata/IME, read-only/disabled and hidden state"
    );
}
