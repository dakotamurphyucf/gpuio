# Selectable text shimmer walkthrough

Read [shimmer_preview.ml](shimmer_preview.ml) and its [interface](shimmer_preview.mli). [pages.ml](pages.ml) mounts `component palette graph` on **Presentation**. `S` aliases [`Text_shimmer`](../../lib/core/text_shimmer.mli), a glyph highlight rather than Loading.Kind.Shimmer’s rectangular placeholder.

Five reactive `B.toggle` values own effect enabled (true), playing/reverse/once/compact (false). `B.state_machine0` owns revision 1 and returns current model plus a unit-action injector; executing the effect increments the latest revision. `let%arr` derives the text and configuration from current values. Merely constructing `refresh ()` does not execute it. Native Refresh shimmer status activation injects the unit action, changes revision, derives changed text under the same key, and reconciles the native text/shimmer presentation.

`S.Appearance.create` derives application-owned dark mode and foreground/background from Palette, recreated when palette values change. `S.Config.create` selects animated state, physical left/right direction and Once/Loop repeat. Defaults provide a two-second sweep and relative spread 0.3; this file starts paused. Configuration construction owns no timer/task/callback and by itself animates nothing.

`V.with_text_shimmer` attaches Some config only when enabled, otherwise removes the effect. The [View contract](../../lib/core/view.mli) accepts only ordinary text/styled text, limits enabled source to 16,384 UTF-8 bytes, and falls back to static rendering above 256 lines or 4,096 shaped glyphs. Frame-wide candidate/glyph budgets can also pause effects. This API remains experimental: native rendering exists, but public acceptance and host capability advertisement remain in progress. Its target is an ordinary selectable V.text keyed `shimmer-status`, with two lines containing Latin, CJK, combining-accent and joined emoji samples. Compact changes width 480→280 while Max_width 100% keeps it within its parent. Native text layout determines glyphs and wrapping; text content and default selection remain ordinary text.

Start, reverse direction, select One sweep and refresh changed status to replay a completed sweep. Native animation handles frames; reduced motion/animated=false request static text. Once completion does not set the Bonsai playing boolean false—this component has no completion callback. The playing button describes requested configuration, not an observed running/completed state. Refresh changes displayed content, not a server connection.

GPUIO owns layout, selection and native animation; Bonsai owns preferences/revision. There is no asset registration, editor controller, Eio work or cleanup callback. Adapt with application status text and theme-derived appearance, retain meaningful static content and avoid an OCaml animation loop. Do not interpret shimmer motion as progress, network activity or acceptance evidence across platforms.

## Run and review

From the repository root:

```sh
./scripts/gpuio build examples/gallery/main.exe
./_build/default/examples/gallery/main.exe
```

Use the repository wrapper for the isolated toolchain. These commands were not executed for this documentation change. There is no standalone executable or self-test for this component. Compilation alone does not establish native keyboard, animation, focus or platform acceptance. See [gallery instructions](README.md) and [development](../../docs/development.md).
