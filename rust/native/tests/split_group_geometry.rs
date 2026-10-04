use gpuio_native::split_group_geometry::*;
use gpuio_protocol::split_group::Panel;
fn panel(id: &str, min: f64, max: f64) -> Panel {
    Panel {
        id: id.into(),
        label: id.into(),
        initial_size: None,
        minimum_size: min,
        maximum_size: max,
        visible: true,
    }
}
fn near(a: f64, b: f64) {
    assert!((a - b).abs() < 1e-7, "{a} != {b}");
}
#[test]
fn fit_honors_all_bounds_and_distinguishes_overflow_from_unused_space() {
    let panels = vec![
        panel("a", 40., 100.),
        panel("b", 10., 90.),
        panel("c", 0., 200.),
    ];
    let layout = fit(&panels, &[Some(1.), Some(100.), Some(0.)], 100.).unwrap();
    near(layout.used, 100.);
    assert!(layout.sizes[0] >= 40. && layout.sizes[1] <= 90.);
    let small = fit(&panels, &[None; 3], 20.).unwrap();
    assert_eq!(small.sizes, vec![40., 10., 0.]);
    near(small.overflow, 30.);
    let large = fit(&panels, &[None; 3], 500.).unwrap();
    assert_eq!(large.sizes, vec![100., 90., 200.]);
    near(large.unused, 110.);
    let mixed = fit(
        &[panel("a", 0., 1000.), panel("b", 0., 1000.)],
        &[Some(240.), None],
        800.,
    )
    .unwrap();
    assert_eq!(mixed.sizes, vec![240., 560.]);
    let proportional = fit(
        &[panel("a", 0., 1000.), panel("b", 0., 1000.)],
        &[Some(200.), Some(300.)],
        1000.,
    )
    .unwrap();
    assert_eq!(proportional.sizes, vec![400., 600.]);
}
#[test]
fn boundaries_transfer_across_a_flat_partition_after_adjacent_limits() {
    let p = vec![
        panel("a", 50., 180.),
        panel("b", 80., 120.),
        panel("c", 90., 110.),
        panel("d", 20., 200.),
    ];
    let sizes = vec![100.; 4];
    let grown = move_boundary(&p, &sizes, "b", 80.).unwrap();
    assert_eq!(grown, vec![160., 120., 90., 30.]);
    let maximum = move_boundary(&p, &sizes, "b", 1000.).unwrap();
    assert_eq!(maximum, vec![170., 120., 90., 20.]);
    let (lo, value, hi) = boundary_range(&p, &sizes, "b").unwrap();
    assert_eq!((lo, value, hi), (130., 200., 290.));
    let minimum = move_boundary(&p, &sizes, "b", -1000.).unwrap();
    assert_eq!(minimum, vec![50., 80., 110., 160.]);
    assert_eq!(move_boundary(&p, &sizes, "d", 1.), Err(Error::NoBoundary));
    let requested = resize_panel(&p, &sizes, "a", 175.).unwrap();
    assert_eq!(requested, vec![175., 80., 90., 55.]);
    let last = resize_panel(&p, &sizes, "d", 160.).unwrap();
    assert_eq!(last, vec![70., 80., 90., 160.]);
}
#[test]
fn hidden_sizes_are_retained_and_do_not_enter_boundary_or_programmatic_transfers() {
    let mut p = vec![
        panel("a", 20., 300.),
        panel("hidden", 20., 300.),
        panel("b", 20., 300.),
    ];
    p[1].visible = false;
    let fitted = fit(&p, &[Some(100.), Some(155.), Some(100.)], 300.).unwrap();
    assert_eq!(fitted.sizes, vec![150., 155., 150.]);
    assert_eq!(
        move_boundary(&p, &fitted.sizes, "a", 50.).unwrap(),
        vec![200., 155., 100.]
    );
    assert_eq!(
        resize_panel(&p, &fitted.sizes, "hidden", 250.).unwrap(),
        fitted.sizes
    );
    assert_eq!(
        boundary_range(&p, &fitted.sizes, "hidden"),
        Err(Error::NoBoundary)
    );
    assert_eq!(
        fit(&[], &[], 200.).unwrap(),
        Layout {
            sizes: vec![],
            used: 0.,
            overflow: 0.,
            unused: 200.
        }
    );
}
#[test]
fn generated_groups_preserve_ranges_fit_feasible_extents_and_conserve_transfers() {
    let mut seed = 17u64;
    let mut random = || {
        seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1);
        (seed >> 32) as u32
    };
    for _ in 0..2000 {
        let count = (random() % 64 + 1) as usize;
        let p: Vec<_> = (0..count)
            .map(|i| {
                let min = (random() % 100) as f64;
                let mut p = panel(&i.to_string(), min, min + (random() % 400) as f64);
                p.visible = random() % 5 != 0;
                p
            })
            .collect();
        let preferences: Vec<_> = (0..count).map(|_| Some((random() % 500) as f64)).collect();
        let extent = (random() % 30000) as f64;
        let layout = fit(&p, &preferences, extent).unwrap();
        let min: f64 = p.iter().filter(|p| p.visible).map(|p| p.minimum_size).sum();
        let max: f64 = p.iter().filter(|p| p.visible).map(|p| p.maximum_size).sum();
        near(layout.used, extent.clamp(min, max));
        for (panel, size) in p.iter().zip(&layout.sizes) {
            assert!(*size >= panel.minimum_size && *size <= panel.maximum_size);
        }
        for after in p.iter().filter(|p| p.visible).take(3) {
            let Ok((lo, at, hi)) = boundary_range(&p, &layout.sizes, &after.id) else {
                continue;
            };
            for delta in [-100000., -1., 0., 1., 100000.] {
                let next = move_boundary(&p, &layout.sizes, &after.id, delta).unwrap();
                near(next.iter().sum(), layout.sizes.iter().sum());
                let (_, value, _) = boundary_range(&p, &next, &after.id).unwrap();
                near(value, (at + delta).clamp(lo, hi));
                for ((panel, old), size) in p.iter().zip(&layout.sizes).zip(&next) {
                    assert!(*size >= panel.minimum_size && *size <= panel.maximum_size);
                    if !panel.visible {
                        assert_eq!(old, size);
                    }
                }
            }
        }
    }
}
#[test]
fn malformed_or_unbounded_geometry_is_rejected() {
    let p = vec![panel("a", 10., 100.), panel("b", 10., 100.)];
    for bad in [f64::NAN, f64::INFINITY, -1.] {
        assert_eq!(fit(&p, &[None; 2], bad), Err(Error::InvalidGeometry));
    }
    assert_eq!(
        fit(&p, &[Some(f64::NAN), None], 100.),
        Err(Error::InvalidGeometry)
    );
    assert_eq!(
        move_boundary(&p, &[5., 95.], "a", 1.),
        Err(Error::InvalidGeometry)
    );
    assert_eq!(
        resize_panel(&p, &[50., 50.], "missing", 60.),
        Err(Error::UnknownPanel)
    );
    assert_eq!(
        fit(&vec![p[0].clone(); 65], &[None; 65], 100.),
        Err(Error::InvalidPanels)
    );
}
