# Document preparation under worker pressure — OCH-17

Local macOS arm64, 2026-10-07, based on `ea6cfc6d`.
[CI run 37713213949](https://github.com/dakotamurphyucf/gpuio/actions/runs/37713213949)
at `88d3c3d8` failed the public document-profile walkthrough after changing
**Amber code highlights**. Its AX dump shows a `ResourceLimit` source fallback
and a Parse/Limit_exceeded event where **Open review card** should have been.
The preceding keyboard actions passed. This is preparation failure evidence,
not merely a button-discovery timeout.

## Reproduction and repair

A deterministic native regression submits two short profile documents while
holding the first worker's reservation. One request reserves **37,234,944 bytes**
of conservative preparation allowance against the unchanged **67,108,864-byte**
shared cap. The old scheduler immediately completed the second request with a
resource-limit error. Finishing the first worker would have reduced its charge
enough for the second request to fit, but the failed request was no longer pending.

The scheduler now leaves a request pending when an active worker temporarily
prevents admission. It continues scanning for other requests that fit. The existing
worker-completion path repumps the queue; no polling, extra worker or larger
memory budget is introduced. Waiting updates retain only the latest properties,
and dropping the view cancels admission. With no active worker, genuinely
insufficient retained capacity still reports the existing limit error.

This directly reproduces a scheduling defect capable of the CI failure. The CI
run did not record its exact reservation total at failure; the final-source hosted
walkthrough still needs revalidation. No historical presentation or scrolling
failure is attributed to this repair.

## Verified scope

- The original two-profile regression fails before the repair and passes after it.
  Two additional regressions verify latest-property replacement, dropped waiters,
  resource retirement, and persistent capacity exhaustion without invoking hooks.
- All **22 document job tests** pass. The full native suite passes **1,187 tests**
  with two existing skips. Strict workspace/all-target Clippy passes with native,
  canvas/image and presentation-diagnostic features. Formatting passes.
- The rebuilt public gallery passes the actual macOS profile walkthrough at both
  normal and Large density: **11 checks each**, including foreground keyboard
  actions, the changed-property pointer action, nested wheel scrolling, retained
  offsets, clipped-control Tab traversal, remounting and final source release.
- Both owned applications exit zero; clipboard contents are preserved and
  restoration verified around each run. VoiceOver settings are untouched.
  A Large-density capture was inspected: the native review controls render after
  the property change. This is not whole-gallery visual or VoiceOver acceptance.

Gallery executable SHA-256:
`31b890f4af199a9082ab5d46108aa349fcdab3bbf62a53aaa2eacdb33b99acad`.

```sh
GPUIO_JOBS=2 ./scripts/gpuio exec cargo test --offline --locked -j2 \
  -p gpuio-native --lib document_jobs:: -- --nocapture
GPUIO_JOBS=2 ./scripts/gpuio exec dune build -j2 examples/gallery/main.exe
GPUIO_JOBS=2 ./scripts/gpuio exec cargo test --offline --locked -j2 \
  -p gpuio-native --lib --features native-canvas-tests,native-image-tests
GPUIO_JOBS=2 ./scripts/gpuio exec cargo clippy --offline --locked -j2 \
  --workspace --all-targets \
  --features gpuio-native/native-tests,gpuio-native/native-canvas-tests,gpuio-native/native-image-tests,gpuio-native/presentation-diagnostics \
  -- -D warnings
python3 scripts/test_macos_document_profile.py --output scratch/profile-normal
python3 scripts/test_macos_document_profile.py --large-density --output scratch/profile-large
```

The recorded wrapper scopes both native commands with
`mac_clipboard.preserved_clipboard` and checks child exits. Exact commands,
source, reports, selected captures and the CI failures are in the
[evidence archive](document-worker-pressure-och17/reports.tar.gz), indexed by a
[SHA-256 manifest](document-worker-pressure-och17/manifest.json).

## Separate hosted Metal findings

The same CI run failed both presentation probes. GPUI recorded two 90-frame
sessions with all outcomes `Zero`, no presented/missing/saturated frames and no
pending callbacks at shutdown. The independent Swift/AppKit/Metal probe submitted
120 frames; all reported active and visible, and all had valid positive GPU
completion intervals, but every `presentedTime` was zero. The device was
`Apple Paravirtual device`; the captured display inventory was empty.

Apple's `MTLDrawable.h` contract says zero can mean a frame was not presented or
was skipped; GPU completion alone is not presentation. These reports establish
that this hosted environment did not qualify presentation timestamps. They do not
prove that every macOS VM lacks the capability, identify a renderer defect, or
qualify real-display latency. The raw reports remain failures; no CI gate or
performance threshold was relaxed. Presentation qualification remains open.
