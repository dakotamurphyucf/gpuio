# Chart appearance values and codec — OCH-41

2026-10-06, macOS 14.5 arm64, source base `f22abd8`. This is an implementation
checkpoint, not chart rendering or whole-catalog acceptance.

The [appearance design](../design/chart-mark-appearance.md) now has typed OCaml
constructors and theme resolution, matching owned Rust representations and a
bounded decoder. Values describe series paths, markers, independent bar corners,
four fill coordinate modes, explicit legend colors and stable-ID datum overrides.
The [interface](../../lib/core/chart_appearance.mli) explicitly states that
attachment to `Chart_style` and native rendering are still pending. Existing
style schema -4 and rendered defaults are unchanged.

Validated locally:

```sh
GPUIO_JOBS=2 ./scripts/gpuio exec dune build -j2 lib/core/gpuio.cma
GPUIO_JOBS=2 ./scripts/gpuio exec dune build -j2 @test/chart/runtest
GPUIO_JOBS=2 ./scripts/gpuio exec cargo test -p gpuio-protocol --test chart_appearance
GPUIO_JOBS=2 ./scripts/gpuio exec cargo test -p gpuio-protocol --locked -j2
GPUIO_JOBS=2 ./scripts/gpuio exec cargo clippy --workspace --all-targets --locked -j2 -- -D warnings
GPUIO_JOBS=2 ./scripts/gpuio exec cargo fmt --all --check
GPUIO_JOBS=2 ./scripts/gpuio exec ocamlformat --check lib/core/chart_appearance.ml lib/core/chart_appearance.mli lib/protocol/chart_appearance_wire.ml test/chart/chart_appearance_test.ml
python3 scripts/audit_example_docs.py
git diff --check
```

All pass. The full Rust protocol suite passes **444 tests**; three are new
appearance tests. The OCaml chart suite includes five new expectations. Strict
Clippy succeeds with the repository's existing dependency warning handling;
vendor deprecation warnings remain in the log.

The independently constructed OCaml and Rust values agree with the checked-in
[byte fixture](../../test/fixtures/chart-appearance.hex). Tests cover omitted
versus explicit settings, all four fill modes, an Oklab brush, ordered corner
fields, every truncated prefix of that fixture, trailing bytes, invalid tags,
numeric/color/ID bounds, duplicate series/datum pairs and bounded list-count
admission before reading elements. A datum ID may repeat in another series.

Both languages construct the largest legal representation using maximum-width
IDs/colors, every optional field, 128 series and 1,024 datum overrides. The encoded
size is **173,447 bytes**, below the independent 192 KiB decoder envelope. The
parent style's combined maximum still needs verification when it gains this
payload; this does not prove that the old parent envelope can contain both.

Theme tests resolve colors in all fill modes and reject a missing token at
23 independently exercised nested positions, including hidden/dormant values.
The initial expect run exposed a test fixture mistake: `accent` already exists
in the default theme. The test now uses a unique missing name; the failed
backtrace was not promoted. Literal fixture bytes and size expectations were
reviewed before the passing run.

The catalog also corrects a source-reading error: the pinned area shape exposes
an optional scalar `y0`, not arbitrary lower/upper per-datum accessors. This
clarifies the remaining scope rather than claiming that a missing feature ships.

No GUI test, VoiceOver session or Linux run was performed for these new values.
Hosted run 37523471664 was still building older `f22abd8` at the last check; it
does not validate this checkpoint. Native preparation/painting, stack-curve
admission, aggregate resolution, per-marker hit bounds, gallery usage and actual
GPU/input/lifecycle checks remain required. Rich inspection and broader catalog
and release requirements also remain open. No milestone gate is waived.
