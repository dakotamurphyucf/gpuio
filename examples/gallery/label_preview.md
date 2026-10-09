# Label preview walkthrough

Read [label_preview.ml](label_preview.ml) and its [interface](label_preview.mli). [pages.ml](pages.ml) mounts `component palette graph` on **Presentation**. Use the [gallery development commands](../../docs/development.md); there is no separate executable or self-test for this component.

From the repository root:

```sh
GPUIO_JOBS=2 ./scripts/gpuio build examples/gallery/main.exe
./scripts/gpuio exec _build/default/examples/gallery/main.exe
```

The [gallery README](README.md) records prerequisites and platform limits.
This source walkthrough adds no native input, VoiceOver or platform acceptance.

The local `mode` type is `Plain | Prefix | All`. `B.state All` owns the selected mode; three `B.toggle` values own secondary-text visibility (initially true), masking (false) and expanded width (false). These are Bonsai state, while [`Label`](../../lib/core/label.mli) is a pure validated display model. `let%arr` reads current state and builds a new model and view. Button effects update the state later; they do not mutate a native label controller.

`primary` is “İstanbul · Élan · agent” and `secondary` is “agent notes · 世界”. Plain mode supplies no highlight; Prefix uses `Label.Match.prefix "i"`; All uses `Label.Match.all "AGENT"`. `Label.create` conditionally adds the secondary text, joining it to primary with one ASCII space, and applies masking when requested. Matching is computed when constructing the model, not on every native paint.

The matching contract uses Unicode scalar-by-scalar lowercase comparison, not locale-specific matching, normalization or full case folding. Prefix `i` can therefore color the whole original `İ` scalar despite its expanded lowercase representation. All `AGENT` colors both occurrences in the combined text; overlapping matches coalesce. A query must be valid UTF-8 and at most 4,096 bytes. Combined source and displayed output must each fit 262,144 UTF-8 bytes, and more than 4,096 foreground runs returns an error rather than truncating. The example’s `ok_exn` is appropriate for its trusted constants; an application should handle errors for external input.

`Presentation.styled_label` resolves the model into one selectable text flow with stable key `styled-label`, 20-pixel font, 28-pixel line height and width 300 or 540 pixels. Native GPUIO handles layout, selection and accessibility. This component acquires no assets, starts no Eio task and needs no controller disposal.

Select **Label: prefix i**, then **Label: all agent**, and turn secondary text off: the second match disappears with that source text. Enable **Mask label**: every Unicode scalar, including the joining space, becomes a bullet. The masked model retains no original source or query formatting, so default selection/copy and accessibility expose bullets. The original constants remain in this caller; masking is not secure erasure or a secret-storage facility. Width changes exercise wrapping without changing the text or selected match mode.

Adapt this by passing validated application text to `Label.create`, keeping semantic label content separate from appearance. Use `Label.Expert.to_text_content` with explicit secondary/highlight colors when you need its resolved runs elsewhere. Choose a different matching algorithm before construction if your product requires locale-aware search; do not interpret these demo modes as a general search engine or an editable field.
