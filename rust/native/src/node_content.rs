//! Component-specific child construction. Keep these large builder temporaries
//! out of the common recursive container frame. Rich controls may recurse into
//! accepted slots here; ordinary containers return before traversing children.
use super::*;

pub(super) struct Render<'a> {
    pub node: &'a crate::tree::Node,
    pub interaction: Interaction,
    pub input_presentation: Interaction,
    pub disabled: bool,
    pub popup_priority: usize,
    pub label: Arc<str>,
    pub selected_style: Option<gpui::StyleRefinement>,
    pub scrolling: &'a Option<Rc<scroll::State>>,
}

impl View {
    pub(super) fn node_content(
        &mut self,
        tree: &crate::tree::Tree,
        render: Render<'_>,
        mut element: gpui::Stateful<gpui::Div>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> gpui::Stateful<gpui::Div> {
        let Render {
            node,
            interaction,
            input_presentation,
            disabled,
            popup_priority,
            label,
            selected_style,
            scrolling,
        } = render;
        let id = node.id;
        if node.kind == Kind::ChoicePicker {
            element = self.picker_element(
                element,
                choice_picker_view::Render {
                    tree,
                    node,
                    interaction,
                    priority: popup_priority,
                },
                window,
                cx,
            );
        } else if node.kind == Kind::Combobox {
            let (editor, state) = self.editors[&id]
                .combobox()
                .expect("validated combobox editor");
            let route = node
                .handler
                .filter(|_| !disabled)
                .map(|handler| choice::Route {
                    window: self.id,
                    node: id,
                    handler,
                    revision: tree.revision(),
                    session: self.session.clone(),
                    gate: self.focus.clone(),
                    transport: self.transport.clone(),
                });
            element = combobox::element(
                element,
                combobox::Render {
                    priority: popup_priority,
                    editor,
                    state,
                    config: node.choice.as_ref().expect("validated combobox choices"),
                    appearance: node
                        .choice_appearance
                        .clone()
                        .unwrap_or_else(crate::appearance::default),
                    filter: node.combobox_filter.expect("validated combobox filter"),
                    route,
                    pointer: interaction.pointer,
                    selected_style,
                },
                window,
                cx,
            );
        } else if let Some(color) = self.color_inputs.get(&id) {
            self.visited.insert(id);
            element = color.element(element, interaction.pointer, cx);
        } else if let Some(calendar) = self.calendars.get(&id) {
            self.visited.insert(id);
            element = calendar.element(element, interaction, cx);
        } else if let Some(otp) = self.otps.get(&id) {
            self.visited.insert(id);
            element = otp.element(element, interaction.pointer, cx);
        } else if self.numbers.contains_key(&id) {
            self.visited.insert(id);
            let presentation = self.number_frame_parts(tree, node, interaction, window, cx);
            element =
                self.numbers[&id].element(element, interaction.pointer, presentation, window, cx);
        } else if node.slider.is_some() {
            self.visited.insert(id);
            if let Some(state) = self.sliders.get(&id) {
                element = slider_view::element(element, state.clone(), interaction.pointer, window);
            }
        } else if let Some(config) = &node.rating {
            let state = self.ratings.entry(id).or_default().clone();
            let route = node
                .handler
                .filter(|_| !disabled && !config.read_only)
                .map(|handler| choice::Route {
                    window: self.id,
                    node: id,
                    handler,
                    revision: tree.revision(),
                    session: self.session.clone(),
                    gate: self.focus.clone(),
                    transport: self.transport.clone(),
                });
            element = rating::element(
                element,
                rating::Render {
                    config,
                    appearance: node.rating_appearance.as_deref().copied(),
                    state,
                    focus: self.buttons[&id].focus.clone(),
                    route,
                    pointer: interaction.pointer,
                },
                window,
                cx,
            );
        } else if let Some(config) = &node.choice {
            element = element.aria_label(config.label.clone());
            let focus = self.buttons[&id].focus.clone();
            let route = node
                .handler
                .filter(|_| !disabled)
                .map(|handler| choice::Route {
                    window: self.id,
                    node: id,
                    handler,
                    revision: tree.revision(),
                    session: self.session.clone(),
                    gate: self.focus.clone(),
                    transport: self.transport.clone(),
                });
            if node.kind == Kind::Select {
                let state = self.selects.entry(id).or_default().clone();
                element = select::element(
                    element,
                    select::Render {
                        priority: popup_priority,
                        config,
                        appearance: node
                            .choice_appearance
                            .clone()
                            .unwrap_or_else(crate::appearance::default),
                        state,
                        focus,
                        route,
                        pointer: interaction.pointer,
                        menu: node.choice_menu,
                        decoration: self.tab_menu_icons(node, interaction, cx),
                        selected_style,
                    },
                    window,
                    cx,
                );
            } else {
                let state = self.radios.entry(id).or_default().clone();
                let viewport = node.tab_viewport.as_ref().map(|viewport| {
                    let state = self.tab_viewports.entry(id).or_default().clone();
                    state.borrow_mut().begin(config, viewport);
                    tab_viewport::Binding {
                        state,
                        scroll: scrolling.clone().expect("tab viewport is scrollable"),
                    }
                });
                let trailing = if node.tab_trailing {
                    let slot = tree
                        .get(*node.children.last().expect("validated trailing slot"))
                        .expect("validated trailing slot");
                    slot.children
                        .first()
                        .map(|child| self.element(tree, *child, interaction, window, cx))
                } else {
                    None
                };
                let parts = if node.tab_content.is_some() {
                    node.children
                        .iter()
                        .take(config.items.len())
                        .enumerate()
                        .map(|(index, slot)| {
                            let slot = tree.get(*slot).expect("validated tab slot");
                            let render_part =
                                |this: &mut Self,
                                 part: usize,
                                 window: &mut Window,
                                 cx: &mut Context<Self>| {
                                    tree.get(slot.children[part])
                                        .and_then(|part| part.children.first())
                                        .map(|child| {
                                            this.element(tree, *child, interaction, window, cx)
                                        })
                                };
                            radio::Parts {
                                prefix: render_part(self, 0, window, cx),
                                label: tree
                                    .get(slot.children[1])
                                    .and_then(|part| part.children.first())
                                    .map(|child| {
                                        self.control_label(
                                            tree,
                                            *child,
                                            interaction,
                                            disabled || config.items[index].disabled,
                                            window,
                                            cx,
                                        )
                                    }),
                                suffix: render_part(self, 2, window, cx),
                            }
                        })
                        .collect()
                } else {
                    Vec::new()
                };
                let labels = node
                    .children
                    .iter()
                    .take(config.items.len())
                    .enumerate()
                    .map(|(index, slot)| {
                        if node.tab_content.is_some() {
                            return None;
                        }
                        let slot_node = tree.get(*slot).expect("validated radio slot");
                        (!slot_node.children.is_empty()).then(|| {
                            self.control_label(
                                tree,
                                *slot,
                                interaction,
                                disabled || config.items[index].disabled,
                                window,
                                cx,
                            )
                        })
                    })
                    .collect();
                element = radio::element(
                    element,
                    radio::Render {
                        tabs: node.kind == Kind::TabBar,
                        tab_appearance: node.tab_appearance.as_deref(),
                        tab_content: node.tab_content.as_deref(),
                        viewport,
                        motion: node.tab_motion.as_ref().map(|config| tab_motion::Owner {
                            state: self.tab_motions.entry(id).or_default().clone(),
                            config: **config,
                        }),
                        trailing,
                        parts,
                        gate: self.focus.clone(),
                        node: id,
                        labels,
                        disabled,
                        appearance: node
                            .control_appearance
                            .as_deref()
                            .unwrap_or_else(|| crate::control_appearance::default()),
                        config,
                        state,
                        focus,
                        route,
                        pointer: interaction.pointer,
                        selected_style,
                    },
                    window,
                    cx,
                );
            }
        } else if let Some(editor) = self.editors.get(&id) {
            let next = self.focus.clone();
            let previous = self.focus.clone();
            element = element
                .capture_action(move |_: &gpui_base::input::IndentInline, window, cx| {
                    next.borrow().traverse(false, window, cx);
                    cx.stop_propagation();
                })
                .capture_action(move |_: &gpui_base::input::OutdentInline, window, cx| {
                    previous.borrow().traverse(true, window, cx);
                    cx.stop_propagation();
                });
            if node.editor_frame.is_some() {
                element = self.editor_frame_element(element, tree, node, interaction, window, cx);
            } else {
                element = element.child(editor.element());
            }
        } else if node.kind == Kind::Text && interaction.selectable.unwrap_or(false) {
            self.visited.insert(id);
            let selection = self
                .selections
                .entry(id)
                .or_insert_with(|| {
                    crate::selection::State::new(node.text.clone(), cx.entity_id(), cx)
                })
                .clone();
            let changed_geometry = {
                let state = selection.borrow();
                state.text != node.text && state.has_geometry(cx)
            };
            if changed_geometry {
                gpui_base::TextSelection::clear(window, cx);
            }
            selection.borrow_mut().update(node.text.clone());
            let highlight = self.highlight_for(tree, id);
            let selection_scope = self.focus.borrow().selection_scope(id);
            let shimmer = self.text_shimmer(node, window);
            element = element.child(crate::selection::element(
                &node.text_spans,
                selection,
                interaction
                    .selection_color
                    .unwrap_or_else(|| rgba(0x386ac880).into()),
                interaction.pointer,
                cx.entity_id(),
                selection_scope,
                highlight,
                shimmer,
            ));
        } else if matches!(node.kind, Kind::Checkbox | Kind::Switch | Kind::Radio) {
            element = self.checkable_element(
                tree,
                checkable::Render {
                    node,
                    interaction,
                    disabled,
                },
                element,
                window,
                cx,
            );
        } else if node
            .button_presentation
            .is_some_and(|config| config.content == gpuio_protocol::button::Content::Rich)
        {
            element = element.child(self.control_label(
                tree,
                node.children[0],
                input_presentation,
                disabled,
                window,
                cx,
            ));
        } else if matches!(node.kind, Kind::Button | Kind::CommandButton)
            && !node.children.is_empty()
        {
            let [leading, trailing] = node.children.as_ref() else {
                unreachable!("validated button icon slots")
            };
            if tree
                .get(*leading)
                .is_some_and(|slot| !slot.children.is_empty())
            {
                element = element.child(self.element(tree, *leading, interaction, window, cx));
            }
            if !label.is_empty() {
                element = element.child(gpui::SharedString::from(label));
            }
            if tree
                .get(*trailing)
                .is_some_and(|slot| !slot.children.is_empty())
            {
                element = element.child(self.element(tree, *trailing, interaction, window, cx));
            }
        } else if !label.is_empty()
            && !matches!(
                node.kind,
                Kind::TabPanel
                    | Kind::Panel
                    | Kind::NavigationStack
                    | Kind::Carousel
                    | Kind::CarouselTrack
            )
        {
            let highlight = self.highlight_for(tree, id);
            let shimmer = self.text_shimmer(node, window);
            if highlight.is_some() || shimmer.is_some() || !node.text_spans.is_empty() {
                let text = crate::styled_text::element(
                    gpui::SharedString::from(label.clone()),
                    &node.text_spans,
                );
                if let Some((paint, cache)) = highlight {
                    element = element.child(crate::highlight_paint::underlay(
                        label,
                        text.layout().clone(),
                        paint,
                        cache,
                    ));
                }
                element = element.child(crate::text_shimmer_clock::decorate(text, shimmer));
            } else {
                element = element.child(gpui::SharedString::from(label));
            }
        }
        element
    }
}
