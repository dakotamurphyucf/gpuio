# Custom spinner walkthrough

Read [spinner_preview.ml](spinner_preview.ml) and its [interface](spinner_preview.mli). [pages.ml](pages.ml) mounts it on **Presentation**, in “Custom spinners”.

[Preview_scope](preview_scope.md) registers the asymmetric arrow SVG and then “Invalid SVG” bytes sequentially through Gpuio_eio.Asset. `Effect.bind` stops after the first registration error; `Effect.map` extracts handles/errors. Ready means registrations exist, not that native SVG decoding succeeded. The scope handles partial failure and visit departure, canceling pending work and disposing registrations; borrowed handles do not independently retain them.

Reactive toggles own animated/slow/linear flags (all false). `B.state_machine0` returns a current mode (initially 1) and unit-action injector cycling modulo three: built-in/arrow/invalid. `B.state` retains latest icon decode state. `let%arr` derives configuration/status. Constructing `next_mode ()` does not execute it. A native source-button click injects the unit action, updates the latest mode from 1 to 2, derives the invalid-handle configuration and updates the existing native spinner. Its later decode callback runs `set_icon_state`, updating that model and deriving fallback status text.

[`Spinner.Config`](../../lib/core/spinner.mli) validates a meaningful label, SVG icon format, 100ms–60s period and easing. This demo chooses 800 or 2000ms, linear or ease-in-out, and stable key `custom-spinner` with 40-pixel bounds. Native decoding uses SVG alpha as decorative monochrome artwork tinted by inherited accent foreground. Loading/failed icons show built-in fallback. The spinner label is the only semantic label, not an additional icon action.

Start animation and cycle source modes to compare custom artwork with failure fallback and built-in strokes. Reduced motion keeps recognizable static artwork. Easing is declarative; there are no per-frame OCaml callbacks. The status is the latest asynchronous decode observation, not a job status or certificate of current paint before an updated observation arrives.

GPUIO owns decode/leases/animation, Bonsai owns controls and last observation, and the visit scope owns registrations. No external work is indicated or started by animation. Adapt with an owned SVG registration, explicit meaningful progress label and a real work-derived animated flag. Do not replace asset registration during each `let%arr` evaluation or assume a borrowed handle survives its scope.

## Run and review

From the repository root:

```sh
./scripts/gpuio build examples/gallery/main.exe
./_build/default/examples/gallery/main.exe
```

Use the repository wrapper for the isolated toolchain. These commands were not executed for this documentation change. There is no standalone executable or self-test for this component. Compilation alone does not establish native keyboard, animation, focus or platform acceptance. See [gallery instructions](README.md) and [development](../../docs/development.md).
