#[path = "common/canvas_fixture.rs"]
mod fixture;
use binprot::BinProtWrite;
use gpuio_protocol::{DecodeError, canvas::*, canvas_scene::*, decode_canvas_scene};

fn encode(scene: &Scene) -> Vec<u8> {
    let mut bytes = Vec::new();
    scene.binprot_write(&mut bytes).unwrap();
    bytes
}

#[test]
fn fixture_covers_all_drawing_resource_and_hit_region_kinds() {
    let scene = fixture::scene();
    assert_eq!(scene.validate(), Ok(()));
    let bytes = encode(&scene);
    let actual = bytes.iter().map(|b| format!("{b:02x}")).collect::<String>();
    assert_eq!(
        actual,
        include_str!("../../../test/fixtures/canvas-v1-scene.hex").trim()
    );
    assert_eq!(decode_canvas_scene(&bytes), Ok(scene));
    for end in 0..bytes.len() {
        assert!(
            decode_canvas_scene(&bytes[..end]).is_err(),
            "accepted truncation {end}"
        );
    }
    let mut trailing = bytes;
    trailing.push(0);
    assert_eq!(decode_canvas_scene(&trailing), Err(DecodeError::Malformed));
}

#[test]
fn domain_rejects_duplicates_stale_references_wrong_kinds_and_invalid_geometry() {
    use ValidationError::*;
    type InvalidCase = (ValidationError, fn(&mut Scene));
    let cases: Vec<InvalidCase> = vec![
        (UnsupportedVersion, |s| s.version = 2),
        (InvalidText, |s| s.description = " ".into()),
        (InvalidText, |s| s.description = "\u{b}".into()),
        (InvalidIdentity, |s| s.items[0].id = 0),
        (DuplicateIdentity, |s| s.items[1].id = 1),
        (DuplicateIdentity, |s| s.resources[1].key.id = 1),
        (InvalidGeometry, |s| s.items[0].transform.a = f64::NAN),
        (InvalidGeometry, |s| s.items[0].transform.d = 0.),
        (InvalidGeometry, |s| s.items[0].clips[0].width = 0.),
        (MissingResource, |s| {
            s.resources.remove(0);
        }),
        (StaleResource, |s| s.resources[0].key.generation += 1),
        (WrongResourceKind, |s| {
            s.resources[0].data = s.resources[1].data.clone()
        }),
        (UnsupportedTransform, |s| s.items[3].transform.b = 0.1),
        (UnsupportedTransform, |s| s.items[4].transform.a = -1.),
        (InvalidGeometry, |s| s.items[0].transform.tx = 1_000_000.),
        (InvalidText, |s| {
            if let ResourceData::Text(text) = &mut s.resources[1].data {
                text.value = "a\nb".into();
            }
        }),
        (InvalidGeometry, |s| {
            s.items[0].drawing = Drawing::Shape(
                Shape::Rectangle(Rect {
                    x: 0.,
                    y: 0.,
                    width: 1.,
                    height: 1.,
                }),
                Paint {
                    fill: None,
                    stroke: None,
                },
            )
        }),
        (InvalidGeometry, |s| {
            if let ResourceData::Path(path) = &mut s.resources[0].data {
                path.0.pop();
            }
            if let Drawing::Shape(_, paint) = &mut s.items[2].drawing {
                paint.fill = Some(0);
            }
        }),
    ];
    for (expected, change) in cases {
        let mut scene = fixture::scene();
        change(&mut scene);
        assert_eq!(scene.validate(), Err(expected));
        assert!(decode_canvas_scene(&encode(&scene)).is_err());
    }
}

#[test]
fn bounded_decoder_rejects_huge_counts_nonfinite_values_and_aggregate_budgets() {
    // Version 1, one-byte description, then resource count greater than 4096.
    let mut hostile = vec![1, 1, b'x'];
    binprot::Nat0(1_000_000)
        .binprot_write(&mut hostile)
        .unwrap();
    assert_eq!(
        decode_canvas_scene(&hostile),
        Err(DecodeError::LimitExceeded)
    );
    assert_eq!(
        decode_canvas_scene(&[1, 1, 0xff, 0, 0]),
        Err(DecodeError::Malformed)
    );
    // Valid prefix through one resource key/path tag, hostile nested command count.
    let mut nested = vec![1, 1, b'x', 1, 1, 1, 0];
    binprot::Nat0(4097).binprot_write(&mut nested).unwrap();
    assert_eq!(
        decode_canvas_scene(&nested),
        Err(DecodeError::LimitExceeded)
    );
    assert_eq!(
        decode_canvas_scene(&vec![0; MAX_BYTES + 1]),
        Err(DecodeError::LimitExceeded)
    );
    let mut scene = fixture::scene();
    scene.items[0].clips = vec![scene.items[0].clips[0]; MAX_CLIPS + 1];
    assert_eq!(
        decode_canvas_scene(&encode(&scene)),
        Err(DecodeError::LimitExceeded)
    );
    let item = fixture::scene().items.remove(0);
    scene.items = (1..=MAX_INTERACTIVE_ITEMS + 1)
        .map(|id| Item {
            id: id as i64,
            ..item.clone()
        })
        .collect();
    assert_eq!(
        decode_canvas_scene(&encode(&scene)),
        Err(DecodeError::LimitExceeded)
    );
    let mut scene = fixture::scene();
    scene.resources = (1..=65)
        .map(|id| Resource {
            key: ResourceKey { id, generation: 1 },
            data: ResourceData::Text(Text {
                value: "a".repeat(16_384),
                font_family: "system".into(),
                font_size: 14.,
                font_weight: 400,
            }),
        })
        .collect();
    scene.items.clear();
    assert_eq!(
        decode_canvas_scene(&encode(&scene)),
        Err(DecodeError::LimitExceeded)
    );
    let commands = std::iter::once(PathCommand::Move(Point { x: 0., y: 0. }))
        .chain(std::iter::repeat_n(
            PathCommand::Line(Point { x: 1., y: 1. }),
            4095,
        ))
        .collect::<Vec<_>>();
    scene.resources = (1..=17)
        .map(|id| Resource {
            key: ResourceKey { id, generation: 1 },
            data: ResourceData::Path(Path(commands.clone())),
        })
        .collect();
    assert_eq!(
        decode_canvas_scene(&encode(&scene)),
        Err(DecodeError::LimitExceeded)
    );
}

#[test]
fn large_scene_preserves_draw_order_and_shared_resource_validation_is_bounded() {
    let mut scene = fixture::scene();
    let mut item = scene.items[2].clone();
    item.interaction = None;
    scene.resources[0].data = ResourceData::Path(Path(
        std::iter::once(PathCommand::Move(Point { x: 0., y: 0. }))
            .chain(std::iter::repeat_n(
                PathCommand::Line(Point { x: 10., y: 10. }),
                4095,
            ))
            .collect(),
    ));
    scene.items = (1..=MAX_ITEMS)
        .map(|id| Item {
            id: id as i64,
            ..item.clone()
        })
        .collect();
    let bytes = encode(&scene);
    assert!(bytes.len() < MAX_BYTES);
    let start = std::time::Instant::now();
    let decoded = decode_canvas_scene(&bytes).unwrap();
    println!(
        "canvas decode+validate: {} items, 4096 shared path commands, {} bytes, {:?}",
        decoded.items.len(),
        bytes.len(),
        start.elapsed()
    );
    assert_eq!(decoded.items.first().unwrap().id, 1);
    assert_eq!(decoded.items.last().unwrap().id, MAX_ITEMS as i64);
    assert_eq!(decoded.resources.len(), 3);
    scene.items.push(Item {
        id: MAX_ITEMS as i64 + 1,
        ..item
    });
    assert_eq!(scene.validate(), Err(ValidationError::LimitExceeded));
}

#[test]
fn topmost_hits_follow_reorder_transform_and_clipping_without_decorative_interception() {
    let mut scene = fixture::scene();
    assert_eq!(scene.hit_test(Point { x: 10., y: 5. }), Some(1));
    assert_eq!(scene.hit_test(Point { x: 40., y: 5. }), Some(2));
    assert_eq!(scene.hit_test(Point { x: 5., y: 35. }), Some(3));
    scene.items[1].transform = Transform::IDENTITY;
    assert_eq!(scene.hit_test(Point { x: 10., y: 5. }), Some(2));
    scene.items.swap(0, 1);
    assert_eq!(scene.hit_test(Point { x: 10., y: 5. }), Some(1));
    scene.items[1].clips = vec![
        Rect {
            x: 0.,
            y: 0.,
            width: 5.,
            height: 5.,
        },
        Rect {
            x: 5.,
            y: 0.,
            width: 5.,
            height: 5.,
        },
    ];
    assert_eq!(scene.hit_test(Point { x: 10., y: 5. }), Some(2));
    assert_eq!(scene.hit_test(Point { x: f64::NAN, y: 0. }), None);
    assert_eq!(scene.validate(), Ok(()));
}
