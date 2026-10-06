# Draft dates, accepted filters and a follow-up picker

[schedule_settings.ml](schedule_settings.ml) and
[schedule_settings.mli](schedule_settings.mli) implement Settings → Dates &
reviews. A native inline calendar edits a review-filter draft; Apply accepts it.
A separate popup date picker confirms a follow-up value. Both use fixed
[October 2026 fixtures](schedule_data.md); nothing is scheduled, persisted or
sent, and no timezone conversion or network call occurs.

Use the [isolated environment](../../../docs/development.md) from the repository root:

```sh
GPUIO_JOBS=2 ./scripts/gpuio exec dune build examples/agent_chat/main.exe -j 2
_build/default/examples/agent_chat/main.exe
```

Select Single review day or Review date range, Apply, change review pages, then
try Choose follow-up date/Save/Cancel. Weekends and October 20 are unavailable
endpoints. macOS is the v1 target; Linux graphical qualification is
[informational](../../../docs/platform-release-policy.md).

## Separate four values and two calendar identities

Read `Model`, `create`, `component`, its local `mutate`/`apply`, calendar event
handler and pagination. The persistent window-owned `t` contains an observable
model and a separate observable `follow_up` selection. `Model.t` stores mode,
inline-calendar draft, applied filter, displayed month, calendar epoch,
`Pagination.t` and notice. Initial mode is Range, both selections are Empty,
month is October 2026, epoch is zero and pagination covers six reviews per page.
The 21 allowed fixture dates initially make four pages. Follow-up starts Empty.

The inline calendar's observed draft is copied into this application model; it
is not yet the applied review filter. The native calendar still owns editing,
keyboard focus and its snapshot/revision. The popup follow-up picker has another
Rust-owned draft; only successful Save writes `follow_up`. The accepted filter,
inline observed draft and follow-up must not be conflated.

`Date_picker.create window` receives Single-mode constraints, the controlled
`follow_up` value, initial month and an `on_change` effect. The
[picker interface](../../../lib/eio/date_picker.mli) says each opening has a fresh
native identity; selecting a day alone does not commit. Confirm reads/revalidates
the native snapshot before calling on_change. Cancel/Escape/dismiss discard that
opening's draft. `Picker.view` provides Choose follow-up date, Save follow-up,
Cancel follow-up and a named 340-pixel overlay; Picker.error produces a diagnostic.
Clear follow-up combines Picker.cancel with a guarded application Empty update.

## Bonsai graph lifetime and stale-event guards

`component ... graph` builds picker state and derives presentation with
`let%arr model = ... and picker = ... and current = ... in ...`. Reactive values
update their dependent views; `let%arr` combines their current values.
`Bonsai.Effect.of_thunk` defers mutation to the UI loop, rather than mutating
state during view derivation. The `mutate` helper reads the latest model when the
effect executes, and writes only if the supplied `is_current ()` guard passes.

In [settings.ml](settings.ml), `match%sub active Schedule` activates this
subcomputation only on its settings page. `match%sub` switches computation
branches according to a reactive value; it is distinct from ordinary OCaml
matching of a fixed value. Deactivation retires picker draft owners. The
persistent `Schedule_settings.t` lives outside that branch, so accepted values
survive reopening; the inline draft/month copied into its model also survive.
`is_current` is a closure fenced by the outer settings epoch/open state, protecting
a delayed old-page effect after navigation or closure.

The inline `Gpuio.View.calendar` uses controller key review-calendar-<epoch>,
mode/constraints, initial draft/month and themed styles. Its event callback
checks **both** the outer guard and whether latest model epoch matches the key's
epoch. Changed/Selected snapshots update draft/month; Observed is ignored;
Rejected updates notice. Switching Single/Range clears draft and advances epoch,
creating a fresh native calendar identity, but leaves the previously applied
filter unchanged. All review dates accepts Empty, clears draft and advances
epoch while preserving the current mode/month.

## Trace Apply and pagination

For a range, the first native selection produces Range_start and the Date draft
label asks for an end date. `Schedule_data.reviews draft` reports an error, so
Apply review dates is disabled. Completing valid endpoints enables Apply.
The button's effect reads the latest model, validates its draft again in
`apply`, writes `applied` and updates page count. Failure changes only notice;
accepted reviews therefore never depend on a partially selected range.

`reviews = Data.reviews model.applied` determines the list; current page is
one-based, and `List.drop ... |> List.take 6` selects the displayed slice.
`Pagination.with_total_pages` clamps an existing page on shrink or clears an
empty page selection; it does not always reset to page 1. Description rows use
stable date-string keys. `Navigation.pagination` supplies localized navigation,
page/gap labels and current-page style; its on_request effect applies
`Pagination.apply_request` against the latest model, so relative Next/Previous
are not based on a stale rendered page. The
[pagination contract](../../../lib/core/pagination.mli) is pure bounded state,
not an async data loader.

For follow-up, opening creates a separate draft, selection updates that native
draft, and Save reads/revalidates it. Its guarded on_change publishes the accepted
selection and the Saved follow-up label updates through Bonsai. Cancel leaves
the accepted value unchanged; changing settings pages cancels the open draft.
No reminder service is called and no Eio producer task is introduced here.
Asynchronous native observations/command replies belong to the public picker;
an opening effect or submitted view is not proof of native draft readiness or a
physically presented frame.

A small adaptation is eight reviews per page: change `total_pages`, drop offset
and take size consistently. For another date fixture, change Schedule_data bounds/
month/labels together. Preserve outer epoch guards, distinct inline calendar
identity and draft-vs-applied values. To implement real follow-up scheduling,
add an explicit service/effect and confirmation result; a stored civil date does
not imply an appointment was created.

`python3 scripts/test_agent_chat_dates_colors.py` is the optional macOS external
runner; [README prerequisites](../README.md) and
[existing evidence](../../../docs/evidence/agent-chat-m5.md) qualify its checks.
This source/prose review runs no picker, keyboard or calendar acceptance test.
