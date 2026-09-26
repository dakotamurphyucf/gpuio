use binprot::BinProtWrite;
use gpuio_native::{
    color_input_state::State,
    mailbox::{MAX_INPUT_BYTES, MAX_INPUT_EVENTS, Mailbox},
    session::Session,
};
use gpuio_protocol::{HandlerId, NodeId, WindowId, color_input as c, color_value as v, v1::*};
use std::sync::Arc;

fn window() -> WindowId {
    WindowId::from_parts(0, 1).unwrap()
}
fn node() -> NodeId {
    NodeId::from_parts(0, 1).unwrap()
}
fn handler() -> HandlerId {
    HandlerId::from_parts(0, 1).unwrap()
}
fn color(packed: i64) -> v::Value {
    v::Value::Color(v::Rgba::from_packed(packed).unwrap())
}
fn config() -> c::Config {
    c::Config {
        labels: c::Labels {
            control: "Accent".into(),
            hue: "Hue".into(),
            saturation: "Saturation".into(),
            lightness: "Lightness".into(),
            alpha: "Opacity".into(),
            hex: "Hex color".into(),
            clear: "Clear color".into(),
        },
        palette: vec![
            c::PaletteEntry {
                color: v::Rgba::new(255, 0, 0, 128),
                label: "Half red".into(),
            },
            c::PaletteEntry {
                color: v::Rgba::new(17, 34, 51, 255),
                label: "Slate".into(),
            },
        ],
        alpha_policy: v::AlphaPolicy::AllowAlpha,
        allow_empty: true,
        disabled: false,
        read_only: true,
    }
}
fn session() -> Session {
    let mut s = Session::default();
    s.hello(VERSION, CAPABILITIES).unwrap();
    s.open(1, window(), "Color", 320., 400.).unwrap();
    s
}
fn create() -> Op {
    Op::Create(node(), Kind::ColorInput, "".into(), Some(handler()))
}
fn set(config: c::Config, initial: v::Value) -> Op {
    Op::SetColorInput(node(), Box::new(config), initial)
}
fn tx(base: i64, operations: Vec<Op>) -> Transaction {
    Transaction {
        window: window(),
        base,
        revision: base + 1,
        operations,
    }
}
fn mount(s: &mut Session) {
    s.apply(&tx(
        0,
        vec![
            create(),
            set(config(), color(0x00ff00ff)),
            Op::SetRoot(Some(node())),
        ],
    ))
    .unwrap();
}
fn snapshot(revision: i64) -> c::Snapshot {
    c::Snapshot {
        revision,
        value: color(0xff000080),
        committed: color(0x00ff00ff),
        channels: v::Hsla::new(0., 1., 0.5, 0.5).unwrap(),
        interaction: Some(c::Interaction {
            id: 1,
            kind: c::InteractionKind::Text(c::Field::Hex),
        }),
        draft: Some(c::Draft {
            text: "#ff000080".into(),
            composing: false,
            status: c::DraftStatus::Valid,
        }),
        value_allowed: true,
        committed_allowed: true,
    }
}
fn idle(revision: i64) -> c::Snapshot {
    c::Snapshot {
        committed: snapshot(revision).value,
        interaction: None,
        draft: None,
        ..snapshot(revision)
    }
}
fn event(e: c::Event) -> Event {
    Event::ColorInputEvent(window(), node(), handler(), 1, e)
}
fn preview(revision: i64) -> Event {
    event(c::Event::Preview(snapshot(revision)))
}
fn pair(revision: i64) -> Vec<Event> {
    vec![
        event(c::Event::Cancelled(
            c::CancelReason::Programmatic,
            idle(revision),
        )),
        event(c::Event::Observed(idle(revision + 1))),
    ]
}
fn hex(value: &impl BinProtWrite) -> String {
    let mut b = vec![];
    value.binprot_write(&mut b).unwrap();
    b.iter().map(|x| format!("{x:02x}")).collect()
}

#[test]
fn independent_envelope_tags_and_exact_request_decoding() {
    let request = Message::Apply(tx(
        0,
        vec![
            create(),
            set(config(), color(0x00ff00ff)),
            Op::SetRoot(Some(node())),
        ],
    ));
    let reference = include_str!("../../../test/fixtures/color-request.hex").trim();
    assert_eq!(hex(&request), reference);
    let mut bytes = vec![];
    request.binprot_write(&mut bytes).unwrap();
    assert_eq!(gpuio_protocol::decode(&bytes), Ok(request));
    for end in 0..bytes.len() {
        assert!(gpuio_protocol::decode(&bytes[..end]).is_err());
    }
    bytes.push(0);
    assert!(gpuio_protocol::decode(&bytes).is_err());
    let mut sample = snapshot(8);
    sample.interaction.as_mut().unwrap().id = 5;
    assert_eq!(
        hex(&vec![event(c::Event::Preview(sample))]),
        include_str!("../../../test/fixtures/color-events.hex").trim()
    );
}

#[test]
fn tree_admission_is_atomic_and_preserves_seed_and_storage_across_configuration() {
    let mut s = session();
    let mut opaque = config();
    opaque.alpha_policy = v::AlphaPolicy::OpaqueOnly;
    for operations in [
        vec![create()],
        vec![
            Op::Create(node(), Kind::ColorInput, "".into(), None),
            set(config(), v::Value::Empty),
        ],
        vec![
            Op::Create(node(), Kind::Container, "".into(), Some(handler())),
            set(config(), v::Value::Empty),
        ],
        vec![
            Op::Create(
                node(),
                Kind::ColorInput,
                "unexpected".into(),
                Some(handler()),
            ),
            set(config(), v::Value::Empty),
        ],
        vec![create(), set(opaque.clone(), color(0xff000080))],
    ] {
        assert!(s.apply(&tx(0, operations)).is_err());
        assert_eq!(s.retained_bytes(), 0);
        assert_eq!(s.tree(window()).unwrap().revision(), 0);
    }
    s.apply(&tx(
        0,
        vec![
            create(),
            set(config(), color(0xff000080)),
            set(opaque.clone(), v::Value::Empty),
            Op::SetRoot(Some(node())),
        ],
    ))
    .unwrap();
    let mounted = s
        .tree(window())
        .unwrap()
        .get(node())
        .unwrap()
        .color_input
        .as_ref()
        .unwrap();
    assert_eq!(mounted.initial, color(0xff000080));
    assert!(!mounted.config.allows(mounted.initial));
    assert!(State::new(mounted.config.clone(), mounted.initial).is_err());
    let restored = State::from_retained(mounted.config.clone(), mounted.initial).unwrap();
    assert!(!restored.snapshot().value_allowed);
    drop(restored);
    let old = Arc::downgrade(&mounted.config);
    let retained = s.retained_bytes();
    let mut invalid = opaque.clone();
    invalid.palette = vec![invalid.palette[0].clone(); 257];
    assert!(
        s.apply(&tx(1, vec![set(invalid, v::Value::Empty)]))
            .is_err()
    );
    assert_eq!(s.retained_bytes(), retained);
    opaque.labels.control = "Updated".into();
    s.apply(&tx(1, vec![set(opaque.clone(), color(0x0000ffff))]))
        .unwrap();
    assert!(old.upgrade().is_none());
    let mounted = s
        .tree(window())
        .unwrap()
        .get(node())
        .unwrap()
        .color_input
        .as_ref()
        .unwrap();
    assert_eq!(mounted.initial, color(0xff000080));
    assert_eq!(s.retained_bytes(), 32 + mounted.config.retained_bytes());
    let last = Arc::downgrade(&mounted.config);
    s.apply(&tx(2, vec![Op::SetRoot(None), Op::Remove(node())]))
        .unwrap();
    assert_eq!(s.retained_bytes(), 0);
    assert!(last.upgrade().is_none());
    let fresh = NodeId::from_parts(0, 2).unwrap();
    s.apply(&tx(
        3,
        vec![
            Op::Create(fresh, Kind::ColorInput, "".into(), Some(handler())),
            Op::SetColorInput(fresh, Box::new(config()), v::Value::Empty),
            Op::SetRoot(Some(fresh)),
        ],
    ))
    .unwrap();
    s.close(window()).unwrap();
    assert_eq!(s.retained_bytes(), 0);
}

#[test]
fn routes_fence_leases_revisions_handlers_faults_and_preserve_cleanup() {
    let mut s = session();
    mount(&mut s);
    let sample = c::Event::Preview(snapshot(2));
    assert_eq!(
        s.color_input_event(window(), node(), handler(), 1, sample.clone()),
        Some(event(sample.clone()))
    );
    assert!(s.press(window(), node(), handler(), 1).is_none());
    for (w, n, h, r) in [
        (WindowId::from_parts(0, 2).unwrap(), node(), handler(), 1),
        (window(), NodeId::from_parts(0, 2).unwrap(), handler(), 1),
        (window(), node(), HandlerId::from_parts(0, 2).unwrap(), 1),
        (window(), node(), handler(), -1),
        (window(), node(), handler(), 2),
    ] {
        assert!(s.color_input_event(w, n, h, r, sample.clone()).is_none());
    }
    assert!(
        s.color_input_event(
            window(),
            node(),
            handler(),
            1,
            c::Event::Committed(c::Source::Text, snapshot(2))
        )
        .is_none()
    );
    let mut c = config();
    c.disabled = true;
    s.apply(&tx(1, vec![set(c, v::Value::Empty)])).unwrap();
    assert!(
        s.color_input_event(
            window(),
            node(),
            handler(),
            1,
            c::Event::Cancelled(c::CancelReason::Disabled, idle(3))
        )
        .is_some()
    );
    assert!(s.overload(window()));
    assert!(
        s.color_input_event(window(), node(), handler(), 2, sample.clone())
            .is_none()
    );
    s.close(window()).unwrap();
    assert!(
        s.color_input_event(window(), node(), handler(), 1, sample)
            .is_none()
    );
}

#[test]
fn previews_coalesce_only_within_interaction_and_never_across_discrete_barriers() {
    let mut m = Mailbox::default();
    m.input(preview(2)).unwrap();
    m.input(preview(3)).unwrap();
    assert_eq!(m.drain(128), [preview(3)]);
    for barrier in [
        event(c::Event::Committed(c::Source::Text, idle(4))),
        event(c::Event::Cancelled(c::CancelReason::Escape, idle(4))),
        event(c::Event::Observed(idle(4))),
        Event::Rendered(window(), 1),
    ] {
        m.input(preview(2)).unwrap();
        m.input(barrier.clone()).unwrap();
        m.input(preview(5)).unwrap();
        assert_eq!(m.drain(128), [preview(2), barrier, preview(5)]);
    }
    for next in [
        Event::ColorInputEvent(
            WindowId::from_parts(0, 2).unwrap(),
            node(),
            handler(),
            1,
            c::Event::Preview(snapshot(3)),
        ),
        Event::ColorInputEvent(
            window(),
            NodeId::from_parts(0, 2).unwrap(),
            handler(),
            1,
            c::Event::Preview(snapshot(3)),
        ),
        Event::ColorInputEvent(
            window(),
            node(),
            HandlerId::from_parts(0, 2).unwrap(),
            1,
            c::Event::Preview(snapshot(3)),
        ),
        Event::ColorInputEvent(
            window(),
            node(),
            handler(),
            2,
            c::Event::Preview(snapshot(3)),
        ),
        event(c::Event::Preview(c::Snapshot {
            interaction: Some(c::Interaction {
                id: 2,
                kind: c::InteractionKind::Text(c::Field::Hex),
            }),
            ..snapshot(3)
        })),
        preview(2),
    ] {
        m.input(preview(2)).unwrap();
        m.input(next.clone()).unwrap();
        assert_eq!(m.drain(128), [preview(2), next]);
    }
    m.input(preview(2)).unwrap();
    m.submit(Message::Hello(VERSION, 0), 2).unwrap();
    m.pop().unwrap();
    m.respond(Event::Failed(1, ErrorCode::NotReady));
    m.input(preview(3)).unwrap();
    assert_eq!(
        m.drain(128),
        [
            preview(2),
            Event::Failed(1, ErrorCode::NotReady),
            preview(3)
        ]
    );
}

#[test]
fn batch_count_admission_is_atomic_even_when_first_preview_can_coalesce() {
    let mut m = Mailbox::default();
    let observed = event(c::Event::Observed(idle(0)));
    for _ in 0..MAX_INPUT_EVENTS - 1 {
        m.input(observed.clone()).unwrap();
    }
    m.input(preview(2)).unwrap();
    let batch = vec![
        preview(3),
        event(c::Event::Committed(c::Source::Text, idle(4))),
    ];
    assert_eq!(m.color_batch(batch.clone()), Err(batch));
    let drained = m.drain(128);
    assert_eq!(drained.len(), 128);
    assert_eq!(drained.last(), Some(&preview(2)));
    for _ in 0..MAX_INPUT_EVENTS - 2 {
        m.input(observed.clone()).unwrap();
    }
    m.input(preview(2)).unwrap();
    m.color_batch(vec![
        preview(3),
        event(c::Event::Committed(c::Source::Text, idle(4))),
    ])
    .unwrap();
    assert!(m.has_window_output(window().slot()));
    let drained = m.drain(128);
    assert_eq!(drained.len(), 128);
    assert_eq!(
        &drained[126..],
        &[
            preview(3),
            event(c::Event::Committed(c::Source::Text, idle(4)))
        ]
    );
    assert!(!m.has_window_output(window().slot()));
    for bad in [
        vec![],
        vec![preview(2), preview(3), preview(4)],
        vec![preview(2), preview(4)],
        vec![preview(2), preview(3)],
        vec![preview(2), Event::Rendered(window(), 1)],
    ] {
        m.input(observed.clone()).unwrap();
        assert!(m.color_batch(bad).is_err());
        assert_eq!(m.drain(128).as_slice(), std::slice::from_ref(&observed));
    }
    m.color_batch(pair(2)).unwrap();
    assert_eq!(m.drain(128), pair(2));
    m.close();
    assert!(m.color_batch(pair(4)).is_err());
    assert!(m.input(preview(6)).is_err());
}

#[test]
fn batch_byte_admission_keeps_the_previous_tail_and_drain_is_bounded() {
    let mut m = Mailbox::default();
    let filler = |len| {
        Event::EditorEvent(
            window(),
            node(),
            handler(),
            1,
            EditorEventKind::Submitted,
            EditorSnapshot {
                revision: 1,
                text: "a".repeat(len),
                selection: EditorSelection { anchor: 0, head: 0 },
                composition: None,
                focused: false,
            },
        )
    };
    let mut remaining = MAX_INPUT_BYTES;
    while remaining > MAX_TEXT_BYTES + 256 {
        m.input(filler(MAX_TEXT_BYTES)).unwrap();
        remaining -= MAX_TEXT_BYTES + 256;
    }
    // Leave 511 bytes: one event fits, a two-event discrete boundary does not.
    m.input(filler(remaining - 256 - 511)).unwrap();
    assert!(m.color_batch(pair(2)).is_err());
    while m.has_output() {
        let drained = m.drain(128);
        assert!(!drained.is_empty());
        assert!(drained.iter().all(|e| matches!(e, Event::EditorEvent(..))));
    }
    let large = |revision| {
        event(c::Event::Preview(c::Snapshot {
            draft: Some(c::Draft {
                text: "x".repeat(4096),
                composing: false,
                status: c::DraftStatus::Invalid,
            }),
            ..snapshot(revision)
        }))
    };
    m.input(large(2)).unwrap();
    m.input(preview(3)).unwrap();
    m.color_batch(pair(4)).unwrap();
    assert_eq!(
        m.drain(128),
        [preview(3), pair(4)[0].clone(), pair(4)[1].clone()]
    );
}
