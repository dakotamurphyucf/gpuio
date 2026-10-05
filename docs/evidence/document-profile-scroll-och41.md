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


## Public scroll profile — 2026-10-05

The independent public review package now includes `review-scroll`, a bounded
150-logical-pixel viewport with a three-step checklist and a native action. Its
Show review start/end buttons remain outside the viewport and support normal
Tab/Enter activation. They provide explicit keyboard alternatives to native wheel
input. The reader continues to own only its outer reveal; it does not reach into
this plugin's scroll handle.

Implementation/harness source: `9b92b18`, tested as a dirty worktree based on
`3216497`. Only the example package, gallery fixture and physical walkthrough
changed; no library, protocol, vendor or dependency changes. The existing
`Open_card` event byte4 and all schema/fingerprint/property bytes remain unchanged.
The final optimized gallery SHA-256 is
`c5f385adcae6d87b63d69b0c650266f95f117336f1144526c5866b0b863ae270`.

The plugin uses immutable occurrence-offset parser data and a native keyed
ScrollHandle in the document element namespace. The key includes source
generation. The handle survives consecutive mounted frames, not arbitrary
unmount/eviction; it holds no Window/App/transport. Current event guards wrap local
scroll mutations and queued activation, while scrolling emits no OCaml event.
The fixed checklist adds no unbounded source payload or per-frame work queue.

### Actual desktop validation

On macOS14.5 arm64, the expanded public profile walkthrough passes **11 cases**.
Existing code/table actions, inline/block Tab navigation, property-change pointer
activation and profile removal/remount continue to pass. Added native checks:

- Show end with keyboard Enter reveals the previously clipped action; Tab reaches
  it and Enter emits exactly one `Open_card` at source revision1.
- Show start restores the top offset. Tab from Show end leaves the composite
  instead of focusing the clipped action or staying on the same button. The
  observed exit is the native window anchor, recorded in the report.
- A real native wheel event over the inner viewport reveals the action. Further
  focus/traversal/redraws retain its offset, and Enter emits one more current event.
- Changing the profile's accent property preserves the mounted offset and yields
  one current action event through the replacement profile.
- Disabling and remounting the profile retires the old scroll state and starts
  at the top. Leaving the page releases all documents, images and source-byte
  registrations; the application closes and exits zero/reaped.

Exactly nine revision1 events are observed across the entire walkthrough, with
none from scrolling or traversal. The first native run passed the original scroll
checks; the second adds property/remount checks and strengthens the Tab-exit
assertion. This is not a production runtime repair. The viewport screenshot was
inspected for the two always-visible controls and independently clipped content.
No VoiceOver, OS preference, clipboard or input-source operation was performed.

Commands (repository's isolated environment, jobs2):

```sh
GPUIO_JOBS=2 ./scripts/gpuio exec cargo test -p gpuio-example-document --offline --locked -j2
GPUIO_JOBS=2 ./scripts/gpuio exec cargo clippy -p gpuio-example-document --all-targets --offline --locked -j2 -- -D warnings
GPUIO_JOBS=2 ./scripts/gpuio exec dune runtest examples/document_profile_package/test
GPUIO_JOBS=2 ./scripts/gpuio exec dune build --profile release examples/gallery/main.exe @fmt
python3 scripts/test_macos_document_profile.py --output scratch/agents/root-20261004-resumed/document-scroll-runtime-002
```

All pass: two Rust preparation/property tests, scoped OCaml expect tests, strict
Clippy, optimized gallery and formatting, and the native walkthrough. The Rust
preparation fixture confirms all three plugins are installed and custom fences
are consumed without becoming ordinary code blocks. Python compilation and
`git diff --check` also pass. Existing upstream deprecation and `block 0.1.6`
future-compatibility warnings remain.

The existing macOS public-profile CI step runs this expanded walkthrough on the
next pushed source. Current hosted run at999e531 predates this addition.
[Report](document-profile-scroll-och41/report.json),
[walkthrough log](document-profile-scroll-och41/walkthrough.log),
[application log](document-profile-scroll-och41/application.log), and
[capture](document-profile-scroll-och41/profile-inner-scroll-end.png) are retained.

This covers the example's explicit inner keyboard/wheel policy. It does not
establish automatic inner reveal for arbitrary plugins, physical trackpad momentum
or boundary gestures, VoiceOver, Linux compositor behavior or whole-gallery
acceptance. VoiceOver remains on the owner's hold; OCH-41/OCH-17 remain open.


### Independent installed consumer

`GPUIO_JOBS=2 python3 scripts/test_extension_consumer.py --example gallery --cleanup`
passes with the new example sources. It stages the public OCaml libraries outside
the checkout, copies the independent profile package including its new module,
generates the static backend, builds/links, and runs no-window catalog admission:
`GALLERY_CATALOGS_PASS counter=1 document_profile=1` and
`INDEPENDENT_EXTENSION_CONSUMER_PASS example=gallery run=False`.
The generated temporary workspace is removed on completion; no opam switch is
modified. The build is distinct from the earlier real desktop walkthrough, and
is not a fresh-machine claim. [Complete compressed build log](document-profile-scroll-och41/consumer-build.log.gz).
An initial command combining explicit `--workspace` and `--cleanup` was rejected
by argument validation before any build; the corrected invocation above passed.
