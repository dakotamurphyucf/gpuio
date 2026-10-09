# How `ScrollCard` retains a bounded native viewport

[Package README](../../README.md) · [Source](scroll_card.rs) · [Factory](lib.md)

This Rust plugin is installed by the profile factory. It is author-side native
code with no Bonsai graph, OCaml scroll model, new wire codec, or file I/O.
It recognizes fenced Markdown `review-scroll` blocks and replaces them with a
NonText native checklist rather than interpreting their body as executable code.

`parse` matches a Code AST node and exact language. It combines parse context
offset with node start offset, stores that occurrence as immutable `usize` data,
and preserves original Markdown. `render` checks that typed plugin data exists
and returns `ScrollCardElement` with a cloned RenderContext. `render_inline`
returns None because this is a block plugin.

`ScrollCardElement::render` runs in the document element namespace. It constructs
key `review-scroll-<source generation>-<occurrence>`, then calls
`window.use_keyed_state` to retain a `gpui::ScrollHandle`. Consecutive mounted frames
and property changes reuse that handle; source generation/reset or removal has
different identity. This transient native state is not persistent application
state across unmount/eviction. The element stores no Window/App or worker handle.

The outer column contains ordinary start/end buttons outside a 150-logical-pixel
viewport. The viewport has accessible Group semantics, `overflow_y_scroll`, and
`track_scroll`; fixed/minimum-height checklist content exceeds that extent.
Native wheel/layout owns scrolling. `scroll_button` clones the handle, sets offset
zero or scrolls to bottom, and calls `window.refresh`. Mouse/touch uses
`guard_pointer`, keyboard/AX uses `guard` from the current document event lease.
Local scrolling emits no application event.

Click “Show review end”: native delivery checks the current lease, changes native
scroll offset, and refreshes the window. The inner “Open scroll review” button
uses the factory's button helper to emit byte 4, decoded by the OCaml package as
Open_card. Thus the user can reveal clipped inner content through keyboard
alternatives without adding an OCaml per-scroll event loop. Outer document reveal
still owns only its own viewport; it cannot promise revealing arbitrary inner
plugin controls. After “Show review start”, Tab from “Show review end” skips
“Open scroll review” because the latter lies outside the inner viewport. This
also applies when the enclosing document uses `Layout.Flow`: the reader scopes
child painting with `with_clipped_input`, and native Divs omit fully clipped
controls from the eligible tab order. After a reveal paints the button, Tab can
reach it again. The plugin owns the offset; this focus filter does not scroll it.
NonText content is not ordinary selectable reader text.

From the repository root using [development setup](../../../../docs/development.md):

```sh
GPUIO_JOBS=2 ./scripts/gpuio exec cargo test --offline --locked -j2 -p gpuio-example-document
GPUIO_JOBS=2 ./scripts/gpuio build examples/gallery/main.exe
python3 scripts/test_macos_document_profile.py --output scratch/profile-native-review
```

The crate tests verify preparation installs this plugin, not native scrolling.
The macOS harness needs the built gallery, graphical session, and automation
permissions described in the README/evidence; it owns its foreground child.
[Scroll evidence](../../../../docs/evidence/document-profile-scroll-och41.md)
qualifies wheel, keyboard, clipping, property changes, and removal. Arbitrary
plugin composites, trackpad momentum, and VoiceOver are separate scope.

For another retained plugin, use an occurrence/source identity in the document
namespace, bound geometry, offer ordinary accessible alternatives, and guard local
mutation as well as emitted events. Never use parsed mutable data as a shared
UI state store or claim compilation validates physical scrolling.
