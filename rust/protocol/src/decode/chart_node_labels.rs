use super::{DecodeError, Decoder};
use crate::chart_node_labels::{self, Line, MAX_BYTES, MAX_LINE_BYTES, MAX_LINES, MAX_NODES, Node};
use std::io::Cursor;

impl Decoder<'_> {
    pub(super) fn chart_node_labels(&mut self) -> Result<Vec<Node>, DecodeError> {
        let nodes = self.list(MAX_NODES, |d| {
            Ok(Node {
                node: d.int()?,
                lines: d.list(MAX_LINES, |d| {
                    Ok(Line {
                        text: d.bounded_text(MAX_LINE_BYTES)?,
                        color: d.option(|d| d.int())?,
                        font_size: d.option(|d| d.float())?,
                    })
                })?,
            })
        })?;
        if chart_node_labels::is_valid(&nodes) {
            Ok(nodes)
        } else {
            Err(DecodeError::Malformed)
        }
    }
}

pub fn decode_chart_node_labels(bytes: &[u8]) -> Result<Vec<Node>, DecodeError> {
    if bytes.len() > MAX_BYTES {
        return Err(DecodeError::LimitExceeded);
    }
    let mut d = Decoder(Cursor::new(bytes));
    let nodes = d.chart_node_labels()?;
    if d.remaining() != 0 {
        return Err(DecodeError::Malformed);
    }
    Ok(nodes)
}
