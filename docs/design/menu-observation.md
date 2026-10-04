# Menu observations and split-button coordination

OCH-41 implementation contract, 2026-10-01. MenuButton observation and root
placement are connected locally; full validation and desktop acceptance remain
open. Checked split composition and native shared hover are now connected too;
their full lifecycle/physical acceptance remains open. The
[pinned button review](../catalog/button-review.md) identifies why composing two
existing buttons is insufficient: the source coordinates ghost hover across both
halves and retains that surface while the menu is open. Its root popup anchor is
also configurable. These remain v1 work, not deferred catalog rows.

## Public interface

The native-managed `View.menu_button` now accepts:

```ocaml
val menu_button
  :  ?key:Key.t
  -> ?style:Style.t
  -> ?appearance:Menu.Appearance.t
  -> ?placement:Placement.t
  -> ?on_open_change:(bool -> 'action)
  -> menu:Menu.t
  -> unit
  -> 'action View.t
```

The Bonsai wrapper takes a Boolean-to-effect callback. `on_open_change` observes
actual native visibility; it is not a request to an application-controlled menu.
Opening, submenu navigation, outside dismissal, focus restoration and command
activation remain native. The callback must never be needed for the menu to work.
Omission preserves existing behavior and creates no event subscription. Do not
add these parameters to platform menu bars until their AppKit lifecycle is
implemented and tested. Context/in-window-bar extensions can share the machinery
but must have explicit semantics and equivalent evidence before exposure.

Placement applies to the root popup only. Preserve the current default and the
existing native submenu flip/clamp behavior. Reuse `Placement` validation and
`Set_placement` encoding, but extend native admission only for the supported menu
presentation. A placement change recomputes popup geometry without closing it,
changing selection or replacing focus. Tests must include all preferred sides,
cross-axis alignment, viewport edges, resize, root scroll and scale changes.

## Observation lifecycle and bridge

Deliver one initial snapshot when a subscription attaches, including attachment
to an already-open menu. Thereafter emit only root closed/open transitions;
moving selection or entering/leaving submenus while the root stays open must not
generate redundant callbacks. On handler replacement, deliver a fresh snapshot
to the replacement handler. Style/appearance-only updates keep the subscription.

The event envelope includes WindowId, full generation-checked NodeId, HandlerId,
accepted tree revision and the observed Boolean. Event67 `Menu_open_changed` has independent OCaml/Rust fixture bytes and
requires protocol epoch3.
The legacy capability mask is already full. Do not overload tooltip events or
silently widen an old epoch. Use existing ordered bounded delivery; lifecycle
edges must not be coalesced away or silently dropped. Queue overflow follows the
window fault policy. There are no synchronous OCaml calls from Rust callbacks.

Core dispatch checks the current accepted subscription. Removal, window close,
handler retirement and menu-definition replacement fence old observations; a
prepared but unaccepted transaction must not change dispatch. Define and test
the replacement fence before implementation, including open→close→open within
one accepted revision and unrelated style transactions between observation and
dispatch. A fresh snapshot restores the new subscriber's state after replacement.

Native culling of a still-retained node publishes closed before dropping its
native menu state. Recreating that owner emits a new closed snapshot; subsequent
redraws are silent. Removing the OCaml node or closing its window retires the
observer and rejects queued old events; it does not call
an already-retired callback with `false`. Application-owned split composition
must clear its visible-open projection when it removes its menu half. An observer
does not own or prolong the menu, window or application task lifetime.

## Native integration points

`rust/native/src/menu.rs` currently changes root visibility through `open_menu`,
`close_menu`, replacement of another open menu, config replacement and
render-time focus/visibility/interaction checks. Centralize transition reporting
so **every** close path participates. Validate the requested target before
closing another menu; an invalid/disabled target must not dismiss a valid one.
Root open/close observations and command invocations must have documented order:
closing for invocation reports closed before queuing the command action.

Some invalidation is discovered during render while `View::render` holds a
shared Session borrow. Do not enqueue through a path whose overflow handling
tries to mutably borrow that Session reentrantly. The implementation validates and enqueues against the current accepted
node/handler/config under the shared borrow. Only overload/fault handling is
deferred until that borrow ends. There is no deferred visibility replay or
separate unbounded pending-event queue. No idle timer
or unbounded history is needed.

## Shared hover and public hover callbacks

Open-state reporting alone is not split-button parity. The pair must share a
native hover surface, with the hovered half emphasized and both halves surfaced
while open. Do not drive frame-by-frame highlight painting through Bonsai or a
foreign round trip. First evaluate a scoped native style-group mechanism versus
a dedicated split-button presentation adapter; document the chosen ownership,
stable identities and style precedence before adding its public interface.
Primary and trigger remain separate actions/focus stops. Support action-only,
menu-only and split modes, shared size/variant/selected policy and independent
primary disabling in addition to whole-pair disabling.

### Chosen split integration

Use a dedicated split composition backed by GPUI's existing native `group` and
`group_hover` support, with one private constant name and GPUI's lexical group
hitbox stack. Full generation-checked element identity owns each half's cached
hover state. Do not expose arbitrary string group names, search by label, or let
unrelated controls share mutable state. The pinned GPUI `GroupHitboxes::pop`
retains empty entries; dynamically naming every node/window would grow that
application-global map across unmounts. The private constant bounds namespace
storage while the lexical stack distinguishes independently painted pairs.
The pinned source uses the same group-hover primitive, with a native menu-open
projection keeping the primary surface visible after the pointer leaves.
GPUI applies group-hover before individual hover/pressed refinements. Both
halves retain their existing action/focus/accessibility owners.

A general public style-group API would require additional nested-group lookup,
arbitrary descendant lifetime and interaction contracts. Those are unnecessary
for this component. The dedicated adapter can validate its small shape and
reuse the existing button/menu lifetimes without a second scheduler or a
subscription back to OCaml for painting.

`Split_button.Appearance` now defines and validates the paint configuration:

```ocaml
val create
  :  ?surface:Style.t
  -> ?menu_open:Style.t
  -> unit
  -> t Or_error.t
```

Only Base Background, Foreground, Border_color, Shadows and Text_decoration are
allowed, at most 64 normalized declarations combined. Tokens resolve against
the current theme. Structural properties, opacity and nested interaction states
are rejected. `surface` is the shared hover/menu-held surface; `menu_open`
additionally decorates the open trigger. Loading/disabled halves receive neither.
Individual hover and pressed presentation retain precedence. The Core/Bonsai
constructor, paired Op70 and native coordinator now consume this configuration.

The checked `View.split_button` constructor accepts optional `primary`
and `menu` views, with at least one present. Primary must resolve to an ordinary
or command button; menu must resolve to MenuButton. Permit a bounded chain of
Tooltip anchors for individual help, not arbitrary containers or interactive
descendant searches. Native admission independently enforces the same shape,
including updates to a descendant's menu presentation. Each part has a fixed
keyed single-child Container slot; the original caller key remains unchanged on
its child, including 256-byte keys. Removing the primary does not replace the
menu owner. These exact slots are validated; arbitrary wrappers are not searched.
Default composition is a horizontal, zero-gap pair aligned to the start of its
parent's cross axis. This keeps the hover hitbox at content width in an ordinary
column. Explicit caller layout styles may override that alignment. Previously,
the default stretched across the column and hovering empty space surfaced both
halves; a production-View regression reproduces the failure before the alignment
correction and passes afterward. The public preview explicitly uses a transparent
resting background so shared hover is visible.
In split mode inner corners and the menu's left border are removed;
single-part mode restores all outside corners. Common size/variant/selected
appearance remains ordinary public style composition. Per-part disabled/loading
policy stays on the existing owner; whole-pair disabling uses the inherited
disabled contract.

```ocaml
val split_button
  :  ?key:Key.t
  -> ?style:Style.t
  -> ?appearance:Split_button.Appearance.t
  -> ?primary:'action View.t
  -> ?menu:'action View.t
  -> unit
  -> 'action View.t Or_error.t
```

The Bonsai wrapper specializes the same constructor to effect-valued views.

The renderer derives native held state from the **current accepted** menu
definition, current focus/visibility/gate and existing menu path before painting
either half. A stale retained menu path alone is insufficient: disable,
definition replacement, focus loss, hidden/inert ancestry, culling and removal
must clear coordinated presentation without waiting for the OCaml observer.
Avoid a traversal-order-dependent stale frame when the primary renders before
the menu performs its usual cleanup. Style coordination must also stay out of
tooltip content and rich passive button labels.

The wire value has Parts tags Primary=0, Menu=1, Split=2 and two Style
lists; its raw decoder enforces a combined 64-field budget and 64 entries per
list. Native admission additionally validates all field values and accounts
for configuration/style/shadow storage. Op70 `Set_split_button (node, config
option)` has independent set/reset fixtures. `None` restores an ordinary
Container; invalid child/tooltip/menu-presentation edits roll back atomically.
Dirty ancestors are revalidated, so a descendant-only update cannot bypass the
shape contract. No capability bit is available. This mandatory operation joins
the existing **unpublished epoch3** implementation in both runtimes. No published
wire epoch is changed. Removing/resetting configuration drops its retained bytes.

Three Core expect checks and two Rust codec tests pass locally for the standalone
appearance value: independent paired bytes, all three part modes, truncated/
trailing/unknown-tag rejection, declaration limits, forbidden properties/states
and current-theme resolution. These are configuration checks only. The focused
commands are `dune build -j2 @test/view_api/runtest` and
`cargo test --locked -j2 -p gpuio-protocol --test split_button`, run through the
repository-isolated environment with `GPUIO_JOBS=2`.

Required follow-through: atomic shape/reset/child replacement; paint precedence
through native pointer dispatch with two independent pairs; kept-open hover,
focus and Escape/command ordering; disabled primary versus disabled pair; rich
labels/tooltips; mode changes retaining surviving owners; hidden/modal/inert/
virtual-row/window cleanup; public gallery and installed consumer; real macOS
pixel/keyboard/AX acceptance. The standalone appearance tests establish none of
that renderer behavior by themselves.

`pointer_area` cannot supply hover observation: it captures press/move/release/
cancel gestures. Tooltip visibility also includes focus, delays and dismissal,
so it cannot stand in for hover entry/exit. A public button hover subscription
must use native hit testing, not rectangle guesses, while preserving click
ownership and selection exclusion. Specify the final leave/reset observation
when a hovered node becomes loading, disabled, hidden, inert, occluded or removed;
retired handlers must never receive callbacks. Keep visual hover native even if
an application subscribes to Boolean changes.

## Completion evidence

Require independent codecs and rejection of old epochs; Core accepted-state and
retired-handler tests; production-View tests for every open/close path, ordering,
subscriber replacement, no duplicate idle notifications, placement and teardown;
public action/menu/split examples; actual macOS pointer, keyboard and AX checks;
GPU shared-hover/open-state and corner/seam inspection; and a fresh installed
consumer run. Linux build/unit/private-bus/consumer checks remain required;
desktop qualification stays deferred under OCH-47. Neither this plan nor the
existing two-button composition establishes completion.

## Local implementation checkpoint

Core/Bonsai `menu_button` now has the two optional arguments above. Only MenuButton
accepts a node observer/placement; other presentations reject those unsupported
combinations atomically. Core rotates the handler for definition/presentation
changes, keeps it for placement/appearance changes, and dispatches through the
accepted current callback. Native observations carry Event67 under epoch3.
Opening checks the current tree definition before changing focus or closing a
peer. All existing root-close paths now report through one transition publisher.

The **Menus that report their state** gallery card demonstrates native-managed
visibility, subscription removal/re-attachment, disable, submenu invocation and
root placement. Its desktop walkthrough is included in `--section buttons` but
is unrun. The separate **Two actions, one control** card now demonstrates split,
action-only and menu-only modes, independent primary disabled/loading policy,
whole-pair disabling, individual tooltip help and native shared hover/held paint.
Its `gallery_split.py` walkthrough is authored and unrun; this does not establish
physical component acceptance.

Independent codec/malformed-envelope and Core accepted-state tests pass. Native
TestPlatform checks cover initial/late/replacement snapshots, repeated frames,
rapid same-revision transitions, invalid opening targets, Right/Start placement
with retained focus, real TestPlatform submenu keys without root duplicates,
closed-before-command ordering, focus-loss closure, native cull/remount, disable,
detach, removal and session close. The culling check explicitly drives the
renderer cleanup hook; it is not a complete virtual-list integration test.
Admission tests cover unsupported presentations, rollback, old epochs, revision/
handler checks and disabled opens.

The expanded TestPlatform placement matrix renders four sides by three alignments
at display scales 1, 1.5 and 2. It checks requested gaps/cross-axis alignment
within one device pixel, plus four edge-flip cases at each of two viewport sizes.
The live trigger moves in those frames without replacing focus or publishing
visibility changes. This exercises actual production layout/prepaint; it does
not measure GPU pixels or simulate root scroll input.

A queue-exhaustion regression fills the 128-event input lane with real native
open/close transitions, then attaches a replacement subscriber whose snapshot is
published during render. All earlier edges remain ordered; exactly one terminal
overload fault follows render, without a reentrant Session borrow panic. Further
ordinary input remains suppressed. The native library passes 493 tests with two
existing skips, and menu admission passes two tests. Full physical input/AX/GPU,
root scrolling, virtual-list/whole-application resource acceptance and shared
native hover/split lifecycle acceptance remain open.

The split renderer checkpoint passes production-View TestPlatform checks for
two independent pairs, per-half hover over shared surface, menu-held paint after
the pointer leaves, primary loading/disable while the menu stays open, definition
replacement before the menu renders, focus loss, retained primary focus owner
and optional reset. The expanded native library reports 494 passed/two existing
skips; one split admission test checks invalid part counts, descendant menu
presentation, slot handlers, unresolved tokens, rejected structural styling,
atomic rollback and released configuration bytes on reset. Six Core checks cover
the paired values/operation, theme/scope validation, bounded tooltip wrappers and
surviving 256-byte caller-key identity through mode changes. Full Dune tests,
formatting and gallery build pass. Actual pointer/keyboard/AX/GPU, full lifecycle
and public/installed-consumer runtime remain open.

The next production-View regression covers a tooltip-wrapped primary's actual
TestPlatform Space down/up producing one primary action, shared-hover isolation
from blank space, whole-pair disable/inert/hidden closure and restoration, removing
the primary while retaining the open menu's focus, Escape and full tree retirement.
It checks that native button/menu maps are empty after unmount. The library now
reports 495 passed/two existing skips. The compact default has a paired Core
alignment/explicit-override check. These additions do not establish modal,
virtual-row, multi-window or real macOS/installed-consumer runtime acceptance.
