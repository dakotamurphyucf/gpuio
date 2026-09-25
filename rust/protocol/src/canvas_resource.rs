//! Correlated retained-scene uploads; one publication is one immutable scene.
use crate::ResourceId;
use binprot::macros::BinProtWrite;

pub const MAX_CHUNK_BYTES: usize = 256 * 1024;

#[derive(Clone, Copy, Debug, PartialEq, Eq, BinProtWrite)]
pub enum Error {
    Closed,
    ResourceLimit,
    StaleHandle,
    InvalidRevision,
    InvalidRange,
    Incomplete,
    Busy,
    NotReady,
    InvalidScene,
    StaleResource,
    UnavailableImage,
    NativeFailure,
}
#[derive(Clone, Debug, PartialEq, Eq, BinProtWrite)]
pub struct Update {
    pub id: ResourceId,
    pub base: i64,
    pub revision: i64,
    pub generation: i64,
    pub bytes: i64,
}
#[derive(Clone, Debug, PartialEq, Eq, BinProtWrite)]
pub enum Request {
    Create,
    Begin(Update),
    Chunk(ResourceId, i64, i64, crate::asset::Chunk),
    Publish(ResourceId, i64),
    Abort(ResourceId, i64),
    Release(ResourceId),
}
#[derive(Clone, Debug, PartialEq, Eq, BinProtWrite)]
pub enum Response {
    Created(ResourceId),
    Ack,
    Failed(Error),
}

#[cfg(test)]
mod tests {
    use super::*;
    use binprot::BinProtWrite;
    #[test]
    fn request_frames_match_independently_constructed_ocaml() {
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
            Request::Chunk(id, 2, 0, crate::asset::Chunk::new(vec![0, 255]).unwrap()),
            Request::Publish(id, 2),
            Request::Abort(id, 2),
            Request::Release(id),
        ];
        let expected = [
            "00",
            "010001010201fe9001",
            "02000102000200ff",
            "03000102",
            "04000102",
            "050001",
        ];
        for (request, expected) in requests.iter().zip(expected) {
            let mut bytes = Vec::new();
            request.binprot_write(&mut bytes).unwrap();
            assert_eq!(
                bytes.iter().map(|b| format!("{b:02x}")).collect::<String>(),
                expected
            );
            let message = crate::v1::Message::Canvas(7, request.clone());
            let mut framed = Vec::new();
            message.binprot_write(&mut framed).unwrap();
            assert_eq!(&framed[..2], &[13, 7]);
            assert_eq!(&framed[2..], bytes);
            assert_eq!(crate::decode(&framed), Ok(message));
            for end in 0..framed.len() {
                assert!(crate::decode(&framed[..end]).is_err());
            }
            framed.push(0);
            assert!(crate::decode(&framed).is_err());
        }
    }

    #[test]
    fn canvas_response_event_tags_match_ocaml_and_bad_frames_are_rejected() {
        use crate::v1::{Event, Message};
        let id = ResourceId::from_parts(0, 1).unwrap();
        for (response, expected) in [
            (Response::Created(id), vec![1, 39, 7, 0, 0, 1]),
            (Response::Ack, vec![1, 39, 7, 1]),
            (
                Response::Failed(Error::UnavailableImage),
                vec![1, 39, 7, 2, 10],
            ),
        ] {
            let mut bytes = Vec::new();
            vec![Event::CanvasResponse(7, response)]
                .binprot_write(&mut bytes)
                .unwrap();
            assert_eq!(bytes, expected);
        }
        let mut invalid = Vec::new();
        Message::Canvas(0, Request::Create)
            .binprot_write(&mut invalid)
            .unwrap();
        assert_eq!(crate::decode(&invalid), Err(crate::DecodeError::Malformed));
        let mut oversized = vec![13, 7, 2, 0, 1, 1, 0];
        binprot::Nat0((MAX_CHUNK_BYTES + 1) as u64)
            .binprot_write(&mut oversized)
            .unwrap();
        assert_eq!(
            crate::decode(&oversized),
            Err(crate::DecodeError::LimitExceeded)
        );
    }
}
