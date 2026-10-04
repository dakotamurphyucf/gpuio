# Production bridge implementation contract (encoding family v1, epoch 3)

Implementation contract for OCH-7. Hosted acceptance is recorded in Linear. This is separate from the private foundation smoke
protocol. No released protocol compatibility is promised before the first API
release. OCH-8 extends the initial style tags with paired
OCaml/Rust definitions and an independent full-field fixture.

## Exact version boundary

The current handshake requires **epoch 3** in both runtimes. Epochs 1–2 and unknown
future epochs are rejected before tree operations. The legacy positive signed
capability mask is full (bits 0–62); epoch 2 adds mandatory Op69
`Set_button_presentation` for rich button content, loading and focus policy.
Epoch 3 additionally defines Event67 `Menu_open_changed` and permits a handler
and root placement on MenuButton nodes. Other menu presentations retain their
previous contracts. The same unpublished epoch now includes mandatory Op70
`Set_split_button (NodeId, Config option)` for validated composition and native
hover/menu-held paint. `None` removes coordination without replacing the node.
The same unpublished epoch extends Op60 Link configuration with a loading Boolean
after disabled, supporting focus-preserving busy links and stale activation fencing.
It also adds mandatory Op71 `Set_hover_observer` and Event68 `Hover_changed`
for a separate generation-checked Button/CommandButton/Link hover subscription.
It now also defines Kind53 `Choice_picker` and Op72 `Set_choice_picker` for
atomic admission of the checked presentation and role-wrapped children. Event69
`Choice_picker_event` carries its ordered selection/open/visibility/query values
with window, node, handler and tree-revision identity. The rendered picker is
still under implementation; this is not a new advertised capability bit.
The same unpublished epoch appends native-editor configuration operations:
Op73 `Set_editor_privacy`, Op74 `Set_editor_frame`, Op75
`Set_editor_content_hint`, Op76 `Set_editor_format`, Op77
`Set_editor_validation`, Op78 `Set_text_area_layout` and Op79
`Set_editor_clear_on_escape`. These preserve the
legacy editor-config payload and retained draft identity. See the
[plain-input contracts](plain-input-extensions.md), [edit filtering](input-validation.md)
and [text-area layout](textarea-layout.md) for mode restrictions, bounded admission
and meaningful local validation. Content-hint status additionally appends private
editor command tag 7 and result tag 2; see [content hints](input-content-hints.md).
[Viewport commands](editor-viewport.md) append editor command tags 8/9 and
result tags 3/4 for last-layout metadata and accepted scroll requests.
Both runtimes must be rebuilt together; none of these additions changes the
release qualification status or allocates another capability bit.
This extends the unreleased paired implementation, not a published wire epoch.
See [menu observations and split coordination](menu-observation.md).
No bit is reused and the sign bit is not a capability. Existing operation tags
retain their numbers; Op60 has the paired payload extension above; the historical Rust `v1` module and this filename name the
encoding family, not a frozen compatibility implementation. Both runtimes ship
in one release. See [button content](button-content.md) for ownership and event
fences; stored application data has a separate compatibility contract.

## Ownership and identity

One native runtime owns its windows. Each window owns a retained tree. Window,
node, handler and resource identities are distinct `(slot, generation)` types.
Slots fit unsigned 32 bits; generations are 1 through 2^32-1, never wrapping.
New node slots are contiguous; freed slots can be reused only with the next
generation. Tombstones contain no text, child arrays or widget resources.
Events must match both the window generation and current node/handler binding.
Changing a callback closure without replacing its binding may retain the handler
ID; dispatch uses the currently committed OCaml callback registry.

## Schema and transaction semantics

Owned bin_prot messages use declaration-order variant tags. Hello requests an
exact protocol version and a required capability mask. Unsupported versions or
capabilities fail explicitly. Typed assets/documents, static native extensions
and retained canvas scene registration have separate capability bits. Canvas
registration alone does not advertise a rendered canvas widget. Extension payloads
require a registered schema and its bounded package-specific validation.
Managed trees advertise bit 39 and managed tables bit 40, separately from the
original retained-view-tree bit. Desktop integration, OS notifications and charts
use bits 41–43. Extended cursor values and start ellipsis require bit 44
(`CAP_STYLE_VALUES`); general input regions require bit 45 (`CAP_INPUT_REGIONS`).
Explicit pointer/wheel occlusion requires bit 46 (`CAP_POINTER_OCCLUSION`) and
appends style field 66 with native-default/pointer/pointer-and-scroll values 0–2.
Atomic ordinary text foreground spans require bit 47 (`CAP_STYLED_TEXT`) and
append Op59 `Set_styled_text`. Composed passive-content links require bit 48
(`CAP_LINKS`), Kind51 `Link` and Op60 `Set_link`; their root owns one action/focus
target. Solid/dashed border patterns require bit 49 (`CAP_BORDER_STYLES`) and
append field 67 `Border_style` with values 0/1. Native proportional layout requires
bit 50 (`CAP_ASPECT_RATIO`) and appends field 68 `Aspect_ratio` with a validated
float width/height ratio. Multiplicative native animation opacity requires bit 51
(`CAP_OPACITY_FACTOR`) and appends animation property tag 11 `Opacity_factor`,
a finite factor in [0,1] mutually exclusive with absolute opacity in one target.
Mounted command binding observations require bit 52 (`CAP_COMMAND_BINDINGS`),
Op62 `Set_command_binding` and event 66 `Command_binding_observed`. Atomic numeric
draft mount seeds require bit 53 (`CAP_NUMBER_INPUT_DRAFT`) and Op63
`Set_number_input_draft (node, string option)`. The seed is bounded to 4096 UTF-8
bytes without NUL/CR/LF, independent of the normalized committed value. Native
creation consumes it; retained editors ignore seed changes. None restores the
default formatted-value seed, while Some empty text is an explicit empty draft.
The paired Core adapter sends the seed only when creating a numeric node.
Inherited disabled subtrees require bit 54 (`CAP_DISABLED_SUBTREES`) and append
Boolean style field 69 `Disabled`. It is base-only and preserves accessible
roles/names/values while denying native input/focus and accessibility actions.
An ancestor's true declaration cannot be overridden by a descendant's false.
See [disabled subtrees](disabled-subtrees.md) for ownership and popup semantics.
Atomic grid placement requires bit 55 (`CAP_GRID_LOCATION`) and appends style
field 70 `Grid_location`. Its fixed record order is column start/end, then row
start/end; each edge is Auto tag 0, signed Line tag 1 with an int64, or Span tag 2
with an int64. Lines are nonzero in -1025..1025; spans are 1..1024. Both axes are
replaced together, including in interaction states. Malformed endpoints are
rejected during native decode and again before direct transaction publication.
Rich avatar fallback requires bit 56 (`CAP_AVATAR_FALLBACK`): one bounded passive
child is selected natively while the avatar's primary image is unavailable.
`Set_avatar` keeps its existing record; ordinary child splices attach the slot.
See [ownership, admission and placement](avatar-fallback.md).
Independent Rating colors require bit 57 (`CAP_RATING_APPEARANCE`) and Op64
`Set_rating_appearance (node, appearance option)`. The record has optional active
and inactive RGBA int64 values in 0..0xffffffff; None resets inheritance. Theme
resolution stays on the OCaml side. See [rating appearance](rating-appearance.md).
Custom spinners require bit 58 (`CAP_SPINNER`) and Op65 `Set_spinner`. It atomically
sets a Loading node's label, animated flag, period, easing and optional SVG source.
The legacy loading setter clears spinner and image state; an independent image
setter is invalid on Loading nodes. A spinner owns one progress-indicator label
and uses queued image observations for its decorative icon. See
[custom spinner ownership and timing](custom-spinner.md).
Extended progress requires bit 59 (`CAP_PROGRESS_PRESENTATION`) and Op66
`Set_progress_presentation`, carrying the existing semantic payload followed by
shape and transition. Circle nodes may contain ordinary keyed children; linear
nodes remain leaves. The legacy setter restores linear/immediate presentation;
shape and child changes are validated against the complete resulting tree.
See [progress presentation](progress-presentation.md).
Checkable indicator appearance requires bit 60 (`CAP_CONTROL_APPEARANCE`) and
Op67 `Set_control_appearance (node, appearance option)`. Only Checkbox, Switch, Radio
and RadioGroup accept it. None restores legacy geometry and inherited paint;
semantic values, handlers and native focus remain separate. See the
[control appearance contract](control-appearance.md).
Rich checkable labels require bit 61 (`CAP_CONTROL_LABELS`). Existing Create and
Splice operations carry passive checkbox/switch labels and keyed radio/tab slots;
final-tree validation checks shape, accessible names and descendant policy.
There is no new opcode. TabBar accepts either no children or one Container slot
per configured choice, each with zero or one passive label subtree. Child-only
updates revalidate the owner. See [rich control labels](control-labels.md) and
[rich tabs](rich-tabs.md).
Standalone Radio (Kind52/Control3), semantic Radio_group (role15) and checkable
Tab ordering (Op68 `Set_tab_order`) require bit62 (`CAP_CHECKABLE_NAVIGATION`).
Checked standalone activation is a no-op; managed groups keep their asynchronous
selection-request contract. See [checkable navigation](checkable-navigation.md).
Bit62 exhausts the positive signed mask; further extensions require an explicit
protocol/version design.
Both language halves require the current shared mask `9223372036854775807`;
an older host fails capability negotiation instead of accepting
unsupported input, commands or style values. Existing style field tags and value
IDs are unchanged: cursor additions occupy 10–21 and start ellipsis occupies 2.
Correlated open/close/frame requests are distinct from per-window transactions.
Acceptance and rendering are distinct events; rendering does not assert physical
screen presentation. The client submits one transaction per window at a time.

OCH-41's experimental text shimmer appends Op61 `Set_text_shimmer` with a node
and optional validated configuration. It decorates ordinary Text; `None` clears
without replacing the node or its source/spans. Native admission limits active
sources to 16,384 UTF-8 bytes and reserves 1,024 payload bytes per declaration
within existing window/session budgets. Core/Bonsai reconciliation, atomic
retained-tree admission and mounted rendering are implemented. The experimental
configuration now includes optional concrete appearance colors/mode; its standalone
bound is 64 bytes. Capability advertisement remains pending public acceptance. See [the contract](text-shimmer.md). Existing tags
and the current capability mask remain unchanged during this development stage.

OCH-41's highlighting integration appends Kind50 `Highlight_scope`, Op57
`Set_highlight_scope` and Event64 `Highlight_observed`. Configuration and optional
observer bindings are validated; each observation carries a positive scope-local
epoch plus typed state. Ready contains up to 16 total/stored counts, bounded before
decoding allocation and checked against the bound configuration. These additions
are under development: no highlighting capability is advertised until scheduling,
painting and public native acceptance are complete. See the
[highlighting contract](subtree-highlighting.md).

Create chooses immutable node kind. Text/style/binding updates preserve identity.
Style lists replace the previous list, so absent properties reset; repeated
refinements compose in order, last property wins. Splice uses the current child
array's zero-based offset and deletion count. Remove removes exactly one node:
the same atomic batch must detach it and remove or reparent its descendants.
The final tree must have exactly one root and one parent per other node, with
no cycles, dangling edges or unreachable nodes. An empty tree has no root.

The native implementation stages changed slots in an overlay. Validation failure
publishes no mutation, generation change or revision. Successful commits advance
exactly one revision. Text/style/binding edits do not walk unrelated history;
structural changes currently validate the complete final tree iteratively.
Untouched text, style and child arrays remain shared immutable allocations.
Structural splice cost includes copying the changed parent's child array; this
is not a claim of constant-time child insertion.

## Bounds and validation

Initial limits: 1 MiB encoded message, 256 KiB text field, 4096 operations,
128 style refinements per node, 100,000 slots per tree, depth 128, and 32 windows.
Decode rejects invalid tags, malformed UTF-8, trailing bytes and non-finite
numbers. Container allocation is bounded by both the declared limit and the
remaining input. Domain validation additionally checks sizes, colors, IDs,
revisions and structural invariants. Public callers must not bypass validation
with generated deserializers. Retained variable-size payloads are limited to
64 MiB per window and 256 MiB per session. Text, styles and child arrays count
against this budget; fixed slot metadata is separately bounded by slot/window
counts. The staging overlay shares untouched payloads, and each operation checks
its prospective payload budget before the batch can publish. Peak memory also
includes the old changed payloads, bounded decoded commands and staging metadata.

The mailbox holds at most 64 commands / 4 MiB encoded command backlog, with 128
response reservations and 128 ordered input/observation events. Decoder count
limits separately bound the in-memory expansion of encoded commands. Admission
reserves a response before accepting a command; Busy means the caller retains
its desired state and retries after draining output. A window's transaction slot
is released when its Accepted/Rejected response is drained. RequestFrame retains
its reservation until a real paint callback or an explicit close failure.

Only consecutive render observations for the same window coalesce; responses
and input events form ordering barriers. Input overflow disables that window's
input/transactions and emits a reserved Overloaded event; close/reopen recovers it.
There is room for 32 overload and 32 native-close notifications, plus terminal
Stopped. Reusing a window slot waits for its old output to drain. Native close
requests and emergency abort do not depend on normal command capacity. Stopped
cancels any outstanding requests, including commands still queued when the OS
closes the last window. Shutdown and closed runtimes reject further submission.
An event envelope contains at most 256 events and preserves wire order.

The initial style subset is native-owned; OCaml will resolve public theme tokens
in OCH-8. Unresolved wire Color.Token values currently fail UnsupportedCapability
instead of substituting a color. Full style parity belongs to OCH-8.

## FFI and native host

`gpuio.native` is a native-code-only OCaml library, linked from a Dune-built Rust
archive. Its opaque handle identifies an explicit transport instance; a bounded
registry holds at most eight handles and GPUI permits one active application.
`run` owns the OS main thread, releasing the OCaml runtime lock while GPUI runs.
`submit` and `drain` run on the OCaml UI domain. Each window has an independent
retained tree and generation, and GPUI owns native element state.

Commands use a bounded asynchronous wake channel. Native callbacks enqueue owned
events and notify a duplicated nonblocking Eio pipe. No callback synchronously
enters OCaml. Encoding/decoding/tree work occurs outside the mailbox mutex.
The caller keeps the pipe reader alive through native exit and worker join, then
disposes the handle. Emergency abort wakes GPUI even when the command queue is
full. Registry admission/disposal cannot race application startup. Exported Rust
panics become OCaml exceptions; application logic must preserve its backtrace,
abort the native host on worker failure, join workers and dispose resources.

## Validation

Independent OCaml/Rust fixtures cover requests and ordered event envelopes,
including Unicode, integer width boundaries, optional handles and styles.
Truncation, unknown tags, invalid identities, allocation bombs and trailing bytes
are rejected. Tests cover structural/text rollback, cycles, duplicate parents,
invalid styles, stale generations, close/reopen, response reservation, queue
pressure, coalescing barriers, overload, terminal stop and retained cleanup.
A deterministic randomized structural reference model checks both accepted and
rejected batches, alongside a separate repeated-text-update model.

`rust/native/tests/allocation.rs` measures a single edit in an isolated test
process. One local macOS run on 10,001 nodes / 10,400,000 retained payload bytes
allocated 2,776 bytes in four allocations and took 5 microseconds. It touched one
record and scanned zero unrelated nodes. Timing is an observation, not an SLA;
the test enforces bounded allocation and touched-record behavior.

`examples/bridge/main.exe` exercises the actual production FFI/GPUI path: protocol
negotiation, two windows, 50 accepted/rendered revisions, rollback of an invalid
batch, continued use after closing one window, and panic containment without
poisoning the registry. It emits `PRODUCTION_BRIDGE_PASS` after clean shutdown.

Public views/reconciliation and Bonsai/Eio scheduling belong to OCH-8/OCH-9.
macOS native validation is the development gate; Linux builds/tests remain
required and graphical checks are informational. Hosted results and any remaining
acceptance gaps belong in the live ticket, not inferred from compilation.

## Typed style extension (OCH-8)

Original Style tags 0–18 remain in order. Tag 19 is Fields, a bounded list of
Field refinements; tag 20 is State(state, fields), where 1=focused, 2=hovered and
3=pressed. States are not recursive. Base refinements use Fields. Every Field
tag is fixed by the paired declaration order and exercised by the independent
`style-v1.hex` fixture. Fill represents solid colors or a two-stop gradient;
Shadow has color, offsets, blur, spread and inset. No unreleased schema change
is a promise of compatibility with an independently installed older host.

There are at most 128 fields per refinement and eight shadows per field. Native
semantic validation rejects invalid units, enums, non-finite values, unknown
state numbers and state-specific interaction policies. Theme tokens resolve to
RGBA in the OCaml adapter; unresolved tokens are rejected by the native host.
Nested field arrays, shadow arrays, font names and accessible labels count
toward retained payload limits, including inside state refinements. Repeated
state entries compose before creating one GPUI handler for each state.

See [typed UI](typed-ui.md) for public reset/inheritance, native interaction and
selection semantics. Owned resource handles remain reserved for the later
resource/component tickets; the pure style values contain no native pointers.

## Toolbar semantic metadata (OCH-41)

Accessibility role tag 14 is `Toolbar` carrying orientation tag 0 (horizontal)
or 1 (vertical). Existing role tags and config field order remain unchanged.
Only ordinary containers admit this role; it conveys no layout or focus model.
Command buttons supply their existing checked semantics and asynchronous actions.
The independent `accessibility-toolbar.hex` fixture is checked in both languages.
This is a paired experimental schema extension, with no new operation or
capability; older decoders reject the unknown role rather than supporting it.
See the [selection review](../catalog/selection-review.md) for behavior and
validation scope.


### Ordinary multiline editor search

Op80 `Set_editor_searchable (node, bool)` is admitted only on ordinary text areas.
Editor command10 `Search` contains Read0, Open1(bool replace), Close2,
Set_query3(string, case), Next4, Previous5, Replace_current6(stamp, replacement)
and Replace_all7(stamp, replacement). Case tags are Sensitive0/Ascii_insensitive1.
Stamps contain editor then search revision; native replacement checks both before
editing. Queries are bounded to2048 UTF-8 bytes before allocation; no NUL is valid.
Replacement text uses the existing262144-byte bound.

Result5 `Search_observed` carries bounded metadata. Result6 `Search_replaced`
carries the ordinary editor snapshot, search metadata and accepted count. Both
snapshots must agree on text revision/byte length; counts are in0..262144.
Errors11/12/13 append Search_unavailable/Stale_search/Not_editable. Existing config,
text snapshot and command/result tags retain their encodings. Search metadata
structure and lifecycle are specified in [the search contract](textarea-search.md).
Event70 `Editor_search_observed (window, node, handler, view_revision, metadata)`
uses the same search snapshot schema. Adjacent same-owner, same-view observations
coalesce only with nonregressing editor/search stamps; other events are barriers.
Dynamic query bytes and search-replacement reply text count against mailbox and
frame limits. See the search contract for opt-in, disable and lease semantics.


Search command tags8..11 append guarded `Close_and_focus activation`,
`Set_query_text query`, `Set_case case`, and `Toggle_case`. Close checks the exact
opening and restores focus only when the current document is eligible; hidden or
disabled documents still close. Query-only/case-only updates preserve the other
native field, and toggle acts on current native policy. Eio binds close to the
snapshot owner and validates the closed activation in its reply. The
[search contract](textarea-search.md) specifies the composed search bar and
remaining desktop acceptance.

### Segmented OTP presentation

Op81 `Set_otp_appearance (node, appearance option)` appends a bounded optional
presentation policy to the same unpublished epoch. It is valid only on OTP
nodes; `None` restores defaults. Existing OTP edit config, command and event
payloads do not change. The [presentation contract](otp-presentation.md) specifies
field order, units, validation and atomic retained-payload accounting. Appearance
changes preserve native editing state and retire previous layout geometry until
the next paint. Both runtimes must be rebuilt together; no capability bit or
synchronous callback is added.

### Numeric input presentation

Op82 `Set_number_presentation (node, presentation option)` appends an optional
numeric frame policy to the unpublished epoch. Its payload contains gap, button
width, button minimum height, stacked button minimum height, editor padding,
optional border width, then frame/editor/decrement/increment style lists in that
order. Dimensions are logical pixels. Each list is bounded to 16 entries; native
admission additionally validates paint-only fields, allowed states and at most
128 total declarations. See the [contract](number-presentation.md).

A present policy requires four plain structural slots (leading, trailing,
decrement, increment), each with at most one child. The first two admit ordinary
interactive views; the last two require passive content. Removing presentation
requires removing all numeric children in the same atomic transaction. Native
editing config and input state remain independent. Geometry changes cancel a
held gesture if its target moves. Both runtimes must be rebuilt together; this
adds no capability bit or synchronous callback.

### Numeric application-step requests

Numeric event tag 5 appends `Step_requested`: positive request id, direction,
source, then a valid noncomposing numeric snapshot. Numeric command tag 10
appends `Resolve_step`: positive request id, expected nonnegative numeric
revision, then optional value (`None` declines). Previous numeric tags retain
their encoding. Maximum numeric event/command byte budgets remain unchanged;
paired independent fixtures cover the new payloads and maximum draft size.

Request events are discrete mailbox boundaries. They do not advance the numeric
snapshot revision, so Core tracks request tokens independently for exactly-once
delivery at unchanged revisions. Resolution binds the original editor generation,
request token and numeric revision, and rechecks native interaction eligibility.
See the [request contract](number-step-requests.md). Op83
`Set_number_step_mode (node, Native | Application)` independently opts numeric
owners into user-input request emission. Native is tag 0/default; Application
is tag 1. Other kinds and unknown mode tags are rejected atomically. Mode changes
retire pending requests/holds while preserving editing state. Managed Eio handler
cleanup and physical platform acceptance remain open.


OCH-41 measured disclosure motion appends Op85 `Set_reveal` with a node and an
optional `{ expanded; retain; spring }` configuration. Only Panel accepts it.
Spring bounds are the existing animation bounds. Admission checks every changed
node after applying the complete transaction: collapsed configured panels must
have base Display Hidden; collapsed Unmount panels must have no descendants.
Configuration removal releases the native owner reservation (512 bytes plus
configuration storage). Frames are entirely native and produce no reveal events
or synchronous OCaml callbacks. Use matching library/backend builds; this is an
experimental addition within the current release train. Existing tags are not
renumbered. See the [disclosure contract](disclosure-presentation.md).

OCH-41 calendar presentation appends Op86 `Set_calendar_appearance` with a node
and optional fixed-size appearance. Only Calendar accepts it; None resets the
one-month defaults. Fields are month count (1..12), six finite geometry values
and six optional RGBA colors. Both decoders/admission enforce bounds; retained
accounting includes the appearance. Existing snapshot bytes are unchanged:
`month` still contains the focused civil date, while Rust keeps the first-visible
pane separately. No callbacks cross the FFI during pane layout. Use matching
library/backend builds within the current release train; older tags are unchanged.
See the [calendar presentation contract](calendar-presentation.md).

OCH-41 color presentation appends Op87 `Set_color_presentation` with a node and
optional section/appearance/panel metadata. Only ColorInput accepts it; None
resets flat/default presentation with both panels. Sections are `(featured, label, count)` records,
bounded to 32 sections, 256-byte labels and 256 total entries. Only the first
section may be featured. Nine finite geometry fields and two optional packed
RGBA border colors follow. Final tree validation requires nonempty section counts
to match the native palette length after all transaction operations. Retained
accounting includes metadata/labels. Existing color configuration, command and
snapshot bytes are unchanged. Use matching library/backend builds; older tags
are unchanged. After the two optional border colors, the payload includes Panels:
0 All, or 1 Tabs followed by two bounded labels and initial Panel (0 Palette,
1 Channels). This extends the unreleased Op87 payload and independent fixture.
See the [color presentation contract](color-presentation.md).


OCH-41 shared popover semantics appends Op88 `Set_popover (NodeId, bool)` on a
Container. True declares the existing anchor plus optional Popover overlay shape;
false removes the declaration. Final-tree admission checks one/two children and
a Popover overlay in the second slot, including child-only updates. A direct
Button/CommandButton anchor derives expanded and dialog-popup metadata from the
native surface; custom anchors remain unmodified. This is fixed-size semantic
coordination, not an additional callback/focus owner. See the
[picker trigger contract](picker-triggers.md#shared-popover-trigger-state).


OCH-41 calendar content appends Op89 `Set_calendar_content (NodeId, metadata option)`.
Metadata is a canonically sorted list of at most 1024 unique typed slots, each
with an optional bounded accessible description. Ordered retained wrapper
children supply passive content; metadata and the final tree are validated
atomically, including descendant-only changes. `None` requires an empty child
list. Native decoding bounds aggregate description allocation and retained heap
accounting charges metadata storage. Calendar actions, focus, selection and
command identity remain native. See the [calendar content contract](calendar-content.md)
for the 4096-node/128-depth content bounds and viewport distinction.


OCH-41 modal paint appends Op90 `Set_overlay_backdrop (NodeId, int64 option)`.
The optional value is resolved RGBA 0..0xffffffff; None restores half-opacity
black. A nonempty value requires a modal FocusScope in the accepted final tree.
Transparent color affects paint only, preserving modal input/focus/dismissal
policy. The paired fixture covers RGBA, transparent and reset values. See the
[backdrop contract](overlay-backdrop.md).

OCH-41 modal entry appends Op91 `Set_overlay_motion (NodeId, bool)`.
True requests native entry presentation for a modal FocusScope; false restores
immediate presentation. Admission rejects nonmodal targets in the final tree.
Entry runs inside the deferred surface without OCaml frame callbacks and never
retains removed content. See [modal motion](overlay-motion.md).

OCH-41 tooltip presentation appends Op92 `Set_tooltip_motion (NodeId, bool)`.
True opts a Tooltip into native entry and managed rapid switching; false is
immediate. This operation is rejected on HoverCard and other node kinds. It does
not change the existing Tooltip wire record or reset native timing configuration.
See the [tooltip motion contract](tooltip-motion.md).

OCH-41 shared positioning appends Op93 `Set_placement_geometry (NodeId, config option)`.
The fixed-size config contains a finite viewport margin and optional corner/point.
Default side geometry omits it; None resets it. Final-tree admission allows only
popover FocusScope, Tooltip/HoverCard and button menus. Menus apply the point to
the root and inherit only the margin for submenus. See [placement geometry](placement-geometry.md).

OCH-41 sheet layout appends Op94 `Set_sheet_insets (NodeId, insets option)`.
Four finite, bounded logical-pixel values reserve top/right/bottom/left content
space. None clears it; the public default omits metadata. Only sheet FocusScopes
may retain it in the final tree. The backdrop remains full-viewport and modal;
no timer or owner is added. See [sheet insets](sheet-insets.md).

OCH-41 calendar observation appends Op95 `Set_calendar_viewport_observer (NodeId,
HandlerId option)` and event tag71 `Calendar_viewport_changed`. Its independent
subscription carries a checked sequence and logical day/month/year display;
selection snapshots and primary callbacks remain unchanged. Consecutive same-route
observations may coalesce; selection and response boundaries stay ordered. See
[calendar viewport](calendar-viewport.md).


OCH-41 measured tracks append Kind54 `Carousel_track`, Op96
`Set_carousel_track (NodeId, Config)` and event tag72 `Carousel_track_requested`.
The root owns one Container track with retained Panel items matching the bounded
model count. Separate collection lineage, geometry epochs and source generations
fence layout observations and automatic proposals. Layout remains observable while
disabled; relative requests remain ordered. No existing tag or payload moves.
This is still an unfinished component: checked transport and construction are
implemented, with measured native rendering, motion, viewport keys and automatic
deadlines, pointer/trackpad/wheel input and related-control focus now in place.
Offscreen focus/reveal and full AX qualification remain pending. Op97 `Set_carousel_track_motion (NodeId, Motion option)`
adds independent duration/easing presentation without changing model revision. See the
[measured-track design](carousel-track.md).


OCH-41 measured-track control relationships append Kind55 `Carousel_track_group`.
It declares one CarouselTrack viewport followed by an optional plain controls
Container (up to 128 direct Buttons), with empty implicit text and no handlers on
the group/container. Candidate validation covers child-only and nonstructural
updates. Native input discovers related controls from this checked shape, not
application keys. Ordinary Press callbacks remain the application actions; the
group adds focus, keyboard routing and auto-pause scope without another model.
The paired unpublished epoch remains 3; no earlier tag moves.


## Tab target presentation

The unpublished paired epoch-3 protocol appends Op98 `Set_tab_appearance
(node, appearance option)`. None restores the legacy target presentation. Only
TabBar nodes accept it. The payload is variant (Tab0/Outline1/Pill2/Segmented3/
Underline4), height/gap/padding floats, shared style list, then a list of
`(choice-ID string, style list)` overrides. It contains no selection or callback.

IDs are unique nonempty UTF-8 strings without NUL, at most 256 bytes; up to 128
overrides are allowed. Geometry and aggregate style/state limits are checked
before retention. All styles together permit 256 declarations, counting an empty
raw style block as one work unit. Native admission enforces the target-only
presentation property scope, rolls back malformed updates atomically and charges
nested styles/IDs to retained memory. Per-ID overrides can outlive a filtered
choice. Reconciliation emits changed resolved presentation or an explicit reset;
ordinary choice revision/selection and native owner identity remain separate.
See [rich tabs](rich-tabs.md) for styling and unfinished scope.

## Structured tab parts

Op99 `Set_tab_content (NodeId, config option)` appends ordered label modes
(Default0/Custom1/Hidden2) and optional maximum whole-tab width. The encoded record
orders `max_width` before `labels`. Width is finite, 1..1e6 logical pixels; decode
admits at most 4096 modes. None restores the legacy simple/decorative shape.
No earlier tag or published epoch changes; both unpublished epoch-3 peers must be
built together.

With Some, each configured choice has one Container with exactly three Container
slots, in prefix/label/suffix order. Each part has at most one child; Custom requires
one label child, while Default/Hidden require none. Structural wrappers have no
text, styles, handlers, accessibility, scope or other independent behavior. IDs
and accessible names remain in ChoiceConfig; names must be meaningful. All parts,
including wrappers, fit within 4096 nodes and 128 levels. Label subtrees remain
passive; prefix/suffix are ordinary retained views. Native candidate validation
rechecks child-only and nonstructural updates, rolls back invalid shapes atomically
and charges the retained modes. Reconciliation compares metadata independently
of choice selection and sends an explicit reset when leaving structured content.

## Tab viewport and reveal

Op100 `Set_tab_viewport (NodeId, config option)` appends opt-in native scrolling
for TabBar. Some config contains one optional reveal record, encoded as positive
int64 serial then Choice-ID string (nonempty UTF-8 without NUL, at most 256 bytes).
None restores the non-viewport layout. Only the paired unpublished epoch-3 bridge
is supported; earlier tags remain unchanged.

Viewport requests are separate from ChoiceConfig selection and target appearance.
A missing target is a consumed/cancelled request rather than an invalid tree.
Native admission validates payloads and node kind atomically and reserves memory
for bounded scroll/request state plus the current choice count's measured boxes.
The existing NodeId generation protects placement identity. Reconciliation emits
changed viewport metadata and explicit removal; unchanged serials do not replay
on render, style updates or selection echoes. See [rich tabs](rich-tabs.md) for
hidden-node, request withdrawal and viewport lifetime behavior.

## Tab trailing slot

Op101 `Set_tab_trailing (NodeId, bool)` declares one structural trailing slot
after the logical options of a TabBar. False is the default. With true, the root
has exactly `ChoiceConfig.items.length + 1` children; the final child is an inert
structural Container with zero or one ordinary view child. That view may be
interactive. The option slots still obey the plain/decorative or Op99 structured
contract, and passive-label validation excludes only the trailing subtree.
All tab content, including the trailing subtree and wrappers, stays within
4096 nodes/128 levels. Candidate validation rechecks nonstructural slot updates.
The Boolean uses strict bin_prot Boolean encoding. Earlier tags and the paired
unpublished epoch remain unchanged.

The trailing view renders after options and does not contribute a logical choice
index, selection callback or accessibility position. Its ordinary node allocations,
resources and callback lifetimes remain accounted by the retained tree. Resetting
the flag must remove the trailing slot in the same atomic transaction.


## Choice menu presentation

Op102 `Set_choice_menu (NodeId, bool)` is a strict Boolean on Select only, default
false. It switches the existing choice trigger/popup to Button/Menu/MenuItemRadio
semantics and native selection checkmarks, without changing `ChoiceConfig` IDs,
controlled state or asynchronous Choice events. Other node kinds and stale node
generations fail atomic admission. The frame helper owns this internal projection
from its current tabs; no additional public Select mode is introduced. Earlier
tags and paired unpublished epoch 3 remain unchanged. See [rich tabs](rich-tabs.md).


With Op102 enabled, a Select may additionally own one structural Container per
choice, in choice order. Each slot is empty or contains one decorative leaf Icon;
wrapper styles/text/behavior and named images are rejected. With no icon map the
Select is childless. Changing the flag to false requires removing the slots in
the same atomic transaction. Dirty ancestor validation rechecks late icon/slot
updates. This adds no opcode or encoding: it defines the retained child shape of
the paired menu presentation. Slots render only within virtualized popup rows;
image source readers are acquired at accepted tree updates, not during painting.


Tab motion uses paired internal Op103 `Set_tab_motion` on TabBar nodes. Its
optional config carries the existing spring fields and `color_duration_ms`
(0–60000). Admission is atomic, validates the spring and charges a fixed 4096-byte
native owner reservation. Reset releases that reservation. Geometry, velocity,
paint epochs and frame scheduling stay native; this operation adds no callbacks,
selection events or reveal commands. See [tab motion](tab-motion.md).

Flat split groups append paired internal Kind56 `Split_group`, Op104
`Set_split_group (node, config, appearance)` and Event73 `Split_group_resized`.
The event carries window/node/handler IDs, accepted revision, reset generation
and a bounded keyed snapshot/source. Configuration changes rotate the Core handler;
native publication also requires exact current config and matching snapshot IDs,
ranges and request serial. Appearance changes retain the handler. Each panel has
a stable structural wrapper containing one content slot and an optional passive
grip slot; admission rechecks dirty descendants after nonstructural edits.
Group admission reserves 8192 bytes plus 4096 per panel and variable descriptions/
styles. Input queue accounting includes every snapshot ID and numeric extent.
These are quota units, not measured allocator RSS. Geometry and pointer previews
stay native. See [split-group contract](split-group.md).

Toast placement appends Op105 `Set_toast_placement (node, placement option)`.
The eight anchor tags are top-left, top-right, bottom-left, bottom-right,
top-center, bottom-center, left-center, right-center; four f64 insets follow in
top/right/bottom/left order. Each inset must be finite in 0..16384 logical pixels.
Only toast-stack nodes admit the operation. `None` restores the existing corner
metadata and 16px window margins. Placement carries no callback or lifetime reset;
ordinary stack metadata remains Op22. Admission charges the fixed placement
record size while present and releases it on reset/removal; this is quota
accounting, not measured allocator RSS. See [placement contract](toast-placement.md).


Toast layering appends Op106 `Set_toast_layering (node, layering option)`.
Three f64 values (peek, expanded gap, fractional width reduction per depth) precede
an int64 visible-layer count. Peek/gap are finite 0..16384, width reduction is
finite 0..0.1, and visible layers are 1..8. Only ToastStack admits this metadata;
`None` restores the ordinary expanded column. Reconciliation emits changes/reset
without recreating children or changing their handlers. Admission reserves the
record plus 512 quota bytes per configured stack and 1024 per submitted child;
existing child payload accounting remains separate. These are quota units, not
RSS. No new events or synchronous callbacks accompany measurement, hover,
scrolling or keyboard expansion. Native motion uses separate optional metadata.
See [toast presentation](toast-presentation.md).


Toast motion appends Op107 `Set_toast_motion (node, motion option)`. The optional
record carries spring stiffness/damping/mass/epsilon f64s, spring maximum-duration
int64 milliseconds, entry/exit int64 milliseconds and slide-offset f64. Spring
bounds match the existing checked animation contract; entry/exit allow 0..60000ms,
slide is finite 0..16384px. Only ToastStack admits this metadata. `None` settles
phases and restores immediate dismissal; it never remounts child owners or reopens
closed keys. Admission reserves the record plus 1024 stack quota bytes and 4096 per
submitted child, in addition to existing layering/content accounting. Accepted
native dismissal retains an opaque session token until finite exit completion;
Op107 introduces no frame events or synchronous callbacks to OCaml. Existing
ToastDismissed identity/handler/revision fencing applies. See
[toast presentation](toast-presentation.md).

## Transcript log semantics — OCH-41

The paired unpublished epoch-3 accessibility Role appends `Log` at tag16;
existing tags and Config field layouts remain unchanged. Only ordinary containers
and managed-list roots accept the role. It defaults to Polite in Core; explicit
Off/Assertive uses the existing Live field. Native semantics projects AccessKit
Log and the exact live priority without adding focus/click actions, callback
owners, timers or retained offscreen rows. Tree input still requires Tree and
cannot be enabled on a Log root. Both runtimes must be rebuilt together.

The gallery uses a named Log with Live.Off while manually streaming fragments.
Applications can announce completed responses in a separate Status instead of
having assistive technology announce every fragment or viewport remount.
Virtualized content remains limited to retained rows; a log role does not create
an accessible copy of all history. The pinned Cocoa adapter maps Log to AXGroup
with AXApplicationLog subrole; actual notification delivery/VoiceOver behavior
still needs macOS qualification. The independent `accessibility-log.hex` fixture
fixes tag, name and polite priority on both sides.

## Shared scrollbar metadata — OCH-41

Op108 `Set_scrollbar (node, config option)` attaches a checked, fully resolved
Scrollbar description to an ordinary Container or managed VirtualList root.
Tree/table presenters use that managed root. Other kinds reject the operation,
including reset; native editors retain their own scroll behavior. The independent
`scrollbar-operation.hex` fixture fixes attach/reset tags and payload bytes in
both languages. Existing operation tags and record layouts are unchanged.

The payload contains axis selection, visibility mode, sparse base/hover/pressed
track/thumb refinements, bounded native motion and a named accessibility area.
No offsets, handles, timers or callback pointers cross the bridge. The existing
strict config decoder checks hidden states too; theme tokens must resolve before
OCaml submission. Reconciliation stores the resolved description, so a theme
update can emit metadata without replacing child nodes or event bindings.

Admission reserves the native description size, bounded label and 8192 fixed
native-state units per attached viewport. These conservative quota units include
range/focus/clock/paint bookkeeping; they are not measured RSS. Invalid data or
budget overflow rejects the complete transaction. Reset releases the reservation
and restores that owner's default presentation, retaining its scroll offset.
Container metadata may remain dormant without overflow; it never changes the
owner's allowed axes. Managed scrollbar visibility flags retain precedence.

Production host integration and gallery/consumer acceptance must accompany this
operation before claiming complete shared-scrollbar support. A passing codec or
admission test alone does not prove native viewport behavior.

## Managed-table retained headers — OCH-41

Op113 `Set_table_header (node, target option)` marks an inert generated Container
with exactly one child. `Column id` uses tag0; `Group { level; columns }` uses tag1,
zero-based levels 0..3 and 1..64 strictly sorted distinct IDs. Both use the existing
bounded UTF-8 column-ID representation. Clearing is explicit. Existing operation
layouts and tags remain unchanged; paired runtimes must be rebuilt together.

Only direct children of a managed table may carry this metadata. Final admission
requires exact current schema membership and unique targets, at most 320 slots,
and disjoint exhaustive ownership by headers and `Set_list_rows` body entries.
Header wrappers cannot introduce styles, handlers, focus, accessibility or other
interactive wrapper behavior; their ordinary child Views carry the content.
Header target metadata and native map storage are charged against retained bytes.
Changing the target-to-wrapper map requires a strictly newer table schema revision;
updating a surviving header's content does not. Invalid batches roll back fully.
See [renderer slots](table-renderer-slots.md) and
[local rendering evidence](../evidence/table-header-rendering-och41.md).

## Managed-table scoped presentation — OCH-41

Op114 `Set_table_header_style (node, styles)` and Op115
`Set_table_row_style (node, styles)` append checked `Style` lists; empty lists clear
presentation. Earlier tags remain unchanged; paired runtimes must rebuild together.
Only Fields/State layers are accepted, with at most 128 total declarations and
128 layers. The decoder enforces aggregate declaration limits and the shared
paint/typography scope; admission additionally validates all values and ownership.
Header styles belong to configured table roots. Row styles belong to inert direct
managed body-row wrappers, never cell/header slots. Final-snapshot ownership and
byte quotas reject invalid transactions atomically. Metadata and native-map
reservation are charged; reset/removal releases their charge.

No schema revision bump or event is introduced for these paint updates. Rust
resolves native selection/focus/pointer state locally over retained descriptions.
See [renderer contract](table-renderer-slots.md) for state precedence, lifecycle and
geometry limits. This wire implementation does not establish physical GPU/input
acceptance.

## Document selection format — OCH-41

Op116 `Set_document_selection_format (node, markdown)` appends a strict Boolean
metadata update to a DocumentView. False is the mount default and explicitly
restores plain selection copy; true chooses Markdown reconstruction for rendered
Markdown selections. Wrong-kind and stale-node operations reject atomically.
The setting is node-generation scoped, with no source/parser revision change.
Earlier opcodes and Set_document payloads are unchanged; paired runtimes rebuild
together. See [selection contract](document-selection-format.md) for exact copy
semantics and the independent source/code/table Copy controls.

## Document preview observations

Op117 appends `Set_document_preview (node, { epoch; max_lines; observe })`;
Event76 appends `Document_preview_observed`. The configuration uses a strictly
positive monotone epoch, optional 1..4096 body-line budget and Boolean observer.
The event includes source revision/generation and a typed presentation state.
See [the preview contract](document-preview.md) for compatibility validation,
installed-picture provenance, queued delivery and clearing. Existing document
and selection-format bytes remain unchanged.

## HTML reader mode

The append-only `Document_wire.Mode.Html` constructor uses tag 3. Existing
Markdown (0), Code (1 with language) and Diff (2) bytes are unchanged. Paired
runtimes rebuild together; unknown modes are rejected. HTML shares registered
source/image identity, queued navigation and bounded worker preparation. Op117
preview configuration also accepts HTML Flow. See [the reader contract](document-html.md).

## Internal document styling

Op118 appends `Set_document_text_style (node, config option)`. The configuration
contains six named resolved color parts, optional paragraph/heading metrics,
inline-code highlights and four passive part style lists. Decode limits are
256 KiB/config and 512 total part declarations; scopes, finite values, concrete
colors and final Markdown/HTML ownership are checked before atomic admission.
None restores native defaults. It changes no source revision or callback epoch;
retained byte quotas include its allocation. Existing wire tags remain unchanged;
paired runtimes rebuild together. See [styling](document-styling.md).

## Document link activation metadata

Event30 retains its envelope and original Navigation tags0/1. Tag2 appends
`Link_activated (url, activation)` with typed Mouse/Keyboard/Touch source and five
release-time modifiers. Nonmouse modifiers must all be false. URL validation and
queued source/handler ownership remain unchanged. Legacy Link decodes with unknown
activation; current readers emit the richer tag. No operation or implicit URL
opening is added. See [the routing contract](document-link-activation.md).

## Markdown parser options

Op119 appends `Set_document_markdown_options (node, options)`; the fixed payload
is Frontmatter tag0 Disabled/tag1 Code_block/tag2 Description_list, followed by
strict Boolean mdx. Description_list uses restricted YAML metadata with code
fallback; see [the renderer contract](document-frontmatter.md).
Defaults clear the override; nondefault options require final Markdown ownership.
Only DocumentView accepts the operation. Existing source/config bytes do not change;
paired runtimes rebuild together. Options join worker identity without changing
source revision. See [the parser contract](document-markdown-options.md).

## Rich document actions

Op120 adds `Set_document_actions`; Event77 adds `Document_action`. The independent
config carries a positive monotonic epoch, observer/Copy flags and bounded code/
table descriptors. Final admission requires a rich DocumentView and a handler for
observed actions. Events carry installed revision/generation, optional trustworthy
source range, activation metadata and a bounded immutable code/table snapshot.
No native callback crosses synchronously into OCaml. See the
[action contract](document-actions.md) for limits and stale-input fencing.


## Desktop scrollbar preference snapshot — OCH-41

The desktop request enum appends tag 7 `Scrollbar_preference`; its response enum
appends tag 6 with `Auto_hide` (0) or `Always_visible` (1). Earlier desktop tags
remain unchanged. Unknown preference tags, incomplete payloads and trailing event
bytes are rejected. Requests use the existing positive correlation and bounded
asynchronous desktop lane. No window identity or desktop registration is required.
Linux responds with the existing `Failed Unsupported`, not a fabricated value.
See [the contract](scrollbar-preference.md) and
[paired/native evidence](../evidence/scrollbar-preference-och41.md).

## Editor source range geometry — OCH-41

Editor command tag 11 `Read_range_bounds` / `ReadRangeBounds` appends a
nonnegative source revision and a directed UTF-8 byte selection (anchor/head,
each 0..262144). Result tag 7 `Range_bounds` / `RangeBounds` carries an option of
`(revision, x, y, width, height)`, with bin_prot int64 revision and float64 metrics.
Coordinates are finite in ±1e9 logical pixels; width is 0..1e9 and height is
positive and at most 1e9. A present result must match the requested revision.

The query is metadata-only and uses the existing editor lease, correlation,
64-request capacity and close rules. It normalizes directed selections natively,
validates current source boundaries, and distinguishes stale source from absent
matching layout. Prior command/result encodings remain stable; the unpublished
bridge ships both languages together. See [range geometry](editor-range-geometry.md)
for timing, coordinate and visibility limits, and [paired validation](../evidence/editor-range-api-och41.md).
