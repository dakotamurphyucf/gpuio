# Managed trees (OCH-38)

Status: the Core `Tree` collection and `Tree_state` expansion, selection, visible
projection and logical navigation reducer are implemented. `Tree_loading` now
provides a bounded Core paging model and scoped Eio worker adapter. `Tree_rows`
projects these models into an incremental keyed list collection with distinct
item and lazy-boundary identities. `Gpuio_bonsai.Tree_rows` now mounts bounded
rows and connects viewport demand/collapse policy to Eio loading controls. Paired
tree accessibility metadata and actual native outline/item semantics are now
implemented and locally tested. Opt-in native keyboard/pointer requests and
AppKit focus/selection now reach typed Core/Bonsai handlers. The explicit reveal
controller can hand focus to the same row after asynchronous mounting. Unicode
typeahead now reduces native text input against current Core labels. Per-row
accessibility selection and expansion setters are implemented. The public Bonsai
widget now owns preferences, ordered reduction and deferred reveal, with a locally
tested Eio filesystem example. Drag/move integration, context-action demonstration,
visual focus review and full native workload acceptance remain. No tree capability
is advertised.
This document preserves the full live ticket scope; semantic getters alone do not
establish interactive native widget acceptance.

## Existing implementation and adapter choice

The pinned `vendor/gpui-base/src/tree.rs` provides `Tree`, `TreeState`, `TreeItem`
and uniform-list rendering. It supplies useful keyboard and visual behavior, but
`set_items` clears index-based selection, expansion mutates shared native item
state, and rebuilding recursively clones a hierarchy into its visible entries.
It supports one selected index. Its render callback is synchronous Rust code.
A direct FFI wrapper would not meet stable application identity, controlled
expansion or asynchronous row construction. No change to the source pin is needed.

Build the adapter on the existing managed virtual-list layout, keyed row identity,
viewport requests, native focus pins and bounded Bonsai `Managed_rows.assoc`.
Use the pinned tree's hierarchy/key behavior as a reference, with GPUIO native
roles and style tokens. Rust renders already admitted row descriptions; an absent
row remains a native placeholder until the ordinary asynchronous bridge delivers
it. No layout callback enters OCaml and no filesystem work runs on the GPUI thread.

## Data, preferences and transient state

The Core collection is a validated immutable **flat keyed forest**. Stable public
`Tree.Id` values identify nodes; array offsets and native node handles do not.
Each record contains a label, disabled flag, arbitrary application payload and
children described as either a leaf or an ordered branch with a paging boundary.
An empty branch is distinct from a leaf and can start with `More None` for lazy
children. Roots are a separately ordered list. Payloads never cross the wire.

The constructor and structural updates reject duplicate IDs, missing references,
multiple parents, roots that are also children, unreachable nodes and cycles.
Iterative validation enforces a depth bound without recursive traversal of
untrusted input. A rejected update changes no collection, preference or lease.
Sibling reorder preserves IDs and payloads; moving a subtree is an explicit
application update validated against the proposed forest, never a native mutation.

Initial implementation budgets: 100,000 loaded nodes, depth 128 (root depth 1),
256 UTF-8 bytes per ID, 4,096 UTF-8 bytes per label, and 8 MiB aggregate IDs/labels/
cursors. Branch cursors are opaque strings bounded to 4,096 bytes. Application
payload memory is outside these metadata bounds and remains application-owned.
Metadata accounting conservatively includes each node declaration and every
root/child ID reference, even when strings are physically shared by the caller.
Native row resources continue to obey the existing tree/session quotas. A tree
with 100,000 logical items must not create 100,000 retained native row nodes.

Expansion and selection are separate immutable preferences keyed by ID. They
survive row deactivation and native cache eviction. Removing a subtree prunes
its selected/expanded IDs; collapsing preserves descendant preferences but moves
an active descendant focus to the collapsed ancestor. Re-expansion restores the
preferences, not evicted transient row computation models. Disabled nodes cannot
be selected/expanded by user requests; explicit application state changes remain
possible. Leaf nodes cannot be expanded.

Provide Single and Multiple selection policy. Plain activation selects one,
platform toggle modifier changes one membership, Shift selects the visible range
from a stable anchor, and the combined modifier extends it. Structural changes
revalidate the anchor; an absent anchor falls back to the current focused node or
single target. Hidden selections may remain preferences, but keyboard focus and
range traversal use the current visible order. Selection and activation are
separate callbacks so a context action need not open a file.

The initial Core collection interface is implemented in `lib/core/tree.mli`;
this excerpt shows its main constructors and updates:

```ocaml
module Tree : sig
  module Id : sig
    type t
    include Comparator.S with type t := t
    val of_string : string -> t Or_error.t
    val to_string : t -> string
  end

  module Children : sig
    type t = Leaf | Branch of { ids : Id.t list; next : List_paging.Boundary.t }
  end

  module Node : sig
    type 'data t
    val create
      : label:string -> ?disabled:bool -> children:Children.t -> 'data
      -> 'data t Or_error.t
  end

  type 'data t
  val create : roots:Id.t list -> (Id.t * 'data Node.t) list -> 'data t Or_error.t
  val revision : _ t -> int64
  val find : 'data t -> Id.t -> 'data Node.t option
  val set_data : 'data t -> id:Id.t -> 'data -> 'data t Or_error.t
  val replace
    : 'data t -> roots:Id.t list -> (Id.t * 'data Node.t) list -> 'data t Or_error.t
end
```

Structural operations increment a checked revision and invalidate only affected
parent load generations. Data-only updates preserve visible order and identity;
they must not flatten the complete forest for each streamed payload fragment.
`Tree.Expert.incarnation` and `children_revision` expose collection-local
versions for the future loader. They are not globally unique tokens: an Eio
controller must pair them with its own identity/reset generation. The precise
preferences/request and incremental splice interfaces will be drafted beside
their model implementation before adding native tags.

## Persistent state and visible projection

`Tree_state` is separate from the payload-bearing `Tree`. It stores incarnation-
checked selected/expanded ID maps, a range anchor, a logical active ID, and one
cached visible order/index. It owns no payloads, row computations, OS focus handles
or asynchronous producers. A full selection of 100,000 logical items therefore
must not become 100,000 native focus pins; the native adapter still has to enforce
that contract during managed-row integration.

Constructors/programmatic setters validate duplicates, membership and leaf
expansion. Programmatic selection/expansion may include disabled or hidden nodes;
user selection, focus and expansion requests ignore them. Single mode treats all
selection gestures as replacement. Multiple supports toggle and visible-range
replacement/union, skipping disabled rows. Range anchors survive repeated Shift
movement; hidden anchors fall back to an eligible active ID, then the target.
Programmatic selection replacement and mode changes clear the range anchor.

`reconcile` removes absent/reincarnated preferences and expansion for nodes that
are now leaves. Collapsing preserves descendant preferences, while the logical
cursor moves to the closest eligible visible ancestor. Deletion uses surviving
old ancestors first, then the next enabled item at the former position, or the
last preceding enabled item. Empty/fully disabled orders clear it. Source resets
must create new state because collection-local incarnation numbers do not cross
independent `Tree.create` lineages.

The visible projection rebuilds after hierarchy/expansion changes. It contains
loaded node IDs and minimal parent/incarnation/disabled metadata, not payloads.
Payload-only changes share `Tree.preorder`, allowing state reconciliation to return
the existing snapshot. Selection and focus operations share visible metadata.
`Tree_state.navigate` implements Previous/Next/First/Last/Parent/Child with optional
selection gestures: it can move the cursor without changing selection, extend a
range, or collapse/expand while preserving selection. It is a pure reducer. The
managed row primitive supplies native input and explicit reveal/focus requests;
`Gpuio_bonsai.Tree` connects these outcomes, typeahead and default presentation.

### Incremental managed-row data

`Tree_rows` owns one current loader snapshot, reconciled preferences and an
OCH-13 `List_collection`. Its Item records carry the node wrapper, hierarchy
position, selected/expanded/logical-active state and branch loading status.
An expanded incomplete branch adds one Boundary row after all its loaded
descendants. Boundaries are loading/retry controls, never application nodes or
members of selection ranges/sibling totals. Completed branches have no boundary.
The maximum projection is 200,000 logical rows for 100,000 loaded nodes, within
the existing managed-order limit. These records do not create native views.

Application IDs may use the full 256-byte key allowance. Prefixing them for
synthetic rows would overflow that allowance; hashing would require collision
handling. Instead, compact projection keys contain loader generation and a
checked monotonic serial. Separate item/boundary maps hold only currently visible
IDs and their source incarnations. Reorder and data/status changes preserve keys;
collapse, deletion and reincarnation retire them. Reappearance gets a fresh key.
There is no historical identity map. Snapshot ownership distinguishes separate
loaders even when both have numeric generation zero; projection updates reject
foreign owners. A reset creates a fresh projection and
preferences, and the managed component must reset its mounted generation too.
Delayed commands must resolve their original key against the current projection.

Hierarchy/expansion changes rebuild positional metadata. Point updates use
`Tree.fold_changed_nodes`, `Tree_state.fold_changed_items` and
`Tree_loading.Snapshot.fold_changed_statuses`, backed by persistent map sharing.
They do not compare arbitrary payloads, scan all selected items on a cursor move,
or reconstruct all visible records on a streamed payload fragment. Updated rows
retain collection order and unaffected value wrappers; hidden payload changes
only update the owned source snapshot. Status invalidation includes error-detail
eviction, so a mounted failed row switches to the generic retry message when its
detail leaves the bounded cache. Historical projections retained explicitly by
applications retain their historical payloads; the current projection has no
back-reference chain. The Bonsai primitive supplies the OCH-13 lifetime/viewport
layer described below.

### Bonsai viewport and row lifetimes

`Gpuio_bonsai.Tree_rows.component` is the managed primitive for the higher-level
native tree adapter. It accepts loader snapshots, application-owned preferences,
bounded list configuration and a row renderer. The renderer receives typed Item/
Boundary data, a projection key and `Managed_rows.Lifetime`; its transient models
must obey the same reset/guard contract as OCH-13. Optional `accessibility` reaches
the actual native list root; `Accessibility.Role.Tree multiple` marks the tree.
Renderers annotate their row containers with `Tree_rows.Item.accessibility`. The
managed envelope lifts this metadata to the native row focus owner, avoiding a
duplicate TreeItem on the inner presentation. Boundary rows remain ordinary
status/action content, not application tree items. `on_request` explicitly enables
native input and receives identity-checked `Tree_interaction.Request` values;
applications reduce them against their latest snapshot/state. The managed primitive
does not apply preferences implicitly. `Controller.reveal ~focus:true` requests
scrolling and eventual focus of a current projection key. The high-level adapter
connects logical outcomes and ancestor expansion after displaying the new projection.

The component checkpoints its accepted projection after display. Coalesced source
changes compare with that checkpoint, and the existing list adapter independently
checks height invalidation against its own accepted collection. Persistent
selection is never converted into row retention. Explicit pins and native
viewport/interaction demand share `Config.max_active`; only that subset creates
row computations. A 100,000-node selection with a four-row configuration therefore
still mounts at most four rows, including native pins.

Source generations use a comparable, payload-free `Tree_loading.Lease`. Its owner
comes from a generative Core type identity, with no payload or historical registry;
the reset counter distinguishes successive generations of that owner. Bonsai
keys a managed subtree by the full lease, and the native wrapper has a distinct
source key. Independent controllers at numeric generation zero cannot share
row models. Deactivation resets projection/list/row models and retires controller
effects even if the same source is subsequently remounted. The data loader belongs
to the application scope and is not closed by removing this view.

The primitive controller reveals a current visible projection key and provides
request/retry/cancel effects for a lightweight branch `Target`. Reveal does not
expand ancestors or assert OS focus; the higher-level tree controller will add
those semantics. Delivery peeks at current projection state and checks lifetime,
branch identity and visibility. A target holds only source lease, stable parent
ID, incarnation and child revision. Its equality and currentness do not retain
application payloads; page-prefix changes invalidate it, while payload edits and
sibling reorder preserve it.

`Gpuio_eio.Tree_loading.controls` checks tokens again on the UI domain. Closed,
reset, foreign, deleted and obsolete branch delivery is ignored. Queue saturation
leaves Ready/Failed unchanged as backpressure. Worker admission and producer
failures publish Failed; exceptional remaining control errors use `on_error`
(by default raised at the UI effect boundary). Building controls starts no work.

After display, visible Ready boundary rows request pages up to the remaining
queue capacity and active-row budget. Failure requires explicit retry. Demand
callbacks peek at the latest policy/state before acting, so queued observation
effects do not restore an old expansion decision. `auto_load=false` suspends new
automatic requests; `cancel_hidden=false` opts into deliberate background prefetch.
Default collapse cancellation releases hidden branch requests while preserving
accepted data. Unmounting only retires view effects: in-flight application-owned
loads can finish and provide their data to a later mounted view.

Typical primitive composition (with renderer/state supplied by the tree adapter):

```ocaml
Gpuio_bonsai.Tree_rows.component
  (Gpuio_eio.Tree_loading.value loader)
  ~state
  ~loading:(Bonsai.Cont.return (Gpuio_eio.Tree_loading.controls loader))
  ~config
  ~render_row
  graph
```

## Keyboard, focus and accessibility

Native Up/Down/Home/End, Left/Right, Space/Enter, modifiers and bounded typeahead
produce ordered typed intents. Left collapses an expanded branch or moves to its
parent; Right expands a branch or moves to its first visible child. Application
reduction uses current stable IDs and validated order. Typeahead searches labels
in the visible loaded order, wraps once, and uses a bounded native prefix timeout;
it does not fetch every unloaded subtree or synchronously call OCaml.

Rust keeps native focus/selection paint and applies accepted focus/reveal commands
through the managed list. Logical focus can target a not-yet-mounted row: reveal
requests it, and focus waits for that same key/generation to mount. Obsolete
commands after deletion, collection replacement or window close are discarded.
A pending target never silently turns into the row now at its old numeric index.

The tree root and row expose Tree/TreeItem semantics, hierarchy level, expanded,
selected, disabled and loading state, with sibling position/count when known.
Unknown lazy sibling totals must not be invented. The implemented metadata path
uses validated levels 1–128, zero-based sibling indices and optional known counts.
The pinned macOS adapter maps levels to AppKit's zero-based disclosure level and
exposes disclosed state only for branches. Its small reproducible patch is recorded
in `vendor/accesskit-macos/GPUIO.md`; dependency versions remain unchanged. Native
getters, update/removal and teardown pass locally. Accessibility actions still
require explicit input opt-in. AppKit focus and press-to-select are locally
verified, including per-item desired expansion and selection setters.
Metadata alone does not enable input.

Native accessibility actions
use the same intents and generational admission as pointer/keyboard input.
Rows may compose existing context menus and drag/drop descriptions. Child controls
retain their own key handling; tree traversal must not steal an embedded editor's
arrows or first IME Escape. Inline rename and editable cells are outside this ticket.

## Generation-checked interaction reduction

`Tree_interaction` now provides the pure request contract for the pending native
adapter. A `Target` holds the loader lease, stable ID and node incarnation, with
no snapshot or payload. Unlike a child-page target, it survives accepted child
paging. Payload updates and reorder preserve it; source reset, foreign ownership
and deletion/recreation retire it. A separate mounted lifetime guard is still
required before a queued view event reaches this reducer.

Relative navigation requests carry a source lease and reduce against the latest
logical cursor in delivery order. Targeted selection/focus/activation require a
current visible enabled item. Expansion requests specify the desired Boolean
state, so a repeated accessibility expansion is idempotent. Collapsing an active
ancestor reports the repaired logical cursor as a new reveal/focus target.
Activation is an explicit action and does not silently change selection.

Programmatic reveal opens the loaded target's ancestors, preserves selection and
range anchor, and optionally changes the logical cursor. It may expand disabled
ancestors as application preferences but rejects a disabled target. It performs
no I/O and cannot discover an unloaded ID. Already-open paths inspect at most the
128 ancestors, rather than scanning all expansion preferences; a changed path
rebuilds the visible projection. The outcome reports a stable reveal target and
focus flag. The explicit managed controller provides native scrolling and eventual
mounted focus; the high-level adapter connects these outcomes after display. The
native adapter rechecks the same target before a delayed focus handoff.

A Move action is an application-approved proposal with source and destination
targets plus Before/After/Inside placement. Reduction never modifies the hierarchy.
Both endpoints must be current, visible and enabled; Inside requires a branch;
self/descendant destinations reject. `Move.is_current` lets asynchronous approval
recheck identity, visibility and topology immediately before the application
constructs a validated `Tree.replace`. Collapsing or deleting an endpoint therefore
cannot redirect an old proposal to a different item.

Native input now appends operation tag 50 (`Set_tree_input`) and event tag 55
(`Tree_input`) without changing existing tags. Opt-in requires a VirtualList node,
Tree metadata and a live handler. Reconciliation rotates the handler when enabling
or disabling input, so a queued event cannot revive after disable/re-enable.
The event path translates monotonic native row IDs through current `List_identity`
and the Bonsai list's existing key map. Tree rows then check mounted lifetime and
the latest projection before capturing the application target. Removed/recreated
rows cannot inherit an old event; relative requests remain ordered in the bounded
input mailbox and reduce against the latest logical cursor.

The exact root/row focus handles own arrows, Home/End, Space and Enter. Shift
requests a range, the platform toggle modifier supports union or cursor-only
movement, and Enter requests activation separately. Embedded native widgets keep
keyboard/IME ownership; tree-row native controls block ancestor pointer clicks but
allow wheel propagation. The root is one Tab stop; programmatically/pointer-focused
rows remain outside the Tab sequence. Inherited pointer policy and inert/modal
visibility still gate row requests. AppKit press selects rather than activating.
Per-row AppKit selection setters carry desired membership through nested request
tag 8 (`Set_selected`), rather than translating it to a click/toggle. Single mode
replaces selection when selecting and removes only that item when deselecting;
Multiple adds/removes only that item's membership. Both preserve the logical
cursor and range anchor, with no implicit reveal, focus or activation. The same
current-visible-enabled/source-incarnation checks apply as for other user input.

The pinned macOS adapter advertises expanded/disclosed setters only for enabled
branch TreeItems with Expand/Collapse handlers. It uses two declared CustomAction
IDs for desired selection because the pinned AccessKit has no SetSelected action;
only opted-in rows with both IDs use that path. Generic Press/Click behavior and
unrelated roles are unchanged. The adapter queues even equal desired states so
opposite requests before the next render retain their order; Core reduction is
idempotent. The reproducible vendor patch and native AppKit evidence document this
private platform contract. This does not claim VoiceOver speech or external AX
notification-observer acceptance. Drag sessions still need implementation and native acceptance. High-level
outcome/focus integration is covered by the public widget and filesystem example.

### Unicode typeahead

The native adapter sends printable UTF-8 text as nested request tag 7, together
with reset and repeated-prefix cycling flags. Rust owns the last-input `Instant`
and handler epoch, with a one-second expiry checked on the next input. No timer,
task, label index or search prefix lives in Rust. Navigation, focus leaving the
tree, window deactivation, hidden layout and handler retirement reset the clock.
Exact tree root/row focus owns text; embedded editors retain their own keyboard
and IME handling. Platform/control/function shortcuts are excluded. Option-key
Unicode text is accepted when GPUI supplies `key_char`. A single Unicode grapheme
permits repeated-prefix cycling, including a joined emoji sequence.

Core `Tree_typeahead` retains only one prefix of at most 256 UTF-8 bytes. Both
incoming text and its normalized form must fit this bound; controls and invalid
UTF-8 are rejected. Matching uses Unicode 17 NFD, default case folding, then NFD,
via explicit runtime dependencies `uucp` and `uunf` 17.0.0. Canonically equivalent
accents match, accents remain significant, and matching is locale-independent.
Normalization also applies across keystroke boundaries, including combining-mark
reordering. The isolated lock adds these runtime requirements without changing
the accepted OCaml, Bonsai or formatter versions.

Search visits the current loaded visible order, skips disabled items and wraps
once. A fresh or repeated-prefix search starts after the active item. An extended
prefix first considers that item; a miss retries using only the latest input.
Prefix overflow similarly restarts. A match replaces selection and produces a
focus/reveal outcome; a miss preserves the cursor and requests neither focus nor
activation. Unloaded branches are never fetched for typeahead. Queued requests
reduce in delivery order against current state and labels under the existing
source and mounted-lifetime guards.

There is no persistent label cache. An ASCII prefix fast path avoids normalization;
otherwise two normalizers are reused within one search and stream each label only
until the prefix decides its match. Worst-case search remains linear in visible
items and examined label content. The evidence records a 100,000-node Core
benchmark separately from the still-pending full native workload; these timings
are measurements, not an end-to-end latency guarantee.

### Deferred native row focus

`Tree_rows.Controller.reveal ~focus:true` uses the list controller's
`focus_tree_row` command. It appends nested Scroll_target tag 3 to the existing
serialled Scroll_list operation; older offset/reveal/end encodings are unchanged.
Core and native admission require opted-in tree input. Bonsai checks the current
mounted projection key and source lifetime; native commands use the monotonic
logical row ID, never a transient list position.

An accepted new focus command reveals the logical row and focuses the tree
surface immediately, releasing the previous row's focus pin. This lets even a
one-row active budget request the destination. Pending focus has priority over
other visible rows within the same budget, including a partially visible leading
row. It adds no retention pin or extra active-row allocation. Rust retains one
pending row/serial/handler and two scoped subscriptions per list (blur and window
activation), with no timer or polling loop. Repeated serials do nothing; every
newer scroll request supersedes a pending focus request.

The actual row receives focus only when its current enabled TreeItem is
materialized in the visible native layout, intersecting the content mask and
window viewport. A fully clipped or zero-size layout retires pending focus during
prepaint. Sibling reorder preserves the target
and reveals its new position. Logical removal, input/handler retirement, disabling
or hiding the target/list, user blur, window deactivation and scrolling away retire
the request. Hiding retires it during transaction synchronization, without waiting
for an invisible paint. Restoring visibility or returning focus cannot revive it.
An inactive window may scroll but is never activated by this command. Mounted
subscriptions disappear on completion, cancellation, list removal or window close.

This controller requests focus; it does not acknowledge an OS focus change or
modify tree selection. The high-level widget applies the latest
Tree_interaction outcome, expands loaded ancestors where requested, obtains the
current projection key after that update, and then invokes reveal/focus. Arbitrary
old projection keys are intentionally rejected after collapse/reappearance.

## Public Bonsai widget

`Gpuio_bonsai.Tree.component` takes a reactive loader snapshot, list configuration,
accessible label and optional loading controls. Its output provides the view,
current preferences/projection, active-row counts, viewport and a controller.
Default rows include indentation, selection paint, ellipsized labels and pointer
hit regions for disclosure. Disclosure has no additional Tab stop; the native
TreeItem owns keyboard/accessibility interaction. A custom `render_item` supplies
content inside this chrome and receives a target, controller and guarded row
lifetime. Loading/error/retry boundaries remain separate from TreeItems.

The widget owns preferences for one mounted source lease. Reactive initial
selection/expansion inputs seed that generation once; later changes do not
replace user choices. Mode remains controlled and reconciles selection. Applications
can observe `Output.state` and persist deliberate preferences outside the widget.
A source reset or unmount retires transient models and old controller effects.
Neither event cancels the independently owned application loading scope.

Controllers require an explicit payload-free `Target`, captured through
`Output.target` before constructing delayed commands. They hold no historical
snapshot; absent IDs fail capture, and stale, removed or reincarnated targets are
ignored when delivered. Holding an entire Output intentionally retains its source.
Native and controller requests reduce in order against the latest source/state.
Relative toggle requests preserve two queued clicks, while desired expansion and
selection setters remain idempotent. Only activation and approved-move proposals
reach `on_action`; this callback never implicitly changes the application tree.

Reveal expands loaded ancestors before native scrolling/focus. After display, a
single pending target/serial is checked against the current model and projection;
a newer reveal supersedes it, and stale or ineligible targets are dropped. The
native adapter independently checks asynchronous focus ownership. This sequencing
avoids passing a row key from the projection that preceded expansion.

With loading controls and automatic demand enabled, newly opened eligible Ready
branches request their first page even when their boundary lies just below the
viewport. This is bounded by available loading queue slots and the configured
active-row budget; later pages follow viewport demand. Failed pages require
explicit retry. Loading effects run outside Bonsai evaluation and retain the
loader's own generation checks and worker limits.

The [filesystem example](../../examples/tree/README.md) uses an Eio directory
capability and pages of at most 128 children. Symlinks are leaves. It demonstrates
mode changes, activation, reset, native focus and selection, with 32 active rows.
It does not watch files, rename/move entries or promise stable paging across
filesystem changes. Directory-read allocations/application payloads are outside
native row budgets; the documented metadata/ID limits still apply.

## Lazy children and asynchronous ownership

`Gpuio.Tree_loading` and `Gpuio_eio.Tree_loading` reuse OCH-13's `More cursor`/`End`,
explicit retry, progress and obsolete-completion rules. Status adds Queued to
Ready/Loading/End/Failed. A controller admits up to 64 queued branches and four
running requests, with at most one request per branch. Pages contain at most
2,048 new nodes as a closed forest, whose roots append to that parent's children.
Reusing an existing ID or exceeding the final combined depth/metadata budget fails
atomically. The controller adopts updates from its latest collection lineage;
reset is explicit and increments its independent generation. A completion
token carries controller identity, collection generation, parent incarnation and
request serial. Reusing a deleted public key does not resurrect its old request.

Opening a branch requests its missing child page asynchronously. Loading/error/
retry are real bounded rows in the visible projection, with synthetic identities
outside the application ID namespace. Empty pages must reach End or advance the
cursor. Duplicate/cyclic current responses fail atomically and become retryable;
obsolete responses are ignored without importing their records into the forest.

Collapse cancels outstanding loads under the collapsed branch by default and
invalidates queued completions; it preserves already accepted child data. Deletion,
reset and scope close cancel affected producers and retire their tokens. An
application can keep a separate data-service scope when it intentionally wants
background fetches to survive visibility; native row lifetime never chooses that
policy implicitly. In-flight data requests are independent of transient row scopes.

The Eio adapter lazily allocates at most four long-lived scoped workers, each
waiting on a one-slot request stream while idle. Every load runs in a nested
cancellation context. Cancelling a branch immediately invalidates its token but
keeps the worker busy until its producer has exited and returned the slot through
the bounded UI inbox. Rapid cancellation/reopening therefore cannot exceed the
physical producer bound. Delivery runs outside the request cancellation context
and inside the outer scoped task, so backpressure remains cancellable at shutdown.
No polling, timer, per-node idle fiber or scheduler-library patch is introduced.

A successful load, invalid page or producer failure updates immutable snapshots
on the UI loop; `value` publishes those through Bonsai. Actual I/O captures Eio
capabilities in `load`. Scope/worker admission failures become retryable Failed
snapshots. Close cancels only this controller's workers, unregisters cleanup and
releases queued work/failure details from both model and published snapshot;
the latest application tree remains readable. The parent scope's other tasks live
on. Unchanged cancellation/observation does not publish another snapshot.

Failure markers are bounded by loaded node count. Detailed error strings have a
separate FIFO cache of 64 entries, each at most 4,096 UTF-8 bytes. NUL and invalid
UTF-8 messages are sanitized. Evicted detail leaves a generic Failed status;
scrolling must not accidentally retry an old failure. These are retained-storage
bounds, not a bound on arbitrary application payloads or temporary error formatting.
Historical snapshots explicitly retained by the application have its lifetime.

Payload updates preserve valid loads. When application data changes the external
parameters used by a loader, call `invalidate` explicitly; the controller does not
compare arbitrary payloads to guess that a filesystem path or service changed.
`cancel_hidden` reconciles the supplied `Tree_state` and applies the default
collapse policy. Explicit prefetch may omit it. The managed component now connects
controlled preference changes and viewport demand to these operations after
display. Native input still needs to produce the higher-level tree requests.

The filesystem example receives an Eio directory capability. It loads children in
scoped producers, uses deterministic ordering and explicit failure/retry, and does
not follow symlink cycles implicitly. Window closure cancels its loader. No host
filesystem path is treated as a native callback or ambient Rust capability.

## Transport and resource plan

Retain OCH-13's logical order/viewport protocol and per-row admission wherever
possible. Do not serialize the full hierarchy/payload into every visible row or
redraw. The implemented accessibility extension appends Role tags 12 (Tree with
multiple-selection policy) and 13 (TreeItem with level, sibling index/optional count,
optional expanded state, selected, disabled and busy flags). Existing tags and
Config layout are unchanged. Independent OCaml/Rust fixtures and strict decoding
cover bounds and malformed input. Tree metadata is supported on VirtualList roots,
TreeItem on container rows. Native key intents may request pure
OCaml reduction asynchronously, allowing offscreen typeahead/ancestor lookup
without uploading a duplicate label index. Measure this path's latency under load.

Updates and commands carry checked revision/generation identity. Relative intents
remain ordered; focus/reveal and drag move intents validate their original target
against the accepted forest. A drag captures stable source IDs and destination
relation (before/after/inside), not indices. Collapse/deletion/replacement during
an active drag cancels invalid targets and never changes hierarchy silently.

Requested/overscanned/pinned rows share `Virtual_list.Config.max_active`; default
admission will target 256 active rows. Persistent selection does not pin every
selected row. Focus/composition/active interaction pins are bounded and released
on teardown. Cached row content must remain bounded across a full traversal and
revisit, following the existing managed-row reset and `Lifetime.guard` contract.
No capability bit is added until public and native acceptance pass.

## Required implementation and acceptance sequence

1. Pure collection/preferences and generation-checked load model, with expect
   tests for malformed forests, 100,000 nodes, maximum depth, reorder, collapse,
   removal/reuse, selection/range anchors, stale commands and atomic failure.
2. Core/Bonsai tree projection and Eio loader interfaces over existing managed-row
   and paging machinery; no new scheduler or source dependency pins.
3. Native row semantics, tree keyboard/typeahead/focus and move intent adapter;
   paired fixtures, admission limits and real macOS keyboard/pointer/IME/AX tests.
4. Public filesystem explorer: Eio capabilities, loading/error/retry, context
   actions, approved move demonstration and explicit task/row lifetime examples.
5. Full traversal/revisit, deep branches, collapse/deletion during load/drag,
   resize/reveal and window-close tests prove bounded metadata/cache/native rows
   and cancelled timers/producers. Then advertise the family and record evidence.

Consolidated hosted macOS/Linux builds/tests and milestone merge remain required.
Full Linux GUI validation stays in OCH-17. OCH-46 incorporates the finished tree
into the polished chat application's project/artifact navigation.
