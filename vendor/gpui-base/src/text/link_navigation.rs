//! Logical Markdown links, independent of virtualized rows and shaped fragments.
use std::collections::BTreeMap;

use gpui::SharedString;

use super::{
    document::ParsedDocument,
    inline::accessible_runs,
    node::{BlockNode, LinkMark, NodeContext, Paragraph},
};

#[derive(Clone, Debug, PartialEq)]
pub(super) struct Link {
    pub source_start: usize,
    pub block: usize,
    pub label: String,
    pub url: SharedString,
}

#[derive(Default)]
pub(super) struct Navigation {
    pub links: Vec<Link>,
    pub active: Option<usize>,
}

impl Navigation {
    pub fn refresh(
        &mut self,
        document: &ParsedDocument,
        context: &NodeContext,
        unchanged_prefix: Option<usize>,
    ) {
        let old = self
            .active
            .and_then(|id| self.links.iter().find(|link| link.source_start == id))
            .cloned();
        let mut links = BTreeMap::new();
        for (index, block) in document.blocks.iter().enumerate() {
            collect(block, index, context, &mut links);
        }
        self.links = links.into_values().collect();
        self.active = old
            .filter(|old| {
                unchanged_prefix.is_some_and(|prefix| old.source_start < prefix)
                    && self.links.iter().any(|new| new == old)
            })
            .map(|old| old.source_start);
    }

    /// No wrapping: leaving either edge returns control to the enclosing widget.
    pub fn step(&mut self, backward: bool) -> Option<&Link> {
        let current = self
            .active
            .and_then(|id| self.links.iter().position(|link| link.source_start == id));
        let next = match (current, backward) {
            (Some(index), true) => index.checked_sub(1),
            (Some(index), false) => Some(index + 1),
            (None, true) => self.links.len().checked_sub(1),
            (None, false) => Some(0),
        }
        .and_then(|index| self.links.get(index));
        self.active = next.map(|link| link.source_start);
        next
    }

    pub fn selected(&self) -> Option<&Link> {
        self.active
            .and_then(|id| self.links.iter().find(|link| link.source_start == id))
    }
}

fn add(links: &mut BTreeMap<usize, Link>, block: usize, mark: &LinkMark, text: &str) {
    let Some(id) = mark.source_start else {
        return;
    };
    links
        .entry(id)
        .or_insert_with(|| Link {
            source_start: id,
            block,
            label: String::new(),
            url: mark.url.clone(),
        })
        .label
        .push_str(text);
}

fn paragraph(
    paragraph: &Paragraph,
    block: usize,
    context: &NodeContext,
    links: &mut BTreeMap<usize, Link>,
) {
    for inline in &paragraph.children {
        if let Some(image) = &inline.image {
            if let Some(link) = &image.link {
                add(
                    links,
                    block,
                    &link.resolved(&context.link_refs),
                    &image.title(),
                );
            }
        } else if let Some(custom) = &inline.custom {
            if let Some(link) = inline.marks.iter().find_map(|(_, mark)| mark.link.as_ref()) {
                add(
                    links,
                    block,
                    &link.resolved(&context.link_refs),
                    &custom.shared_accessibility_name(),
                );
            }
        } else {
            let marks = inline
                .marks
                .iter()
                .filter_map(|(range, mark)| {
                    mark.link
                        .as_ref()
                        .map(|link| (range.clone(), link.resolved(&context.link_refs)))
                })
                .collect::<Vec<_>>();
            for (range, link) in accessible_runs(&inline.text, &marks) {
                if let Some(link) = link {
                    add(links, block, &link, &inline.text[range]);
                }
            }
        }
    }
}

fn collect(
    node: &BlockNode,
    block: usize,
    context: &NodeContext,
    links: &mut BTreeMap<usize, Link>,
) {
    match node {
        BlockNode::Root { children, .. }
        | BlockNode::Blockquote { children, .. }
        | BlockNode::List { children, .. }
        | BlockNode::ListItem { children, .. } => {
            for node in children {
                collect(node, block, context, links);
            }
        }
        BlockNode::Paragraph(p) | BlockNode::Heading { children: p, .. } => {
            paragraph(p, block, context, links)
        }
        BlockNode::Table(table) => {
            for row in &table.children {
                for cell in &row.children {
                    paragraph(&cell.children, block, context, links);
                }
            }
        }
        BlockNode::CodeBlock(_)
        | BlockNode::Custom(_)
        | BlockNode::Break { .. }
        | BlockNode::HorizontalRule { .. }
        | BlockNode::Definition { .. }
        | BlockNode::Unknown => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn refresh(navigation: &mut Navigation, source: &str, prefix: Option<usize>) {
        let mut context = NodeContext::default();
        let document = super::super::format::markdown::parse_bounded(source, &mut context).unwrap();
        navigation.refresh(&document, &context, prefix);
    }

    #[test]
    fn logical_links_keep_source_order_across_styles_references_and_blocks() {
        let mut navigation = Navigation::default();
        let source = "[one **bold** `code`](test:one) [same](test:one)\n\n\
                      - [世界][guide]\n\n\
                      | Link |\n| --- |\n| [last](test:last) |\n\n\
                      [guide]: test:unicode\n";
        refresh(&mut navigation, source, None);
        assert_eq!(
            navigation
                .links
                .iter()
                .map(|link| (link.label.as_str(), link.url.as_ref(), link.block))
                .collect::<Vec<_>>(),
            vec![
                ("one bold code", "test:one", 0),
                ("same", "test:one", 0),
                ("世界", "test:unicode", 1),
                ("last", "test:last", 2)
            ]
        );
        assert!(
            navigation
                .links
                .windows(2)
                .all(|pair| pair[0].source_start < pair[1].source_start)
        );
        for expected in ["one bold code", "same", "世界", "last"] {
            assert_eq!(navigation.step(false).unwrap().label, expected);
        }
        assert!(navigation.step(false).is_none());
        assert_eq!(navigation.step(true).unwrap().label, "last");
        assert_eq!(navigation.step(true).unwrap().label, "世界");
    }

    #[test]
    fn appended_links_preserve_focus_but_reset_or_changed_target_clears_it() {
        let mut navigation = Navigation::default();
        let source = "[first](test:first)";
        refresh(&mut navigation, source, None);
        navigation.step(false);
        let next = format!("{source}\n\n[last](test:last)");
        refresh(&mut navigation, &next, Some(source.len()));
        assert_eq!(navigation.selected().unwrap().url.as_ref(), "test:first");
        refresh(
            &mut navigation,
            &next.replace("test:first", "test:other"),
            Some(source.len()),
        );
        assert!(navigation.selected().is_none());
        navigation.step(false);
        refresh(&mut navigation, &next, None);
        assert!(navigation.selected().is_none());
    }
}
