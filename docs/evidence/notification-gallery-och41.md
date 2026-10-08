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
