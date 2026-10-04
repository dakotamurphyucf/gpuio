# Plugin-owned document scrolling — OCH-41

Local macOS arm64 checkpoint, 2026-10-04, on the dirty worktree based on
`83eb87e865c86717a8bc51b9db6fe1f379d909a9`. Production host, parser/profile worker,
reader and native elements run on GPUI TestPlatform. No OS windows were opened.

The test-only block plugin contains an independent 80-logical-pixel scroll area
with two native buttons separated by a 240-pixel spacer. The reader uses its own
180-pixel viewport. The regression verifies:

- Tab reaches the visible top button and leaves the reader without focusing the
  clipped bottom button or getting trapped in repeated reveal attempts.
- Reverse host traversal reenters at the document anchor. Forward traversal
  visits the visible control; reverse traversal from that control returns to the
  reader anchor.
- A wheel event over the inner area reveals the bottom button. Subsequent
  traversal and redraws retain that inner scroll position sufficiently to focus
  and activate the bottom button.
- Navigation emits no activation events. Enter emits exactly one payload with
  the current source generation and revision.

This is a qualification of the existing ownership boundary, with no production
runtime or vendor patch changes. An independent scroll area still needs its own
keyboard scrolling/reveal policy to expose all its contents without a pointer.
The deliberately minimal test fixture is not a complete accessible widget recipe.
See the [author contract](../design/document-profiles.md#plugin-owned-scroll-areas).

The initial fixture incorrectly expected reverse host traversal to jump directly
to a descendant. Source inspection showed that host reentry restores the reader
anchor, whose reverse traversal exits toward preceding toolbar controls. The
corrected test follows that existing contract; no production failure is claimed.

## Validation

Commands use `GPUIO_JOBS=2 ./scripts/gpuio exec`. Logs are local, ignored evidence
under `scratch/agents/root-20261003-release-notices/`, not build inputs.

| Command | Result | Log |
| --- | --- | --- |
| `cargo test -p gpuio-native --lib --features native-canvas-tests,native-image-tests --offline --locked -j2 profiles::scroll::` | One passed | `document-profile-scroll-native-005.log` |
| `cargo test -p gpuio-native --lib --features native-canvas-tests,native-image-tests --offline --locked -j2` | 919 passed; two existing macOS private-bus skips | `document-profile-scroll-full-001.log` |
| `cargo clippy -p gpuio-native --all-targets --features native-canvas-tests,native-image-tests --offline --locked -j2 -- -D warnings` | Passed | `document-profile-scroll-clippy-001.log` |
| `GPUIO_JOBS=2 ./scripts/gpuio check-fmt` (without `exec`) | Passed | `document-profile-scroll-format-001.log` |

The existing `block 0.1.6` future-compatibility warning remains. Five edited
documents pass relative-link checks, and the tracked whitespace check passes.
No OCaml, production Rust, dependency pins or maintained vendor patches changed;
the preceding OCaml/consumer evidence is not claimed as a new run here.

Physical keyboard, trackpad momentum/boundary gestures, VoiceOver, arbitrary
plugin composites and the final gallery/release acceptance remain open. This
checkpoint does not complete OCH-41, OCH-17 or milestone 07.
