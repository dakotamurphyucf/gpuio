# Flow-document selection reveal — OCH-17

Local macOS 14.5 arm64 checkpoint after `132c253e`, 2026-10-07. This closes the
ordinary ScrollView reveal gap identified by the
[atomic-selection checkpoint](rendered-selection-objects-och17.md). It does not
certify complete accessibility or release acceptance.

## Reproduced failures

A production host fixture mounts a 100-line flow document in an ordinary
440×200 ScrollView. An actual native `SetTextSelection` action successfully
selected and copied `target` near the end, but the viewport stayed at offset 0.
The ordinary container did not consume GPUI's autoscroll request.

After precise reveal was connected, the newer-wheel regression exposed a second
path: the TextView canceled its older reveal correctly, but the host's automatic
focus reveal moved the viewport from the user's `-32px` offset back to 0. The
host now recognizes a handled wheel scroll as taking precedence for the focused
content inside that scroll ancestor.

## Implementation and ownership

`TextView::on_flow_reveal` observes its own measured selection/link request. It
stores the rectangle in the prepaint state and reports it during paint. It leaves
the request available to enclosing GPUI Lists, so their layout retries still
work and the callback receives final painted geometry. Requests from preceding
siblings are isolated during measurement.

The native host checks a weak presentation, interpretation identity, installed
source and live input policy before recording the node, focus handle and target.
After the entire paint, the focus manager matches the current eligible focused
entry and validates its full scroll/clip path before moving any owner. It reveals
the precise target even when focus has not changed. Offsets change after paint
and request a new frame; painting, hitboxes and accessibility keep consistent
geometry within each frame. Frame-local requests are consumed/reset and do not
cause idle snapback. No OCaml callback or protocol change was added.

On a handled wheel scroll, the focus manager suppresses older automatic reveal
only if the focused entry belongs to that scroll ancestor. Scrolling an unrelated
container does not cancel another focus owner's reveal.

## Regression scope

Two new production-host tests cover single and nested horizontal/vertical scroll
ancestors, repeated far/near/far selections with unchanged focus, exact Copy,
actual last-glyph bounds in every viewport, manual scrolling without snapback,
and the newer-wheel race. The existing GPUI List positive-control test now
installs the flow observer too, checking that observing does not steal its
request. Logical terminal separators remain geometry-free; assertions query the
last selected glyph without changing the selected range.

These are native TestPlatform results. Dedicated flow-document hard-clip cases,
other host input-cancellation paths, reset/remount and installed-consumer OS
scenarios remain to qualify. Cached AX replay, visual-line adjacency, VoiceOver,
full catalog/performance/resource/distribution/provenance and current-source
platform gates also remain open. OCH-17/OCH-41 and milestone 07 remain in progress.

## Validation evidence

All **1,166 native tests pass**, with two isolated Linux private-bus fixtures
ignored on macOS. Strict workspace Clippy, Rust formatting, full Rust workspace,
actual macOS editor/document tests and gallery Dune rebuild pass. Exact commands
and durations are in archived `ax-flow-checked-results.json`. Full Dune suites
were not repeated; their last complete evidence remains the earlier reader
checkpoint. No new Linux or physical presentation qualification is claimed.

Base reconstructs exactly from its pinned archive and maintained patch across
**243 files**, excluding Cargo.lock. Patch SHA-256:
`beb2f1c69be6594723cbbdaf7638252c84ba629f938d8e58c6824ab3fc7c738e`.
The GPUI fork is unchanged.

The rebuilt gallery passed the real macOS regression:

```sh
python3 scripts/test_macos_text_selection.py --rendered-only \
  --output scratch/agents/root-20261004-resumed/ax-flow-maintained-os
```

AX selected text/ranges and native Command-C agree for the heading, CJK text,
joined emoji and code; the subsequent caret remains stable. This rechecks the
existing OS range/Copy contract; the tall/nested ScrollView geometry checks above
are native test-context evidence, not a new OS scrolling probe. The owned app
exited normally with status 0, and captured clipboard representations were
restored and verified. VoiceOver and system settings were untouched.
Executable SHA-256: `4a2784cadfe80b63e7c857248a741fef38cfb881be019cf896e92eaa405d96ef`.

The [report archive](rendered-selection-flow-och17/reports.tar.gz) and
[verified manifest](rendered-selection-flow-och17/manifest.json) preserve
25 files of commands, logs, source hashes, reconstruction and OS observations.
