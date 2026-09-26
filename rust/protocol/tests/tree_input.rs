use binprot::BinProtWrite;
use gpuio_protocol::{HandlerId, NodeId, WindowId, tree_input::*, v1::*};
fn bytes(value: &impl BinProtWrite) -> Vec<u8> {
    let mut bytes = vec![];
    value.binprot_write(&mut bytes).unwrap();
    bytes
}
fn hex(value: &impl BinProtWrite) -> String {
    bytes(value).iter().map(|b| format!("{b:02x}")).collect()
}
#[test]
fn independent_request_event_and_opt_in_fixtures() {
    let requests = vec![
        Request::Navigate(Navigation::Next, Some(Selection::Range { extend: true })),
        Request::Select(42, Selection::Toggle),
        Request::Focus(42),
        Request::SetExpanded(42, true),
        Request::Activate(42),
        Request::SelectActive(Selection::Replace),
        Request::ActivateActive,
    ];
    assert_eq!(
        hex(&requests),
        include_str!("../../../test/fixtures/tree-input-requests.hex").trim()
    );
    assert!(requests.iter().all(Request::is_valid));
    assert!(!Request::Focus(0).is_valid());
    assert!(!Request::SetExpanded(-1, false).is_valid());
    let window = WindowId::from_parts(0, 1).unwrap();
    let node = NodeId::from_parts(0, 1).unwrap();
    let message = Message::Apply(Transaction {
        window,
        base: 0,
        revision: 1,
        operations: vec![Op::SetTreeInput(node, true)],
    });
    let encoded = bytes(&message);
    assert_eq!(
        hex(&message),
        include_str!("../../../test/fixtures/tree-input-transaction.hex").trim()
    );
    assert_eq!(gpuio_protocol::decode(&encoded), Ok(message));
    for end in 0..encoded.len() {
        assert!(gpuio_protocol::decode(&encoded[..end]).is_err());
    }
    let mut invalid = encoded.clone();
    *invalid.last_mut().unwrap() = 2;
    assert!(gpuio_protocol::decode(&invalid).is_err());
    let event = Event::TreeInput(
        window,
        node,
        HandlerId::from_parts(0, 1).unwrap(),
        1,
        requests[0].clone(),
    );
    assert_eq!(
        hex(&event),
        include_str!("../../../test/fixtures/tree-input-event.hex").trim()
    );
}
