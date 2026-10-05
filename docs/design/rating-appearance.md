# Rating appearance

OCH-41 implementation contract. Implemented locally; native acceptance is pending.
The [source review](../catalog/rating-review.md) identifies the independent
active/outline color gap this change addresses.

`Rating.Appearance.create ?active ?inactive ()` accepts validated `Color.t`
values. `View.rating ?appearance` and its Bonsai specialization keep appearance
separate from the controlled value/configuration and request reducer. Omitting
appearance preserves the existing API and rendering defaults.

An omitted active color uses the computed foreground at paint time. An omitted
inactive color uses that foreground with the existing 0.7 opacity. An explicit
color replaces its respective default, including alpha; no extra 0.7 multiplier
is applied to an explicit inactive color. Active colors also apply to filled
hover-preview stars. Explicit colors take precedence over root foreground state
refinements; omitted colors follow those refinements. Ancestor opacity/clipping
and the existing disabled/read-only policy continue to apply.

Theme references and color opacity expressions resolve during ordinary Core
reconciliation. Missing tokens fail preparation atomically. The mounted state
remembers resolved appearance, so theme changes update paint without replacing
the rating node, focus owner, handler or native-only hover state. Resetting an
appearance uses the same operation with no payload; explicit default appearance
normalizes to absence. Appearance introduces no callback, task or native timer.

The paired bridge appends operation 64, `Set_rating_appearance`, with an optional
record of optional active/inactive RGBA int64 values. Values must be in
0..0xffffffff. It applies only to Rating nodes, is charged to retained storage
and participates in atomic transaction validation. Existing config and event
tags remain unchanged. Capability bit 57 distinguishes the new representation;
the aggregate required mask becomes 9223372036854775807 (2^63−1).

Independent OCaml/Rust bytes, invalid-color/kind/reset rollback, theme-error
atomicity, stable identity/current callbacks and no-op reconciliation require
tests. Native GPU colors, state refinements, hover/AX/keyboard behavior, ownership
and fresh gallery-consumer behavior require actual platform validation. No such
acceptance is implied by this contract or compilation.

## Local verification — 2026-10-01 UTC

On macOS arm64, `83eb87e` plus the uncommitted milestone-07 work:

```sh
GPUIO_JOBS=2 ./scripts/gpuio exec dune build -j2 @all @runtest @fmt
GPUIO_JOBS=2 ./scripts/gpuio exec cargo test --workspace --locked -j2 --no-fail-fast
GPUIO_JOBS=2 ./scripts/gpuio exec cargo test --locked -j2 -p gpuio-native \
  --features native-image-tests,native-canvas-tests --lib
GPUIO_JOBS=2 ./scripts/gpuio exec cargo clippy --locked -j2 \
  -p gpuio-native -p gpuio-protocol \
  --features gpuio-native/native-image-tests,gpuio-native/native-canvas-tests \
  --all-targets -- -D warnings
GPUIO_JOBS=2 ./scripts/gpuio exec cargo test --locked -j2 -p gpuio-native \
  --features native-image-tests,native-canvas-tests --test native_presentation --no-run
GPUIO_JOBS=2 python3 scripts/test_extension_consumer.py --example gallery \
  --workspace scratch/agents/root-20260929-m7-resumed/rating-appearance-installed-consumer
```

All passed. The feature-enabled library suite reports 427 passing tests and two
ignored private-bus tests on macOS. The consumer result is `run=False`; it builds
against staged installed libraries without modifying another opam switch.
Rust formatting, Python syntax and the structural catalog audit also pass.

Two new Core expect tests cover independently specified operation/capability
bytes and resolved theme/alpha changes, stable node/callback identity, missing
token failure, default normalization and resets. The gallery reducer check covers
size/appearance retention through read-only changes, stale range requests,
disabled requests and the alternate click policy. Native admission checks verify
wrong-kind/out-of-range rollback, handler/value preservation, retained storage
and reset. Rust codec checks reject truncated/trailing/malformed colors and
match the independently written `rating-appearance.hex` fixture.

`rating_appearance_test.rs` is linked into `native_presentation` and the gallery
driver is `scripts/test_gallery.py --section rating`. The public gallery scenario now passes on macOS; see the
[dated evidence](../evidence/numeric-disabled-rating-och41.md). The dedicated GPU
appearance scenario remains **unrun**. Their assertions cover GPU colors/alpha, hover, inherited opacity
and foreground, reset, geometry, actual input and retirement; they do not become
passing native evidence through compilation. No new Linux, hosted CI, VoiceOver
or physical display-scale acceptance is claimed.
