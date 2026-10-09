use gpuio_native::toast_geometry::*;
use gpuio_protocol::NodeId;
fn card(i: i64, height: f64) -> Card {
    Card {
        id: NodeId::from_parts(i, 1).unwrap(),
        height,
        ending: false,
    }
}
#[test]
fn variable_heights_mirror_and_hide_deep_layers_without_keeping_their_footprint() {
    let cards = [card(0, 10_000.), card(1, 120.), card(2, 60.)];
    let config = Layering {
        visible: 2,
        ..Default::default()
    };
    let top = layout(&cards, 200., config, false, false).unwrap();
    assert_eq!(top.height, 134.);
    assert_eq!(
        top.items
            .iter()
            .map(|i| (i.x, i.y, i.width, i.visible, i.interactive))
            .collect::<Vec<_>>(),
        vec![
            (5., 28., 190., false, false),
            (5., 14., 190., true, false),
            (0., 0., 200., true, true)
        ]
    );
    let bottom = layout(&cards, 200., config, false, true).unwrap();
    for (a, b) in top.items.iter().zip(&bottom.items) {
        assert_eq!(a.y + b.y + a.height, top.height);
        assert_eq!(a.id, b.id);
    }
    let expanded = layout(&cards, 200., config, true, false).unwrap();
    assert_eq!(expanded.height, 10_208.);
    assert_eq!(
        expanded.items.iter().map(|i| i.y).collect::<Vec<_>>(),
        vec![208., 74., 0.]
    );
    assert!(
        expanded
            .items
            .iter()
            .all(|i| i.interactive && i.width == 200.)
    );
}
#[test]
fn zero_dimensions_ending_ids_and_invalid_inputs_are_explicit() {
    let config = Layering::default();
    assert!(
        layout(&[], 0., config, false, false)
            .unwrap()
            .items
            .is_empty()
    );
    let mut c = card(0, 80.);
    c.ending = true;
    assert!(!layout(&[c], 200., config, true, false).unwrap().items[0].interactive);
    c.ending = false;
    assert!(!layout(&[c], 0., config, true, false).unwrap().items[0].interactive);
    assert!(layout(&[c, c], 200., config, true, false).is_err());
    for bad in [-1., 1_000_001., f64::NAN, f64::INFINITY] {
        assert!(layout(&[card(0, bad)], 200., config, true, false).is_err());
        assert!(layout(&[c], bad, config, true, false).is_err());
    }
    for config in [
        Layering {
            visible: 0,
            ..config
        },
        Layering {
            visible: 9,
            ..config
        },
        Layering {
            width_step: 0.11,
            ..config
        },
        Layering {
            peek: f64::NAN,
            ..config
        },
        Layering { gap: -1., ..config },
    ] {
        assert!(layout(&[c], 200., config, true, false).is_err());
    }
    assert!(
        layout(
            &(0..33).map(|i| card(i, 1.)).collect::<Vec<_>>(),
            200.,
            config,
            true,
            false
        )
        .is_err()
    );
}
#[test]
fn generated_measurements_remain_finite_ordered_nonoverlapping_and_bounded() {
    let mut seed = 19u64;
    for n in 0..1000 {
        let count = n % 33;
        let cards = (0..count)
            .map(|i| {
                seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1);
                card(i as i64, ((seed >> 32) % 10000) as f64 / 10.)
            })
            .collect::<Vec<_>>();
        let config = Layering {
            visible: 1 + n % 8,
            width_step: 0.1,
            ..Default::default()
        };
        for expanded in [true, false] {
            for bottom in [true, false] {
                let result = layout(&cards, 320., config, expanded, bottom).unwrap();
                assert_eq!(
                    result.items.iter().map(|i| i.id).collect::<Vec<_>>(),
                    cards.iter().map(|c| c.id).collect::<Vec<_>>()
                );
                let mut shown = result
                    .items
                    .iter()
                    .filter(|i| i.visible)
                    .collect::<Vec<_>>();
                assert_eq!(
                    shown.len(),
                    if expanded {
                        count
                    } else {
                        count.min(config.visible)
                    }
                );
                for i in &shown {
                    assert!(i.y >= -1e-8 && i.y + i.height <= result.height + 1e-8);
                    assert!(i.width > 0. && i.x >= 0. && i.x + i.width <= 320.);
                }
                if expanded {
                    shown.sort_by(|a, b| a.y.total_cmp(&b.y));
                    for pair in shown.windows(2) {
                        assert!(pair[0].y + pair[0].height <= pair[1].y + 1e-8);
                    }
                }
            }
        }
    }
}
