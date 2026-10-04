# Pagination and breadcrumb presentation

OCH-41, 2026-10-02. Extends the accepted OCH-37 navigation model without
introducing native navigation history or a new wire operation.

## Stateless Core compositions

`Navigation.breadcrumbs` accepts a pure `is_navigable` predicate and `item_style`
function for each `Choice`. Passive intermediate members are text without a
callback. Disabled links retain disabled semantics. The last member remains
passive with current-location metadata. Application route changes rebuild the
path with stable IDs; the helper never resolves a route or owns application state.
Per-member styles follow the shared `Appearance` overrides.

`Navigation.pagination` retains its bounded full layout and now offers
`Pagination_layout.Compact`: two arrows with localized previous/next accessible
labels. Their native identities survive layout switches. Compact does not invoke
hidden page/gap formatters. Current-page metadata remains independent of focus.

Full-layout gaps remain passive by default. Supplying `on_gap` makes an ellipsis
an ordinary button with an inclusive range and localized accessible label. The
callback receives the rendered interval; consumers must validate it against
current state. Disabled models suppress all interactive controls. A changed gap
has a different key and cannot receive an event through its retired handler.

`Navigation.Gap_popup.create` describes optional panel content, overlay config,
style and dismissal callback. Supplying `gap_popup` requires `on_gap`; invalid
combinations return an error. Each rendered gap retains a range-keyed popover
wrapper and its own direct native button, even when closed. The factory runs once
per visible full-layout gap; compact skips it. Disabled models suppress content.
The framework owns the button, so a caller cannot accidentally replace its
native activation, range label, disabled state or popup metadata.

## Managed chooser

`Gpuio_eio.Pagination.create` receives a reactive application-owned
`Gpuio.Pagination.t`, optional reactive layout and reactive request callback.
`view` renders interactive full-layout ellipses with an ordinary native popover.
The caller applies requests with `Pagination.apply_request` to the latest model.

An opening owns at most seven page shortcut buttons (near each end and at the
midpoint), one native numeric editor and Cancel/Go controls. Work and tree size
stay bounded even when an interval contains almost a billion pages. Shortcut
buttons request a page immediately. The field uses integer steps and bounds equal
to the gap, and native commit rounds/clamps according to `Numeric.Domain`.
Enter and arrow stepping normalize the draft without navigating. Go performs an
explicit asynchronous commit and then requests the normalized page. This avoids
mistaking a keyboard step for confirmation: both produce numeric keyboard commit
events. No OCaml callback runs in native layout, painting or editing.

The controller stores only the current opening, source model, latest observation,
confirmation state and optional command error. Every opening gets a fresh
monotonic identity and native controller key. A stale closing effect cannot
close a later opening. No page data or route payload is cached here.

Before opening, the reducer validates the captured model and the exact interval
against current `Pagination.items`. Source-model changes (including current page,
count, siblings or disabled policy), compact layout and deactivation discard the
opening. Rendering also synchronizes this condition, so a stale popup cannot
remain visible while waiting for an edge effect.

Concurrent Go actions coalesce. The command reads/commits the actual native draft;
it does not submit a captured OCaml string. Replies must match the opening,
editor owner, domain and a revision at least as new as the latest observation.
The draft must be settled, integral and within the original interval. Rejection
or active composition preserves the opening and reports an error. Closing,
choosing a shortcut, changing models or deactivating while awaiting a command
suppresses late navigation. The callback is sampled from current Bonsai input.

Native popover focus, dismissal and restoration are reused. A pending native
command may finish editing its original field before removal; cancellation does
not roll back such a native operation, but its reply cannot navigate or affect a
new editor. There is at most one in-flight confirmation per opening; opening a
new chooser does not bypass the App's shared bounded command admission.

## Presentation and limits

Navigation style, item/current/gap `Appearance`, localized pagination labels and
popup panel style remain application-controlled. The chooser's range text,
field label and Cancel/Go/error text are currently English; this is a documented
localization limitation. The caller supplies localized overlay metadata.

This is a functional bounded alternative to upstream's dropdown enumerating all
hidden pages. It does not reproduce that unbounded menu, upstream's exact
visible-page-count algorithm, or clickable final breadcrumbs. Pagination accepts
an empty dataset and up to one billion pages; the upstream minimum is one.
Current numbered pages remain actionable; the application reducer performs a
no-op when the selection is unchanged.

The gallery demonstrates live breadcrumb path navigation, passive workspace
labels and custom per-item styling, plus full/compact paging, disabled state,
120/billion-page counts, shrinking to three and clearing to zero. The existing
workflow-stepper gallery remains a separate component.


The managed chooser anchors each popup to its own gap button. Only the selected
gap has panel content; both buttons retain their native IDs and dialog-popup
metadata. Expanded state follows accepted panel visibility. Placement uses the
gap's bounds, not the navigation row. Direct-button popovers prefer that eligible
button when closing with focus still inside; this also covers semantic activation
and replacement while another chooser's editor is focused. If the trigger is no
longer eligible, ordinary previous-focus fallback applies. Closing never steals
focus after the user has moved outside. Custom anchors retain previous-focus
behavior without guessing a descendant trigger. Physical accessibility acceptance
remains open.
