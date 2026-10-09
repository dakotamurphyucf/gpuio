# Semantic colors for explicit light and dark appearance

[palette.ml](palette.ml) and [palette.mli](palette.mli) define the Agent Workspace's
pure color policy. `of_dark` returns an immutable record; `theme` translates a
subset into GPUIO theme tokens. There is no Bonsai state, native handle, I/O,
asset registration or system appearance query in this module.

From the repository root after [isolated setup](../../../docs/development.md):

```sh
GPUIO_JOBS=2 ./scripts/gpuio exec dune build examples/agent_chat/main.exe -j 2
_build/default/examples/agent_chat/main.exe
```

Toggle light/dark using the workspace controls; compare cards, native text
controls, source tree and results. macOS is the v1 target; Linux GUI coverage is
[informational](../../../docs/platform-release-policy.md).

Read `t`, `of_dark` and `theme`. The record names semantic roles: canvas/sidebar,
surface/raised, line, text/muted/faint, accent/accent_surface/accent_ink and
success. Callers use the role matching a UI purpose instead of scattering RGB
literals through each component. `of_dark dark` defines a local `c light night`
that chooses one checked-in RGB integer and calls `Color.rgb_exn`. The literal
values are valid colors; there is no runtime validation of user theme files or
palette persistence. The [color interface](../../../lib/core/color.mli) describes
color construction separately from native theme ownership.

`theme t` creates four tokens: background = surface, foreground = text,
accent = **accent_surface**, muted = muted. Native selected-control backgrounds
use the softer accent surface, while explicitly styled labels/strokes can use
`p.accent`. Do not equate a token named accent with this record's accent field;
that mapping is an intentional application policy. `Theme.create` validates the
mapping and its returned value owns no window resources. Read the
[theme interface](../../../lib/core/theme.mli).

In [workspace.ml](workspace.ml), `toggle_theme` updates the window's observable
dark Boolean. Its `Bonsai.Edge.on_change` callback returns a deferred effect
calling `App.Window.set_theme window (theme (of_dark dark))`. Components' reactive
`let%arr ... and dark = dark in ...` expressions independently derive explicit
Palette styles and Presentation.Appearance choices. A reactive value updates
its dependent views; an effect describes the window command, not another RGB
record. This is the concrete trace: command → dark state → native token update
and derived explicit styles → native presentation. Nothing in this helper
chooses the active theme automatically or changes OS preferences.

The Boolean is window-local, so another workspace can keep its own theme. Color
records can be rebuilt cheaply and need no cleanup; native window themes retire
with the window. [Annotation settings](annotation_settings.md) uses the current
palette's accent only when its accepted override is Empty; an explicit RGBA
color stays fixed across appearance changes. Native submission is distinct from
physically displayed contrast/rendering evidence.

A small adaptation is changing accent hues: update accent, accent_surface and
accent_ink coherently in both branches and review text/selection/stroke contrast
in actual UI. Adding a token requires deciding whether it replaces an explicit
style or complements it; merely adding a record field does not theme native
controls. Keep style policy pure and actual theme switching in the workspace
controller. The [README](../README.md) links current appearance/test evidence;
this source review adds no new visual or accessibility acceptance check.
