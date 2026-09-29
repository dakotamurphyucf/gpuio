//! Production-document GPU and installed-revision lifecycle checks. Rendering
//! starts in the background; selection painting briefly requires an active window.
use super::*;
use crate::{session::Session, transport::Transport};
use gpui::{AppContext, AsyncApp, Bounds, WindowBounds, WindowHandle, WindowOptions, size};
use gpuio_protocol::{
    HandlerId, WindowId,
    document::{Mode, Request, Response, Status, Update},
    highlight,
};
use std::{
    os::fd::{AsRawFd, FromRawFd, OwnedFd},
    time::Duration,
};
fn node(n: i64) -> NodeId {
    NodeId::from_parts(n, 1).unwrap()
}
fn handler(n: i64) -> HandlerId {
    HandlerId::from_parts(0, n).unwrap()
}
fn config(radius: f64) -> highlight::Config {
    highlight::Config(vec![highlight::Spec {
        query: Some(highlight::Query {
            text: "aaa".into(),
            case_sensitive: true,
            whole_word: false,
        }),
        ranges: vec![],
        appearance: highlight::Appearance {
            color: 0xff0000ff,
            active_color: 0xff0000ff,
            radius,
        },
        active_index: None,
        match_index_offset: 0,
    }])
}
fn document(source: ResourceId, mode: Mode) -> Config {
    Config {
        source: Some(source),
        mode,
        dark: false,
        layout: Layout::Viewport(230.),
        label: "Highlighted document".into(),
        path: None,
        line_numbers: false,
        initially_collapsed: false,
        search: String::new(),
        images: vec![],
    }
}
fn publish(session: &mut Session, source: ResourceId, base: i64, text: &str) {
    assert_eq!(
        session.document_request(Request::Begin(Update {
            id: source,
            base,
            revision: base + 1,
            generation: base + 1,
            from_byte: 0,
            suffix_bytes: text.len() as i64,
            status: Status::Complete
        })),
        Response::Ack
    );
    assert_eq!(
        session.document_request(Request::Chunk(
            source,
            base + 1,
            0,
            gpuio_protocol::asset::Chunk::new(text.as_bytes().to_vec()).unwrap()
        )),
        Response::Ack
    );
    assert_eq!(
        session.document_request(Request::Publish(source, base + 1)),
        Response::Ack
    );
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
                .unwrap();
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
async fn ready(
    cx: &mut AsyncApp,
    handle: WindowHandle<View>,
    transport: &Transport,
    count: i64,
) -> highlight::Observation {
    for _ in 0..1000 {
        let events = transport.mailbox.lock().unwrap().drain(128);
        for event in events {
            if let Event::HighlightObserved(_, _, _, _, sample) = event
                && sample.state
                    == highlight::State::Ready(vec![highlight::Count {
                        total: count,
                        stored: count,
                    }])
            {
                draw(cx, handle);
                return sample;
            }
        }
        pause(cx).await;
    }
    let state = handle
        .update(cx, |v, _, _| {
            v.highlights.get(&node(0)).map(|s| s.borrow().observation())
        })
        .unwrap();
    panic!("document counts did not reach {count}: {state:?}");
}
fn red_pixels(cx: &mut AsyncApp, handle: WindowHandle<View>) -> usize {
    handle
        .update(cx, |_, window, _| {
            window
                .render_to_image()
                .unwrap()
                .pixels()
                .filter(|pixel| pixel.0 == [255, 0, 0, 255])
                .count()
        })
        .unwrap()
}
fn presentation(cx: &mut AsyncApp, handle: WindowHandle<View>) -> Entity<Presentation> {
    handle
        .update(cx, |v, _, _| {
            v.documents[&node(1)].presentation.clone().unwrap()
        })
        .unwrap()
}

async fn installed_revision(
    cx: &mut AsyncApp,
    handle: WindowHandle<View>,
    transport: &Transport,
    p: &Entity<Presentation>,
    revision: i64,
) {
    for _ in 0..1000 {
        // Rebinding while preparation is pending can correctly report the old
        // installed document. Wait for the requested installation, then bind a
        // fresh observer after draining those acknowledged older samples.
        transport.mailbox.lock().unwrap().drain(128);
        draw(cx, handle);
        if p.read_with(cx, |p, _| {
            p.installed.as_ref().is_some_and(|s| s.revision == revision)
        }) {
            transport.mailbox.lock().unwrap().drain(128);
            apply(
                cx,
                handle,
                vec![Op::Bind(node(0), Some(handler(100 + revision)))],
            );
            return;
        }
        pause(cx).await;
    }
    panic!("requested document revision {revision} was not installed");
}

async fn markdown_checks(
    cx: &mut AsyncApp,
    handle: WindowHandle<View>,
    session: &Rc<RefCell<Session>>,
    transport: &Transport,
    source: ResourceId,
    p: &Entity<Presentation>,
) {
    let mut base = 3;
    for (name, text, count) in [
        ("heading", "# aaa\n", 1),
        ("formatting", "a**a**a\n", 1),
        ("inline code split", "a`a`a\n", 1),
        ("fenced code", "```txt\naaa\n```\n", 1),
        ("literal HTML block", "<div>aaa</div>\n", 1),
        (
            "literal HTML inline",
            "before <span title=\"aaa\">text</span> after\n",
            1,
        ),
        ("image placeholder", "![aaa](asset://highlight)\n", 1),
        (
            "table cells",
            "| aaa | aaa |\n|---|---|\n| aaa | aaa |\n",
            4,
        ),
        (
            "wrapped text",
            "aaa aaa aaa aaa aaa aaa aaa aaa aaa aaa aaa aaa aaa aaa aaa aaa aaa aaa aaa aaa\n",
            20,
        ),
    ] {
        // Drop acknowledged observations before starting a distinct revision.
        transport.mailbox.lock().unwrap().drain(128);
        publish(&mut session.borrow_mut(), source, base, text);
        handle
            .update(cx, |view, _, cx| view.document_changed(source, cx))
            .unwrap();
        apply(
            cx,
            handle,
            vec![
                Op::SetDocument(node(1), document(source, Mode::Markdown)),
                Op::Bind(node(0), Some(handler(base + 3))),
                Op::SetHighlightScope(node(0), config(0.)),
            ],
        );
        installed_revision(cx, handle, transport, p, base + 1).await;
        ready(cx, handle, transport, count).await;
        p.read_with(cx, |p, _| {
            assert_eq!(p.installed.as_ref().unwrap().revision, base + 1)
        });
        let pixels = red_pixels(cx, handle);
        assert!(
            pixels > 20,
            "{name} paints rounded-background provider: {pixels}"
        );
        let markdown = p.read_with(cx, |p, _| p.markdown.clone()).unwrap();
        markdown.update(cx, |state, cx| state.select_all(cx));
        draw(cx, handle);
        assert!(
            red_pixels(cx, handle) < pixels,
            "{name}: selection paints above highlights"
        );
        markdown.update(cx, |state, cx| state.clear_selection(cx));
        draw(cx, handle);
        assert_eq!(
            red_pixels(cx, handle),
            pixels,
            "{name}: removing selection restores washes"
        );
        base += 1;
    }
    let before = p.read_with(cx, |p, cx| {
        p.markdown
            .as_ref()
            .unwrap()
            .read(cx)
            .displayed_text()
            .unwrap()
    });
    let old_backgrounds = p.read_with(cx, |p, _| {
        p.markdown_highlight.as_ref().unwrap().backgrounds.clone()
    });
    publish(
        &mut session.borrow_mut(),
        source,
        base,
        "# aaa\n\naaa **aaa** aaa\n",
    );
    handle
        .update(cx, |view, _, cx| view.document_changed(source, cx))
        .unwrap();
    p.read_with(cx, |p, cx| {
        assert!(
            Arc::ptr_eq(
                &before,
                &p.markdown
                    .as_ref()
                    .unwrap()
                    .read(cx)
                    .displayed_text()
                    .unwrap()
            ),
            "pending parse retains installed fragments"
        )
    });
    installed_revision(cx, handle, transport, p, base + 1).await;
    ready(cx, handle, transport, 4).await;
    p.read_with(cx, |p, cx| {
        assert!(!Arc::ptr_eq(
            &before,
            &p.markdown
                .as_ref()
                .unwrap()
                .read(cx)
                .displayed_text()
                .unwrap()
        ))
    });
    let square = red_pixels(cx, handle);
    let markdown = p.read_with(cx, |p, _| p.markdown.clone()).unwrap();
    assert!(
        !markdown.update(cx, |state, cx| state
            .set_text_backgrounds(Some(old_backgrounds), cx)),
        "an old prepared AST cannot reinstall its backgrounds on a streamed replacement"
    );
    draw(cx, handle);
    assert_eq!(
        red_pixels(cx, handle),
        square,
        "current presenter restores only the valid prepared owner"
    );
    apply(
        cx,
        handle,
        vec![
            Op::Bind(node(0), Some(handler(40))),
            Op::SetHighlightScope(node(0), config(8.)),
        ],
    );
    ready(cx, handle, transport, 4).await;
    let rounded = red_pixels(cx, handle);
    assert!(
        rounded > 20 && rounded < square,
        "Markdown radius: square={square}, rounded={rounded}"
    );
    p.update(cx, |p, cx| {
        p.collapsed = true;
        p.invalidate_row(cx);
        cx.notify();
    });
    ready(cx, handle, transport, 0).await;
    assert_eq!(red_pixels(cx, handle), 0);
    p.update(cx, |p, cx| {
        p.collapsed = false;
        p.invalidate_row(cx);
        cx.notify();
    });
    ready(cx, handle, transport, 4).await;
}

async fn image_checks(
    cx: &mut AsyncApp,
    handle: WindowHandle<View>,
    session: &Rc<RefCell<Session>>,
    transport: &Transport,
    source: ResourceId,
    p: &Entity<Presentation>,
) {
    let base = p.read_with(cx, |p, _| p.installed.as_ref().unwrap().revision);
    publish(
        &mut session.borrow_mut(),
        source,
        base,
        "![aaa](asset://highlight) ![aaa](asset://highlight)\n",
    );
    handle
        .update(cx, |view, _, cx| view.document_changed(source, cx))
        .unwrap();
    installed_revision(cx, handle, transport, p, base + 1).await;
    ready(cx, handle, transport, 2).await;
    assert!(
        red_pixels(cx, handle) > 20,
        "visible image placeholder is searchable"
    );
    let mut active = config(8.);
    active.0[0].active_index = Some(1);
    active.0[0].appearance.active_color = 0x00ff00ff;
    apply(
        cx,
        handle,
        vec![
            Op::Bind(node(0), Some(handler(300))),
            Op::SetHighlightScope(node(0), active),
        ],
    );
    ready(cx, handle, transport, 2).await;
    let green = handle
        .update(cx, |_, window, _| {
            window
                .render_to_image()
                .unwrap()
                .pixels()
                .filter(|p| p.0 == [0, 255, 0, 255])
                .count()
        })
        .unwrap();
    assert!(
        green > 20 && red_pixels(cx, handle) > 20,
        "identical custom objects retain distinct match ordinals"
    );
    let before = p.read_with(cx, |p, cx| {
        p.markdown
            .as_ref()
            .unwrap()
            .read(cx)
            .displayed_text()
            .unwrap()
    });
    let image = {
        let mut session = session.borrow_mut();
        let store = session.assets().unwrap();
        let mut bytes = b"P6\n16 16\n255\n".to_vec();
        for _ in 0..256 {
            bytes.extend([20, 100, 240]);
        }
        let image = store
            .begin(gpuio_protocol::asset::Format::Pnm, bytes.len())
            .unwrap();
        store.append(image, 0, &bytes).unwrap();
        store.finish(image).unwrap();
        image
    };
    let mut with_image = document(source, Mode::Markdown);
    with_image.images = vec![("asset://highlight".into(), ImageSource::Reference(image))];
    apply(cx, handle, vec![Op::SetDocument(node(1), with_image)]);
    ready(cx, handle, transport, 0).await;
    assert_eq!(
        red_pixels(cx, handle),
        0,
        "loaded image has no placeholder glyph wash"
    );
    let blue = handle
        .update(cx, |_, window, _| {
            window
                .render_to_image()
                .unwrap()
                .pixels()
                .filter(|p| p.0[2] > 200 && p.0[0] < 80 && p.0[1] > 60 && p.0[1] < 160)
                .count()
        })
        .unwrap();
    assert!(blue > 50, "decoded image actually paints: {blue}");
    p.read_with(cx, |p, cx| {
        let displayed = p
            .markdown
            .as_ref()
            .unwrap()
            .read(cx)
            .displayed_text()
            .unwrap();
        assert!(!Arc::ptr_eq(&before, &displayed));
        assert_eq!(displayed.opaque_nodes(), 0);
        assert!(
            displayed
                .fragments()
                .iter()
                .all(|f| f.text().trim().is_empty())
        );
        assert_eq!(
            p.installed.as_ref().unwrap().revision,
            base + 1,
            "resource-only projection does not reparse the document"
        );
    });
    apply(
        cx,
        handle,
        vec![Op::SetDocument(node(1), document(source, Mode::Markdown))],
    );
    ready(cx, handle, transport, 2).await;
    assert!(
        red_pixels(cx, handle) > 20,
        "resource removal restores visible placeholder query"
    );
    let markdown = p.read_with(cx, |p, _| p.markdown.clone()).unwrap();
    markdown.update(cx, |state, cx| state.select_all(cx));
    draw(cx, handle);
    markdown.read_with(cx, |state, _| {
        assert_eq!(
            state.selected_text().trim(),
            "aaa aaa",
            "copy alternative stays independent of painted placeholder"
        )
    });
    markdown.update(cx, |state, cx| state.clear_selection(cx));
    draw(cx, handle);
    apply(
        cx,
        handle,
        vec![
            Op::Bind(node(0), Some(handler(399))),
            Op::SetHighlightScope(node(0), highlight::Config(vec![])),
        ],
    );
    draw(cx, handle);
    p.read_with(cx, |p, _| {
        assert!(
            p.markdown_highlight.is_none(),
            "empty scopes retain no Markdown paint owner"
        )
    });
    apply(
        cx,
        handle,
        vec![
            Op::Bind(node(0), Some(handler(400))),
            Op::SetHighlightScope(node(0), config(8.)),
        ],
    );
    ready(cx, handle, transport, 2).await;
    session
        .borrow_mut()
        .assets()
        .unwrap()
        .release(image)
        .unwrap();
}
async fn directional_checks(
    cx: &mut AsyncApp,
    handle: WindowHandle<View>,
    session: &Rc<RefCell<Session>>,
    transport: &Transport,
    source: ResourceId,
    p: &Entity<Presentation>,
) {
    for (index, mode) in [Mode::Code("txt".into()), Mode::Markdown]
        .into_iter()
        .enumerate()
    {
        let base = p.read_with(cx, |p, _| p.installed.as_ref().unwrap().revision);
        publish(&mut session.borrow_mut(), source, base, "אבג\n");
        handle
            .update(cx, |view, _, cx| view.document_changed(source, cx))
            .unwrap();
        let mut search = config(0.);
        search.0[0].query.as_mut().unwrap().text = "ב".into();
        apply(
            cx,
            handle,
            vec![
                Op::Bind(node(0), Some(handler(500 + index as i64))),
                Op::SetHighlightScope(node(0), search),
                Op::SetDocument(node(1), document(source, mode)),
            ],
        );
        installed_revision(cx, handle, transport, p, base + 1).await;
        // Installation polling can drain Ready; rotate just the observer after it.
        apply(
            cx,
            handle,
            vec![Op::Bind(node(0), Some(handler(510 + index as i64)))],
        );
        ready(cx, handle, transport, 1).await;
        assert!(
            red_pixels(cx, handle) > 20,
            "RTL middle glyph receives a wash in adapter {index}"
        );
    }
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
        gpui_base::init(cx);cx.set_quit_mode(gpui::QuitMode::Explicit);
        let session=Rc::new(RefCell::new(Session::default()));
        session.borrow_mut().hello(VERSION,CAPABILITIES).unwrap();
        let id=WindowId::from_parts(0,1).unwrap();session.borrow_mut().open(1,id,"Document highlighting",440.,340.).unwrap();
        let Response::Created(source)=session.borrow_mut().document_request(Request::Create)else{panic!("source")};
        let text="aaa before\nbbb middle\naaa tail\n";publish(&mut session.borrow_mut(),source,0,text);
        let handle=cx.open_window(WindowOptions{window_bounds:Some(WindowBounds::Windowed(Bounds::centered(None,size(px(440.),px(340.)),cx))),focus:false,..Default::default()},|_,cx|cx.new(|_|View::new(id,session.clone(),transport.clone()))).unwrap();
        cx.spawn(async move|cx|{
            let checked=crate::host::native_test::protect(async{
                apply(cx,handle,vec![Op::Create(node(0),Kind::HighlightScope,"".into(),Some(handler(1))),Op::SetHighlightScope(node(0),config(0.)),Op::SetStyle(node(0),vec![Style::Fields(vec![Field::Width(Length::Px(440.)),Field::Height(Length::Px(340.)),Field::Background(Fill::Solid(Color::Rgba(0xffffffff)))])]),Op::Create(node(1),Kind::DocumentView,"".into(),None),Op::SetDocument(node(1),document(source,Mode::Code("txt".into()))),Op::Splice(node(0),0,0,vec![node(1)]),Op::SetRoot(Some(node(0)))]);
                let initial=ready(cx,handle,&transport,2).await;
                let square=red_pixels(cx,handle);assert!(square>20,"code has visible red washes: {square}");
                apply(cx,handle,vec![Op::Bind(node(0),Some(handler(2))),Op::SetHighlightScope(node(0),config(8.))]);
                let rounded=ready(cx,handle,&transport,2).await;assert_eq!(rounded.epoch,initial.epoch);
                let pixels=red_pixels(cx,handle);assert!(pixels>20&&pixels<square,"rounded corners remove pixels: square={square}, round={pixels}");
                let p=presentation(cx,handle);
                cx.update(|cx|cx.activate(true));
                handle.update(cx,|_,window,cx|{window.activate_window();window.focus(&p.read(cx).editor.read(cx).focus_handle(cx),cx);}).unwrap();
                for _ in 0..300 { if handle.update(cx,|_,window,_|window.is_window_active()).unwrap(){break;} pause(cx).await; }
                assert!(handle.update(cx,|_,window,_|window.is_window_active()).unwrap(),"selection requires active test window");
                p.update(cx,|p,cx|p.editor.update(cx,|editor,cx|editor.bridge_select(0,3,cx)));
                draw(cx,handle);let selected_pixels=red_pixels(cx,handle);assert!(selected_pixels<pixels,"native selection paints above search wash");
                p.update(cx,|p,cx|p.editor.update(cx,|editor,cx|editor.bridge_select(0,0,cx)));
                draw(cx,handle);assert!(red_pixels(cx,handle)>selected_pixels,"clearing selection restores search wash");
                p.update(cx,|p,cx|{p.collapsed=true;p.invalidate_row(cx);cx.notify();});
                ready(cx,handle,&transport,0).await;assert_eq!(red_pixels(cx,handle),0);
                p.update(cx,|p,cx|{p.collapsed=false;p.invalidate_row(cx);cx.notify();});
                ready(cx,handle,&transport,2).await;
                handle.update(cx,|_,window,cx|p.update(cx,|p,cx|{p.page_start=text.find("aaa tail").unwrap();p.show_page(window,cx);cx.notify();})).unwrap();
                ready(cx,handle,&transport,1).await;
                handle.update(cx,|_,window,cx|p.update(cx,|p,cx|{p.page_start=0;p.show_page(window,cx);cx.notify();})).unwrap();
                ready(cx,handle,&transport,2).await;
                publish(&mut session.borrow_mut(),source,1,"aaa aaa aaa\n");
                handle.update(cx,|view,_,cx|view.document_changed(source,cx)).unwrap();
                p.read_with(cx,|p,_|assert_eq!(p.installed.as_ref().unwrap().revision,1,"pending parse keeps old installed source"));
                ready(cx,handle,&transport,3).await;
                p.read_with(cx,|p,_|assert_eq!(p.installed.as_ref().unwrap().revision,2));
                publish(&mut session.borrow_mut(),source,2,"--- a/a\n+++ b/a\n@@ -1 +1 @@\n-aaa\n+aaa\n");
                handle.update(cx,|view,_,cx|view.document_changed(source,cx)).unwrap();
                apply(cx,handle,vec![Op::SetDocument(node(1),document(source,Mode::Diff))]);
                ready(cx,handle,&transport,2).await;assert!(red_pixels(cx,handle)>20,"diff washes paint over syntax backgrounds");
                let retired=p.read_with(cx,|p,_|p.highlight_paint.as_ref().map(|(paint,_)|paint.clone())).unwrap();
                let background_owner=p.read_with(cx,|p,_|Rc::downgrade(&p.highlight_paint.as_ref().unwrap().1));
                markdown_checks(cx,handle,&session,&transport,source,&p).await;
                image_checks(cx,handle,&session,&transport,source,&p).await;
                let markdown_owner=p.read_with(cx,|p,_|Rc::downgrade(&p.markdown_highlight.as_ref().unwrap().backgrounds));
                directional_checks(cx,handle,&session,&transport,source,&p).await;
                assert!(markdown_owner.upgrade().is_none(),"replaced image Markdown releases prepared owner");
                let directional_owner=p.read_with(cx,|p,_|Rc::downgrade(&p.markdown_highlight.as_ref().unwrap().backgrounds));
                apply(cx,handle,vec![Op::SetRoot(None),Op::Remove(node(1)),Op::Remove(node(0))]);
                draw(cx,handle);pause(cx).await;drop(p);drop(retired);draw(cx,handle);
                assert!(background_owner.upgrade().is_none(),"retired editor frame releases prepared owner");
                assert!(markdown_owner.upgrade().is_none(),"retired Markdown frame releases prepared owner");
                assert!(directional_owner.upgrade().is_none(),"unmounted directional Markdown releases prepared owner");
                assert_eq!(red_pixels(cx,handle),0);
                eprintln!("GPUIO_NATIVE_HIGHLIGHT_DOCUMENT_OK: code/diff/Markdown GPU washes; headings, cross-format and inline-code runs, fenced code, tables and wrap; rounded radius, selection precedence, cosmetic epoch, collapse, native pages, pending installed revision, streaming replacement and unmount/owner disposal");
            }).await;
            *task_failure.borrow_mut()=checked.err();
            let _=handle.update(cx,|_,window,_|window.remove_window());
            crate::highlight_host::shutdown(cx).await;crate::document_host::shutdown(cx).await;crate::image_host::shutdown(cx).await;
            cx.update(crate::host::stop_application);
        }).detach();
    });
    if let Some(error) = failure.borrow_mut().take() {
        std::panic::resume_unwind(error);
    }
}
