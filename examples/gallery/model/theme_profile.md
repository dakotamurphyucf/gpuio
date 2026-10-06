# A validated example theme format

[`Theme_profile.decode`](theme_profile.ml) turns a string into an abstract profile
or `Or_error`. Read the [interface](theme_profile.mli), `Colors`/`File`, then
`decode` and `presentation`. The parser is pure OCaml/Core code: it opens no files,
creates no Bonsai state and owns no native resource. The
[Eio adapter](../files/theme_file.md) reads bytes; the
[preview](../theme_preview.md) decides when a result may change the window.

Use the [gallery build/run commands](../application.md) and load a copy of
[`aurora.sexp`](../themes/aurora.sexp). This is GPUIO's **example** format, not
compatibility with another framework's theme JSON or a universal theme importer.

The serialized record has `version`, `name`, `appearance` and `colors`. Version
must be 1, appearance must be Light or Dark, and all ten colors are required:
background, surface, foreground, muted, accent, border, on_solid, success, warning
and danger. The typed S-expression decoder rejects unknown/duplicate fields.

Before typed conversion, `decode` enforces at most 16 KiB, valid UTF-8 without
literal NUL, one S-expression and a maximum parsed list depth of 16. The depth
check occurs **after parsing**; it is not a streaming parser depth limit. Names
must be nonblank and no more than 128 UTF-8 bytes, also without NUL. Errors from
parsing and typed conversion are values rather than an uncaught expected parse
exception.

Each color is parsed with the shared
[`Color_value.Rgba.of_hex`](../../../lib/core/color_value.mli): strict ASCII
3/4/6/8 hex digits, optionally prefixed by `#`. Colors are concrete values; the
string `accent` is not a token reference in this format. Errors identify the
color field. The internal generic `Colors.t` lets the wire record contain strings
and the validated profile contain `Gpuio.Color.t` values with the same names.

Accessors expose the basic colors and appearance. `presentation` builds the
full `Presentation.Appearance`, using background as its raised color and including
success/warning/danger/on_solid. [Palette](../palette.md) resolves how the profile
affects the gallery; changing the parser alone does not register global themes.

Follow Reload file: Eio supplies file bytes, this decoder produces a profile,
the preview checks the request identity, and `Theme_selection.complete` changes
the window's model. Bonsai then derives view/theme updates. An invalid color or
version leaves the previous palette in place through the caller's error policy.

To change the accent, edit only the sample's `accent` hex value and reload. To
extend the file schema, explicitly decide version compatibility, update the
validated representation/accessors and test missing/unknown fields and limits.
Do not silently accept arbitrary records or increase an input bound without
reviewing the loading/UI policy. The
[expect tests](../../../test/gallery/theme_profile_test.ml) cover malformed input,
last-good/stale selection and style changes without editor replacement; they do
not establish physical IME or screen-reader behavior.
