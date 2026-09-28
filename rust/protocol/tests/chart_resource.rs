use binprot::BinProtWrite;
use gpuio_protocol::{
    DecodeError, ResourceId, chart_resource::*, decode_chart_request, decode_chart_response,
};

fn encode(value: &impl BinProtWrite) -> Vec<u8> {
    let mut bytes = vec![];
    value.binprot_write(&mut bytes).unwrap();
    bytes
}
fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

#[test]
fn independent_resource_frames_and_all_truncations() {
    let id = ResourceId::from_parts(0, 1).unwrap();
    let requests = [
        Request::Create,
        Request::Begin(Update {
            id,
            base: 1,
            revision: 2,
            generation: 1,
            bytes: 400,
        }),
        Request::Chunk(
            id,
            2,
            0,
            gpuio_protocol::asset::Chunk::new(vec![0, 255]).unwrap(),
        ),
        Request::Publish(id, 2),
        Request::Abort(id, 2),
        Request::Release(id),
    ];
    for (request, expected) in requests.into_iter().zip([
        "00",
        "010001010201fe9001",
        "02000102000200ff",
        "03000102",
        "04000102",
        "050001",
    ]) {
        let mut bytes = encode(&request);
        assert_eq!(hex(&bytes), expected);
        assert_eq!(decode_chart_request(&bytes), Ok(request));
        let mut frame = vec![21, 7];
        frame.extend_from_slice(&bytes);
        assert_eq!(
            gpuio_protocol::decode(&frame),
            Ok(gpuio_protocol::v1::Message::Chart(
                7,
                decode_chart_request(&bytes).unwrap()
            ))
        );
        for end in 0..frame.len() {
            assert!(gpuio_protocol::decode(&frame[..end]).is_err());
        }
        frame[1] = 0;
        assert_eq!(gpuio_protocol::decode(&frame), Err(DecodeError::Malformed));
        for end in 0..bytes.len() {
            assert!(decode_chart_request(&bytes[..end]).is_err());
        }
        bytes.push(0);
        assert_eq!(decode_chart_request(&bytes), Err(DecodeError::Malformed));
    }
    let errors = [
        Error::Closed,
        Error::ResourceLimit,
        Error::StaleHandle,
        Error::InvalidRevision,
        Error::InvalidRange,
        Error::Incomplete,
        Error::Busy,
        Error::NotReady,
        Error::InvalidData,
        Error::Cancelled,
        Error::NativeFailure,
    ];
    let responses = [Response::Created(id), Response::Ack]
        .into_iter()
        .chain(errors.into_iter().map(Response::Failed));
    for (response, expected) in responses.zip([
        "000001", "01", "0200", "0201", "0202", "0203", "0204", "0205", "0206", "0207", "0208",
        "0209", "020a",
    ]) {
        let mut bytes = encode(&response);
        assert_eq!(hex(&bytes), expected);
        assert_eq!(decode_chart_response(&bytes), Ok(response));
        let event =
            gpuio_protocol::v1::Event::ChartResponse(7, decode_chart_response(&bytes).unwrap());
        let mut expected = vec![1, 61, 7];
        expected.extend_from_slice(&bytes);
        assert_eq!(encode(&vec![event]), expected);
        for end in 0..bytes.len() {
            assert!(decode_chart_response(&bytes[..end]).is_err());
        }
        bytes.push(0);
        assert_eq!(decode_chart_response(&bytes), Err(DecodeError::Malformed));
    }
}

#[test]
fn chunk_and_frame_bounds_precede_allocation() {
    let mut oversized = vec![2, 0, 1, 1, 0];
    binprot::Nat0((MAX_CHUNK_BYTES + 1) as u64)
        .binprot_write(&mut oversized)
        .unwrap();
    assert_eq!(
        decode_chart_request(&oversized),
        Err(DecodeError::LimitExceeded)
    );
    let id = ResourceId::from_parts(0, 1).unwrap();
    let chunk = Request::Chunk(
        id,
        1,
        0,
        gpuio_protocol::asset::Chunk::new(vec![255; MAX_CHUNK_BYTES]).unwrap(),
    );
    assert_eq!(decode_chart_request(&encode(&chunk)), Ok(chunk));
    assert_eq!(
        decode_chart_request(&vec![0; MAX_CHUNK_BYTES + 129]),
        Err(DecodeError::LimitExceeded)
    );
    for bytes in [&[6][..], &[5, 0, 0], &[255]] {
        assert!(decode_chart_request(bytes).is_err());
    }
    for bytes in [&[3][..], &[2, 11], &[0, 0, 0]] {
        assert!(decode_chart_response(bytes).is_err());
    }
}
