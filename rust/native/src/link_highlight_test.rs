//! Styled Link text participates in an outer scope without extra action owners.
use super::{
    content_test::{events, frame, idle},
    *,
};
use gpuio_protocol::{highlight, text_content};
use std::time::Duration;

fn highlight(query: &str, color: i64) -> highlight::Config {
    highlight::Config(vec![highlight::Spec {
        query: Some(highlight::Query {
            text: query.into(),
            case_sensitive: true,
            whole_word: false,
        }),
        ranges: vec![],
        appearance: highlight::Appearance {
            color,
            active_color: color,
            radius: 0.,
        },
        active_index: None,
        match_index_offset: 0,
    }])
}
fn content(text: &str) -> text_content::Content {
    text_content::Content {
        text: text.into(),
        spans: vec![text_content::Span {
            start_byte: 0,
            end_byte: 3,
            foreground: 0x800080ff,
        }],
    }
}
async fn ready(cx: &mut gpui::AsyncApp, handle: WindowHandle<View>, count: i64) -> i64 {
    let expected = highlight::State::Ready(vec![highlight::Count {
        total: count,
        stored: count,
    }]);
    for _ in 0..200 {
        frame(cx, handle).await;
        let observation = handle
            .update(cx, |view, _, _| {
                view.highlights[&id(28)].borrow().observation()
            })
            .unwrap();
        if observation.state == expected {
            return observation.epoch;
        }
        cx.background_executor()
            .timer(Duration::from_millis(16))
            .await;
    }
    panic!("outer scope did not observe {count} Link text matches");
}
fn pixels(cx: &mut gpui::AsyncApp, handle: WindowHandle<View>, wash: Option<[u8; 4]>) {
    handle
        .update(cx, |view, window, _| {
            let bounds = view.probes.borrow()[&id(30)].bounds;
            let scale = window.scale_factor();
            let image = window.render_to_image().unwrap();
            let mut washes = 0;
            let mut purple = 0;
            for y in (f32::from(bounds.top()) * scale) as u32
                ..(f32::from(bounds.bottom()) * scale) as u32
            {
                for x in (f32::from(bounds.left()) * scale) as u32
                    ..(f32::from(bounds.right()) * scale) as u32
                {
                    let pixel = image.get_pixel(x, y).0;
                    if wash.is_some_and(|expected| pixel == expected) {
                        washes += 1;
                    }
                    if pixel[0] > 90
                        && pixel[0] < 170
                        && pixel[1] < 40
                        && pixel[2] > 90
                        && pixel[2] < 170
                    {
                        purple += 1;
                    }
                    if wash.is_none() {
                        assert_ne!(pixel, [255, 0, 0, 255], "retired red wash");
                        assert_ne!(pixel, [0, 255, 0, 255], "retired green wash");
                    }
                }
            }
            assert!(
                purple > 10,
                "foreground span must paint its own glyph color"
            );
            if wash.is_some() {
                assert!(washes > 20, "outer scope must paint under Link glyphs");
            }
        })
        .unwrap();
}
pub(super) async fn exercise(
    cx: &mut gpui::AsyncApp,
    handle: WindowHandle<View>,
    transport: &Transport,
) {
    events(cx, handle, transport, false, None);
    apply(
        cx,
        handle,
        vec![
            Op::Create(id(28), Kind::HighlightScope, String::new(), None),
            Op::SetHighlightScope(id(28), highlight("aaa", 0xff0000ff)),
            Op::SetStyle(
                id(28),
                vec![Style::Fields(vec![
                    Field::Width(Length::Px(300.)),
                    Field::Height(Length::Px(60.)),
                    Field::Background(Fill::Solid(Color::Rgba(0xffffffff))),
                    Field::UserSelect(true),
                ])],
            ),
            Op::Create(
                id(29),
                Kind::Link,
                String::new(),
                Some(gpuio_protocol::HandlerId::from_parts(29, 1).unwrap()),
            ),
            Op::SetLink(id(29), config(29, 0)),
            Op::SetStyle(
                id(29),
                vec![Style::Fields(vec![
                    Field::Width(Length::Px(300.)),
                    Field::Height(Length::Px(60.)),
                ])],
            ),
            Op::Create(id(30), Kind::Text, String::new(), None),
            Op::SetStyledText(id(30), content("aaa 世界")),
            Op::SetStyle(
                id(30),
                vec![Style::Fields(vec![
                    Field::Width(Length::Px(260.)),
                    Field::Height(Length::Px(36.)),
                    Field::FontSize(24.),
                    Field::LineHeight(Length::Px(32.)),
                ])],
            ),
            Op::Splice(id(29), 0, 0, vec![id(30)]),
            Op::Splice(id(28), 0, 0, vec![id(29)]),
            Op::Splice(id(0), 0, 0, vec![id(28)]),
        ],
    );
    let epoch = ready(cx, handle, 1).await;
    pixels(cx, handle, Some([255, 0, 0, 255]));
    events(cx, handle, transport, true, None);
    let (focus_handle, scope) = handle
        .update(cx, |view, _, _| {
            assert!(
                view.selections.is_empty(),
                "Link suppresses inherited selection even inside a scope"
            );
            (
                view.buttons[&id(29)].focus.clone(),
                Rc::downgrade(&view.highlights[&id(28)]),
            )
        })
        .unwrap();
    focus(cx, handle, 5);
    draw(cx, handle);
    click(cx, handle, 30);
    focused(cx, handle, 29);
    events(cx, handle, transport, false, Some(id(29)));
    key(cx, handle, "enter");
    events(cx, handle, transport, false, Some(id(29)));

    apply(
        cx,
        handle,
        vec![Op::SetHighlightScope(id(28), highlight("aaa", 0x00ff00ff))],
    );
    assert_eq!(
        ready(cx, handle, 1).await,
        epoch,
        "cosmetic highlight changes preserve matcher epoch"
    );
    pixels(cx, handle, Some([0, 255, 0, 255]));
    events(cx, handle, transport, true, None);
    apply(
        cx,
        handle,
        vec![Op::SetStyledText(id(30), content("zzz 世界"))],
    );
    assert!(ready(cx, handle, 0).await > epoch);
    pixels(cx, handle, None);
    events(cx, handle, transport, true, None);
    handle
        .update(cx, |view, _, _| {
            assert_eq!(view.buttons[&id(29)].focus, focus_handle)
        })
        .unwrap();
    focused(cx, handle, 29);
    #[cfg(target_os = "macos")]
    assert_eq!(
        accessible(cx, handle, 29, AxAction::InspectFocus),
        Some(true)
    );

    apply(
        cx,
        handle,
        vec![
            Op::Splice(id(0), 0, 1, vec![]),
            Op::Remove(id(30)),
            Op::Remove(id(29)),
            Op::Remove(id(28)),
        ],
    );
    idle(cx, handle).await;
    events(cx, handle, transport, true, None);
    assert!(
        scope.upgrade().is_none(),
        "removed Link content retains no highlight scope"
    );
    handle
        .update(cx, |view, _, _| {
            assert!(view.highlights.is_empty() && view.selections.is_empty())
        })
        .unwrap();
    eprintln!(
        "GPUIO_LINK_HIGHLIGHT_OK: span glyphs and outer GPU wash, one action/focus, inherited selection suppression, cosmetic/source updates, scope retirement and idle"
    );
}
