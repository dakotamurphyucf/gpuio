# Calendars and date pickers — OCH-35

Status: implementation contract in progress. Civil-date models, bounded standalone
codecs, typed locale/configuration/command/observation contracts and a native
calendar policy owner pass local tests. The OCaml model also provides strict date
formatting/parsing. Retained-tree/transport integration, GPUI rendering, public
controllers, popup composition and native acceptance remain required.
No calendar capability is advertised yet. See the
[foundation evidence](../evidence/calendar-och35.md).

## Civil values and bounded work

Public dates are `Core.Date.t`, interpreted in the proleptic Gregorian calendar
from 0001-01-01 through 9999-12-31, inclusive. They are never midnight timestamps.
No implicit time zone, system clock or locale participates in selection. The
application supplies an optional today marker and may update it using Eio.
The wire representation will be a validated signed integer day ordinal, with
0001-01-01 = 0 and 9999-12-31 = 3652058. Unsupported values fail before conversion.

`Calendar.Month` is an abstract year/month, with checked offset navigation and
a bounded 6-by-7 day grid. Cells beyond the civil-date domain are blank, not
wrapped into another year. Month/year navigation will materialize only the
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

A retained Rust calendar owner will hold selection, displayed month, day focus,
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
available without advancing revisions. The GPUI/bridge adapter still must check
leases, actual focus gates, publish event batches atomically and fault on lost
required output; standalone policy tests do not establish those adapter behaviors.

Standalone Rust decoders check full consumption and cap configurations at 24 KiB,
constraint blocks at 7100 bytes, selections at 19 bytes, events/responses at 128
bytes and commands at 64 bytes. Collection counts are checked before allocation;
domain conversion validates every date, interval, weekday, label and mode.
Valid raw duplicate constraints canonicalize on either side. Five independently
assembled fixtures cover configuration, constraints, selection, completion and
guarded replacement. Retained message/op/event envelopes remain to be connected.

## Pinned upstream evaluation

Reviewed `vendor/gpui-base/src/calendar.rs` and `date_picker.rs`, from the pinned
GPUI Kit base source. `CalendarState::new` reads `Local::now`; `Date::Range` allows
end-only/reversed states; `CalendarEvent::Selected` omits partial ranges;
`Matcher::is_match` checks complete endpoints only. `year_range` materializes all
years, and month navigation is unchecked. `Calendar::render` invokes
`set_number_of_months`, which calls `cx.notify` unconditionally; its runtime idle
effect has not been measured and is not claimed as a verified upstream defect.

The native adapter will reuse suitable base presentation/behavior with explicit
provenance, but cannot treat this state machine as the public contract unchanged.
`CalendarItem::new` is private, so direct reuse of those parts would require a
small audited vendor change or equivalent first-party GPUI elements. The base
`DatePicker` supplies a controlled focus/open root; trigger, popup, calendar and
placement are application responsibilities. Existing GPUIO overlays already own
those lifetimes. Record the actual chosen adapter and compatibility evidence as
implementation proceeds; no whole styled-library compatibility is inferred.

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
