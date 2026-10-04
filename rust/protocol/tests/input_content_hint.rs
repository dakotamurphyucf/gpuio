use binprot::BinProtWrite;
use gpuio_protocol::{DecodeError, NodeId, WindowId, decode, input_content_hint::Hint, v1::*};
fn message(hints: impl IntoIterator<Item = Option<Hint>>) -> Message {
    Message::Apply(Transaction {
        window: WindowId::from_parts(0, 1).unwrap(),
        base: 0,
        revision: 1,
        operations: hints
            .into_iter()
            .map(|hint| Op::SetEditorContentHint(NodeId::from_parts(1, 2).unwrap(), hint))
            .collect(),
    })
}
fn bytes(message: &Message) -> Vec<u8> {
    let mut out = vec![];
    message.binprot_write(&mut out).unwrap();
    out
}
#[test]
fn content_hints_have_paired_bytes_and_exact_tag_bounds() {
    let msg = message([
        None,
        Some(Hint::EmailAddress),
        Some(Hint::Password),
        Some(Hint::CellularImei),
        Some(Hint::GivenName),
    ]);
    let data = bytes(&msg);
    assert_eq!(
        data.iter().map(|b| format!("{b:02x}")).collect::<String>(),
        include_str!("../../../test/fixtures/input-content-hint-operation.hex").trim()
    );
    assert_eq!(decode(&data), Ok(msg));
    for end in 0..data.len() {
        assert!(decode(&data[..end]).is_err());
    }
    let mut extra = data.clone();
    extra.push(0);
    assert_eq!(decode(&extra), Err(DecodeError::Malformed));
    for tag in 0..45 {
        let hint = Hint::from_tag(tag).unwrap();
        let msg = message([Some(hint)]);
        assert_eq!(decode(&bytes(&msg)), Ok(msg));
        if let Some(value) = hint.macos_value() {
            assert!(value.len() <= 64 && !value.is_empty());
        }
    }
    for tag in 45..=255 {
        let mut data = bytes(&message([Some(Hint::Name)]));
        *data.last_mut().unwrap() = tag;
        assert_eq!(decode(&data), Err(DecodeError::Malformed));
    }
    assert_eq!(Hint::EmailAddress.macos_value(), Some("email"));
    assert_eq!(Hint::NewPassword.macos_value(), Some("new-password"));
    assert_eq!(Hint::CellularEid.macos_value(), None);
    assert_eq!(Hint::CellularImei.macos_value(), None);
}

#[test]
fn content_hint_query_and_statuses_have_independent_wire_fixtures() {
    use gpuio_protocol::input_content_hint::{Status, Unavailability};
    let window = WindowId::from_parts(0, 1).unwrap();
    let node = NodeId::from_parts(1, 2).unwrap();
    let request = Message::EditorCommand(7, window, node, EditorCommand::ReadContentHintStatus);
    let data = bytes(&request);
    assert_eq!(
        data.iter().map(|b| format!("{b:02x}")).collect::<String>(),
        include_str!("../../../test/fixtures/input-content-hint-request.hex").trim()
    );
    assert_eq!(decode(&data), Ok(request));
    for end in 0..data.len() {
        assert!(decode(&data[..end]).is_err());
    }
    let mut extra = data;
    extra.push(0);
    assert_eq!(decode(&extra), Err(DecodeError::Malformed));
    let statuses = [
        Status::Inactive(None),
        Status::Inactive(Some(Hint::EmailAddress)),
        Status::Exposed(Hint::EmailAddress),
        Status::Unavailable(Hint::EmailAddress, Unavailability::Backend),
        Status::Unavailable(Hint::CellularImei, Unavailability::Mapping),
        Status::Unavailable(Hint::Url, Unavailability::NativeView),
    ];
    let events: Vec<_> = statuses
        .into_iter()
        .enumerate()
        .map(|(i, status)| {
            Event::EditorResult(
                7 + i as i64,
                window,
                node,
                EditorResult::ContentHintStatus(status),
            )
        })
        .collect();
    let mut data = vec![];
    events.binprot_write(&mut data).unwrap();
    assert_eq!(
        data.iter().map(|b| format!("{b:02x}")).collect::<String>(),
        include_str!("../../../test/fixtures/input-content-hint-events.hex").trim()
    );
}
