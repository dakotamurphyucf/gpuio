# Select and Combobox: pinned behavior review

OCH-41, 2026-10-01. This review uses the exact GPUI Kit revision
`84f57fdfcb4910623fb0bb7f795b077e249f9271`. Eleven unmodified source snapshots
cover the two base roots, two styled components and seven shared searchable-list
files. Their paths and SHA-256 values are in `sources/manifest.json`. They were
read from the archive whose SHA-256 is pinned in `third_party/sources.json`, and
that archive checksum was verified before extraction. This is source evidence,
not completion of either family.

## Original gap inventory: existing Select and Combobox

The table below records the baseline reviewed before the additive Choice_picker
work. Its “missing” entries describe those existing controls, not the current
implementation status of the new picker. See the continuation checkpoint below.

### Distinct interfaces behind similar names

The base Select and Combobox are controlled native roots. They provide focus,
keyboard/open/dismiss/confirm behavior while the caller supplies children and
selection presentation. The styled components add a searchable-list owner.
Styled Combobox is a trigger opening a popup with an optional search field; it
supports single or multiple selection. GPUIO's existing `Gpuio_eio.Combobox` is
an editable input with suggestions and exactly one selected application ID.
That is useful functionality but does not cover every styled Combobox behavior.

| Source behavior | Current public GPUIO mapping and remaining work |
| --- | --- |
| Stable selected values, disabled items and label/value separation | `Choice.Id`, `Choice.Collection`, `Choice.Config`; native requests carry IDs. Reordering does not redefine identity. Only one selected ID is supported by these controls. |
| Single-select native popup and keyboard navigation | `View.select`: retained open/highlight state, enabled-option traversal, Home/End, prefix typeahead, Escape/Tab/outside dismissal and active-descendant semantics. See `docs/design/native-controls.md` for exact behavior and limits. |
| Searchable Select with a search editor inside its popup | Missing as an integrated control. Select prefix typeahead is not an IME-capable popup editor. The existing editable Combobox has a different layout and focus contract. |
| Multiple selection, per-toggle change, final confirmation on close | Missing from the native choice controls. Upstream keeps the popup open for multiple selection and emits Change on toggles and Confirm on closure; a set of unrelated checkboxes does not by itself provide that popup contract. |
| Query editing and programmatic replacement | Existing Eio Combobox exposes native snapshots, focus, revision-checked replacement and IME fencing. Selection does not implicitly overwrite the query. These explicit ownership rules must remain. |
| Default local search | Existing Combobox Substring matches lowercase labels and preserves order, like the default searchable item matcher. Neither promises accent folding or Unicode normalization. |
| Custom matching and asynchronous search delegate | `Unfiltered` plus application-owned Eio search results is a partial equivalent. The application owns cancellation/order and must include any selected ID in the complete supplied collection. No automatic stale-result protection is implied by rendering a result list. |
| Grouped sections, group headers and rich option rows | Missing from Choice's flat text-item collection. The upstream delegate supports section counts, headers and custom rows, including icons/secondary text. Text containing a group name is not a section or an equivalent accessible structure. |
| Custom selected-title display and trigger | Current Select displays the selected text or its config label. No separate placeholder, title-prefix/icon slot or custom trigger-content contract exists. Root styling cannot supply arbitrary child content. |
| Clear affordance and clearing requests | Applications may set selection to None or add an external clear button. There is no integrated clear affordance/event on the existing choice root. Preserve disabled selections and distinguish explicit clearing from missing data. |
| Custom empty content, footer and selected-item check icon | Current `Choice.Appearance` supplies empty text and part styles, not arbitrary empty/footer/row content. These remain gaps. |
| Popup sizing and appearance | Current explicit popup width, uniform row height, visible-row limit and restricted part styles are implemented. Source uses GPUI Length and menu max height; this is functional bounded geometry, not exact constructor/value parity. Rich/group rows require a new measurement contract. |
| Controlled open state and open/dismiss/confirm callbacks | Existing choice popup state stays entirely native and is not exposed as a general observation/control API. Base root callbacks therefore remain unmapped for these controls. |
| Delegate veto/transformation and rendering hooks | No synchronous OCaml callback may run inside native list/input/render borrows. Current-model reducers and submitted passive View content are the appropriate asynchronous equivalents. Native callbacks carrying a replacement selection vector must not overwrite newer OCaml state. |

## Existing guarantees that the extension must preserve

Choice collections admit at most 4,096 unique IDs with bounded UTF-8 payloads.
Disabled items can remain selected; selecting them through native input is
rejected. Empty collections are valid. Native popup rows are virtualized with
explicit fixed-height geometry and a bounded native highlight/search cache.
The editable controller retains query/caret/composition/undo in Rust and uses
window/node/revision leases for commands. Application selection is separate.

Existing examples and tests establish those narrower contracts. They do not
prove multi-selection, grouping, custom render slots, popup-query focus or the
new API that will implement them. A native extension escape hatch is not the
completed reusable choice component required by this source review.

## Required continuation

The [choice picker extension design](../design/choice-picker.md) records an
additive implementation direction and its unresolved details. Validated Core
catalog/selection/configuration, native owner/renderer, public Core/Bonsai View,
reconciler and Eio query controller now exist. The Pickers gallery includes grouped
multiple selection, native search, rich rows, unavailable items and a clear footer.
Current local checks include native TestPlatform behavior, Core callback/model
fences, cross-language public-View transaction admission and a rebuilt gallery.
This does not declare complete catalog or desktop acceptance. Expand and validate
the gallery for single/multiple/search/group/clear/custom slots. Native checks must cover rapid
queued toggles, changed/disabled/removed values, composition, focus/Tab/Escape,
open/close ordering, scroll culling and retirement. Physical macOS and installed
consumer runtime, resource limits and required Linux checks remain acceptance
gates. These capabilities have not been deferred out of milestone 07.

### Current picker integration checkpoint — 2026-10-01

The additive picker now has actual named AX group parentage, explicit query-owner
active-descendant handling, independent keyboard Clear, clipping-driven closure,
controller sequencing/remount tests and inherited text-measurement invalidation.
TestPlatform accessibility activation exposed duplicate query/footer rendering,
which was fixed in the generic host child traversal. These are local pre-platform
checks; external VoiceOver/AX notifications and physical input/IME remain open.

The public gallery now includes grouped rich multiple selection, controlled single
selection, 4,096 searchable workspaces and an empty/create/reset workflow. The
controlled reducer separates requested opening from observed visibility and checks
current permission for queued selections. Full OCaml tests/format/gallery build
pass. `scripts/gallery_choice_picker.py` is wired to the gallery runner's
`choice-pickers` section and Python-compiles; it remains unrun. This updates local
implementation coverage, not whole-family catalog or macOS release acceptance.
