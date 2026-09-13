# Native controls (OCH-11, in progress)

The initial controls add `View.checkbox`, `View.switch`, `View.radio_group`, `View.select` and disabled buttons
to both the pure action API and the Bonsai effect API. The rest of OCH-11 remains
in progress: comboboxes, general overlays, commands, pointer/drag interactions, dialogs,
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
identity. Select shares this contract; the combobox adapter follows.

A radio group owns one Tab stop. Arrow keys wrap through enabled options;
Home/End reach the first/last enabled option. Navigation requests selection, and
Enter/Space requests the active option. The native active ID is distinct from
OCaml's committed selected ID: it remains stable across reorders and intermediate
value commits while focused. Removing/disabling the active option selects a valid
navigation fallback, without generating a user selection event. Outside keyboard
focus, the navigation target follows an enabled committed selection or the first
enabled option. Accessibility Focus moves focus without changing application
selection; accessibility Click requests selection.

## Select popup

`View.select ~config ~on_select ()` uses the same application-owned options and
selected ID. It shows the selected label, or the configuration label when absent.
The trigger is the only Tab stop. Enter/Space opens the popup; a subsequent
Enter/Space requests its highlighted option and closes it. Up/Down opens it;
while open, arrows move through enabled options and Home/End reach the ends.
Typing printable text opens the popup and searches enabled option labels with
Unicode lowercasing. Repeating one character cycles through matching choices;
subsequent characters refine the prefix. An unmatched prefix can restart from
the latest character. Highlighting alone does not request a selection. Escape cancels; Tab/Shift-Tab
closes and continues ordinary traversal. Pointer or accessibility activation of
an enabled option requests its stable ID. Outside clicks dismiss without choosing.

Rust owns open state, one active ID, a scroll handle and a search prefix bounded
to 256 UTF-8 bytes. The prefix resets after one second between keypresses and
when opening/navigating with arrow keys. Expiration is checked on input, so this
interaction requires no timer task. This non-editable prefix search does not
provide IME composition or accent/normalization folding; those belong to the
editable Combobox input contract. Focus stays on the
trigger; option semantics use active-descendant state and expose the committed
selection independently. Losing focus, disabling or removing the control closes
its popup. Reordering preserves the active ID and reveals its new position.
Removing/disabling the active option reconciles to a valid fallback. No popup
query or open-state event needs an OCaml round trip.

GPUI's `uniform_list` renders only visible fixed-height rows. Defaults are 32
logical pixels per row, eight visible rows, and a 320-pixel popup width. Deferred
positioning reads current-frame trigger bounds, prefers below, flips above when
needed and clamps to window margins. An explicit surface blocks pointer input to
covered content. The native test checks a 4096-option collection, offscreen
navigation and reorder. The trigger has an outlined, padded default and a painted
chevron; ordinary root styles can override its layout and appearance.

### Choice appearance

`View.select ?appearance` accepts `Choice.Appearance.t`. Its validated constructor
provides `popup_width`, `row_height`, `max_visible_rows`, localized `empty_label`,
and `popup_style`, `option_style`, and `empty_style` using ordinary `Style.t` and
theme tokens. Geometry requires finite positive dimensions at most 1,000,000
logical pixels and 1..64 visible rows; native geometry remains clamped to the
window. Empty text requires 1..1024 UTF-8 bytes without NUL. There are at most 128
style declarations across the three parts.

Part styles support background/foreground, opacity, border color, corner radii,
shadows, font/text presentation and cursor. Structural and interaction properties
are rejected; use explicit geometry settings and the outer view style for layout.
This keeps uniform-list measurement and accessibility semantics reliable.
Popup styles support Base and Hovered; option styles additionally support Focused
(the native highlighted option), Pressed, Selected and Disabled; empty styles
support Base. Option precedence is base, committed Selected, highlighted Focused,
then pointer hover/press; disabled options suppress pointer states and apply
Disabled. The legacy outer Selected style still applies below explicit option
Selected refinements. Empty collections show the configured label and never
fabricate a selection.

Appearance is a separate native configuration. The reconciler resolves theme
colors and sends an update only when the resolved appearance changes. Such
updates retain the node/handler, open state, keyboard focus and active choice;
row-height or viewport-size changes reveal the active option as needed. The
native test checks actual active/selected/disabled colors, row height and popup
width during an open appearance change, plus localized macOS accessibility text.

General nested overlay-scope integration remains OCH-11 work. This adapter
requires no additional native dependency or vendored patch.

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
Selected refinements apply to committed selected radio/select options. Base
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

Capability bit 16 advertises simple controls; bit 32 advertises stable choices;
bit 64 advertises Select; bit 128 advertises choice appearance (current mask 255). Append-only tags:
checkbox/switch kinds 5/6; Set_control operation 8; Control variants button 0,
checkbox 1, switch 2; check states unchecked/checked/indeterminate 0/1/2. Each
control's final Boolean is disabled. Semantic style states occupy 4 through 7.
Radio-group kind 7, Select kind 8, Set_choice operation 9 and Choice event 13 extend that schema.
Set_choice_appearance operation 10 carries geometry, localized text and three
bounded style lists. Nested strings, shadows and style records count against
retained-tree budgets; invalid appearance updates roll back atomically.
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
