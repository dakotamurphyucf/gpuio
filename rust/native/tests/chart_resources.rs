use binprot::BinProtWrite;
use gpuio_native::session::{ChartDispatch, Session};
use gpuio_protocol::{
    ResourceId,
    chart_data::{Contents, Data},
    chart_resource::{Error, Request, Response, Update},
    v1::{CAPABILITIES, VERSION},
};

fn immediate(session: &mut Session, request: Request) -> Response {
    match session.chart_request(request) {
        ChartDispatch::Immediate(response) => response,
        ChartDispatch::Publish(_) => panic!("unexpected deferred publication"),
    }
}
fn create(session: &mut Session) -> ResourceId {
    match immediate(session, Request::Create) {
        Response::Created(id) => id,
        other => panic!("unexpected allocation {other:?}"),
    }
}
fn stage(session: &mut Session, id: ResourceId) {
    let data = Data {
        version: 2,
        bar_backgrounds: vec![],
        contents: Contents::Pie(vec![]),
    };
    let mut bytes = vec![];
    data.binprot_write(&mut bytes).unwrap();
    assert_eq!(
        immediate(
            session,
            Request::Begin(Update {
                id,
                base: 0,
                revision: 1,
                generation: 1,
                bytes: bytes.len() as i64
            })
        ),
        Response::Ack
    );
    assert_eq!(
        immediate(
            session,
            Request::Chunk(id, 1, 0, gpuio_protocol::asset::Chunk::new(bytes).unwrap())
        ),
        Response::Ack
    );
}
fn work(session: &mut Session, id: ResourceId) -> gpuio_native::chart_store::Work {
    match session.chart_request(Request::Publish(id, 1)) {
        ChartDispatch::Publish(work) => work,
        ChartDispatch::Immediate(other) => panic!("expected asynchronous work, got {other:?}"),
    }
}

#[test]
fn publication_is_deferred_and_readers_survive_registration_release() {
    let mut session = Session::default();
    assert_eq!(
        immediate(&mut session, Request::Create),
        Response::Failed(Error::NotReady)
    );
    session.hello(VERSION, CAPABILITIES).unwrap();
    let id = create(&mut session);
    stage(&mut session, id);
    let completion = std::thread::spawn({
        let job = work(&mut session, id);
        move || job.run()
    })
    .join()
    .unwrap();
    assert!(matches!(session.chart(id), Err(Error::NotReady)));
    assert_eq!(session.complete_chart(completion), Response::Ack);
    let lease = session.chart(id).unwrap();
    let snapshot = lease.snapshot().unwrap();
    assert_eq!((snapshot.revision(), snapshot.generation()), (1, 1));
    assert_eq!(snapshot.data().validate().unwrap().values, 0);
    assert_eq!(immediate(&mut session, Request::Release(id)), Response::Ack);
    assert!(lease.snapshot().is_none());
    drop(lease);
    assert!(session.chart_bytes() > 0);
    drop(snapshot);
    assert_eq!(session.chart_bytes(), 0);
}

#[test]
fn session_shutdown_cancels_pending_publication_and_rejects_late_work() {
    let mut session = Session::default();
    session.hello(VERSION, CAPABILITIES).unwrap();
    let id = create(&mut session);
    stage(&mut session, id);
    let work = work(&mut session, id);
    session.shutdown();
    assert!(session.chart_bytes() > 0);
    assert_eq!(
        session.complete_chart(work.run()),
        Response::Failed(Error::Closed)
    );
    assert_eq!(session.chart_bytes(), 0);
    assert_eq!(
        immediate(&mut session, Request::Create),
        Response::Failed(Error::Closed)
    );
}
