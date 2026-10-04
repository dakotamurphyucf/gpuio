//! Frame-local reading order and logical links for measured rich paragraphs.
use std::{cell::RefCell, collections::BTreeMap, rc::Rc, sync::Arc};

use gpui::{
    AnyElement, App, AvailableSpace, Bounds, ClickEvent, Element, ElementId, GlobalElementId,
    InspectorElementId, InteractiveElement as _, IntoElement, LayoutId, Pixels, SharedString,
    StatefulInteractiveElement as _, Styled as _, Window, div, prelude::FluentBuilder as _, px,
    size,
};

use super::{
    node::LinkMark,
    text_view::{LinkClickHandlerFn, handle_link_click},
};
use crate::GlobalState;

// Keep a direct, platform-filtered accessibility child per visual slot without
// adding a layout box or changing the native element's measurement/input.
struct VisualSlot {
    slot: usize,
    element: AnyElement,
}

impl IntoElement for VisualSlot {
    type Element = Self;
    fn into_element(self) -> Self {
        self
    }
}

impl Element for VisualSlot {
    type RequestLayoutState = ();
    type PrepaintState = ();
    fn id(&self) -> Option<ElementId> {
        Some(("flow-visual", self.slot).into())
    }
    fn source_location(&self) -> Option<&'static core::panic::Location<'static>> {
        None
    }
    fn a11y_role(&self) -> Option<gpui::Role> {
        Some(gpui::Role::GenericContainer)
    }
    fn request_layout(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        window: &mut Window,
        cx: &mut App,
    ) -> (LayoutId, ()) {
        (self.element.request_layout(window, cx), ())
    }
    fn prepaint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        _: Bounds<Pixels>,
        _: &mut (),
        window: &mut Window,
        cx: &mut App,
    ) {
        self.element.prepaint(window, cx);
    }
    fn paint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        _: Bounds<Pixels>,
        _: &mut (),
        _: &mut (),
        window: &mut Window,
        cx: &mut App,
    ) {
        self.element.paint(window, cx);
    }
}

pub(super) fn visual_slot(slot: usize, element: impl IntoElement) -> AnyElement {
    VisualSlot {
        slot,
        element: element.into_any_element(),
    }
    .into_any_element()
}

#[derive(Clone)]
pub(super) struct Collector {
    fragments: Rc<RefCell<Vec<Fragment>>>,
    labels: bool,
    enabled: bool,
}

struct Fragment {
    slot: usize,
    bounds: Bounds<Pixels>,
    text: String,
    link: Option<LinkMark>,
    native: bool,
}

pub(super) struct Run {
    pub slot: usize,
    pub bounds: Bounds<Pixels>,
    pub fragments: Vec<Bounds<Pixels>>,
    pub text: String,
    pub link: Option<LinkMark>,
}

impl Collector {
    pub fn new(window: &Window, cx: &App) -> Self {
        let labels = window.is_a11y_active();
        let enabled = labels
            || GlobalState::global(cx)
                .text_view_state()
                .is_some_and(|view| {
                    let state = view.read(cx);
                    state.max_lines.is_some()
                        || state.link_navigation.active.is_some()
                        || state.link_reveal.is_some()
                });
        Self {
            fragments: Rc::default(),
            labels,
            enabled,
        }
    }

    pub fn enabled(&self) -> bool {
        self.enabled
    }

    pub fn push(&self, slot: usize, bounds: Bounds<Pixels>, text: &str, link: Option<LinkMark>) {
        if self.enabled {
            self.fragments.borrow_mut().push(Fragment {
                slot,
                bounds,
                text: if self.labels {
                    text.to_owned()
                } else {
                    String::new()
                },
                link,
                native: false,
            });
        }
    }

    /// An unlinked native object keeps its real descendants at this position.
    pub fn native(&self, slot: usize) {
        if self.enabled {
            self.fragments.borrow_mut().push(Fragment {
                slot,
                bounds: Bounds::default(),
                text: String::new(),
                link: None,
                native: true,
            });
        }
    }

    pub fn finish(&self) -> Vec<Run> {
        coalesce(std::mem::take(&mut *self.fragments.borrow_mut()))
    }
}

fn coalesce(fragments: Vec<Fragment>) -> Vec<Run> {
    let mut runs: Vec<Run> = Vec::new();
    let mut adjacent = false;
    for fragment in fragments {
        if fragment.native {
            adjacent = false;
            continue;
        }
        if adjacent
            && let Some(link) = &fragment.link
            && link.source_start.is_some()
            && let Some(previous) = runs.last_mut()
            && previous.link.as_ref() == Some(link)
        {
            previous.bounds = previous.bounds.union(&fragment.bounds);
            previous.fragments.push(fragment.bounds);
            previous.text.push_str(&fragment.text);
        } else {
            runs.push(Run {
                slot: fragment.slot,
                bounds: fragment.bounds,
                fragments: vec![fragment.bounds],
                text: fragment.text,
                link: fragment.link,
            });
        }
        adjacent = true;
    }
    runs
}

/// Record links against the actual paint mask, including the whole-line clip.
/// This runs even when accessibility and keyboard link focus are inactive.
pub(super) fn record_preview_link(
    link: &LinkMark,
    bounds: Bounds<Pixels>,
    window: &Window,
    cx: &mut App,
) {
    let Some(source) = link.source_start else {
        return;
    };
    let Some(view) = GlobalState::global(cx).text_view_state().cloned() else {
        return;
    };
    if view.read(cx).max_lines.is_none() {
        return;
    }
    let visible = bounds.intersect(&window.content_mask().bounds);
    if visible.size.width > px(0.) && visible.size.height > px(0.) {
        view.update(cx, |state, _| {
            state.preview_links.insert(source);
        });
    }
}

pub(super) fn activation_allowed(
    target: &Option<(gpui::WeakEntity<super::TextViewState>, Option<usize>)>,
    cx: &App,
) -> bool {
    target.as_ref().is_none_or(|(view, source)| {
        view.upgrade().is_some_and(|view| {
            let state = view.read(cx);
            source.map_or(state.max_lines.is_none(), |source| {
                state.link_is_visible(source)
            })
        })
    })
}

pub(super) fn reveal(runs: &[Run], window: &mut Window, cx: &mut App) {
    let Some(view) = GlobalState::global(cx).text_view_state().cloned() else {
        return;
    };
    let target = {
        let state = view.read(cx);
        state.link_reveal.filter(|_| !state.link_reveal_claimed)
    };
    if let Some(target) = target
        && let Some(run) = runs
            .iter()
            .find(|run| run.link.as_ref().and_then(|link| link.source_start) == Some(target))
        && let Some(first) = run.fragments.first()
    {
        window.request_autoscroll(Bounds::new(first.origin, size(px(2.), first.size.height)));
        view.update(cx, |state, _| state.link_reveal_claimed = true);
    }
}

pub(super) fn elements(
    runs: &[Run],
    owner: Option<&GlobalElementId>,
    handler: &Option<Arc<LinkClickHandlerFn>>,
    window: &mut Window,
    cx: &mut App,
) -> Vec<(usize, AnyElement)> {
    if !window.is_a11y_active() {
        return Vec::new();
    }
    let view = GlobalState::global(cx).text_view_state().cloned();
    let mut occurrences = BTreeMap::<usize, usize>::new();
    let mut active_claimed = false;
    runs.iter()
        .enumerate()
        .map(|(index, run)| {
            let area = run.bounds;
            let id: SharedString = match run.link.as_ref().and_then(|link| link.source_start) {
                Some(source) => {
                    let occurrence = occurrences.entry(source).or_default();
                    let url = &run.link.as_ref().expect("source belongs to a link").url;
                    let id = format!("flow-link-{source}-{occurrence}-{url}");
                    *occurrence += 1;
                    id
                }
                None => format!("flow-text-{index}"),
            }
            .into();
            let mut element = div().id(id).w(area.size.width).h(area.size.height);
            if let Some(link) = &run.link {
                let mut label = run.text.clone();
                let active = view.as_ref().is_some_and(|view| {
                    view.update(cx, |state, _| {
                        // The prepared name includes whitespace elided by line wrapping.
                        if let Some(source) = link.source_start
                            && let Ok(index) = state
                                .link_navigation
                                .links
                                .binary_search_by_key(&source, |entry| entry.source_start)
                            && let Some(prepared) = state.link_navigation.links.get(index)
                            && prepared.url == link.url
                        {
                            label.clone_from(&prepared.label);
                        }
                        if link.source_start.is_none()
                            || state.link_navigation.active != link.source_start
                        {
                            return false;
                        }
                        let Some(owner) = owner else {
                            return false;
                        };
                        if state.link_active_owner.is_none() {
                            state.link_active_owner = Some(owner.clone());
                        }
                        state.link_active_owner.as_ref() == Some(owner)
                    })
                });
                let active = active && !std::mem::replace(&mut active_claimed, true);
                let url = link.url.clone();
                let activation_target = view
                    .as_ref()
                    .map(|view| (view.downgrade(), link.source_start));
                let focus_target = view
                    .as_ref()
                    .filter(|_| link.source_start.is_some())
                    .map(|view| (view.downgrade(), link.clone()));
                let metadata_url = url.clone();
                let handler = handler.clone();
                element = element
                    .role(gpui::Role::Link)
                    .aria_label(label)
                    .when(active, |element| element.aria_active_descendant())
                    .when_some(focus_target, |element, (view, link)| {
                        element.on_a11y_action(
                            gpui::AccessibleAction::Focus,
                            move |_, window, cx| {
                                let _ = view
                                    .update(cx, |state, cx| state.focus_link(&link, window, cx));
                            },
                        )
                    })
                    .a11y_synthetic_children(move |builder| {
                        builder.parent_node().set_url(metadata_url.to_string())
                    })
                    .on_a11y_action(gpui::AccessibleAction::Click, move |_, window, cx| {
                        if !super::inline_semantics::activation_allowed(&activation_target, cx) {
                            return;
                        }
                        handle_link_click(
                            &handler,
                            url.clone(),
                            ClickEvent::Keyboard(gpui::KeyboardClickEvent {
                                bounds: area,
                                ..Default::default()
                            }),
                            window,
                            cx,
                        );
                    });
            } else {
                element = element.role(gpui::Role::Label).aria_value(run.text.clone());
            }
            let mut element = element.into_any_element();
            element.prepaint_as_root(
                area.origin,
                size(
                    AvailableSpace::Definite(area.size.width),
                    AvailableSpace::Definite(area.size.height),
                ),
                window,
                cx,
            );
            (run.slot, element)
        })
        .collect()
}

pub(super) fn paint_focus(runs: &[Run], window: &mut Window, cx: &App) {
    let Some(view) = GlobalState::global(cx).text_view_state() else {
        return;
    };
    let state = view.read(cx);
    if !state.focus_handle().is_focused(window) {
        return;
    }
    let Some(active) = state.link_navigation.active else {
        return;
    };
    for run in runs {
        if run.link.as_ref().and_then(|link| link.source_start) != Some(active) {
            continue;
        }
        for area in &run.fragments {
            window.paint_quad(gpui::quad(
                *area,
                px(2.),
                gpui::transparent_black(),
                gpui::Edges::all(px(1.5)),
                state.text_view_style.link(),
                gpui::BorderStyle::default(),
            ));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use gpui::point;

    fn fragment(slot: usize, text: &str, source: Option<usize>) -> Fragment {
        Fragment {
            slot,
            bounds: Bounds::new(point(px(slot as f32 * 10.), px(0.)), size(px(10.), px(20.))),
            text: text.into(),
            link: Some(LinkMark {
                source_start: source,
                url: "test:same".into(),
                ..Default::default()
            }),
            native: false,
        }
    }

    #[test]
    fn rich_fragments_share_one_action_but_same_url_is_not_identity() {
        let runs = coalesce(vec![
            fragment(0, "Read ", Some(4)),
            fragment(1, "code", Some(4)),
            fragment(2, " 世界", Some(4)),
            fragment(3, "Again", Some(40)),
        ]);
        assert_eq!(runs.len(), 2);
        assert_eq!(runs[0].text, "Read code 世界");
        assert_eq!(runs[0].slot, 0);
        assert_eq!(runs[0].bounds.size, size(px(30.), px(20.)));
        assert_eq!(runs[0].fragments.len(), 3);
        assert_eq!(runs[1].slot, 3);
    }

    #[test]
    fn wrapped_link_keeps_first_fragment_and_unions_painted_bounds() {
        let first = fragment(0, "first ", Some(4));
        let first_bounds = first.bounds;
        let mut second = fragment(1, "line", Some(4));
        second.bounds = Bounds::new(point(px(0.), px(20.)), size(px(60.), px(20.)));
        let runs = coalesce(vec![first, second]);
        assert_eq!(runs.len(), 1);
        assert_eq!(runs[0].bounds.size, size(px(60.), px(40.)));
        assert_eq!(runs[0].fragments[0], first_bounds);
        assert_eq!(runs[0].text, "first line");
    }

    #[test]
    fn native_objects_and_unknown_identities_prevent_accidental_coalescing() {
        let mut native = fragment(1, "", None);
        native.native = true;
        let runs = coalesce(vec![
            fragment(0, "left", Some(4)),
            native,
            fragment(2, "right", Some(4)),
            fragment(3, "one", None),
            fragment(4, "two", None),
        ]);
        assert_eq!(runs.len(), 4);
        assert_eq!(
            runs.iter().map(|run| run.slot).collect::<Vec<_>>(),
            [0, 2, 3, 4]
        );
    }
}
