# Native notification gallery and restoration race — 2026-10-08

The public Feedback example could lose a restored sample card when Show was
pressed during that card's animated exit. Native dismissal immediately removes
input/accessibility eligibility; its terminal callback reaches Bonsai after exit.
The example reused the still-mounted `layered-sample-N` key, so restoring the
same list during that interval did not reopen the retiring native lifetime.

`Feedback_state.Samples` now owns abstract batch/item identities. Show creates a
fresh batch and three fresh view keys. Dismiss filters the complete identity, so
a callback from an earlier batch cannot remove a replacement. Placement, palette
and margin updates retain keys. Only explicitly showing a batch restarts these
persistent demonstration cards. Counter exhaustion cannot wrap and reuse keys.
This is an example state-model repair; the Rust lifetime contract is unchanged.
The [walkthrough](../../examples/gallery/model/feedback_state.md) explains the
reducer and its Bonsai/native event trace.

## Local native result

On macOS 14.5 arm64, the new `--section notifications` walkthrough passes:

- All eight anchors in Light and Dark, with Motion both disabled and enabled:
  **32 geometry cases**, plus two asymmetric-margin cases. Measured cards have
  different heights, 360px width and 14px expanded gaps; centers/edges and inset
  offsets match. Existing AX owners survive placement and palette updates.
- Older collapsed paint layers are absent from the accessible tree. Focusing
  the named Notifications group expands them. Native Tab focuses the oldest
  source card's close button; leaf focus and its parent card are checked before
  Escape. Paint stacking does not reverse source-order traversal.
- Escape dismisses the intended card. Immediate Show restores it with a fresh
  owner, including the sequence that failed before the model repair. Page
  departure removes the stack and cards, returning recreates visible content,
  and the application closes normally with exit zero.

Both palette captures were inspected. This is settled geometry and native
keyboard/lifecycle evidence, **not** animation smoothness/FPS, VoiceOver,
physical IME, retained-editor streaming, reduced-motion, hover expiry, long-stack
scrolling or memory/idle-resource qualification. Those contracts retain their
separate native and release evidence; this does not close the whole family.

The first fixture attempt wrongly assumed newest-first Tab order and inverted
the theme-button convention. The second queried application-level focus, which
returned the hosting NSWindow. A short independent probe established oldest-first
Tab/Escape delivery; the final fixture checks the focused leaf and its parent.
The third run reproduced the real restoration race. The fourth passes after the
model fix. All attempts and owned-window cleanup remain in the evidence archive.

```sh
GPUIO_JOBS=2 ./scripts/gpuio exec dune build -j2 examples/gallery/main.exe @test/gallery/runtest @fmt
python3 scripts/test_gallery.py --section notifications --images <fresh-directory>
python3 scripts/audit_example_docs.py
python3 scripts/audit_component_catalog.py
```

The gallery build/expect/format checks pass, including the new stale-exit callback
regression. Documentation inventory, catalog audit, Python syntax, workflow lint
and diff checks pass. Source base is `b28db8b0` plus this change; exact executable
hashes and source snapshots are retained in the [archive](notification-gallery-och41/reports.tar.gz)
and [SHA-256 manifest](notification-gallery-och41/manifest.json).
The new four-minute Foundation step has not yet run on the hosted release tree.

## Independently installed consumer

A fresh staged public-library installation and independent gallery build also
pass, followed by the same complete native walkthrough (34 geometry cases plus
keyboard, restoration and departure). Both owned processes exit normally.

```sh
GPUIO_JOBS=2 python3 scripts/test_extension_consumer.py --example gallery --workspace <fresh-workspace>
python3 scripts/test_gallery.py --section notifications --executable <fresh-workspace>/consumer/_build/default/main.exe --images <fresh-directory>
```

This validates the public installed APIs and the example repair locally. It is
not a signed distribution, clean-machine or final-source hosted result.

## Hover, focus and active-time expiry follow-up

At `bda282a3` plus fixture/documentation changes, the real macOS
`--section notification-policy` walkthrough passes against both the repository
executable and the freshly installed consumer above. No application or runtime
code changed in this follow-up.

For both application Full and Reduced motion preferences, actual pointer hover
expands all three variable-height cards with the expected 14px gaps. Moving out
collapses them and removes the two older cards from accessible traversal. The
ordinary saved-preview notification then stays visible through more than 5.6
seconds of hover, exceeding its five-second active lifetime. Moving out allows
expiry, and the asynchronous callback updates the Bonsai status to No pending
notification. A second save repeats this with native accessibility focus inside
the close button, then focus outside the toast. This phase uses accessibility
focus actions, not physical typing. Application System motion is restored and
each owned process exits zero.

| Executable | Preference | Hover pause / expiry after leaving | Focus pause / expiry after leaving |
| --- | --- | --- | --- |
| Repository | Full | 5.64s / 5.34s | 5.70s / 5.31s |
| Repository | Reduced | 5.63s / 5.13s | 5.70s / 5.12s |
| Installed consumer | Full | 5.62s / 5.40s | 5.64s / 5.35s |
| Installed consumer | Reduced | 5.65s / 5.17s | 5.62s / 5.09s |

These elapsed observations include polling, native exit and asynchronous delivery;
they are not exact timer-deadline, input-to-paint or presentation measurements.
They qualify functional behavior under Reduced motion, not absence of every
intermediate animated frame. Smoothness, immediate reduced-motion pixels,
VoiceOver, physical IME, retained-editor streaming, long-stack scrolling and
memory/idle-resource qualification remain separate.

The initial attempt completed the behavioral phases but failed on a misspelled
final reset button. The next attempt's pointer guard rejected a toast center
still below the window during entrance: the point belonged to Finder. The fixture
now waits, within three seconds, for contained stable geometry before posting
hover; it does not retry a failed ownership assertion. Both failed attempts are
retained. No macOS setting, clipboard or input-source configuration changed.

```sh
python3 scripts/test_gallery.py --section notification-policy --images <fresh-directory>
python3 scripts/test_gallery.py --section notification-policy --executable <installed-gallery> --images <fresh-directory>
```

The [policy archive](notification-gallery-och41/policy-reports.tar.gz) and
[manifest](notification-gallery-och41/policy-manifest.json) retain the four runs,
reports, captures and exact fixture sources. Documentation inventory, Python
syntax, workflow lint and diff checks pass. A separate three-minute Foundation
step is added; hosted acceptance on the final release source remains outstanding.

## Native entrance geometry follow-up

At `a90400cb` plus fixture/documentation changes, both the repository and installed
galleries pass `--section notification-motion`. No application/runtime changes
were required. The fixture resolves the Show button, current front card and
retained Notifications group before starting a fresh batch. It discovers the
replacement card through direct children and samples its actual native AX bounds
for 1.2 seconds. This avoids consuming the entrance interval in whole-tree search.

In Full motion the first observed bottom-edge displacement is 95px, followed by
21 intermediate samples locally and 22 in the installed consumer, then zero at
the declared bottom anchor. Observed motion stays within the 96px entrance range
(allowing pixel rounding) and progresses toward the anchor without reversing.
In Reduced motion every observed sample is already at that same endpoint.
Both cases verify a fresh native identity rather than measuring an old card that
was already stationary. Application System motion is restored and both processes
close normally. No OS setting, input source or clipboard configuration changes.

These are sampled native accessibility geometries, not a count of distinct
presented frames, per-frame pixel evidence or physical FPS. They establish the
observed Full/Reduced entrance behavior; they cannot exclude an unsampled visual
defect or qualify complete animation smoothness/exit/reflow/streaming performance.
VoiceOver, physical IME, retained-editor streaming, long-stack scrolling and
memory/idle-resource requirements also remain separate.

```sh
python3 scripts/test_gallery.py --section notification-motion --images <fresh-directory>
python3 scripts/test_gallery.py --section notification-motion --executable <installed-gallery> --images <fresh-directory>
```

Both first attempts pass; the [motion archive](notification-gallery-och41/motion-reports.tar.gz)
and [manifest](notification-gallery-och41/motion-manifest.json) preserve exact
sources, executable hashes, all samples, captures and logs. Documentation/catalog
audits, Python syntax, workflow lint and diff checks pass. A separate two-minute
Foundation step is added; current-source hosted acceptance remains outstanding.
