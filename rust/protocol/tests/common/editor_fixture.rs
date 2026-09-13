use gpuio_protocol::{HandlerId, NodeId, WindowId, v1::*};
pub fn requests() -> Vec<Message> {
    let window = WindowId::from_parts(3, 2).unwrap();
    let node = NodeId::from_parts(4, 3).unwrap();
    let mut messages = vec![
        Message::Hello(VERSION, CAPABILITIES),
        Message::Apply(Transaction {
            window,
            base: 127,
            revision: 128,
            operations: vec![
                Op::Create(
                    node,
                    Kind::Input,
                    "é界".into(),
                    Some(HandlerId::from_parts(5, 4).unwrap()),
                ),
                Op::Create(
                    NodeId::from_parts(6, 1).unwrap(),
                    Kind::Textarea,
                    "".into(),
                    None,
                ),
                Op::SetEditor(
                    node,
                    EditorConfig {
                        label: "Nom".into(),
                        placeholder: "écrire".into(),
                        read_only: true,
                        disabled: false,
                        submit_on_enter: true,
                        auto_focus: false,
                        min_rows: 1,
                        max_rows: 1,
                    },
                ),
            ],
        }),
    ];
    let selection = EditorSelection { anchor: 5, head: 2 };
    let commands = vec![
        EditorCommand::Replace(
            "é界".into(),
            EditorSelectionPolicy::Start,
            EditorUndoPolicy::Record,
            None,
        ),
        EditorCommand::Replace(
            "é界".into(),
            EditorSelectionPolicy::End,
            EditorUndoPolicy::Reset,
            Some(128),
        ),
        EditorCommand::Replace(
            "é界".into(),
            EditorSelectionPolicy::Preserve,
            EditorUndoPolicy::Record,
            Some(127),
        ),
        EditorCommand::Replace(
            "é界".into(),
            EditorSelectionPolicy::Select(selection.clone()),
            EditorUndoPolicy::Reset,
            None,
        ),
        EditorCommand::Select(selection),
        EditorCommand::Focus,
        EditorCommand::Undo,
        EditorCommand::Redo,
    ];
    messages.extend(
        commands
            .into_iter()
            .enumerate()
            .map(|(i, command)| Message::EditorCommand(128 + i as i64, window, node, command)),
    );
    messages
}
pub fn events() -> Vec<Event> {
    let window = WindowId::from_parts(3, 2).unwrap();
    let node = NodeId::from_parts(4, 3).unwrap();
    let handler = HandlerId::from_parts(5, 4).unwrap();
    let snapshot = EditorSnapshot {
        revision: 128,
        text: "é界".into(),
        selection: EditorSelection { anchor: 5, head: 2 },
        composition: None,
        focused: true,
    };
    let mut marked = snapshot.clone();
    marked.composition = Some(EditorSelection { anchor: 0, head: 2 });
    let mut events = vec![
        Event::EditorEvent(window, node, handler, 128, EditorEventKind::Changed, marked),
        Event::EditorEvent(
            window,
            node,
            handler,
            128,
            EditorEventKind::Submitted,
            snapshot.clone(),
        ),
        Event::EditorResult(128, window, node, EditorResult::Applied(snapshot)),
    ];
    events.extend(
        [
            EditorError::NotMounted,
            EditorError::Closed,
            EditorError::StaleEditor,
            EditorError::StaleRevision,
            EditorError::Composing,
            EditorError::InvalidSelection,
            EditorError::LimitExceeded,
            EditorError::Busy,
            EditorError::NativeFailure,
            EditorError::InvalidText,
        ]
        .into_iter()
        .enumerate()
        .map(|(i, error)| {
            Event::EditorResult(129 + i as i64, window, node, EditorResult::Failed(error))
        }),
    );
    events
}
