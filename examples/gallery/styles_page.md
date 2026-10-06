# Styling page composition

Read [styles_page.ml](styles_page.ml) and its [interface](styles_page.mli). [pages.ml](pages.ml)
routes Styles with application theme loader and shared theme-selection Var. `B` aliases
`Bonsai.Cont`, `V` `Gpuio_bonsai.View`; graph hosts reactive state/child computations, and
`let%arr` derives current views.

The [theme loader](theme_preview.md) owns scoped loading/application appearance changes; this
page passes capabilities rather than reading files during rendering. The
[aspect preview](aspect_preview.md) is separately composed. Local `source` is a fictional path
string, not a file dependency. `cursors` enumerates 22 typed cursor choices; state machine
starts index 0 and returns current model plus unit-action injector reduced against latest model
on execution. Width/Oklab/thick start false; dashed/rounded true.

Native Next cursor activation runs `next ()`, advances index, and makes `let%arr` derive Cursor
style/readout; GPUIO updates pointer artwork over the named surface. Effect construction does
not change cursor immediately. Cursor artwork is platform-specific; equal macOS shapes do not
mean the typed values are identical.

`sample` builds stable text leaves at width 250 or 140 with No_wrap, Overflow Hidden and
Clip/Ellipsis/Ellipsis_start. Truncation changes paint, while full source remains accessible.
Border preview derives solid/dashed, width 1/4 and radius 16/0 around a stable key. Gradient
uses the same red/blue stops and 90-degree angle with explicit Srgb/Oklab interpolation;
toggling the color-space model derives a new checked background description, without changing a
global native theme.

Compare truncation/width, pointer shapes, borders and gradient interpolation. Native GPUIO owns
shaping/paint/cursor; Bonsai owns choices, and child theme scope owns actual file work. No local
assets/timers/controllers are created for these surfaces. Adapt with your application text/theme
tokens and validated styles. The supplied path is not opened, and retained choices/native
rendering do not constitute physical accessibility or platform acceptance.

## Run

From the repository root:

```sh
./scripts/gpuio build examples/gallery/main.exe
./_build/default/examples/gallery/main.exe
```

These commands were not executed for this documentation change. The component has no standalone
executable/self-test; compilation does not establish native interaction or platform acceptance.
See [gallery instructions](README.md) and [development](../../docs/development.md).
