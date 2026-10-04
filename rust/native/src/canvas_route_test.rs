//! Native routing admission on TestPlatform. No OS AX, GPU or OCaml-delivery claim.
use super::*;
use crate::canvas_plan::{self, Budget};
use binprot::BinProtWrite;
use gpuio_protocol::{
    canvas::Transform,
    canvas_resource::{Request, Response, Update},
    canvas_scene::{Drawing, Interaction, Item, Paint, Scene, Shape},
    canvas_view::Viewport,
};
use std::{os::fd::AsRawFd, os::unix::net::UnixStream, sync::atomic::AtomicBool};

fn publish(session: &SharedSession, source: ResourceId, revision: i64) {
    let rect = Rect {
        x: revision as f64,
        y: 0.,
        width: 20.,
        height: 20.,
    };
    let scene = Scene {
        version: 1,
        description: "Route fixture".into(),
        resources: vec![],
        items: vec![Item {
            id: 1,
            transform: Transform::IDENTITY,
            clips: vec![],
            interaction: Some(Interaction {
                label: "Item".into(),
                hit_region: HitRegion::Rectangle(rect),
                draggable: true,
                activatable: true,
            }),
            drawing: Drawing::Shape(
                Shape::Rectangle(rect),
                Paint {
                    fill: Some(0xffffffff),
                    stroke: None,
                },
            ),
        }],
    };
    let mut bytes = vec![];
    scene.binprot_write(&mut bytes).unwrap();
    for request in [
        Request::Begin(Update {
            id: source,
            base: revision - 1,
            revision,
            generation: 1,
            bytes: bytes.len() as i64,
        }),
        Request::Chunk(
            source,
            revision,
            0,
            gpuio_protocol::asset::Chunk::new(bytes).unwrap(),
        ),
        Request::Publish(source, revision),
    ] {
        assert_eq!(session.borrow_mut().canvas_request(request), Response::Ack);
    }
}

fn prepared(snapshot: Arc<Snapshot>) -> canvas_jobs::Ready {
    let quality = Quality::new(1., 1.).unwrap();
    let plan = canvas_plan::prepare(
        &snapshot.scene,
        quality,
        &Budget::default(),
        &AtomicBool::new(false),
    )
    .unwrap();
    canvas_jobs::Ready {
        snapshot,
        quality,
        plan,
    }
}

fn route(state: &Shared) -> Route {
    let current = state.borrow();
    Route {
        state: state.clone(),
        token: current.input.token.clone(),
        snapshot: Arc::downgrade(current.native.as_ref().unwrap().snapshot()),
        item: 1,
    }
}

fn drain_activations(transport: &Transport) -> Vec<(i64, i64, i64)> {
    transport
        .mailbox
        .lock()
        .unwrap()
        .drain(128)
        .into_iter()
        .filter_map(|event| match event {
            Event::CanvasEvent(
                _,
                _,
                _,
                _,
                _,
                revision,
                generation,
                Observation::Activated(item),
            ) => Some((revision, generation, item)),
            Event::CanvasEvent(_, _, _, _, _, _, _, Observation::SelectionChanged(_)) => None,
            event => panic!("unexpected route event: {event:?}"),
        })
        .collect()
}

#[test]
fn activation_fences_publication_and_retired_callbacks() {
    let mut app = gpui::TestAppContext::single();
    let cx = app.add_empty_window();
    let (_reader, writer) = UnixStream::pair().unwrap();
    let transport = Arc::new(Transport::new(writer.as_raw_fd()).unwrap());
    let session = Rc::new(RefCell::new(Session::default()));
    let window_id = WindowId::from_parts(0, 1).unwrap();
    let node = NodeId::from_parts(0, 1).unwrap();
    session.borrow_mut().hello(VERSION, CAPABILITIES).unwrap();
    session
        .borrow_mut()
        .open(1, window_id, "Route", 100., 100.)
        .unwrap();
    let Response::Created(source) = session.borrow_mut().canvas_request(Request::Create) else {
        panic!("scene creation");
    };
    publish(&session, source, 1);
    let config = Arc::new(Config {
        source: Some(source),
        label: "Canvas".into(),
        initial_viewport: Viewport::default(),
        minimum_zoom: 0.05,
        maximum_zoom: 64.,
        selectable: true,
        draggable: true,
        pan_zoom: true,
        disabled: false,
        selection_color: 0xffffffff,
        command: None,
    });
    session
        .borrow_mut()
        .apply(&Transaction {
            window: window_id,
            base: 0,
            revision: 1,
            operations: vec![
                Op::Create(
                    node,
                    Kind::CanvasView,
                    "".into(),
                    Some(HandlerId::from_parts(0, 1).unwrap()),
                ),
                Op::SetCanvas(node, (*config).clone()),
                Op::SetRoot(Some(node)),
            ],
        })
        .unwrap();
    let gate = crate::host::focus::Manager::new(window_id, session.clone());
    let lease = session.borrow().canvas(source).unwrap();
    let initial = lease.snapshot();
    let native = canvas_state::State::new((*config).clone(), initial.clone())
        .unwrap()
        .0;
    let state = cx.update(|_, cx| {
        Rc::new(RefCell::new(State {
            input: input::Input::new(gate.clone(), cx),
            node,
            window: window_id,
            config,
            handler: Some(HandlerId::from_parts(0, 1).unwrap()),
            revision: 1,
            lease: Some(lease),
            native: Some(native),
            job: None,
            requested: None,
            ready: Some(prepared(initial.clone())),
            content: None,
            failure: None,
            reported: None,
            reported_preparation: None,
            closed: false,
            transport: Arc::downgrade(&transport),
            session: Rc::downgrade(&session),
        }))
    });
    cx.update(|window, _| window.activate_window());
    cx.run_until_parked();
    let first = route(&state);
    cx.update(|window, cx| {
        assert_eq!(first.rejection(&state.borrow(), window), None);
        first.invoke(true, window, cx);
    });
    assert_eq!(drain_activations(&transport), [(1, 1, 1)]);

    cx.deactivate_window();
    cx.update(|window, cx| {
        assert_eq!(
            first.rejection(&state.borrow(), window),
            Some(Rejection::Input(input::InputRejection::InactiveWindow))
        );
        first.invoke(true, window, cx);
    });
    cx.update(|window, _| window.activate_window());
    cx.run_until_parked();
    let ready = state.borrow_mut().ready.take();
    cx.update(|window, cx| {
        assert_eq!(
            first.rejection(&state.borrow(), window),
            Some(Rejection::Input(input::InputRejection::NotPrepared))
        );
        first.invoke(true, window, cx);
    });
    state.borrow_mut().ready = ready;
    assert!(drain_activations(&transport).is_empty());

    // Publication changes the lease immediately. A callback from the previous
    // AX tree must not emit even while the old native presentation is retained.
    publish(&session, source, 2);
    cx.update(|window, cx| {
        assert_eq!(
            first.rejection(&state.borrow(), window),
            Some(Rejection::Input(input::InputRejection::PublicationPending))
        );
        first.invoke(true, window, cx);
    });
    assert!(drain_activations(&transport).is_empty());

    // Install the prepared replacement, exactly the publication boundary used
    // by paint. The old route remains stale; a freshly constructed route works.
    {
        let mut state = state.borrow_mut();
        let snapshot = state.lease.as_ref().unwrap().snapshot();
        state
            .native
            .as_mut()
            .unwrap()
            .publish(snapshot.clone())
            .unwrap();
        state.ready = Some(prepared(snapshot));
    }
    cx.update(|window, cx| {
        assert_eq!(
            first.rejection(&state.borrow(), window),
            Some(Rejection::StaleSnapshot)
        );
        first.invoke(true, window, cx);
    });
    assert!(drain_activations(&transport).is_empty());
    let current = route(&state);
    cx.update(|window, cx| current.invoke(true, window, cx));
    assert_eq!(drain_activations(&transport), [(2, 1, 1)]);

    Arc::make_mut(&mut state.borrow_mut().config).disabled = true;
    cx.update(|window, cx| {
        assert_eq!(
            current.rejection(&state.borrow(), window),
            Some(Rejection::Input(input::InputRejection::Disabled))
        );
        current.invoke(true, window, cx);
    });
    Arc::make_mut(&mut state.borrow_mut().config).disabled = false;
    gate.borrow_mut().set_hidden([node].into());
    cx.update(|window, cx| {
        assert_eq!(
            current.rejection(&state.borrow(), window),
            Some(Rejection::Input(
                input::InputRejection::UnavailablePlacement
            ))
        );
        current.invoke(true, window, cx);
    });
    gate.borrow_mut().set_hidden(Default::default());
    state.borrow_mut().input.token = Rc::new(());
    cx.update(|window, cx| {
        assert_eq!(
            current.rejection(&state.borrow(), window),
            Some(Rejection::ReplacedCallback)
        );
        current.invoke(true, window, cx);
    });
    assert!(drain_activations(&transport).is_empty());
    let current = route(&state);
    cx.update(|window, cx| current.invoke(true, window, cx));
    assert_eq!(drain_activations(&transport), [(2, 1, 1)]);
    cx.update(|window, cx| {
        state.borrow_mut().close(window);
        assert_eq!(
            current.rejection(&state.borrow(), window),
            Some(Rejection::Input(input::InputRejection::Closed))
        );
        current.invoke(true, window, cx);
    });
    assert!(drain_activations(&transport).is_empty());
}
