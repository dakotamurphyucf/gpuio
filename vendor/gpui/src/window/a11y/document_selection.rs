//! Resolve paint-time claims against the final tree, once for all Documents.
use accesskit::{NodeId, Role, TextSelection, TreeUpdate};
use collections::{FxHashMap, FxHashSet};

pub(super) fn publish(
    update: &mut TreeUpdate,
    claims: &FxHashMap<NodeId, Option<TextSelection>>,
) {
    if claims.is_empty() {
        return;
    }
    let nodes: FxHashMap<_, _> = update.nodes.iter().map(|(id, node)| (*id, node)).collect();
    let mut pending = vec![(super::ROOT_NODE_ID, None, false)];
    let mut seen = FxHashSet::default();
    let mut documents = FxHashSet::default();
    let mut runs = FxHashMap::default();
    while let Some((id, inherited, excluded)) = pending.pop() {
        if !seen.insert(id) {
            continue;
        }
        let Some(node) = nodes.get(&id) else {
            continue;
        };
        let excluded = excluded || node.is_hidden() || node.is_disabled();
        let scope = if node.role() == Role::Document {
            if !excluded {
                documents.insert(id);
            }
            Some(id)
        } else if independent_text_owner(node.role()) {
            None
        } else {
            inherited
        };
        if !excluded && node.role() == Role::TextRun {
            runs.insert(id, (scope, node.character_lengths().len()));
        }
        pending.extend(node.children().iter().map(|child| (*child, scope, excluded)));
    }
    for (id, node) in &mut update.nodes {
        let Some(selection) = claims.get(id) else {
            continue;
        };
        if node.role() != Role::Document {
            continue;
        }
        let valid = selection.as_ref().filter(|selection| {
            documents.contains(id)
                && [selection.anchor, selection.focus].iter().all(|position| {
                    runs.get(&position.node).is_some_and(|(scope, length)| {
                        *scope == Some(*id) && position.character_index <= *length
                    })
                })
        });
        if let Some(selection) = valid {
            node.set_text_selection(*selection);
        } else {
            node.clear_text_selection();
        }
    }
}

// Match the independent text scopes in the pinned AccessKit consumer's
// text_node_filter/is_text_input. Labels remain part of their outer Document.
fn independent_text_owner(role: Role) -> bool {
    matches!(
        role,
        Role::Terminal
            | Role::TextInput
            | Role::MultilineTextInput
            | Role::SearchInput
            | Role::DateInput
            | Role::DateTimeInput
            | Role::WeekInput
            | Role::MonthInput
            | Role::TimeInput
            | Role::EmailInput
            | Role::NumberInput
            | Role::PasswordInput
            | Role::PhoneNumberInput
            | Role::UrlInput
            | Role::EditableComboBox
            | Role::SpinButton
    )
}
