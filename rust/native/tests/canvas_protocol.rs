use binprot::BinProtWrite;
use gpuio_native::{
    mailbox::{MAX_COMMANDS, MAX_INPUT_EVENTS, Mailbox},
    session::Session,
};
use gpuio_protocol::{HandlerId, NodeId, WindowId, canvas_resource::*, v1::*};
fn submit(mailbox: &mut Mailbox, message: Message) -> Result<(), ErrorCode> {
    let mut bytes = Vec::new();
    message.binprot_write(&mut bytes).unwrap();
    mailbox.submit(gpuio_protocol::decode(&bytes).unwrap(), bytes.len())
}
fn execute(session: &mut Session, mailbox: &mut Mailbox) {
    let Message::Canvas(correlation, request) = mailbox.pop().unwrap() else {
        panic!("expected canvas request")
    };
    mailbox.respond(Event::CanvasResponse(
        correlation,
        session.canvas_request(request),
    ));
}
#[test]
fn scene_response_uses_reserved_lane_under_full_input_pressure() {
    let mut session = Session::default();
    let mut mailbox = Mailbox::default();
    session.hello(VERSION, CAPABILITIES).unwrap();
    submit(&mut mailbox, Message::Canvas(7, Request::Create)).unwrap();
    for _ in 0..MAX_INPUT_EVENTS {
        mailbox
            .input(Event::Press(
                WindowId::from_parts(0, 1).unwrap(),
                NodeId::from_parts(0, 1).unwrap(),
                HandlerId::from_parts(0, 1).unwrap(),
                1,
            ))
            .unwrap();
    }
    execute(&mut session, &mut mailbox);
    assert_eq!(mailbox.drain(MAX_INPUT_EVENTS).len(), MAX_INPUT_EVENTS);
    assert!(!mailbox.has_window_output(0));
    let response = mailbox.drain(1);
    let [Event::CanvasResponse(7, Response::Created(id))] = response.as_slice() else {
        panic!("{response:?}")
    };
    submit(&mut mailbox, Message::Canvas(8, Request::Release(*id))).unwrap();
    execute(&mut session, &mut mailbox);
    assert_eq!(
        mailbox.drain(1),
        vec![Event::CanvasResponse(8, Response::Ack)]
    );
}
#[test]
fn canvas_backpressure_preserves_correlations_and_shutdown_fences_late_requests() {
    let mut session = Session::default();
    let mut mailbox = Mailbox::default();
    session.hello(VERSION, CAPABILITIES).unwrap();
    for correlation in 1..=MAX_COMMANDS {
        submit(
            &mut mailbox,
            Message::Canvas(correlation as i64, Request::Create),
        )
        .unwrap();
    }
    assert_eq!(
        submit(&mut mailbox, Message::Canvas(100, Request::Create)),
        Err(ErrorCode::Busy)
    );
    for _ in 0..MAX_COMMANDS {
        execute(&mut session, &mut mailbox);
    }
    for (index, event) in mailbox.drain(MAX_COMMANDS).into_iter().enumerate() {
        let Event::CanvasResponse(correlation, Response::Created(id)) = event else {
            panic!("wrong response")
        };
        assert_eq!(correlation, index as i64 + 1);
        assert_eq!(session.canvas_request(Request::Release(id)), Response::Ack);
    }
    session.shutdown();
    submit(&mut mailbox, Message::Canvas(101, Request::Create)).unwrap();
    execute(&mut session, &mut mailbox);
    assert_eq!(
        mailbox.drain(1),
        vec![Event::CanvasResponse(101, Response::Failed(Error::Closed))]
    );
}
