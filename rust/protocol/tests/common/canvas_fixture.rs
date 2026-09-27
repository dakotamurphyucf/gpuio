use gpuio_protocol::{ResourceId, canvas::*, canvas_scene::*};

pub fn scene() -> Scene {
    let point = |x, y| Point { x, y };
    let rect = Rect {
        x: 0.,
        y: 0.,
        width: 20.,
        height: 10.,
    };
    let key = |id, generation| ResourceKey { id, generation };
    let paint = Paint {
        fill: Some(0x102030ff),
        stroke: Some(Stroke {
            color: 0xff0000ff,
            width: 2.,
        }),
    };
    Scene {
        version: 1,
        description: "Diagram 🦀".into(),
        resources: vec![
            Resource {
                key: key(1, 2),
                data: ResourceData::Path(Path(vec![
                    PathCommand::Move(point(0., 0.)),
                    PathCommand::Line(point(10., 10.)),
                    PathCommand::Line(point(0., 10.)),
                    PathCommand::Close,
                ])),
            },
            Resource {
                key: key(2, 1),
                data: ResourceData::Text(Text {
                    value: "héllo 🦀".into(),
                    font_family: "system".into(),
                    font_size: 14.,
                    font_weight: 500,
                }),
            },
            Resource {
                key: key(3, 3),
                data: ResourceData::Image(ResourceId::from_parts(7, 2).unwrap()),
            },
        ],
        items: vec![
            Item {
                id: 1,
                transform: Transform::IDENTITY,
                clips: vec![rect],
                drawing: Drawing::Shape(Shape::Rectangle(rect), paint),
                interaction: Some(Interaction {
                    label: "Rectangle".into(),
                    hit_region: HitRegion::Rectangle(rect),
                    draggable: true,
                    activatable: true,
                }),
            },
            Item {
                id: 2,
                transform: Transform {
                    tx: 30.,
                    ..Transform::IDENTITY
                },
                clips: vec![],
                drawing: Drawing::Shape(
                    Shape::Ellipse(rect),
                    Paint {
                        fill: Some(0xff00ffff),
                        stroke: None,
                    },
                ),
                interaction: Some(Interaction {
                    label: "Ellipse".into(),
                    hit_region: HitRegion::Ellipse(rect),
                    draggable: false,
                    activatable: true,
                }),
            },
            Item {
                id: 3,
                transform: Transform {
                    ty: 30.,
                    ..Transform::IDENTITY
                },
                clips: vec![],
                drawing: Drawing::Shape(
                    Shape::Path(key(1, 2)),
                    Paint {
                        fill: None,
                        stroke: paint.stroke,
                    },
                ),
                interaction: Some(Interaction {
                    label: "Path".into(),
                    hit_region: HitRegion::Polygon(vec![
                        point(0., 0.),
                        point(10., 10.),
                        point(0., 10.),
                    ]),
                    draggable: true,
                    activatable: false,
                }),
            },
            Item {
                id: 4,
                transform: Transform::IDENTITY,
                clips: vec![],
                drawing: Drawing::Text(key(2, 1), point(10., 50.), 0xffffffff),
                interaction: None,
            },
            Item {
                id: 5,
                transform: Transform::IDENTITY,
                clips: vec![],
                drawing: Drawing::Image(key(3, 3), rect),
                interaction: None,
            },
        ],
    }
}
