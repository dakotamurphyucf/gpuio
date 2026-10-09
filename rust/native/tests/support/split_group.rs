use gpuio_protocol::{
    HandlerId, NodeId, WindowId,
    split::Axis,
    split_group::{Config, Panel},
    v1::*,
};
pub fn n(i: i64) -> NodeId {
    NodeId::from_parts(i, 1).unwrap()
}
pub fn h(i: i64) -> HandlerId {
    HandlerId::from_parts(i, 1).unwrap()
}
pub fn w() -> WindowId {
    WindowId::from_parts(0, 1).unwrap()
}
pub fn config() -> Config {
    Config {
        label: "Workspace".into(),
        axis: Axis::Horizontal,
        keyboard_step: 10.,
        reset_generation: 0,
        resize: None,
        panels: (0..3)
            .map(|i| Panel {
                id: format!("p{i}"),
                label: format!("Panel {i}"),
                initial_size: Some(100.),
                minimum_size: 40.,
                maximum_size: 500.,
                visible: true,
            })
            .collect(),
    }
}
pub fn initial() -> Vec<Op> {
    let mut ops = vec![
        Op::Create(n(0), Kind::SplitGroup, "".into(), Some(h(0))),
        Op::SetSplitGroup(n(0), config(), Default::default()),
        Op::SetStyle(
            n(0),
            vec![Style::Fields(vec![
                Field::Width(Length::Px(300.)),
                Field::Height(Length::Px(120.)),
            ])],
        ),
    ];
    for i in 0..3 {
        let base = 1 + i * 5;
        for id in base..base + 3 {
            ops.push(Op::Create(n(id), Kind::Container, "".into(), None));
        }
        ops.extend([
            Op::Create(
                n(base + 3),
                Kind::Button,
                format!("Button {i}"),
                Some(h(i + 1)),
            ),
            Op::SetControl(n(base + 3), Control::Button(false)),
            Op::SetStyle(
                n(base + 3),
                vec![Style::Fields(vec![
                    Field::Width(Length::Px(30.)),
                    Field::Height(Length::Px(24.)),
                ])],
            ),
            Op::Create(n(base + 4), Kind::Text, "Grip".into(), None),
            Op::Splice(n(base + 1), 0, 0, vec![n(base + 3)]),
            Op::Splice(n(base + 2), 0, 0, vec![n(base + 4)]),
            Op::Splice(n(base), 0, 0, vec![n(base + 1), n(base + 2)]),
        ]);
    }
    ops.extend([
        Op::Splice(n(0), 0, 0, vec![n(1), n(6), n(11)]),
        Op::SetRoot(Some(n(0))),
    ]);
    ops
}
