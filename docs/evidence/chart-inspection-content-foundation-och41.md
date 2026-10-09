# Chart inspection content foundation — OCH-41

2026-10-06, macOS 14.5 arm64, source base `e41b7d6` plus this change.
The [integration contract](../design/chart-inspection-content.md) preserves the
pinned Tooltip's arbitrary-child capability. This increment adds validated
OCaml target/content values and standalone paired metadata codecs. It does not
attach content to `View.chart` or implement native rendering, input or gallery
coverage. OCH-41 remains open.

Singular targets retain semantic IDs across reorder. Sum/Mean/OHLC targets retain
the actual resource identity, exact publication and complete selection, including
one-observation aggregates. OCaml binding additionally checks the application
owner; mismatched content becomes an explicitly hidden slot. Content remains
generic and is never invoked or serialized by the metadata conversion.

Collections admit at most 128 unique present wire targets. Card/Overlay choice
does not change target identity. Multiple hidden slots are valid. Rust decoding
also bounds the whole standalone metadata message to 16 KiB. No parent chart-view
schema changes: production view/options/style/data remain **-1/9/-7/1**. Adding
actual child slots requires a separately versioned parent envelope and accounting.

## Local checks

- Focused OCaml chart expect tests pass. New cases cover all target tags and
  both containers using independent fixed bytes, stable exact identity, aggregate
  publication changes, different resources and different application owners with
  equal numeric handles, hidden slots, invalid values, duplicates and the cap.
- All **452 Rust protocol tests** pass, including four new metadata tests.
  They cover the same fixed bytes, every truncated prefix, trailing bytes,
  unknown tags, invalid selection/publication/IDs, duplicates, exact versus
  aggregate identity, and maximum-size metadata using long integer encodings.
- Strict workspace/all-target Clippy passes. Existing vendored dependency
  warnings remain; no first-party lint suppression was added.
- Full Dune `@all @runtest @fmt` passes.
- Example inventory remains 421 sources in 262 reviewed groups. A scoped
  GPT-6.1 Sol review clarified the existing inspection preset guide's actual
  `Chart_style` → `Chart.Config` → `V.chart` path; all 12 links pass.

Final review strengthened the admission test to reject otherwise-valid singular
Cartesian/candle selections wrapped as aggregates. The four targeted tests,
targeted protocol lint and formatting pass after this test-only addition and a
module-comment clarification. [Logs and commands](chart-inspection-content-foundation-och41-logs.tar.gz)
and the [verified manifest](chart-inspection-content-foundation-och41-manifest.json)
retain the results and final source-file hashes.

No native window was opened for this metadata-only change. These checks do not
establish custom-content rendering, pointer/focus/IME/accessibility, Linux GUI,
physical presentation, installed-consumer or whole-release acceptance. Hosted
run 37538145025 tests the preceding `e41b7d6`, not this new foundation.

## Commands

```sh
GPUIO_JOBS=2 ./scripts/gpuio exec dune build -j2 @test/chart/runtest
GPUIO_JOBS=2 ./scripts/gpuio exec cargo test -p gpuio-protocol --locked -j2
GPUIO_JOBS=2 ./scripts/gpuio exec cargo clippy --workspace --all-targets --locked -j2 -- -D warnings
GPUIO_JOBS=2 ./scripts/gpuio exec dune build -j2 @all @runtest @fmt
GPUIO_JOBS=2 ./scripts/gpuio exec cargo test -p gpuio-protocol --test chart_inspection_content --locked -j2
GPUIO_JOBS=2 ./scripts/gpuio exec cargo clippy -p gpuio-protocol --test chart_inspection_content --locked -j2 -- -D warnings
python3 scripts/audit_example_docs.py
git diff --check
```
