use binprot::BinProtWrite;
use gpuio_native::{
    mailbox::{MAX_COMMANDS, MAX_INPUT_EVENTS, Mailbox},
    session::Session,
};
use gpuio_protocol::{HandlerId, NodeId, ResourceId, WindowId, asset::*, v1::*};

fn submit(mailbox: &mut Mailbox, message: Message) -> Result<(), ErrorCode> {
    let mut bytes = vec![];
    message.binprot_write(&mut bytes).unwrap();
    mailbox.submit(gpuio_protocol::decode(&bytes).unwrap(), bytes.len())
}
fn execute(session: &mut Session, mailbox: &mut Mailbox) {
    let Message::Asset(correlation, request) = mailbox.pop().unwrap() else {
        panic!()
    };
    mailbox.respond(Event::AssetResponse(
        correlation,
        session.asset_request(request),
    ));
}
#[test]
fn asset_responses_survive_input_pressure_and_do_not_block_window_reuse() {
    let mut session = Session::default();
    let mut mailbox = Mailbox::default();
    session.hello(VERSION, CAPABILITIES).unwrap();
    submit(
        &mut mailbox,
        Message::Asset(1, Request::Begin(Format::Png, 4)),
    )
    .unwrap();
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
    assert!(
        !mailbox.has_window_output(0),
        "application asset response is not old-window output"
    );
    let response = mailbox.drain(1);
    let [Event::AssetResponse(1, Response::Begun(id))] = response.as_slice() else {
        panic!("{response:?}")
    };
    submit(
        &mut mailbox,
        Message::Asset(
            2,
            Request::Append(*id, 0, Chunk::new(vec![0, 255]).unwrap()),
        ),
    )
    .unwrap();
    execute(&mut session, &mut mailbox);
    assert_eq!(mailbox.drain(1), [Event::AssetResponse(2, Response::Ack)]);
    submit(&mut mailbox, Message::Asset(3, Request::Finish(*id))).unwrap();
    execute(&mut session, &mut mailbox);
    assert_eq!(
        mailbox.drain(1),
        [Event::AssetResponse(3, Response::Failed(Error::Incomplete))]
    );
    assert_eq!(session.assets().unwrap().stats().reserved_bytes, 0);
}
#[test]
fn response_backpressure_prevents_unacknowledged_asset_operations() {
    let mut session = Session::default();
    let mut mailbox = Mailbox::default();
    session.hello(VERSION, CAPABILITIES).unwrap();
    let absent = ResourceId::from_parts(0, 1).unwrap();
    for batch in 0..2 {
        for index in 0..MAX_COMMANDS {
            submit(
                &mut mailbox,
                Message::Asset(
                    (batch * MAX_COMMANDS + index + 1) as i64,
                    Request::Release(absent),
                ),
            )
            .unwrap();
        }
        for _ in 0..MAX_COMMANDS {
            execute(&mut session, &mut mailbox);
        }
    }
    assert_eq!(
        submit(
            &mut mailbox,
            Message::Asset(200, Request::Begin(Format::Png, 1))
        ),
        Err(ErrorCode::Busy)
    );
    assert!(mailbox.pop().is_none());
    assert_eq!(session.assets().unwrap().stats().reserved_bytes, 0);
    let replies = mailbox.drain(256);
    assert_eq!(replies.len(), 128);
    assert!(replies.iter().all(|event| matches!(
        event,
        Event::AssetResponse(_, Response::Failed(Error::StaleHandle))
    )));
    submit(
        &mut mailbox,
        Message::Asset(200, Request::Begin(Format::Png, 1)),
    )
    .unwrap();
    execute(&mut session, &mut mailbox);
    assert!(matches!(
        mailbox.drain(1).as_slice(),
        [Event::AssetResponse(200, Response::Begun(_))]
    ));
    session.shutdown();
    submit(
        &mut mailbox,
        Message::Asset(201, Request::Begin(Format::Png, 1)),
    )
    .unwrap();
    execute(&mut session, &mut mailbox);
    assert_eq!(
        mailbox.drain(1),
        [Event::AssetResponse(201, Response::Failed(Error::Closed))]
    );
}
