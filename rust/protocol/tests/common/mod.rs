use gpuio_protocol::{HandlerId, NodeId, WindowId, v1::*};

pub fn request() -> Message {
    let window = WindowId::from_parts(0, 1).unwrap();
    let root = NodeId::from_parts(0, 1).unwrap();
    let child = NodeId::from_parts(1, 2).unwrap();
    let handler = HandlerId::from_parts(2, 3).unwrap();
    Message::Apply(Transaction {
        window,
        base: 127,
        revision: 128,
        operations: vec![
            Op::Create(root, Kind::Container, String::new(), None),
            Op::Create(child, Kind::Button, "λ 🦀".into(), Some(handler)),
            Op::SetStyle(
                root,
                vec![
                    Style::Width(Length::Percent(100.)),
                    Style::Padding(12.),
                    Style::Background(Color::Rgba(0x112233ff)),
                    Style::HoverBackground(Color::Token(1)),
                    Style::Opacity(0.75),
                ],
            ),
            Op::Splice(root, 0, 0, vec![child]),
            Op::SetRoot(Some(root)),
        ],
    })
}

pub fn events() -> Vec<Event> {
    let window = WindowId::from_parts(0, 1).unwrap();
    vec![
        Event::Welcome(VERSION, 7),
        Event::Opened(1, window),
        Event::Accepted(window, 128),
        Event::Rendered(window, 128),
        Event::Press(
            window,
            NodeId::from_parts(1, 2).unwrap(),
            HandlerId::from_parts(2, 3).unwrap(),
            128,
        ),
        Event::Rejected(window, 129, ErrorCode::InvalidTree),
        Event::FrameRequested(9, window, 128),
        Event::Failed(10, ErrorCode::Busy),
        Event::Overloaded(window),
        Event::Closed(11, window),
        Event::Stopped,
    ]
}
