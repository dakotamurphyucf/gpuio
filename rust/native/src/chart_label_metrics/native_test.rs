//! Executes the platform font shaper on a worker without opening a window.
//! This is not rendered-label, focus, IME or accessibility acceptance.
use super::*;
use crate::chart_paint::{Layout, prepare_with_text};
use gpuio_protocol::{
    chart_data::{Edge, Node},
    chart_node_labels::{Line, Node as LabelNode},
    chart_options::{LabelPlacement, Options},
    chart_sampling::Policy,
};
use std::{cell::RefCell, rc::Rc};

fn exercise(context: Context, ui_thread: std::thread::ThreadId) {
    assert_ne!(std::thread::current().id(), ui_thread);
    let data = Data {
        version: 1,
        contents: Contents::Sankey(
            ["WWWWWW", "iiiiii", "日本語 λ 👨‍👩‍👧‍👦"]
                .into_iter()
                .enumerate()
                .map(|(i, label)| Node {
                    id: i as i64 + 1,
                    label: label.into(),
                })
                .collect(),
            vec![
                Edge {
                    id: 1,
                    source: 1,
                    target: 2,
                    value: 1.,
                },
                Edge {
                    id: 2,
                    source: 2,
                    target: 3,
                    value: 1.,
                },
            ],
        ),
    };
    let cancel = AtomicBool::new(false);
    let mut style = Style::default();
    let initial = context.measure(&data, &style, &cancel).unwrap();
    let wide = initial[0].unwrap().width;
    let narrow = initial[1].unwrap().width;
    assert!(
        wide > narrow * 1.5,
        "proportional glyph widths: {wide}/{narrow}"
    );
    assert!(initial[2].unwrap().width > 0.);
    let mut padded = context.clone();
    padded.style.padding += 5.;
    let extra = padded.measure(&data, &style, &cancel).unwrap();
    assert!((extra[0].unwrap().width - wide - 10.).abs() < 0.001);
    assert!(context != padded);
    let mut bold = context.clone();
    bold.style.font = bold.style.font.bold();
    assert!(context != bold);
    let bold_metrics = bold.measure(&data, &style, &cancel).unwrap();
    assert!(
        bold_metrics
            .iter()
            .all(|metric| metric.is_some_and(|m| m.width > 0.))
    );
    style.node_labels = vec![
        LabelNode {
            node: 1,
            lines: vec![],
        },
        LabelNode {
            node: 2,
            lines: vec![
                Line {
                    text: "WWWWWW".into(),
                    font_size: Some(32.),
                    color: None,
                },
                Line {
                    text: "日本語 λ".into(),
                    font_size: Some(14.),
                    color: None,
                },
            ],
        },
    ];
    let rich = context.measure(&data, &style, &cancel).unwrap();
    assert!(rich[0].is_none());
    assert!(rich[1].unwrap().width > wide * 2.);
    assert_eq!(rich[1].unwrap().height, 54.);
    let mut options = Options::default();
    options.sankey.label_placement = LabelPlacement::Outside;
    for scale in [1., 1.25, 1.5, 2.] {
        let plan = prepare_with_text(
            &data,
            Policy::default(),
            &options,
            &style,
            Layout::new(500., 300., scale).unwrap(),
            Some(&context),
            &cancel,
        )
        .unwrap();
        let labels = &plan.geometry().labels;
        assert_eq!(labels.len(), 3);
        assert!(labels.iter().all(|label| match label.kind {
            crate::chart_geometry::LabelKind::Flow { placement, .. }
            | crate::chart_geometry::LabelKind::FlowLine { placement, .. } => placement.is_some(),
            _ => false,
        }));
    }
    assert_eq!(
        context.measure(&data, &style, &AtomicBool::new(true)),
        Err(Error::Cancelled)
    );
    eprintln!(
        "NATIVE_CHART_LABEL_METRICS_PASS: worker platform shaping, Unicode, proportional widths, padding, rich/hidden labels and four preparation scales; no window opened"
    );
}

pub(crate) fn run() {
    let failure = Rc::new(RefCell::new(None));
    let result = failure.clone();
    gpui_platform::application().run(move |cx| {
        cx.set_quit_mode(gpui::QuitMode::Explicit);
        let context = Context {
            style: LabelStyle::new(gpui::font(".SystemUIFont"), 4.),
            system: cx.text_system().clone(),
        };
        let ui_thread = std::thread::current().id();
        let task = cx.background_executor().spawn(async move {
            std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                exercise(context, ui_thread)
            }))
        });
        cx.spawn(async move |cx| {
            let timeout = cx
                .background_executor()
                .timer(std::time::Duration::from_secs(20));
            *result.borrow_mut() = crate::host::native_test::protect(async {
                match futures_lite::future::race(async { Some(task.await) }, async {
                    timeout.await;
                    None
                })
                .await
                {
                    Some(Ok(())) => (),
                    Some(Err(error)) => std::panic::resume_unwind(error),
                    None => panic!("native chart font measurement timed out"),
                }
            })
            .await
            .err();
            cx.update(crate::host::stop_application);
        })
        .detach();
    });
    if let Some(error) = failure.borrow_mut().take() {
        std::panic::resume_unwind(error);
    }
}
