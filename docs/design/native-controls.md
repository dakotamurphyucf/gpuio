# Native controls (OCH-11, in progress)

The initial controls add `View.checkbox`, `View.switch`, `View.radio_group` and disabled buttons
to both the pure action API and the Bonsai effect API. The rest of OCH-11 remains
in progress: selects/comboboxes, overlays, commands, pointer/drag interactions, dialogs,
assets, and their acceptance checks. This document records the implemented
contracts and the integration findings; it is not completion evidence for the
whole ticket.

## Application values and activation

OCaml owns checkbox/switch values. Rust retains native focus and immediate input
state. Activation requests an action; it does not send a next Boolean computed
from an earlier render. Apply that action against the current model:

```ocaml
let component graph =
  let checked, toggle = Bonsai.Cont.toggle ~default_model:false graph in
  let open Bonsai.Cont.Let_syntax in
  let%arr checked = checked and toggle = toggle in
  Gpuio_bonsai.View.switch ~checked ~on_toggle:toggle "Stream responses"
```

For a three-state checkbox, use `Check_state.t` and a state machine that applies
`Check_state.activate` to its current model. Indeterminate activates to checked.
Two native activations before a render remain two actions. An application can
ignore or reject an action; the native control continues showing its committed
value. There is no optimistic application-value mirror or acknowledgement race.

`?disabled:true` suppresses activation and removes the control from Tab order.
The reconciler invalidates its handler; re-enabling allocates a new generation,
so a queued activation from before disabling cannot fire afterward. The native
session also checks the current disabled state when an older painted callback
runs. Value/configuration changes retain the keyed native node and focus handle.
Changing primitive kind replaces the node and invalidates its old events.

`Choice.Id` and `Choice.Collection` provide stable choice identity. Labels and positions are independent of stable identity. Collections
reject duplicate IDs, preserve ordering, permit disabled selected options and
bound item count and aggregate text. `Choice.Config.create` validates the accessible
label, options and selected membership; it permits an absent or disabled selected
value. `View.radio_group ~config ~on_select ()` emits a stable `Choice.Id.t`.
Both native event publication and OCaml dispatch check the latest enabled-option
membership. Reordering refreshes configuration without changing node or handler
identity. Select and combobox adapters follow.

A radio group owns one Tab stop. Arrow keys wrap through enabled options;
Home/End reach the first/last enabled option. Navigation requests selection, and
Enter/Space requests the active option. The native active ID is distinct from
OCaml's committed selected ID: it remains stable across reorders and intermediate
value commits while focused. Removing/disabling the active option selects a valid
navigation fallback, without generating a user selection event. Outside keyboard
focus, the navigation target follows an enabled committed selection or the first
enabled option. Accessibility Focus moves focus without changing application
selection; accessibility Click requests selection.

## Native behavior and accessibility

The pinned GPUI `Div` already synthesizes keyboard clicks. Controls reuse this
behavior for Enter and Space, which activate once on key release. A second
manual Enter handler emitted duplicate actions; it has been removed. The older
button test now sends the complete down/up sequence and checks both phases.

A retained window focus target, excluded from Tab order, receives keyboard
navigation before the first mouse click and after removal/disable of the focused
control. This is the baseline window traversal; nested overlay trapping and
restoration remain to be implemented under this ticket.

The actual focus root owns the semantic role, name, toggled value, disabled state,
Focus and Click actions. Accessibility Click is routed directly: GPUI's fallback
simulates mouse input, which would otherwise be suppressed by `Pointer_events
false`. That property suppresses pointer activation while preserving keyboard
and accessibility activation. It remains distinct from disabling a control.

`accesskit_macos` 0.26.3 converts both checked and mixed to `NSNumber(true)`.
At the macOS semantic boundary, mixed checkboxes are represented as numeric value
2, the native mixed value. Other backends retain AccessKit's `Toggled::Mixed`.
The adaptation is in `rust/native/src/semantics.rs`; remove it when the pinned
backend preserves mixed values itself. It requires no dependency fork.

## Styling and bounded lifetime

`Style.State` adds `Checked`, `Indeterminate`, `Disabled` and `Selected` vocabulary.
Selected refinements apply to the committed selected radio option. Base
properties are refined by the applicable value state, then focus/hover/pressed.
Disabled controls suppress hover/pressed and apply disabled refinements with a
default opacity of 0.5. Checkbox/switch indicators inherit foreground color,
including theme and state refinements. Their geometry is painted natively and
does not depend on a glyph being present in a font.

Editor focus styling reads the native editor's actual focus handle. Registering
that handle again on an outer wrapper would introduce a duplicate Tab stop.

Control configuration has fixed size and lives in the existing bounded retained
tree. No value history accumulates. The focus registry is pruned with retained
node generations; removing a tree releases its control/editor state. Resource
limits for overlays and assets are still part of the remaining implementation.

## Protocol and checks

Capability bit 16 advertises simple controls; bit 32 advertises stable choices
(current mask 63). Append-only tags:
checkbox/switch kinds 5/6; Set_control operation 8; Control variants button 0,
checkbox 1, switch 2; check states unchecked/checked/indeterminate 0/1/2. Each
control's final Boolean is disabled. Semantic style states occupy 4 through 7.
Radio-group kind 7, Set_choice operation 9 and Choice event 13 extend that schema.
Choice configurations are bounded to 4096 options and 256 KiB aggregate option
ID/label text; their retained text and item records count against tree budgets.
The decoder rejects unknown tags and malformed Booleans; retained-tree validation
rejects mismatched/missing configuration atomically.

`controls-v1.hex` has independent OCaml and Rust constructors. The earlier editor
fixture intentionally retains its original capability mask 15. Tests cover
truncation, invalid tags, rapid actions, latest callbacks, handler generations,
rollback and retained memory. `native_controls` exercises an actual window and
macOS accessibility actions; Linux GUI remains informational under OCH-17.

`examples/controls/main.ml` demonstrates the public Bonsai API. Run it with
`./scripts/gpuio exec dune exec examples/controls/main.exe`.
