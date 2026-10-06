# Appearance preference and logical size

[`Appearance`](appearance.ml) is a pure model used by the gallery's
[component](../component.md) and [palette helpers](../palette.md). Read the
[interface](appearance.mli), top-level `Light | Dark`, then `Preference` and
`Scale`. This module has no Bonsai graph, native handle, I/O or mutation.

Build and launch through the [application guide](../application.md). Use the
Light/Dark button, Follow system and the size button to see its values in action.
It is an application appearance model, not an API that changes OS preferences.

`toggle` and `label` operate on the effective Light/Dark value. A preference is
either `Explicit appearance` or `System`. `Preference.resolve` keeps explicit
choices regardless of native observations. System follows
`Window.Appearance.is_dark`, using Dark until the first native snapshot arrives.
The gallery starts with an explicit Dark selection, not System.

`Scale` cycles Compact → Comfortable → Large → Compact. The factors are 0.85,
1.0 and 1.2, used by selected application size helpers in logical pixels. This
does not change the monitor scale, prove Retina behavior or resize every hardcoded
dimension. `all`, `next`, `label` and `factor` keep the finite choices explicit.

A size-button event updates the window's scale variable in Component. Bonsai
derives a new `Palette.t`; its view helpers use `Scale.factor` to change dimensions.
This module simply computes values in that interaction. It neither delivers
events nor owns a timer/task that needs cleanup.

To add an Extra_large choice, extend the variant and every exhaustive Scale
function, then inspect bounded windows and controls using the new factor. To add
a high-contrast theme, design its colors and native accessibility behavior
explicitly; mapping all dark native variants to Dark here does not constitute
high-contrast support. The [platform policy](../../../docs/platform-release-policy.md)
separates native qualification from successful builds.
