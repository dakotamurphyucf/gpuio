# Command, palette and menu source review

Reviewed 2026-10-05 against GPUI Kit
`84f57fdfcb4910623fb0bb7f795b077e249f9271`. Source inputs are the manifest-pinned
`base-actions`, four `component-command-*`, six `component-menu-*`, four
`component-native_menu-*`, and shared `component-searchable_list*` snapshots.
`component/actions` is an inline crate-private re-export of Base actions, not a
second widget family. Windows implementation details are outside GPUIO's target
platforms. This review records functional equivalents and actual gaps; it does
not certify blanket command/menu parity or close OCH-41.

## Commands, identity and dispatch

[`Command`](../../lib/core/command.mli) and its registry provide stable string
IDs, labels, checked/enabled state and shortcuts. Buttons, menus and the palette
resolve those same commands in lexical scopes; scope-local IDs may shadow outer
registries. Native edit commands act on the retained native editor target,
without an OCaml editing round trip. Other activations enqueue application
intents which must be applied to the current model. An upstream boxed GPUI action
or synchronous callback is not an OCaml object passed into Rust.

Base's Confirm/Cancel/directional/page/column actions map to the owning widget's
native navigation, not a parallel public command registry for each Rust action
type. `Confirm.secondary` has meaning only where its consumer uses it: the pinned
command palette ignores that payload, while selectable-list confirmation has an
explicit primary/secondary contract. Do not promise an alternate palette action
merely because the common action structure contains a Boolean.

The source's action-context focus handle and restoration handle are distinct.
GPUIO similarly distinguishes command route/editor target from focus restoration;
native menu/bar routes are refreshed against current scopes and availability.
[`Command_binding`](../../lib/core/command_binding.mli) supplies asynchronous,
bounded binding observations, including conflicts, native priority, composition
and unavailable contexts. A displayed shortcut is not a promise that a future key
will invoke it. [`View.command_button`](../../lib/core/view.mli) has rich leading/
trailing icon presentation, but those decorations are not automatically menu or
palette item metadata.

## Palette and searchable content

| Pinned source behavior | GPUIO mapping and limits |
| --- | --- |
| Native search field, highlighted item, Enter confirmation and focus | `View.command_palette` / [`Command_palette.Config`](../../lib/core/command_palette.mli) mount a native-owned search session, modal by default. Query/navigation do not roundtrip through OCaml. Its private query is separate from document editors; native edit commands preserve the captured document target. |
| Inline/persistent command chooser | `Presentation.Embedded` uses ordinary layout and Tab navigation with no automatic focus or backdrop. Repeated selection keeps the query mounted; Escape requests application cancellation. Native edit commands use the current/last eligible document. Live presentation changes retain query identity and recapture the modal target/return focus. [Contract](../design/palette-embedded.md); [local qualification](../evidence/palette-embedded-och41.md). |
| Labels, keywords and local matching | Source `CommandItem.matches` tests the entire lowercase query as a substring of the label **or any keyword**. GPUIO now exposes bounded per-command keywords and `All_terms` (default), `Substring` and `Unfiltered` policies. `Substring` matches the source’s label/keyword semantics; the default also matches command IDs. Declared order is preserved. Neither is fuzzy ranking or Unicode normalization. [Contract](../design/palette-policies.md) and [local evidence](../evidence/palette-policies-och41.md). |
| Escape | Source clears a nonempty query first, then propagates empty-query Cancel to its host. GPUIO defaults to immediate dismissal and offers `Clear_query_first` for the source’s clear-then-dismiss interaction. Composition Escape ends marked text first; hidden queries do not require an extra Escape. Dismissal remains asynchronously delivered. |
| Checked/disabled actions and current active binding | Registry checked/enabled state is rendered and native eligibility is rechecked at invocation. Binding observations support reusable shortcut presentation. No claim that every upstream row indicator/binding layout matches the built-in palette. |
| Groups and separators | `Command_palette.Group` / `Entry` / `Config.create_entries` add stable groups, optional passive headings and separators. Filtering removes empty groups and redundant separators; keyboard selection remains command-only. Measured native rows preserve logical scroll anchors. [Contract](../design/palette-layout.md); local qualification is recorded in [grouped palette evidence](../evidence/palette-layout-och41.md). |
| Icons, arbitrary rich rows, header/footer and custom empty state | `View.with_palette_content` adds passive command-ID-keyed rows and ordinary interactive header/footer/empty Views. Rows keep registry names/actions and have measured heights; native query and document target ownership remain intact. Bounded structural slots, filtered-content input/animation gates, initial query focus and visual Tab order have [native and installed macOS evidence](../evidence/palette-content-och41.md). [Contract](../design/palette-content.md). |
| `searchable(false)`, `filterable(false)`, async query callback, loading and programmatic highlight | Built-in `searchable=false` hides the retained query and bypasses filtering; `Search.Unfiltered` keeps the query visible without local filtering. Optional `on_change` now delivers asynchronous native query/composition/highlight/count snapshots; `Snapshot.same_query` distinguishes accepted edits and observer lifetimes. [Contract](../design/palette-observation.md) and [local native/installed evidence](../evidence/palette-observation-och41.md). The public Eio `Palette_controller` now sends correlated read/focus/query/highlight commands with exact subscription identity and an optional native query fence. [Contract](../design/palette-commands.md) and [local qualification](../evidence/palette-commands-och41.md). Native `Set_loading` and observed loading state now preserve query/selection/undo, suppress empty content and provide reduced-motion-aware progress. [Contract](../design/palette-loading.md) and [native/installed macOS qualification](../evidence/palette-loading-och41.md). `Search.External` and query-fenced `Publish_results` now stage dynamic registry references and atomically install ordered/grouped results with loading completion; [contract](../design/palette-external-results.md) and [local native/installed evidence](../evidence/palette-external-results-och41.md) distinguish accepted View staging from publication. `Presentation.Embedded` now provides a persistent normal-flow chooser through those same APIs; see the [contract](../design/palette-embedded.md) and [native/installed macOS evidence](../evidence/palette-embedded-och41.md). [`Gpuio_eio.List_search`](../../lib/eio/list_search.mli) supplies scoped debounce, cancellation, retry, stale-query fencing and ordered external results for managed lists. [`Gpuio_bonsai.Selectable_list`](../../lib/bonsai/selectable_list.mli) supplies cursor/selection/controller, query input and before/after content. These are functional building blocks, not automatic equivalence for command dispatch. |
| `on_select` / `on_confirm` original index path and synchronous `on_cancel` | Built-in palette exposes command invocation, typed dismissal and asynchronous query/highlight snapshots, using stable command IDs rather than source index paths. Stable command IDs replace positional action identity. GPUIO does not expose synchronous OCaml callbacks in native input dispatch. |

Source `CommandState` measures custom row content and retains its query entity;
GPUIO now uses bounded measured native virtualization for command rows, headings
and short dividers, including rich per-command content. The generalized
searchable-list family belongs also to the [choice](choice-review.md) and
[collections](collections-review.md) reviews: reusable delegates, sections,
selection, custom content, empty/loading and query lifecycle must be qualified
there as well. A popup chooser does not by itself demonstrate a persistent list,
and a persistent list does not by itself demonstrate a command palette.

## Menus and platform behavior

| Pinned source behavior | GPUIO mapping and limits |
| --- | --- |
| Popup commands, separators, disabled submenus and checked items | [`Menu.Item`](../../lib/core/menu.mli) references registry commands, separators and nested immutable menus. Labels/state/availability are shared with other command presentations. Menus are bounded to eight levels, 1,024 items and 256 KiB text; application callbacks never build rows during native paint. |
| Button dropdown / context menu / application menu bar | `View.menu_button`, `context_menu` and `menu_bar` provide these presentations. Context menus wrap one child and support right-click/Shift-F10. Default platform bar uses AppKit on macOS and an in-window bar on Linux; `platform=false` explicitly requests the in-window bar. |
| Dropdown anchor and open-state callback | Public `Placement` and `on_open_change` support root popup placement and initial/edge observations; callbacks are asynchronous, generation-fenced and not required to open or navigate the menu. [Physical placement evidence](../evidence/installed-split-paint-menu-placement-och41.md) covers sides/alignment and root scrolling. Context/platform bars do not inherit that observer API implicitly. |
| Scrolling and nested menus | Native GPUIO menus virtualize large menus, reveal selected items and support cascading panels. The source documents a submenu restriction for its `scrollable` mode; that source restriction is not a reason to remove GPUIO's existing nested scrolling behavior. |
| Width/height, rows, empty state and theme | `Menu.Appearance = Choice.Appearance` supplies checked popup/row/empty-state style and geometry. This is GPUIO's theme vocabulary, not exact source preset dimensions. Source checkmark-side and external-link-icon switches are not public menu options. |
| Noninteractive label rows | `Menu.Item.Label` adds bounded passive text to drawn dropdown/context/bar menus, including nested submenus. Keyboard navigation skips labels; they have no command, focus or press action. Platform menu bars explicitly reject them. [Contract](../design/menu-labels.md) and [installed macOS evidence](../evidence/menu-labels-och41.md). |
| Per-item/submenu icons and custom element rows | `Menu.Item_path` and `View.with_menu_item_content` compose registered icons and passive rich labels for command/submenu/section positions; `editor_menu ~item_content` covers the editor wrapper. Native row ownership, names, checks and navigation remain intact. [Contract](../design/menu-content.md) and [installed macOS evidence](../evidence/menu-content-och41.md). Content uses the configured uniform row height; arbitrary nested interactive controls and per-item variable heights are not provided. Platform bars reject these slots. This is an explicit functional equivalent for passive rich action rows, not unrestricted source `AnyElement` parity. |
| Link items | An application command can request the desktop URL-opening service. It does not automatically add source link artwork or change menu accessibility semantics. Source handler/action precedence is replaced by one explicit command behavior. |
| `NativeMenu.show(position)` outside window bounds | `View.context_menu ~platform:true` now supplies an AppKit context popup on macOS, with right-click/Shift-F10, retained native ownership and revalidated asynchronous command dispatch. [Local and installed evidence](../evidence/native-popup-och41.md) covers physical input, a popup beyond window bounds, native Copy, window close, owner removal and stale-command rejection. The default remains drawn. [Design](../design/native-popup-menu.md) records remaining programmatic show-at-position, native icons and lifecycle qualification; right-click support alone does not complete the source API mapping. |
| Native popup Linux fallback | Source itself uses a drawn, window-clipped popup on Linux, held by its Root overlay. GPUIO's existing drawn menus are a corresponding rendering route; full Linux desktop behavior remains OCH-47, not proven by source similarity. |

The AppKit bar is application-global: current-window ownership, scope changes and
last-window disposal matter independently of the OCaml menu value. The source's
`AppMenuBar.reload` rebuilds from its application global; GPUIO instead reconciles
per-window definitions and installs the active window's routes. Platform menu
accelerator/key-equivalent presentation remains distinct from functioning native
shortcut dispatch; source action-binding text must not be inferred from labels.

## Evidence and next implementation work

Existing [menu](../evidence/native-menus-och11.md),
[palette](../evidence/native-palette-och11.md),
[command lifecycle](../evidence/command-lifecycle-och17.md) and
[menu observations](../design/menu-observation.md) record native input, stale
route rejection, editor targeting, bounded rows, active-window ownership and
cleanup. Earlier OCH-11 checkpoint statements are historical; they are not proof
of a new binary or screen-reader qualification. The Feedback gallery and
standalone `examples/menus` / `examples/palette` expose the currently supported
APIs. Current consolidated macOS acceptance remains OCH-17; Linux build/unit/
consumer and deferred desktop evidence remain separate.

The remaining programmatic OS popup operations, native icons, lifecycle and consolidated palette/menu family
qualification are catalog work, not newly approved post-v1 exclusions.
Implement typed, bounded content/identity contracts first, retain native command
resolution and editor targeting, and demonstrate the resulting public API in the
gallery. General-purpose list composition alone must not close those rows. The
review records these gaps so the family cannot be certified solely from its
existing `Command`, `Menu` and `Command_palette` module names.
