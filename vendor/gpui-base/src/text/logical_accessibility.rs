//! Complete unpainted top-level owners in the existing Document tree. Native
//! attachments keep their original descendants/actions; logical nodes have no
//! invented layout geometry or input handlers.
use std::{
    collections::{HashMap, HashSet},
    ops::Range,
};

use super::{
    RenderedAccessiblePartId, RenderedSemanticId, RenderedSemanticKind, RenderedText,
    semantic_attachments::{RenderedSemanticAttachment, TextRun},
};
use gpui::{
    A11ySubtreeBuilder,
    accesskit::{Node, NodeId, Role},
};

pub(super) fn publish(
    projection: &RenderedText,
    native: &[RenderedSemanticAttachment],
    has_native_caret: bool,
    builder: &mut A11ySubtreeBuilder,
) -> Vec<TextRun> {
    let original = builder.parent_node().children().to_vec();
    // This callback is synchronous. Keep completed roots locally until the
    // final merge, so grouping one fresh subtree never scans/copies all earlier
    // siblings. The builder still owns every node in postorder throughout.
    // Keep a typed empty vector: the pinned AccessKit push_child does not
    // restore the vector property after clear_children marks it absent.
    builder.parent_node().set_children(Vec::<NodeId>::new());
    let native: HashMap<_, _> = native
        .iter()
        .map(|item| (item.owner(), item.node()))
        .collect();
    let mapped: HashSet<_> = native.values().copied().collect();
    let mut remaining = original.iter().copied().peekable();
    let mut order = Vec::new();
    let mut runs = Vec::new();
    let mut cursor = 0;
    for owner in projection.semantic_children(None) {
        let span = semantic_span(projection, owner.id());
        if let Some(span) = &span {
            order.extend(emit_text(
                projection,
                cursor..span.start,
                builder,
                &mut runs,
            ));
        }
        if let Some(id) = native.get(&owner.id()) {
            // Unknown native siblings (e.g. a rule) retain their order around
            // actual attachments. They are not substituted with text nodes.
            for child in remaining.by_ref() {
                if child == *id {
                    break;
                }
                if !mapped.contains(&child) {
                    order.push(child);
                }
            }
            order.push(*id);
        } else {
            order.push(emit_owner(projection, owner.id(), builder, &mut runs));
        }
        if let Some(span) = span {
            cursor = span.end;
        }
    }
    let end = projection
        .accessible_parts()
        .last()
        .map_or(0, |part| part.utf16_range().end);
    order.extend(emit_text(projection, cursor..end, builder, &mut runs));
    if end == 0 && !has_native_caret {
        if let Some(part) = projection.accessible_parts().first() {
            order.push(emit_part(projection, part.id(), 0..0, builder, &mut runs));
        }
    }
    order.extend(remaining.filter(|id| !mapped.contains(id)));
    builder.parent_node().set_children(order);
    runs
}

pub(super) fn semantic_span(
    projection: &RenderedText,
    owner: RenderedSemanticId,
) -> Option<Range<usize>> {
    let selection = projection.semantic_selection(owner)?;
    Some(
        projection.accessible_utf16_offset(selection.anchor())?
            ..projection.accessible_utf16_offset(selection.head())?,
    )
}

/// Complete one native owner's direct-child intervals before that owner is
/// finalized. Existing child identities/actions and postorder stay unchanged.
/// Ambiguous/out-of-order intervals leave the original tree untouched.
pub(super) fn complete_native(
    projection: &RenderedText,
    span: Range<usize>,
    children: &[(NodeId, Option<Range<usize>>)],
    builder: &mut A11ySubtreeBuilder,
) -> Option<Vec<TextRun>> {
    let mut cursor = span.start;
    for (_, range) in children {
        if let Some(range) = range {
            if range.start < cursor || range.end > span.end || range.start > range.end {
                return None;
            }
            cursor = range.end;
        }
    }
    builder.parent_node().set_children(Vec::<NodeId>::new());
    let mut order = Vec::new();
    let mut runs = Vec::new();
    let mut cursor = span.start;
    for (child, range) in children {
        if let Some(range) = range {
            order.extend(emit_text(
                projection,
                cursor..range.start,
                builder,
                &mut runs,
            ));
            cursor = range.end;
        }
        order.push(*child);
    }
    order.extend(emit_text(projection, cursor..span.end, builder, &mut runs));
    builder.parent_node().set_children(order);
    Some(runs)
}

// Explicit work stack avoids recursion proportional to document nesting.
enum Task {
    Owner(RenderedSemanticId),
    Text(Range<usize>),
    Finish {
        owner: RenderedSemanticId,
        node: Node,
    },
}

fn emit_owner(
    projection: &RenderedText,
    owner: RenderedSemanticId,
    builder: &mut A11ySubtreeBuilder,
    runs: &mut Vec<TextRun>,
) -> NodeId {
    let mut tasks = vec![Task::Owner(owner)];
    // Each level collects only its direct members; the outer sentinel collects
    // the single completed root. Every child edge is grouped exactly once.
    let mut levels: Vec<Vec<NodeId>> = vec![Vec::new()];
    while let Some(task) = tasks.pop() {
        match task {
            Task::Text(span) => {
                levels
                    .last_mut()
                    .expect("active semantic owner")
                    .extend(emit_text(projection, span, builder, runs));
            }
            Task::Finish { owner, node } => {
                let members = levels.pop().expect("active semantic owner");
                let key = ("logical-document-owner", owner);
                let id = if members.is_empty() {
                    let id = builder.synthetic_node_id(key);
                    assert!(
                        builder.push_child(id, node),
                        "unique prepared semantic owner"
                    );
                    id
                } else {
                    builder.parent_node().set_children(members.clone());
                    builder
                        .group_children(key, node, &members)
                        .expect("new contiguous semantic children")
                };
                builder.parent_node().set_children(Vec::<NodeId>::new());
                levels
                    .last_mut()
                    .expect("enclosing semantic owner")
                    .push(id);
            }
            Task::Owner(owner) => {
                let node = semantic_node(projection, owner);
                levels.push(Vec::new());
                tasks.push(Task::Finish { owner, node });
                let Some(span) = semantic_span(projection, owner) else {
                    continue;
                };
                let mut cursor = span.start;
                let mut children = Vec::new();
                for child in projection.semantic_children(Some(owner)) {
                    if let Some(child_span) = semantic_span(projection, child.id()) {
                        children.push(Task::Text(cursor..child_span.start));
                        children.push(Task::Owner(child.id()));
                        cursor = child_span.end;
                    } else {
                        children.push(Task::Owner(child.id()));
                    }
                }
                children.push(Task::Text(cursor..span.end));
                tasks.extend(children.into_iter().rev());
            }
        }
    }
    let roots = levels.pop().expect("root collector");
    debug_assert!(levels.is_empty() && roots.len() == 1);
    roots[0]
}

fn emit_text(
    projection: &RenderedText,
    span: Range<usize>,
    builder: &mut A11ySubtreeBuilder,
    runs: &mut Vec<TextRun>,
) -> Vec<NodeId> {
    projection
        .accessible_slices(span)
        .map(|(part, chars)| emit_part(projection, part.id(), chars, builder, runs))
        .collect()
}

fn emit_part(
    projection: &RenderedText,
    part: RenderedAccessiblePartId,
    chars: Range<usize>,
    builder: &mut A11ySubtreeBuilder,
    runs: &mut Vec<TextRun>,
) -> NodeId {
    let text = projection
        .accessible_part_slice(part, chars.clone())
        .expect("prepared character range");
    let base = projection
        .accessible_part_byte(part, chars.start)
        .expect("prepared character start");
    let mut start = 0;
    let mut first = chars.start;
    let mut members = Vec::new();
    for end in text
        .match_indices('\n')
        .map(|(offset, _)| offset + 1)
        .chain(std::iter::once(text.len()))
    {
        if start == end && !text.is_empty() {
            continue;
        }
        let last = projection
            .accessible_part_character_at_byte(part, base + end)
            .expect("line boundary");
        let id = builder.synthetic_node_id(("logical-document-run", part, first, last));
        let mut node = Node::new(Role::TextRun);
        node.set_value(text[start..end].to_owned());
        let mut lengths = Vec::with_capacity(last - first);
        let mut characters = text[start..end].chars().peekable();
        while let Some(character) = characters.next() {
            let len = if character == '\r' && characters.peek() == Some(&'\n') {
                characters.next();
                2
            } else {
                character.len_utf8()
            };
            lengths.push(len as u8);
        }
        node.set_character_lengths(lengths);
        assert!(
            builder.push_child(id, node),
            "unique prepared text interval"
        );
        runs.push(TextRun::new(projection, id, part, first..last).expect("prepared text run"));
        members.push(id);
        first = last;
        start = end;
    }
    let mut label = Node::new(Role::Label);
    label.set_value(text.to_owned());
    let id = builder
        .group_children(
            ("logical-document-label", part, chars.start, chars.end),
            label,
            &members,
        )
        .expect("new contiguous text runs");
    builder.parent_node().set_children(Vec::<NodeId>::new());
    id
}

fn semantic_node(projection: &RenderedText, owner: RenderedSemanticId) -> Node {
    use RenderedSemanticKind as Kind;
    let kind = projection
        .semantic_node(owner)
        .expect("prepared owner")
        .kind();
    let role = match kind {
        Kind::Paragraph => Role::Paragraph,
        Kind::Heading { .. } => Role::Heading,
        Kind::List => Role::List,
        Kind::ListItem => Role::ListItem,
        Kind::Link { .. } => Role::Link,
        Kind::Table { .. } => Role::Table,
        Kind::Row { .. } => Role::Row,
        Kind::Cell { header: true, .. } => Role::ColumnHeader,
        Kind::Cell { .. } => Role::Cell,
        Kind::DescriptionList => Role::DescriptionList,
        Kind::Term => Role::Term,
        Kind::Definition => Role::Definition,
        Kind::Group | Kind::Blockquote | Kind::Code | Kind::NativeObject => Role::Group,
    };
    let mut node = Node::new(role);
    match kind {
        Kind::Heading { level } => node.set_level(usize::from(*level)),
        Kind::Table { rows, columns } => {
            node.set_row_count(*rows);
            node.set_column_count(*columns);
        }
        Kind::Row { index } => node.set_row_index(*index),
        Kind::Cell { row, column, .. } => {
            node.set_row_index(*row);
            node.set_column_index(*column);
        }
        Kind::Link { url, .. } => {
            node.set_url(url.to_string());
            if let Some(selection) = projection.semantic_selection(owner) {
                if let Some(label) = projection.selected_text(&selection) {
                    node.set_label(label.to_owned());
                }
            }
        }
        _ => {}
    }
    node
}
