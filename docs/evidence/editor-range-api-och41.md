# OCaml editor range query — OCH-41

Implementation checkpoint 2026-10-04, macOS arm64, worktree based on
`83eb87e865c86717a8bc51b9db6fe1f379d909a9`. The dirty worktree contains the actual
sources; HEAD alone does not identify this implementation. Local boundary checks pass; physical acceptance remains open. OCH-41 and OCH-17 remain open.

## Delivered boundary

`Gpuio_eio.Text_input.range_bounds editor ~snapshot ~range` queries the native
lease and source revision carried by an explicit text snapshot. Core exposes
validated immutable `Editor_geometry` (revision and unclipped window-content
x/y/width/height). Directed selections are normalized natively, with UTF-8
boundaries validated against both the supplied snapshot and current native text.
The request sends only revision and offsets. Read-only/disabled fields and active
composition may be inspected without focus, scroll, edit or history changes.

The query reuses the [validated native helper](editor-range-geometry-och41.md),
the existing 64-request editor budget, correlation and window-close rules. It
returns `Stale_revision` for a different live source; `None` distinguishes an
unavailable matching layout. Present bounds must carry the requested revision.
Late metadata never replaces the controller's newer text observation. Results
describe native execution, not guaranteed current geometry on delivery.
See [the contract](../design/editor-range-geometry.md).

The public gallery's **Room to write → Inspect selection bounds** reads a fresh
snapshot, then queries that snapshot's selection. The button preserves focus.
One request chain is active per preview; departing the page cancels its child
scope and suppresses later query steps/status updates. There is no timer or
continuous geometry subscription. The later [macOS geometry checkpoint](editor-accessibility-geometry-och41.md)
verifies the public inspector and repairs native AX range publication; it
distinguishes AX actions from physical keyboard/IME/VoiceOver qualification.

## Validation record

Logs are under ignored `scratch/agents/root-20261003-release-notices/` and are
not build dependencies. Initial core/Eio library compilation passed. The next
focused build found a missing numeric-editor command classification, an unused
Rust import and an unqualified constructor in an OCaml fixture; those were fixed.
Only formatter output has been promoted, never expected test output.

All commands use the repository environment on macOS arm64. Final checks:

| Command | Result | Log |
| --- | --- | --- |
| `GPUIO_JOBS=2 ./scripts/gpuio exec dune build -j2 @runtest examples/gallery/main.exe` | Passed full OCaml suite and independent gallery backend build | `editor-range-api-full-ocaml-002.log` |
| `GPUIO_JOBS=2 ./scripts/gpuio exec cargo test -p gpuio-native --lib --features native-canvas-tests,native-image-tests --offline --locked -j2` | 914 passed; two existing macOS private-bus skips | `editor-range-api-native-001.log` |
| `GPUIO_JOBS=2 ./scripts/gpuio exec cargo test -p gpuio-protocol --offline --locked -j2` | 392 passed | `editor-range-api-protocol-001.log` |
| `GPUIO_JOBS=2 ./scripts/gpuio exec cargo clippy -p gpuio-native -p gpuio-protocol --all-targets --features native-canvas-tests,native-image-tests --offline --locked -j2 -- -D warnings` | Passed | `editor-range-api-clippy-001.log` |
| `GPUIO_JOBS=2 ./scripts/gpuio check-fmt` | Passed | `editor-range-api-format-check-001.log` |

The first full gallery build found a misplaced `disabled` argument on the button
config instead of `View.button`; the corrected public call passes. The existing
`block 0.1.6` future-compatibility notice remains.

Passing tests for this boundary cover:

- Independent Rust/OCaml request/event bytes, signed coordinates, zero-width
  carets, revision and dimensional limits, malformed options, truncation,
  trailing bytes and invalid source offsets.
- Native directed ranges, source revision mismatch, invalid UTF-8 boundaries,
  unpainted composition, nonmutation, disabled/read-only fields and retired
  native node identity.
- Eio wrong-node/correlation replies, duplicate replies, revision/geometry/kind
  mismatch, preflight source validation, shared capacity, close cleanup and late
  replies that preserve newer controller text.

The Base pin/patch are unchanged from the native-helper checkpoint. No new
dependency, fork change, global switch change, OS window or hosted check belongs
to this API slice. Linux execution, physical macOS shaping/IME/accessibility,
gallery walkthrough and all broader release gates remain separate requirements.
