//! Production mounted limits, actual accessibility expansion and resize/disposal.
use super::*;

fn large_scene(color: i64) -> Scene {
    Scene {
        version: 1,
        description: "Twenty thousand marks with two thousand forty-eight interactive objects"
            .into(),
        resources: vec![],
        items: (0..20_000)
            .map(|index| {
                let bounds = Rect {
                    x: 0.,
                    y: 0.,
                    width: 2.,
                    height: 2.,
                };
                Item {
                    id: index + 1,
                    transform: Transform {
                        tx: ((index % 64) * 3) as f64,
                        ty: (((index / 64) % 64) * 3) as f64,
                        ..Transform::IDENTITY
                    },
                    clips: vec![],
                    interaction: (index >= 20_000 - 2048).then(|| {
                        gpuio_protocol::canvas_scene::Interaction {
                            label: format!("Load item {index}"),
                            hit_region: HitRegion::Rectangle(bounds),
                            draggable: true,
                            activatable: true,
                        }
                    }),
                    drawing: Drawing::Shape(
                        Shape::Rectangle(bounds),
                        Paint {
                            fill: Some(color),
                            stroke: None,
                        },
                    ),
                }
            })
            .collect(),
    }
}
#[cfg(target_os = "macos")]
fn accessible_counts(cx: &mut gpui::AsyncApp, handle: WindowHandle<View>) -> (usize, usize) {
    use objc2::{msg_send, runtime::AnyObject};
    use objc2_foundation::NSString;
    unsafe fn visit(object: *mut AnyObject, depth: usize, counts: &mut (usize, usize)) {
        if object.is_null() || depth > 16 {
            return;
        }
        unsafe {
            let title: *mut NSString = msg_send![object, accessibilityTitle];
            if !title.is_null() {
                let title = (*title).to_string();
                counts.0 += usize::from(title.starts_with("Load item "));
                counts.1 += usize::from(title.starts_with("Activate Load item "));
            }
            let children: *mut AnyObject = msg_send![object, accessibilityChildren];
            if !children.is_null() {
                let count: usize = msg_send![children, count];
                assert!(count <= 2048);
                for index in 0..count {
                    visit(msg_send![children, objectAtIndex: index], depth + 1, counts);
                }
            }
        }
    }
    let view = super::super::super::editor_test::native_view(cx, handle) as *mut AnyObject;
    let mut counts = (0, 0);
    unsafe {
        let window: *mut AnyObject = msg_send![view, window];
        visit(msg_send![window, contentView], 0, &mut counts);
    }
    counts
}
fn pixel(cx: &mut gpui::AsyncApp, handle: WindowHandle<View>, coordinate: f32) -> [u8; 4] {
    handle
        .update(cx, |_, window, _| {
            let image = window.render_to_image().unwrap();
            let p = (coordinate * window.scale_factor()) as u32;
            image.get_pixel(p, p).0
        })
        .unwrap()
}
pub(super) async fn exercise(
    cx: &mut gpui::AsyncApp,
    handle: WindowHandle<View>,
    session: SharedSession,
    transport: Arc<Transport>,
) {
    let scene = large_scene(0x00ff00ff);
    let mut bytes = Vec::new();
    scene.binprot_write(&mut bytes).unwrap();
    let started = std::time::Instant::now();
    for cycle in 0..3 {
        let root = NodeId::from_parts(0, cycle + 2).unwrap();
        let canvas = NodeId::from_parts(1, cycle + 2).unwrap();
        let source = match session.borrow_mut().canvas_request(Request::Create) {
            Response::Created(id) => id,
            _ => panic!("create workload scene"),
        };
        publish_scene(&session, source, 0, 1, scene.clone());
        apply(cx, handle, mount_nodes(source, root, canvas));
        ready_node(cx, handle, canvas, 1).await;
        #[cfg(target_os = "macos")]
        {
            accessible_counts(cx, handle); // activate if needed
            frame(cx, handle).await;
            assert_eq!(accessible_counts(cx, handle), (2048, 2048));
        }
        draw(cx, handle);
        handle
            .update(cx, |view, _, _| {
                assert_eq!(view.canvas_budget.borrow().used_vertices(), 120_000);
            })
            .unwrap();
        assert_eq!(pixel(cx, handle, 130.), [0, 255, 0, 255]);
        apply(
            cx,
            handle,
            vec![Op::SetStyle(
                canvas,
                vec![Style::Fields(vec![
                    Field::Width(Length::Px(120.)),
                    Field::Height(Length::Px(120.)),
                ])],
            )],
        );
        draw(cx, handle);
        assert_eq!(pixel(cx, handle, 130.), [16, 16, 16, 255]);
        assert_eq!(pixel(cx, handle, 31.), [0, 255, 0, 255]);
        publish_scene(&session, source, 1, 1, large_scene(0x0000ffff));
        handle
            .update(cx, |view, window, cx| {
                view.canvas_changed(source, window, cx)
            })
            .unwrap();
        ready_node(cx, handle, canvas, 2).await;
        assert_eq!(pixel(cx, handle, 31.), [0, 0, 255, 255]);
        assert!(
            !observations(&transport)
                .iter()
                .any(|event| matches!(event, Observation::Failed(_)))
        );
        apply(
            cx,
            handle,
            vec![
                Op::Splice(root, 0, 1, vec![]),
                Op::Remove(canvas),
                Op::SetRoot(None),
                Op::Remove(root),
            ],
        );
        assert_eq!(
            session
                .borrow_mut()
                .canvas_request(Request::Release(source)),
            Response::Ack
        );
        frame(cx, handle).await;
        #[cfg(target_os = "macos")]
        assert_eq!(accessible_counts(cx, handle), (0, 0));
        assert_eq!(session.borrow().retained_canvas_bytes(), 0, "cycle {cycle}");
        cx.update(|cx| {
            assert_eq!(canvas_host::measurements(cx).unwrap().3, 0);
            assert_eq!(crate::canvas_content::retained_text_bytes(cx), 0);
        });
    }
    eprintln!(
        "GPUIO_CANVAS_MOUNTED_WORKLOAD_OK: 20000 items, 2048 interactive, {} encoded bytes; 3 mount/update/resize/dispose cycles {:?}; 120000 shape vertices per full frame, 4096 AX nodes on macOS; zero scene/mesh/text accounting after each cycle",
        bytes.len(),
        started.elapsed()
    );
}
