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
