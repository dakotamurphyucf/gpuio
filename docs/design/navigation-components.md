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
