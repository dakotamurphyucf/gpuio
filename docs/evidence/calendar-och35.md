# OCH-35 calendar/date-picker evidence

Status: **in progress**. This records incremental evidence, not completed widget acceptance.
The milestone retains the full calendar and popup-picker scope. A retained
`View.calendar`, correlated Eio calendar controller and controlled-value popup
picker are implemented. No calendar capability is advertised yet. The mounted
calendar and picker have incremental macOS input, visual and lifecycle evidence
below; remaining full acceptance is listed explicitly.

## Civil model checkpoint

Source: `lib/core/calendar.{ml,mli}`, `rust/protocol/src/calendar.rs`,
`test/view_api/calendar_test.ml`, `rust/protocol/tests/calendar.rs`.
The [design](../design/calendar.md) records the pinned base-source evaluation and
the accepted ownership/selection/constraint direction.

On macOS arm64 with the repository's isolated pinned toolchain:

- OCaml view-API expect suite passes, including seven new calendar expect tests.
- Rust's five new calendar model tests pass.
- Both implementations validate years 1..9999, checked civil/day/month bounds,
  ordered ranges and empty/partial/complete selection modes. Out-of-domain values
  and extreme navigation offsets reject before unsafe conversion/arithmetic.
- Each implementation checks 33,600 six-week month grids across a complete
  Gregorian 400-year cycle and all seven first-weekday choices. Neighboring cells
  stay consecutive, every month is complete, and civil boundary cells never wrap.
- Ten shared ordinal/weekday reference vectors were independently checked with
  Python's `datetime.date`, including leap-century and 0001/9999 endpoints.
  OCaml uses Core.Date conversions; Rust computes Gregorian ordinals independently
  without a new chrono dependency. These are value vectors, not yet paired full
  bridge-codec fixtures.
- Each implementation compares 5,112 bounded range-policy queries against a
  daily reference, plus full civil-span weekday exclusions. Canonical constraints
  enforce raw input quotas before deduplication and merge disabled intervals.
- Partial range changes, earlier-date restart, same-day completion, disabled
  interior versus endpoint policy, wrong-mode rejection and preserved historical
  selections have model coverage.
- OCaml strict ASCII ISO/day-first/month-first parsing checks invalid dates,
  leap centuries, ambiguous ordering and whitespace/Unicode rejection; 438,291
  format/parse round trips cover all dates in a Gregorian cycle.

Commands:

```sh
GPUIO_JOBS=2 ./scripts/gpuio exec dune build -j 2 @test/view_api/runtest @fmt
GPUIO_JOBS=2 ./scripts/gpuio exec cargo test --locked -p gpuio-protocol --test calendar -j 2
GPUIO_JOBS=2 ./scripts/gpuio exec cargo clippy --locked -p gpuio-protocol --all-targets -j 2 -- -D warnings
```

No GUI windows are needed for these model checks. Compile corrections were Core
local-open identifier collisions, optional-argument eta expansion, and a typed
weekday constructor annotation. One manually reviewed expected-output correction
uses Core's uppercase weekday sexps. No failing behavior was promoted away.

## Still required for this ticket

- Full inline calendar acceptance: actual accessibility, modal/pointer policies,
  civil boundaries, locale/format, scale/layout, independent windows, managed-list
  pins, idle and bounded workload coverage beyond the initial checks below.
- Broader popup placement/nested-overlay and visual coverage, independent-window
  checks, managed-list pins and idle/retention/disposal workloads beyond the public
  popup lifecycle/input evidence below.
- Required macOS/Linux consolidated CI, merge and Linear completion; final
  integration into OCH-46. Full Linux GUI release acceptance remains OCH-17.

## Configuration, codec and native policy checkpoint

Added `lib/protocol/calendar_wire.ml`, typed Core configuration/labels/commands/
observations, `rust/protocol/src/calendar_input.rs`, the bounded
`decode/calendar.rs` adapter, and `rust/native/src/calendar_state.rs`.
The native policy owner has no GPUI rendering or retained placement yet.

Local macOS checks pass:

- Four additional Core expect tests cover typed configuration, locale changes,
  semantic decoding, leases, commands and observations. Together with the initial
  seven tests, these run within the full view-API expect suite; `@fmt` passes.
- Five calendar codec Rust tests cover five independently assembled bin_prot
  fixtures (Python datetime plus explicit byte assembly, consumed independently by
  OCaml and Rust), full consumption, every truncated fixture prefix, invalid date/
  range/weekday/tag/Boolean values, malformed UTF-8, control/oversized labels,
  collection counts rejected before allocation, canonicalization and a maximum
  valid configuration. The five prior Rust civil-model tests remain passing.
- Five native policy unit tests cover explicit initial month/cursor, programmatic
  observations, guarded commands, partial/restarted/completed ranges, consecutive
  Changed/Selected revisions, rejection preservation, historical invalidation on
  configuration changes, read-only/disabled/modal gates, leap-day navigation,
  failed native-focus callback rollback and revision exhaustion before mutation
  or invoking focus. These are policy tests, not actual OS focus validation.

Commands:

```sh
GPUIO_JOBS=2 ./scripts/gpuio exec dune build -j 2 @test/view_api/runtest @fmt
GPUIO_JOBS=2 ./scripts/gpuio exec cargo test --locked -p gpuio-protocol --test calendar --test calendar_codec -j 2
GPUIO_JOBS=2 ./scripts/gpuio exec cargo test --locked -p gpuio-native --lib calendar_state -j 2
GPUIO_JOBS=2 ./scripts/gpuio exec cargo clippy --locked -p gpuio-native --all-targets --features native-image-tests -j 2 -- -D warnings
```

The configuration and all response/input structures are bounded and validated,
but this checkpoint adds no top-level retained protocol tags and advertises no
calendar capability. Native-image all-target Clippy also passes with warnings
denied. Native rendering, transport/window ownership, public mounted
controllers and picker acceptance remain required.


## Retained admission and event-routing checkpoint

Source: `View.calendar`, `Reconciler`, `rust/native/src/tree.rs`, `session.rs`,
`mailbox.rs`, `rust/native/tests/calendar.rs`,
`test/view_api/calendar_retained_test.ml` and the retained-envelope test in
`rust/protocol/tests/calendar_codec.rs`.

- Four native integration tests cover atomic tree rejection/rollback, required
  configuration/handler and leaf shape, immutable mode, seed retention, historical
  constraint invalidation, byte accounting and disposal. Routing tests reject old
  window/node/handler generations, future/negative tree revisions, invalid snapshot
  shapes/modes, overload and closed windows; disabled cleanup observations remain
  deliverable and generic button presses cannot target calendars.
- Mailbox tests exercise completion count limits, a byte budget leaving only 511
  bytes for a 512-byte charged pair, malformed pairs, consecutive revisions,
  unchanged queued predecessors, closed mailboxes and window-output draining.
  Partial selections and completion boundaries remain discrete and ordered.
- Two Core expect tests cover independent retained envelope bytes, every truncated
  event prefix, trailing bytes and malformed observations; stable identity and
  seed-only recomputation; updated callbacks and events arriving between prepare/
  accept; immutable mode, duplicate controllers, historical invalidation, invalid
  initial selections, stale revisions/leases, remount and close.
- The sixth Rust calendar codec test checks independently assembled kind 40,
  operation 46 and event 50 fixtures, every truncated request prefix, trailing
  data and invalid nested dates/months/configuration. Boxing the Rust configuration
  preserves those exact bytes while avoiding growth of every operation variant.

Commands (isolated macOS arm64 toolchain):

```sh
GPUIO_JOBS=2 ./scripts/gpuio exec dune build -j 2 @test/view_api/runtest @fmt lib/eio/gpuio_eio.cmxa
GPUIO_JOBS=2 ./scripts/gpuio exec cargo test --locked -j 2 -p gpuio-native --test calendar --test otp_input -p gpuio-protocol --test calendar_codec
GPUIO_JOBS=2 ./scripts/gpuio exec cargo clippy --locked -j 2 -p gpuio-native --all-targets --features native-image-tests -- -D warnings
```

The Cargo selection also runs the protocol calendar-model and OTP-codec suites;
native OTP mailbox regressions remain included because completion infrastructure
is shared. Test-development corrections were use of the public checked ordinal
converter and draining the mailbox across its bounded output batches. No failed
expectation was promoted. No calendar GUI windows have been run. Native renderer,
actual publication/focus/cleanup, controller/commands, picker integration and full
OCH-35 acceptance remain outstanding; no calendar capability is advertised.


## Initial mounted native calendar checkpoint

`rust/native/src/calendar_view.rs` now owns a GPUI entity per retained placement,
using first-party GPUI elements over the tested civil policy. Host rendering,
focus styling/order, disabled state, managed-list focus pins, hidden cleanup and
atomic transport publication are connected. This is initial integration, not
complete OCH-35 acceptance or a capability advertisement.

Local macOS arm64 checks pass:

- The actual GPUI window test exercises autofocus, day navigation across leap-day/
  month boundaries, single selection with ordered Changed/Selected, unchanged
  activation, pointer activation of month/year headers, keyboard month/year choice,
  M/Y/D mode switching and Backspace clearing.
- Configuration changes preserve selection/navigation while updating locale and
  reporting historical invalidity. Read-only allows navigation but rejects edits;
  disabled calendars lose focus and ignore keyboard input.
- Range mode reports partial and completed selections distinctly. Hide/show
  preserves selection and prevents hidden keyboard input. Removing the first owner
  releases its weak entity; closing the session clears retained accounting/owners.
- A single transaction that seeds a valid range start and then disables that date
  before GPUI owner construction preserves the historical value. Restoring the
  constraints and clearing/selecting still work through the native widget.
- A mounted range completion with only one free input slot cannot publish either
  member of its two-event pair. It emits one overload notification and prevents
  subsequent navigation. This exercises `Route` and `Transport`, beyond the prior
  mailbox-only tests.
- Seven native policy unit tests pass (including two new atomic presentation and
  historical-construction tests). The four retained calendar and six OTP bridge/
  mailbox regression tests pass. Feature-enabled all-target Clippy passes.
- Four GPU readback images were generated and visually inspected: light day/month/
  year views and a dark day view. Text, neighboring-month dimming, cursor/today
  borders and mode layout are visible. These are initial visual checks, not yet
  the scale/constrained/localized-content acceptance matrix. Black margins outside
  the widget belong to the otherwise empty native test window.

Commands:

```sh
GPUIO_JOBS=2 ./scripts/gpuio exec cargo test --locked -j 2 -p gpuio-native --lib calendar_state
GPUIO_JOBS=2 ./scripts/gpuio exec cargo test --locked -j 2 -p gpuio-native --test calendar --test otp_input
GPUIO_JOBS=2 ./scripts/gpuio exec cargo test --locked -j 2 -p gpuio-native --features native-image-tests --test native_calendar
GPUIO_JOBS=2 ./scripts/gpuio exec cargo clippy --locked -j 2 -p gpuio-native --all-targets --features native-image-tests -- -D warnings
```

Set `GPUIO_CALENDAR_SCREENSHOT_DIR` to an output directory when running the native
image-enabled test to reproduce the four screenshots. Local artifacts are under
`scratch/agents/root-20260924-m5/calendar-render/`; they are not build inputs.
The test uses actual GPUI windows and GPUI platform-input dispatch, not external
physical OS keyboard automation or an AppKit accessibility audit. Owned windows
and processes closed normally. CI definitions now compile/run the calendar test;
no hosted results or Linux GUI acceptance are claimed. Public commands/controllers,
popup semantics/examples and the remaining native acceptance matrix are next.

## Public command/controller checkpoint

Local macOS arm64 checks on 2026-09-25 pass for the correlated command bridge and
`Gpuio_eio.Calendar` controller. Message 17/event 51 have independent byte fixtures,
positive-correlation checks, bounded command decoding, full-consumption/truncation
checks and malformed snapshot rejection in the OCaml and Rust suites. Native
mailbox tests cover reserved replies, render-event barriers and window-slot
retirement while a reply remains queued.

The public `examples/calendar/main.exe --self-test` opens/closes a real GPUI window
and exercises single/range mode, snapshot reads without revision changes, 64
concurrent accepted requests plus one Busy response, actual focus, revision guards,
wrong mode, disabled endpoints/interiors, invalid large offsets, navigation and
presentation, disabled-date discovery, hidden/disabled/read-only policy, remount
seeds, old-lease rejection and ordered window close. Programmatic selection changes
do not emit Selected. Both controllers are observed before the test begins.

The initial test found a hidden-calendar snapshot could retain `focused=true` even
though focus requests were correctly blocked. Native hide and focus subscriptions
now publish actual platform focus, including when a hidden dispatch node disappears
before blur delivery. The public test passes after that fix. The native window
suite also passes guarded disabled clearing, no Selected from commands,
visibility cleanup, and command rejection without mutation after a window fault.
Its existing keyboard/pointer/range/overload/disposal checks continue to pass.

Commands (all passed locally):

```sh
GPUIO_JOBS=2 ./scripts/gpuio exec dune runtest -j 2 test/view_api
GPUIO_JOBS=2 ./scripts/gpuio exec cargo test --locked -j 2 -p gpuio-protocol --test calendar_codec
GPUIO_JOBS=2 ./scripts/gpuio exec cargo test --locked -j 2 -p gpuio-native --test calendar
GPUIO_JOBS=2 ./scripts/gpuio exec dune build -j 2 examples/calendar/main.exe
_build/default/examples/calendar/main.exe --self-test
GPUIO_JOBS=2 ./scripts/gpuio exec cargo test --locked -j 2 -p gpuio-native --features native-image-tests --test native_calendar
```

All-target native-image Clippy (`-D warnings`) and `dune build -j 2 @fmt`
also pass locally. Rust codec suite: 7 tests; native bridge suite: 5 tests. Public output:
`GPUIO_CALENDAR_PUBLIC_OK`. All test windows closed and processes exited.
The CI definition includes the public self-test; hosted checks have not run.
Popup-picker integration and the remaining full native acceptance matrix are
still required. No new calendar capability or completed ticket is claimed.

## Controlled-value popup checkpoint

Implemented `Gpuio.Date_picker` pure lifecycle/confirmation policy and
`Gpuio_eio.Date_picker` public Bonsai/Eio controller. The application retains its
confirmed value; one native calendar owns each open draft. Explicit Apply reads
native state before rechecking current policy, committed value, session and native
lease/revision. Partial ranges cannot apply. Cancel/dismiss never call on_change.
Five new expectation tests cover confirmation, partial/empty ranges, read-only and
constraint changes, stale sessions/revisions/native leases, external value/mode/
disabled invalidation and historical disallowed values. A trusted reconciler event
may introduce a newly mounted native lease; old replies cannot replace its state
or error display.

`examples/calendar/picker.exe --self-test` passes locally on macOS arm64. It checks
partial/complete Apply, Cancel, an in-flight confirmation cancelled before reply,
old session actions, view-only remount/reseeding, keyed Bonsai deactivation and
reactivation closed without an old draft, external resets, read-only/disabled and
constraint updates, historical disallowed values and window close. Closing the
window preserves Native Closed even after the Bonsai input becomes inactive.
No synchronous native callback into OCaml was introduced.

`python3 scripts/test_date_picker.py` also passes against the final implementation:
real AppKit AX buttons/selection, Apply disabled for a partial range, Apply/Cancel,
actual OS Escape and day-navigation/Enter, outside pointer dismissal, clear and
read-only/disabled policy. Escape/Cancel/Apply restore the eligible trigger focus.
An outside pointer click preserves the newly focused outside control. The pointer
target is checked through the system AX hit test before sending a real OS click;
targeted CG mouse events alone did not establish this behavior. Keyboard reopening
starts the cursor at the range's first endpoint, consistent with the native policy.
All windows/processes closed; no hosted run is claimed.

Commands (passed locally):

```sh
GPUIO_JOBS=2 ./scripts/gpuio exec dune build -j 2 examples/calendar/picker.exe @test/view_api/runtest @fmt
_build/default/examples/calendar/picker.exe --self-test
python3 scripts/test_date_picker.py
```

Success markers: `GPUIO_DATE_PICKER_PUBLIC_OK`, `GPUIO_DATE_PICKER_AX_OK`.
CI definitions include both checks. Broader calendar/picker visual, boundary,
modal, multiple-window/list and workload acceptance remains open above. OCH-35
and milestone 5 remain in progress, with the full milestone-05 scope unchanged.
