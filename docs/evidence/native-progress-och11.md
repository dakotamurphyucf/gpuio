# OCH-11 local progress-indicator evidence

Platform: macOS arm64; stock OCaml 5.3, Bonsai/Core v0.17, Eio, Dune 3.24.2,
Rust 1.97.1 and GPUI `a57ba9b17c433ea1ebfdec8f649f4fa5a402d03b`.
No upstream or vendor patch was added. This is a local component checkpoint;
OCH-11 and OCH-12 remain unfinished.

## Reproduction

Run sequentially from the repository root; Dune also invokes Cargo:

```sh
./scripts/gpuio exec dune build @runtest @all @fmt
./scripts/gpuio exec dune exec examples/progress/main.exe -- --self-test
./scripts/gpuio exec cargo test --workspace --locked
./scripts/gpuio exec cargo clippy --workspace --all-targets --features native-tests --locked -- -D warnings
./scripts/gpuio exec cargo fmt --all -- --check
./scripts/gpuio exec cargo test -p gpuio-native --features native-tests --test native_progress --locked
./scripts/gpuio exec cargo test -p gpuio-native --features native-tests --test native_controls --locked
```

All passed locally. `native_progress` runs the real-window scenario independently;
`native_controls` includes it after the existing controls, focus, overlays,
tooltips, commands, menus and palette scenarios. The combined run passed; the
previously recorded intermittent tooltip-hover failure did not recur. Its cause
remains unproven and is still recorded for final OCH-11 integration work.

## What the checks establish

- Actual canvas paint has the expected 25%, 50% and 100% widths and configured
  height/foreground color. The indicator responds to Indeterminate state styling.
- macOS accessibility reports ProgressIndicator, its label and percentages,
  including zero; indeterminate progress has no numeric value.
- The element does not take keyboard focus when clicked and publishes no
  activation event. An initial native test caught ancestor default-focus behavior;
  the progress element now prevents that mouse default while preserving ordinary
  event propagation.
- Indeterminate paint advances during a timer wait without requesting a test frame
  or changing the accepted OCaml tree revision. This establishes native animation
  without OCaml commits, rather than merely forcing several test renders.
- Hidden progress stops painting; returning to determinate stops requesting
  animation frames. Removing the element releases the retained paint closure.
- OCaml expect tests reject invalid fractions/labels and verify stable identity,
  metadata-only updates and no-op reconciliation. Independent Rust/OCaml request
  bytes agree. Rust session tests reject invalid numbers/handlers atomically,
  preserve prior revision/budget on failure, and release metadata on removal.
- The public Bonsai/Eio example passes native mount, transitions through
  indeterminate/halfway/completed values, unmount and application shutdown.

The targeted and combined native scenarios passed before a subsequent log-only
change separated the macOS accessibility marker from the platform-neutral marker.
No behavior changed in that marker adjustment.

## Limits and remaining work

These are real local GPUI/AppKit programmatic paint/input/accessibility checks,
not a complete screen-reader, physical input-device or display-scale audit.
Required Linux build/unit validation and consolidated hosted CI remain pending
until the full remaining OCH-11 implementation is ready, per the owner workflow.
Full Linux GUI acceptance remains OCH-17; no Linux GUI success is claimed here.
OCH-12 still supplies the public declarative animation API and shared motion/
reduced-motion integration. In-app notifications, pointer/desktop interactions,
assets and the remaining combined OCH-11 acceptance checks are separate work.
