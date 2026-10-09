use gpuio_native::split_group_state::{Error, State};
use gpuio_protocol::{split::Axis, split_group::*};
use std::sync::Arc;
fn config() -> Config {
    Config {
        label: "Group".into(),
        axis: Axis::Horizontal,
        keyboard_step: 10.,
        reset_generation: 0,
        resize: None,
        panels: (0..3)
            .map(|i| Panel {
                id: format!("p{i}"),
                label: format!("Panel {i}"),
                initial_size: Some((i + 1) as f64 * 100.),
                minimum_size: 20.,
                maximum_size: 1000.,
                visible: true,
            })
            .collect(),
    }
}
fn sizes(state: &mut State) -> Vec<f64> {
    state.measure(600.).unwrap().unwrap().layout.sizes
}
#[test]
fn stable_ids_keep_geometry_through_labels_reorder_hide_insert_and_reset() {
    let mut config = config();
    let mut state = State::new(Arc::new(config.clone())).unwrap();
    assert_eq!(sizes(&mut state), vec![100., 200., 300.]);
    config.panels[0].label = "Files".into();
    config.panels[0].initial_size = Some(500.);
    state.reconcile(Arc::new(config.clone())).unwrap();
    assert_eq!(sizes(&mut state), vec![100., 200., 300.]);
    config.panels.reverse();
    state.reconcile(Arc::new(config.clone())).unwrap();
    assert_eq!(sizes(&mut state), vec![300., 200., 100.]);
    config.panels[1].visible = false;
    state.reconcile(Arc::new(config.clone())).unwrap();
    assert_eq!(sizes(&mut state), vec![450., 200., 150.]);
    config.panels[1].visible = true;
    state.reconcile(Arc::new(config.clone())).unwrap();
    assert_eq!(sizes(&mut state), vec![337.5, 150., 112.5]);
    config.reset_generation = 1;
    state.reconcile(Arc::new(config.clone())).unwrap();
    assert_eq!(sizes(&mut state), vec![180., 120., 300.]);
    let before = sizes(&mut state);
    let mut invalid = config.clone();
    invalid.reset_generation = 0;
    assert_eq!(
        state.reconcile(Arc::new(invalid)),
        Err(Error::DecreasingGeneration)
    );
    assert_eq!(sizes(&mut state), before);
    config.panels.insert(
        1,
        Panel {
            id: "new".into(),
            initial_size: Some(100.),
            ..config.panels[0].clone()
        },
    );
    state.reconcile(Arc::new(config.clone())).unwrap();
    let inserted = sizes(&mut state);
    assert_eq!(inserted.len(), 4);
    assert!((inserted.iter().sum::<f64>() - 600.).abs() < 1e-8);
    config.panels.remove(1);
    state.reconcile(Arc::new(config)).unwrap();
    let removed = sizes(&mut state);
    for (a, b) in removed.iter().zip(before) {
        assert!((a - b).abs() < 1e-8);
    }
}
#[test]
fn previews_are_noncommitting_cancel_restores_and_completed_gestures_observe_once() {
    let config = Arc::new(config());
    let mut state = State::new(config.clone()).unwrap();
    let initial = sizes(&mut state);
    state.begin_drag("p0").unwrap();
    state.drag_to(60.).unwrap();
    assert_eq!(sizes(&mut state), vec![160., 140., 300.]);
    assert!(state.cancel_drag());
    assert_eq!(sizes(&mut state), initial);
    assert!(state.finish_drag().is_none());
    state.begin_drag("p0").unwrap();
    state.drag_to(60.).unwrap();
    let observed = state.finish_drag().unwrap();
    assert_eq!(observed.source, Source::Pointer);
    assert_eq!(observed.sizes[0].1, 160.);
    assert!(state.finish_drag().is_none());
    state.begin_drag("p0").unwrap();
    state.drag_to(20.).unwrap();
    assert!(state.measure(800.).unwrap().unwrap().cancelled_drag);
    assert!(!state.is_dragging());
    let mut policy = (*config).clone();
    policy.panels[0].maximum_size = 150.;
    state.begin_drag("p0").unwrap();
    assert!(state.reconcile(Arc::new(policy)).unwrap());
    assert!(!state.is_dragging());
    assert!(sizes(&mut state)[0] <= 150.);
}
#[test]
fn requests_wait_for_usable_visible_target_and_never_replay_after_reset_or_clear() {
    let mut config = config();
    config.panels[1].visible = false;
    config.resize = Some(ResizeRequest {
        id: "p1".into(),
        size: 250.,
        serial: 1,
    });
    let mut state = State::new(Arc::new(config.clone())).unwrap();
    assert!(state.measure(0.).unwrap().is_none());
    assert_eq!(state.pending_serial(), Some(1));
    assert!(state.measure(600.).unwrap().unwrap().observation.is_none());
    config.panels[1].visible = true;
    state.reconcile(Arc::new(config.clone())).unwrap();
    let measured = state.measure(600.).unwrap().unwrap();
    let observed = measured.observation.unwrap();
    assert_eq!(observed.source, Source::Request(1));
    assert_eq!(observed.sizes[1].1, 250.);
    assert!(state.measure(600.).unwrap().unwrap().observation.is_none());
    config.reset_generation = 1;
    state.reconcile(Arc::new(config.clone())).unwrap();
    assert!(state.measure(600.).unwrap().unwrap().observation.is_none());
    config.resize = Some(ResizeRequest {
        id: "missing".into(),
        size: 200.,
        serial: 2,
    });
    state.reconcile(Arc::new(config.clone())).unwrap();
    assert_eq!(state.pending_serial(), None);
    config.resize = Some(ResizeRequest {
        id: "p1".into(),
        size: 200.,
        serial: 3,
    });
    state.reconcile(Arc::new(config.clone())).unwrap();
    assert_eq!(state.pending_serial(), Some(3));
    config.resize = None;
    state.reconcile(Arc::new(config.clone())).unwrap();
    assert_eq!(state.pending_serial(), None);
    config.resize = Some(ResizeRequest {
        id: "p1".into(),
        size: 300.,
        serial: 3,
    });
    state.reconcile(Arc::new(config)).unwrap();
    assert!(state.measure(600.).unwrap().unwrap().observation.is_none());
}
#[test]
fn zero_layout_preserves_preferences_and_invalid_inputs_leave_geometry_unchanged() {
    let mut state = State::new(Arc::new(config())).unwrap();
    let before = sizes(&mut state);
    state.begin_drag("p0").unwrap();
    state.drag_to(20.).unwrap();
    assert!(state.measure(0.).unwrap().is_none());
    assert!(!state.is_dragging());
    assert_eq!(sizes(&mut state), before);
    assert!(state.measure(f64::NAN).is_err());
    assert_eq!(sizes(&mut state), before);
    assert!(
        state
            .adjust_boundary("missing", 20., Source::Keyboard)
            .is_err()
    );
    assert_eq!(sizes(&mut state), before);
    let snapshot = state
        .adjust_boundary("p0", 10., Source::Accessibility)
        .unwrap()
        .unwrap();
    assert_eq!(snapshot.sizes[0].1, 110.);
    assert!(snapshot.valid_for(state.config()));
    assert!(
        state
            .adjust_boundary("p0", 0., Source::Keyboard)
            .unwrap()
            .is_none()
    );
}
