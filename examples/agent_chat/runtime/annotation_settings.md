# Confirm an annotation color without committing previews

[annotation_settings.ml](annotation_settings.ml) and
[annotation_settings.mli](annotation_settings.mli) implement Settings → Annotation
color. A native picker edits a temporary color; Apply annotation stores a
window-owned override used by run-diagram connectors. Cancel keeps the previous
accepted color. Empty means follow the theme accent, not transparent black.
No file is read/written, no network request occurs and no timer task is created.

Build/run from the repository root after [isolated setup](../../../docs/development.md):

```sh
GPUIO_JOBS=2 ./scripts/gpuio exec dune build examples/agent_chat/main.exe -j 2
_build/default/examples/agent_chat/main.exe
```

Open Annotation color, try a swatch or hex/channel edit, compare preview/confirmed
swatches, Apply and inspect the run diagram. Compare light/dark themes; Use theme
accent clears the override. macOS is the v1 target; Linux GUI checks remain
[informational](../../../docs/platform-release-policy.md).

## Value, palette and native draft

Read `create`, `value`, `describe`, `concrete`, then `component`. `t` is an
observable `Color_value.Value.t` initialized to `Value.Empty`; `value` exposes
it as a reactive Bonsai value. `Color of Rgba.t` is an accepted concrete color.
`describe` formats Empty as Theme accent and colors with `Rgba.to_hex`.
`concrete value palette` resolves Empty to that palette's accent and converts
an explicit color without dropping alpha.

`Color_input.Config.create` permits empty values and alpha. Its validated native
labels use Diagram annotation. Three literal palette entries are Iris #7C6FF0,
Rose #EF6B9580 and Mint #54C6A2; Rose's eight-digit hex value deliberately includes
alpha. Constructors validate these literals through `Rgba.of_hex` and
`Palette_entry.create`; arbitrary user values should handle errors rather than
copy the module's `Or_error.ok_exn` convention blindly. See
[color input](../../../lib/core/color_input.mli) and
[color value](../../../lib/core/color_value.mli) for editing/value contracts.

`Color_picker.create window` receives constant config, the controlled confirmed
value and a guarded on_change effect. The
[public picker contract](../../../lib/eio/color_picker.mli) gives each opening
its own Rust-owned color-input draft. Native channel previews/text commits do
not change the accepted application color. Confirm reads that draft and checks
the opening/current value, alpha/empty policies and editable validity before
calling on_change; invalid/composing text or an active drag cannot confirm.
The popup uses native hex/channel editors, not OCaml strings updated per key.

## Derive preview and fence retired page effects

`let%arr confirmed = value t and picker = picker and current = is_current and
dark = dark in ...` combines reactive current values into the settings view.
Reactive values cause dependent computations to update; this derivation builds
views rather than starting I/O or mutating the stored color. The accepted write
is an `Effect.of_thunk`, deferred until the UI loop and gated by `current ()`.

The derived preview uses `Picker.draft` only while open. Before the first
asynchronous draft snapshot arrives, it falls back to confirmed; it never assumes
an opening or window frame acknowledgement makes the snapshot available.
`Input.Snapshot.value` supplies the preview value when present. The two swatches
resolve colors over a raised background with border/padding and labels. Preview
may change rapidly while confirmed stays unchanged, making the ownership visible.

`Picker.view` supplies Choose annotation color, Apply annotation, Cancel
annotation and a 360-pixel overlay named Choose diagram annotation. Picker.error
shows Finish a valid color before applying. The surrounding settings group and
live-Off banner explain application behavior; they own no picker draft themselves.

In [settings.ml](settings.ml), `match%sub active Annotation` activates this
computation only for the annotation page. `match%sub` switches reactive
computation branches, rather than testing a one-time Boolean. Deactivation cancels
open picker drafts; the persistent `Annotation_settings.t` was created in the
outer window settings controller, so its accepted override survives. `is_current`
checks the outer settings epoch/open state; delayed retired-page effects cannot
save a color after the page/window settings visit changes.

## Trace Apply into diagram rendering

1. Choose annotation color opens a fresh native draft based on confirmed.
   Selecting Rose or editing channels changes only that draft and preview swatch.
2. Apply annotation reads/revalidates the draft through the picker. Its on_change
   effect checks current-page identity and publishes the accepted color variable.
3. `Settings.annotation` exposes that same reactive value to
   [Workspace.component](workspace.ml), which passes it through Inspector into
   [Diagram.component](diagram.ml).
4. Diagram's `Bonsai.Edge.on_change` compares active/theme/annotation values,
   updates its controller state and publishes a scene. `Run_diagram.scene` in
   [run_diagram.ml](run_diagram.ml) resolves the annotation and uses it for
   connector `Stroke.create` colors. This changes actual scene presentation,
   rather than just the settings preview.

Cancel/dismiss leaves accepted color intact. Use theme accent first cancels any
open picker, then guardedly sets Empty. An explicit color stays the same under
theme switches, while Empty resolves to the new theme's accent. Alpha remains
part of the accepted value; choosing a translucent color is not an opacity
animation or a lost-alpha conversion. Page deactivation/canceling a draft does
not release the accepted diagram scene/controller; those resources have their
own window scope. Native command submission/scene publication also does not
prove physical painting.

A small adaptation is another palette swatch: parse valid six/eight-digit hex,
give it a readable label and keep `Allow_alpha` if translucency is intended.
If storing colors externally, validate decoded values before accepting them and
represent Empty explicitly rather than substituting a concrete accent at save
time. Keep native drafts separate from the controlled confirmed value and retain
the stale-page guard when adding another Apply action.

`python3 scripts/test_agent_chat_dates_colors.py` is the optional macOS external
runner. Its [README](../README.md)/[recorded evidence](../../../docs/evidence/agent-chat-m5.md)
qualifies picker/diagram/theme checks; this source/link review performs none of
those native tests and adds no new acceptance claim.
