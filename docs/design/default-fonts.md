# Native default monospace family

OCH-41 implementation contract, 2026-10-04. Application-selected font families
remain explicit style values; this policy applies only to the framework's
unconfigured monospace default.

Implemented locally; [validation and remaining qualification](../evidence/default-fonts-och41.md).

At native application initialization, after Base initialization and before any
window mounts, resolve the default once per GPUI application. Prefer Menlo on
macOS, then Monaco and Courier New; prefer DejaVu Sans Mono on Linux, then Noto
Sans Mono, Liberation Mono and Ubuntu Mono. Choose the first candidate listed by
the native text system. If none is listed, use GPUI's virtual `.SystemUIFont`.
That last resort keeps ordinary text usable but is not guaranteed monospace.

These candidate family names are not entries that pinned GPUI unconditionally
adds to its general fallback stack. Nevertheless enumeration is not a successful
glyph-shaping guarantee: corrupted fonts, unavailable glyphs and later system
font changes remain the native text system's responsibility. This is not font
installation, embedding, a platform font chooser or a per-frame filesystem probe.

Keep the result in Base's application-owned default typography token. Rich code
already uses that token; source/code/diff editor containers must read it as well
instead of independently hardcoding a platform family. A preconfigured nondefault
family is preserved without enumeration. Do not rewrite explicit per-view font
styles or alter window theme tokens. No new wire field or FFI callback is needed.

Validation must cover both platform candidate orders, absent defaults, no matching
candidate, explicit-family preservation, and initialization ownership/idempotence.
Native document/layout regressions still run separately. Unit/TestPlatform results
do not certify font availability or glyph appearance on clean macOS/Linux machines.
