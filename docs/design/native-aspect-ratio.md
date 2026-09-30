# Native aspect ratio

OCH-41. `Style.Property.Aspect_ratio` exposes the pinned GPUI layout property
needed by square attachment media and other proportional previews. This does not
complete the Attachment component contract.

```ocaml
Style.create_exn
  [ Width (Length.percent_exn 100.)
  ; Aspect_ratio 1.
  ; Shrink 0.
  ]
```

The value is width divided by height, finite and between `0.000001` and
`1000000`, inclusive. OCaml construction and native admission reject zero,
negative, nonfinite and out-of-range ratios. These bounds preserve a positive,
finite `f32` at the native boundary. Percentages retain their normal containing
block semantics. Ratio is not inherited and does not crop content or create a
new resource, callback or timer.

This is a preferred native layout ratio. An automatic dimension can follow a
definite dimension; two explicit dimensions retain their authority. Native
flex/grid sizing, padding/borders, intrinsic content and min/max constraints
still apply. In particular, pinned Taffy transfers min/max constraints through
an aspect ratio before clamping preferred dimensions: `Width 120px`,
`Min_height 100px`, ratio 2 produces 200×100 in the checked flex layout. Likewise,
`Width 120px`, `Max_height 90px`, ratio 1 produces 90×90. See the pinned
[GPUI property](../../vendor/gpui/src/style.rs) and
[Taffy flex layout](../../vendor/taffy/src/compute/flexbox.rs). Do not assume a
width declaration overrides transferred minimum/maximum constraints.

Base and interaction states follow ordinary style precedence. Last declaration
wins in a layer. `Style.unset ... Aspect_ratio` removes that layer's declaration:
unsetting a hover override exposes the base; unsetting the base exposes the
receiving component default. Removing all ratio declarations from an otherwise
empty, auto-height container restores zero content height. Receiving specialized
widgets may have their own sizing defaults; the current native matrix uses
ordinary retained containers.

## Transport and native integration

`Field.Aspect_ratio` appends tag 68 (`44` hex) with one bin_prot float64; tags
0–67 retain their encodings. The mapper writes the existing
`gpui::StyleRefinement.aspect_ratio`; no dependency or fork change is required.
Malformed ratios reject the complete transaction before publication, including
preceding text changes. Revision and retained-byte accounting stay unchanged.

Capability bit 50 (`CAP_ASPECT_RATIO`) requires a matching host. The full mask
at introduction was `2251799813685247`; see [bridge-v1](bridge-v1.md) for
the current aggregate mask. A paired client/host rebuild is required; an older host
cannot silently ignore a layout property.

| Payload | Hex |
| --- | --- |
| Ratio 0.5 | `44000000000000e03f` |
| Ratio 1 | `44000000000000f03f` |
| Ratio 2 | `440000000000000040` |
| Hello requiring aspect ratio | `0001fc0000000000000400` |
| Hello requiring all capabilities at introduction | `0001fcffffffffffff0700` |

## Validation

The background `native_aspect_ratio` fixture uses the production Session/Host
and actual GPU readback. Sixteen geometry cases cover proportional sizing after
parent resizing, percentage widths inside padding, width-from-height, explicit
dimensions, and transferred min/max constraints. Additional dispatched GPUI
pointer checks cover hover override/restoration; style removal clears the ratio,
and unmount releases retained tree bytes. The first draft used the wrong wire
value for the fixture's column; another expectation omitted native min/max
transfer. Both fixture corrections were checked against the pinned source.
Neither required a renderer or layout-engine change.

The public Styling details page has a proportional preview containing a stateful
button. Its focused `--section aspect-ratio` check passes twelve combinations of
two themes, three ratios and two widths, plus explicit-height reset. Actual macOS
AX bounds and retained surface/button identities pass; thirteen targeted OS
Return events verify retained keyboard focus and working state. Page departure
removes the native subtree; remount creates a fresh native button with the retained
Bonsai count. The first driver checked absence before the asynchronous page change
settled; it now waits for destination-page content before checking teardown.

Both the normal repository application and a fresh independently staged consumer
pass that check on macOS 14.5 arm64. The latter uses
`/private/tmp/gpuio-m7-aspect-consumer-20260930`, its own installed-library prefix
and independent backend lockfile, with the existing pinned toolchain and repository
native sources. No opam switch was modified. Each GUI run had a 480-second
process-group deadline and closed/reaped its application normally.

The complete local Dune build/expect/format suite, 242 Rust protocol tests, 413
enabled native unit tests, six session tests and three style-validation tests pass.
Two private-bus tests remain required on Linux. Strict native/protocol all-target
Clippy, Rust formatting, Python syntax, workflow actionlint and structural catalog
checks also pass. CI compiles the native fixture on both platforms and runs its
bounded background GPU check on macOS; hosted acceptance remains pending.

```sh
GPUIO_JOBS=2 ./scripts/gpuio exec dune build -j 2 @all @runtest @fmt
GPUIO_JOBS=2 ./scripts/gpuio exec cargo test --locked -p gpuio-protocol -j 2
GPUIO_JOBS=2 ./scripts/gpuio exec cargo test --locked -p gpuio-native -j 2 --lib --test style_values --test session
GPUIO_JOBS=2 ./scripts/gpuio exec cargo test --locked -p gpuio-native -j 2 --features native-image-tests --test native_aspect_ratio
python3 scripts/test_gallery.py --section aspect-ratio
GPUIO_JOBS=2 python3 scripts/test_extension_consumer.py --example gallery \
  --workspace /private/tmp/gpuio-m7-aspect-consumer-20260930
python3 scripts/test_gallery.py --section aspect-ratio \
  --executable /private/tmp/gpuio-m7-aspect-consumer-20260930/consumer/_build/default/main.exe
```

This is scoped ordinary-container layout and macOS keyboard/AX evidence, not IME,
screen-reader, Linux desktop, application performance or clean-machine distribution
acceptance. Attachment integration and specialized-widget sizing remain separate.
