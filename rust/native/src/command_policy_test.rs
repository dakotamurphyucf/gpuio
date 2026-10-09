use super::*;

fn shortcut(key: &str, modifiers: Vec<ShortcutModifier>) -> Shortcut {
    Shortcut {
        key: key.into(),
        modifiers,
        priority: ShortcutPriority::NativeFirst,
        text_input: ShortcutTextInput::ModifiedOnly,
        during_composition: false,
    }
}

const BUTTON: InputContext = InputContext {
    editing: false,
    composing: false,
    focused_kind: Some(Kind::Button),
};
const EDITOR: InputContext = InputContext {
    editing: true,
    composing: false,
    focused_kind: Some(Kind::Input),
};

#[test]
fn traversal_and_activation_reservations_are_phase_and_control_specific() {
    for key in ["tab", "enter", "space"] {
        let mut binding = shortcut(key, vec![]);
        let stroke = keystroke(&binding);
        assert_eq!(
            BUTTON.gate(&binding, &stroke),
            Some(InputGate::NativeNavigation)
        );
        binding.priority = ShortcutPriority::Override;
        assert_eq!(BUTTON.gate(&binding, &stroke), None);
    }
    for kind in [
        Kind::Button,
        Kind::CommandButton,
        Kind::Checkbox,
        Kind::Switch,
    ] {
        assert!(InputContext::reserves_navigation(
            &Keystroke::parse("enter").unwrap(),
            Some(kind)
        ));
    }
    assert!(!InputContext::reserves_navigation(
        &Keystroke::parse("enter").unwrap(),
        Some(Kind::Input)
    ));
    for kind in [None, Some(Kind::Container), Some(Kind::Input)] {
        assert!(InputContext::reserves_navigation(
            &Keystroke::parse("shift-tab").unwrap(),
            kind
        ));
        assert!(!InputContext::reserves_navigation(
            &Keystroke::parse("ctrl-tab").unwrap(),
            kind
        ));
        assert!(!InputContext::reserves_navigation(
            &Keystroke::parse("alt-enter").unwrap(),
            kind
        ));
    }
}

#[test]
fn editing_and_composition_filters_do_not_confuse_shift_with_a_command_modifier() {
    let mut binding = shortcut("x", vec![]);
    assert_eq!(
        EDITOR.gate(&binding, &keystroke(&binding)),
        Some(InputGate::TextInput)
    );
    binding.modifiers = vec![ShortcutModifier::Shift];
    assert_eq!(
        EDITOR.gate(&binding, &keystroke(&binding)),
        Some(InputGate::TextInput)
    );
    for modifier in [
        ShortcutModifier::Primary,
        ShortcutModifier::Control,
        ShortcutModifier::Alt,
        ShortcutModifier::Super,
    ] {
        binding.modifiers = vec![modifier];
        assert_eq!(EDITOR.gate(&binding, &keystroke(&binding)), None);
    }
    binding.text_input = ShortcutTextInput::Never;
    assert_eq!(
        EDITOR.gate(&binding, &keystroke(&binding)),
        Some(InputGate::TextInput)
    );
    assert_eq!(BUTTON.gate(&binding, &keystroke(&binding)), None);
    binding.text_input = ShortcutTextInput::Always;
    binding.modifiers.clear();
    assert_eq!(EDITOR.gate(&binding, &keystroke(&binding)), None);
    let composing = InputContext {
        composing: true,
        ..EDITOR
    };
    assert_eq!(
        composing.gate(&binding, &keystroke(&binding)),
        Some(InputGate::Composition)
    );
    binding.during_composition = true;
    assert_eq!(composing.gate(&binding, &keystroke(&binding)), None);
    binding.text_input = ShortcutTextInput::Never;
    assert_eq!(
        composing.gate(&binding, &keystroke(&binding)),
        Some(InputGate::TextInput)
    );
}

#[test]
fn physical_aliases_and_priority_use_the_native_matcher() {
    let binding = shortcut("k", vec![ShortcutModifier::Primary]);
    let physical = if cfg!(target_os = "macos") {
        "cmd-k"
    } else {
        "ctrl-k"
    };
    let key = Keystroke::parse(physical).unwrap();
    assert!(EDITOR.matches(&binding, &key, ShortcutPriority::NativeFirst));
    assert!(!EDITOR.matches(&binding, &key, ShortcutPriority::Override));
    assert!(!EDITOR.matches(
        &binding,
        &Keystroke::parse("alt-k").unwrap(),
        ShortcutPriority::NativeFirst
    ));
    let alias = if cfg!(target_os = "macos") {
        ShortcutModifier::Super
    } else {
        ShortcutModifier::Control
    };
    let combined = shortcut("k", vec![ShortcutModifier::Primary, alias]);
    assert_eq!(
        keystroke(&binding).modifiers,
        keystroke(&combined).modifiers
    );
    assert!(EDITOR.matches(&combined, &key, ShortcutPriority::NativeFirst));
}
