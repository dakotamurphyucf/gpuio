//! Same native model, editor and focus handles across grouping and appearance.
use super::*;
use gpuio_protocol::color_presentation::{Presentation, Section};

#[::core::prelude::v1::test]
fn groups_sizes_and_theme_retain_composition_slot_focus_and_native_selection() {
    with_picker(|owner, cx, transport| {
        let state = input(owner, cx);
        let (fields, focus) = state.read_with(cx, |s, _| {
            (
                s.editors
                    .fields
                    .iter()
                    .map(|e| e.state.clone())
                    .collect::<Vec<_>>(),
                s.palette_focus.clone(),
            )
        });
        command(owner, cx, c::Command::Focus(c::Field::Hex));
        cx.update(|window, cx| {
            fields[0].update(cx, |s, cx| {
                let length = s.value().encode_utf16().count();
                s.replace_and_mark_text_in_range(
                    Some(0..length),
                    "#abcdef",
                    Some(1..4),
                    window,
                    cx,
                );
            })
        });
        draw(cx);
        let before = state.read_with(cx, |s, _| s.model.snapshot());
        assert!(before.draft.as_ref().unwrap().composing);
        let p = Presentation {
            sections: vec![
                Section {
                    featured: true,
                    label: "Favorites".into(),
                    count: 1,
                },
                Section {
                    featured: false,
                    label: "Colors".into(),
                    count: 2,
                },
            ],
            swatch_size: 32.,
            featured_size: 48.,
            channel_height: 40.,
            selected_border: Some(0x112233ff),
            ..Presentation::default()
        };
        apply(
            owner,
            cx,
            vec![Op::SetColorPresentation(node(), Some(p.clone()))],
        );
        assert_eq!(input(owner, cx).entity_id(), state.entity_id());
        state.read_with(cx, |s, _| {
            assert_eq!(s.model.snapshot(), before);
            assert_eq!(s.palette_focus, focus);
            for (old, field) in fields.iter().zip(&s.editors.fields) {
                assert_eq!(old.entity_id(), field.state.entity_id());
            }
            assert_eq!(s.presentation.as_ref(), &p);
        });
        fields[0].read_with(cx, |s, _| {
            assert_eq!(s.value().as_ref(), "#abcdef");
            assert!(s.bridge_composition().is_some());
        });
        let tree = cx.a11y_tree().unwrap();
        let scale = cx.update(|w, _| f64::from(w.scale_factor()));
        let width = |label| {
            tree.nodes
                .iter()
                .find(|(_, n)| n.label() == Some(label))
                .unwrap()
                .1
                .bounds()
                .unwrap()
                .width()
                / scale
        };
        assert_eq!(width("Translucent red"), 48.);
        assert_eq!(width("Green"), 32.);
        assert_eq!(width("Red again"), 32.);
        assert!(
            tree.nodes
                .iter()
                .any(|(_, n)| n.label() == Some("Favorites") && n.role() == accesskit::Role::Group)
        );
        assert!(
            tree.nodes
                .iter()
                .any(|(_, n)| n.label() == Some("Colors") && n.role() == accesskit::Role::Group)
        );
        let original_position = location(cx, "Red again");
        hover(cx, "Red again");
        assert_eq!(state.read_with(cx, |s, _| s.model.snapshot()), before);
        command(owner, cx, c::Command::Cancel);
        let point = location(cx, "Red again");
        cx.simulate_click(point, Modifiers::default());
        draw(cx);
        assert_eq!(
            state.read_with(cx, |s, _| s.model.snapshot().value),
            Value::Color(config().palette[2].color)
        );
        assert!(
            transport
                .mailbox
                .lock()
                .unwrap()
                .drain(256)
                .iter()
                .any(|event| matches!(
                    event,
                    Event::ColorInputEvent(_, _, _, _, c::Event::Committed(c::Source::Palette, _))
                ))
        );
        cx.update(|window, cx| window.focus(&focus[2], cx));
        let selected = state.read_with(cx, |s, _| s.model.snapshot());
        let regrouped = Presentation {
            sections: vec![Section {
                featured: false,
                label: "All shades".into(),
                count: 3,
            }],
            swatch_size: 40.,
            ..p
        };
        apply(
            owner,
            cx,
            vec![Op::SetColorPresentation(node(), Some(regrouped))],
        );
        cx.update(|window, _| assert!(focus[2].is_focused(window)));
        assert_eq!(state.read_with(cx, |s, _| s.model.snapshot()), selected);
        assert_ne!(location(cx, "Red again"), original_position);
        apply(owner, cx, vec![Op::SetColorPresentation(node(), None)]);
        state.read_with(cx, |s, _| {
            assert_eq!(s.model.snapshot(), selected);
            assert_eq!(s.presentation.as_ref(), &Presentation::default());
        });
        let tree = cx.a11y_tree().unwrap();
        assert!(
            !tree
                .nodes
                .iter()
                .any(|(_, n)| n.label() == Some("All shades"))
        );
        assert_eq!(
            tree.nodes
                .iter()
                .find(|(_, n)| n.label() == Some("Red again"))
                .unwrap()
                .1
                .bounds()
                .unwrap()
                .width()
                / scale,
            28.
        );
    });
}
