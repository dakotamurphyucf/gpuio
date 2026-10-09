//! Real deferred tooltip paint on TestPlatform, not physical OS acceptance.
use super::super::popover_semantics_test::{
    apply, ax, draw, events, handler, id as base_id, with_view,
};
use super::*;
use gpui::{Entity, VisualTestContext};
fn id(n: i64) -> NodeId {
    base_id(match n {
        9..=12 => n - 7,
        19..=22 => n - 13,
        29..=32 => n - 19,
        _ => n,
    })
}
fn button(n: i64, label: &str) -> Vec<Op> {
    vec![
        Op::Create(id(n), Kind::Button, label.into(), Some(handler(n))),
        Op::SetControl(id(n), Control::Button(false)),
        Op::SetStyle(
            id(n),
            vec![
                Style::Width(Length::Px(160.)),
                Style::Height(Length::Px(30.)),
            ],
        ),
    ]
}
fn config(n: i64, open_state: TooltipOpenState) -> TooltipConfig {
    TooltipConfig {
        label: format!("Tip {n}"),
        width: 180.,
        open_state,
        disabled: false,
        hoverable: true,
        show_delay_ns: 250_000_000,
        hide_delay_ns: 80_000_000,
        skip_delay_ns: 300_000_000,
    }
}
fn tip(n: i64, x: f64, y: f64) -> Vec<Op> {
    let mut ops = vec![
        Op::Create(id(n - 1), Kind::Container, "".into(), None),
        Op::SetStyle(
            id(n - 1),
            vec![Style::Fields(vec![
                Field::Position(1),
                Field::Left(Length::Px(x)),
                Field::Top(Length::Px(y)),
            ])],
        ),
        Op::Create(id(n), Kind::Tooltip, "".into(), Some(handler(n))),
        Op::SetTooltip(id(n), config(n, TooltipOpenState::Managed(false))),
        Op::SetTooltipMotion(id(n), true),
        Op::SetPlacement(
            id(n),
            Some(Placement {
                side: Side::Bottom,
                align: Align::Start,
                offset: 6.,
            }),
        ),
        Op::SetStyle(id(n), vec![Style::Background(Color::Rgba(0x1144ffff))]),
    ];
    ops.extend(button(n + 1, &format!("Anchor {n}")));
    ops.extend([
        Op::Create(
            id(n + 2),
            Kind::Input,
            "Retained draft".into(),
            Some(handler(n + 2)),
        ),
        Op::SetEditor(
            id(n + 2),
            EditorConfig {
                label: format!("Content {n}"),
                placeholder: "".into(),
                read_only: false,
                disabled: false,
                submit_on_enter: false,
                auto_focus: false,
                min_rows: 1,
                max_rows: 1,
            },
        ),
        Op::SetStyle(
            id(n + 2),
            vec![
                Style::Width(Length::Px(160.)),
                Style::Height(Length::Px(30.)),
            ],
        ),
    ]);
    ops.extend([
        Op::Splice(id(n), 0, 0, vec![id(n + 1), id(n + 2)]),
        Op::Splice(id(n - 1), 0, 0, vec![id(n)]),
    ]);
    ops
}
fn setup(owner: &Entity<View>, cx: &mut VisualTestContext) {
    cx.update(|_, cx| cx.set_reduce_motion(false));
    let mut ops = tip(10, 200., 200.);
    ops.extend(tip(20, 600., 200.));
    ops.extend(tip(30, 600., 400.));
    ops.push(Op::Splice(id(0), 1, 0, vec![id(9), id(19), id(29)]));
    apply(owner, cx, ops);
}
fn input(owner: &Entity<View>, cx: &mut VisualTestContext, n: i64, event: Input) {
    cx.update(|w, cx| owner.update(cx, |v, cx| v.tooltip_input(id(n), event, w, cx)));
    draw(cx);
    draw(cx);
}
fn request(owner: &Entity<View>, cx: &mut VisualTestContext, n: i64, open: bool) {
    cx.update(|w, cx| owner.update(cx, |v, cx| v.tooltip_request(id(n), open, w, cx)));
    draw(cx);
    draw(cx);
}
fn tick(cx: &mut VisualTestContext, ms: u64) {
    cx.executor().advance_clock(Duration::from_millis(ms));
    draw(cx);
    draw(cx);
}
fn bounds(cx: &mut VisualTestContext, n: i64) -> gpui::accesskit::Rect {
    let b = ax(cx, &format!("Tip {n}")).1.bounds().unwrap();
    let scale = cx.update(|w, _| f64::from(w.scale_factor()));
    gpui::accesskit::Rect::new(b.x0 / scale, b.y0 / scale, b.x1 / scale, b.y1 / scale)
}
fn changes(events: Vec<Event>) -> Vec<(NodeId, bool)> {
    events
        .into_iter()
        .filter_map(|e| match e {
            Event::TooltipOpenChanged(_, n, _, _, open) => Some((n, open)),
            _ => None,
        })
        .collect()
}
#[test]
fn entry_and_same_row_replacement_paint_without_replacing_models() {
    with_view(|owner, cx, t| {
        setup(owner, cx);
        input(owner, cx, 10, Input::Anchor(true));
        assert!(!owner.read_with(cx, |v, _| v.tooltips[&id(10)].open));
        tick(cx, 250);
        assert!(owner.read_with(cx, |v, _| v.tooltips[&id(10)].open));
        let retained = owner.read_with(cx, |v, cx| v.editors[&id(12)].focus_handle(cx));
        let first = bounds(cx, 10);
        let stable = ax(cx, "Tip 10").0;
        tick(cx, 75);
        let mid = bounds(cx, 10);
        assert!((first.y0 - mid.y0 - 3.5).abs() < 0.01);
        cx.update(|w, _| {
            let color: gpui::Hsla = rgba(0x1144ffff).into();
            assert!(w.painted_quads().into_iter().any(|q| {
                q.background
                    .as_solid()
                    .is_some_and(|c| c.alpha(1.) == color && (c.a - 0.875).abs() < 0.01)
            }));
        });
        assert_eq!(ax(cx, "Tip 10").0, stable);
        tick(cx, 75);
        let settled = bounds(cx, 10);
        assert!((first.y0 - settled.y0 - 4.).abs() < 0.01);
        draw(cx);
        cx.update(|w, cx| assert_eq!(w.simulate_next_frame(cx), 0));
        events(t);
        // Replacement is immediate despite the new trigger's normal show delay.
        input(owner, cx, 20, Input::Anchor(true));
        assert_eq!(changes(events(t)), vec![(id(10), false), (id(20), true)]);
        assert!(!owner.read_with(cx, |v, _| v.tooltips[&id(10)].open));
        assert_eq!(
            retained,
            owner.read_with(cx, |v, cx| v.editors[&id(12)].focus_handle(cx))
        );
        input(owner, cx, 10, Input::Focus); // stale focus observation after displacement
        assert!(!owner.read_with(cx, |v, _| v.tooltips[&id(10)].open));
        assert!(changes(events(t)).is_empty());
        let switch_start = bounds(cx, 20);
        assert!((switch_start.x0 - settled.x0).abs() < 0.01);
        let identity = ax(cx, "Tip 20").0;
        tick(cx, 100);
        let switch_mid = bounds(cx, 20);
        assert!((switch_mid.x0 - switch_start.x0 - 200.).abs() < 0.01);
        // Route actual pointer input at the moving child, after physical/logical conversion.
        let child = ax(cx, "Content 20").1.bounds().unwrap();
        let scale = cx.update(|w, _| f64::from(w.scale_factor()));
        events(t);
        cx.simulate_click(
            gpui::point(
                px(((child.x0 + child.x1) / 2. / scale) as f32),
                px(((child.y0 + child.y1) / 2. / scale) as f32),
            ),
            gpui::Modifiers::default(),
        );
        draw(cx);
        cx.update(|w, cx| {
            owner.read_with(cx, |v, cx| {
                assert!(v.editors[&id(22)].focus_handle(cx).is_focused(w))
            })
        });
        assert_eq!(ax(cx, "Tip 20").0, identity);
        tick(cx, 100);
        let end = bounds(cx, 20);
        assert!((end.x0 - switch_mid.x0 - 200.).abs() < 0.01);
        // A different trigger row changes immediately, with no fade or sliding frame loop.
        input(owner, cx, 30, Input::Anchor(true));
        let cross = bounds(cx, 30);
        tick(cx, 100);
        assert_eq!(bounds(cx, 30), cross);
        draw(cx);
        cx.update(|w, cx| assert_eq!(w.simulate_next_frame(cx), 0));
    });
}
#[test]
fn controlled_intents_do_not_replace_visible_content_and_motion_updates_keep_deadlines() {
    with_view(|owner, cx, t| {
        setup(owner, cx);
        // Presentation-only metadata leaves a pending show deadline intact.
        input(owner, cx, 10, Input::Anchor(true));
        apply(owner, cx, vec![Op::SetTooltipMotion(id(10), false)]);
        assert!(owner.read_with(cx, |v, _| v.tooltips[&id(10)].has_timer()));
        tick(cx, 250);
        let settled = bounds(cx, 10);
        apply(owner, cx, vec![Op::SetTooltipMotion(id(10), true)]);
        assert_eq!(bounds(cx, 10), settled);
        apply(
            owner,
            cx,
            vec![Op::SetTooltip(
                id(20),
                config(20, TooltipOpenState::Controlled(false)),
            )],
        );
        events(t);
        request(owner, cx, 20, true);
        assert_eq!(changes(events(t)), vec![(id(20), true)]);
        assert!(owner.read_with(cx, |v, _| v.tooltips[&id(10)].open));
        assert!(!owner.read_with(cx, |v, _| v.tooltips[&id(20)].open));
        // The application accepts the swap in one transaction; IDs/content are retained.
        apply(
            owner,
            cx,
            vec![
                Op::SetTooltip(id(10), config(10, TooltipOpenState::Controlled(false))),
                Op::SetTooltip(id(20), config(20, TooltipOpenState::Controlled(true))),
            ],
        );
        let first = bounds(cx, 20);
        tick(cx, 100);
        assert!(bounds(cx, 20).x0 > first.x0);
        request(owner, cx, 20, false);
        assert!(owner.read_with(cx, |v, _| v.tooltips[&id(20)].open));
        apply(
            owner,
            cx,
            vec![Op::SetTooltip(
                id(20),
                config(20, TooltipOpenState::Controlled(false)),
            )],
        );
        draw(cx);
        cx.update(|w, cx| assert_eq!(w.simulate_next_frame(cx), 0));
    });
}
#[test]
fn reduced_hidden_disabled_and_removed_owners_retire_motion_and_history() {
    with_view(|owner, cx, _| {
        setup(owner, cx);
        request(owner, cx, 10, true);
        tick(cx, 50);
        cx.update(|w, cx| {
            cx.set_reduce_motion(true);
            w.refresh();
        });
        draw(cx);
        draw(cx);
        let settled = bounds(cx, 10);
        cx.update(|w, cx| {
            cx.set_reduce_motion(false);
            w.refresh();
        });
        draw(cx);
        assert_eq!(bounds(cx, 10), settled);
        draw(cx);
        cx.update(|w, cx| assert_eq!(w.simulate_next_frame(cx), 0));
        request(owner, cx, 10, false);
        assert!(owner.read_with(cx, |v, _| v.tooltip_previous.is_some()));
        // Disabling the previous owner invalidates the shared positional history.
        let mut disabled = config(10, TooltipOpenState::Managed(false));
        disabled.disabled = true;
        apply(owner, cx, vec![Op::SetTooltip(id(10), disabled)]);
        assert!(owner.read_with(cx, |v, _| v.tooltip_previous.is_none()));
        request(owner, cx, 20, true);
        tick(cx, 30);
        apply(
            owner,
            cx,
            vec![Op::SetStyle(
                id(19),
                vec![Style::Fields(vec![Field::Display(3)])],
            )],
        );
        draw(cx);
        cx.update(|w, cx| assert_eq!(w.simulate_next_frame(cx), 0));
        assert!(owner.read_with(cx, |v, _| v.tooltips[&id(20)].painted.get().is_none()));
        let mut ops = vec![Op::SetRoot(None)];
        for n in [12, 11, 10, 9, 22, 21, 20, 19, 32, 31, 30, 29, 1, 0] {
            ops.push(Op::Remove(id(n)));
        }
        apply(owner, cx, ops);
        draw(cx);
        cx.update(|w, cx| assert_eq!(w.simulate_next_frame(cx), 0));
        assert!(owner.read_with(cx, |v, _| v.tooltips.is_empty()
            && v.tooltip_previous.is_none()
            && v.session.borrow().retained_bytes() == 0));
    });
}

#[test]
fn invisible_paint_and_expired_grace_do_not_seed_a_switch() {
    with_view(|owner, cx, _| {
        setup(owner, cx);
        request(owner, cx, 10, true);
        tick(cx, 150);
        apply(
            owner,
            cx,
            vec![Op::SetStyle(id(10), vec![Style::Opacity(0.)])],
        );
        assert!(owner.read_with(cx, |v, _| v.tooltips[&id(10)].painted.get().is_none()));
        request(owner, cx, 10, false);
        assert!(owner.read_with(cx, |v, _| v.tooltip_previous.is_none()));
        apply(
            owner,
            cx,
            vec![Op::SetStyle(
                id(10),
                vec![Style::Background(Color::Rgba(0x1144ffff))],
            )],
        );
        request(owner, cx, 10, true);
        tick(cx, 150);
        request(owner, cx, 10, false);
        assert!(owner.read_with(cx, |v, _| v.tooltip_previous.is_some()));
        tick(cx, 301);
        request(owner, cx, 20, true);
        let first = bounds(cx, 20);
        assert!(
            (first.x0 - 600.).abs() < 0.01,
            "expired history must not slide from the previous trigger"
        );
        tick(cx, 75);
        assert!((first.y0 - bounds(cx, 20).y0 - 3.5).abs() < 0.01);
        let identity = ax(cx, "Tip 20").0;
        apply(owner, cx, vec![Op::SetTooltipMotion(id(20), false)]);
        let final_bounds = bounds(cx, 20);
        apply(owner, cx, vec![Op::SetTooltipMotion(id(20), true)]);
        assert_eq!(bounds(cx, 20), final_bounds);
        assert_eq!(ax(cx, "Tip 20").0, identity);
        draw(cx);
        cx.update(|w, cx| assert_eq!(w.simulate_next_frame(cx), 0));
    });
}

#[test]
fn replacement_ignores_hidden_peers_and_batches_multiple_visible_managed_tips() {
    with_view(|owner, cx, t| {
        setup(owner, cx);
        request(owner, cx, 10, true);
        tick(cx, 150);
        apply(
            owner,
            cx,
            vec![Op::SetStyle(
                id(9),
                vec![Style::Fields(vec![Field::Display(3)])],
            )],
        );
        events(t);
        request(owner, cx, 20, true);
        assert_eq!(changes(events(t)), vec![(id(20), true)]);
        assert!(
            owner.read_with(cx, |v, _| v.tooltips[&id(10)].open),
            "hidden accepted state is not displaced"
        );
    });
    with_view(|owner, cx, t| {
        setup(owner, cx);
        // Explicit accepted opens may coexist; returning to Managed retains them.
        apply(
            owner,
            cx,
            [10, 20, 30]
                .into_iter()
                .map(|n| Op::SetTooltip(id(n), config(n, TooltipOpenState::Controlled(true))))
                .collect(),
        );
        tick(cx, 150);
        apply(
            owner,
            cx,
            [10, 20, 30]
                .into_iter()
                .map(|n| Op::SetTooltip(id(n), config(n, TooltipOpenState::Managed(false))))
                .collect(),
        );
        request(owner, cx, 10, false);
        events(t);
        request(owner, cx, 10, true);
        assert_eq!(
            changes(events(t)),
            vec![(id(20), false), (id(30), false), (id(10), true)]
        );
        assert!(owner.read_with(cx, |v, _| v.tooltips[&id(10)].open
            && !v.tooltips[&id(20)].open
            && !v.tooltips[&id(30)].open));
    });
}
