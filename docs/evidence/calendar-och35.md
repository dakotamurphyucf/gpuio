# OCH-35 calendar/date-picker evidence

Status: **in progress**. This records model evidence, not native widget acceptance.
The milestone retains the full calendar and popup-picker scope. No capability
bit, `View.calendar`, mounted native owner or public Eio controller is claimed.

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

- Bounded independently checked OCaml/Rust codecs, validating malformed dates,
  list lengths, modes and intervals before allocation/admission.
- Configuration/locale labels, native ownership and revisioned commands/events,
  retained-tree admission and public Core/Bonsai/Eio controllers.
- Native inline calendar with day/month/year navigation, focus and selection;
  strict locale/format policy, disabled/read-only/hidden/modal behavior.
- Popup date-picker integration with the existing overlay system and explicit
  confirm/cancel/partial-selection, dismissal and focus-restoration semantics.
- Public inline/popup examples, actual macOS input/accessibility/visual checks,
  stale-lifetime and independent-window tests, idle/retention/disposal workloads.
- Required macOS/Linux consolidated CI, merge and Linear completion; final
  integration into OCH-46. Full Linux GUI release acceptance remains OCH-17.
