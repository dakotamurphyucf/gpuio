# Writing and maintaining component adapters

An adapter translates a public OCaml contract into native GPUI behavior. It is
not a Rust object wrapper. Applications own their domain model and asynchronous
work; Rust owns focus, editing sessions, pointer capture, layout and paint.
Events return asynchronously through the bounded bridge. Never call OCaml from
a layout, paint, input delegate or other synchronous native callback.

Start with the [engineering standards](design/engineering-standards.md),
[catalog](catalog/README.md), and the relevant family's design and evidence.
For application-specific native components, prefer the existing
[extension SDK](design/extensions.md) and its independent consumer example.
Expose disabled/read-only state on the actual accessible controls inside a package;
state on the host group does not automatically propagate to child controls.
Keep live event guards in addition to render-time accessibility metadata.
Do not add an internal bridge opcode merely to avoid using that SDK.

## Choose the smallest implementation that owns the behavior

| Requirement | Implementation | Reference |
| --- | --- | --- |
| Labels, badges, cards, form layout, descriptions | Ordinary public View composition and semantic metadata | `lib/core/presentation.ml`, `lib/core/form.ml` |
| Native editing or focus/input state | Reuse pinned GPUI Base behavior with a GPUIO native adapter | `lib/eio/text_input.mli`, `rust/native/src/editor.rs`, `docs/design/native-editor.md` |
| Controlled choice or popup | Validated immutable configuration, queued proposals, explicit confirmation/cancel state | `lib/eio/date_picker.ml`, `lib/core/date_picker.mli` |
| Large collections/documents | Existing managed viewport and prepared immutable data contracts | `docs/design/managed-lists.md`, `docs/design/documents.md` |
| New application-specific native widget | Statically linked SDK registration and public codec/controller | `examples/extension_consumer`, `examples/signal_studio` |

Read the exact pinned sources before deciding what can be reused. Longbridge's
`gpui-base` and styled `component` crate have different responsibilities. GPUIO
vendors the former against one Zed GPUI package identity. The styled facade is
not a drop-in dependency: its manifest, theme globals, assets and composition
assumptions need a compatibility review. Matching an upstream widget name does
not establish behavior, accessibility or every styling option. GPUIO style tokens
and ordinary view composition are preferable when the upstream facade adds only
presentation. Preserve extracted algorithms' source attribution, as the chart
adapter does in `rust/plot/UPSTREAM.md`.

## Write the public contract first

Define a domain module with its principal `t`, an `.mli`, and a small intended
call-site example. Keep configuration, observations, proposals and commands
distinct. Constructors validate finite numbers, ranges, text bounds and unique
identities. State units explicitly: logical pixels, UTF-8 byte offsets, civil
dates, native revisions or durations. Keep wire versions separate from public
types and validate decoded values too.

For each adapter, specify:

- Who owns the value during interaction; whether an observation changes model
  state, a proposal requires acceptance, or an explicit command changes native
  state. Initial values apply once per native session, not on every render.
- Identity and lifetime on keyed reorder, configuration change, hide, unmount,
  remount and window close. Capture the live generation for commands and reject
  stale delivery; do not resolve an old command against a replacement by name.
- Ordering and coalescing. A latest-state reducer must recheck current disabled,
  read-only and revision constraints. Terminal command/submit events cannot be
  casually merged with ordinary observations.
- Keyboard defaults, focus restoration, IME interaction, accessibility role,
  name, value, state and actions. Do not require a late OCaml callback to cancel a
  default action that Rust has already performed.
- Theme, scale, reduced motion, clipping and hidden content policy. Paint-only
  animations stay native; visibility and teardown must stop unnecessary frames.
- Limits on live resources, bytes, outstanding requests, retained rows/history
  and caches. Define overflow, cancellation and typed recovery outcomes.

Application tasks and data must not accidentally die with a virtual row. Native
leases do die when their placement unmounts. Bonsai branch model retention is a
separate choice: Component Studio retains sample values, remounts native editors
from their initial value, and clears transient chooser/toast state on departure.

## Implement and prove the boundary

Add Core types, Bonsai views and Eio controllers only where each layer is needed.
Keep first-party I/O in Eio scopes. A controller borrows its owning window; it
must not keep disposed windows alive. Contain exceptions/panics at the FFI
boundary. Pair protocol changes with independent OCaml/Rust fixtures; round trips
alone can conceal the same encoding error on both sides.

Use meaningful expect tests for model transitions, malformed input and delayed
events. Native tests should exercise the actual behavior being claimed: real OS
keyboard/clipboard/IME is distinct from GPUI-injected events or an AX action.
Check native accessibility, multiple windows, removal with pending work, and
bounded repeated lifetimes. A screenshot complements those assertions.

Add a public gallery preview and an installed-library consumer where applicable.
Record commands, checked revision, OS/architecture, display and actual outcomes
in `docs/evidence` and Linear. Update `docs/catalog/families.json` and the detailed
style/event ledger; a helper maps to its owning API rather than becoming a fake
standalone widget. Build success cannot promote a pending row to validated.
The current release gate is macOS; Linux build/unit/private-bus/consumer checks
remain required, with GUI qualification tracked separately in OCH-47.

## Reconstruct before upgrading

Normal builds use committed vendor sources and lockfiles. Reconstruction must
reproduce those inputs without editing a working switch or vendor directory.
`third_party/sources.json` records archive, patch and source hashes. The scripts
refuse an existing output rather than modifying it in place.

For GPUI Base, choose a new destination under ignored scratch:

```sh
python3 scripts/vendor_gpui_base.py --output scratch/reconstruction/gpui-base
diff -ru vendor/gpui-base scratch/reconstruction/gpui-base
```

This command needs Python 3.12+, curl and patch; it verifies both the downloaded
archive and the patch, retains the upstream manifest/license, and unifies GPUI,
macros and sum-tree with the recorded Zed revision. `--archive PATH` accepts a
local archive with the same hash validation. A nonempty diff needs investigation.
Use a fresh output path when repeating the check. A local
`vendor/gpui-base/Cargo.lock` is explicitly Git-ignored and is not a reconstructed
source input; confirm that with `git ls-files`/`git check-ignore` before classifying
that sole difference. Do not ignore other differences.

GPUI core uses the analogous `scripts/vendor_gpui.py --output` reconstruction
and optional hash-verified `--archive` input. Its test-support appearance accessor
forwards to the existing simulated platform notification; it does not implement
production appearance behavior. Keep the deferred native observer path and weak
window ownership checks when upgrading; see [appearance evidence](evidence/window-appearance-och41.md).

The read-only diff adapter also carries a bounded Base row-adornment hook. Its
gutter controls and metadata suffixes are laid out with current shaped source
rows, while source bytes, selection and highlights remain owned by one editor.
When upgrading Base, retain its row/fold/scroll mapping, clipping, slot width and
prepaint/paint ordering checks; see the [diff contract](design/diff-controls.md#native-file-headers)
and [native evidence](evidence/diff-controls-och41.md). Update the patch and its
recorded hash together, then reconstruct into a fresh directory.

Rich Markdown accessibility also adapts Base's measured inline flow. Its
frame-local collector groups painted fragments by source link identity and keeps
native custom children in reading order. Preserve custom-object measurement IDs,
actual shaped geometry, first-fragment reveal and guarded queued link actions when
upgrading; see [the contract](design/document-accessibility.md) and
[the evidence](evidence/document-accessibility-och17.md).

Reconstruct the Bonsai family into a fresh directory without changing the live
vendor tree:

```sh
python3 scripts/vendor_bonsai.py --output scratch/reconstruction/native-bonsai
```

For offline verification, add `--archive-dir PATH`, containing `<name>.tar.gz`
for every requested member of `native_bonsai` in `third_party/sources.json`.
Local archives and patches must match their recorded hashes; missing local files
fail rather than falling back to downloads. All requested inputs are verified
and patched before output copying begins. Existing destinations, including dangling
symlinks, are refused. An I/O failure while copying may leave an incomplete output;
use a new destination for a retry and never treat its existence as success.

`--package bonsai` (repeatable) explicitly limits a maintenance check to selected
members. A subset result does not establish full-family reconstruction. Compare
all reconstructed files, symlink targets and executable bits with their live vendor
counterparts; do not move/delete the live tree to run the checker. The current
[Bonsai-only evidence](evidence/bonsai-reconstruction-och17.md) records the exact
scope and remaining archive availability.

Ordinary reconstruction omits `--record-archives`; that flag records new hashes
and belongs only in a deliberate source-update review. See [development](development.md)
for opam and Cargo lock updates in the isolated environment.

Native dependency changes also affect the independent application lockfiles:
`examples/extension_consumer/backend/Cargo.lock` (used by Component Studio) and
`examples/signal_studio/backend/Cargo.lock`, as well as the root `Cargo.lock`.
They do not inherit updates to the root lockfile. Resolve each affected manifest
with the isolated toolchain, review the exact diff, and retain existing versions
unless the upgrade explicitly requires a change. Verify the corresponding
`--locked` application builds. A new dependency edge to an already locked package
can otherwise pass root native tests but break the gallery or installed consumer.

An upgrade changes exact pins, patch files, their hashes, lockfiles and source
snapshots together. Review removed patches as carefully as new ones, including
grapheme editing, document selection, lifecycle snapshots, retained action paths
and accessibility changes. Do not assume an upstream replacement preserves the
GPUIO contract. Re-run cross-language fixtures, lifecycle and native affected
families, installed consumers and both platform build suites. Check for duplicate
GPUI package identities in the resolved Cargo dependency graph.

Catalog snapshots have their own exact-Git-blob/SHA-256 manifest under
`docs/catalog/sources`. On an intentional catalog pin change, regenerate the
snapshots from that commit, update their manifest, then run
`python3 scripts/audit_component_catalog.py --write` and review all new, removed
and changed nested exports and values. That command updates the inventory only;
it does not bless family behavior or dependency compatibility.

Preserve upstream copyright and license files in all copied sources. Update
[THIRD_PARTY.md](../THIRD_PARTY.md) and distribution acknowledgements, including
syntax/theme asset notices and extracted chart sources. A project's Apache-2.0
license does not replace the licenses of its dependencies.

## Native document selection ownership

Window-wide reads use the maintained Base `TextSelection::selected_text_limited`
collector. Keep renderer callbacks responsible for plain/source normalization;
do not replace them with a second parser or truncate the final merged string.
The collector bounds copied fallback text and merged output, snapshots callback
ownership before invocation and stops after overflow. An existing callback may
allocate its whole fragment before its size is checked. Preserve this explicit
limitation when adapting third-party renderers. Never invoke synchronous OCaml
callbacks from the native collector. See [the public selection contract](design/window-selection.md)
and [regression/reconstruction evidence](evidence/window-selection-och41.md).

Document selection format updates must reach both the retained TextView and the
window selection participant's copy callback. The callback normalizes rendered
paragraph separators only for effective plain copy; Markdown source selection
must retain its own whitespace. Testing `TextViewState.selected_text` alone misses
the participant adapter. The [selection-format regression](evidence/document-selection-format-och41.md)
exercises the real window copy provider and records exact fork reconstruction.

Read-only document bodies inherit the optional `User_select` declaration and
otherwise default to selectable. Editable inputs retain their widget-owned
selection. Do not implement read-only selection policy by disabling all pointer
input: links, scrolling and explicit copy controls remain independent.

The host mounts one Base `TextSelectionLayer` first in each native window and
assigns documents stable selection scopes from retained focus traps. Activate the
current trap scope before rendering; a fresh per-frame scope would clear selection
on every frame. Inactive scopes must be excluded from local-selection Copy as well
as geometric snapshots. The [selection audit](design/selection-style-audit.md)
records the narrow Base patch, tested behavior and remaining acceptance work.

Ordinary selectable text uses the same window layer and scope identities. Register
once per paint with `register_in_paint_order`; callbacks must not strongly retain
the node. Local keyboard range changes clear prior window geometry before updating
the participant, outside the node's mutable borrow. Refresh local caret anchors
after layout for Shift-click. Copy preserves nonempty whitespace payloads. When
source and displayed text differ, project displayed glyphs and map their ranges
back to source bytes; never treat ellipsis byte offsets as source offsets. Reuse
`text_projection` for this mapping and highlight painting. Retiring a registered
endpoint must retire the window gesture, not just its current painted range.

Retain ordinary `TextSelectionRun` values through `update` and set the painted
text alignment with `with_text_align`. Projection caches immutable shaped glyph
cells lazily; replacing runs every frame discards that reuse. Logical scalar caret
positions are not glyph widths, especially for joined graphemes or reordered bidi
glyphs. Keep source/display mapping separate from shaped visual geometry. Local
caret/word hits and Shift-click anchors use `caret_for_position`,
`index_for_position` and `position_for_index` on the retained run. These use
resolved bidi direction and downstream cluster affinity; do not substitute the raw
GPUI layout accessors, which omit alignment in this pinned version. Read-only
arrow movement remains logical grapheme order. See the audit for tested cases and
remaining typography acceptance.

Renderer-specific Copy normalization belongs in that participant's callback;
never trim the combined window result in a focused widget's Copy handler.
Markdown Select All clears shared geometry before borrowing its native state,
then invokes the local selection listener. Clear callbacks can revisit that same
entity, so clearing from inside the listener's mutable borrow is unsafe.

Document preview clipping needs separate visual, pointer, keyboard and accessibility
handling. A paint mask alone does not rewrite prepainted hitboxes or disable
focused child controls. The pinned fork now offers scoped hitbox refinement,
interactive-Div input clipping with focus recovery, and finalized accessibility
subtree clipping. These underpin the public [document preview API](design/document-preview.md). Custom extension elements registering input directly must enforce
their own visibility contract. See [the regression evidence](evidence/document-preview-prerequisites-och41.md).

The [HTML reader adapter](design/document-html.md) validates DOM structure before
recursive conversion and replaces all URL-backed image nodes before layout.
PreparedText retains its format so selection/source behavior cannot accidentally
use Markdown reconstruction for HTML. Reader format is part of worker identity;
renderer-only asset updates preserve the prepared source. See [local evidence](evidence/document-html-och41.md).

Internal [document styling](design/document-styling.md) caches the theme-resolved
configuration independently of the parser request. Do not route paint or metrics
changes through source replacement. Effective metric updates invalidate both the
TextView virtual row measurements and its owning managed-list row while retaining
logical selection. The [native regression](evidence/document-styling-och41.md)
checks actual painted parts and scroll extent restoration, not only outer bounds.

[Markdown parser settings](design/document-markdown-options.md) are part of worker
request equality. Retain the installed extension configuration while new work is
pending, then install matching preparation and renderers together. Source revision
alone cannot fence an option-only reparse: old link closures also capture the
installed interpretation's identity. Renderer-only image updates use installed
options so they do not accidentally restart parsing on the UI thread.

[Frontmatter descriptions](design/document-frontmatter.md) use prepared native
label/value Paragraphs so selection, displayed-text fragments, reflow and semantic
clipping describe the actual rendered text. Unsupported restricted-YAML input
falls back to code. This built-in renderer does not establish arbitrary document
plugin ownership or turn opaque plugin content into searchable/selectable text.

[Document actions](design/document-actions.md) capture an installed interpretation,
source revision/generation and configuration epoch. Revalidate them against the
live View before enqueueing; retain immutable snapshots rather than AST handles.
Native Copy is independent of custom-event payload limits. Action geometry changes
invalidate both TextView measurements and the enclosing managed-list row. Recognize
focus inside the text owner: exact focus equality makes the host's fallback steal
focus from nested controls. Painted child tab stops supplement logical link order;
arbitrary static renderer ownership and offscreen traversal still need qualification.


Declared document-plugin block Text now participates in ordinary glyph selection.
The maintained Base adaptation gives each parsed occurrence retained selection
state and connects it to the AST copy/clear traversal; partial Markdown copy
falls back to selected display text when no source-character mapping exists.
See [selection regression and reconstruction evidence](evidence/document-profile-selection-och41.md).

The same reconstruction tool supports `--crate gpui_macos`. Its separate patch
breaks a native view/accessibility-adapter retention cycle during window close;
see [GPUI macOS retirement](design/gpui-macos-adaptation.md). Keep the root,
generated backends and source-tree dependencies consistent when changing it.
