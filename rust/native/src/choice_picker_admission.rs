//! Picker presentation/child checks used by atomic tree admission.
//! Mounted picker rendering and input are implemented separately.
use crate::tree::Node;
use gpuio_protocol::{
    NodeId,
    choice_picker::{Checkmark, Presentation, Slot},
    v1::*,
};
use std::collections::{BTreeMap, BTreeSet};

pub fn validate(presentation: &Presentation) -> Result<(), ErrorCode> {
    if !presentation.has_valid_shape() {
        return Err(ErrorCode::Malformed);
    }
    crate::appearance::validate_parts([
        (presentation.popup_style.as_slice(), &[0, 2][..]),
        (
            presentation.option_style.as_slice(),
            &[0, 1, 2, 3, 6, 7][..],
        ),
        (presentation.header_style.as_slice(), &[0][..]),
        (presentation.empty_style.as_slice(), &[0][..]),
    ])
}

#[derive(Debug, Default, PartialEq, Eq)]
pub struct Children {
    pub trigger: Option<NodeId>,
    pub query: Option<NodeId>,
    pub empty: Option<NodeId>,
    pub footer: Option<NodeId>,
    pub groups: BTreeMap<String, NodeId>,
    pub options: BTreeMap<String, (NodeId, Checkmark)>,
}

fn passive(node: &Node) -> bool {
    matches!(
        node.kind,
        Kind::Container
            | Kind::Text
            | Kind::Image
            | Kind::Icon
            | Kind::Avatar
            | Kind::Loading
            | Kind::Progress
            | Kind::Animated
            | Kind::AnimationProgram
    ) && node.handler.is_none()
        && node.hover_handler.is_none()
        && !node.style.iter().any(|style| {
            let fields: &[Field] = match style {
                Style::Fields(fields) | Style::State(_, fields) => fields,
                _ => &[],
            };
            fields.iter().any(|field| {
                matches!(
                    field,
                    Field::UserSelect(true)
                        | Field::Inert(true)
                        | Field::Disabled(true)
                        | Field::OverflowX(3)
                        | Field::OverflowY(3)
                        | Field::PointerOcclusion(1 | 2)
                )
            })
        })
}

/// Slot order exactly matches the root's child list. Each slot is an empty,
/// unstyled, unbound container with exactly one content child. Wrappers prevent
/// application keys from colliding with structural slot identity. Validate all
/// descendants on dirty-ancestor admission, not just when child edges change.
/// The complete slot forest is limited to 4096 nodes and 128 levels.
pub fn children<'a>(
    presentation: &Presentation,
    ids: &[NodeId],
    get: impl Fn(NodeId) -> Option<&'a Node>,
) -> Result<Children, ErrorCode> {
    validate(presentation)?;
    if ids.len() != presentation.slots.len() {
        return Err(ErrorCode::InvalidTree);
    }
    let mut result = Children::default();
    let mut seen = BTreeSet::new();
    for (id, role) in ids.iter().zip(&presentation.slots) {
        let wrapper = get(*id).ok_or(ErrorCode::InvalidTree)?;
        if wrapper.kind != Kind::Container
            || !wrapper.text.is_empty()
            || !wrapper.style.is_empty()
            || wrapper.handler.is_some()
            || wrapper.hover_handler.is_some()
            || wrapper.children.len() != 1
            || !seen.insert(*id)
        {
            return Err(ErrorCode::InvalidTree);
        }
        let child = get(wrapper.children[0]).ok_or(ErrorCode::InvalidTree)?;
        if child.parent != Some(*id) {
            return Err(ErrorCode::InvalidTree);
        }
        match role {
            Slot::Trigger => result.trigger = Some(child.id),
            Slot::Empty => result.empty = Some(child.id),
            Slot::Footer => result.footer = Some(child.id),
            Slot::Group(id) => {
                result.groups.insert(id.clone(), child.id);
            }
            Slot::Option(id, mark) => {
                result.options.insert(id.clone(), (child.id, *mark));
            }
            Slot::Query => {
                let expected = EditorConfig {
                    label: presentation.config.label.clone(),
                    placeholder: presentation.config.search_placeholder.clone(),
                    read_only: false,
                    disabled: presentation.config.disabled,
                    submit_on_enter: false,
                    auto_focus: false,
                    min_rows: 1,
                    max_rows: 1,
                };
                if child.kind != Kind::Input
                    || child.editor_content_hint.is_some()
                    || child.editor_format.is_some()
                    || child.editor_validation.is_some()
                    || child.editor_clear_on_escape
                    || child.editor_frame.is_some()
                    || child.editor_privacy != gpuio_protocol::v1::EditorPrivacy::Plain
                    || child.editor.as_deref() != Some(&expected)
                    || child.text.contains(['\n', '\r'])
                    || child.handler.is_none()
                    || !child.children.is_empty()
                    || !child.style.is_empty()
                {
                    return Err(ErrorCode::InvalidTree);
                }
                result.query = Some(child.id);
            }
        }
        let mut pending = vec![(child.id, 2)];
        while let Some((id, depth)) = pending.pop() {
            if seen.len() >= 4096 || depth > 128 {
                return Err(ErrorCode::LimitExceeded);
            }
            if !seen.insert(id) {
                return Err(ErrorCode::InvalidTree);
            }
            let node = get(id).ok_or(ErrorCode::InvalidTree)?;
            if !matches!(role, Slot::Footer | Slot::Query) && !passive(node) {
                return Err(ErrorCode::InvalidTree);
            }
            if node.children.len() > 4096usize.saturating_sub(seen.len() + pending.len()) {
                return Err(ErrorCode::LimitExceeded);
            }
            for child in node.children.iter() {
                if get(*child).is_none_or(|child| child.parent != Some(id)) {
                    return Err(ErrorCode::InvalidTree);
                }
                pending.push((*child, depth + 1));
            }
        }
    }
    Ok(result)
}

/// Retained presentation payload. Child nodes are charged individually by Tree.
/// This does not account for future mounted projection/editor/list caches.
pub fn retained_bytes(p: &Presentation) -> usize {
    use gpuio_protocol::choice_picker::{Collection, Selection};
    let items = |items: &[gpuio_protocol::choice_picker::Item]| {
        std::mem::size_of_val(items)
            + items
                .iter()
                .map(|item| item.id.len() + item.label.len())
                .sum::<usize>()
    };
    let config = &p.config;
    std::mem::size_of::<Presentation>()
        + config.label.len()
        + config.placeholder.len()
        + config.search_placeholder.len()
        + match &config.options {
            Collection::Flat(values) => items(values),
            Collection::Grouped(groups) => {
                std::mem::size_of_val(groups.as_slice())
                    + groups
                        .iter()
                        .map(|group| group.id.len() + group.label.len() + items(&group.items))
                        .sum::<usize>()
            }
        }
        + match &config.selected {
            Selection::Single(id) => id.as_ref().map_or(0, String::len),
            Selection::Multiple(ids) => {
                std::mem::size_of_val(ids.as_slice()) + ids.iter().map(String::len).sum::<usize>()
            }
        }
        + p.empty_label.len()
        + std::mem::size_of_val(p.slots.as_slice())
        + p.slots
            .iter()
            .map(|slot| match slot {
                Slot::Group(id) | Slot::Option(id, _) => id.len(),
                Slot::Trigger | Slot::Query | Slot::Empty | Slot::Footer => 0,
            })
            .sum::<usize>()
        + [
            &p.popup_style,
            &p.option_style,
            &p.header_style,
            &p.empty_style,
        ]
        .iter()
        .map(|styles| {
            std::mem::size_of_val(styles.as_slice())
                + styles
                    .iter()
                    .map(crate::style::retained_bytes)
                    .sum::<usize>()
        })
        .sum::<usize>()
}

/// Retained host owner: a cloned Config is bounded by the presentation payload;
/// 4 KiB reserves State, Arc/map metadata and small ownership records. Wrapper
/// arrays and Presentation are Arc-shared with Tree. Each structural slot reserves
/// another 256 bytes for hidden-node and accepted-content maps. Projection/list
/// reservations cover the lazy cache even while the owner is closed.
pub fn owner_reserved_bytes(p: &Presentation) -> usize {
    retained_bytes(p) + 4096 + 256 * p.slots.len() + render_reserved_bytes(p)
}

/// Conservative retained projection/ListState/cursor reservation: 512 bytes per
/// catalog row plus three copies of ID text (projection keys and current/previous
/// GPUI frame identities) and four copies of display/semantic label text,
/// including named group nodes. Mounted captures share the projection rather than
/// copying group text per option. Add 64 KiB of fixed widget/list metadata and two
/// maximum query buffers for searchable controls. Closed owners retain the same
/// reservation. Config Arcs share the already charged owner configuration.
fn render_reserved_bytes(p: &Presentation) -> usize {
    use gpuio_protocol::choice_picker::{Collection, MAX_QUERY_BYTES, Search};
    let items = |items: &[gpuio_protocol::choice_picker::Item]| {
        items.len() * 512
            + items
                .iter()
                .map(|i| 3 * i.id.len() + 4 * i.label.len())
                .sum::<usize>()
    };
    let rows = match &p.config.options {
        Collection::Flat(values) => items(values),
        Collection::Grouped(groups) => groups
            .iter()
            .map(|g| 512 + 3 * g.id.len() + 4 * g.label.len() + items(&g.items))
            .sum(),
    };
    65_536
        + rows
        + if p.config.search == Search::None {
            0
        } else {
            2 * MAX_QUERY_BYTES
        }
}
