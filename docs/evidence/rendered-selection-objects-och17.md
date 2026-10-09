# Atomic document selection and later-input cancellation — OCH-17

Local macOS 14.5 arm64 checkpoint after `4bbf7947`, 2026-10-07. This extends
[native selection dispatch](rendered-selection-dispatch-och17.md); it does not
certify full accessibility or release acceptance.

## Reproduced failures and repairs

An empty custom block shares a rendered byte offset with a structural separator.
Byte-only reveal normalization incorrectly selected the preceding glyph instead
of the block's distinct object edge. Preserve `(byte, object slot)` provenance
before normalizing separators. At a shared top-level boundary, noncollapsed
forward selection uses the preceding block; backward selection and carets use the
following block. Neither operation changes the logical selection.

A second regression delivers an accessibility selection, followed synchronously
by a newer wheel event before the next paint. The wheel moved the viewport to
`-32px`, but the stale reveal then moved it to `-1933px`. Document wheel, pointer
and keyboard handlers now cancel that pending reveal. Copy retains the selected
text. No OCaml protocol or synchronous OCaml callback was added.

## Behavior coverage

The added atomic-card fixture exercises nonempty and empty alternatives, rejected
interior positions, forward/backward selection and actual edge geometry for a
700px card inside a 200px viewport. Empty alternatives remain noncollapsed atomic
ranges. Switching to source-format Copy produces the original source without
changing endpoint identities; the embedded native button remains actionable.

The competing-input fixture places a flow document in a GPUI List. Its positive
control requires an offscreen head to reveal when no later input intervenes.
Separate wheel, right-pointer and Escape cases require the viewport to retain the
input's measured offset after the next paint, while Copy still yields `target`.
These are native test-context events, not physical OS input/VoiceOver evidence.

An initial plain-Div cancellation fixture passed even without cancellation code.
It was a false positive: plain Div never consumed the autoscroll request. The
corrected List fixture reproduced the failure before the fix. **Ordinary GPUIO
ScrollView selected-head reveal remains open**: its current scroll Frame also
has no descendant autoscroll consumer. It needs a host-level positive control and
geometry-consistent implementation. These list tests do not establish that
behavior, or cancellation for every nested control's propagation policy.

## Validation

All **1,164 native tests pass**, with two isolated Linux private-bus fixtures
ignored on macOS. Strict workspace Clippy, Rust formatting, full Rust workspace,
actual macOS editor/document tests and the gallery Dune rebuild pass. The complete
commands, logs and durations are in the archived `ax-objects-checked-results.json`.
Full Dune suites were not repeated; their last complete evidence is at the earlier
reader checkpoint. No new Linux qualification is claimed.

Base reconstructs exactly from its pinned archive and maintained patch across
**243 files**, excluding Cargo.lock. Patch SHA-256:
`71f79fe80f4ccd29760c8a2b5acd602a803b5d401bd8338974c089c2af70e709`.
The GPUI fork is unchanged in this checkpoint.

The rebuilt gallery passed:

```sh
python3 scripts/test_macos_text_selection.py --rendered-only \
  --output scratch/agents/root-20261004-resumed/ax-objects-maintained-os
```

Actual macOS AX mutation, selected text/range and native Command-C agree for the
heading `[33,17]`, CJK `[91,2]`, joined emoji `[96,11]` and code `[392,7]` (UTF-16).
A subsequent caret stays at `[392,0]`. The owned app exited normally with status 0;
captured clipboard representations were restored and verified. VoiceOver and
system settings were untouched. Executable SHA-256:
`dffd32f192036a5e49335a5de334b55e52e244926aa4dcbc29dccbb4e81d8a1a`.

The [report archive](rendered-selection-objects-och17/reports.tar.gz) and
[verified manifest](rendered-selection-objects-och17/manifest.json) preserve
24 files, including source snapshots/hashes, failures, final checks and OS report.

Ordinary ScrollView reveal, reset/remount and installed-consumer OS scenarios,
cached AX replay, visual-line adjacency and VoiceOver remain open. Catalog,
performance/resource, distribution/provenance and current-source platform gates
remain required. OCH-17/OCH-41 and milestone 07 remain in progress.
