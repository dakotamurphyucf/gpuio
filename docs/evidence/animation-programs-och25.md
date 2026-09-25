# Advanced animation implementation evidence (OCH-25)

## Spring parameter and trajectory foundation

Local macOS arm64 checkpoint, 2026-09-25. This is partial implementation, not
OCH-25 acceptance. The advanced-program transport, mounted rendering integration,
sequence scheduler, playback controls, shared groups and demonstrations remain.
The existing OCH-12 API and wire encoding are unchanged. No additional capability
is advertised from these foundation tests.

`Animation.Spring.create` validates physical parameters and finite maximum active
duration. The independent OCaml/Rust fixture `animation-spring.hex` establishes
field order and bin_prot encoding; it does not yet prove an end-to-end advanced
program transaction. Public constructors and native `Spring::is_valid` test finite
numeric bounds, NaN/infinity, permitted zero damping and duration boundaries.

`motion::spring::Trajectory` uses the pinned GPUI analytic step and conservative
settling-time solver, evaluated relative to the target. Its immutable samples
include position, velocity and completion. A caller can construct a retargeted
trajectory from a painted sample without any discarded preview changing state.
The eventual retained owner must enforce paint, generation and lifetime rules;
those are not supplied by this numerical helper alone.

Seven Rust tests pass:

- Under-, critically and over-damped springs settle with exact endpoints and zero
  terminal velocity; both displacement and velocity are checked before completion.
- Undamped motion overshoots and finishes at its explicit maximum active duration.
- Retargeting preserves the supplied sampled position/velocity and initially
  continues momentum even when the new target is behind the current motion.
- Frame partitioning gives equivalent motion within native numerical tolerance;
  small changes around large absolute coordinates retain displacement precision.
- Opacity clips to its legal domain, suppresses outward retargeting velocity and
  allows overshoot to return into range.
- Invalid parameters and states are rejected before invoking the solver.
- Admitted parameter extremes and velocity bounds remain finite and terminate.

Commands use `GPUIO_JOBS=2 ./scripts/gpuio` and the isolated project toolchain:

```sh
./scripts/gpuio exec cargo test --locked -p gpuio-native --lib motion::spring -j 2
./scripts/gpuio exec cargo test --locked -p gpuio-protocol --test animation -j 2
./scripts/gpuio exec dune runtest -j 2 test/view_api
./scripts/gpuio exec cargo test --locked -p gpuio-native --test motion --test animations -j 2
./scripts/gpuio exec cargo clippy --locked -p gpuio-native --all-targets --features native-tests -j 2 -- -D warnings
./scripts/gpuio check-fmt
```

All pass: seven spring tests, four protocol animation tests, the OCaml view API
expect suite, nine baseline motion tests and the atomic tree animation lifecycle
test. These checks open no GUI windows. No Linux or hosted execution is claimed.
The [implementation design](../design/animation-programs.md) records remaining
native scheduling, event, group and lifetime requirements.

## Typed programs, bounded decoding and compiled timelines

The next local checkpoint adds public Timing/Stage/Clock/Playback/Program types,
immutable pause/restart/reverse configuration, a matching wire schema and a bounded
standalone native decoder. This is still partial OCH-25 implementation: it does
not yet mount an advanced program, emit its observations or own shared-clock state.
No advanced runtime capability is advertised.

The independent `animation-program.hex` fixture contains a delayed tween followed
by a delayed physical spring, explicit initial values, paused playback, a restart
token and a positive admission generation. Both OCaml and Rust construct it
independently. Rust round-trips it and rejects every truncated prefix, trailing
bytes, malformed numeric/property data, excess stages/properties and overlong
configuration/group names. Shared-clock validation admits only timed positive
repeating cycles and rejects missing initial values or an initial delay.

The OCaml expect tests also prove immutable playback/restart configuration,
32-stage boundaries, cycle duration limits, UTF-8 group validation and reversing
a sequence twice restores its complete configuration. A compatibility test covers
the old API's one-day initial delay plus one-day duration: the new common
representation preserves both instead of incorrectly limiting their sum to one day.
The pre-existing duration API's binary fixture is unchanged.

Six native timeline tests cover mixed tween/spring timing, separate initial/stage
delays, a late sample crossing multiple stages, spring velocity on retarget,
initial values for newly added properties, cleared velocity for timed stages,
constant-interval deadline scheduling, 32 zero-duration stages, immediate first
placement without initial values, invalid input and retained-byte accounting.
Sampling is immutable. These tests prove numerical stage traversal; the retained
owner must still confirm paint before delivering any completed-stage prefix.

Validation commands at this checkpoint, through the isolated jobs=2 wrapper:

```sh
./scripts/gpuio exec cargo test --locked -p gpuio-protocol --test animation --test animation_program -j 2
./scripts/gpuio exec cargo test --locked -p gpuio-native --lib motion -j 2
./scripts/gpuio exec cargo test --locked -p gpuio-native --test motion --test animations -j 2
./scripts/gpuio exec dune runtest -j 2 test/view_api
./scripts/gpuio exec cargo clippy --locked -p gpuio-native --all-targets --features native-tests -j 2 -- -D warnings
./scripts/gpuio check-fmt
```

These pass locally on macOS: ten protocol tests, fourteen motion-related native
unit tests, the nine baseline motion tests, atomic tree lifecycle test and OCaml
view API suite. The expect-test correction was whitespace layout only; the values
were reviewed. No GUI or Linux result is claimed from this checkpoint.

The full isolated `dune build -j 2 @all @runtest` also passes with the new exported
types and timeline code. All validation processes exited; none opened GUI windows.
