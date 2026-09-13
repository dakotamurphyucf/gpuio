# Native controls (OCH-11, in progress)

The controls add `View.checkbox`, `View.switch`, `View.radio_group`, `View.select`,
`View.combobox`, `View.tooltip`, menus, command palettes, shared commands, focus scopes,
dialogs/popovers and disabled buttons to both the pure action API and the Bonsai
effect API. The rest of OCH-11 remains in progress: progress/notifications, pointer/drag interactions,
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

## Editable Combobox

`Gpuio.Combobox.Config.create` constructs a coherent single-line editor and choice
configuration with one label/disabled state. `View.combobox` is the pure view;
`Gpuio_eio.Combobox.create` supplies the stable Bonsai controller and revisioned
commands. Both use `Choice.Collection`, stable `Choice.Id` values and
`Choice.Appearance`. One controller has one placement in a window.

Rust owns query text, caret, selection, undo and IME composition. The application
owns its selected ID and complete supplied option collection. An activation emits
`Combobox.Selection.t`: a stable ID and the exact native editor snapshot. It
neither changes the selected application ID nor replaces the query. Observations
are never replacement commands. `replace_if_unchanged` can replace the query after
accepting an intent, guarded by that exact editor lease/revision; later typing is
preserved through `Stale_revision`, and a remount returns `Stale_editor`.

The default `Substring` filter compares Unicode lowercase strings, retaining
collection order and disabled rows. It does not normalize Unicode or fold accents.
Selection can remain outside the filtered visible rows. `Unfiltered` displays the
supplied collection, permitting application-specific or remote search. The
application owns ordering/cancellation of its asynchronous results. A selected ID
must still belong to the complete supplied collection; include it when building
remote result collections. This is not an automatic server-query versioning API.
Filtering caches are invalidated on query/configuration/filter changes, bounded by
the existing collection and editor limits, and released with the editor.

The actual input retains the only Tab stop. Up/Down open or navigate suggestions,
Enter requests the highlighted enabled option, Escape closes, and Tab closes and
traverses normally. Left/Right/Home/End, selection and clipboard shortcuts remain
native text editing. Enter is not a free-text submission callback. Typing opens
suggestions; successful explicit replacement closes them until subsequent native
editing. Composition closes suggestions and retains composing key sequences in
the editor/OS input path. Selection activation during composition is rejected. Disabling/re-enabling rotates
the event handler generation while retaining the native editor, so a queued intent
from before disabling cannot activate the newly enabled control.

GPUI resolves key bindings before raw key listeners, so the adapter captures the
input's Enter/Escape actions at the Combobox boundary. This policy does not depend
on an asynchronous OCaml prevent-default reply. The actual input carries
`EditableComboBox` semantics (macOS `AXComboBox`), its value, expanded state and
native editor focus/set-value actions; popup rows retain shared option semantics.

`examples/combobox` demonstrates the public controller. Its `--self-test` exercises
actual bridge command acknowledgements, guarded replacement, undo and stale
unmount. The Rust `native_controls` test separately checks actual keyboard choice
activation and macOS native text-client composition/commit. These checks do not
claim physical IME candidate-panel or complete screen-reader certification.

## Protocol and checks

Capability bit 16 advertises simple controls; bit 32 advertises stable choices;
bit 64 advertises Select; bit 128 advertises choice appearance; bit 256 advertises
Combobox; bit 512 advertises focus scopes; bits 1024/2048 advertise overlays/placement; bit 4096 advertises tooltips (current mask 8191). Append-only tags:
checkbox/switch kinds 5/6; Set_control operation 8; Control variants button 0,
checkbox 1, switch 2; check states unchecked/checked/indeterminate 0/1/2. Each
control's final Boolean is disabled. Semantic style states occupy 4 through 7.
Radio-group kind 7, Select kind 8, Set_choice operation 9 and Choice event 13 extend that schema.
Combobox kind 9, Set_combobox_filter operation 11 (Substring 0 / Unfiltered 1),
and Combobox_selected event 14 (ID plus exact editor snapshot) extend the schema.
Combined nodes validate editor/choice label and disabled state atomically and
reserve the same native editor memory budget as an ordinary input.
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

## Focus scopes

`Focus_scope.create` declares native subtree policy; `View.focus_scope ~config`
and the Bonsai equivalent wrap arbitrary children. Defaults are untrapped,
no automatic entry, and restoration enabled. Trapping always requests entry.
Keyed mount/unmount determines lifetime; configuration and descendant updates
preserve the restoration target and native editor identity.

The newest mounted trap governs the window. Tab/Shift-Tab traverse eligible
painted controls and wrap inside it, skipping hidden/disabled descendants. Entry
uses the first eligible painted control or an empty scope's non-Tab root. Closing
restores the prior eligible focus when possible, falling back to an enclosing
scope or window root. Traversal never temporarily focuses an outside editor.
Explicit outside editor focus returns `Focus_blocked`; native pointer and
accessibility activation/focus obey the same gate. Programmatic text replacement
remains available for an unfocused editor. Removing a scope releases its handles;
focus repair schedules a frame only after a retained-tree update, with no new
permanent polling loop. Hiding a scope with styles is not unmounting it: applications
must unmount closed modal content to release its trap.

Wire kind 10 and operation 12 (`Set_focus_scope`) carry three Boolean policies;
editor command error 10 is `Focus_blocked`. Independent `focus-v1-request.hex`
and `focus-v1-events.hex` fixtures cover both languages. Actual local macOS native
controls tests cover nested traps/restoration, empty scopes, reordered and hidden/
disabled descendants, blocked command/accessibility requests and cleanup. This
primitive is also the foundation for the dialog/popover surfaces below.

## Dialogs and popovers

`View.dialog ~config ~on_dismiss content` uses an optional ordinary view as its
content. `None` closes and unmounts the modal subtree. `Some content` mounts a
native trapped scope on a centered, viewport-sized backdrop. `View.popover`
additionally takes `~anchor`; that anchor remains mounted while optional panel
content opens and closes. Popovers enter focus without trapping it. Both restore
eligible previous focus on close and support arbitrary styled child views,
including native editors and choices. `Overlay.Config.create` validates the
accessible label and desired width (logical pixels); Escape dismissal defaults
to enabled, outside-pointer dismissal to disabled. Enable outside dismissal
explicitly for dismissible popovers. The panel uses theme background/foreground/
muted tokens by default; `~style` customizes it.

`Overlay.Dismissal.Escape` and `Outside_pointer` request closure. They do not
silently modify the application's state or remove the trap. The application
handles the request and renders `None`; queued requests for an old mount are
rejected by node/handler generation checks. Callback and dismissal-policy changes
are checked against the latest accepted view. There is no asynchronous
prevent-default round trip.

Overlay placement uses current-frame prepaint geometry, including scrolling and
anchor movement. Modal surfaces occupy the viewport without contributing layout
space to their mounting parent. Popovers flip/clamp using the shared popup
positioner. Native mount order determines stacking: a child Select/Combobox popup
paints above its containing surface, and a later nested dialog paints above both.
The current top overlay receives dismissal. Popup bounds belonging to descendants
count as inside the overlay even when they extend outside its panel. Native child
widgets consume their own Escape before an overlay sees it; marked-text Escape
ends composition without also dismissing the overlay. Backdrops block underlying
pointer focus and wheel routing. Panels carry Dialog accessibility semantics;
modal panels additionally expose the modal flag.

Capability 1024 adds optional `Set_overlay` operation 13 on a
focus-scope node and `Overlay_dismissed` event 15. The configuration carries
Dialog/Popover kind, bounded label, desired width and two dismissal policies.
Native validation checks kind/policy invariants and charges configuration/string
storage to the retained-tree budget. Removing metadata or the scope releases that
storage and its per-window placement/focus records. Independent overlay fixtures,
OCaml callback/lifetime expectations and native transaction checks cover this
contract. Local macOS control-window checks exercise nested dialog restoration,
child choice activation beyond panel bounds, native marked-text Escape, and a
moving popover anchor. Full screen-reader and Linux GUI acceptance remain separate
validation work.

Run `./scripts/gpuio exec dune exec examples/overlays/main.exe` for the public
Bonsai/Eio example. Add `-- --self-test` for actual native mount/editor-command,
modal focus-denial, stale unmount, popover and shutdown checks. Keyboard, pointer
and macOS text-client dismissal checks are separately in `native_controls`.


`Placement.create ?side ?align ?offset ()` controls anchored surfaces. Sides are
Top/Right/Bottom/Left; cross-axis alignment is Start/Center/End. The default is
Bottom/Start with zero gap. Signed offsets are logical pixels, finite and bounded
to -16384..16384; negative offsets permit overlap. Pass a placement to
`Overlay.Config.create ~placement`. Centered dialogs ignore anchored placement.
The native positioner prefers the requested side, flips when the opposite side
fits or offers more space, and clamps both axes to the current viewport. Position
updates retain the overlay, editor and focus identity. Capability 2048 and appended
`Set_placement` operation 14 carry optional placement metadata; previous overlay
records and fixtures remain unchanged. `placement-v1-request.hex` independently
checks the new operation in both languages.

## Tooltips

`View.tooltip ~config ~anchor ~content ()` accepts arbitrary retained views for
both anchor and content. `Tooltip.Config.create ~label ()` defaults to native
managed visibility, 250 ms hover opening, 80 ms delayed closure, a 300 ms shared
per-window grace interval and Top/Center placement with a six-pixel gap. Focus
within the anchor opens immediately. Moving into hoverable content cancels delayed
closure. Escape and pointer-down on the anchor dismiss; unchanged hover/focus does
not immediately reopen it. Native editors consume composition Escape first.

`Tooltip.Open_state.Managed { initially_open }` reads the initial Boolean on mount
and keeps subsequent transient state in Rust. `Controlled open_` follows the
accepted application value; optional `~on_open_change` reports requests. A
controlled request never changes visibility before the application accepts it.
Callback-only updates need no wire transaction; disable/re-enable rotates handler
generations to reject queued requests. Disabling the tooltip suppresses its
surface and accessible description without disabling its anchor. Switching from
Controlled to Managed preserves current visibility.

Content remains in the retained tree while closed, preserving Bonsai models and
native editor buffers, undo history and revisioned controller identity. Closed
content does not paint or appear in the accessibility tree; its editors cannot
autofocus or receive explicit focus commands. Focus scopes inside closed content
are inactive. Opening does not itself steal focus, but an explicit child focus
scope may request entry. Unmounting the tooltip disposes its children and makes
old editor controllers stale. `hoverable=false` disables pointer interaction with
the panel; applications should use noninteractive content for that mode.

The panel uses the normal style vocabulary, inherited text styling, native state
refinements and theme background/foreground/muted tokens. The anchor owns its own
style. Placement uses the same current-frame flip/clamp implementation as popovers.
Descendant tooltip bounds count as inside their containing popover for outside-click dismissal.
The panel exposes Tooltip semantics; anchor descendants expose the tooltip label
as their accessible description even while the panel is closed. Tooltip-owned
show/hide tasks and focus subscriptions are cancelled on unmount. No additional
permanent polling loop is introduced.

Capability 4096 (current mask 8191) adds structural kind 11, `Set_tooltip` operation
15 and `Tooltip_open_changed` event 16. The node has exactly two children; config
label/width/delays are validated and charged to retained memory. Both languages
independently verify `tooltip-v1-request.hex` and `tooltip-v1-events.hex` alongside
previous fixtures. Native checks cover keyboard and pointer triggers, controlled
visibility, hidden focus traps/editors, native identity retention, timer disposal
and macOS accessibility exposure. Full screen-reader and Linux GUI acceptance
remain separate work.

`examples/tooltips` demonstrates managed information and an application-controlled
editable note. `--self-test` exercises retained editor commands and visibility
through the public Bonsai/Eio bridge.

## Shared commands and shortcuts

`Command.Id` names an application action. `Command.create` stores its label,
optional checked state, enabled state, shortcuts and OCaml callback;
`Command.native` selects Copy/Cut/Paste/Select_all/Undo/Redo instead. A
`Command.Registry` belongs to a `View.command_scope`. A
`View.command_button ~command ()` obtains its label and behavior from the nearest
registry defining that ID. Nested registries shadow outer definitions, including
when the inner definition is disabled. A reference with no enclosing definition
is rejected before submitting the tree; native transaction validation also checks
references against the final parent graph atomically.

Callbacks execute as Bonsai effects on the OCaml UI domain. Rust keeps only
serializable command metadata and sends the owning scope, handler, command ID,
generation and source through the bounded event bridge. The reconciler refreshes
callback closures without replacing native nodes or publishing a transaction when
metadata is unchanged. Disable/re-enable, removal/reintroduction and changing the
target invalidate old command generations. Labels, checked state and shortcuts
can change without replacing command identity. Native dispatch and OCaml delivery
both reject stale or disabled commands.

`Shortcut.create` describes one key chord. `Primary` maps to Command on macOS and
Control on Linux; explicit Control/Alt/Shift/Super are available. Matching uses
GPUI's keyboard-layout-aware key matching. `Native_first` is the default: native
editor actions, Tab traversal and button Enter/Space activation retain priority.
`Override` intercepts before those actions. `Modified_only` preserves ordinary
editor typing; `Always` and `Never` explicitly change text-input behavior.
Composition suppresses shortcuts unless `during_composition=true`. Resolve scopes
from the focused element outward, then use registry declaration order for chord
conflicts. Disabled inner commands do not fall through to an outer definition of
the same ID. A rejected invocation does not consume the keystroke. Multi-stroke
sequences are not part of this initial shortcut API.

Native editing actions use the focused editor, or the last eligible editor when
focus has moved to a command button. They restore that editor's focus and dispatch
the native edit action without an OCaml editing round trip. Hidden, disabled or
modal-blocked editors cannot become targets. Copy requires a selection; Cut also
requires an editable buffer; Select_all requires nonempty text. Paste/Undo/Redo
require an editable buffer. Undo/Redo may be no-ops when history is empty; this
adapter does not yet expose history availability. Ordinary buttons expose Button
semantics, enabled state and accessibility press; commands with a checked value
also expose their toggle state.

Each registry is limited to 1024 unique IDs, four shortcuts per command and
256 KiB aggregate text. Registries count against native retained-tree budgets.
The window owns one removable keystroke subscription, filtered by GPUI window
identity; dynamic commands never append to or clear the application's global
keymap. Unmounting scopes removes their metadata, callbacks and references.
Closing a window releases its subscription. Capability 8192 (mask 16383) appends
kinds 12/13, operations 16/17 and event 17. Independent OCaml/Rust fixtures cover
all shortcut policies, native edit targets and event sources.

`examples/commands` demonstrates stateful Bonsai callbacks, shared command buttons,
nested shortcut scopes and a native Copy button. Its `--self-test` checks public
registry mounting, render acknowledgements, disable/re-enable and scope removal.
Actual native tests separately exercise key dispatch, native-first/override
priority, composition, editing targets, accessibility activation, nested shadowing
and two-window isolation. Menus use the same registry as described below. The
command palette remains OCH-11 work; its reserved event source does not imply
that component is implemented.


## Menus

`Menu.create ~label items` constructs a validated immutable menu. Items are
`Menu.Item.Command id`, `Separator` and `Submenu menu`. Command references obtain
labels, enabled state, optional checked state and actions from `Command.Registry`;
there is no second callback or shortcut registration mechanism. Define referenced
commands in an enclosing `View.command_scope`, including commands that a focused
inner scope may override. Missing references are rejected on both sides of the
bridge, including after reparenting a physically shared OCaml view.

Three presentations reuse this definition:

- `View.menu_button ~menu ()` renders a dropdown trigger.
- `View.context_menu ~menu child` wraps an arbitrary view. Right-click opens at
  the pointer; Shift-F10 opens from keyboard focus within the child.
- `View.menu_bar menus` uses the native application menu on macOS and an
  in-window bar on Linux. `~platform:false` renders an in-window bar on either OS.
  At most one platform bar may be mounted per window; multiple ordinary bars and
  dropdowns are allowed. An empty bar is valid.

Dropdowns, context menus and ordinary in-window bars resolve references at their
location in the view tree. The native macOS application menu resolves commands
from the focused scope outward, falling back to the menu's enclosing registry.
A disabled inner definition shadows an enabled outer definition. The active
window owns the application menu; opening an inactive window does not replace it.
Activation updates ownership without waiting for new OCaml state. Closing an
active window restores the surviving window's menu. Hiding or unmounting the bar
clears it, and the last window releases the installed menu definitions.

Rust owns open state, active row, submenu path, prefix search, scroll position and
focus restoration. Navigation does not require an OCaml render. Up/Down/Home/End
skip separators and disabled items; Right opens a submenu, Left returns, and
Enter/Space activates. Escape closes the menu before an enclosing popover can
receive dismissal. Tab closes the menu and resumes ordinary focus traversal.
Pointer hover opens nested menus and pointer/accessibility activation uses the
same command route. Context menus restore the previous eligible focus on Escape
or activation; outside clicks close without stealing the destination's focus.
Native editing commands restore the retained editor and run on the GPUI thread.

Popup positioning uses current-frame trigger/row geometry, preferred bottom/right
placement, edge flipping and viewport clamping. Every submenu panel participates
in its enclosing overlay's hit region, even where it extends beyond that panel.
Overlay priority reserves space for all eight nested menu levels. Lists virtualize
uniform rows; wheel scrolling remains independent of the keyboard highlight until
navigation requests a new reveal. Only visible row anchors and open-depth scroll
handles are retained, and closing releases them. Updating the menu definition
closes transient navigation; changing command metadata refreshes rows without
replacing the menu definition.

`Menu.Appearance` shares `Choice.Appearance`: popup width, row height, maximum
visible rows, localized empty label, and popup/option/empty styles, resolved
through the OCaml theme. Focused row styling marks the navigation target; Selected
styling marks checked commands. Native OS menu styling is controlled by macOS;
these appearance settings apply to GPUI-rendered menus. Semantic output includes
MenuBar/Menu/MenuItem roles, labels, checked/disabled state, expanded submenus,
active descendants and accessible activation. No full screen-reader acceptance
is inferred from the targeted accessibility checks.

A definition is bounded to eight levels, 1024 aggregate items and 256 KiB of
labels/command IDs; a bar has at most 32 top-level menus, with collective bounds.
Labels are nonblank UTF-8 without NUL and at most 4096 bytes. Native decoding
checks recursive limits before allocation. Definitions count against retained-tree
budgets. Installed native actions carry window, source node and command identity;
dispatch revalidates lifetime, availability and current command scope, so queued
actions cannot target a closed window or an obsolete focused scope. Native edit
availability reads borrowed editor text/selection rather than copying the buffer
on every menu refresh.

Capability 16384 (combined mask 32767) adds kind 14 and `Set_menu` operation 18.
The previously reserved `Command_source.Menu` now carries its source node ID on
both sides of the still-unreleased protocol. Independent `menus-v1-request.hex`
and updated command event fixtures verify the encoding.

`examples/menus` demonstrates a platform bar, dropdowns, a context-wrapped editor
and nested command registries. Its public self-test checks mounting, native render
acknowledgements, metadata changes and scope removal; actual input behavior is
validated separately in `native_menus` and the combined `native_controls` scenario.
Targeted macOS checks cover NSMenu activation, accessibility activation/state,
right-click Copy, 1000-entry scrolling/navigation, popover hit routing, focused
scope changes, active-window replacement and stale actions. Final whole-ticket
CI and Linux graphical acceptance are not implied by these local checks.

Current presentation limits: registry shortcuts execute through the shared native
matcher, but menu rows do not yet show accelerator labels, and macOS items do not
install OS key equivalents or responder-selector associations. This avoids adding
mutable global key bindings that could bypass the declared native-first and IME
policies. The command palette and broader desktop services remain separate work
within OCH-11.


## Command palette

`View.command_palette ~config ~on_dismiss ()` mounts a modal native search session.
`Command_palette.Config.create ~label ~commands ()` accepts unique, ordered
`Command.Id.t` references resolved through the enclosing command registries.
Command labels, enabled/checked state, generation and implementation stay in the
registry; the palette does not duplicate command callbacks or own application
state. The [public Bonsai/Eio example](../../examples/palette/main.ml) opens the
chooser with a command button or Primary-Shift-P.

The application controls whether the view exists. Mounting opens the session;
Escape, permitted outside pointer input, or choosing a command closes it natively
and restores eligible prior focus. `on_dismiss` receives `Escape`,
`Outside_pointer`, or `Selected id` and should remove the view. A closed view
remains closed across metadata updates: unmount/remount it, or give the next
session a new key. Hiding the palette or an ancestor ends the session with
`Escape`; application unmount itself does not emit a dismissal. This differs from
an application-controlled dialog, whose trap persists until accepted unmount.

Rust owns a private GPUI Base input entity, query text/composition/undo history,
highlight and native virtualized result list. Search or navigation does not send
an event through OCaml. Unicode-lowercase, whitespace-separated query terms must
all occur in the command label or ID; results retain declaration order. This is
substring matching, without fuzzy ranking, accent folding or Unicode normalization.
Up/Down and PageUp/PageDown navigate enabled results; Home/End/Left/Right remain
text-editing keys. Enter executes the current enabled result. During native
composition Enter does not execute; Escape first clears marked composition.

The query is not an application document editor and cannot become the target of
a registry native-edit command. Opening captures the eligible application editor;
choosing a native editing command closes the palette and restores focus before
revalidating/performing that command. Existing modal scopes continue to constrain
that target. Callback commands enqueue `Command_invoked` before the palette's
`Selected` dismissal, preserving invocation-before-removal ordering. Selection
checks current query, registry, generation and membership even before another
paint; disabled or stale painted actions cannot execute a superseded command.
Ordinary shortcut matching retains the shared editing/composition priority rules.

`Command_palette.Appearance` aliases `Choice.Appearance`. Popup geometry, uniform
row height, maximum visible rows, empty label, and popup/option/empty styles reuse
that bounded contract. Defaults are a 560-pixel popup and eight visible rows;
geometry clamps to the current viewport. Option Focused styles mean the highlighted
result; Checked reflects registry state. Hovered/Pressed apply to enabled pointer
rows. Popup Hovered and ordinary root Focused/Hovered/Pressed styles are supported;
root Focused includes focus in the query. `Pointer_events false` prevents query/row
pointer actions and outside dismissal while preserving keyboard and accessibility
operation. The modal surface shields background wheel and pointer input.

The palette exposes a modal Dialog, editable ComboBox query, ListBox results and
Option semantics with labels, disabled/checked state and active descendant.
Accessibility Focus/SetValue operate on the bounded query; Click performs the
same validated command route as keyboard and pointer activation. Labels and
placeholder refresh with configuration. Actual platform role mapping remains
GPUI/AccessKit-owned (macOS represents the result option as `AXStaticText`).

Limits: 1024 unique command references, 256 KiB aggregate metadata, 4096 UTF-8 bytes
per label/placeholder and query, and 64 KiB of query undo history. Labels must be
nonblank; strings reject NUL. Accessibility query replacement also rejects line breaks. Visible
rows are virtualized, and removing the view releases its query entity,
subscription, scroll state and row anchors. The protocol adds kind 15, operation
19, event 18 and capability bit 32768; existing wire tags retain their meaning.

Local verification and platform limits are recorded in the
[palette evidence report](../evidence/native-palette-och11.md). This component does
not complete the remaining OCH-11 families or OCH-12 animations.
