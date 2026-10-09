# Calendar viewport observations

`Snapshot.month` remains the keyboard cursor month. A two-month calendar can move
the cursor from February into March while continuing to display February–March.
`Calendar.Viewport` separately reports the logical displayed panes.

Use `View.calendar ?on_viewport_change`, `Gpuio_eio.Calendar.view` with the same
option, or `Gpuio_eio.Date_picker.view` / `view_with_trigger` with
`?on_calendar_viewport_change`. The effect/action callback is asynchronous. It
receives an initial observation on subscription and changes thereafter. Ordinary
callback updates use the latest accepted callback without creating a subscription.
Removing and readding the callback creates a new subscription identity. Unmount,
picker session replacement and window close retire it.

`Viewport.Display` is a private variant:

- `Days { first_month; months; first_weekday }`: 1–12 consecutive panes, civil
  years 1–9999, with an explicit first weekday.
- `Months { year }`: the twelve month choices in one civil year.
- `Years { first; last }`: a 20-year aligned page, capped at year 9999.

`Viewport.months` returns day-pane months. `Viewport.dates` returns their sorted,
unique six-week-grid dates, including adjacent-month cells and excluding dates
outside the civil domain. Both return an empty list in month/year selection mode.
At most 504 grid dates are considered. This is a bounded prefetch set, not an
assertion that a date is selectable or that its pixels are on screen. Hidden
mounted calendars retain logical panes; hiding alone does not change the display.

An application can store the latest observation in its Bonsai model, load dates
through its Eio scope, and compare `Viewport.equal` with the observation captured
when starting the request before installing results. Equality includes native
owner, subscription generation and sequence. Clear the stored observation and
cancel the corresponding scope when the application removes the calendar; an
observation alone cannot prove its former owner is still mounted. To compare only
the logical content, compare `Viewport.display` using `Display.equal`.
The library does not initiate I/O. Missing rich day content uses native fallback.

## Native and bridge lifecycle

Append-only Op95 `Set_calendar_viewport_observer (node, handler option)` registers
an independent secondary handler on a Calendar node. It leaves the primary
calendar input handler, model snapshot and command formats unchanged. Event tag
71 is `Calendar_viewport_changed (window, node, handler, tree_revision, sample)`;
the sample contains a nonnegative monotonic sequence and checked display. Days,
Months and Years use tags 0, 1 and 2. The first month is the existing zero-based
civil month index; weekday is Sunday=0 through Saturday=6.

Native synchronization publishes after normal calendar model events. User
selection's Changed/Selected pair remains atomic and ordered. Command observation
publication precedes the command reply. Appearance month-count changes can publish
without changing selection revision; moving or focusing inside unchanged panes
cannot publish a duplicate. There are no polling or animation timers, synchronous
OCaml callbacks, or per-paint observations.

The native publisher retains one last display/subscription and a checked sequence.
A conservative additional 128 bytes per calendar is included in retained resource
accounting. Mailbox coalescing replaces only adjacent, valid, newer observations
for exactly the same window/node/handler/tree revision. It never crosses another
input or response boundary. Queue exhaustion faults the window rather than
silently losing selection events; sequence exhaustion also faults without wrap.
Core rejects stale owner/subscription IDs, duplicate sequences, future tree
revisions, malformed payloads and events after close. Its independent observation
filter never advances a selection revision or invalidates pending picker Apply.

## Source mapping and evidence

The pinned base CalendarEvent exposes Selected, not a dedicated viewport event.
Its public current-month/year and month-days getters with entity observation allow
native callers to inspect display state. This API provides that functionality
through an asynchronous serialized boundary; it does not reproduce an upstream
callback signature. See [local validation](../evidence/calendar-viewport-och41.md).
Physical desktop qualification remains separate from deterministic TestPlatform
rendering/ownership checks.

Shutdown regression coverage also guards the generic input lane and overload
reporting after a queued or drained `Stopped` marker. Previously, a late input
could append after that marker; the input gate now rejects it and a resulting
late overload report is ignored. Existing queued output still drains before stop.
