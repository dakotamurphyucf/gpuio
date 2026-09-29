//! Native shaped text and GPU pixels. This tests the paint adapter independently
//! of mounted scope observation/document collection, which have separate gates.
use super::*;
use gpui::{
    Context, HighlightStyle, Render, SharedString, StyledText, Window, WindowBounds, WindowHandle,
    WindowOptions, div,
};

struct Scene {
    source: Arc<str>,
    range: Range<usize>,
    radius: f32,
    selected: bool,
    align: TextAlign,
    overflow: Option<TextOverflow>,
    cache: SharedCache,
    layout: Option<TextLayout>,
    paint: Option<Paint>,
}
impl Scene {
    fn prepare(&mut self) {
        use crate::{
            highlight_jobs as jobs,
            highlight_projection::{Group, Kind, Projection, Run, Source},
        };
        use gpuio_protocol::{
            NodeId,
            highlight::{Appearance, Range, Spec},
        };
        let key = RunKey {
            node: NodeId::from_parts(0, 1).unwrap(),
            fragment: 0,
        };
        let config = Arc::new(Config(vec![Spec {
            query: None,
            ranges: vec![Range {
                start_byte: self.range.start as i64,
                end_byte: self.range.end as i64,
            }],
            appearance: Appearance {
                color: 0xff0000ff,
                active_color: 0xff0000ff,
                radius: self.radius as f64,
            },
            active_index: None,
            match_index_offset: 0,
        }]));
        let source = Arc::new(
            Projection::new(vec![Group {
                kind: Kind::Ordinary,
                runs: vec![Run {
                    key,
                    source: Source::Text(self.source.clone()),
                }],
            }])
            .unwrap(),
        );
        let mut pool = jobs::Pool::default();
        let handle = pool.request(source, config.clone()).unwrap();
        let completion = pool.next_work().unwrap().run();
        assert!(pool.complete(completion));
        let jobs::Status::Ready(ready) = handle.status() else {
            panic!("prepared paint result")
        };
        self.paint = Some(resolve(ready, key, &config));
    }
}
impl Render for Scene {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        let mut text = StyledText::new(SharedString::from(self.source.clone()));
        if self.selected {
            text = text.with_highlights(vec![(
                self.range.clone(),
                HighlightStyle {
                    background_color: Some(rgba(0x0000ffff).into()),
                    ..Default::default()
                },
            )]);
        }
        let layout = text.layout().clone();
        self.layout = Some(layout.clone());
        let paint = self.paint.clone().unwrap();
        let mut body = div()
            .relative()
            .w(px(90.))
            .text_size(px(20.))
            .line_height(px(30.))
            .text_color(gpui::black());
        body.style().text.text_align = Some(self.align);
        if let Some(overflow) = &self.overflow {
            body.style().text.text_overflow = Some(overflow.clone());
            body = body.whitespace_nowrap().overflow_hidden();
        }
        div().size_full().bg(gpui::white()).child(
            body.child(underlay(
                self.source.clone(),
                layout,
                paint,
                self.cache.clone(),
            ))
            .child(text),
        )
    }
}
fn draw(cx: &mut gpui::AsyncApp, window: WindowHandle<Scene>) {
    cx.update_window(window.into(), |_, window, cx| window.draw(cx).clear(cx))
        .unwrap();
}
fn pixel(cx: &mut gpui::AsyncApp, window: WindowHandle<Scene>, x: f32, y: f32) -> [u8; 4] {
    window
        .update(cx, |_, window, _| {
            let image = window.render_to_image().expect("native GPU readback");
            let scale = window.scale_factor();
            image.get_pixel((x * scale) as u32, (y * scale) as u32).0
        })
        .unwrap()
}
fn layout(cx: &mut gpui::AsyncApp, window: WindowHandle<Scene>) -> TextLayout {
    window
        .update(cx, |scene, _, _| scene.layout.clone().unwrap())
        .unwrap()
}
pub(crate) fn run() {
    let failure = Rc::new(RefCell::new(None));
    let task_failure = failure.clone();
    gpui_platform::application().run(move|cx|{
        cx.set_quit_mode(gpui::QuitMode::Explicit);
        let window=cx.open_window(WindowOptions{window_bounds:Some(WindowBounds::Windowed(Bounds::centered(None,size(px(200.),px(220.)),cx))),focus:false,..Default::default()},|_,cx|cx.new(|_|{let mut scene=Scene{source:Arc::from("aaa aaa aaa aaa"),range:0..15,radius:0.,selected:false,align:TextAlign::Left,overflow:None,cache:Default::default(),layout:None,paint:None};scene.prepare();scene})).unwrap();
        cx.spawn(async move|cx|{
            let result=crate::host::native_test::protect(async{
                draw(cx,window);
                let shaped=layout(cx,window);let bounds=shaped.bounds();
                assert!(!shaped.line_layouts()[0].wrap_boundaries.is_empty());
                assert_eq!(pixel(cx,window,bounds.left().as_f32()+4.,bounds.top().as_f32()+2.),[255,0,0,255]);
                assert_eq!(pixel(cx,window,bounds.left().as_f32()+4.,bounds.top().as_f32()+32.),[255,0,0,255]);
                window.update(cx,|s,_,cx|{s.radius=8.;s.prepare();cx.notify();}).unwrap();draw(cx,window);
                assert_eq!(pixel(cx,window,bounds.left().as_f32(),bounds.top().as_f32()),[255,255,255,255],"rounded wash corner remains background");
                window.update(cx,|s,_,cx|{s.selected=true;s.prepare();cx.notify();}).unwrap();draw(cx,window);
                assert_eq!(pixel(cx,window,bounds.left().as_f32()+4.,bounds.top().as_f32()+2.),[0,0,255,255],"selection paints over the search wash");
                window.update(cx,|s,_,cx|{s.source="X".into();s.range=0..1;s.radius=0.;s.selected=false;s.align=TextAlign::Center;s.prepare();cx.notify();}).unwrap();draw(cx,window);
                let shaped=layout(cx,window);let bounds=shaped.bounds();let width=shaped.line_layouts()[0].unwrapped_layout.width;
                let x=bounds.left()+(bounds.size.width-width)/2.;
                assert_eq!(pixel(cx,window,x.as_f32()+3.,bounds.top().as_f32()+2.),[255,0,0,255]);
                window.update(cx,|s,_,cx|{s.source="prefix prefix prefix tail".into();s.range=s.source.len()-4..s.source.len();s.align=TextAlign::Left;s.overflow=Some(TextOverflow::TruncateStart("…".into()));s.prepare();cx.notify();}).unwrap();draw(cx,window);
                let shaped=layout(cx,window);let displayed=shaped.text();assert!(displayed.starts_with('…')&&displayed.ends_with("tail"));
                let position=shaped.position_for_index(displayed.len()-4).unwrap();
                assert_eq!(pixel(cx,window,position.x.as_f32()+3.,position.y.as_f32()+2.),[255,0,0,255]);
                assert_eq!(pixel(cx,window,shaped.bounds().left().as_f32()+2.,shaped.bounds().top().as_f32()+2.),[255,255,255,255],"ellipsis itself is not highlighted");
                eprintln!("GPUIO_NATIVE_HIGHLIGHT_PAINT_OK: shaped wrapping, rounded GPU pixels, selection precedence, centered text and original-byte start ellipsis mapping");
            }).await;
            *task_failure.borrow_mut()=result.err();
            let _=window.update(cx,|_,w,_|w.remove_window());
            cx.update(crate::host::stop_application);
        }).detach();
    });
    if let Some(error) = failure.borrow_mut().take() {
        std::panic::resume_unwind(error);
    }
}
