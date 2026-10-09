# Avatar overlap and intrinsic layout — OCH-41

On 2026-10-05 the physical public avatar gallery exposed an overflow indicator
painted over the first member instead of after the last member. Three fixed 48px
avatars with 30% overlap contributed zero intrinsic width to their row. The
failure reproduced in production-host TestPlatform and directly in pinned Taffy
0.13.0, independently of OCaml, image loading and the gallery.

The layout engine computes a negative intrinsic contribution by dividing by
`max(1, flex_shrink * inner_flex_basis)` but reconstructed it by multiplying by
`max(1, flex_shrink) * inner_flex_basis`. With shrink zero this magnified a negative
margin by the child size. The scoped [vendor patch](../../vendor/taffy/intrinsic-shrink-factor.patch)
uses the same factor for both operations. It leaves flexible free-space
allocation, the dependency version and public API unchanged. Avatar members keep
their stable keys and native identities; no manual group-width limit is added.

## Evidence

Base revision `724222e9ae8b66bd736f34dc4dbed7cd5b934561` plus the archived source
patch and additional source fixtures. [Raw archive](avatar-layout-och41/local-validation.tar.gz)
and [manifest](avatar-layout-och41/manifest.json) retain failures, passing logs,
source patches, upstream test lockfile and physical screenshots.

- Before: direct engine regression reports width 0 instead of 115.2px. A broader
  matrix also fails a 0.5px case, proving the defect is not confined to one avatar
  size. The original native/gallery geometry failures remain failures.
- After: 320 layout combinations pass (row/column, min/max-content constraints,
  four sizes, five shrink factors and four signed margins), checking parent size,
  child positions and fixed child dimensions.
- The full GPUIO native library suite passes **930 tests**, with two existing
  macOS private-bus skips. This includes the new production-host assertion that
  the separate overflow slot paints 4px after the last overlapping member.
- The exact upstream Taffy revision, with both local patches applied, passes
  **129 unit + 61 hand-written + 5,525 XML tests**. Four upstream hand-written
  tests remain ignored. These were run from a separate scratch checkout; no
  registry source or unrelated switch was modified. The generated dependency
  lockfile is archived. The repository's GPUIO tests separately exercise the
  production dependency graph.
- Reconstruction from the checksum-verified registry archive matches all 65
  upstream-listed files after both patches, without offsets/fuzz; the license
  checksum remains unchanged. Exact upstream Git source hashes also match before
  patch application.
- Strict native Clippy (library/tests with image/canvas test features and warnings
  denied), Rust formatting, Python syntax and the structural catalog audit pass.
- The rebuilt dev gallery passes the complete real macOS avatar walkthrough:
  **48 geometry/identity cases**, Light/Dark, five sizes, three overlaps,
  limits zero through five, reorder, six source transitions, rich-fallback owner
  retention, native keyboard activation of overflow, removal and page remount.
  Both driver and app exit successfully. Screenshots show the separate overflow
  correctly after the last member; this is not a full pixel-color/clipping audit.

The first repaired-gallery run passed geometry but failed an incorrect harness
expectation that leaving the page resets its Bonsai model. The corrected driver
checks the retained action count and fresh native avatar identities after the
page returns. It still verifies absence on departure and key retirement when the
limit removes members; it does not weaken native lifetime checks.

Physical environment: macOS 14.5 arm64, M1 Max, built-in display. Tested gallery
SHA256 `ce89d72c3b7edd65b580d1ad4ae925588c10d86c029578877b332c9d18dc6fa6`.

## Commands

```sh
GPUIO_JOBS=2 ./scripts/gpuio exec cargo test --locked -p gpuio-native \
  --test flex_intrinsic_size
GPUIO_JOBS=2 ./scripts/gpuio exec cargo test --locked -p gpuio-native \
  --features native-image-tests,native-canvas-tests --lib
GPUIO_JOBS=2 ./scripts/gpuio exec cargo test \
  --manifest-path scratch/agents/root-20261004-resumed/taffy-upstream-source/DioxusLabs-taffy-45a5629/Cargo.toml \
  -p taffy --lib --tests
GPUIO_JOBS=2 ./scripts/gpuio build examples/gallery/main.exe
python3 scripts/test_gallery.py --section avatar-groups \
  --executable _build/default/examples/gallery/main.exe \
  --images scratch/agents/root-20261004-resumed/gallery-avatar-groups-005/images
```

The reconstruction script is included in the archive and uses a verified local
registry archive read-only. The [maintenance note](../../vendor/taffy/GPUIO.md)
records both patches and rebase requirements.

## Remaining scope

Dedicated avatar pixel clipping/palette coverage, independent installed-consumer
qualification, final-source hosted checks and broader release/performance
acceptance remain separate. Previous performance binaries predate this layout
patch; their results retain their original source scope. Linux GUI qualification
remains OCH-47. VoiceOver is untouched and remains on the owner's hold.
OCH-41/OCH-17 and milestone 07 remain open.
