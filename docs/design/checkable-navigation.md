# Checkable composition and Tab order

OCH-41 implementation contract, 2026-10-01. Core/Bonsai constructors, paired
protocol operations, native admission/rendering and the public gallery are wired.
Local verification is in progress; real macOS keyboard/pointer/AX and installed
consumer runtime acceptance remain open.

## Types and API direction

`Tab_order.t` is a validated value with `tab_stop` (default true) and signed
`index` (default zero, -1,000,000..1,000,000). Match the existing Link contract:
ascending index, stable tree/paint order for ties; negative indices do not disable
Tab traversal. `tab_stop=false` removes sequential traversal, while pointer,
programmatic and accessibility focus remain available. Disabled/inert/hidden and
modal policies still take priority. Updating order preserves the native handle.
This is native GPUI ordering, not HTML tabindex semantics.

`Radio.Position.t` uses zero-based `index` and `count` in 1..100,000 with
0 <= index < count, matching existing bounded semantic collection metadata.
Only its eventual accessibility output is one-based. Position/count are a single
validated value so they cannot be half-specified or contradictory.

View surface:

```ocaml
val radio
  :  ?key:Key.t
  -> ?style:Style.t
  -> ?appearance:Control_appearance.t
  -> ?accessible_name:string
  -> ?disabled:bool
  -> ?tab_order:Tab_order.t
  -> ?position:Radio.Position.t
  -> checked:bool
  -> on_select:(unit -> 'action)
  -> string
  -> 'action t
```

A checked rich-label counterpart follows `checkbox_with_label` with required
accessible name and one passive label. Existing checkbox, switch and managed
radio-group constructors gain the same optional Tab-order value; omitted values
restore their normal traversal policy. Bonsai wrappers adapt unit effects.

A standalone radio is one retained RadioButton with its own focus handle,
checked/toggled/selected semantics and optional position metadata. It cannot
uncheck itself: unchecked activation queues a selection intent; checked or
disabled activation is a no-op. Apply `on_select` as an idempotent selection,
not a Boolean toggle. Revalidate queued native callbacks against current
accepted config and handler generation. Unmount/replacement retires the owner.

No implicit group/exclusivity or Arrow navigation is inferred for standalone
controls. Applications own selection. The existing managed `radio_group` is the
recommended collection API and preserves rapid pre-commit selection requests;
its A→B→A behavior must not be replaced by the standalone checked-click rule.
`Accessibility.Role.Radio_group orientation` marks an ordinary container for
arbitrary standalone compositions. It supplies group semantics and orientation
without inventing a selection model or Arrow navigation.

## Bridge

The retained-tree extension appends Kind52 `Radio`, Control3
`Radio (checked, position option, disabled)`, Op68
`Set_tab_order (node, Tab_order option)` and semantic role15
`Radio_group orientation`. Existing tags and Link bytes are unchanged. Admission
validates the complete resulting tree and raw Rust values; malformed late changes
roll back atomically. Radio reuses scalable indicator painting and the passive
rich-label rules, with its own native role and focus owner.

Capability bit 62 is the last positive bit in the current signed Int64 mask.
`CAP_CHECKABLE_NAVIGATION` now advertises the connected extension. Its Hello bytes
are `0001fc0000000000000040`; the complete mask is signed Int64 maximum
`9223372036854775807` with bytes `0001fcffffffffffffff7f`. An older host rejects
negotiation. Further capability expansion requires an explicit paired protocol
and version design; do not silently shift into the negative sign bit.

Required evidence includes constructor and decoding boundaries, independent bytes,
invalid late changes/rollback, retained handles, plain/rich label updates, checked
no-op versus unchecked selection, stale actions after disable/remount, signed
stable Tab order, skipped stops, modal/virtual-row interaction and zero ownership
on removal/window close. Public gallery/installed-consumer and real macOS
keyboard/pointer/AX tests remain separate from TestPlatform evidence.

## Validation and remaining acceptance

Abstract Core values and paired checked codecs cover defaults, boundary values,
signed extremes, invalid positions, truncation, trailing data and malformed
Boolean/orientation encoding. Independent fixtures cover the values, transaction
and semantic group. Reconciler expectations cover checked callback retirement,
fresh reactivation, stale-event rejection, rich-label updates and reset without
replacing the owner. Native transaction checks cover invalid late changes,
rollback, disabled activation and close cleanup. The production View TestPlatform
check covers signed stable order, skipped stops, pointer/programmatic focus,
checked click suppression, real TestPlatform Space/Enter press/release and
Tab/Shift-Tab dispatch, retained handles and teardown with no idle frames.
The full native library passes 481 tests with two existing skips. The full Dune
test suite, formatting and gallery build also pass, as do the Rust workspace
and strict native/protocol all-target Clippy with the headless test features.
A fresh independently installed gallery consumer builds; it was not launched.

The public Controls gallery's “Compose a choice your way” card uses only documented
Core/Bonsai APIs. It demonstrates application-owned exclusive selection,
rich/plain labels, semantic grouping, reverse ordering, skipped Tab stops,
disabling and restoring normal order. The managed-group preview remains separate.

These are local codec/model/production-View checks, not macOS GPU or OS-input
acceptance. Public and independently installed
runtime behavior, real keyboard/pointer/AX checks and full selection-family
acceptance remain open. See the [selection review](../catalog/selection-review.md).


The next native-only checkpoint passes 484 library tests (two existing skips) and
strict all-target Clippy. It adds a trapped modal with custom indices and a skipped
stop, outside-click rejection and focus restoration; a 100,000-logical-row list
with three described radios, retained focus on updates, clipped-focus exclusion,
eviction/remount generation fencing and new-owner activation; and session-close
rejection while a View is retained. A production semantic-builder test checks
RadioButton role, selected/toggled values, optional one-based membership and
disabled selection. These tests use TestPlatform and do not establish OS AX/input
or whole-application performance acceptance.
