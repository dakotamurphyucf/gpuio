# Pinned collections review — OCH-41

Reviewed 2026-10-03 against gpui-kit
`84f57fdfcb4910623fb0bb7f795b077e249f9271`. Exact Base list/scroll/tree/table and
component list/message-scroller/scroll/tree/table snapshots are recorded in
[the source manifest](sources/manifest.json). They are unmodified Git objects,
not the adapted files compiled by GPUIO. The structural audit verifies hashes;
it does not establish functional parity or physical platform acceptance.

The existing OCH-13/38/39 implementations supply the main managed collection
contracts. This review finds additional public capabilities that need explicit
implementation or an exercised composition before catalog acceptance. It does
not reopen completed foundational work or require Rust API spelling parity.
All remaining catalog work belongs to OCH-41; editing/docking remains OCH-42.

## Managed lists and scrolling

| Pinned surface | GPUIO mapping and limitation |
| --- | --- |
| `v_virtual_list`, row sizes, sizing policy and measurement-index helpers | `Virtual_list.Config`, `View.virtual_list` and `Gpuio_bonsai.Virtual_list` retain fixed/estimated vertical rows. Native measurement replaces estimates; a bounded viewport and explicit active-row budget replace arbitrary synchronous delegate measurement. No OCaml callback runs from layout. The component virtual-list module is a Base reexport, not a second widget. |
| `h_virtual_list` | `Virtual_list.Config.horizontal ~width`, `View.virtual_list` and the managed Bonsai APIs now provide actual horizontal virtualization with bounded demand, fixed/estimated widths, anchors, paging and tail state. Reactive `component_with_config`/`paged_with_config` preserve surviving row models when axes change and wait for fresh geometry before paging. Collections → Horizontal cards exercises 10k unequal-width items, growth/reorder, commands and shared scrollbars. See [integration evidence](../evidence/horizontal-list-integration-och41.md) and the [physical macOS follow-up](../evidence/horizontal-list-macos-och41.md): repository/installed pointer, Space, both-axis wheel, scaled control containment, surviving row state and edit anchors pass. Hardware trackpads, VoiceOver and frame/resource qualification remain separate. |
| Stable list handle, item/end scrolling and remeasurement | Keyed `List_collection` edits and generation-bound `Virtual_list.Controller` commands; native main-axis measurement invalidation follows changed values, and prepend/reorder preserves surviving anchors. Explicit source generation resets discard transient row models. Upstream mutable handles never cross the bridge. |
| `ListState` search, selection, confirm/cancel, sections and async delegates | Choice picker, select/combobox and command palette already cover their own bounded popup lists. For an arbitrary persistent collection, Eio search plus `Virtual_list` and ordinary row/section Views provide data and rendering pieces, **a standalone selectable/searchable-list composition**, whose public gallery now has scoped physical keyboard evidence linked below. Full disabled-item/pointer/loading, VoiceOver and resource acceptance remain explicit; a single path does not certify every combination. A popup picker alone is not the standalone list. The [identity/selection foundation](../evidence/selectable-list-model-och41.md) and [bridge/semantic foundation](../evidence/selectable-list-bridge-och41.md) are implemented; [production native input/AX handlers](../evidence/selectable-list-native-input-och41.md) now pass local Host and AppKit checks. [Public Core callbacks and query references](../evidence/selectable-list-core-input-och41.md) now pass paired/native/driver checks. [Managed selection/controller lifetimes, visible projection and themed bounded rows](../evidence/selectable-list-managed-och41.md) now pass Bonsai lifecycle tests. [Eio search and the public Searchable list gallery](../evidence/selectable-list-search-och41.md) have local scheduler/component tests and compile evidence. The [physical macOS matrix](../evidence/selectable-list-macos-och41.md) now covers both themes/three sizes, OS query typing, confirm/context/cancel/selection, fetched membership, updates, orientation, retry and source reset in repository/installed galleries. The remaining scopes above stay open. |
| `ListItem`, separator, check/suffix, active/right-click highlighting | Ordinary row/button/icon/label/separator composition and styles; `Command`/`Menu` own actions and context menus. Confirmed selection and keyboard cursor are separate application state. `ListSettings.active_highlight` is a presentation policy, not a required global singleton. Arbitrary mouse callbacks use the typed input observation API. |
| `RowsCache`, section index paths and loading helpers | Internal metadata maps to stable `List_collection` identity/order and bounded managed rows. Headers/footers can be explicit keyed data entries; applications own section flattening. No separate cache, index-path or loading-widget compatibility API is needed. Existing tree paging already uses typed boundary rows. |
| More data, loading threshold, empty/initial/loading renderers | `List_paging`/`Gpuio_eio.List_paging` and `Virtual_list.paged` own before/after demand, ready/error/retry/cancel and stale-generation rejection. Prefetch uses pixel overscan and bounded demand rather than upstream's fixed twenty-item threshold. Empty/loading/error content uses application Views. Viewport eviction does not cancel application data work. |
| `MessageScroller` append/prepend/splice, tail state, scroll to latest | `List_collection` plus `Follow_tail_when_at_end`, `Output.viewport` and `Controller.jump_to_latest`. Point updates retain order and invalidate only changed row values; they must not splice for every streamed fragment. Agent chat already composes a latest-message action. The Collections gallery now exercises earlier history, append and incremental latest-row growth with a 24-row active budget. |
| Message content/list/row/jump-button styling and optional bottom fade | Ordinary View styles, caller-owned row wrappers and buttons are functional composition points. The Collections message preview now demonstrates optional bottom fading and an animated **Follow latest** action, using the existing native animation scheduler over stable list geometry. Hidden controls become inert immediately and settle outside the clip, preserving wheel input. Jump/fade/motion switches are independent. See the [composition contract](../design/message-follow-presentation.md) and [local evidence](../evidence/message-follow-presentation-och41.md) and [physical follow-up](../evidence/message-follow-macos-och41.md). Repository/installed theme/size checks cover bounded overlay geometry, Space/pointer, hidden-slot wheel input and reading anchors under data updates; full frame/VoiceOver/resource acceptance remains open. The public `Accessibility.Role.Log` now marks an ordinary container or managed list. It defaults to polite live semantics, with an explicit Off override used by the fragment-streaming gallery. This supplies semantic metadata, not announcement batching or an offscreen history mirror; VoiceOver acceptance remains open. |
| `ScrollableElement` x/y/both wrappers and `ScrollableMask` | Ordinary `Style.Overflow_x/y Scroll` containers retain native scroll state. `rust/native/src/scroll.rs` performs same-axis nested wheel routing and reveal. Managed lists share their native handle with a mask/scrollbar. The helper traits themselves stay in Rust. |
| Scrollbar axis, visibility mode, base/hover/active track/thumb styles, finite entrance/exit/width motion | **Partial:** checked `Scrollbar` values and `View.with_scrollbar` now connect paired metadata/admission to native ordinary-container, managed-list/tree and scoped table owners, preserving existing handles and visibility flags ([table evidence](../evidence/scrollbar-table-och41.md)). Collections now demonstrates the modes, axes, state styling and finite motion through public controls and a two-axis growth preview; its fresh installed-consumer build passes. Physical macOS acceptance remains unfinished. Root styles alone do not configure internal scrollbar parts. Shared presentation owns no separate offsets or OCaml frame loop. |
| `AutoScroll` edge calculation and per-frame callback | A native interaction helper for its owning gesture. It is not a public OCaml timer callback requirement. Do not infer generic drag-edge auto-scroll from keyboard reveal or wheel support; evaluate it with each affected native drag interaction. |

Baseline behavior/resource evidence:
[managed lists](../evidence/managed-lists-och13.md),
[choice review](choice-review.md),
`test/virtual_list`, `test/runtime/list_paging_test.ml`,
`rust/native/src/list_view.rs` and `rust/native/src/scroll.rs`.
The message gallery model bounds additions to 32 at either end and latest-row
growth to eight fragments. These are demonstration limits, not library limits.
[Current local evidence](../evidence/collections-catalog-och41.md) records model,
paired Log codec/native semantics, full tests/lint and installed-consumer results;
the updated physical gallery walkthrough remains unrun.

## Trees

| Pinned surface | GPUIO mapping and limitation |
| --- | --- |
| Stable item ID/label, child hierarchy, expanded/disabled state | `Tree`, `Tree.Node`, `Tree_state` and immutable loaded topology. Empty branches remain folders, unlike the upstream children-is-empty folder test. Selection/expansion survives keyed moves; deleted/reintroduced IDs have distinct incarnations. |
| Flattened entries, selected item/index, programmatic reveal and ancestor expansion | `Tree_rows`, managed `Tree.Output`, `Controller.select/reveal/set_expanded` and identity-only captured Targets. Reveal opens loaded ancestors; it never implicitly loads an unknown path. Source and mount generations reject stale commands. |
| Arrow keys, confirm, focus and disabled rows | Native tree input reduces through `Tree_state`; it skips disabled items and supports first/last, parent/child, typeahead, multiple/range selection and explicit activation. Upstream Up/Down wraps; GPUIO stops at boundaries. Opening/closing is separate from activation/selection rather than reproducing upstream's folder-toggle-on-row-click policy. These are deliberate interaction contracts. |
| Custom row renderer, indentation, selected/secondary-selected paint, list style | `render_item` supplies retained child Views inside one themed native TreeItem. The tree owns disclosure, semantic position and selection/focus overlays; child editors/buttons retain native input. Style inheritance does not expose every internal selected/disclosure part independently. |
| Styled tree per-row context menu | Already composed in `examples/tree/outline_demo.ml` and `examples/agent_chat/runtime/sources.ml`: per-row `Command.Registry`, `View.command_scope/context_menu/menu_button`, captured Target and managed-row lifetime guard. The explicit menu button supplies a keyboard path. Context actions target the row without implicitly changing primary selection. Upstream right-click secondary highlighting is not a separately exposed Tree preference. |
| Scroll handle and scrollbar | Managed-list controller/budget/scrollbar ownership. General scrollbar presentation gaps above apply here too. |
| Lazy children, drag/move approval, row retention | GPUIO adds bounded Eio loading and generation-checked move proposals; native drag never mutates application hierarchy. These have independent OCH-38 contracts and are not synchronous upstream callbacks. |

[Tree evidence](../evidence/managed-trees-och38.md) records native input,
AppKit semantics, custom-row focus, reveal, lazy loading and 100k-node retention
workloads at their tested revisions. `examples/tree` remains the deep behavioral
example; the gallery shows expansion, selection and deep reveal. Source review
does not certify current VoiceOver speech or physical macOS release acceptance.

## Managed and structural tables

| Pinned surface | GPUIO mapping and limitation |
| --- | --- |
| `DataTable`/`TableState`, retained delegates and virtual rows/columns | `Table_data`, `Table.Config`, `Gpuio_bonsai.Table` and the extracted Rust table adapter. Synchronous delegate calls read admitted native state only. All columns of each active row count toward the cell budget even when horizontal painting culls them; this is not unbounded two-axis materialization. |
| Column identity, widths/min/max, fixed-left, alignment, resizing/movement/sort | `Table_column` and validated `Collection`; native proposals use stable keys. Application accepts width/order/sort and supplies a new schema/data order. Grouped headers preserve contiguous membership and pinned boundaries. Sorting never silently sorts an incomplete remote page. |
| Global resizing/movement/sort switches | Compose per-column flags through the schema. No second global Boolean is needed to express the same behavior. |
| Row/column/cell selection, right-click target, keyboard and activation | Typed `Selection_mode`, optional column selection, `Request.Select/Context/Activate`, and generation-bound controller batches. Context target is independent of primary selection. Tab remains host focus traversal, unlike upstream cell-navigation Tab; native arrows/home/end/page keys handle the table. |
| `row_header`, looping selection, per-column `selectable` | `Table.Config.row_header`, `Boundary.Stop/Wrap` and `selectable_headers` now expose these controls, with immutable setters and defaults preserved. Eligibility gates **column-header selection**, not cells, sorting or embedded controls. Changes fence stale input and repair forbidden selection while retaining the native owner and surviving Bonsai cells. Collections → Result table demonstrates all three. See [contract](../design/table-behavior.md) and [local behavior evidence](../evidence/table-behavior-och41.md); physical acceptance remains open. |
| Stripes, cell sizing, borders, independent scrollbar visibility | Row height and outer border/foreground/background use public config/styles. Shared `View.with_scrollbar` supplies independent axis presentation controls. `Table.Appearance` now supplies native stripes and named header/row/stripe/selection/sort/border colors; empty space is painted by the native row renderer, including decorative short-table fillers. See [presentation contract](../design/table-appearance.md) and [local evidence](../evidence/table-appearance-och41.md). Physical GPU color/alpha qualification remains open. |
| Per-column padding, render header/group/row, custom empty/loading | Custom retained cells support arbitrary Views and copy text. Application can compose an outer loading/empty state. Shared and per-column native leaf-header/body padding are public through `Table.Appearance`; zero padding removes the native inset. Rich leaf/group Views now submit through `Table_header` and Core/Bonsai `?headers`, with stable keyed native controls, exact group membership, separate body budgets and schema fencing. The gallery demonstrates this; see [local rendering evidence](../evidence/table-header-rendering-och41.md). Checked native header/body-row presentation now uses `Table_presentation.Header/Row`, Bonsai active-row computations and native state refinements, with a gallery toggle and [local evidence](../evidence/table-scoped-presentation-och41.md). Physical qualification remains open. The [renderer-slot contract](../design/table-renderer-slots.md) separates rich header content from checked row/header styling. Native shape must remain a submitted description, not a synchronous OCaml delegate. |
| Scroll row/column, reveal, selected values and visible ranges | `Table.Controller`, `Output.selection/viewport`, shared list demand and revision-fenced commands. `Table.Column_viewport` and Bonsai `Output.column_viewport` now report stable visible column IDs, pins and horizontal full/partial visibility, including empty-data headers. Actual pane/clip measurements exclude overscan and preserve the existing cell budget. The Result table gallery displays the observation. See [contract](../design/table-column-viewport.md) and [local evidence](../evidence/table-column-viewport-och41.md); physical acceptance remains open. |
| `headers`, `dump`, `dump_range`, `cell_text` | Application-owned `Table_data`/paged source is the export authority; use Eio for I/O. `Table.Cell.copy_text` is retained accessibility/copy data. Native copy is bounded (1 MiB) and requires a complete retained selection; missing data preserves the clipboard and emits an application intent. No full-data native mirror or synchronous export callback is needed. |
| Structural `Table`, Header/Body/Footer, Row, Head/Cell with spans, Caption | `Table_view` composes checked, keyed cell/row/section descriptions into ordinary Views, with explicit table names, derived global row/column indices/counts, header roles, actual grid column spans and captions. All cells remain mounted; no managed dataset/controller is introduced. Native semantic vocabulary also supports independently composed containers. The gallery demonstrates grouped headers, row headers, footer, caption, reversal and native buttons. See [contract](../design/structural-tables.md) and [local evidence](../evidence/structural-tables-och41.md). Physical gallery/VoiceOver acceptance remains open. |
| Editing, spreadsheet behavior, LSP/editor integration | Deferred OCH-42. None of the presentation/semantic gaps above is automatically deferred with editing. |

[OCH-39 audit](../evidence/data-tables-och39-audit.md) and
[evidence](../evidence/data-tables-och39.md) retain the existing native selection,
copy, grouped-header, sort/resize/reorder, pinned-column, 100k-row retention and
AppKit results. [Adapter provenance](../../rust/table/UPSTREAM.md) records the
selected extraction; the whole styled crate is not a dependency or a claimed
compatible library.

## Remaining acceptance

Keep these three family rows open. Shared scroll presentation and horizontal
managed virtualization have local integration evidence. The focused horizontal-card
walkthrough now passes physical input and sampled geometry in both themes/three
application sizes, including the installed consumer; full scrollbar/trackpad,
VoiceOver and resource acceptance remain separate. The standalone list now has managed selection, Eio search
and a public gallery with scoped physical keyboard/appearance qualification;
complete the broader pointer/accessibility and resource review. Table
behavior, part colors, stripes, padding and visible-column observations now have
local integration evidence. Structural table composition now also has public APIs,
gallery and local semantic/layout evidence. Retained rich headers and checked
native header/row styling now have public examples and local renderer/lifecycle
evidence; qualify physical table rendering/input separately. Preserve existing wire layouts,
retained native owners, bounded admission and asynchronous event delivery.
The message follow presentation now has a public-API gallery recipe and native
layout/input/settlement evidence; qualify physical transcript accessibility and
scrolling alongside the remaining capabilities. Do not add separate tickets for internal
cache/handle/helper types or recreate synchronous Rust delegates in OCaml.

For each new native behavior, validate reset/update/removal/window-close,
disabled/hidden/composition/focus precedence and bounded work. Physical macOS
keyboard/IME/VoiceOver/GPU/resource and clean-consumer release gates remain OCH-17.
Linux compilation/unit/private-bus/consumer checks remain required; X11/Wayland
desktop qualification is deferred OCH-47. Old evidence that assigns full Linux
desktop acceptance to OCH-17 is superseded by the
[platform release policy](../platform-release-policy.md).
