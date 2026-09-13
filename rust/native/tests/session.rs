use gpuio_native::{
    mailbox::{MAX_COMMANDS, MAX_INPUT_EVENTS, Mailbox},
    session::Session,
};
use gpuio_protocol::{HandlerId, NodeId, WindowId, v1::*};
fn window(g: i64) -> WindowId {
    WindowId::from_parts(0, g).unwrap()
}
fn transaction() -> Transaction {
    Transaction {
        window: window(1),
        base: 0,
        revision: 1,
        operations: vec![
            Op::Create(
                NodeId::from_parts(0, 1).unwrap(),
                Kind::Button,
                "click".into(),
                Some(HandlerId::from_parts(0, 1).unwrap()),
            ),
            Op::SetRoot(Some(NodeId::from_parts(0, 1).unwrap())),
        ],
    }
}
#[test]
fn negotiation_closed_generations_and_render_requests() {
    let mut session = Session::default();
    assert_eq!(
        session.validate_open(window(1), "one", 100., 100.),
        Err(ErrorCode::NotReady)
    );
    assert_eq!(session.hello(2, 0), Err(ErrorCode::UnsupportedVersion));
    assert_eq!(
        session.hello(VERSION, 32),
        Err(ErrorCode::UnsupportedCapability)
    );
    assert_eq!(
        session.hello(VERSION, CAPABILITIES),
        Ok(Event::Welcome(VERSION, CAPABILITIES))
    );
    session.open(1, window(1), "one", 100., 100.).unwrap();
    session.apply(&transaction()).unwrap();
    session.request_frame(2, window(1)).unwrap();
    assert_eq!(session.request_frame(3, window(1)), Err(ErrorCode::Busy));
    assert_eq!(session.painted(window(1), 0), vec![]);
    assert_eq!(
        session.painted(window(1), 1),
        vec![
            Event::Rendered(window(1), 1),
            Event::FrameRequested(2, window(1), 1)
        ]
    );
    assert!(session.painted(window(1), 1).is_empty());
    session.request_frame(4, window(1)).unwrap();
    assert_eq!(session.close(window(1)), Ok(Some(4)));
    assert_eq!(session.retained_bytes(), 0);
    assert_eq!(
        session.validate_open(window(1), "old", 100., 100.),
        Err(ErrorCode::StaleHandle)
    );
    session.open(5, window(2), "new", 100., 100.).unwrap();
    assert_eq!(
        session.press(
            window(1),
            NodeId::from_parts(0, 1).unwrap(),
            HandlerId::from_parts(0, 1).unwrap(),
            1
        ),
        None
    );
    assert!(session.painted(window(1), 1).is_empty());
    assert_eq!(session.shutdown(), vec![Event::Stopped]);
    assert_eq!(session.hello(VERSION, 0), Err(ErrorCode::Closed));
}
#[test]
fn submissions_reserve_responses_and_ack_drain_releases_window() {
    let mut mailbox = Mailbox::default();
    mailbox.submit(Message::Apply(transaction()), 100).unwrap();
    assert_eq!(
        mailbox.submit(Message::Apply(transaction()), 100),
        Err(ErrorCode::Busy)
    );
    assert!(matches!(mailbox.pop(), Some(Message::Apply(_))));
    mailbox.respond(Event::Accepted(window(1), 1));
    assert_eq!(
        mailbox.submit(Message::Apply(transaction()), 100),
        Err(ErrorCode::Busy)
    );
    assert_eq!(mailbox.drain(1), vec![Event::Accepted(window(1), 1)]);
    mailbox.submit(Message::Apply(transaction()), 100).unwrap();
    for _ in 1..MAX_COMMANDS {
        mailbox.submit(Message::Hello(VERSION, 0), 1).unwrap();
    }
    assert_eq!(mailbox.submit(Message::Shutdown, 1), Err(ErrorCode::Busy));
    for _ in 0..MAX_COMMANDS {
        mailbox.pop().unwrap();
        mailbox.respond(Event::Failed(0, ErrorCode::NotReady));
    }
    assert_eq!(mailbox.drain(256).len(), MAX_COMMANDS);
    mailbox.close();
    assert_eq!(mailbox.submit(Message::Shutdown, 1), Err(ErrorCode::Closed));
}
#[test]
fn input_pressure_has_ordered_explicit_overload_and_coalescing_barriers() {
    let mut mailbox = Mailbox::default();
    mailbox.input(Event::Rendered(window(1), 0)).unwrap();
    mailbox.input(Event::Rendered(window(1), 1)).unwrap();
    let click = Event::Press(
        window(1),
        NodeId::from_parts(0, 1).unwrap(),
        HandlerId::from_parts(0, 1).unwrap(),
        1,
    );
    mailbox.input(click.clone()).unwrap();
    mailbox.input(Event::Rendered(window(1), 2)).unwrap();
    assert_eq!(
        mailbox.drain(256),
        vec![
            Event::Rendered(window(1), 1),
            click.clone(),
            Event::Rendered(window(1), 2)
        ]
    );
    for _ in 0..MAX_INPUT_EVENTS {
        mailbox.input(click.clone()).unwrap();
    }
    assert_eq!(mailbox.input(click.clone()), Err(Box::new(click)));
    mailbox.fault(window(1));
    mailbox.fault(window(1));
    let events = mailbox.drain(256);
    assert_eq!(events.len(), MAX_INPUT_EVENTS + 1);
    assert_eq!(events.last(), Some(&Event::Overloaded(window(1))));
}

#[test]
fn window_close_and_terminal_stop_are_delivered_under_full_queues() {
    let mut mailbox = Mailbox::default();
    for _ in 0..MAX_COMMANDS {
        mailbox.submit(Message::Hello(VERSION, 0), 1).unwrap();
    }
    for i in 0..MAX_INPUT_EVENTS {
        let id = WindowId::from_parts((i % 2) as i64, 1).unwrap();
        mailbox.input(Event::Rendered(id, 0)).unwrap();
    }
    mailbox.request_close(window(1));
    mailbox.request_close(window(1));
    assert_eq!(mailbox.pop_close(), Some(window(1)));
    assert_eq!(mailbox.pop_close(), None);
    mailbox.native_closed(window(1));
    mailbox.close();
    assert!(mailbox.pop().is_none());
    let events = mailbox.drain(256);
    assert_eq!(
        &events[events.len() - 2..],
        &[Event::Closed(0, window(1)), Event::Stopped]
    );
    mailbox.close();
    assert!(mailbox.drain(256).is_empty());
}
