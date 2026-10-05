# Numeric disabled styles and public rating walkthrough — OCH-41

Local macOS validation on 2026-10-05 repairs the Numbers & codes page rejecting
its initial transaction with `Malformed`. OCaml `Style.Disabled` encodes state 6;
the numeric part validator and renderer incorrectly used 7 (`Selected`). The
frame/editor and both step buttons now consistently accept and paint state 6.
Selected remains rejected. No wire format, public API or dependency change.

Source checkpoint: base `84117ea60aa68f957b60ac6bdf59d9816e328d9a` plus the exact
[archived source patch](numeric-disabled-rating-och41/local-validation.tar.gz).
That snapshot also includes a separate, unfinished avatar layout regression;
this evidence does not claim it passes. Archive hashes and environment are in
[the manifest](numeric-disabled-rating-och41/manifest.json).

## Checks and results

- Before the fix, `numeric_parts_accept_disabled_but_reject_selected` fails with
  `Malformed` for state 6. The previous real gallery run exits 2 on page mount;
  the driver exits 1 before reaching rating assertions. Both failures remain archived.
- After the fix, all 24 `number_` library tests pass. New admission coverage checks
  all four parts accept Disabled and reject Selected. Native TestPlatform paint
  verifies enabled → disabled → read-only → enabled, retained editor identity,
  disabled frame/editor/buttons and read-only buttons only. It accounts for the
  host's existing whole-control disabled opacity. Existing composition/history,
  repeat and application-step tests also pass.
- Strict native Clippy (library and tests, warnings denied), Rust formatting,
  Python syntax and the structural catalog audit pass. The catalog audit is
  structural evidence only.
- The dev gallery rebuild passes. Physical `--section rating` then passes 41
  geometry/identity cases across Light/Dark, three sizes and three maxima,
  independent color configuration, actual Home/Right keys and pointer clicks,
  click policy changes, read-only/disabled behavior and page retirement. The
  child and driver exit successfully. The screenshot was inspected; it shows
  the rating with independent fill/outline colors, but is not a pixel-color audit.

Commands (repository-isolated environment):

```sh
GPUIO_JOBS=2 ./scripts/gpuio exec cargo test --locked -p gpuio-native \
  --features native-image-tests --lib number_
GPUIO_JOBS=2 ./scripts/gpuio exec cargo clippy --locked -p gpuio-native \
  --features native-image-tests --lib --tests -- -D warnings
GPUIO_JOBS=2 ./scripts/gpuio build examples/gallery/main.exe
python3 scripts/test_gallery.py --section rating \
  --executable _build/default/examples/gallery/main.exe \
  --images scratch/agents/root-20261004-resumed/gallery-rating-002/images
```

Tested gallery SHA256:
`faeedb79eebd03092c9c159eb9163e68cf50865d8d62075a7601ce946475a93e`.
The harness now uses the current page name and explicitly selects the desired
appearance: the shell toggle names its current mode, not its destination.

## Limits

This qualifies the numeric style repair and the public rating walkthrough on one
real Mac. Numeric editing/IME/application-step physical qualification, dedicated
GPU color/alpha/hover assertions, fresh installed consumer, broader native suite,
new-source hosted gates and release acceptance remain separate. Linux desktop
qualification stays deferred to OCH-47. VoiceOver was untouched and remains on
the owner's explicit hold. OCH-41/OCH-17 remain open.

The dedicated native Rating paint suite subsequently [passed at the avatar-layout
follow-up](rating-gpu-och41.md); its evidence retains that newer source scope.
