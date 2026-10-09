# Chart inspection parent envelope — OCH-41

2026-10-06, macOS 14.5 arm64, base `47c8a1a` plus this change.
The [content contract](../design/chart-inspection-content.md) now has transport
and retained-tree admission support. This is still preparatory: public
`View.chart` attachment, native arbitrary-child rendering/interaction and gallery
qualification remain required. OCH-41 stays open.

Chart-view schema **-2** inserts inspection entries after radar axis IDs and before
legend/disabled flags. Older -1 frames are explicitly rejected; their fixtures
are retained. Options/style/data stay **9/-7/1**, requiring matching bridge
packages. The parent byte budget adds the standalone metadata's 16-KiB allowance;
the enclosing transaction cap is unchanged. Retained accounting includes metadata
vector capacity. Empty inspection lists preserve existing application behavior.

Native tree admission requires exactly the combined wrapper count, radar first,
with one child in each empty-text Container. Both families retain their independent
limits (64 radar, 128 inspection). `Chart.Expert.with_inspection_content` binds
metadata using the config's data identity; content for another source stays hidden.
The ordinary `View.chart` API does not expose this incomplete renderer yet.

## Evidence

- Focused OCaml chart expects pass the independent mixed-parent fixture, binding
  a foreign-source aggregate to a hidden slot, duplicate rejection, old-version
  rejection and the 128/129 boundary. Existing ordinary/radar fixtures now use -2.
- All **453 protocol tests** pass. New parent cases cover the same mixed bytes,
  every truncated prefix, invalid nested metadata and explicit old-frame rejection.
  The maximal style/options case now also contains 64 radar and 128 long-integer
  aggregate entries; parent decode and existing transaction-size bounds pass.
- All **four native chart-tree tests** pass. Raw mixed-slot transactions reject
  count mismatches, duplicate/oversized metadata, an old schema and broken wrappers
  atomically, preserving revision, child structure and retained-byte count.
  A valid inspection reorder preserves child IDs; complete removal returns the
  retained-tree count to zero. This is not a public View reconciliation test.
- Native feature library tests pass **1,052**, with two existing ignored tests.
  Strict workspace and feature-enabled Clippy pass.
- The existing hidden-window mounted chart regression passes real GPU paint,
  source publication/reset, radar content/resources and cleanup, with final
  metrics `(177, 19, 2, 0, 0)`. It does not render the new inspection slots or
  establish foreground keyboard/IME/VoiceOver acceptance for them.
- Full Dune `@all @runtest @fmt` passes. The example documentation inventory
  remains 421 source files in 262 reviewed groups.

The current label adapter renders only the radar prefix and keeps remaining
wrappers hidden. The complete inspection adapter must supply target eligibility,
measurement, pointer/focus ownership and immediate stale-action retirement before
the public API is enabled. No current-source hosted, fresh-installed consumer or
whole-release qualification is inferred from this preparatory work.

[Logs and exact commands](chart-inspection-parent-och41-logs.tar.gz) and the
[verified manifest](chart-inspection-parent-och41-manifest.json) retain the results,
fixture derivation and final source-file hashes.

## Commands

All commands use `GPUIO_JOBS=2` and the repository-local environment.

```sh
./scripts/gpuio exec dune build -j2 @test/chart/runtest
./scripts/gpuio exec cargo test -p gpuio-protocol --locked -j2
./scripts/gpuio exec cargo test -p gpuio-native --test chart_tree --locked -j2
./scripts/gpuio exec cargo test -p gpuio-native --features native-canvas-tests,native-image-tests --lib --locked -j2
./scripts/gpuio exec cargo clippy --workspace --all-targets --locked -j2 -- -D warnings
./scripts/gpuio exec cargo clippy -p gpuio-native --features native-canvas-tests,native-image-tests --lib --tests --locked -j2 -- -D warnings
./scripts/gpuio exec cargo test -p gpuio-native --features native-canvas-tests --test native_chart_view --locked -j2
./scripts/gpuio exec dune build -j2 @all @runtest @fmt
```
