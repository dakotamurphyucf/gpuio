# Calendars and date pickers — OCH-35

Status: implementation contract in progress. Civil-date models, bounded standalone
codecs, typed locale/configuration/command/observation contracts and a native
calendar policy owner pass local tests. Retained view descriptions, tree admission,
revision-checked event routing and atomic completion mailbox admission are connected.
The OCaml model also provides strict date formatting/parsing. A mounted GPUI
calendar now renders and passes initial macOS keyboard/pointer and event/lifetime
checks. The public Bonsai/Eio controller and correlated command bridge pass local
integration tests. Popup composition and full native acceptance remain required.
No calendar capability is advertised yet. See the
[foundation evidence](../evidence/calendar-och35.md).

## Civil values and bounded work

Public dates are `Core.Date.t`, interpreted in the proleptic Gregorian calendar
from 0001-01-01 through 9999-12-31, inclusive. They are never midnight timestamps.
No implicit time zone, system clock or locale participates in selection. The
application supplies an optional today marker and may update it using Eio.
The wire representation is a validated signed integer day ordinal, with
0001-01-01 = 0 and 9999-12-31 = 3652058. Unsupported values fail before conversion.

`Calendar.Month` is an abstract year/month, with checked offset navigation and
a bounded 6-by-7 day grid. Cells beyond the civil-date domain are blank, not
wrapped into another year. Month/year navigation materializes only the
current page, never an array spanning every supported year. Calendar display is
independent of selection: navigating does not select a date.

Selections distinguish empty, single date, range start and ordered complete
range. End-only and reversed ranges cannot be constructed. Mode is fixed for a
mounted placement. A first range activation starts a partial range; a later or
equal date completes it. Activating an earlier date restarts the partial range.
Activating after a completed range also starts a new partial range. These partial
changes are observable. Programmatic replacement emits an observation, not a
user completion event; clicking an unchanged single date does not fabricate a
change. Native keyboard focus is separate from selection.

Constraints contain inclusive bounds, at most 512 explicit disabled dates,
128 closed disabled intervals and 7 disabled weekdays. Input limits apply before
deduplication. Constructors validate and canonicalize lists; decoding must impose
the same bounds. Disabled intervals are sorted and merged. Empty allowed sets
are valid and produce a navigable calendar with no selectable dates.

Range policy is explicit: `Every_day` (default) disallows a disabled interior;
`Endpoints_only` permits one. Interval intersection and weekday arithmetic keep
range checks bounded by constraint count, independent of the number of days in
the selected range. Configuration updates preserve an existing selection and
report whether it remains allowed; they do not silently clear application data.
New selection commands and user activations must satisfy current constraints.

## Native ownership and integration

A retained Rust calendar owner holds selection, displayed month, day focus,
day/month/year presentation and interaction state. Typed commands support
replacement/clear, month navigation, focus/reveal and snapshot reads. Window/node
lease and revision checks follow existing numeric controllers. Mounting seeds
state once; ordinary Bonsai recomputation does not reset selection or navigation.
Native changes produce bounded asynchronous observations, including partial
selection, completed selection and navigation. Stale events cannot revive an old
selection after reset/remount or window close. No layout, day predicate or input
callback crosses synchronously into OCaml.

The inline calendar and popup picker share the same date model. The picker uses
existing overlay placement, nested Escape/outside-click routing, focus restoration
and accessible trigger behavior. Open/closed state is distinct from partial date
selection. An explicit application-selected initial/committed value remains
separate from any popup selection checkpoint; dismissal/confirmation semantics
must be documented and tested with the controller before acceptance. Hidden
calendars must not remain keyboard active or schedule idle redraws.

Locale data is presentation, not a second date representation: bounded month,
weekday and action labels; explicit first weekday; deterministic formatting.
Initial parsing formats are fixed-width ASCII `YYYY-MM-DD`, `DD/MM/YYYY` and
`MM/DD/YYYY`, with strict validity and no heuristic reinterpretation. There is no
implicit natural-language parsing, calendar-system conversion or time picker.
If editable date entry is exposed, it must reuse the existing native input
draft/IME/commit contract rather than translating every keystroke into a date.

### Concrete configuration and command contracts

`Calendar.Labels` contains 12 January-first month names, 7 Sunday-first weekday
names and 7 short weekday names, plus previous/next, choose-month/year, today and
clear action labels. All 32 labels are nonblank UTF-8, at most 128 bytes each,
without ASCII controls; no label is an executable formatting program. The first
weekday independently defaults to Monday. Configuration defaults to English
labels, single mode, unrestricted constraints and no today marker; its required
accessible label permits up to 4096 bytes under the same text rules. It carries
neither the selected value nor the initial displayed month.

The native owner takes an explicit initial month and validated seed selection.
Its keyboard cursor prefers a selected endpoint in that month, then the supplied
today marker in that month, then the first day. Cursor, displayed month and
selection are separate. Month navigation clamps the existing cursor day to the
target month (for example, leap day to February 28 next year); it never changes
selection. Day/month/year presentation is explicit. Focus loss preserves these
values and is reported separately from a selection change.

Snapshots carry a nonnegative native revision, mode, selection, selection-allowed
flag, displayed month, cursor date, presentation and actual focus. The cursor
must belong to the displayed month. Core observations additionally bind a
window/node lease. Empty selection is always allowed; a nonempty historical
selection may remain present while disallowed by newer constraints.

`Observed` covers initial/configuration/programmatic changes. `Changed` covers
native navigation/focus and selection changes, including partial ranges. A
changed complete native selection emits `Changed`, then `Selected` with the next
revision. Repeated unchanged single-date activation emits neither. Rejected date
activation preserves selection and navigation and emits an ordered `Rejected`.
No programmatic command emits `Selected`.

Commands are Replace/Clear (optional revision guard), Show_month/Move_months,
Focus_date/Focus, Set_presentation and Read_snapshot. Replace/Clear remain
available read-only/disabled, but replacements must satisfy current mode and
constraints. Navigation commands preserve selection; Focus_date reveals a date
and confirms native focus before mutating the owner. Disabled dates may receive
the discovery cursor but cannot be selected. Native read-only navigation remains
available; disabled or modal/hidden-blocked native interaction is denied.
Mode changes require remount. Configuration updates retain selection/navigation.

The owner reserves event revision capacity before any mutation or native focus
side effect, including both slots of a potential Changed/Selected pair. Exhaustion
is conservative even when an operation might be a no-op. Read_snapshot remains
available without advancing revisions. The GPUI adapter checks leases and actual focus gates, publishes completion pairs
atomically and faults on lost required output. Local mounted tests cover those
paths separately from standalone policy tests.

Standalone Rust decoders check full consumption and cap configurations at 24 KiB,
constraint blocks at 7100 bytes, selections at 19 bytes, events/responses at 128
bytes and commands at 64 bytes. Collection counts are checked before allocation;
domain conversion validates every date, interval, weekday, label and mode.
Valid raw duplicate constraints canonicalize on either side. Five independently
assembled fixtures cover configuration, constraints, selection, completion and
guarded replacement. Retained envelopes append kind 40, operation 46 and event 50;
two independent envelope fixtures preserve the same byte layout in both languages.
The Rust operation boxes the configuration so its locale payload does not enlarge
every unrelated operation in a transaction.

`View.calendar` uses its controller key as stable placement identity. Seed changes
alone emit no operation. Configuration changes retain the original seed selection
and displayed month in the native tree; mode changes require remounting. Initial
selection must satisfy constraints at first admission. Later constraints may make
a historical selection invalid without rejecting the configuration update.
Reconciliation rejects duplicate controllers, refreshes callbacks on acceptance and
shares the observed revision fence across pending preparations. Invalid events do
not advance that fence. Window/node/handler generations, tree revisions, snapshot
shape, mode and increasing native revisions all constrain delivery.

Calendar input events are currently discrete; no navigation or partial-selection
coalescing is claimed. The mailbox admits a complete `Changed`/`Selected` pair
atomically after validating matching routes, identical snapshots except for
consecutive revisions, and the entire pair's count/byte budget. Failed admission
preserves the old queue. Window-output accounting includes calendar observations.
The GPUI adapter uses this pair admission and faults the window on required-output
loss. A mounted-window test leaves only one input slot and verifies that native
range completion publishes neither half, emits one overload notification, and
blocks subsequent selection/navigation.

## Pinned upstream evaluation

Reviewed `vendor/gpui-base/src/calendar.rs` and `date_picker.rs`, from the pinned
GPUI Kit base source. `CalendarState::new` reads `Local::now`; `Date::Range` allows
end-only/reversed states; `CalendarEvent::Selected` omits partial ranges;
`Matcher::is_match` checks complete endpoints only. `year_range` materializes all
years, and month navigation is unchecked. `Calendar::render` invokes
`set_number_of_months`, which calls `cx.notify` unconditionally; its runtime idle
effect has not been measured and is not claimed as a verified upstream defect.

The chosen inline adapter is first-party GPUI elements over `calendar_state::State`,
in `rust/native/src/calendar_view.rs`; it does not copy or modify upstream calendar
code or adopt the upstream state contract. The private `CalendarItem::new` is not
called. The base `DatePicker` supplies a controlled focus/open root; trigger, popup,
calendar and placement are application responsibilities. Existing GPUIO overlays
already own those lifetimes; popup integration remains required. No whole styled-
library compatibility is inferred.

The native presentation materializes 42 day slots, 12 months or at most 20 years,
with blank slots beyond the civil domain. One composite focus handle keeps the
calendar in the application focus order and pins a focused managed-list row.
Day arrows move by one day/week; Home/End use the configured first weekday;
Page Up/Down move a month (Shift moves a year). Month arrows move by one/three
months, year arrows by one/four years; their page keys move a year/20 years.
M/Y/D switch month/year/day views; Enter/Space select the current date or confirm
the current month/year; Backspace/Delete clear selection. Tab/Escape remain
available to the surrounding focus/overlay system. Choosing a year opens its
month page; choosing a month opens days. Each choice is a single navigation event
and does not change selection. Today reveals the supplied marker without selecting.

Pointer activation uses release/click; mouse-down only establishes composite
focus. Configuration, live route, modal/visibility, pointer and edit permissions
are checked when actions run. Initial observation is emitted once, focus changes
are observed, and hidden/removed calendars release keyboard focus. No timer is
installed by the calendar. Actual idle, accessibility, scale/constrained-layout,
independent-window and managed-list acceptance still require dedicated checks.

The tree may admit a valid seed and then change constraints in the same transaction
before creating a GPUI entity. The private native `from_retained` constructor accepts
that previously validated historical seed and reports its current validity; public
policy construction and first tree admission still reject initially disabled seeds.
The real mounted test covers this case as well as ordinary retained updates.

## Required acceptance

Pure/paired-codec tests cover leap/century dates, civil limits, exact date parsing,
ordered and partial ranges, disabled endpoints/interiors/weekdays, bounded input,
month grids and malformed wire values. Native/public tests must cover keyboard
navigation, selection and events, month/year boundaries, locale/config changes,
modal/hidden/disabled/read-only behavior, popup dismissal and focus restoration,
stale commands/events, independent windows, scale/themes/accessibility, idle work
and repeated disposal. Include inline and popup public examples and final OCH-46
chat integration. macOS native acceptance and required Linux builds/tests follow
project policy; full Linux GUI release acceptance remains OCH-17.

## Public controller and correlated commands

`Gpuio_eio.Calendar.create` takes a window, reactive configuration, a seed
selection and an initial month. Place `Calendar.view controller` once. The
controller exposes snapshots and effects for reads, focus/reveal, month navigation,
presentation, replace and clear. `replace_if_unchanged` guards both the exact
native lease and the supplied observation revision. An unplaced controller has
`Not_mounted`; an old mounted lease returns `Stale_input` after removal/remount.
The retained controller can keep its last observation while unplaced; this does
not grant access to a replacement native owner.

Protocol message 17 carries correlation/window/node/command; event 51 returns
correlation/window/node/result. Independent fixtures freeze these tags. The Eio
adapter admits at most 64 pending calendar requests per application, validates
command conversion before queueing, and checks reply identity, mode and minimum
observed revision. Closing a window completes outstanding requests; already
admitted native reads remain ordered before close. Replies cannot replace a
newly mounted lease or a newer observation in the controller.

Explicit selection replacements remain available hidden/disabled/read-only,
subject to current mode and constraints. Focus and reveal require actual native
focus eligibility; failed reveal must not change navigation. Reads never advance
revisions. Native hide cleanup observes the platform focus directly: removing a
focused dispatch node must not leave a stale `focused=true` snapshot. Focus/blur
subscriptions also sample actual focus, making delayed cleanup idempotent.

The [Calendar Lab](../../examples/calendar/README.md) exercises both selection
modes and supplies a real-window bridge self-test. It does not yet demonstrate a
popup picker or establish external OS keyboard/accessibility acceptance.
