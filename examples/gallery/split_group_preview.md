# Retained split panels walkthrough

[split_group_preview.ml](split_group_preview.ml) has no separate interface.
[navigation_page.ml](navigation_page.ml) mounts it on **Navigation**. `B` is `Bonsai.Cont`, `V`
is `Gpuio_bonsai.View`, and `Input` is Gpuio_eio.Text_input. `graph` hosts reactive
options/controller computations; `let%arr` derives config/content from their current values.

Toggles own reversed order, inspector visibility (true), outline insertion, axis, limits and
custom grips. `counter` creates saturating Int64 state machines for resize serial and reset
generation. Each returns current model and unit-action injector reducing against the latest
model when executed. The editor is created once outside derivation, with a multiline draft.
Constructing a toggle/request effect does not execute it.

Panels have stable files/draft/inspector IDs plus optional outline.
[`Split_group.Panel`](../../lib/core/split_group.mli) seeds draft at 150 pixels, others 100,
with minimum 60 and maximum 800 or 200. Inspector visibility hides a retained panel; removing
Outline removes its content. Reversal changes order without changing identities. Config supplies
axis, reset generation and optional 320-pixel draft resize with positive serial. Native
sizes/gestures belong to Rust, not these initial seeds.

Native Resize draft activation injects a counter action, increments serial, and makes `let%arr`
derive a new one-shot request. GPUIO applies it with panel/sibling constraints; a completed
resize snapshot runs `observe`, updates the notice model, and derives size text. Under the
200-pixel maximum, 320 is a request subject to clamping. `Reset` increments generation to
reapply size seeds, not recreate the draft. Counters stop at `Int64.max_value`, so cannot issue
another distinct request after exhaustion.

`V.split_group` mounts key flat-workspace in bounded height 220 or 360, with full-width panel
content and independent scrolling. Appearance separates painted thickness 3 from hit extent 16;
optional passive text grips change artwork, not input geometry. Pointer, keyboard and AX
resizing remain native. Type in Draft, reorder/hide/add panels, then resize/reset to exercise
retained ownership.

No worker/asset scope exists. Bonsai owns identities/options/notices, GPUIO owns
layout/sizes/input and the editor adapter owns its lease. Adapt with stable domain IDs, finite
ordered constraints and increasing serials. Do not continuously feed observations back as new
resize requests or mistake initial_size for a controlled size updated each render. Store draft
data externally if it must survive page destruction.

## Run and review

From the repository root:

```sh
./scripts/gpuio build examples/gallery/main.exe
./_build/default/examples/gallery/main.exe
```

These repository-wrapper commands are instructions, not validation performed for this
documentation change. There is no standalone executable or self-test for this component.
Compilation does not establish native focus, keyboard, animation or platform acceptance. See
[gallery instructions](README.md) and [development](../../docs/development.md).
