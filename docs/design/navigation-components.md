# Disclosure, navigation and supplementary overlays (OCH-37)

Status: implementation in progress. Pure application models compile and pass local expect tests;
new native component capability and acceptance are not yet advertised. This
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

Extend semantic roles/expanded/current metadata and the existing overlay variants
where needed. Preserve Dialog/Popover/Tooltip behavior and nested Escape/outside
routing. Do not declare a component complete from a styled column or model tests.
Paired codec fixtures are required for new wire contracts. Expect tests cover
history, disclosure and shrink/request races; actual macOS tests must cover nested
focus/IME, lazy/retained lifetimes, nested overlays, constrained themed layouts,
carousel timing/idle/disposal and public Bonsai/Eio usage. Linux builds/unit tests
and the consolidated macOS hosted checks remain required before milestone merge;
Linux GUI acceptance stays under OCH-17. OCH-46 integrates these into the chat demo.
