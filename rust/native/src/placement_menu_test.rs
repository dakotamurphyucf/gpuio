//! Actual native menu point/row geometry on TestPlatform.
use super::super::popover_semantics_test::{apply, draw, events, handler, id, with_view};
use super::*;
use gpuio_protocol::placement_geometry::{Config, Corner, Point};
fn geometry(corner: Corner, x: f64, y: f64, margin: f64) -> Option<Config> {
    Some(Config {
        viewport_margin: margin,
        point: Some(Point { corner, x, y }),
    })
}
fn near(actual: f64, expected: f64) {
    assert!((actual - expected).abs() < 0.02, "{actual} != {expected}");
}
#[test]
fn menu_root_uses_point_but_submenu_uses_its_row_and_shared_margin() {
    with_view(|owner, cx, transport| {
        cx.simulate_resize(gpui::size(px(800.), px(600.)));
        apply(
            owner,
            cx,
            vec![
                Op::Create(id(2), Kind::CommandScope, "".into(), Some(handler(2))),
                Op::SetCommands(
                    id(2),
                    vec![CommandConfig {
                        id: "run".into(),
                        generation: 1,
                        label: "Run".into(),
                        enabled: true,
                        checked: None,
                        shortcuts: vec![],
                        target: CommandTarget::Callback,
                    }],
                ),
                Op::Create(id(3), Kind::Menu, "".into(), Some(handler(3))),
                Op::SetMenu(
                    id(3),
                    MenuConfig {
                        presentation: MenuPresentation::Button,
                        menus: vec![MenuDefinition {
                            label: "Placed menu".into(),
                            disabled: false,
                            items: vec![MenuItem::Submenu(MenuDefinition {
                                label: "More".into(),
                                disabled: false,
                                items: vec![MenuItem::Command("run".into())],
                            })],
                        }],
                    },
                ),
                Op::SetChoiceAppearance(
                    id(3),
                    ChoiceAppearance {
                        popup_width: 160.,
                        ..crate::appearance::default().as_ref().clone()
                    },
                ),
                Op::SetPlacementGeometry(id(3), geometry(Corner::TopLeft, 300., 200., 32.)),
                Op::Splice(id(2), 0, 0, vec![id(3)]),
                Op::Splice(id(0), 1, 0, vec![id(2)]),
            ],
        );
        cx.update(|w, cx| owner.update(cx, |v, cx| v.open_menu(id(3), 0, None, w, cx)));
        draw(cx);
        draw(cx);
        let retained = owner.read_with(cx, |v, _| v.menus[&id(3)].clone());
        let root = retained.borrow().panels[0].get();
        near(f64::from(root.left()), 301.);
        near(f64::from(root.top()), 201.);
        cx.simulate_keystrokes("right");
        draw(cx);
        draw(cx);
        let child = retained.borrow().panels[1].get();
        let row = retained.borrow().rows.borrow()[&(0, 0)].get();
        near(f64::from(child.left() - row.right()), 3.);
        assert_ne!(
            child.origin, root.origin,
            "submenu must not reuse the root point"
        );
        events(transport);
        apply(
            owner,
            cx,
            vec![Op::SetPlacementGeometry(
                id(3),
                geometry(Corner::TopLeft, 1e6, 1e6, 32.),
            )],
        );
        assert!(owner.read_with(cx, |v, _| Rc::ptr_eq(&retained, &v.menus[&id(3)])));
        {
            let state = retained.borrow();
            assert_eq!(state.path.len(), 2);
            let root = state.panels[0].get();
            let child = state.panels[1].get();
            near(f64::from(root.right()), 767.);
            let row = state.rows.borrow()[&(0, 0)].get();
            near(f64::from(row.left() - child.right()), 3.);
            for b in [root, child] {
                assert!(
                    b.left() >= px(33.)
                        && b.right() <= px(767.)
                        && b.top() >= px(33.)
                        && b.bottom() <= px(567.)
                );
            }
        }
        assert!(
            events(transport).is_empty(),
            "geometry is not a menu visibility change"
        );
        cx.simulate_keystrokes("enter");
        draw(cx);
        assert!(
            events(transport)
                .iter()
                .any(|e| matches!(e, Event::CommandInvoked(..)))
        );
    });
}
