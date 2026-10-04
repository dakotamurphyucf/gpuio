//! Table admission against the final retained snapshot. No GPUI/OCaml calls.
use super::{Node, Plan};
use gpuio_protocol::{
    NodeId,
    table::Command,
    v1::{ErrorCode, Kind},
};
use std::collections::BTreeMap;

fn inert_container(node: &Node) -> bool {
    node.kind == Kind::Container
        && node.handler.is_none()
        && node.text.is_empty()
        && node.table.is_none()
}
impl Plan<'_> {
    pub(super) fn validate_table(&self, node: &Node) -> Result<(), ErrorCode> {
        if !node.table_header_style.is_empty() && node.table.is_none() {
            return Err(ErrorCode::InvalidTree);
        }
        if !node.table_row_style.is_empty() {
            let owner = self.node(node.parent.ok_or(ErrorCode::InvalidTree)?)?;
            if !inert_container(node)
                || node.table_cell.is_some()
                || node.table_header.is_some()
                || owner.table.is_none()
                || !owner.list_rows.iter().any(|row| row.node == node.id)
            {
                return Err(ErrorCode::InvalidTree);
            }
        }
        if let Some(target) = &node.table_header {
            if !inert_container(node)
                || node.table_cell.is_some()
                || node.children.len() != 1
                || !node.style.is_empty()
                || node.hover_handler.is_some()
                || node.accessibility.is_some()
                || node.focus_scope.is_some()
                || node.highlight_scope.is_some()
                || node.command_binding.is_some()
                || node.commands.is_some()
                || node.command_ref.is_some()
                || node.overlay.is_some()
                || node.tab_order.is_some()
                || node.scrollbar.is_some()
                || node.drag_source.is_some()
                || node.drop_target.is_some()
                || node.pointer.is_some()
                || node.input_region.is_some()
                || node.placement.is_some()
                || node.placement_geometry.is_some()
                || node.popover
                || node.reveal.is_some()
            {
                return Err(ErrorCode::InvalidTree);
            }
            let owner = self.node(node.parent.ok_or(ErrorCode::InvalidTree)?)?;
            if !owner
                .table
                .as_ref()
                .is_some_and(|config| target.matches(&config.schema))
            {
                return Err(ErrorCode::InvalidTree);
            }
        }
        // Payload validity is established by SetTable/SetTableCell. Recheck only
        // cross-node relationships here: a streaming child update must not scan
        // every retained copy string again.
        if let Some(cell) = &node.table_cell {
            if !inert_container(node) || node.table_header.is_some() || node.children.len() != 1 {
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
            return if node.table_behavior.is_none() && node.table_appearance.is_none() {
                Ok(())
            } else {
                Err(ErrorCode::InvalidTree)
            };
        };
        // Header-only edits dirty their ancestor but need not otherwise edit
        // its record. Validate ownership and schema fences on that final tree.
        self.validate_list(node)?;
        let mut headers = BTreeMap::new();
        for child in node.children.iter() {
            let child = self.node(*child)?;
            if let Some(target) = &child.table_header
                && (!target.matches(&config.schema)
                    || headers.insert(target.clone(), child.id).is_some())
            {
                return Err(ErrorCode::InvalidTree);
            }
        }
        if let Some(old) = self.original.get(node.id)
            && let Some(previous) = &old.table
        {
            let previous_headers = old
                .children
                .iter()
                .filter_map(|id| {
                    let child = self.original.get(*id)?;
                    Some((child.table_header.clone()?, child.id))
                })
                .collect::<BTreeMap<_, _>>();
            if previous_headers != headers && config.schema_revision <= previous.schema_revision {
                return Err(ErrorCode::InvalidTree);
            }
        }
        if node
            .table_behavior
            .as_ref()
            .is_some_and(|b| !b.valid_schema(config))
            || self.original.get(node.id).is_some_and(|old| {
                old.table_behavior != node.table_behavior
                    && old
                        .table
                        .as_ref()
                        .is_some_and(|previous| config.schema_revision <= previous.schema_revision)
            })
        {
            return Err(ErrorCode::InvalidTree);
        }
        if node
            .table_appearance
            .as_ref()
            .is_some_and(|a| !a.valid_schema(config))
            || self.original.get(node.id).is_some_and(|old| {
                !gpuio_protocol::table::Appearance::geometry_equal(
                    old.table_appearance.as_deref(),
                    node.table_appearance.as_deref(),
                ) && old
                    .table
                    .as_ref()
                    .is_some_and(|previous| config.schema_revision <= previous.schema_revision)
            })
        {
            return Err(ErrorCode::InvalidTree);
        }
        if node.kind != Kind::VirtualList
            || node.handler.is_none()
            || node.tree_input
            || node.tree_moves
            || node.table_cell.is_some()
            || node.table_header.is_some()
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
                || row.table_header.is_some()
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
            || node
                .table_behavior
                .as_ref()
                .is_some_and(|b| !b.allows_target(&command.target))
        {
            return Err(ErrorCode::InvalidTree);
        }
        self.node_mut(id)?.table_serial = command.serial;
        Ok(())
    }
}
