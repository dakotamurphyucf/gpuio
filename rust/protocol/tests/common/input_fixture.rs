use gpuio_protocol::{
    input::*,
    pointer::{PointerButton as B, PointerModifiers},
};
pub fn config() -> Config {
    use Kind::*;
    Config {
        label: "Input café".into(),
        disabled: false,
        focus: gpuio_protocol::input::Focus::Tab,
        subscriptions: [
            Click,
            AuxiliaryClick,
            MouseDown,
            MouseUp,
            MouseMove,
            MouseEnter,
            MouseLeave,
            MouseDownOutside,
            KeyDown,
            KeyUp,
            Focus,
            Blur,
            Scroll,
        ]
        .into_iter()
        .map(|kind| Subscription {
            kind,
            phase: Phase::Bubble,
            policy: Policy::Observe,
        })
        .collect(),
    }
}
pub fn location() -> Location {
    Location {
        window: Position { x: 40.5, y: 70. },
        local: Position { x: -2., y: 7.25 },
        modifiers: PointerModifiers {
            shift: true,
            control: false,
            alt: true,
            command: false,
            function: true,
        },
    }
}
pub fn events() -> Vec<Event> {
    let mouse = Mouse {
        location: location(),
        button: B::Left,
        click_count: 2,
    };
    let key = Key {
        key: "é".into(),
        character: Some("é".into()),
        modifiers: location().modifiers,
    };
    let mut events = vec![
        Event::Click(mouse),
        Event::AuxiliaryClick(Mouse {
            button: B::Right,
            ..mouse
        }),
        Event::MouseDown(mouse),
        Event::MouseUp(mouse),
        Event::MouseMove(Motion {
            location: location(),
            pressed_button: None,
        }),
        Event::MouseEnter,
        Event::MouseLeave,
        Event::MouseDownOutside(mouse),
        Event::KeyDown(key.clone(), true),
        Event::KeyUp(key),
        Event::Focus,
        Event::Blur,
        Event::Scroll(Scroll {
            location: location(),
            delta: Delta::Pixels(Position { x: 0.5, y: -12. }),
            phase: TouchPhase::Moved,
        }),
    ];
    for button in [B::Left, B::Right, B::Middle, B::Back, B::Forward] {
        events.push(Event::MouseMove(Motion {
            location: location(),
            pressed_button: Some(button),
        }));
        events.push(Event::MouseDown(Mouse {
            button,
            click_count: u32::MAX as i64,
            ..mouse
        }));
    }
    for phase in [
        TouchPhase::Started,
        TouchPhase::Moved,
        TouchPhase::Ended,
        TouchPhase::Cancelled,
    ] {
        events.push(Event::Scroll(Scroll {
            location: location(),
            delta: Delta::Lines(Position { x: -1., y: 0. }),
            phase,
        }));
    }
    events.push(Event::KeyDown(
        Key {
            key: "space".into(),
            character: None,
            modifiers: PointerModifiers::default(),
        },
        false,
    ));
    events
}

pub fn policy_configs() -> Vec<Config> {
    let mut configs = Vec::new();
    for phase in [Phase::Capture, Phase::Bubble] {
        for policy in [
            Policy::Observe,
            Policy::StopPropagation,
            Policy::PreventDefault,
            Policy::PreventAndStop,
        ] {
            configs.push(Config {
                label: "Keys".into(),
                disabled: true,
                focus: Focus::None,
                subscriptions: vec![Subscription {
                    kind: Kind::KeyDown,
                    phase,
                    policy,
                }],
            });
        }
    }
    configs.push(Config {
        label: "Focus".into(),
        disabled: false,
        focus: Focus::Click,
        subscriptions: vec![Subscription {
            kind: Kind::Focus,
            phase: Phase::Bubble,
            policy: Policy::Observe,
        }],
    });
    configs
}
