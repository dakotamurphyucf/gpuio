# Disclosure, navigation and supplementary overlays (OCH-37)

Status: implementation in progress. Core models, mounted disclosure, breadcrumbs
and pagination pass the local checks recorded in the evidence ledger. Full-family
capability and acceptance are not yet advertised. This
specification retains the full live OCH-37 scope, including native behavior,
examples and platform gates.

## Ownership and component mapping

| Family | Application-owned values | Native responsibilities |
| --- | --- | --- |
| Collapsible/accordion | Stable item IDs, single/multiple expanded set, disabled policy | Expanded semantics, trigger activation, header traversal, reveal and focus handoff |
| Navigation stack | Stable entry IDs and payloads, back/current/forward history | Incoming/outgoing transition, focus restoration, hidden subtree gating |
| Sidebar/breadcrumbs | Bounded labelled destinations, current entry, selection requests | Navigation/current semantics, keyboard and pointer activation |
| Pagination | Page count and selected page, relative/absolute requests | Accessible navigation controls and focus; no data fetching |
| Hover card | Content description, optional controlled visibility | Hover/focus delays, anchor placement, dismissal, interaction within content |
| Sheet/drawer | Open content and dismissal policy, edge/size | Existing modal stack, edge placement, focus trap and restoration |
| Alert dialog | Open content, explicit confirm/cancel actions | Alert-dialog semantics and existing modal focus/dismissal routing |
| Carousel | Ordered IDs, selected item, optional auto-advance policy | Keyboard/pointer requests, bounded timer, transitions, visibility/focus/motion gating |

The pinned base implementation is under `vendor/gpui-base/src/`. Accordion and
collapsible expose controlled open state. Base navigation keeps history and one
outgoing view; GPUIO places history in OCaml to avoid dual sources of truth.
Its push discards the forward branch, pop protects the root, and replace preserves
forward entries. Pagination's implicit minimum of one page is not adopted:
GPUIO models an empty dataset explicitly. Hover-card state owns cancellable delays;
sheet and alert-dialog use modal focus hosts. GPUIO must integrate its existing
native overlay/focus adapters, rather than introduce another modal registry.
The styled-layer sources were also inspected at the exact same revision:
`crates/component/src/sidebar/{mod,menu,group,header,footer}.rs`,
`breadcrumb.rs`, and `carousel/{carousel,state}.rs`. The local research checkout
is evidence only, never a build dependency. Sidebar supports left/right placement,
header/footer/groups, icons, selected/disabled destinations, nested expansion,
context menus and Icon/Offcanvas/None collapse. Breadcrumbs provide an ordered
path, separators and optional disabled links. Carousel supports horizontal/vertical
orientation, first/last/previous/next selection, looping, keyboard navigation,
pointer axis locking/drag snapping, wheel scrolling and pagination controls.
These features inform GPUIO's functional counterparts; importing the entire styled
crate and its theme/i18n/dependency stack is unnecessary. Reuse existing icon/menu,
scroll, pointer-capture and motion infrastructure for the corresponding behavior.
The ticket's optional auto-advance is an additional native scheduling contract.

## Pure models

`Navigation_stack` holds up to 128 entries across back/current/forward history.
An entry ID identifies an instance, not a route name: two visits to the same route
use different IDs to keep independent drafts. Payloads remain arbitrary
application values and are never serialized by the navigation model. Push/pop/
forward/replace/update are pure. Pop at root and forward at the end do nothing.
Replace can preserve the current ID to update its content, but cannot steal an ID
from another entry. Push requires a fresh ID within the existing history, even
when it discards the forward branch. Neither dropping an entry nor hiding it
implicitly cancels an Eio task: application resource owners decide that policy.
Operations and enumeration are O(n), with n bounded to 128; no hidden cache grows.

`Disclosure` uses the existing bounded `Choice.Collection` for labels and IDs.
Expanded IDs are unique, valid members, and returned in collection order. Single
mode optionally requires one expanded member when nonempty. Disabled members may
remain expanded as historical application state; user requests cannot change them.
On collection/mode replacement, missing members drop, single mode keeps the first
expanded member in collection order, and required-single mode falls back to the
first enabled member (or the first member if all are disabled). Selection is
otherwise not silently corrected at construction. Expand/collapse/toggle requests
are reduced against the latest model so repeated queued toggles do not collapse
into one stale Boolean assignment. Native observations do not mutate application
selection without its reducer accepting the request.

`Pagination` uses one-based pages; zero total pages means no selection. Explicit
construction/selection rejects invalid pages. An explicit count change preserves
the page where possible, clamps to the new last page on shrink, clears at zero,
and selects page 1 on growth from zero. Next/previous/first/last requests reduce
against current state. An absolute request outside the latest count, or any user
request while disabled, is ignored. Data retrieval and loading/error state remain
separate. At most 1,000,000,000 pages are represented by a count; visible items are
bounded to 13 (two endpoints, a neighborhood of at most nine, and two gaps), never
a list proportional to the dataset. Gaps carry inclusive omitted ranges.

## Hidden content and motion

The View contract will require an explicit retained-versus-unmounted policy.
Retained content keeps keyed native editors/lists mounted and is excluded from
paint, pointer/AX/keyboard input, IME and animation work while hidden. Unmounted
content disposes those native resources; later appearance creates a fresh lease.
This does not itself deactivate a Bonsai computation that an application still
builds: lazy Bonsai branches/associations control model and component-task lifetime.
Examples must demonstrate these independently and keep conversation/network tasks
in an owner whose lifetime matches the data rather than a transient visible page.

Semantic navigation changes are immediate application decisions. Native motion
may retain at most one outgoing visual snapshot/subtree per container, with no
focus/input in that outgoing content. Interruptions retire obsolete work;
reduced motion settles immediately. Focus leaving a collapsed panel returns to
its eligible trigger; route changes restore an eligible destination or focus its
first control. Nested dialogs retain precedence over container focus restoration.
Existing tab bars, tab panels and resizable panes remain the workspace vocabulary.

### Navigation presenter contract

The mounted API is `View.navigation_stack model ~hidden ~label ~content ()`,
with optional `key`, `style`, `page_style` and `motion`. `content` receives an
entry, so its payload remains an OCaml value. The presenter uses one keyed Panel
wrapper per history entry, in history order, and a selected index. With `Retain`,
each page's builder runs and matching native descendants survive navigation.
With `Unmount`, only the selected builder runs and inactive wrappers have no
children. Empty wrappers preserve the ordered history positions without retaining
editors, buffers or other page resources. App-supplied Bonsai computations and Eio
data scopes retain their separate ownership rules.

Native presentation follows generation-checked page node IDs, not route labels or
indices cached across transactions. There is one current page and at most one
outgoing painted page. Inactive retained pages must still take the host's normal
hidden-element path so retained button/focus owners are not accidentally swept as
unvisited; they do not paint or accept input. The outgoing page paints through the
inert boundary. Only the current page is eligible for focus/IME and accessibility,
including during the transition. The container clips to its assigned dimensions;
page content does not determine a competing animated window size.

`Navigation_stack.Motion` offers immediate, slide and fade policies. Default slide
is 200 ms; duration is bounded to 0..10 seconds and rounded up to milliseconds.
Slide offsets are fractions of the current assigned width, so resize does not
require a synchronous measurement round trip. Smoothstep easing is native. Back
and forward direction use stable page identity in the new order before falling
back to history positions. Same-position replacement fades. Initial placement,
zero duration and immediate policy settle without a run. Label/data updates to
the same selected ID do not restart motion; changing the motion/retention policy
settles an existing run. Hidden/reduced-motion presenters settle and do not replay
old navigation when shown again.

Interruption starts from the last accepted paint, never a speculative layout.
Reversal can reuse the previous exit as the incoming page. A third destination
retires the older exit and uses the last painted current page as its single exit.
A destination selected and replaced before it paints does not become an outgoing
visual. Samples carry an internal epoch; paint from before retarget/removal/settle
cannot request another frame or alter the new run. The pure motion state stores
only bounded IDs and geometry, with no native resource leases or application
history payloads.

Removal has precedence over animation. Replacing/removing an entry releases its
native subtree in that transaction. Consequently a removed page has no outgoing
visual; its replacement still enters. Unmount likewise animates incoming content
without retaining a removed outgoing editor. Returning to a retained page should
restore its last eligible control, otherwise its first eligible control. Focus
history must be bounded by current page membership, use weak native handles, and
yield to a higher modal scope. A separate workspace or native route model is not
introduced.

The Core/Bonsai API, transaction adapter and native renderer are now connected.
Kind 45 and operation 48 append the navigation node/configuration without changing
earlier tags. Admission requires at most 128 direct labelled Panel children and a
selected index agreeing with the child count; Unmount rejects populated inactive
pages. Invalid changes roll back atomically. Both bridge halves must be rebuilt
together under the existing single-release compatibility contract.

The focus manager checks selected-page ancestry directly against the admitted
tree, independently of transition progress. Outgoing scopes are therefore retired
before widget synchronization. Remembered focus uses weak handles bounded by
retained page membership; disabled/removed controls fall back to the destination's
first eligible painted control. Pending destination focus retries after actual
paint when the first incoming frame is fully clipped, without a timer or additional
frame request. Higher modal scopes and a user focus choice already inside the
destination take precedence. Hidden retained panels still build native elements,
but have no paint or accessibility subtree; outgoing content uses the inert paint
boundary. No transition endpoint crosses the bridge.

Mounted macOS tests now cover nested routes, higher and outgoing modal scopes,
native marked text isolation, pointer shielding, Tab/Shift-Tab during painted exits,
resize during motion and all 128 pages. A route request deferred by a higher modal
remains pending while its destination is still visible; accepted modal close then
restores destination focus. A hidden/removed destination retires the request.
When painted but ineligible controls exist, keyboard traversal uses the focus
manager's eligible order even without an active modal trap. Otherwise ordinary
native traversal remains in use. These rules close gaps that an input callback
guard or accessibility hiding alone cannot address.

The 128-entry history bound is independent of resource quotas. In particular,
ordinary editors still reserve 8 MiB each against the 64 MiB per-window logical
budget, as documented in the native-editor contract. These are conservative
admission units, not measured RSS. Retain does not exempt hidden editors. The
mounted workload uses 128 pages/buttons with four editor pages and read-only
content elsewhere, checks a rejected 128-editor proposal leaves state unchanged,
and verifies Unmount releases inactive widgets. Applications with many editor
pages can explicitly unmount inactive native content and keep data/drafts under
application ownership. No quota was widened to admit this workload.

These results do not yet establish full OCH-37 family acceptance; supplementary
overlays, carousel, broader sidebar presentation/workload and final hosted gates
remain.

## Native bridge and validation still to implement

Use generation-checked node routes and ordered bounded requests. Relative
requests must not be coalesced away. Native timers are per mounted owner with
cancelled tasks/generation checks; no permanent carousel polling or synchronous
OCaml callbacks. Auto-advance pauses while focus/hover interaction, ancestor/window
visibility or reduced-motion policy requires it; at most one pending selection
request waits for application reconciliation. Stop at teardown/window close.

Navigation/current and disclosure expanded metadata are implemented below. Extend
the existing overlay variants while preserving Dialog/Popover/Tooltip behavior and
nested Escape/outside routing. Do not declare a component complete from a styled column or model tests.
Paired codec fixtures are required for new wire contracts. Expect tests cover
history, disclosure and shrink/request races; actual macOS tests must cover nested
focus/IME, lazy/retained lifetimes, nested overlays, constrained themed layouts,
carousel timing/idle/disposal and public Bonsai/Eio usage. Linux builds/unit tests
and the consolidated macOS hosted checks remain required before milestone merge;
Linux GUI acceptance stays under OCH-17. OCH-46 integrates these into the chat demo.

## Mounted disclosure contract

`Content_policy.Retain | Unmount` and `View.panel`, `View.disclosure` and
`View.accordion` now have Core and Bonsai bindings. Panel is a labelled native
region (not a tab panel). Inactive regions use native hidden layout/input policy.
Retain preserves children; Unmount removes them and creates fresh child leases on
reopening. The disclosure trigger and enclosing region keep stable keys. Accordion
content callbacks run while constructing an OCaml description, never in Rust;
Unmount skips callbacks for collapsed items. Bonsai branching and Eio scopes still
control computation/task lifetimes independently.

The wire appends kinds 42 Panel, 43 Disclosure and 44 Accordion. It reuses existing
style changes, splice operations, button handler leases and Press events. A
Disclosure has exactly one Button followed by one Panel; an Accordion has at most
4,096 direct Disclosure children. Admission checks these relationships atomically.
Panel labels require 1..4,096 UTF-8 bytes without NUL. A native header's expanded
property follows its associated panel's effective visibility. Up/Down wrap among
eligible direct headers; Home/End choose endpoints. Modified keys and nested
accordion groups remain independent. Enter/Space and accessibility Press use the
same existing button activation path, delivering one semantic toggle intent.

The focus manager records each painted control's disclosure ancestry, bounded by
the retained tree's depth. A collapse/unmount can therefore restore an eligible
trigger even after the old focused editor disappears from the new tree. Hidden
nested triggers are skipped for an eligible outer trigger; modal restoration and
entry retain priority. Scope collection skips style-hidden subtrees so an invisible
retained modal scope does not continue trapping focus. Header lookup uses an index
of painted handles rather than scanning the full focus list once per header.

The pinned macOS adapter needed a focused correction: AccessKit 0.26.3 stored
expanded state but its macOS adapter did not expose the corresponding getter or
selector. `vendor/accesskit-macos` preserves that exact version and adds the getter,
selector availability and change notification. See [patch provenance](../../vendor/accesskit-macos/GPUIO.md).
Both standard and composed static backends use it; no global installation or GPUI
version upgrade is involved. Native AppKit getter/Press checks pass. VoiceOver
speech and external notification observation remain distinct from those checks.

This is an initial mounted disclosure implementation, not complete OCH-37
acceptance. Broader layout/appearance, reveal/navigation motion, public examples,
other navigation/overlay/carousel families and full lifecycle/workload acceptance
remain to be finished before capability advertisement.

## Breadcrumb and pagination compositions

`Navigation.breadcrumbs` accepts an ordered, bounded `Choice.Collection`. It keys
wrappers by destination ID, uses native links for eligible ancestors, and exposes
the final member as current-location text. Separators are decorative borders with
no labels, handlers or focus stops. Labels can change without replacing an
ancestor's button. Callbacks carry IDs for resolution against current application
state; the helper does not capture route payloads or own a second route model.

`Navigation.pagination` renders the existing `Pagination` model with native
buttons, range-labelled inert gaps and a navigation landmark. Four boundary
controls plus at most 13 items bound the entire component to 17 direct children,
even with a billion pages. Empty/boundary/disabled controls have no active handler.
The current page stays actionable and is independent of keyboard focus. Relative
intents reach the latest application reducer; count shrink clamps selection and
retires absent page handlers. Stable page/boundary keys preserve overlapping
button identity. Ordinary Tab/Shift-Tab, Enter/Space and accessibility Press are
used; no competing roving-focus or native page-selection state is introduced.

`Navigation.Appearance` refines item/current/gap/separator styles and defaults to
existing theme tokens. `Pagination_labels` supplies localized navigation/control/
current strings and bounded pure page/range formatters; formatters run in OCaml
only for visible items. Invalid strings return an error before reconciliation.

`Accessibility.Role.Navigation` is appended as role tag 11. `Current` distinguishes
True/Page/Step/Location/Date/Time from selection, focus, toggled and expanded state.
An optional current field is appended to semantic Config; absent clears the
property. Current metadata is supported on text and button/command-button nodes,
not arbitrary containers or form fields. Navigation landmarks require containers.
Each current item requires a localized description as well: the pinned AccessKit
macOS and Unix adapters do not consistently map `aria_current`. GPUIO emits that
property in AccessKit and preserves the description through native help, without
inventing a macOS attribute or claiming screen-reader speech validation. The
current style also uses weight and a border, so color is not the sole visual cue.

This changes the unreleased metadata binary layout. Rebuild both the OCaml and
Rust halves together, including separately composed static backends; mixing older
semantic encodings is unsupported. Independent existing fixtures were rebuilt
with the new trailing option, and a new current-page fixture verifies both codecs.
No released dependency pin or component capability advertisement changes here.

The [Navigation Lab](../../examples/navigation/README.md) demonstrates the public
Bonsai/Eio usage. Native panel retention keeps an editor mounted while hidden;
Bonsai conditional branches independently activate/deactivate lazy computation;
a data scope outlives both visibility changes and is cancelled when its window
closes. Lazy deactivation itself is not a promise to discard the Bonsai model.

## Sidebar model and initial composition

`Sidebar` owns immutable destination metadata, current ID, expanded IDs and the
requested collapse preference. Destination IDs are unique across groups; group
IDs have their own namespace. A destination may also contain children. Selecting
it and toggling its children are separate requests. Requests reduce against the
latest model: missing, hidden and disabled destinations cannot navigate; leaves
cannot expand; icon mode ignores expansion requests whose control is hidden.
Collapsing an ancestor preserves descendant expansion and historical selection.
Replacing groups clears absent selection and drops missing/now-leaf expansion,
without choosing an unrelated route. Explicit programmatic updates remain possible
while built-in user actions are disabled.

Bounds are 4,096 items, depth 16, 128 groups and 256 KiB of aggregate text/IDs.
Subtree constructors cache validated counts so models can check aggregate limits
without unbounded recursion. These are model bounds, not a promise that arbitrary
custom decoration trees bypass the normal retained-tree, message or resource
budgets. Sidebar is ordinary navigation; use managed list/tree components for
large virtualized datasets. Models hold no route payload, native owner or task.

`Sidebar.view` composes native links, separate expansion buttons, labelled regions,
managed compact-mode tooltips and existing context menus. Header/footer callbacks
receive compact state; suffixes disappear in icon mode; scoped SVG icons retain
the existing icon ownership contract. A validated compact label provides an
iconless fallback. Left/right placement selects the inner border; the application
places the sidebar on the corresponding side of its workspace. The external
`Sidebar.toggle` can live in an always-visible toolbar. Built-in disabled policy
covers navigation/expansion requests; supplied slot controls and context-menu
commands own their enabled policy.

`Icon` preserves top-level link identity and names while hiding nested content,
headings, suffixes and expansion buttons. `Offcanvas` immediately hides content/accessibility and releases its allocated
layout width over the configured transition; `Never` ignores collapse without discarding the stored
preference. `Content_policy.Retain | Unmount` governs native hidden descendants,
not Bonsai computation or Eio task lifetime. Unmount skips hidden-content builder
callbacks; retain preserves native nodes. Structural decoration changes, such as
adding a context-menu wrapper, may replace descendants. Ordinary collapse and
selection preserve matching keyed links. Labels and all dynamic expansion labels
are bounded/validated; appearance refines existing tokens and state styles.

`Sidebar.Motion` now configures native allocated-width transitions (default 200 ms
with ease-in-out, or `immediate`; durations 0..10 seconds). The stable animated
wrapper uses existing OCH-12 ownership: immediate first placement, interruption
from the painted width, no OCaml callback per frame, native reduced-motion settling
and disposal. Fixed-width inner content takes its target geometry at model update,
so its text does not reflow with each intermediate wrapper width. Width arguments
own this geometry even if an appearance style supplies another width; other style
refinements remain available. The inner border and clip alignment follow `Side`.

Retained offcanvas content now uses base-only `Style.Inert true`: its layout and
paint survive while focus/IME, accessibility, pointer input and active nested
popup/timer eligibility cease immediately. The fixed-width inner panel aligns
toward the content edge, so decreasing the outer clip width slides left/right
outward without per-frame text reflow. At zero width it stays retained and clipped;
the shared animation owner becomes idle. Reopening clears inert state and reverses
from the painted allocation, preserving native editor/link identity.

`Content_policy.Unmount` removes descendants immediately, preserving its explicit
native-lifetime contract; only the remaining empty allocation animates. Removing
the sidebar itself also disposes immediately. This does not postpone reconciliation
or transfer application task ownership to a native animation. Broader sidebar
appearance/workload acceptance and mounted navigation transitions remain pending.

### Native inert presentation boundary

`Inert` appends portable field tag 65; old tags remain unchanged. Both bridge halves
must be rebuilt together within the existing single-release compatibility policy.
It is rejected in state refinements and follows normal last-base-declaration
replacement on one node; a descendant `Inert false` cannot override an ancestor.
Ordinary Display/Visibility hidden styles continue to remove paint.

The existing focus manager excludes the subtree, including its modal/tooltip and
popover scopes, and denies programmatic focus/submit. A native wrapper delegates
exact layout and paint, marks an accessibility root hidden and registers a blocking
hitbox after descendants. It covers specialized renderers as well as ordinary
containers without changing child layout or native entities. Existing native
visibility hooks suspend nested animations and other active widget work. Application
Bonsai computations/Eio tasks remain application-owned; commands that intentionally
edit retained buffers remain distinct from native user input. The wrapper needs no
GPUI fork, screenshot readback in production, or cross-runtime paint callbacks.

Inert content is a live retained presentation, not a frozen screenshot. Modal and
hover surfaces are deliberately retired instead of remaining interactive or
floating beyond the exiting panel. Navigation must still enforce at most one
outgoing presentation and its route/focus/lifetime contract; this primitive alone
is not a completed navigation stack.

### Independent actions inside disclosure headers

`View.disclosure_with_header` accepts arbitrary header content and a dedicated
plain Button trigger. It constructs a header Container whose last child is that
trigger, followed by the usual content Panel. The original direct Button/Panel
shape remains supported. Native admission validates the proposed transaction's
header shape atomically. Shared tree lookup identifies the toggle without searching
arbitrary descendants, so a primary navigation Link does not accidentally gain
expanded semantics or accordion arrow behavior. The dedicated toggle receives
expanded state, accordion traversal and collapse-focus restoration. Removing the
focused child also restores it when eligible, even if the empty panel remains
visible; enclosing modal scope policy keeps precedence.

Hidden retained tooltip/overlay owners have no active focus scope. Rendering must
therefore avoid requiring their old anchor or mounting a deferred surface. A
hidden tooltip retains its anchor layout/owner with no hover listeners or active
surface; its native synchronization cancels pending timers. Hidden overlays skip
deferred rendering, and parent containers only capture active popover anchors.
This preserves retained ownership while keeping hidden popup content inactive.

## Sheets and alert dialogs

`Sheet.Config` separates `Edge.Left | Right | Top | Bottom` from a validated
`extent` in logical pixels. The default is a right drawer of 360 pixels; extent
is finite in 1..16384. Native layout clamps it to the viewport and fills the
other axis. Panel dimensions, min/max dimensions, margins, offsets and positioning
cannot detach a sheet from its edge, including hover/focus style refinements.
Ordinary colors, borders, padding and content layout remain application styles.
The default panel scrolls vertically when content exceeds its bounds.

`View.sheet` uses the existing modal focus scope and restoration policy. Escape
and outside-pointer dismissal default to enabled and can be configured separately.
Changing edge/extent with the same key keeps the scope and native children alive.
A dismissal is an asynchronous request; until the application submits an accepted
close, the trap and backdrop remain active. Nested popups keep existing hit-surface
registration and innermost Escape handling.

`Alert_dialog.Config` exposes label, width (default 480) and Escape policy. Outside
pointer dismissal is deliberately unavailable, and native admission rejects wire
configurations that enable it. `View.alert_dialog` uses the AlertDialog accessibility
role, the existing modal host and the same application-controlled close contract.
It enters the first eligible control. Put a safe/cancel action first in content
order for destructive confirmation. Confirmation is an ordinary explicit button
action; the panel has no implicit Enter-to-confirm behavior. Escape never confirms.

Both constructors accept optional content: `None` immediately unmounts native
resources, while OCaml models and Eio tasks follow their own declared lifetimes.
No exit animation retains removed content; neither adapter allocates a new timer,
queue, focus manager or native application callback. Retained hidden ancestors use
the existing focus/visibility gate, which omits deferred surfaces while hidden.

The existing overlay wire layout is unchanged. Appended kind tags are SheetLeft=2,
SheetRight=3, SheetTop=4, SheetBottom=5 and AlertDialog=6; Dialog=0 and Popover=1 are
unchanged. The existing internal `width` scalar carries sheet extent. Public OCaml
APIs use the accurate `extent` name. Every modal kind requires a trapped focus
scope at native admission. Existing window/node/editor quotas still apply.

The public Navigation Lab demonstrates four drawer edges and nested safe-first
confirmation. Local native and public evidence belongs in the OCH-37 evidence
ledger; these adapters do not complete hover cards, carousel or the full ticket.

## Interactive hover cards

`Hover_card.Config` shares the `Managed { initially_open } | Controlled bool`
ownership vocabulary with tooltips. It defaults to a 320-pixel panel, Top/Center
placement with a six-pixel gap, 600 ms opening delay and 300 ms closing delay.
Both delays are validated in 0..60 seconds. Content is interactive; cards neither
consume nor populate the separate tooltip grace clock. Native admission enforces
interactive content and zero grace even for raw protocol clients.

`View.hover_card` has a focusable `anchor` and arbitrary `content`, with optional
`on_open_change`. Its panel uses nonmodal Dialog semantics instead of Tooltip,
and its label is not copied into the anchor's tooltip-help description. Hover
opens without changing focus. Keyboard focus opens immediately; Tab can enter
content and leave the card normally. Pointer movement across the trigger/panel
gap is tolerated by the hide delay. An editor, select or nested overlay handles
its own Escape before the card does. Escape, trigger pointer-down and outside
pointer-down request closure. An accepted close of focused content restores the
first eligible painted trigger control when available, otherwise using the
normal enclosing/window fallback. It does not steal outside focus.
Restoration does not request reopening; a fresh entry after leaving can reopen.

Managed changes are native observations; Controlled changes are intents and the
visible value follows the accepted application update. Initial managed visibility
is mount-only, and switching back to Managed uses the accepted visible value.
Closing retains content descriptions and native editor buffers. Hidden content
is excluded from focus/input/accessibility. Unmount disposes resources; Bonsai
computation and Eio task lifetimes remain explicit application decisions.

The existing tooltip state/placement/focus-surface adapter provides one cancellable
native deadline per mounted owner, weak host/generational addressing and no idle
polling. Hiding an ancestor or unmounting cancels pending deadlines. Card closure
also preserves parent-overlay hit routing. Original tooltip semantics remain in
their separate Kind=11 path. HoverCard appends Kind=46 while intentionally reusing
the existing SetTooltip configuration and TooltipOpenChanged event encodings; the
public OCaml names remain `Hover_card` and `on_open_change`.

The Navigation Lab adds a controlled contributor preview with a retained native
note editor. The public self-test covers open/close focus eligibility and retained
text; the dedicated native suite covers actual keyboard/pointer/IME/AX and timer
behavior. Linux native-suite compilation is wired to the required build gate;
Linux graphical acceptance remains OCH-17.

## Carousel selection and native presentation

`Carousel.t` owns an ordered collection of labelled, stable-ID items and the
selected item. It is a value model; payloads, Bonsai state and Eio tasks stay in
the application. The limits are 128 items and 256 KiB of IDs/labels, independently
of native resource quotas. Reordering preserves the selected ID. Removing it
selects the old position clamped to the remaining collection. Empty collections
have no selection; refilling selects the first item. Single-item collections
cannot advance, including in looping mode.

Manual `Previous`, `Next`, `First`, `Last` and `Select` requests reduce against
the latest application model. They are ordered requests, not coalesced selected
snapshots. Disabled models ignore requests; explicit application selection is
still permitted. Relative movement records its direction, so last-to-first Next
and first-to-last Previous can animate consistently with the user's action.
Direct selection leaves the presenter to infer direction from positions.

Automatic advancement is opt-in, defaults to five seconds and accepts intervals
from one second through one hour, rounded up to milliseconds. Its intended native
eligibility requires settled visible paint, an active visible window, no hover,
focus or drag within the component, and normal motion policy. Pausing cancels the
deadline; resuming waits a full interval without catching up. At most one automatic
proposal waits for an application response. Ignoring it cannot accumulate events.
`restart_auto_advance` explicitly rearms after a rejected proposal.

An automatic request carries model revision, source ID and successor ID. Selection,
ordered membership, looping, disabled and automatic policy changes advance a
checked revision; same-order payload/label refreshes preserve it. A full loop
cannot revive an old request. Revision exhaustion returns an error without wrap.
A freshly created model begins a new lineage and requires a fresh mounted key.
The mounted adapter must reject backwards revisions and changed logical state at
the same revision; presentation-only changes do not create a new selection owner.

The pure native clock owns one immutable epoch ticket and a bounded schedule of
revision, generational source/target node IDs and interval. Redraws preserve its
absolute deadline. Old tickets, early wakes, disposal and unacknowledged proposals
cannot generate a second tick. The future host adapter must recheck eligibility
before waking and own/cancel the actual task through a weak, generational host.

`View.carousel` in Core and Bonsai wraps a keyed native NavigationStack viewport
and optional ordinary controls. `show_controls` defaults to true; first/previous,
bounded numbered pagination, next/last controls emit typed application requests.
Numbered controls use stable item IDs in their own key namespace, accepting even
maximum-length IDs or IDs equal to boundary-control names. Current-item metadata
is distinct from focus/selection and does not disable the current button. Controls
and viewport/page styles are independently customizable. Assign the owner or
viewport a height; the viewport can use flex allocation. `Carousel.Motion` shares
the navigation motion vocabulary, with horizontal/vertical axis selected separately.

The native root is Kind=47, SetCarousel appends operation=49, and CarouselRequested
appends event=54. Independent fixtures cover the envelopes without changing earlier
tags. The root owns a required request handler and one viewport plus optional
controls. Admission verifies its item count/selection against the viewport, including
viewport-only updates that otherwise leave the owner unchanged. Config revisions
cannot regress or change logical state at equal revision. Axis changes are allowed
at equal revision and settle an existing slide, including owner-only updates.
Retained config bytes include the bounded item ID storage in ordinary tree quotas.

Requests share the existing ordered bounded input mailbox. Generational node,
handler, window and accepted tree revision checks precede delivery. Relative manual
intents are not discarded solely because the last rendered snapshot is at an edge:
a preceding queued request can change the model before OCaml reduces them. Unknown
absolute IDs, disabled/empty owners and obsolete automatic proposals are rejected.
A full mailbox follows the existing explicit overload contract rather than silently
coalescing navigation. The reconciler repeats automatic/config guards against the
latest accepted model and rejects stale model reuse on the same mounted key.

Presentation reuses the bounded current/outgoing page layers and native ownership.
Incoming direction honors explicit Next/Previous through loop boundaries; direct
selection uses page positions. Outgoing paint is inert immediately. Carousel focus
restoration runs only when focus was in the departing page; controls and the
surrounding application keep focus during selection changes. Native hidden-page
input rejection, retained editors and immediate disposal are unchanged.

Implementation status: Core/Bonsai constructors, paired envelopes, admission,
request dispatch and mounted horizontal/vertical page presentation are implemented.
Local macOS tests cover presentation/focus/retained editor behavior. Keyboard/drag/
wheel input, host task/eligibility integration for the tested pure automatic clock,
public Navigation Lab scenarios and full acceptance remain pending. No carousel
capability is advertised. The full target still includes axis-locked pointer drag,
snapping and wheel gestures; these have not been replaced by the default buttons.
