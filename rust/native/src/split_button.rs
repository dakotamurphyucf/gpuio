//! Bounded shape and paint admission for the split composition.
use crate::tree::Node;
use gpuio_protocol::{
    NodeId,
    split_button::{Config, Parts},
    v1::*,
};

pub(crate) struct Owners {
    pub primary: Option<NodeId>,
    pub menu: Option<NodeId>,
}

pub(crate) fn owners<'a>(
    root: &'a Node,
    get: impl Fn(NodeId) -> Option<&'a Node>,
) -> Result<Owners, ErrorCode> {
    let config = root.split_button.as_ref().ok_or(ErrorCode::InvalidTree)?;
    if root.kind != Kind::Container
        || root.handler.is_some()
        || !root.text.is_empty()
        || root.children.len() != if config.parts == Parts::Split { 2 } else { 1 }
    {
        return Err(ErrorCode::InvalidTree);
    }
    let owner = |index: usize, menu: bool| -> Result<NodeId, ErrorCode> {
        let slot = get(root.children[index]).ok_or(ErrorCode::InvalidTree)?;
        if slot.kind != Kind::Container
            || slot.children.len() != 1
            || slot.handler.is_some()
            || !slot.text.is_empty()
            || slot.split_button.is_some()
        {
            return Err(ErrorCode::InvalidTree);
        }
        let mut node = get(slot.children[0]).ok_or(ErrorCode::InvalidTree)?;
        for depth in 0..=8 {
            if node.kind == Kind::Tooltip && depth < 8 && node.children.len() == 2 {
                node = get(node.children[0]).ok_or(ErrorCode::InvalidTree)?;
            } else {
                let valid = if menu {
                    node.kind == Kind::Menu
                        && node
                            .menu
                            .as_ref()
                            .is_some_and(|config| config.presentation == MenuPresentation::Button)
                } else {
                    matches!(node.kind, Kind::Button | Kind::CommandButton)
                };
                return if valid {
                    Ok(node.id)
                } else {
                    Err(ErrorCode::InvalidTree)
                };
            }
        }
        Err(ErrorCode::InvalidTree)
    };
    Ok(match config.parts {
        Parts::Primary => Owners {
            primary: Some(owner(0, false)?),
            menu: None,
        },
        Parts::Menu => Owners {
            primary: None,
            menu: Some(owner(0, true)?),
        },
        Parts::Split => Owners {
            primary: Some(owner(0, false)?),
            menu: Some(owner(1, true)?),
        },
    })
}

pub(crate) fn validate(config: &Config) -> Result<(), ErrorCode> {
    if !config.has_valid_shape() {
        return Err(ErrorCode::Malformed);
    }
    crate::tree::validate_style(&config.surface)?;
    crate::tree::validate_style(&config.menu_open)
}

pub(crate) fn retained_bytes(config: &Config) -> usize {
    std::mem::size_of::<Config>()
        + [&config.surface, &config.menu_open]
            .into_iter()
            .map(|styles| {
                std::mem::size_of_val(styles.as_slice())
                    + styles
                        .iter()
                        .map(crate::style::retained_bytes)
                        .sum::<usize>()
            })
            .sum::<usize>()
}
