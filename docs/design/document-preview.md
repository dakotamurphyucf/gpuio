# Document preview — OCH-41

`Document.Config.create ?max_lines` accepts a 1..4096 body-line-height budget
for Markdown or HTML with `Layout.Flow`. Omission removes the limit. Supplying a limit
for Code, Diff or Viewport is an error. This is the native height budget:
paragraph spacing, headings, tables and embedded content consume it. It is not a
count of source lines and does not reduce parser/source admission limits.
Toolbar/collapse controls are outside the preview body. Copy-source behavior is
unchanged.

`View.document ?on_preview` and its Bonsai equivalent observe the native
presentation asynchronously. The callback requires Markdown/HTML Flow, but does not
require a limit, allowing the same mounted view to expand. `Document.Preview`
exposes these states:

- `Pending`: no body has been installed; provenance names the requested snapshot.
- `Collapsed`: the body is collapsed; provenance names the requested snapshot.
- `Source_view`: explicit source mode or rich-rendering fallback is displayed.
- `Rich { clamped }`: rich Markdown/HTML is displayed; the flag describes its latest
  painted preview.

Source-view and rich observations name the installed picture. A newer same-
generation parse can be pending without changing that picture. Applications must
not treat an older observation's source revision as the latest source. Closed
windows, retired nodes, replaced handlers, old configuration epochs and stale
source generations cannot deliver callbacks. The Eio source registry checks
provenance again before driver dispatch.

Changing the limit retains the source registration, native rich-text entity,
prepared document and parser request. Native measured-row invalidation lets an
owning managed list update its geometry. The renderer obtains the observation
after body painting, then defers delivery through the bounded native mailbox.
No synchronous callback into OCaml occurs during layout/paint. Unchanged
state/provenance/config is silent; adjacent observations for the same identity,
tree revision, configuration epoch and source generation may coalesce. Actions
and identity changes remain ordering barriers. There is no polling timer.

Append-only Op117 `Set_document_preview (node, config)` carries a positive,
strictly increasing epoch, optional bounded line count and observer flag.
Inactive configuration explicitly clears the limit/observer while retaining the
epoch fence. It does not change existing Set_document bytes. Final-tree admission
checks mode/layout compatibility and requires a handler for observations.
Event76 `Document_preview_observed` carries window/node/handler/tree revision,
source identity, configuration epoch, source revision/generation and state.
Both endpoints reject a clamped-rich observation when the current limit is absent.

The underlying clip must agree across drawing, hitboxes, keyboard focus and
accessibility; see [native prerequisites](../evidence/document-preview-prerequisites-och41.md).
This contract does not establish physical desktop acceptance. Codec, Core,
admission, presenter/queue and gallery validation must pass before this catalog
row is accepted; physical macOS qualification remains in OCH-17.
