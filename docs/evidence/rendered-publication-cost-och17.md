# Document accessibility publication cost — OCH-17

Local macOS arm64 change after `d7760dea`, 2026-10-07. This improves the
[logical document publisher](rendered-logical-document-och17.md); it does not
complete rich accessibility or release performance acceptance.

## Change

Selection validation previously scanned every prepared part to determine whether
either endpoint was inside an atomic object. Accessibility publication calls this
validation repeatedly when deriving semantic ranges. Parts are ordered and do not
overlap, so a binary search now finds the only possible containing interval for
each endpoint. Foreign positions still fail; shared boundaries, empty objects and
the existing scalar/CRLF rules retain their behavior.

Synthetic subtree construction also collects each owner's direct children
separately. Grouping a new label or semantic owner no longer scans all previously
completed document siblings. Native subtrees, actions, stable IDs and final reading
order are preserved. The pinned AccessKit representation requires setting a typed
empty child vector between groups: clearing the property prevents subsequent
`push_child` calls from restoring it. No AccessKit dependency change is needed.

## Diagnostic

On the Apple M1 Max / 32 GiB development Mac, the same unoptimized GPUI
TestPlatform fixture uses 60 introductory paragraphs
followed by an offscreen list. Each of three draws verifies complete prepared text,
unique and stable accessibility IDs, and fewer than 20 realized native blocks.
Input and preparation limits are unchanged.

| List items / AX nodes | Before: first / next two draws (ms) | After: first / next two draws (ms) |
| --- | --- | --- |
| 250 / 1,808 | 5.39 / 2.28, 2.34 | 3.03 / 1.68, 1.28 |
| 1,000 / 6,308 | 50.89 / 25.16, 24.24 | 11.51 / 5.49, 4.96 |

Grouping isolation alone left the larger fixture near its original timings. The
substantial measured reduction followed indexed endpoint validation. These are
three diagnostic draws per fixture, not a statistical benchmark, optimized build,
physical presentation measurement or proof of 120 fps.

## Validation scope

The native suite passes **1,141 tests, with 2 existing ignored tests**. A linear
reference check compares every valid position in mixed ordinary text, Unicode,
CRLF and atomic alternatives against the indexed validation, in both selection
directions. Existing empty-object edge, native control, identity, virtualization
and rollback checks remain in the suite. Strict all-target workspace Clippy passes.

GPUI Base reconstructs exactly from the pinned cached archive: **243 files**,
excluding Cargo.lock. Maintained patch SHA-256:
`1d3f25845a508b48ad1280d17cd2f210591a2880a7b68b6d7ae3caaa67e50a46`.

The full Rust workspace suite and `dune build -j2 @all @runtest @fmt` pass.
Actual macOS native editor/document checks pass, including the NSView text-input
client checks. The rebuilt gallery passes reading order, rendered Select All/Copy
and normal shutdown; the driver restores the typed clipboard and reaps its child.
Rust formatting, diff checks and the example documentation inventory also pass.

The [report archive](rendered-publication-cost-och17/reports.tar.gz) and
[manifest](rendered-publication-cost-och17/manifest.json) retain source hashes,
commands, logs, initial failed experiments and reconstruction evidence.

These desktop checks do not establish complete rendered OS text selection or
VoiceOver acceptance. Visible nested text gaps, final-paint selection and guarded
accessibility actions remain open. No current-source Linux acceptance is implied
by these local checks. OCH-17/OCH-41 and milestone 07 remain open.
