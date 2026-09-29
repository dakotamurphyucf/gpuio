//! Production retained View, actual GPU paint and queued observations. No OS
//! keyboard/mouse events are needed; the window stays in the background.
use super::*;
use crate::{session::Session, transport::Transport};
use gpui::{AppContext, AsyncApp, Bounds, WindowBounds, WindowHandle, WindowOptions, px, size};
use gpuio_protocol::{
    WindowId,
    highlight::{Appearance, Query, Spec},
    v1::*,
};
use std::{
    os::fd::{AsRawFd, FromRawFd, OwnedFd},
    time::Duration,
};

fn id(slot: i64) -> NodeId {
    NodeId::from_parts(slot, 1).unwrap()
}
fn handler(generation: i64) -> HandlerId {
    HandlerId::from_parts(0, generation).unwrap()
}
fn config(color: i64) -> Config {
    Config(vec![Spec {
        query: Some(Query {
            text: "aaa".into(),
            case_sensitive: true,
            whole_word: false,
        }),
        ranges: vec![],
        appearance: Appearance {
            color,
            active_color: color,
            radius: 0.,
        },
        active_index: None,
        match_index_offset: 0,
    }])
}
fn row(y: f64, selectable: bool, hidden: bool) -> Vec<Style> {
    vec![Style::Fields(vec![
        Field::Position(1),
        Field::Left(Length::Px(20.)),
        Field::Top(Length::Px(y)),
        Field::Width(Length::Px(250.)),
        Field::Height(Length::Px(30.)),
        Field::FontSize(20.),
        Field::LineHeight(Length::Px(30.)),
        Field::UserSelect(selectable),
        Field::SelectionColor(Color::Rgba(0x0000ffff)),
        Field::Display(if hidden { 3 } else { 1 }),
    ])]
}
fn apply(cx: &mut AsyncApp, handle: WindowHandle<View>, operations: Vec<Op>) {
    handle
        .update(cx, |view, window, cx| {
            let base = view.session.borrow().tree(view.id).unwrap().revision();
            let applied = view
                .session
                .borrow_mut()
                .apply(&Transaction {
                    window: view.id,
                    base,
                    revision: base + 1,
                    operations,
                })
                .unwrap_or_else(|error| {
                    panic!("highlight fixture transaction {}: {error:?}", base + 1)
                });
            view.update_editors(&applied.dirty, window, cx);
            cx.notify();
        })
        .unwrap();
}
async fn pause(cx: &mut AsyncApp) {
    cx.background_executor()
        .timer(Duration::from_millis(16))
        .await;
}
fn draw(cx: &mut AsyncApp, handle: WindowHandle<View>) {
    cx.update_window(handle.into(), |_, window, cx| {
        window.refresh();
        window.draw(cx).clear(cx);
    })
    .unwrap();
}
fn observations(transport: &Transport) -> Vec<(NodeId, HandlerId, Observation)> {
    transport
        .mailbox
        .lock()
        .unwrap()
        .drain(128)
        .into_iter()
        .filter_map(|event| match event {
            Event::HighlightObserved(_, node, handler, _, sample) => Some((node, handler, sample)),
            _ => None,
        })
        .collect()
}
async fn observed(
    cx: &mut AsyncApp,
    handle: WindowHandle<View>,
    transport: &Transport,
    handler: HandlerId,
    state: Outcome,
) -> Observation {
    for _ in 0..300 {
        for (node, h, sample) in observations(transport) {
            if node == id(1) && h == handler && sample.state == state {
                return sample;
            }
        }
        pause(cx).await;
    }
    let actual = handle
        .update(cx, |view, _, _| {
            view.highlights
                .get(&id(1))
                .map(|s| s.borrow().observation())
        })
        .unwrap();
    panic!("no queued highlight observation for {handler:?}/{state:?}; mounted state {actual:?}");
}
fn ready(count: i64) -> Outcome {
    Outcome::Ready(vec![Count {
        total: count,
        stored: count,
    }])
}
fn result(cx: &mut AsyncApp, handle: WindowHandle<View>) -> Arc<jobs::Ready> {
    handle
        .update(cx, |view, _, _| {
            let scope = view.highlights[&id(1)].borrow();
            let jobs::Status::Ready(ready) = scope.job.as_ref().unwrap().status() else {
                panic!("ready scope")
            };
            ready
        })
        .unwrap()
}
fn pixel(cx: &mut AsyncApp, handle: WindowHandle<View>, x: f32, y: f32) -> [u8; 4] {
    handle
        .update(cx, |_, window, _| {
            let image = window.render_to_image().unwrap();
            let scale = window.scale_factor();
            image.get_pixel((x * scale) as u32, (y * scale) as u32).0
        })
        .unwrap()
}
pub(crate) fn run() {
    let failure = Rc::new(RefCell::new(None));
    let task_failure = failure.clone();
    let mut fds = [0; 2];
    assert_eq!(unsafe { libc::pipe(fds.as_mut_ptr()) }, 0);
    let _reader = unsafe { OwnedFd::from_raw_fd(fds[0]) };
    let writer = unsafe { OwnedFd::from_raw_fd(fds[1]) };
    let transport = Arc::new(Transport::new(writer.as_raw_fd()).unwrap());
    gpui_platform::application().run(move|cx|{
        cx.set_quit_mode(gpui::QuitMode::Explicit);
        let session=Rc::new(RefCell::new(Session::default()));
        session.borrow_mut().hello(VERSION,CAPABILITIES).unwrap();
        let window_id=WindowId::from_parts(0,1).unwrap();
        session.borrow_mut().open(1,window_id,"Mounted highlight check",400.,200.).unwrap();
        let handle=cx.open_window(WindowOptions{window_bounds:Some(WindowBounds::Windowed(Bounds::centered(None,size(px(400.),px(200.)),cx))),focus:false,..Default::default()},|_,cx|cx.new(|_|View::new(window_id,session.clone(),transport.clone()))).unwrap();
        cx.spawn(async move|cx|{
            let checked=crate::host::native_test::protect(async{
                apply(cx,handle,vec![
                    Op::Create(id(0),Kind::Container,"".into(),None),
                    Op::SetStyle(id(0),vec![Style::Fields(vec![Field::Width(Length::Px(400.)),Field::Height(Length::Px(200.)),Field::Background(Fill::Solid(Color::Rgba(0xffffffff)))])]),
                    Op::Create(id(1),Kind::HighlightScope,"".into(),Some(handler(1))),
                    Op::SetHighlightScope(id(1),config(0xff0000ff)),
                    Op::SetStyle(id(1),vec![Style::Fields(vec![Field::Width(Length::Px(400.)),Field::Height(Length::Px(200.))])]),
                    Op::Create(id(2),Kind::Text,"aaa".into(),None),Op::SetStyle(id(2),row(20.,false,false)),
                    Op::Create(id(3),Kind::Text,"aaa".into(),None),Op::SetStyle(id(3),row(60.,true,false)),
                    Op::Create(id(4),Kind::HighlightScope,"".into(),None),Op::SetHighlightScope(id(4),Config(vec![])),
                    Op::Create(id(5),Kind::Text,"aaa".into(),None),Op::SetStyle(id(5),row(100.,false,false)),
                    Op::Splice(id(4),0,0,vec![id(5)]),Op::Splice(id(1),0,0,vec![id(2),id(3),id(4)]),Op::Splice(id(0),0,0,vec![id(1)]),Op::SetRoot(Some(id(0))),
                ]);
                let initial=observed(cx,handle,&transport,handler(1),ready(2)).await;
                assert_eq!(initial.epoch,1);
                draw(cx,handle);
                assert_eq!(pixel(cx,handle,24.,22.),[255,0,0,255],"ordinary text wash");
                assert_eq!(pixel(cx,handle,24.,62.),[255,0,0,255],"selectable text wash");
                assert_eq!(pixel(cx,handle,24.,102.),[255,255,255,255],"empty nested scope blocks inherited wash");
                // Production selection decoration stays above the search wash.
                handle.update(cx,|view,_,cx| {view.selections[&id(3)].borrow_mut().selection=crate::selection::Selection{anchor:0,head:3};cx.notify();}).unwrap();
                draw(cx,handle);assert_eq!(pixel(cx,handle,24.,62.),[0,0,255,255]);
                handle.update(cx,|view,_,cx| {view.selections[&id(3)].borrow_mut().selection=Default::default();cx.notify();}).unwrap();
                draw(cx,handle);
                let original=result(cx,handle);
                // Cosmetic edits rotate the callback but retain source/result/epoch.
                apply(cx,handle,vec![Op::Bind(id(1),Some(handler(2))),Op::SetHighlightScope(id(1),config(0x00ff00ff))]);
                let cosmetic=observed(cx,handle,&transport,handler(2),ready(2)).await;
                assert_eq!(cosmetic.epoch,initial.epoch);assert!(Arc::ptr_eq(&original,&result(cx,handle)));
                draw(cx,handle);assert_eq!(pixel(cx,handle,24.,22.),[0,255,0,255]);
                // A parent layout update alone must not emit another observation.
                apply(cx,handle,vec![Op::SetStyle(id(0),vec![Style::Fields(vec![Field::Width(Length::Px(400.)),Field::Height(Length::Px(200.)),Field::Background(Fill::Solid(Color::Rgba(0xffffffff)))])])]);
                draw(cx,handle);pause(cx).await;pause(cx).await;
                assert!(observations(&transport).is_empty());
                assert!(Arc::ptr_eq(&original,&result(cx,handle)));
                // Replacing text retires the old painter while work is pending.
                apply(cx,handle,vec![Op::SetText(id(2),"zzz".into())]);
                draw(cx,handle);
                handle.update(cx,|view,_,_|{let scope=view.highlights[&id(1)].borrow();assert_eq!(scope.epoch,initial.epoch+1);assert!(scope.paints.is_empty());}).unwrap();
                let changed=observed(cx,handle,&transport,handler(2),ready(1)).await;
                assert!(changed.epoch>cosmetic.epoch);
                draw(cx,handle);assert_eq!(pixel(cx,handle,24.,22.),[255,255,255,255]);
                assert_eq!(pixel(cx,handle,24.,62.),[0,255,0,255]);
                // Visibility updates change the projection without changing labels.
                apply(cx,handle,vec![Op::SetStyle(id(3),row(60.,true,true))]);
                let hidden=observed(cx,handle,&transport,handler(2),ready(0)).await;assert!(hidden.epoch>changed.epoch);
                apply(cx,handle,vec![Op::SetStyle(id(3),row(60.,true,false))]);
                observed(cx,handle,&transport,handler(2),ready(1)).await;
                // Native structural visibility can change without a transaction.
                let before_native_hide=handle.update(cx,|view,_,_|view.session.borrow().tree(view.id).unwrap().revision()).unwrap();
                handle.update(cx,|view,_,cx|{view.focus.borrow_mut().set_query_hidden([id(3)].into_iter().collect());cx.notify();}).unwrap();
                observed(cx,handle,&transport,handler(2),ready(0)).await;
                handle.update(cx,|view,_,cx|{assert_eq!(view.session.borrow().tree(view.id).unwrap().revision(),before_native_hide);view.focus.borrow_mut().set_query_hidden(Default::default());cx.notify();}).unwrap();
                observed(cx,handle,&transport,handler(2),ready(1)).await;
                for text in ["aaa", "bbbb", "aaa aaa aaa"] {
                    apply(cx,handle,vec![Op::SetText(id(2),text.into())]);draw(cx,handle);
                }
                observed(cx,handle,&transport,handler(2),ready(4)).await;
                // Invalid UTF-8 boundary ranges are typed failures, never paint.
                let mut invalid=config(0xff0000ff);invalid.0[0].query=None;invalid.0[0].ranges=vec![gpuio_protocol::highlight::Range{start_byte:1,end_byte:2}];
                apply(cx,handle,vec![Op::SetText(id(2),"é".into()),Op::Bind(id(1),Some(handler(3))),Op::SetHighlightScope(id(1),invalid)]);
                observed(cx,handle,&transport,handler(3),Outcome::InvalidRange(InvalidRange{spec_index:0,range_index:0,reason:RangeError::ScalarBoundary})).await;
                draw(cx,handle);assert_eq!(pixel(cx,handle,24.,62.),[255,255,255,255]);
                apply(cx,handle,vec![Op::Bind(id(1),Some(handler(4))),Op::SetHighlightScope(id(1),config(0xff0000ff))]);
                observed(cx,handle,&transport,handler(4),ready(1)).await;
                let weak=handle.update(cx,|view,_,_|Rc::downgrade(&view.highlights[&id(1)])).unwrap();
                let old=Arc::downgrade(&original);drop(original);
                assert!(old.upgrade().is_none(),"retired source/result must leave scope and frame caches");
                apply(cx,handle,vec![Op::SetRoot(None),Op::Remove(id(5)),Op::Remove(id(4)),Op::Remove(id(3)),Op::Remove(id(2)),Op::Remove(id(1)),Op::Remove(id(0))]);
                draw(cx,handle);pause(cx).await;pause(cx).await;
                assert!(weak.upgrade().is_none(),"unmounted scope reclaimed after full paint");
                assert!(handle.update(cx,|view,_,_|view.highlights.is_empty()).unwrap());
                assert!(observations(&transport).is_empty(),"no callback from unmounted scope");
                eprintln!("GPUIO_NATIVE_HIGHLIGHT_VIEW_OK: mounted GPU ordinary/selectable paint and selection precedence, nested empty barrier, queued counts, cosmetic reuse, source retirement/latest update, native visibility, invalid range recovery and unmount cleanup");
            }).await;
            *task_failure.borrow_mut()=checked.err();
            let _=handle.update(cx,|_,window,_|window.remove_window());
            highlight_host::shutdown(cx).await;
            cx.update(crate::host::stop_application);
        }).detach();
    });
    if let Some(error) = failure.borrow_mut().take() {
        std::panic::resume_unwind(error);
    }
}
