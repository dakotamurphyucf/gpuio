# Explicit animation iterations and directions — OCH-41

Local macOS arm64 validation, 2026-10-05, based on
`4c959f574890257ac9afb6c73e49e2f185814649` plus the archived source changes.
The [contract](../design/animation-iterations.md) adds full unsigned 64-bit finite
counts, explicit infinite policies and all four playback directions. Legacy
policies preserve their semantics. This implements the remaining named timing
gap in the [motion source review](../catalog/motion-review.md); broader catalog
and release qualification remain open.

## API, protocol and native state

The public abstract count accepts integer/decimal construction without floating
point; decimal conversion and fixtures cover both sides of signed 64-bit range
and the maximum unsigned value. Independent fixtures pin 32 finite/infinite
count/direction encodings. Native decoders validate each unsigned limb, direction,
cycle duration, required initial values and shared-clock policy. The paired API
and program byte limits include the new payload. `Program.with_repeat` preserves
playback/restart identity while revalidating the replacement.

Deterministic native tests cover all four finite directions with asymmetric
easing, zero/one/multiple counts, exact cycle boundaries and one paint-confirmed
terminal event. Other tests cover reverse stage pauses without frame polling,
spring trajectory/velocity reversal, huge counts without proportional storage,
hidden/paused time, negative initial offsets, restart, reduced motion and stale
paint rejection. Shared retargets use the declared range at group phase. Reverse
linear-stop curves sample their active progress-1 value before the finite run's
eventual exact-endpoint settlement.

Review found that an enormous constant finite run must not pass `Duration::MAX`
to an OS timer. Native waits are bounded to one day; completion comparisons retain
the exact u128 duration product. The regression checks representable waits,
bounded retention and no early completion even at a saturated logical clock.

Full local OCaml `@runtest`, formatting and gallery build pass. A later focused
view API/format run includes the additional policy-edit identity test. The final
full Rust command passes **1,350 tests**, with two existing skips. Strict Clippy
passes after replacing the even-parity expression with `is_multiple_of(2)`;
the 25-test native program suite also passes after that lint-only change.

Original failures remain archived: an OCaml digit conversion accidentally used
integer subtraction inside an Int64 local open; a new native test expected exact
floating-point equality for 75 versus 74.99999999999999. Their corrections preserve
the intended contracts. Review also corrected the test's generation increments.
No expectation output or acceptance threshold was changed to hide a behavior defect.

## Fresh installed macOS consumer

The public Motion page adds a counted-animation card with four finite directions,
zero cycles, continuous reverse, and pause/resume/cancel controls. The independently
installed gallery passes the complete Motion walkthrough (session 62205, exit 0),
including the new directions/endpoints and continuous reverse controls/reduced
motion. Existing curves, signed delays, spring sequence, interruption, shared
motion, second-window behavior, retirement/remount and shutdown also pass.
The owned application process was confirmed absent afterward.

Binary SHA-256:
`5317afc820088924c331edd4cd069f0d80ee670557c7625f42bfda45a44bac6b`.
Its source snapshot precedes the final lint-only parity-expression spelling
change; the post-lint tests cover that final spelling. No screenshots, VoiceOver,
physical presentation latency, Linux GUI or complete-release acceptance is claimed.

[Commands, source snapshots, independent fixture, original failures and final logs](animation-iterations-och41/validation.tar.gz)
are retained with a [verified manifest](animation-iterations-och41/manifest.json).
Hosted validation of this addition remains required.
