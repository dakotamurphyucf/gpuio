//! Deferred TestPlatform appearance notifications through production observers.
use super::input_tests::{apply, draw, mount};
use super::*;
use gpui::{Entity, TestAppContext, VisualTestContext, WindowAppearance as Native};

fn watch_owner(owner: &Entity<View>, cx: &mut VisualTestContext) {
    cx.update(|w, cx| owner.update(cx, |view, cx| watch_appearance(view, w, cx)));
}
fn drain(owner: &Entity<View>, cx: &mut VisualTestContext) -> Vec<Event> {
    cx.update(|_, cx| owner.read(cx).transport.mailbox.lock().unwrap().drain(128))
}
fn appearances(events: Vec<Event>) -> Vec<wire::Appearance> {
    events
        .into_iter()
        .filter_map(|e| match e {
            Event::WindowChanged(_, s) => Some(s.appearance),
            _ => None,
        })
        .collect()
}
#[test]
fn initial_and_changed_appearance_use_actual_native_state_and_coalesce() {
    let mut app = TestAppContext::single();
    let (owner, mut cx, _reader) = mount(&mut app);
    // Change before watching/first paint: initial snapshot must not assume Light.
    cx.simulate_appearance_change(Native::VibrantDark);
    cx.run_until_parked();
    cx.update(|w, cx| {
        owner.update(cx, |view, cx| {
            watch_appearance(view, w, cx);
            observe(view, w);
            assert_eq!(snapshot(view, w).appearance, wire::Appearance::VibrantDark);
        })
    });
    assert_eq!(
        appearances(drain(&owner, &mut cx)),
        vec![wire::Appearance::VibrantDark]
    );
    for (native, expected) in [
        (Native::Light, wire::Appearance::Light),
        (Native::VibrantLight, wire::Appearance::VibrantLight),
        (Native::Dark, wire::Appearance::Dark),
        (Native::VibrantDark, wire::Appearance::VibrantDark),
    ] {
        cx.simulate_appearance_change(native);
        cx.run_until_parked();
        assert_eq!(appearances(drain(&owner, &mut cx)), vec![expected]);
    }
    for native in [Native::Light, Native::Dark, Native::VibrantLight] {
        cx.simulate_appearance_change(native);
        cx.run_until_parked();
    }
    assert_eq!(
        appearances(drain(&owner, &mut cx)),
        vec![wire::Appearance::VibrantLight]
    );
    draw(&mut cx);
    draw(&mut cx);
    assert!(
        drain(&owner, &mut cx).is_empty(),
        "unchanged paints add no observations"
    );
}

#[test]
fn appearance_notifications_preserve_native_editor_focus_draft_and_selection() {
    let mut app = TestAppContext::single();
    let (owner, mut cx, _reader) = mount(&mut app);
    let node = NodeId::from_parts(0, 1).unwrap();
    apply(
        &owner,
        &mut cx,
        vec![
            Op::Create(
                node,
                Kind::Input,
                "draft λ".into(),
                Some(gpuio_protocol::HandlerId::from_parts(0, 1).unwrap()),
            ),
            Op::SetEditor(
                node,
                EditorConfig {
                    label: "Draft".into(),
                    placeholder: "".into(),
                    read_only: false,
                    disabled: false,
                    submit_on_enter: false,
                    auto_focus: false,
                    min_rows: 1,
                    max_rows: 1,
                },
            ),
            Op::SetRoot(Some(node)),
        ],
    );
    watch_owner(&owner, &mut cx);
    let focus = cx.update(|_, cx| owner.read(cx).editors[&node].focus_handle(cx));
    cx.update(|w, cx| w.focus(&focus, cx));
    cx.simulate_keystrokes("secondary-a");
    let before = cx.update(|w, cx| owner.read(cx).editors[&node].snapshot(w, cx));
    drain(&owner, &mut cx);
    for native in [Native::Dark, Native::Light, Native::VibrantDark] {
        cx.simulate_appearance_change(native);
        draw(&mut cx);
        cx.update(|w, cx| {
            assert!(focus.is_focused(w));
            assert_eq!(owner.read(cx).editors[&node].snapshot(w, cx), before);
        });
    }
    cx.simulate_input("retained");
    assert_eq!(
        cx.update(|w, cx| owner.read(cx).editors[&node].snapshot(w, cx).text),
        "retained"
    );
}

#[test]
fn appearance_subscriptions_are_window_owned_replaceable_and_releasable() {
    let mut app = TestAppContext::single();
    let (first, mut a, _reader_a) = mount(&mut app);
    let (second, mut b, _reader_b) = mount(&mut app);
    watch_owner(&first, &mut a);
    watch_owner(&second, &mut b);
    watch_owner(&first, &mut a); // replaces, rather than accumulates, subscription
    drain(&first, &mut a);
    drain(&second, &mut b);
    a.simulate_appearance_change(Native::Dark);
    a.run_until_parked();
    assert_eq!(
        appearances(drain(&first, &mut a)),
        vec![wire::Appearance::Dark]
    );
    assert!(drain(&second, &mut b).is_empty());
    b.simulate_appearance_change(Native::VibrantLight);
    b.run_until_parked();
    assert_eq!(
        appearances(drain(&second, &mut b)),
        vec![wire::Appearance::VibrantLight]
    );
    assert!(drain(&first, &mut a).is_empty());
    a.update(|_, cx| first.update(cx, |view, _| view.appearance_subscription = None));
    a.simulate_appearance_change(Native::Light);
    a.run_until_parked();
    assert!(
        drain(&first, &mut a).is_empty(),
        "dropped subscription cannot publish"
    );
    watch_owner(&first, &mut a);
    let weak = first.downgrade();
    // Queue the ordinary deferred callback, then close before it can run.
    a.simulate_appearance_change(Native::VibrantDark);
    a.update(|w, _| w.remove_window());
    drop(first);
    a.run_until_parked();
    app.update(|_| {});
    assert!(
        weak.upgrade().is_none(),
        "appearance callback must not retain its owner"
    );
    b.simulate_appearance_change(Native::Dark);
    b.run_until_parked();
    assert_eq!(
        appearances(drain(&second, &mut b)),
        vec![wire::Appearance::Dark]
    );
}
