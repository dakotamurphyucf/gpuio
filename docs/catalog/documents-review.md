# Display-document source review — OCH-41

Checkpoint: 2026-10-03. This is a source/API audit, not document-family acceptance.
Read-only rich text and document customization are distinct from the explicitly
deferred editable code-editor/LSP work in OCH-44/OCH-45.

## Reproducible inputs

Nine unmodified sources at GPUI Kit
`84f57fdfcb4910623fb0bb7f795b077e249f9271` are retained in
[sources/manifest.json](sources/manifest.json): Base text module, TextView,
TextViewStyle, MarkdownExtensions and format module; component text module,
compatibility facade, style facade and FrontmatterPlugin. They were read from
the source archive whose SHA-256 is pinned by `third_party/sources.json`:
`909c00c97bbfce11607eef7d3eccd01201d591af9d26cd9ebadfe0c90fca502f`.
The structural catalog audit verifies every retained snapshot hash. The adapted
vendor tree is inspected separately; its GPUIO extensions are not attributed to
the original upstream API.

## Functional mapping

| Pinned surface | Current public GPUIO mapping / remaining work |
| --- | --- |
| `Text::String`, `Text::TextView` | Ordinary `View.text` and registered `View.document`; separate domain types provide the functional distinction without mirroring the Rust sum type. |
| `TextView::markdown`, state-based construction and updates | `Document.Mode.Markdown`, `Text_source` and `Gpuio_eio.Document`. Full revisioned snapshots, asynchronous parsing and stale-result rejection are implemented. The deliberate full-snapshot parser avoids upstream append-fragment interpretation differences. |
| `TextView::html` / `html` | `Document.Mode.Html` now provides a bounded native reader with explicit registered images, queued links, plain selection copy, original source retention and a gallery example. Parser/worker/keyboard/copy/preview tests pass locally; physical acceptance remains open. HTML embedded in Markdown remains literal. See [HTML evidence](../evidence/document-html-och41.md). |
| GFM content, code fences and table rendering | Implemented through prepared native Markdown. Fixed Copy code/Copy table actions are built by `document_view.rs`. Sources, streaming, images, code and diff have gallery examples; current-revision physical acceptance is still open. |
| `selectable` and native selection | Inherited `Style.User_select` controls participation; native retained selection and window selection layers exist. Scoped native policy, mixed-selection, reset and copy tests provide local evidence. Actual VoiceOver/keyboard/clipboard qualification remains separate. |
| `SelectionFormat` | `Document.Selection_format.Plain_text/Markdown` and optional config now expose native selected-content copy behavior through Op116. Format changes preserve native selection/source identity; dedicated whole-source/code/table Copy remains independent. Core and native TestPlatform checks plus the gallery build pass; physical clipboard/keyboard qualification remains open. See [evidence](../evidence/document-selection-format-och41.md). |
| `scrollable`, source/view lifetime | `Document.Layout.Flow` or `Viewport height`, canonical Eio source registration and retained native view state supply the functional equivalent. GPUIO also imposes documented worker/source/fallback bounds. |
| `max_lines`, state `is_clamped` | Public `Document.Config.max_lines` and `View.document ~on_preview` expose a validated Markdown/HTML Flow preview, with queued installed-source provenance and explicit pending/collapsed/source-view/rich states. Op117 changes preserve source/parser/native identity. Gallery expansion and Core/codec/admission/native resize/queue checks are implemented; physical qualification remains open. See [evidence](../evidence/document-preview-api-och41.md). |
| Link callbacks | `Document.Navigation.Link { url; activation }` and queued `View.document ~on_navigate` expose mouse button/release modifiers and Keyboard/Touch source, with source-generation/live-view guards. Legacy URL-only events have unknown metadata; native synthetic accessibility uses Keyboard. No ambient URL opening. Production Markdown/HTML TestPlatform routing and paired codec/Core checks pass; physical qualification stays open. See [evidence](../evidence/document-link-activation-och41.md). |
| Foreground/muted/link/selection/code/background/border palette | `Document.Style` exposes six independently theme-resolved parts. Explicit Selection overrides inherited View selection color for rich content. Theme-only and clear updates preserve prepared text. See [styling evidence](../evidence/document-styling-och41.md). |
| Paragraph spacing, heading size callback, code/inline-code/table/header/cell refinements | `Document.Style` supplies bounded paragraph/heading metrics, a six-level heading map, inline-code highlights and passive code/table/header/cell refinements. Base-only styles deliberately exclude visibility, placement, clipping, fixed height and independent interaction/scroll ownership; arbitrary native style callbacks are not supported. The gallery toggle and production TestPlatform tests cover painting, selection retention and virtualized remeasurement. See [contract](../design/document-styling.md). |
| Custom code-block and table action renderers | `Document.Actions` supplies bounded declarative code/table buttons, independent native Copy visibility and queued immutable snapshots through `View.document ~on_action`. Native mouse/keyboard/AX, configuration/source fencing and layout checks pass locally. Static profiles now install native code/table renderers with queued stamped events; [renderer tests](../evidence/document-profile-renderers-och41.md) exercise real host paths. A [public profile package/gallery](../evidence/document-profile-package-och41.md) passes installed consumer, typed codec and catalog checks. Offscreen code/table/profile controls have [production-host focus tests](../evidence/document-virtual-focus-och41.md); broader stress and physical qualification remain open. See [action contract](../design/document-actions.md). |
| Fenced-code highlighter / global TextView defaults | Built-in Syntect/Two Face language and appearance selection are functional defaults. Static profile highlighters now run on workers and install complete checked styles in the reader. [Application-owned defaults](../evidence/document-defaults-och41.md) now resolve inherited/built-in/explicit settings and typed profile callbacks through the runtime, with Core/window/installed-consumer evidence. Physical qualification remains open. No synchronous OCaml parse/paint callbacks. |
| `MarkdownExtensions.frontmatter`, component `FrontmatterPlugin` | Public `Document.Markdown_options.Frontmatter.Disabled/Code_block/Description_list` selects parser detection and either code or native restricted metadata rows. Unsupported YAML and more than 128 entries retain code fallback. Label/value selection, displayed-text search paint, reflow, semantics, clipping and source/native retention have local evidence; physical qualification remains open. See [frontmatter evidence](../evidence/document-frontmatter-och41.md). |
| `MarkdownExtensions.mdx` / `markdown_mdx` | `Document.Markdown_options.create ~mdx:true` enables JSX/expression AST constructs and disables raw HTML parsing without JavaScript evaluation. Unclaimed JSX preserves inline/block children, expressions stay text/code, and original source remains available. A pinned-conversion child-loss regression is fixed; worker/presenter/codec/admission/gallery checks pass locally. Static custom component registration and physical qualification remain separate. See [contract](../design/document-markdown-options.md). |
| Block parser/renderer registration, `MarkdownPlugin` and inline elements | **Static integration implemented; acceptance incomplete.** The [document SDK foundation](../evidence/document-sdk-foundation-och41.md) provides checked registry/preparation/plugin contracts. The [profile bridge](../evidence/document-profile-bridge-och41.md) adds tested Core/Bonsai attachment, native admission/catalogs and backend composition, and the [worker path](../evidence/document-profile-workers-och41.md) now has checked preparation/reservations/cancellation. Native view installation and code/table/inline/block rendering now have [production TestPlatform evidence](../evidence/document-profile-renderers-och41.md), including lifecycle/resource checks. The [public package/gallery](../evidence/document-profile-package-och41.md) now passes independent installed build/codec/catalog checks; virtual controls also have [production-host focus tests](../evidence/document-virtual-focus-och41.md); mounted Text/NonText/Opaque search-paint, select-all copy and AX-role checks now have [local evidence](../evidence/document-profile-semantics-och41.md); declared Text pointer selection and traversal across 96 passive plugin candidates now have [local repair/stress evidence](../evidence/document-profile-selection-och41.md); independent plugin scroll clipping, focus exit and wheel reveal now have [local qualification](../evidence/document-profile-scroll-och41.md). Automatic inner keyboard reveal remains plugin-owned; physical qualification remains unfinished. GPUIO uses internal plugins for images and literal HTML. The general component extension SDK does not by itself provide access to a document's parser, inline baseline, displayed-text/selection or invalidation lifetime. Dynamic plugins remain excluded. |
| Generic `TextViewPlugin`, compatibility wrappers and node/context types | Generic plugin application configures the owning TextView; it is not another widget. AST nodes, source spans, inline layout context and TableData belong to the document extension contract. Component wrappers adapt styles/highlighting; their presence upstream does not expose them through the OCaml bridge. |

## Existing evidence, without broader claims

Public API inspected: `lib/core/document.mli`, `lib/eio/document.mli`, the
`View.document` constructors, gallery Documents page and highlighting page.
Native integration inspected: `document_jobs.rs`, `document_markdown.rs` and
`document_view.rs`, including the concrete renderer chain and fixed style palette.

Existing tests include `document_selection_policy_test.rs`,
`document_selection_style_test.rs`, `document_mixed_selection_test.rs`,
`document_multi_selection_test.rs`, `document_copy_control_test.rs`,
`document_reset_test.rs`, `document_table_appearance_test.rs` and the diff tests.
These are entry points for affected behavior, not evidence for absent public
features. [Document design](../design/documents.md),
[subtree-highlighting evidence](../evidence/subtree-highlighting-och41.md) and
[M4 evidence](../evidence/agent-workspace-m4.md) describe existing contracts and
their measured limits. No native or physical tests were rerun for this source-only
checkpoint; no runtime behavior changed.

## Required continuation

Keep these gaps in OCH-41's required catalog review/implementation and OCH-17's
release acceptance. Do not mark the document family complete because Markdown
can already render or because editor/LSP is deferred. Implement coherent public
configurations, paired admission/codecs, retention/stale-event/resource behavior
and public gallery examples before updating each row to implemented. Any proposed
scope exclusion needs an explicit owner decision; this audit introduces none.

Selection format, preview clamping, the HTML reader, internal styling, link
metadata and Markdown parser switches now have public implementations. Continue
with remaining interaction and physical qualification of the implemented static
action renderers/highlighting/defaults and document registration. Mounted
Text/NonText/Opaque semantics have [local evidence](../evidence/document-profile-semantics-och41.md).
Frontmatter descriptions now have a public built-in renderer. Future parser-affecting options must join worker
identity. Renderer-only updates should retain canonical source and valid selections.
Custom content must declare its displayed text, copy/search/AX semantics and
lifetime; an opaque native element cannot be assumed to participate correctly in
document search and selection. Exact bridge designs belong beside the existing
document contract before coding.
