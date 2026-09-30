use binprot::BinProtWrite;
use gpuio_native::{
    mailbox::{MAX_INPUT_EVENTS, Mailbox},
    session::Session,
};
use gpuio_protocol::{
    HandlerId, NodeId, ResourceId, WindowId, document, document_diff as diff, v1::*,
};
fn window() -> WindowId {
    WindowId::from_parts(0, 1).unwrap()
}
fn node() -> NodeId {
    NodeId::from_parts(0, 1).unwrap()
}
fn handler() -> HandlerId {
    HandlerId::from_parts(0, 1).unwrap()
}
fn apply(s: &mut Session, operations: Vec<Op>) -> Result<(), ErrorCode> {
    let base = s.tree(window()).unwrap().revision();
    s.apply(&Transaction {
        window: window(),
        base,
        revision: base + 1,
        operations,
    })
    .map(|_| ())
}
fn config(source: ResourceId, mode: document::Mode) -> document::Config {
    document::Config {
        source: Some(source),
        mode,
        dark: false,
        layout: document::Layout::Flow,
        label: "Diff".into(),
        path: None,
        line_numbers: true,
        initially_collapsed: false,
        search: String::new(),
        images: vec![],
    }
}
fn publish(s: &mut Session, id: ResourceId, base: i64, generation: i64) {
    assert_eq!(
        s.document_request(document::Request::Begin(document::Update {
            id,
            base,
            revision: base + 1,
            generation,
            from_byte: 0,
            suffix_bytes: 0,
            status: document::Status::Streaming
        })),
        document::Response::Ack
    );
    assert_eq!(
        s.document_request(document::Request::Publish(id, base + 1)),
        document::Response::Ack
    );
}
fn sample(epoch: i64, revision: i64, generation: i64) -> diff::Event {
    diff::Event {
        config_epoch: epoch,
        source_revision: revision,
        source_generation: generation,
        observation: diff::Observation::ShowMore {
            visible: 0,
            hidden: 1,
            applied_limit: Some(200),
        },
    }
}
fn setup() -> (Session, ResourceId) {
    let mut s = Session::default();
    s.hello(VERSION, CAPABILITIES).unwrap();
    s.open(1, window(), "diff", 300., 200.).unwrap();
    let document::Response::Created(id) = s.document_request(document::Request::Create) else {
        panic!("source");
    };
    publish(&mut s, id, 0, 1);
    apply(
        &mut s,
        vec![
            Op::Create(node(), Kind::DocumentView, String::new(), Some(handler())),
            Op::SetDocument(node(), config(id, document::Mode::Diff)),
            Op::SetDocumentDiff(node(), 1, Some(diff::Config::default())),
            Op::SetRoot(Some(node())),
        ],
    )
    .unwrap();
    (s, id)
}

#[test]
fn atomic_type_epoch_and_configuration_validation() {
    let (mut s, source) = setup();
    let revision = s.tree(window()).unwrap().revision();
    let retained = s.retained_bytes();
    for operations in [
        vec![Op::SetDocumentDiff(
            node(),
            1,
            Some(diff::Config::default()),
        )],
        vec![Op::SetDocumentDiff(node(), 0, None)],
        vec![Op::SetDocument(
            node(),
            config(source, document::Mode::Markdown),
        )],
        vec![Op::SetDocumentDiff(
            node(),
            2,
            Some(diff::Config {
                line_limit: diff::LineLimit::Controlled(Some(-1)),
                ..diff::Config::default()
            }),
        )],
    ] {
        assert_eq!(apply(&mut s, operations), Err(ErrorCode::InvalidTree));
        assert_eq!(s.tree(window()).unwrap().revision(), revision);
        assert_eq!(s.retained_bytes(), retained);
    }
    let large = diff::Config {
        collapse: diff::Collapse::Controlled(vec![diff::FileKey::Path("x".repeat(4096))]),
        ..diff::Config::default()
    };
    apply(&mut s, vec![Op::SetDocumentDiff(node(), 2, Some(large))]).unwrap();
    assert!(s.retained_bytes() >= retained + 4096);
    apply(
        &mut s,
        vec![
            Op::SetDocumentDiff(node(), 3, None),
            Op::SetDocument(node(), config(source, document::Mode::Markdown)),
        ],
    )
    .unwrap();
    assert!(
        s.tree(window())
            .unwrap()
            .get(node())
            .unwrap()
            .document_diff
            .is_none()
    );
    assert!(
        s.document_diff_event(window(), node(), handler(), source, sample(2, 1, 1))
            .is_none()
    );
    s.close(window()).unwrap();
    assert_eq!(s.retained_bytes(), 0);
}

#[test]
fn native_events_fence_handler_source_epoch_generation_and_future_revision() {
    let (mut s, source) = setup();
    let accept =
        |s: &Session, event| s.document_diff_event(window(), node(), handler(), source, event);
    assert!(accept(&s, sample(1, 1, 1)).is_some());
    assert!(accept(&s, sample(2, 1, 1)).is_none());
    assert!(accept(&s, sample(1, 2, 1)).is_none());
    assert!(accept(&s, sample(1, 1, 2)).is_none());
    assert!(
        s.document_diff_event(
            window(),
            node(),
            HandlerId::from_parts(0, 2).unwrap(),
            source,
            sample(1, 1, 1)
        )
        .is_none()
    );
    assert!(
        s.document_diff_event(
            window(),
            node(),
            handler(),
            ResourceId::from_parts(0, 2).unwrap(),
            sample(1, 1, 1)
        )
        .is_none()
    );
    publish(&mut s, source, 1, 1);
    assert!(
        accept(&s, sample(1, 1, 1)).is_some(),
        "older installed picture may still be visible"
    );
    publish(&mut s, source, 2, 2);
    assert!(accept(&s, sample(1, 1, 1)).is_none());
    assert!(
        accept(&s, sample(1, 1, 2)).is_none(),
        "revision predates this source generation"
    );
    assert!(accept(&s, sample(1, 3, 2)).is_some());
    apply(
        &mut s,
        vec![Op::SetDocumentDiff(
            node(),
            2,
            Some(diff::Config::default()),
        )],
    )
    .unwrap();
    assert!(accept(&s, sample(1, 3, 2)).is_none());
    assert!(accept(&s, sample(2, 3, 2)).is_some());
    assert_eq!(
        s.document_request(document::Request::Release(source)),
        document::Response::Ack
    );
    assert!(accept(&s, sample(2, 3, 2)).is_none());
}

#[test]
fn queued_diff_payloads_are_bounded_ordered_and_fitted_to_drain_batches() {
    let source = ResourceId::from_parts(0, 1).unwrap();
    let path = "x".repeat(4096);
    let mut q = Mailbox::default();
    for epoch in 1..=MAX_INPUT_EVENTS {
        let event = diff::Event {
            config_epoch: epoch as i64,
            source_revision: 1,
            source_generation: 1,
            observation: diff::Observation::Line(diff::Line {
                file: diff::File {
                    index: 0,
                    key: diff::FileKey::Path(path.clone()),
                    before_path: Some(path.clone()),
                    after_path: Some(path.clone()),
                },
                before: Some(1),
                after: Some(1),
                start_byte: 0,
                end_byte: 16384,
                text: "a".repeat(16384),
            }),
        };
        assert_eq!(event.payload_bytes(), 4096 * 3 + 16384);
        q.input(Event::DocumentDiffEvent(
            window(),
            node(),
            handler(),
            1,
            source,
            event,
        ))
        .unwrap();
    }
    assert!(
        q.input(Event::DocumentDiffEvent(
            window(),
            node(),
            handler(),
            1,
            source,
            sample(1, 1, 1)
        ))
        .is_err()
    );
    assert!(q.has_window_output(0));
    let mut next_epoch = 1;
    let mut batches = 0;
    while q.has_output() {
        let events = q.drain(128);
        assert!(!events.is_empty());
        let mut bytes = Vec::new();
        events.binprot_write(&mut bytes).unwrap();
        assert!(bytes.len() <= MAX_MESSAGE_BYTES);
        for event in events {
            let Event::DocumentDiffEvent(_, _, _, _, _, event) = event else {
                panic!("diff event");
            };
            assert_eq!(event.config_epoch, next_epoch);
            next_epoch += 1;
        }
        batches += 1;
    }
    assert_eq!(next_epoch, MAX_INPUT_EVENTS as i64 + 1);
    assert!(batches > 1);
    assert!(!q.has_window_output(0));
}
