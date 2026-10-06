# Internal measured Sankey layout engine

2026-10-06, macOS 14.5 arm64. Implementation starts from `bb24014`.
This implements the pure geometry portion of the
[outside-label design](../design/sankey-label-placement.md). **The public option,
wire schema, worker text measurement and native example are not implemented yet.**
The production entry point still supplies no metrics and uses the existing
inside-label policy; options/style/data remain 5/-2/1.

`chart_geometry::prepare_with_flow_labels` accepts measured block dimensions in
the exact source node order. It validates those dimensions, reuses one topology,
reserves independently capped side/top/bottom margins and fits node width/padding
to the remaining extent. Per-node None hides a label; globally disabled labels
reserve no space. Placement metadata distinguishes aligned side labels from
centered above-node blocks and survives rich multiline expansion. Presentation
clips one coherent block instead of moving each line onto its neighbor.

Four new tests cover three columns in both source orders, all four alignments
and both flow scales; explicit first/last/middle placement and side caps; zero-flow
omission and original node hit provenance; hidden/global-disabled labels; rich
multiline clipping down to 0.125 × 0.25 logical pixels; single-column precedence;
malformed metric lengths/dimensions and cancellation. These supplied metric values
are test inputs, **not approximate runtime font measurements**.

Commands, using the isolated repository toolchain:

```sh
./scripts/gpuio exec cargo fmt --all
GPUIO_JOBS=2 ./scripts/gpuio exec cargo test --locked -p gpuio-native --lib chart_ -- --test-threads=2
GPUIO_JOBS=2 ./scripts/gpuio exec cargo test --locked -p gpuio-native --lib --features native-tests
GPUIO_JOBS=2 ./scripts/gpuio exec cargo clippy --locked -p gpuio-native --lib --tests --features native-tests --no-deps -- -D warnings
./scripts/gpuio exec cargo fmt --all --check
git diff --check
```

Initial focused tests: 90 pass, two fail because new coordinate assertions assumed
exact f64 equality for the pinned f32 layout (e.g. 60.000030517578125 versus 60).
Assertions now permit 0.0002 logical pixels at the tested 500-pixel scale; source
identity and discrete placement policy stay exact. Hit-provenance checks were
added before the final run. The full library suite passes **639 tests, two existing
skips**. Strict first-party Clippy and formatting pass; upstream deprecation/
future-compatibility warnings remain. This is the native **library** suite, not
every external native/GUI harness. No OCaml files, dependencies or GUI were changed
or executed by this slice.

[Archived initial failure, final library-test and Clippy logs](sankey-label-layout-engine/logs.tar.gz)
and [verified checksums](sankey-label-layout-engine/manifest.json) retain the raw
evidence unchanged. An initial whitespace check flagged a trailing blank line in
the raw test log; archiving the exact bytes keeps diagnostics out of source
whitespace checks. The final repository whitespace check passes.
Runtime worker metrics/font invalidation, paired public codecs, real painting/
input, independent installed-consumer checks and Linux qualification of this
source remain the next steps. This does not close OCH-41 or release acceptance.
