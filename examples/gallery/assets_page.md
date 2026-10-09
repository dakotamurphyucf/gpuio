# Assets page: registered bytes, native decoding and current-value copy

[assets_page.ml](assets_page.ml) and its [interface](assets_page.mli) demonstrate SVG
and raster images, fit policies, decode failure/recovery, meaningful icons,
decorative button icons and copying application text. The component accepts the
application/window, a reactive palette and Bonsai graph; it returns a reactive view.

From the repository root:

```sh
GPUIO_JOBS=2 ./scripts/gpuio build examples/gallery/main.exe
./scripts/gpuio exec _build/default/examples/gallery/main.exe
```

Choose **Images & icons**. Switch between the landscape and gradient, choose a fit,
simulate failure and restore the image. Approve a sample, then copy the current
approval. There are no files or remote assets to fetch: all four fixtures are
in-memory bytes. There are no page-specific launch diagnostics. `--background` is
useful for layout checks, not focused keyboard validation. The
[development guide](../../docs/development.md) covers toolchain setup; this review
adds no native input, clipboard persistence, Linux GUI or VoiceOver acceptance.

Read `Sources`, `Assets.create`, `fit_label`/`state_label`, then `component`.
The aliases at the top distinguish responsibilities: `B` builds the Bonsai graph,
`E` describes effects, `V` builds GPUIO views, `Registered` owns scoped native
registrations and `Copy` builds clipboard controllers. `open Core` supplies the
collection, result and error helpers. `ok = Or_error.ok_exn`, `style` and `px` unwrap
validated fixture/configuration construction. These hard-coded demo values are
expected to be valid; handle user-supplied bytes, descriptions or sizes through
typed error results instead of adopting this exception shortcut indiscriminately.

`Sources.landscape` is colored SVG and `Sources.check` is a small white check path.
[Image_samples](image_samples.ml) supplies `gradient_pnm`, with its
[interface](image_samples.mli) documenting the fixture. `Assets.create` registers
landscape SVG, raster PNM, intentionally malformed PNM and check SVG in that order.
It chains effects with `E.bind`, stops on the first error and maps successful
registrations to immutable `Asset.Handle.t` values. Its record contains handles,
not decoded pixel buffers. The common visit scope owns the registrations, including
any already acquired before a later registration fails.

`Asset.Source.of_bytes` describes encoded input, and `Registered.register` completes
when encoded bytes are published natively. This is deliberately distinct from a
successful native decode: the malformed fixture can register, then fail when the
image reader attempts decoding. The [asset interface](../../lib/eio/asset.mli)
documents bounded pending uploads, cancellation and retained reader leases.
`Preview_scope.acquire` displays Loading/Failed/Ready and retires all four resources
on departure; returning reacquires them. See its [walkthrough](preview_scope.md).
A stored handle does not keep a cancelled registration alive.

## Reactive choices and GPUIO composition

`B.toggle` starts raster/broken at false; `B.state` starts fit at Contain and native
image state at Loading. These functions create reactive values and effects that
change them. `B.state_machine0` owns an approval count starting at zero; each unit
action increments it, saturating at `Int.max_value` rather than overflowing.
`let%arr` combines current palette/resources/choices/count/copy controllers into a
derived view. Effects execute on user events, independently of view construction.
For example, `let fit, set_fit = B.state Image.Fit.Contain graph` returns a reactive
`fit` and a reactive setter function. Inside `let%arr`, both names become their
current ordinary values: `set_fit candidate` constructs a `unit E.t` effect, which
the button executes on activation. `and` bindings declare the inputs to this one
derived computation; they are not a sequence of asynchronous operations.
`B.state_machine0` needs no reactive input to reduce an action: its reducer receives
the previous integer count and the unit action `()`. Both **Advance approval value**
and **Approve sample** send that same action, so their two count readouts agree.
`B.state Icon_transform_sample.Default graph` also owns the icon preset; its
setter and current value enter the same `let%arr` computation. The deactivation
hook resets the image observation to Loading, while the other
choices, count and copied notice remain retained.

The local `image` helper constructs `Image.Config` from a registered handle,
`Image.Description.label` and selected `Image.Fit`. `V.image` uses stable key
`main-gallery-image`, a responsive width and 210-pixel scaled height. Its `on_change`
setter stores native Loading/Ready/Failed observations; `state_label` displays pixel
dimensions and frame count only from Ready metadata. Contain/Cover/Fill/Scale_down/
None are public fit policies, not new source uploads.
Contain preserves aspect ratio while fitting the whole image, Cover preserves it
while covering the box, and Fill stretches to the box. Scale_down avoids enlarging
the image; None uses its intrinsic size. The caption names None **Intrinsic size**.
The image metadata uses pixel dimensions; `Palette.size p 210.` determines the
logical layout height, so these are separate quantities. The main image's key
identifies the view slot across source and fit changes, rather than identifying
which bytes it displays. Switching a fit changes the configuration without changing
the registered asset handle.

The thumbnail renders the raster with Cover. `V.icon` uses `Icon.Config` and an
accessible “Check mark” label, tinting the icon through Foreground. A separate
`Icon.Decoration` occupies the leading slot of **Approve sample**, sharing that
button's accessible name instead of creating another named target. Colored SVG
used as an image preserves its colors. Relevant contracts are
[image.mli](../../lib/core/image.mli), [icon.mli](../../lib/core/icon.mli) and
[asset.mli](../../lib/core/asset.mli).

The **Small details, clear actions** card now includes discrete icon transform
presets. [Icon_transform_sample](icon_transform_sample.md) supplies pure labels,
validated transforms and selected buttons; the page owns the reactive selection.
It computes one optional transform and forwards it with `?transform` to both
`Icon.Config.create` and `Icon.Decoration.create`. **Rotate icon**, for example,
sets the preset through its Bonsai setter, derives a clockwise 90° transform and
updates both icon descriptions using the existing check SVG handle. **Default
icon** supplies `None` and restores the default. The image fits, source handles,
approval count and assigned icon/control bounds remain independent of this
artwork transform. These controls add no timer or animation task.

Click **Simulate decode failure**. Its toggle effect sets `broken=true`; `let%arr`
selects `assets.invalid` ahead of the raster/landscape choice, retaining the image
view's key. Native decode emits Failed; `on_change:set_state` updates the readout.
**Restore image** selects a valid handle again, producing native loading/readiness
observations. Registration success, decode readiness and physical presentation are
three distinct facts; the status establishes only the observed decode state.

## Copy the latest application value

Two `Gpuio_eio.Clipboard.Copy.create` graphs receive reactive `Clipboard.Text` values.
The literal uses `B.return` for constant “Hello from GPUIO · λ 世界\n”; the current
copy uses `B.map approvals` to derive “Approval N · λ 世界\n”. `B.map` transforms a
reactive value, unlike a plain immediate `List.map`. A mapped `on_copied` callback
stores the actual copied text in the notice. `Copy.view` supplies ordinary native
buttons, labels, busy/copied feedback and error state. The page displays either
controller's error.

Click **Advance approval value**: `approve ()` is the state-machine injection
effect, making the count one and deriving the new current text. Clicking **Copy
current approval** reads that current value when the action is reduced, submits an
asynchronous native clipboard write and, on success, reports “Copied: Approval 1 …”.
The controller suppresses duplicate activations while busy and during two-second
copied feedback. The page's separate copied-text notice is not cleared by that
two-second deadline. Text changes or Bonsai deactivation fence obsolete feedback;
an already-submitted OS write cannot be undone. The shared Bonsai clock handles the
deadline; this page creates no polling fiber. See the
[clipboard interface](../../lib/eio/clipboard.mli) for UI-domain and shutdown rules.
Success means the native API was invoked, not guaranteed clipboard persistence.

To add an image, register validated encoded bytes within `Assets.create` and add
its handle to the record and view. For file/network input, obtain bytes with explicit
Eio capabilities before evaluating registration on the UI loop; this example has
no such I/O. To copy another current value, map its reactive model into validated
`Clipboard.Text` and reuse `Copy.create` instead of capturing stale text in a callback.

The adjacent [image fixture walkthrough](image_samples.md) explains the PNM
header, RGB byte generation and reuse without sharing native registrations.
