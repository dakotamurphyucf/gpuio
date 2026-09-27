//! Revisioned chart uploads. Publish is acknowledged only after atomic commit.
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
    InvalidData,
    Cancelled,
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
