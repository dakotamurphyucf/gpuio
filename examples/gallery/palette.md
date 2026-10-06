# Shared colors and small view helpers

[`Palette`](palette.ml) is an ordinary immutable OCaml value plus pure view
helpers. It is unrelated to the searchable `Command_palette` widget. Read the
[interface](palette.mli), `create`, `theme`, then `text`, `card` and `button`.

Run the gallery using the [application guide](application.md), then change
Light/Dark or Compact/Comfortable/Large. [Component](component.md) observes those
choices and calls `Palette.create`; this module has no Bonsai graph, effects of
its own, Eio I/O or native resource lifetime.

The private record stores six concrete colors, presentation appearance,
document appearance and a size factor. Light/Dark defaults come from integer RGB
values validated by `Color.rgb_exn`. An optional
[validated theme profile](model/theme_profile.md) replaces colors and presentation
appearance. Document/avatar appearance is derived from the effective Light/Dark
choice passed by the caller. `create` does not load or watch a theme file.

The [Appearance model](model/appearance.md) maps Compact, Comfortable and Large
to 0.85, 1.0 and 1.2. `size` multiplies logical dimensions by that factor.
This is application sizing, not a simulation of Retina/display DPI. Some shell
dimensions intentionally remain fixed; these helpers do not globally rescale
every value in the application.

`theme` exposes four named native tokens: background uses the **surface** color,
then foreground, accent and muted. This is narrower than the complete record.
`text` derives font size and foreground; `card` wraps supplied children with a
title, padding, border and background. `button` derives selected colors, disabled
policy and hover styling, forwarding the supplied click effect unchanged.

Follow a theme change: the native button invokes a Component effect, the theme
selection model changes, Bonsai derives a new palette, and pages call these pure
helpers with new values. The separate Component theme hook updates native tokens.
The helpers do not execute click effects or make callbacks from Rust paint.

To change the card design, edit the radius/padding in `card` and review small
windows and all size choices. To add a semantic color, add it to the record and
public interface, supply defaults/profile behavior and use it intentionally;
adding a native theme token is a separate choice. Keep colors consistent with
text contrast, and do not interpret matching screenshots as accessibility proof.
