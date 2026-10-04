use gpui::IntoElement;
use gpui_base::text::MarkdownExtensions;
use gpuio_document_sdk::*;
use std::sync::{
    Arc,
    atomic::{AtomicBool, AtomicUsize, Ordering},
};

#[derive(Clone, Copy)]
enum Behavior {
    Text,
    NonText,
    Opaque,
    WrongName,
    Panic,
    Oversized,
    PresentationFailure,
    LargeNonText,
}
struct Tag {
    name: &'static str,
    block: bool,
    behavior: Behavior,
    parses: Arc<AtomicUsize>,
}
impl Plugin for Tag {
    fn name(&self) -> &'static str {
        self.name
    }
    fn is_block(&self) -> bool {
        self.block
    }
    fn parse(
        &self,
        node: &markdown_ast::Node,
        cx: &MarkdownParseContext<'_>,
        work: &PrepareContext<'_>,
    ) -> Result<Option<MarkdownNode>, Error> {
        work.check()?;
        let accepted = if self.block {
            matches!(node, markdown_ast::Node::Code(_))
        } else {
            matches!(node, markdown_ast::Node::InlineCode(_))
        };
        if !accepted {
            return Ok(None);
        }
        self.parses.fetch_add(1, Ordering::SeqCst);
        if matches!(self.behavior, Behavior::Panic) {
            panic!("parser failed");
        }
        let name = if matches!(self.behavior, Behavior::WrongName) {
            "other.node"
        } else {
            self.name
        };
        let text = if matches!(self.behavior, Behavior::Oversized) {
            "x".repeat(MAX_CODE_BYTES + 1)
        } else if matches!(self.behavior, Behavior::LargeNonText) {
            "x".repeat(60_000)
        } else {
            "世界 tag".into()
        };
        Ok(Some(
            MarkdownNode::new(name, ())
                .text(text)
                .markdown(cx.node_source(node).unwrap_or("").to_owned()),
        ))
    }
    fn presentation(&self, node: &MarkdownNode) -> Result<MarkdownPresentation, Error> {
        match self.behavior {
            Behavior::NonText | Behavior::LargeNonText => Ok(MarkdownPresentation::NonText),
            Behavior::Opaque => Ok(MarkdownPresentation::Opaque),
            Behavior::PresentationFailure => Err(Error::Parse),
            _ => Ok(MarkdownPresentation::Text(node.as_text().to_owned().into())),
        }
    }
    fn render(
        &self,
        _: &MarkdownNode,
        _: &RenderContext,
        _: &mut gpui::Window,
        _: &mut gpui::App,
    ) -> Result<gpui::AnyElement, Error> {
        Ok(gpui::div().into_any_element())
    }
    fn render_inline(
        &self,
        _: &MarkdownNode,
        _: &InlineRenderContext,
        _: &RenderContext,
        _: &mut gpui::Window,
        _: &mut gpui::App,
    ) -> Result<Option<InlineElement>, Error> {
        Ok(Some(InlineElement::new(gpui::div())))
    }
}
fn tag(block: bool, behavior: Behavior) -> Arc<dyn Plugin> {
    Arc::new(Tag {
        name: if block {
            "example.block"
        } else {
            "example.inline"
        },
        block,
        behavior,
        parses: Arc::new(AtomicUsize::new(0)),
    })
}
fn prepare(profile: Profile, source: &str) -> Result<Prepared, Error> {
    prepare_markdown(
        Arc::new(profile),
        Preparation {
            source,
            base: MarkdownExtensions::default(),
            parser_epoch: 1,
            dark: false,
            cancelled: Arc::new(AtomicBool::new(false)),
        },
    )
    .map_err(|failure| failure.error)
}
#[test]
fn block_and_inline_plugins_keep_real_source_and_truthful_displayed_text() {
    let source = "Before `tag` after.\n\n```tag\nblock\n```\n";
    let profile = Profile::new()
        .with_plugin(tag(true, Behavior::Text))
        .unwrap()
        .with_plugin(tag(false, Behavior::Text))
        .unwrap();
    let prepared = prepare(profile, source).unwrap();
    assert_eq!(prepared.document.source().as_ref(), source);
    assert_eq!(prepared.parser_epoch, 1);
    let displayed = prepared.document.displayed_text();
    assert_eq!(displayed.opaque_nodes(), 0);
    let strings = displayed
        .fragments()
        .iter()
        .map(|f| f.text().as_ref())
        .collect::<Vec<_>>();
    assert_eq!(strings, vec!["Before ", "世界 tag", " after.", "世界 tag"]);
    assert!(prepared.highlights.is_empty());
    assert_eq!(
        prepared.document.plain_text().matches("世界 tag").count(),
        2
    );
}
#[test]
fn nontext_and_opaque_plugins_do_not_invent_searchable_glyphs() {
    for (behavior, opaque) in [(Behavior::NonText, 0), (Behavior::Opaque, 1)] {
        let prepared = prepare(
            Profile::new().with_plugin(tag(true, behavior)).unwrap(),
            "```tag\nblock\n```\n",
        )
        .unwrap();
        let displayed = prepared.document.displayed_text();
        assert_eq!(displayed.opaque_nodes(), opaque);
        assert_eq!(displayed.text_bytes(), 0);
    }
}
#[test]
fn invalid_plugin_results_fail_preparation_instead_of_silently_declining() {
    for (behavior, expected) in [
        (Behavior::WrongName, Error::InvalidPlugin),
        (Behavior::Panic, Error::Panicked),
        (Behavior::Oversized, Error::LimitExceeded),
        (Behavior::PresentationFailure, Error::Parse),
    ] {
        let result = prepare(
            Profile::new().with_plugin(tag(true, behavior)).unwrap(),
            "```tag\nblock\n```\n",
        );
        assert!(matches!(result, Err(error) if error == expected));
    }
}
#[test]
fn registry_plugin_names_are_explicit_and_cannot_collide_with_internal_adapters() {
    for name in ["unqualified", "gpuio.private", "example.Bad"] {
        let plugin = Arc::new(Tag {
            name,
            block: true,
            behavior: Behavior::Text,
            parses: Arc::default(),
        });
        assert!(matches!(
            Profile::new().with_plugin(plugin),
            Err(Error::InvalidPlugin)
        ));
    }
    let profile = Profile::new()
        .with_plugin(tag(true, Behavior::Text))
        .unwrap();
    assert_eq!(profile.plugins()[0].name(), "example.block");
    assert!(profile.plugins()[0].is_block());
    assert!(matches!(
        profile.with_plugin(tag(true, Behavior::Text)),
        Err(Error::DuplicatePlugin)
    ));
}
struct Color;
impl Highlighter for Color {
    fn highlight(&self, code: &Code<'_>, cx: &PrepareContext<'_>) -> Result<Vec<Highlight>, Error> {
        cx.check()?;
        assert_eq!(code.language(), Some("tag"));
        Ok(vec![Highlight::new(0..code.text().len(), [100, 150, 200])])
    }
}
#[test]
fn fenced_code_highlights_are_prepared_with_the_same_document() {
    let source = "```tag\n世界\n```\n";
    let prepared = std::thread::spawn(move || {
        prepare(Profile::new().with_highlighter(Arc::new(Color)), source)
    })
    .join()
    .unwrap()
    .unwrap();
    assert_eq!(prepared.document.source().as_ref(), source);
    assert_eq!(prepared.highlights.len(), 1);
    assert_eq!(
        prepared.highlights[&("tag".into(), "世界".into())],
        vec![Highlight::new(0..6, [100, 150, 200])]
    );
}
struct Many;
impl Highlighter for Many {
    fn highlight(&self, code: &Code<'_>, _: &PrepareContext<'_>) -> Result<Vec<Highlight>, Error> {
        Ok((0..code.text().len())
            .map(|i| Highlight::new(i..i + 1, [0; 3]))
            .collect())
    }
}
#[test]
fn aggregate_highlights_and_cancelled_work_cannot_escape_single_document_limits() {
    let line = "x".repeat(MAX_RUNS / 4);
    let text = format!("{line}\n{line}");
    let source = format!("```tag\n{text}\n```\n\n```tag\n{text}\n```\n");
    assert!(source.len() < MAX_CODE_BYTES);
    assert_eq!(
        prepare(Profile::new().with_highlighter(Arc::new(Many)), &source).err(),
        Some(Error::LimitExceeded)
    );
    let parses = Arc::new(AtomicUsize::new(0));
    let profile = Profile::new()
        .with_plugin(Arc::new(Tag {
            name: "example.tag",
            block: true,
            behavior: Behavior::Text,
            parses: parses.clone(),
        }))
        .unwrap();
    assert!(matches!(
        prepare_markdown(
            Arc::new(profile),
            Preparation {
                source: "```tag\nx\n```",
                base: MarkdownExtensions::default(),
                parser_epoch: 1,
                dark: false,
                cancelled: Arc::new(AtomicBool::new(true))
            }
        ),
        Err(PreparationFailure {
            stage: PreparationStage::Parse,
            error: Error::Cancelled
        })
    ));
    assert_eq!(parses.load(Ordering::SeqCst), 0);
}

#[test]
fn html_keeps_its_parser_and_requires_host_owned_image_replacement() {
    let parses = Arc::new(AtomicUsize::new(0));
    let profile = Profile::new()
        .with_highlighter(Arc::new(Color))
        .with_plugin(Arc::new(Tag {
            name: "example.block",
            block: true,
            behavior: Behavior::Text,
            parses: parses.clone(),
        }))
        .unwrap();
    let source = "<p>Reader</p><pre><code class=\"language-tag\">世界</code></pre><img src=\"https://invalid.test/image.png\" alt=\"remote image\">";
    let images = Arc::new(AtomicUsize::new(0));
    let count = images.clone();
    let prepared = prepare_html(
        Arc::new(profile),
        Preparation {
            source,
            base: MarkdownExtensions::default(),
            parser_epoch: 9,
            dark: false,
            cancelled: Arc::new(AtomicBool::new(false)),
        },
        move |image| {
            count.fetch_add(1, Ordering::SeqCst);
            assert_eq!(image.url.as_ref(), "https://invalid.test/image.png");
            MarkdownNode::new("example.safe_image", ()).text("Image placeholder")
        },
    )
    .unwrap();
    assert_eq!(prepared.document.source().as_ref(), source);
    assert_eq!(parses.load(Ordering::SeqCst), 0);
    assert_eq!(images.load(Ordering::SeqCst), 1);
    assert_eq!(
        prepared.highlights[&("tag".into(), "世界".into())],
        vec![Highlight::new(0..6, [100, 150, 200])]
    );
    assert_eq!(prepared.document.displayed_text().opaque_nodes(), 1);
}

#[test]
fn generated_copy_and_accessibility_strings_are_bounded_even_without_glyphs() {
    let source = "```tag\nx\n```\n\n";
    let profile = Profile::new()
        .with_plugin(tag(true, Behavior::LargeNonText))
        .unwrap();
    assert!(prepare(profile.clone(), &source.repeat(8)).is_ok());
    assert_eq!(
        prepare(profile, &source.repeat(9)).err(),
        Some(Error::LimitExceeded)
    );
}

#[test]
fn plugin_count_boundary_prevents_unbounded_parser_chains() {
    const NAMES: &[&str] = [
        "example.p0",
        "example.p1",
        "example.p2",
        "example.p3",
        "example.p4",
        "example.p5",
        "example.p6",
        "example.p7",
        "example.p8",
        "example.p9",
        "example.p10",
        "example.p11",
        "example.p12",
        "example.p13",
        "example.p14",
        "example.p15",
        "example.p16",
        "example.p17",
        "example.p18",
        "example.p19",
        "example.p20",
        "example.p21",
        "example.p22",
        "example.p23",
        "example.p24",
        "example.p25",
        "example.p26",
        "example.p27",
        "example.p28",
        "example.p29",
        "example.p30",
        "example.p31",
        "example.p32",
    ]
    .as_slice();
    let mut profile = Profile::new();
    for &name in &NAMES[..MAX_PLUGINS] {
        profile = profile
            .with_plugin(Arc::new(Tag {
                name,
                block: true,
                behavior: Behavior::Text,
                parses: Arc::default(),
            }))
            .unwrap();
    }
    assert_eq!(profile.plugins().len(), MAX_PLUGINS);
    assert!(matches!(
        profile.with_plugin(Arc::new(Tag {
            name: NAMES[MAX_PLUGINS],
            block: true,
            behavior: Behavior::Text,
            parses: Arc::default()
        })),
        Err(Error::LimitExceeded)
    ));
}
