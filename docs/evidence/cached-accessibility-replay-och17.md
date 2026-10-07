# Cached accessibility replay — OCH-17

Implementation after `2de36360`, local macOS arm64, 2026-10-07. This addresses
accessibility data disappearing when GPUI actually reuses a view's scene. It is
scoped cache evidence, not complete accessibility or release acceptance.

## Reproduction and implementation

The regression first establishes an active accessibility tree, then notifies
only the parent and draws without `Window.refresh`. The cached child's render
count stays unchanged, but its native button originally disappeared from the
tree. An earlier TextView fixture rerendered and did not prove cache reuse; it is
not counted as acceptance of the replay implementation.

GPUI prepaint/paint range indices now include accessibility ranges. Prepaint
replay copies the prior closed nodes under the current open parent, preserving
internal child links, duplicate-ID checks, focus mappings, bounds and debug
provenance. It uses the existing mutation journal for replayed focus/bounds so
prepaint rollback has the same restoration path. Active-descendant replay is
restricted to a target in that cached range and the current physical focus owner.

Paint replay moves mutable action callbacks from the previous frame and replays
Document selection claims. Final document-scope validation still runs on the
completed tree. The same ranges serve cached views and deferred draws. The cache
key includes accessibility activation, focus and inherited hidden/disabled
context, alongside the existing bounds, content mask and text style.

Raw semantic nodes are captured once per frame only when a cached view was used.
Inheritance and tree repair operate on the outgoing tree, so disabled state does
not contaminate the raw snapshot. Storage is bounded to current/previous frames;
there is no retained node snapshot per nested cached view. A full-suite regression
caught extra callback retention in ordinary uncached windows; those frames now
drop prior registrations immediately, preserving their earlier release behavior.
The follow-up [mixed-owner check](cached-accessibility-owners-och17.md) tightens
callback retirement in cache-containing frames: after all paint replay finishes,
unconsumed old callbacks are released in that removal frame.

Postorder parent semantics can hide a child without re-rendering it. Keep raw
registrations for replay, but reject explicit and fallback actions against the
completed tree's hidden/disabled set. This allows actions to return when the
ancestor is shown again without allowing hidden or disabled controls to act.

## Native regression coverage

Three tests exercise actual reuse, proven by unchanged render counters:

- Semantic Document/button identities, action delivery and a synthetic painted
  TextRun selection claim survive reuse; valid positions remain authorized by
  the completed Document scope.
- Repeated hidden/revealed ancestry and disabled inheritance retain identities,
  reject actions while unavailable and restore actions afterward. Unmounting
  retires actions and releases captured callback owners after two empty frames.
- Nested deferred content preserves nodes and repeated actions. Accessibility
  activation/reactivation, physical focus and dirty label changes remain current;
  notifying the child invalidates its cache, and subsequent parent-only updates
  reuse the new content.

These are native TestPlatform checks. The synthetic selection fixture is not
end-to-end cached TextView selection or OS screen-reader evidence. Dedicated
prepaint-retry/rollback tests, further active-descendant/context transitions,
TextView cache invalidation/selection and mixed cached/uncached owner-lifetime
qualification remain. Visual-line adjacency, VoiceOver and full catalog,
performance/resource, distribution/provenance and platform gates also remain.

The direct command `cargo test --offline --locked -j2 -p gpui --lib window::a11y`
cannot run here: Cargo rejects testing dev-dependencies of a package which is not
a workspace member. No upstream GPUI unit-suite pass is claimed. Production GPUI
code is exercised by the native regressions; a separate upstream-unit harness
would need an isolated manifest/dependency setup.

## Validation evidence

All **1,169 native tests pass**, with two isolated Linux private-bus fixtures
ignored on macOS. Strict workspace Clippy, Rust formatting, actual macOS
editor/document tests, full Rust workspace tests and gallery rebuild pass.
Vendor changes were also formatted explicitly with the pinned rustfmt because
the vendor crate is outside the workspace formatting target. Full Dune suites
were not repeated; their last complete evidence remains the earlier reader
checkpoint. No current-source Linux or physical presentation pass is claimed.

The GPUI fork reconstructs exactly from its pinned source archive and patch:
**159 files**, excluding Cargo.lock. Patch SHA-256:
`120a7fad2d586d2edef33e286ea111e8142fdfad12640b5718f2bc7f1afd06a6`.
Base is unchanged from the preceding flow-reveal checkpoint.

The rebuilt gallery also passes:

```sh
python3 scripts/test_macos_text_selection.py --rendered-only \
  --output scratch/agents/root-20261004-resumed/ax-cache-maintained-os
```

Actual OS AX text/ranges and native Copy agree for the heading, CJK, joined emoji
and code; the following caret remains stable. The owned app exits normally with
status 0, and captured clipboard representations are restored and verified.
VoiceOver and system settings were untouched. This rechecks the ordinary gallery
OS path; it does not turn the cache fixtures into OS/VoiceOver acceptance.
Executable SHA-256: `47ac47df8005a040bc5f33abad07c1a5c23b85b76e6244444c3d75728db32fe3`.

The [report archive](cached-accessibility-replay-och17/reports.tar.gz) and
[verified manifest](cached-accessibility-replay-och17/manifest.json) contain
35 files of source snapshots/hashes, regressions, commands, logs,
reconstruction and OS observations. OCH-17/OCH-41 and milestone 07 remain in progress.
