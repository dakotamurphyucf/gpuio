# Native Rating paint follow-up — OCH-41

The dedicated native presentation executable passes locally on macOS 14.5 arm64,
M1 Max, on 2026-10-05. Production source matches `883cb3c` (numeric disabled-style
repair plus intrinsic flex sizing correction); the run occurred before that
commit, with no subsequent source changes. [Log](rating-gpu-och41/native-presentation.log.gz)
and [manifest](rating-gpu-och41/manifest.json) retain hashes and exit status.

```sh
GPUIO_JOBS=2 ./scripts/gpuio exec cargo test --locked -p gpuio-native \
  --features native-image-tests,native-canvas-tests --test native_presentation
```

The executable exits 0 after these relevant completion markers:

- `GPUIO_RATING_GPU_OK`: fill/outline paint, maximum geometry and synthetic
  1x/1.5x/2x/3x density, restoring the original window size/scale.
- `GPUIO_RATING_APPEARANCE_OK`: independent active/outline/hover colors and alpha,
  inherited opacity/foreground, reset, stable native owner and value.
- `GPUIO_RATING_NATIVE_OK`: hover, ordered native key requests, AX actions,
  read-only/disabled, pointer/modal gates, idle and disposal.

The same suite also passes loading minimize/restore, reduced/static/hidden idle,
resume/disposal, semantic metadata updates, native link actions and retained
editor metadata/composition checks. Those input/composition helpers do not prove
actual OS IME candidate behavior. No VoiceOver testing/settings were used.

This supplements the [physical public Rating walkthrough](numeric-disabled-rating-och41.md).
Synthetic scale checks are not physical monitor transitions. A fresh installed
public-gallery consumer, final-source macOS/Linux hosted gates and broader
release acceptance remain separate. Linux GUI qualification is deferred to
OCH-47; VoiceOver remains on the owner's explicit hold.
