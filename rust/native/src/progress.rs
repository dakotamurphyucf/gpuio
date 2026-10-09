//! Mounted progress presentation, weak native frame demand and close cleanup.
use gpui::{canvas, prelude::*};

#[cfg(feature = "native-tests")]
#[derive(Clone, Copy, Default)]
pub(super) struct Paint {
    pub bounds: gpui::Bounds<gpui::Pixels>,
    pub color: gpui::Hsla,
    pub count: u64,
}

#[cfg(all(test, feature = "native-image-tests"))]
#[path = "progress_lifecycle_test.rs"]
mod lifecycle_tests;
#[cfg(feature = "native-tests")]
pub(super) type Probe = std::rc::Rc<std::cell::Cell<Paint>>;

impl super::View {
    pub(super) fn sync_progress(&mut self, window: &gpui::Window, cx: &mut gpui::Context<Self>) {
        if self.progress_close.is_none() {
            let window_id = window.window_handle().window_id();
            let view = cx.entity().downgrade();
            self.progress_close = Some(cx.on_window_closed(move |cx, closed| {
                if closed == window_id {
                    let _ = view.update(cx, |view, _| view.progresses.clear());
                }
            }));
        }
        let session = self.session.borrow();
        self.progresses.retain(|id, owner| {
            let Some(config) = session
                .tree(self.id)
                .and_then(|tree| tree.get(*id))
                .and_then(|node| node.progress_presentation.as_ref())
            else {
                return false;
            };
            owner
                .update(config.clone())
                .expect("admitted progress configuration");
            owner.prepare_frame();
            true
        });
    }

    pub(super) fn progress_element(
        &mut self,
        node: &crate::tree::Node,
        corners: super::image_corners::Shared,
        inert: bool,
    ) -> gpui::AnyElement {
        let owner = self.progresses.entry(node.id).or_insert_with(|| {
            crate::progress_clock::Owner::new(
                node.progress_presentation.as_ref().unwrap().clone(),
                self.progress_clock.clone(),
            )
            .expect("admitted progress configuration")
        });
        let driver = owner.driver();
        #[cfg(feature = "native-tests")]
        let probe = self.progress_probes.entry(node.id).or_default().clone();
        canvas(
            |_, _, _| (),
            move |bounds, _, window, cx| {
                let report =
                    crate::progress_paint::paint(&driver, bounds, corners.get(), inert, window, cx);
                #[cfg(not(feature = "native-tests"))]
                let _ = report;
                #[cfg(feature = "native-tests")]
                if let crate::progress_paint::Report::Visible { sample, .. } = report {
                    let mut fill = bounds;
                    if sample.shape == gpuio_protocol::progress_presentation::Shape::Linear {
                        let (start, end) = match sample.value {
                            crate::progress_clock::Value::Determinate(value) => (0., value as f32),
                            crate::progress_clock::Value::Indeterminate {
                                static_presentation: true,
                                ..
                            } => (0.375, 0.625),
                            crate::progress_clock::Value::Indeterminate { phase, .. } => {
                                let start = phase * 1.25 - 0.25;
                                (start.clamp(0., 1.), (start + 0.25).clamp(0., 1.))
                            }
                        };
                        fill.origin.x += bounds.size.width * start;
                        fill.size.width = bounds.size.width * (end - start);
                    }
                    probe.set(Paint {
                        bounds: fill,
                        color: window.text_style().color,
                        count: probe.get().count + 1,
                    });
                }
            },
        )
        .absolute()
        .top_0()
        .left_0()
        .size_full()
        .into_any_element()
    }
}
