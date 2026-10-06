# Chart Studio's index adapter

[gallery.ml](gallery.ml) adapts the typed [shared fixture catalog](samples/gpuio_chart_samples.md)
to Chart Studio's integer chooser. It is synchronous pure Core code: it owns no
Bonsai state, native data, Eio task or window. This name refers to a dataset adapter,
not the separate component-gallery application.

Build and run its consumer from the repository root:

```sh
GPUIO_JOBS=2 ./scripts/gpuio build examples/charts/main.exe
./scripts/gpuio exec _build/default/examples/charts/main.exe
```

There are no assets or independent executable for this module. Ordinary launch
opens Chart Studio; `--background` avoids requesting focus. `--self-test` is optional
verification described in [main.md](main.md), not required to use the adapter.
The release is macOS-first; compilation does not qualify Linux desktop behavior.

Read the aliases `Samples` and `Preset`, then `family` and `names`, then the four
forwarding functions. `Preset` preserves the shared closed variant rather than
creating a second incompatible preset type. `names` derives an array from
`Samples.Family.all`; therefore the array labels and the integer-to-family mapping
have the same order. `family index` calls `Family.of_index`, converting its
`Or_error.t` to a value with `Or_error.ok_exn`. An invalid index raises. Valid chooser
and test indices are 0–6; this convenience is for internal example controls, not a
validated user-input parser.

`edge_data index` converts the index and returns that family's immutable edge
fixture. `preset_data preset index phase` forwards both the preset and typed family
to `preset_data_exn`; `description` forwards to the matching title generator.
`describe_selection` is a direct alias because its arguments already have the
public `Chart_data.t` and `Chart_selection.t` types. Read their
[shared interface](samples/gpuio_chart_samples.mli) and
[selection contract](../../lib/core/chart_selection.mli) for source-span and
publication invariants.

For a concrete interaction, click **Pie** in Chart Studio. `main.ml` generates that
button with index 3 from `names`; its `choose`/`load_family` handlers call
`Gallery.preset_data Standard 3 phase`. Here `family 3` becomes `Family.Pie`, and the
shared constructor returns the four fixed slices. The controller resets its scoped
registration, Bonsai derives the new chart configuration, and native preparation
later emits Ready. Clicking Reasoning emits a typed slice target; the controller
checks that the source is published before calling this adapter's
`describe_selection`, which returns “Reasoning · 44”. This helper neither delivers
the event nor checks publication itself.

To add another family, update the shared closed type, `Family.all` and exhaustive
constructor/label matches together; this adapter's array then follows automatically.
Also update the standalone self-test, whose loop currently visits indices 1–6 after
initial Line. To accept an externally supplied index, handle `Family.of_index`'s
error explicitly instead of exposing this raising `family` helper. Runtime ownership,
cleanup and Bonsai syntax are explained in the [application walkthrough](main.md).
