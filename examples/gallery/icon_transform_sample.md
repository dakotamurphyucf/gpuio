# Icon transform presets

[icon_transform_sample.ml](icon_transform_sample.ml) and its
[interface](icon_transform_sample.mli) define seven finite presets and their
ordinary button controls. The [Assets page](assets_page.md) owns selection and
applies the selected transform to its named check icon and decorative **Approve
sample** icon. This helper owns no Bonsai state, registration or asynchronous task.

From the repository root:

```sh
GPUIO_JOBS=2 ./scripts/gpuio build examples/gallery/main.exe
./scripts/gpuio exec _build/default/examples/gallery/main.exe
```

Choose **Images & icons**, then use the controls in **Small details, clear actions**.
These are discrete preset changes, not timed animation. The build uses the
[isolated toolchain](../../docs/development.md). This walkthrough describes code;
it does not establish native transform, keyboard, accessibility or Linux GUI
qualification.

Read the variant `t`, then `label`, `transform` and `controls`. Deriving `equal`
creates typed equality for selected-button styling. `label` provides each visible
button/caption string; `transform` returns `Icon.Transform.t option`:

| Preset | Transform |
| --- | --- |
| Default | `None`, leaving the optional constructor argument absent |
| Quarter_turn | Clockwise 90° via `Icon.Transform.rotate_degrees` |
| Mirror | Horizontal scale −1 |
| Stretch | Horizontal scale 1.4, vertical scale 0.7 |
| Offset | Translate 3 logical pixels right and 2 up |
| Combined | Scale −0.8/0.8, rotate clockwise 30°, translate 2 logical pixels right |
| Collapsed | Horizontal scale 0, collapsing artwork |

The [public icon interface](../../lib/core/icon.mli) defines the order: scale about
the assigned icon box center, rotate clockwise, then translate. Scale is
dimensionless, rotation uses degrees and translation uses logical pixels. Layout,
input and accessibility bounds stay at the assigned box. Negative scale reflects;
zero scale is valid. All constructor inputs must be finite; scale is bounded to
−64..64, rotation to −360..360 and translation to −16384..16384. The helper unwraps
`Or_error.t` with `Or_error.ok_exn` because these fixed demo values satisfy those
invariants. For editable values, handle validation errors before publishing a
configuration.

`None` means the constructor uses its default transform. `Some
Icon.Transform.identity` would explicitly request the identity. The sample
reconstructs `Icon.Config` and `Icon.Decoration` using `?transform`, OCaml's syntax
for forwarding an optional argument from an option value. Applications that retain
an existing config can use `Icon.Config.with_transform config None` to reset the
complete transformation; this sample does not call that function.

`controls` receives ordinary `Palette.t`, selected preset and an `on_select`
function returning a Bonsai effect. `List.map` constructs seven buttons in a
wrapping row; `equal selected preset` marks the current choice. Constructing
`on_select preset` describes the action; the button runs it on activation.

On the Assets page, `B.state Icon_transform_sample.Default graph` creates the
reactive selection and setter. Its `let%arr` reads their current values and passes
the setter to `controls`. Click **Rotate icon**: the native button event delivers
the setter effect asynchronously to the OCaml graph; Bonsai updates selection to
`Quarter_turn`; view derivation computes the 90° transform and forwards it to both
icon constructors. GPUIO then reconciles their new descriptions. The existing SVG
registration is reused, and the approval count changes only when an approval button
is activated. Choosing **Default icon** derives `None` for both icons. Neither
action changes the landscape image or gradient thumbnail.

Add a preset by extending `t`, `label`, `transform` and the explicit list in
`controls`. Keep numeric values validated and distinguish artwork motion from
layout motion: an offset icon keeps its original control bounds. The Assets page's
visit scope owns the SVG registration and retires it on departure; this pure
helper owns neither its bytes nor native reader leases.
