# Plain input and text-area source review — OCH-41

Checkpoint: 2026-10-01. This is a functional review and implementation gap list,
not editor-family acceptance. The pinned input module exports single-line input,
ordinary text area and code editor from a shared engine. A root-module mapping
alone hid important differences between those three public controls.

The gallery now presents these examples in bounded groups after a real macOS
run exposed editor-memory admission failure. [Physical forms and group traversal
evidence](../evidence/gallery-editor-admission-och41.md) records the repair and
remaining acceptance limits.

## Reproducible sources

Fourteen unmodified Git blobs from GPUI Kit
`84f57fdfcb4910623fb0bb7f795b077e249f9271` are recorded with SHA-256 hashes in
[sources/manifest.json](sources/manifest.json):

- `base/input/{mod.rs,input/mod.rs,textarea/mod.rs,editor/mod.rs}` establishes
  the concrete wrappers and exports.
- `base/input/base/{mod.rs,state.rs,kind.rs,mask_pattern.rs}` establishes frame
  styles, context-menu capabilities, mode-specific methods and formatting rules.
- `component/input/{mod.rs,input.rs,textarea.rs,state.rs,clear_button.rs,content_type.rs}`
  establishes styled controls, state adaptation, adornments and native semantics.

Snapshots are documentation inputs, not extra build dependencies. They were
extracted with `git show <revision>:<path>` from the existing source checkout;
`python3 scripts/audit_component_catalog.py` verifies their hashes. Generic
history module internals and the complete code-editor/LSP implementation are not
reviewed by this checkpoint; their ownership stays explicit below.

## Behavior mapping

| Pinned behavior | GPUIO mapping and remaining work |
| --- | --- |
| Separate input, ordinary text area and code editor | `Text_input.Config` has `Single_line`/`Multiline`; Rust owns concrete Input/Textarea states. Full editable code editor is OCH-44; LSP is OCH-45. Read-only highlighted documents are a different implemented family. |
| Native editing, directed selection, IME, clipboard shortcuts, undo/redo | Existing OCH-10 adapter, UTF-8 byte selections, command leases/revisions and bounded history. Historical native evidence is linked below; latest final-revision physical IME/clipboard/VoiceOver checks remain required. |
| Default/placeholder values, disabled, read-only, submit and focus | Existing validated config and Eio controller. Creation seed is mount-only, observations never implicitly replace a draft, and explicit commands keep current native ownership. Read-only remains selectable; disabled input cannot receive ordinary focus/editing. |
| Selection and programmatic replacements | Existing `select`, `replace`, revision-conditional replacement, undo/redo and exact snapshot reads. Application code can derive selected text or compute a range replacement from a validated snapshot and submit it conditionally; there is no synchronous Rust-to-OCaml edit callback. |
| Fixed/auto-growing text-area rows | Existing min/max rows and native scrolling; fixed rows use equal limits. Size constraints and ordinary style remain part of View layout. |
| Password masking and reveal | `Text_input.Privacy.Password Hidden/Revealed` now controls retained single-line masking. Public gallery composition is authored; the integrated `View.input_frame` reveal helper is implemented locally; actual desktop acceptance remains open. See [validation](../evidence/editor-privacy-och41.md). |
| Password clipboard/accessibility behavior | Implemented policy: hidden Copy/Cut blocked; revealed copying allowed; both password modes omit AX value and use PasswordInput role. Initial TestPlatform checks pass; real macOS clipboard/AX/VoiceOver remain open. Semantic content-type hints are implemented separately and never substitute for the privacy policy. |
| Semantic content types | `Text_input.Content_hint` now covers the 45 pinned values with retained config/AX updates and a bounded macOS property adapter. Core/codec/admission, native focus and headless AppKit property checks pass locally. Exact-lease asynchronous exposure status and a public gallery example now build and pass codec/Eio/controller/native focus tests. Actual OS qualification remains open. EID/IMEI have no macOS mapping; Linux native exposure remains unavailable. A hint does not promise autofill or password-manager integration. See [partial evidence](../evidence/input-content-hints-och41.md). |
| Input-format masks | `Input_format` plus `Text_input.Config ?format`, paired Op76 and retained native pattern/number editing are implemented locally. Exact commands reject noncanonical text; interactive formatting preserves decimal precision and maps the caret through grouping. A public gallery example is authored. Native policy/composition checks are under final validation; physical IME/clipboard/AX/resource acceptance remains open. See [design](../design/input-formatting.md) and [evidence](../evidence/input-formatting-och41.md). |
| Regex and custom validation | `Input_validation.regex` and `Text_input.Config ?edit_filter` now provide bounded native regex filtering, prepared through Eio and independently compiled at atomic native admission. Formatting order, retained drafts/history, composition and exact-command rules are documented. The gallery example builds; physical input/AX/performance acceptance remains open. Synchronous OCaml predicates are deliberately not called by Rust editing; application/Eio business validation remains separate. See the [contract](../design/input-validation.md) and [evidence](../evidence/input-validation-och41.md). |
| Leading/trailing content, integrated clear, loading indicator | `View.input_frame` now retains the native editor with fixed ordinary-view slots, an undoable guarded clear, existing spinner/busy state and application-controlled reveal. Core/codec/admission and TestPlatform checks pass locally; final gallery/physical acceptance is open. See the [frame contract](../design/input-frame.md). |
| Clear on Escape | `Text_input.Config ?clear_on_escape` now supplies opt-in, native undoable clearing in both modes, with current editability/focus/filter guards. Existing composition handling consumes the first Escape separately. Public gallery and paired Op79 are implemented; Core/full native/OCaml/gallery checks pass, with 308 protocol tests and strict Rust lint passing; physical acceptance remains open. See [contract](../design/editor-escape.md) and [evidence](../evidence/editor-escape-och41.md). |
| Border/background/focus styling and Tab order | Existing View styles and focus policy provide a functional foundation. New frame/parts must respect these controls without duplicate focus owners or changing editor identity. Arbitrary accessibility-role replacement is not required to override an input's semantic role. |
| Context menus | The new opt-in `View.editor_menu` supplies Cut/Copy/Paste/Select-all with native current-state availability and an exact editor lease, including deferred-delivery checks. It uses the existing menu renderer and scoped native commands. Public gallery build, Core/codec/admission and native TestPlatform verification pass; physical/AX/visual acceptance remains open. See the [contract](../design/editor-menu.md). |
| Ordinary text-area wrapping, continuation indent, whitespace and cursor margins | `Text_area_layout` / `Text_input.Config ?layout`, paired Op78 and retained native setters are implemented locally. Core/codec, rendered native geometry/composition/scroll and admission tests pass, along with the full native library suite, full OCaml suite, gallery build and strict Rust lint. Physical visual/input acceptance remains open. See [contract](../design/textarea-layout.md) and [evidence](../evidence/textarea-layout-och41.md). |
| Ordinary text-area search/replace | Typed commands, native observations and public `Gpuio_eio.Search_bar` presentation are implemented locally, with a gallery example. Scoped Find/Replace/F3/Escape and query-only Enter shortcuts, composition guards, exact replacement stamps, per-opening close/focus and retained document identity have Core/Eio/native tests. Query drafts belong to the bar during each opening; raw source query commands do not overwrite them. Physical keyboard/focus/IME/accessibility, visual and performance acceptance remain open. See [contract](../design/textarea-search.md) and [bar evidence](../evidence/search-bar-och41.md). |
| Viewport and caret layout queries | `Editor_viewport` and Eio `Text_input.read_viewport` / `scroll_to` now expose coherent last-layout metadata and accepted/clamped scroll requests without changing the draft. Buffer-line ranges include overscan, not wrapped display-row counts. Native geometry/IME/masking/wheel tests and the full native/OCaml/Eio/gallery checks pass. All 309 protocol tests and strict Rust lint pass; physical acceptance remains open. See [contract](../design/editor-viewport.md) and [evidence](../evidence/editor-viewport-och41.md). |
| Source range geometry | `Editor_geometry` and Eio `Text_input.range_bounds ~snapshot ~range` expose revision-checked, unclipped last-paint bounds. The native helper repairs reproduced negative multiline width and guards stale source/layout. Independent codecs, Core/Eio request lifecycle, native geometry and a public gallery inspector have local evidence. Off-layout endpoints return `None`; delayed bounds never overwrite text observations. [Root/installed macOS geometry checks](../evidence/editor-accessibility-geometry-och41.md) now verify the public inspector and repaired same-frame AX range/caret bounds for wraps, CRLF, Unicode clusters and RTL. Physical IME/VoiceOver and broader editor-family acceptance remain open. See [contract](../design/editor-range-geometry.md), [native repair](../evidence/editor-range-geometry-och41.md) and [API evidence](../evidence/editor-range-api-och41.md). |
| Numeric stepping and OTP wrappers reexported from input | Owned by the `numbers` and `otp` families/OCH-34, not additional editors. Their pinned numeric/OTP review now records existing behavior and remaining presentation/policy gaps; see [review](numeric-review.md). |
| Diagnostics, folding, language providers, multicursor code editing, LSP providers/popovers | Code-editor/LSP subfamilies remain OCH-44/OCH-45 under the accepted post-v1 scope. Internal rope/display-map/history types map to owning APIs; they are not separate public widgets. |

The styled Input's context menu, adornment layout and semantic content types are
not inherited automatically by using the base editing engine. GPUIO intentionally
uses its own native adapter and must implement those integration points explicitly.

## Existing evidence and its limits

[OCH-10 native evidence](../evidence/native-editor-och10.md) records historical
real-window and NSTextInputClient/AX callback checks for the baseline adapter,
including joined-emoji deletion, directed selection, clipboard and exact-submit
behavior. It distinguishes native callback invocation from physical candidate-panel
and full screen-reader qualification. Its earlier Linux milestone references are
superseded by the current [platform policy](../platform-release-policy.md).

The current [command lifecycle checks](../evidence/command-lifecycle-och17.md)
exercise Eio fibers, request limits, stale window identities and cleanup failures
using real native allocation but injected host events. They do not establish the
new masking/formatting/adornment behavior or desktop acceptance.

## Required continuation

The [plain-input extension plan](../design/plain-input-extensions.md) separates
native edit policy, presentation and controller state before implementation.
None of the missing plain-input/text-area features above is silently marked
implemented or moved to OCH-44. Update this mapping as each contract gains public
API examples, paired codecs, native behavior and actual platform evidence.
