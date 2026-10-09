# Empty-state composition walkthrough

Read [empty_preview.ml](empty_preview.ml) and its [interface](empty_preview.mli). [pages.ml](pages.ml) mounts `component app window palette graph` on **Presentation**. `Empty` aliases [`Presentation.Empty_state`](../../lib/core/presentation.mli).

[Preview_scope](preview_scope.md) registers the [gradient PNM fixture](image_samples.md) through `Gpuio_eio.Asset.register`. `Effect.map` extracts a borrowed handle or formats registration errors. Loading/Failed branches render messages; only Ready constructs the demo. Registration is visit-owned even when image mode is off. Departure cancels pending acquisition and disposes registrations; later visits acquire fresh resources. Registration success and native image decode are separate stages.

Reactive toggles control slot presence and appearance; media/title/description/content/extras start visible, other toggles start false. `B.state` retains the latest image-state observation; `B.state_machine0` returns a current action counter and injector that reduces against the latest count when its effect runs. `let%arr` derives current views; constructing `click ()` does not run it. A native Create first item activation executes that unit action, increments the counter, derives a new readout and updates native text. It creates no actual collection item; guide/import actions use the same mock counter.

`Empty.header` assembles optional media/title/description slots only when any exists. Framed media chooses a star in the Icon frame, taking precedence over image mode; otherwise media is a named gradient image or two initials avatars. `Empty.title` combines text with an accent badge. `Empty.description` contains text and an ordinary button; it preserves relative line height when font changes unless the explicit 24-pixel line-height override is enabled. Content contains a checkbox/action; extras contain Import instead. `named` adds accessibility groups to rich slots.

`Empty.create` keeps stable header/content/extras wrappers under key `empty-preview`; removing one slot need not replace surviving controls. Root width is 440 or 240 pixels; leading alignment changes both item alignment and text alignment. Border width exposes the helper’s default dashed pattern; Solid is an independent override. Image `on_change` runs `set_image_state`, deriving a decode/status readout asynchronously. If media is removed, that stored observation is not evidence that an image remains painted.

Toggle title/description/extras around the content checkbox to exercise surviving state. **Keep empty-state updates** changes only that checkbox’s own checked model; it implements no persistence/freeze switch. The model is allocated outside slot selection, so hiding/showing content retains its boolean for this component visit. GPUIO owns native image/layout/control state, Bonsai the options/counters, and the scope the registered resource. Adapt slot builders to real onboarding content, replace mock actions with effects, and distinguish asset ownership, last decode observation and current visible media.

## Run and review

From the repository root:

```sh
./scripts/gpuio build examples/gallery/main.exe
./_build/default/examples/gallery/main.exe
```

The wrapper uses the repository toolchain. These commands are instructions, not checks executed for this documentation change. This component has no standalone executable or self-test. Native interaction requires the running gallery; compilation does not establish focus, keyboard, accessibility or platform acceptance. See [gallery instructions](README.md) and the [development environment](../../docs/development.md).
