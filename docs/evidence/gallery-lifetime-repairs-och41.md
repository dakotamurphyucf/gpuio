# Gallery lifetime repairs — OCH-41 / OCH-17

2026-10-05, macOS 14.5 arm64, M1 Max, built-in display. This follows the
[installed action-control checks](installed-actions-och41.md) and preserves their
failures rather than relaxing their assertions. VoiceOver remains untouched and
on owner hold.

## Tooltip anchor identity

The split-button failure originates in `tooltip_element`, not the split mode
reconciler. Its inactive focus-scope branch omitted the `gpuio-tooltip-anchor`
wrapper present in the active branch. Disabling an ancestor therefore changed
the child's `GlobalElementId` path and semantic node ID. A subsequent mode change
was incorrectly blamed by the old test sequence, which compared against the
reference captured before disabling.

Commit `20923d588f4521509f53b81e95e2dbc0d8372b06` keeps that wrapper in both
branches. The inactive branch still omits hover listeners and the popup surface;
it does not change disabled policy or reactivate tooltip timers. No public API,
protocol or vendored framework change is needed.

The production-View TestPlatform regression fails before the fix with semantic
IDs `11984198560043421472` and `16447489482888880036`. Afterward, it checks the
primary semantic ID/disabled state and retained focus owner through ancestor
disabled, inert and visibility changes/recovery. Existing menu-mode, input,
paint and teardown assertions remain. The full native library passes **930 tests,
two existing skips**; strict native library/test Clippy passes.

A fresh independently staged public gallery at `20923d5` builds and passes its
catalog checks plus the original real macOS `split-buttons` walkthrough. The
subsequent combined `buttons` run also passes rich buttons, appearance/Link,
menu observation and split modes. It then exposes the distinct observation
remount failure below, so that combined run is retained as a failure.

## Managed lifetime and release-profile test execution

After repeated page departures, the command hint remains at `Looking up
shortcut…`, although its isolated walkthrough passed. Current development-profile
tests also fail managed-row branch reactivation and tree/selectable-list
controller recovery. The `562c44e` optimization allocated mutable lifetime tokens
inside a mapped key computation; that allocation is unsafe to treat as pure
under Bonsai graph optimization and constant configurations.

The earlier release-profile `dune runtest` evidence did not establish inline-test
correctness. Dune disables inline tests by default in the release profile; before
the correction, `dune rules --profile release @test/virtual_list/runtest` emits no
rules. This is not disabled OCaml assertions. [Dune's documented profile behavior](https://dune.readthedocs.io/en/latest/reference/dune/env.html).

`test/dune` now explicitly enables first-party inline tests in every profile.
It is scoped to the test tree, leaving production release-library configuration
alone. With that correction, the release-profile tests reproduce the same
reactivation failures as development mode. Earlier claims that these release
commands passed the complete inline suite are withdrawn; they were incomplete
verification. Historical performance measurements remain measurements of their
recorded binaries, but cannot qualify a later lifecycle-correct implementation.

Commit `e210d3e4a7b261e6bdf40023023937fbeb096e85` restores lifecycle-scoped
`B.Expert.thunk` allocation inside the resetter. A dedicated constant-configuration
observer regression checks three visits, fresh sample delivery and rejection of
old effects with Bonsai optimization both off and on. It fails before the repair
and passes afterward; the dynamic-configuration test also now checks a fresh
sample after reactivation.

Full OCaml validation now passes in both profiles, with actual runner execution:

- Release: 28 inline aliases, 271 partition-completion log entries.
- Development: 40 aliases, 295 partition-completion entries, including the
  additional development-only example/vendor suites.

These are runner/partition counts, not a count of individual expectations. Both
commands use `--force --display short`. Repository formatting passes after the
formatter's required blank line in `test/dune` was applied. The fresh installed
gallery at `e210d3e` now passes the complete `buttons`
sequence, including both split identity and command hints after repeated page
changes. Catalog checks and the consumer driver exit successfully. All owned
windows/processes are reaped. No identity or lifecycle assertion was relaxed,
and no expectation was promoted to accept the failing behavior. This investigation does not change performance thresholds
or close milestone release requirements.

## Reproduction and retained evidence

```sh
GPUIO_JOBS=2 ./scripts/gpuio exec cargo test --locked -p gpuio-native --features native-image-tests,native-canvas-tests --lib
GPUIO_JOBS=2 ./scripts/gpuio exec cargo clippy --locked -p gpuio-native --lib --tests --features native-image-tests,native-canvas-tests -- -D warnings
GPUIO_JOBS=2 ./scripts/gpuio exec dune runtest -j2 --profile release --force --display short
GPUIO_JOBS=2 ./scripts/gpuio exec dune runtest -j2 --force --display short
GPUIO_JOBS=2 ./scripts/gpuio exec dune build -j2 @fmt
GPUIO_JOBS=2 python3 scripts/test_extension_consumer.py --example gallery --run --gallery-section buttons --workspace <fresh-workspace>
```

[Raw before/after results](gallery-lifetime-repairs-och41/local-validation.tar.gz)
include the original combined-gallery failure, native identity regression,
release-test execution correction, full suite logs and final independent-consumer
pass. [The manifest](gallery-lifetime-repairs-och41/manifest.json) pins commands,
source revisions, executable hashes and archive entries. This fixes the two
identified defects; whole-family pixel/resource coverage, current optimized
performance, final-source hosted checks and distribution remain separate.
