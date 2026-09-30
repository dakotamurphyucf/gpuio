//! Retained row visibility and bounded sampling through the production host.
use super::*;
use gpuio_protocol::list::{Config, IdRun, Order, Row, ScrollPolicy, ScrollRequest, ScrollTarget};

pub(super) async fn exercise(
    cx: &mut gpui::AsyncApp,
    window: WindowHandle<View>,
    transport: &Transport,
) {
    let mut ops = vec![
        Op::Create(node(10), Kind::VirtualList, "".into(), Some(handler(10, 1))),
        Op::SetStyle(
            node(10),
            vec![
                Style::Width(Length::Px(300.)),
                Style::Height(Length::Px(100.)),
                Style::Fields(vec![Field::Shrink(0.)]),
            ],
        ),
        Op::SetListConfig(
            node(10),
            Config {
                estimated_height: 40.,
                overscan: 0.,
                max_active: 8,
                scroll_policy: ScrollPolicy::KeepPosition,
                scrollbar: true,
                managed: true,
            },
        ),
        Op::SetListOrder(
            node(10),
            Order {
                revision: 1,
                runs: vec![IdRun {
                    first: 1,
                    count: 1000,
                }],
            },
        ),
        Op::ScrollList(
            node(10),
            ScrollRequest {
                serial: 1,
                target: ScrollTarget::Offset(1, 0.),
            },
        ),
    ];
    for slot in [11, 13] {
        ops.extend([
            Op::Create(
                node(slot),
                Kind::Container,
                "".into(),
                Some(handler(slot, 1)),
            ),
            Op::SetCommandBinding(node(slot), Some(query(binding::Context::Here))),
            Op::SetStyle(node(slot), vec![Style::Height(Length::Px(40.))]),
            Op::Create(
                node(slot + 1),
                Kind::Text,
                format!("Observed row {slot}"),
                None,
            ),
            Op::Splice(node(slot), 0, 0, vec![node(slot + 1)]),
        ]);
    }
    ops.extend([
        Op::SetListRows(
            node(10),
            vec![
                Row {
                    id: 1,
                    node: node(11),
                },
                Row {
                    id: 1000,
                    node: node(13),
                },
            ],
        ),
        Op::Splice(node(10), 0, 0, vec![node(11), node(13)]),
        Op::Splice(node(0), 3, 0, vec![node(10)]),
    ]);
    apply(cx, window, ops);
    frame(cx, window).await;
    let first = observations(transport);
    assert_eq!(registry(&first[&node(11)]), binding::Disposition::Declared);
    assert_eq!(first[&node(13)].state, binding::State::Suspended);
    apply(
        cx,
        window,
        vec![Op::ScrollList(
            node(10),
            ScrollRequest {
                serial: 2,
                target: ScrollTarget::End,
            },
        )],
    );
    frame(cx, window).await;
    let last = observations(transport);
    assert_eq!(last[&node(11)].state, binding::State::Suspended);
    assert_eq!(registry(&last[&node(13)]), binding::Disposition::Declared);
    apply(
        cx,
        window,
        vec![Op::ScrollList(
            node(10),
            ScrollRequest {
                serial: 3,
                target: ScrollTarget::Offset(1, 0.),
            },
        )],
    );
    frame(cx, window).await;
    let returned = observations(transport);
    assert_eq!(
        registry(&returned[&node(11)]),
        binding::Disposition::Declared
    );
    assert!(
        returned[&node(11)].epoch > last[&node(11)].epoch,
        "retained row resumes the same subscription"
    );
    assert_eq!(returned[&node(13)].state, binding::State::Suspended);
    // Queue a changed result, then evict both native rows before draining.
    apply(
        cx,
        window,
        vec![Op::SetCommands(node(0), vec![command(false)])],
    );
    frame(cx, window).await;
    apply(
        cx,
        window,
        vec![
            Op::SetListRows(node(10), vec![]),
            Op::Splice(node(10), 0, 2, vec![]),
            Op::Remove(node(12)),
            Op::Remove(node(11)),
            Op::Remove(node(14)),
            Op::Remove(node(13)),
        ],
    );
    let retired = observations(transport);
    assert!(
        !retired.contains_key(&node(11)) && !retired.contains_key(&node(13)),
        "eviction prunes undrained row samples"
    );
    apply(
        cx,
        window,
        vec![
            Op::Splice(node(0), 3, 1, vec![]),
            Op::Remove(node(10)),
            Op::SetCommands(node(0), vec![command(true)]),
        ],
    );
    frame(cx, window).await;
    observations(transport);
    workload(cx, window, transport).await;
    eprintln!(
        "GPUIO_BINDING_RETENTION_OK: sparse 1000-row visibility, retained subscription recovery, eviction, bounded work and recovery"
    );
}

async fn workload(cx: &mut gpui::AsyncApp, window: WindowHandle<View>, transport: &Transport) {
    let commands: Vec<_> = (0..1024)
        .map(|index| {
            let mut command = command(true);
            command.id = format!("budget-{index}");
            command.shortcuts = (0..4)
                .map(|offset| {
                    let mut shortcut = command.shortcuts[0].clone();
                    shortcut.key = char::from_u32(0x4e00 + index * 4 + offset)
                        .unwrap()
                        .to_string();
                    shortcut
                })
                .collect();
            command
        })
        .collect();
    let query_config = binding::Config {
        context: binding::Context::Focused,
        targets: (960..1024)
            .map(|index| binding::Target::Command(format!("budget-{index}")))
            .collect(),
    };
    let last = commands.last().unwrap().clone();
    apply(
        cx,
        window,
        vec![
            Op::SetCommands(node(0), commands),
            Op::Bind(node(3), Some(handler(3, 2))),
            Op::SetCommandBinding(node(3), Some(query_config)),
        ],
    );
    frame(cx, window).await;
    let overloaded = observations(transport);
    assert_eq!(
        overloaded[&node(3)].state,
        binding::State::Capacity,
        "expensive valid query must not masquerade as missing commands"
    );
    frame(cx, window).await;
    assert!(
        observations(transport).is_empty(),
        "unchanged capacity status is silent"
    );
    apply(cx, window, vec![Op::SetCommands(node(0), vec![last])]);
    frame(cx, window).await;
    let recovered = observations(transport);
    let binding::State::Ready(entries) = &recovered[&node(3)].state else {
        panic!("smaller workload did not recover");
    };
    assert_eq!(entries.len(), 64);
    assert!(
        entries[..63]
            .iter()
            .all(|entry| *entry == binding::Entry::MissingCommand)
    );
    assert!(
        matches!(&entries[63], binding::Entry::Registry { candidates, .. } if candidates.len() == 4)
    );
    assert!(recovered[&node(3)].epoch > overloaded[&node(3)].epoch);
    apply(
        cx,
        window,
        vec![
            Op::SetCommands(node(0), vec![command(true)]),
            Op::Bind(node(3), Some(handler(3, 3))),
            Op::SetCommandBinding(node(3), Some(query(binding::Context::Focused))),
        ],
    );
    frame(cx, window).await;
    assert_eq!(
        registry(&observations(transport)[&node(3)]),
        binding::Disposition::Override
    );
}
