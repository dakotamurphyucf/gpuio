# Dense bar background foundations

OCH-41, 2026-10-06. Local macOS 14.5 arm64, Apple M1 Max, after `8d44c58`.
The [design](../design/dense-bar-backgrounds.md) defines immutable data-owned
brushes, appearance precedence, aggregate semantics and data schema 2.
**Public gallery and fresh installed-consumer qualification remain open.** This
checkpoint does not close the catalog row or any broader release gate.

## Implementation

`Chart_data.with_bar_backgrounds` replaces a bounded sidecar keyed by typed
series/datum identities. Construction resolves theme tokens and rejects unknown,
duplicate or nonbar references. Numeric/categorical source order and values stay
unchanged. The whole source, including brushes, must fit 16 MiB. All existing
point/text/style limits remain. Source identity and brushes publish atomically;
there is no separate resource lifetime or render-time OCaml callback.

Native preparation uses a sorted vector lookup. Data fills override series fills;
sparse datum fields override only the fields they supply. A corner-only override
retains the data fill. Single-defined-observation aggregates keep that observation's
appearance and their aggregate provenance. Multi-observation `Inherit_series`
falls back to series appearance; `Uniform` requires agreement of effective fills
and corners, excluding gaps.

The snapshot charge includes the sidecar's actual vector capacity, and the
unchanged 64 MiB decode-workspace bound explicitly includes its maximum storage.
OCaml's conservative registration estimate adds 512 bytes per background. Existing
128 MiB OCaml and 256 MiB native source limits still apply across retained
revisions; validity of an individual source does not guarantee room for a
replacement alongside it. Ordinary point records retain their prior layout.

The owned wire record appends the sidecar and advances data from 1 to **2**.
View/options/style remain -2/9/-9. Both readers reject old data explicitly and
bound counts before allocation. Historical schema-1 fixtures are retained; new
schema-2 fixtures are checked independently by OCaml and Rust. Matching packages
are required.

## Validation

- The OCaml chart expects cover canonical order, source preservation, clearing,
  typed reference/theme/brush failures, old schema rejection, categorical gaps,
  a 100,000-entry roundtrip, accounting and a combined encoding that exceeds
  16 MiB despite legal point/text counts.
- The full Rust protocol run passes 462 tests. A subsequent focused run passes
  all six background tests, including the additional malformed sidecar-count
  case. The shared fixture verifies exact bytes, every truncation and trailing
  input; 100,000 entries and the combined byte limit are tested independently.
- Native unit tests pass **1,057**, with two existing ignored tests. New checks
  cover dense-only resolution, sparse precedence, aggregate appearance, gaps,
  cancellation and source ownership. A real 100,000-entry resource is decoded
  on a worker, published and replaced; held retired snapshots remain charged,
  and final release returns source accounting to zero.
- Actual hidden-window GPU readback passes **48** new cases: solid fills, sparse
  override and checkerboard × four directions × scales 1/1.25/1.5/2. Descending
  source IDs and ascending sidecar keys prove association follows identity.
  Green/blue/yellow and empty controls verify independent fill selection;
  checker cases require both blue and transparent cells. Existing chart family,
  gradient, clipping and geometry cases pass in the same executable.
- Strict Clippy passes with all three native test features and all targets.
  Full Dune `@all @runtest @fmt` and the final Rust formatting check pass.

Two version-rejection tests required explicit fixture maintenance: a malformed
count test still used schema 1, so it stopped at version rejection; a native test
still used version 2 as its unsupported version. Their controls now reach the
intended conditions under schema 2, and historical schema-1 rejection has its
own coverage. An initial OCaml compile also identified the required comparator
witness for the pair key; the implementation uses `Comparator.Make`. Test
expectations were reviewed explicitly, not automatically promoted.

The GPU checks use hidden windows. They do not qualify foreground keyboard,
IME, VoiceOver, physical presentation timing, Linux desktop behavior or application
packaging. Public gallery theme republishing, interaction and installed-consumer
checks remain required. Per-bar baselines and arbitrary pixel-bound callbacks
remain separate catalog work.

```sh
GPUIO_JOBS=2 ./scripts/gpuio exec cargo test -p gpuio-protocol --locked -j2
GPUIO_JOBS=2 ./scripts/gpuio exec cargo test -p gpuio-protocol --test chart_backgrounds --locked -j2
GPUIO_JOBS=2 ./scripts/gpuio exec cargo test -p gpuio-native --features native-image-tests --lib --locked -j2
GPUIO_JOBS=2 ./scripts/gpuio exec cargo test -p gpuio-native --features native-canvas-tests --test native_chart_paint --locked -j2
GPUIO_JOBS=2 ./scripts/gpuio exec cargo clippy -p gpuio-native --features native-tests,native-canvas-tests,native-image-tests --all-targets --locked -j2 -- -D warnings
GPUIO_JOBS=2 ./scripts/gpuio exec dune build @all @runtest @fmt -j2
GPUIO_JOBS=2 ./scripts/gpuio exec cargo fmt --all -- --check
```

The [raw logs](dense-background-foundation-och41-logs.tar.gz) and
[SHA-256 manifest](dense-background-foundation-och41-manifest.json) contain
17 members (3,490,134 uncompressed bytes), including initial failures, exact
final commands/exits and hashes of the changed source and fixtures. Archive
members and hashes were verified after creation.
