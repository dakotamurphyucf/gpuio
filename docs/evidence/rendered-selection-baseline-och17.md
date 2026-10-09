# Rendered Markdown selection baseline — OCH-17 / OCH-41

On macOS 14.5 arm64 / Apple M1 Max, the public gallery at clean `73b4e713`
reproduces missing rendered-document accessibility selection attributes.
Executable SHA-256:
`642b8767672883d22c74092fd8818150eec41cb3f7090d8246f5ecda989b24bb`.

The probe focuses the actual `Document content` AXGroup on the Markdown page,
sends native Command-A and Command-C, and observes the native pasteboard. Copy
returns rendered text, including Unicode, list/table content and the code fence's
text; it does not copy the `**Markdown**` source markers. The observed rich reading
tree preserves text order and the two uniquely named links. However:

- `AXSelectedText` is absent (`null`).
- `AXSelectedTextRange` is absent (`Missing AXSelectedTextRange`).

This is a missing accessibility interface despite working native selection/Copy.
It is separate from the source-editor adapter, whose real AX selection and shaped
geometry already have scoped evidence. It is not an OCaml/Bonsai or serialization
failure. Source inspection finds native TextView selection state and ordinary
Label/Link semantics, without the document TextRun/selection metadata required by
AccessKit's text-range support.

The [implementation plan](../design/rendered-document-selection.md) preserves the
rich semantic hierarchy, existing selection owners, source/interaction identities
and offscreen selection. No implementation or full VoiceOver acceptance is claimed.

[Probe source, application trace, output and machine-readable report](rendered-selection-baseline-och17/reports.tar.gz)
are retained with a [verified manifest](rendered-selection-baseline-och17/manifest.json).
The owned application closes normally (exit0). The outer clipboard guard restores
all captured readable representations and verifies equality. No VoiceOver or OS
settings changed. The probe uses actual AX queries and native keyboard input;
it does not test screen-reader navigation or Linux GUI behavior.
