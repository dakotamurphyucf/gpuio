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
        }
    }
}
