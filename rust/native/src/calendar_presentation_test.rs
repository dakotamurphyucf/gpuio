//! Real native owner and layout on TestPlatform, without an OS window.
use super::*;
use crate::session::Session;
use gpui::{Entity, TestAppContext, VisualTestContext};
use gpuio_protocol::v1::{Length, Style};
use gpuio_protocol::{calendar_presentation::Appearance, v1::*};
use std::{cell::RefCell, os::fd::AsRawFd, os::unix::net::UnixStream, rc::Rc};
pub(super) fn node() -> NodeId {
    NodeId::from_parts(0, 1).unwrap()
}
pub(super) fn date(month: i64, day: i64) -> c::Date {
    c::Date::from_ymd(2024, month, day).unwrap()
}
pub(super) fn config() -> c::Config {
    c::Config {
        mode: c::Mode::Range,
        constraints: c::Constraints::unrestricted(),
        first_weekday: 1,
        labels: c::Labels::english(),
        today: Some(date(2, 29)),
        label: "Dates".into(),
        disabled: false,
        read_only: false,
        auto_focus: false,
    }
}
pub(super) fn apply(owner: &Entity<View>, cx: &mut VisualTestContext, operations: Vec<Op>) {
    cx.update(|window, cx| {
        owner.update(cx, |view, cx| {
            let base = view.session.borrow().tree(view.id).unwrap().revision();
            let result = view
                .session
                .borrow_mut()
                .apply(&Transaction {
                    window: view.id,
                    base,
                    revision: base + 1,
                    operations,
                })
                .unwrap();
            view.update_editors(&result.dirty, window, cx);
            cx.notify();
        });
        window.draw(cx).clear(cx);
    });
    cx.run_until_parked();
    cx.update(|window, cx| window.draw(cx).clear(cx));
}
pub(super) fn command(
    owner: &Entity<View>,
    cx: &mut VisualTestContext,
    command: c::Command,
) -> c::Snapshot {
    cx.update(|window, cx| {
        owner.update(cx, |v, cx| {
            let c::Response::Applied(snapshot) = v.calendars[&node()].command(&command, window, cx)
            else {
                panic!("command rejected");
            };
            snapshot
        })
    })
}
pub(super) fn read(
    owner: &Entity<View>,
    cx: &VisualTestContext,
) -> (
    c::Snapshot,
    crate::calendar_viewport::Viewport,
    gpui::EntityId,
) {
    owner.read_with(cx, |v, cx| {
        let owner = &v.calendars[&node()].state;
        let state = owner.read(cx);
        (state.model.snapshot(), state.viewport, owner.entity_id())
    })
}
#[::core::prelude::v1::test]
fn months_share_range_focus_and_one_accessible_target_per_date() {
    let mut app = TestAppContext::single();
    app.update(gpui_base::init);
    let (_reader, writer) = UnixStream::pair().unwrap();
    let transport = Arc::new(Transport::new(writer.as_raw_fd()).unwrap());
    let session = Rc::new(RefCell::new(Session::default()));
    session.borrow_mut().hello(VERSION, CAPABILITIES).unwrap();
    let id = WindowId::from_parts(0, 1).unwrap();
    session
        .borrow_mut()
        .open(1, id, "Months", 1000., 700.)
        .unwrap();
    let (owner, cx) = app.add_window_view(|_, _| View::new(id, session.clone(), transport.clone()));
    cx.simulate_a11y_active(true);
    let feb = c::Month::new(2024, 2).unwrap();
    apply(
        &owner,
        cx,
        vec![
            Op::Create(
                node(),
                Kind::Calendar,
                "".into(),
                Some(HandlerId::from_parts(0, 1).unwrap()),
            ),
            Op::SetCalendar(node(), Box::new(config()), c::Selection::Empty, feb),
            Op::SetCalendarAppearance(
                node(),
                Some(Appearance {
                    months: 2,
                    ..Appearance::default()
                }),
            ),
            Op::SetRoot(Some(node())),
        ],
    );
    command(&owner, cx, c::Command::FocusDate(date(2, 29)));
    cx.simulate_keystrokes("enter");
    cx.simulate_keystrokes("right");
    cx.simulate_keystrokes("right");
    cx.simulate_keystrokes("enter");
    apply(&owner, cx, vec![]);
    let (snapshot, viewport, identity) = read(&owner, cx);
    assert!(snapshot.is_valid());
    assert_eq!(snapshot.month, c::Month::new(2024, 3).unwrap());
    assert_eq!(
        viewport.first(),
        feb,
        "crossing within the visible span must not jump the panes"
    );
    assert_eq!(
        snapshot.selection,
        c::Selection::Range(c::Range::new(date(2, 29), date(3, 2)).unwrap())
    );
    let tree = cx.a11y_tree().unwrap();
    for label in ["February 29, 2024, Today", "March 1, 2024", "March 2, 2024"] {
        assert_eq!(
            tree.nodes
                .iter()
                .filter(|(_, node)| node.label() == Some(label))
                .count(),
            1,
            "{label}"
        );
    }
    let focused_day = tree
        .nodes
        .iter()
        .find(|(_, n)| n.label() == Some("March 2, 2024"))
        .unwrap()
        .0;
    assert_eq!(
        tree.focus, focused_day,
        "GPUI exports the active day as AccessKit focus"
    );
    let unchanged = command(&owner, cx, c::Command::ShowMonth(snapshot.month));
    assert_eq!(unchanged, snapshot);
    assert_eq!(read(&owner, cx).1.first(), snapshot.month);
    command(&owner, cx, c::Command::FocusDate(date(4, 1)));
    apply(&owner, cx, vec![]);
    assert_eq!(read(&owner, cx).1.first(), c::Month::new(2024, 3).unwrap());
    let before = read(&owner, cx).0;
    apply(
        &owner,
        cx,
        vec![Op::SetCalendarAppearance(
            node(),
            Some(Appearance {
                months: 3,
                cell_height: 40.,
                selected_background: Some(0x11223344),
                ..Appearance::default()
            }),
        )],
    );
    let after = read(&owner, cx);
    assert_eq!(
        after.0, before,
        "presentation does not mutate selection, cursor or revision"
    );
    assert_eq!(after.2, identity);
    assert_eq!(after.1.months().count(), 3);
    apply(
        &owner,
        cx,
        vec![Op::SetStyle(
            node(),
            vec![Style::Fields(vec![Field::Width(Length::Px(320.))])],
        )],
    );
    let tree = cx.a11y_tree().unwrap();
    let bounds = |label| {
        tree.nodes
            .iter()
            .find(|(_, n)| n.label() == Some(label))
            .unwrap()
            .1
            .bounds()
            .unwrap()
    };
    let march = bounds("March 1, 2024");
    let april = bounds("April 1, 2024");
    let scale = cx.update(|window, _| f64::from(window.scale_factor()));
    assert!(
        april.y0 > march.y0 + 100. * scale,
        "narrow containers wrap months without shrinking cells"
    );
    assert!(
        (march.height() - 40. * scale).abs() < 0.01,
        "configured cell height reaches layout: {march:?}; {april:?}"
    );
    apply(&owner, cx, vec![Op::SetCalendarAppearance(node(), None)]);
    assert_eq!(read(&owner, cx).1.first(), before.month);
    assert_eq!(read(&owner, cx).2, identity);
    apply(&owner, cx, vec![Op::SetRoot(None), Op::Remove(node())]);
    owner.read_with(cx, |v, _| assert!(v.calendars.is_empty()));
}
