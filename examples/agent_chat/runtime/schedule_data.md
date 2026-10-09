# Fixed civil dates and sample reviews

[schedule_data.ml](schedule_data.ml) and [schedule_data.mli](schedule_data.mli)
provide the Dates & reviews page's reproducible calendar constraints, labels and
review filtering. These are civil `Core.Date.t` values, not timestamps. The model
uses no ambient clock, timezone conversion, scheduled task, Bonsai state or I/O.
[Schedule_settings](schedule_settings.md) supplies the native calendar/picker.

From the repository root after [isolated setup](../../../docs/development.md):

```sh
GPUIO_JOBS=2 ./scripts/gpuio exec dune build examples/agent_chat/main.exe -j 2
_build/default/examples/agent_chat/main.exe
```

Open Settings → Dates & reviews. Choose a weekday or range and Apply to filter
sample reviews; choose a separate follow-up date without scheduling anything.
macOS is the v1 target; Linux GUI qualification is
[informational](../../../docs/platform-release-policy.md).

## Constraints and meaningful selection cases

Read `date`, `month`, `constraints`, `config`, `Review`, `describe` and `reviews`.
`date day` constructs October 2026 with `Date.create_exn`; checked-in day values
are invariants. `month` converts October 1 to a `Calendar.Month.t`.
`Calendar.Constraints.create` admits October 1–31 inclusive, disables Saturday/
Sunday and October 20, and uses `Endpoints_only`. A selected range's endpoints
must be allowed; disabled dates inside that interval do not invalidate its
endpoints. Review filtering nevertheless excludes every disabled interior date.
Read the [calendar interface](../../../lib/core/calendar.mli) for the selection/
constraint contracts and the different All_dates range policy.

`config ~mode ~label` constructs a validated native calendar configuration with
these constraints. Its mode is explicit Single or Range; configuration alone
does not open a calendar or produce a selection. `ok = Or_error.ok_exn` is used
for literal constraints, not for accepting arbitrary user input without checks.

`Calendar.Selection.t` distinguishes Empty, Single date, Range_start date and a
complete Range. `describe` formats each case: Empty says No date selected, a
partial interval asks for an end, and a complete range formats first/last dates.
It formats application labels separately from native editing syntax; it does
not serialize timezone-dependent timestamps.

`Review.t` is abstract and exposes date/label accessors. Its labels rotate among
Source notes, Workspace changes and Generated artifacts by day-of-month modulo
three. No file/tool review is actually performed.

## Validate before filtering

`reviews selection` chooses Single mode for a Single value and Range for the
other cases. It rejects a Range_start directly, then rejects any selection not
allowed under the fixed constraints/mode. Empty is valid and means all available
reviews rather than no rows. Only after validation does it generate 31 dates,
keep allowed dates and apply Single/range containment. The result is
`Review.t list Or_error.t`.

For an interval October 16–21, valid endpoints encompass a weekend and disabled
October 20. The filter keeps October 16, 19 and 21; it does not invent weekend
reviews merely because the interval spans them. Applying an unavailable endpoint
or partial interval returns an error instead of silently widening the filter.
The full fixture has 21 available review dates, displayed six per page by the
caller. This computation is small and synchronous, and owns no native resource.

The end-to-end path is native calendar selection → Schedule_settings copies the
observed draft → Apply calls `reviews` → accepted selection/page count updates →
Bonsai derives the visible description rows. The separate date picker uses the
same constraints but confirms only on Save follow-up; this module never stores
that value or initiates a reminder.

A small adaptation is another fixed unavailable date. Add it to `disabled_dates`
and update fixture explanations/counts, keeping the date within the month bounds.
For another month, change `date`, month bounds, generation length and headings
together; simply changing the displayed month would leave the filter inconsistent.
Use a timezone policy explicitly if adapting civil dates to real appointments;
none is implied here. The [settings guide](schedule_settings.md) and
[README evidence](../README.md) separate pure filtering from actual native
calendar/picker acceptance; this review performs no GUI checks.
