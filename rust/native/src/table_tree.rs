//! Table admission against the final retained snapshot. No GPUI/OCaml calls.
use super::{Node, Plan};
use gpuio_protocol::{
    NodeId,
    table::Command,
    v1::{ErrorCode, Kind},
};

fn inert_container(node: &Node) -> bool {
    node.kind == Kind::Container
        && node.handler.is_none()
        && node.text.is_empty()
        && node.table.is_none()
}
impl Plan<'_> {
    pub(super) fn validate_table(&self, node: &Node) -> Result<(), ErrorCode> {
        // Payload validity is established by SetTable/SetTableCell. Recheck only
        // cross-node relationships here: a streaming child update must not scan
        // every retained copy string again.
        if let Some(cell) = &node.table_cell {
            if !inert_container(node) || node.children.len() != 1 {
                return Err(ErrorCode::InvalidTree);
            }
            let row = self.node(node.parent.ok_or(ErrorCode::InvalidTree)?)?;
            let owner = self.node(row.parent.ok_or(ErrorCode::InvalidTree)?)?;
            if !owner
                .table
                .as_ref()
                .is_some_and(|config| config.has_column(&cell.column))
                || !owner.list_rows.iter().any(|binding| binding.node == row.id)
            {
                return Err(ErrorCode::InvalidTree);
            }
        }
        let Some(config) = &node.table else {
            return Ok(());
        };
        if node.kind != Kind::VirtualList
            || node.handler.is_none()
            || node.tree_input
            || node.tree_moves
            || node.table_cell.is_some()
            || node.list_config.as_deref() != Some(&config.list_config())
        {
            return Err(ErrorCode::InvalidTree);
        }
        // Generic viewport/retention envelopes carry a handler, but no query.
        // A reset must retire that handler even when it reuses the same order.
        if let Some(old) = self.original.get(node.id)
            && old
                .table
                .as_ref()
                .is_some_and(|previous| previous.query_generation != config.query_generation)
            && old.handler == node.handler
        {
            return Err(ErrorCode::InvalidTree);
        }
        for binding in node.list_rows.iter() {
            let row = self.node(binding.node)?;
            if !inert_container(row)
                || row.table_cell.is_some()
                || row.children.len() != config.schema.columns.len()
            {
                return Err(ErrorCode::InvalidTree);
            }
            for (cell, column) in row.children.iter().zip(&config.schema.columns) {
                let cell = self.node(*cell)?;
                if !inert_container(cell)
                    || cell.children.len() != 1
                    || cell
                        .table_cell
                        .as_ref()
                        .is_none_or(|metadata| metadata.column != column.id)
                {
                    return Err(ErrorCode::InvalidTree);
                }
            }
        }
        Ok(())
    }

    pub(super) fn validate_table_command(
        &mut self,
        id: NodeId,
        command: &Command,
    ) -> Result<(), ErrorCode> {
        let node = self.node(id)?;
        let config = node.table.as_ref().ok_or(ErrorCode::InvalidTree)?;
        let index = node.list_index.as_ref().ok_or(ErrorCode::InvalidTree)?;
        if !command.is_valid()
            || command.serial <= node.table_serial
            || command.query_generation != config.query_generation
            || !config.allows_target(&command.target, |row| index.position(row).is_some())
        {
            return Err(ErrorCode::InvalidTree);
        }
        self.node_mut(id)?.table_serial = command.serial;
        Ok(())
    }
}
