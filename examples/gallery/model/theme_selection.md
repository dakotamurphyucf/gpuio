# Keeping a newer theme choice when an older load finishes

[`Theme_selection`](theme_selection.ml) is a pure state-transition module. Read
its [interface](theme_selection.mli), `Token`, `choose`, then `complete`. It stores
a preference, optional validated profile and opaque identity for the current
choice. The [theme card](../theme_preview.md) owns asynchronous work; this module
does not run Bonsai, Eio or a native widget.

Launch the [gallery](../application.md), choose a theme file, and use the normal
appearance buttons to see the state transitions. The initial state is explicit
Dark with no custom profile. The scope/token race is also covered by deterministic
[model tests](../../../test/gallery/theme_profile_test.ml), without requiring a
particular filesystem delay.

Every `choose` creates a new token and clears the custom profile. Selecting the
same appearance again is still a new user decision. Tokens are private `unit ref`
identities compared with `phys_equal`; callers receive only the abstract token
and its typed equality operation. This is deliberate identity comparison, not
polymorphic equality on the palette or accidental dependence on color values.

`complete t ~token result` ignores a result whose token differs from `t.token`.
A matching error preserves the entire last-good selection. A matching success
stores the validated profile, chooses its explicit Light/Dark appearance and
issues a fresh token. Thus two completions for one old request cannot both replace
the state. Independent windows' `initial` calls also have distinct identities.

For example, a file request captures token A. The user selects System, creating
B. When request A finishes, `complete` returns the System state unchanged. If no
new choice occurred, a successful profile becomes current, and Bonsai in the
caller derives updated view colors. The token guards relevance; the caller's Eio
scope separately guards task/resource lifetime.

`resolve` delegates to [Appearance.Preference](appearance.md). File paths, loading
spinners and error messages stay outside this model. To add a remembered recent
profile list, give that list its own persistence and ownership; do not reuse token
identity as a durable file ID. Tokens are process-local and should not be serialized.
Preserve fresh identity even when two user choices have equal colors.
