use binprot::BinProtRead;
#[cfg(test)]
use binprot::BinProtWrite;
use binprot::macros::BinProtWrite;
use std::io::{Cursor, Read};

pub const MAX_BYTES: usize = 1_048_576;
const MAX_ITEMS: usize = 4096;

#[derive(Clone, Debug, PartialEq)]
pub struct Node {
    pub id: i64,
    pub kind: i64,
    pub text: String,
    pub handler: Option<i64>,
    pub children: Vec<i64>,
}
#[derive(Clone, Debug)]
pub enum Op {
    Upsert(Node),
    Children(i64, Vec<i64>),
    Remove(i64),
    Root(i64),
    Edit(i64, i64, String),
}
#[derive(Clone, Debug)]
pub struct Batch {
    pub base: i64,
    pub next: i64,
    pub ops: Vec<Op>,
}

#[derive(Debug, BinProtWrite)]
pub enum Event {
    Ready,
    Applied(i64, i64, i64),       // revision, node count, operation count
    Click(i64, i64, i64),         // node id, handler id, displayed revision
    Text(i64, i64, String, bool), // id, edit revision, buffer, composing
    Closed,
    Error(String),
    Probe(String),
    Frame(i64),
}

fn int(r: &mut Cursor<&[u8]>) -> Result<i64, String> {
    i64::binprot_read(r).map_err(|e| e.to_string())
}
fn count(r: &mut Cursor<&[u8]>, max: usize) -> Result<usize, String> {
    let n = binprot::Nat0::binprot_read(r).map_err(|e| e.to_string())?.0;
    if n > max as u64 {
        return Err("length limit exceeded".into());
    }
    Ok(n as usize)
}
fn text(r: &mut Cursor<&[u8]>) -> Result<String, String> {
    let n = count(r, MAX_BYTES)?;
    if n > r.get_ref().len().saturating_sub(r.position() as usize) {
        return Err("truncated string".into());
    }
    let mut b = vec![0; n];
    r.read_exact(&mut b).map_err(|e| e.to_string())?;
    String::from_utf8(b).map_err(|e| e.to_string())
}
fn tag(r: &mut Cursor<&[u8]>) -> Result<u8, String> {
    let mut b = [0];
    r.read_exact(&mut b).map_err(|e| e.to_string())?;
    Ok(b[0])
}

pub fn decode(bytes: &[u8]) -> Result<Batch, String> {
    if bytes.len() > MAX_BYTES {
        return Err("batch exceeds size limit".into());
    }
    let r = &mut Cursor::new(bytes);
    if int(r)? != 1 {
        return Err("unsupported protocol version".into());
    }
    let base = int(r)?;
    let next = int(r)?;
    let n = count(r, MAX_ITEMS)?;
    let mut ops = Vec::with_capacity(n);
    for _ in 0..n {
        ops.push(match tag(r)? {
            0 => {
                let id = int(r)?;
                let kind = int(r)?;
                let text = text(r)?;
                let handler = match tag(r)? {
                    0 => None,
                    1 => Some(int(r)?),
                    _ => return Err("invalid option".into()),
                };
                Op::Upsert(Node {
                    id,
                    kind,
                    text,
                    handler,
                    children: vec![],
                })
            }
            1 => {
                let id = int(r)?;
                let n = count(r, MAX_ITEMS)?;
                let mut children = Vec::with_capacity(n);
                for _ in 0..n {
                    children.push(int(r)?);
                }
                Op::Children(id, children)
            }
            2 => Op::Remove(int(r)?),
            3 => Op::Root(int(r)?),
            4 => Op::Edit(int(r)?, int(r)?, text(r)?),
            _ => return Err("unknown operation".into()),
        });
    }
    if r.position() as usize != bytes.len() {
        return Err("trailing bytes".into());
    }
    Ok(Batch { base, next, ops })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn oversized_container_rejected_before_allocation() {
        let mut b = vec![];
        1i64.binprot_write(&mut b).unwrap();
        0i64.binprot_write(&mut b).unwrap();
        1i64.binprot_write(&mut b).unwrap();
        binprot::Nat0(u64::MAX).binprot_write(&mut b).unwrap();
        assert!(decode(&b).unwrap_err().contains("length limit"));
    }
    #[test]
    fn truncated_and_trailing_rejected() {
        assert!(decode(&[]).is_err());
        assert!(decode(&[1, 0, 1, 0, 0]).unwrap_err().contains("trailing"));
    }
}
