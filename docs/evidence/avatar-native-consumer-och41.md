# Avatar GPU and installed-gallery qualification — OCH-41

Local macOS checks on 2026-10-05 now pass the rich-avatar native fixture and a
fresh independent public-gallery consumer. Application source is `4792413`,
including the numeric disabled-style and intrinsic-layout repairs. The public
avatar harness adds palette pixel checks; no application code changed for them.
[Raw logs, screenshots and driver patch](avatar-native-consumer-och41/local-validation.tar.gz)
are indexed by [the hash manifest](avatar-native-consumer-och41/manifest.json).

## Native GPU coverage

`native_image_views` exits 0 after the ordinary-avatar and rich-avatar markers.
Actual GPU readback verifies fallback glyphs, source leases, SVG recovery and
resize/density, rectangular clipping of an oversized rich child, child-owned
corners and their hover refinement, primary raster/SVG/GIF replacement, retained
fallback recovery, decode failure, removal, ownership release and settled idle.
Live AppKit queries verify one semantic owner and hidden/decorative descendants.
Static checks use a background window; animated playback activates its window.

The same run also passes SVG/tint, button-icon, gradient, spinner, progress and
checkable-appearance fixtures. These include native clipping/alpha, owner retention,
reset and cleanup; the checkable fixture reports 21 size/value cases. Those
markers are retained in the raw log. They do not independently establish the
public OCaml interaction or installed-consumer coverage of those other families.
Synthetic native hover/input and scale adjustments are distinct from physical
pointer/keyboard delivery and physical monitor changes. No VoiceOver was used.

## Independent consumer

The consumer script stages the public OCaml packages under a fresh local prefix,
generates its static-extension backend, and builds the gallery from a separate
Dune project using the pinned lockfile/toolchain. It installs nothing into an
opam switch. Its catalog checks and `--run --gallery-section avatar-groups` pass.

Using that same installed executable, the expanded avatar driver passes **50
geometry/identity cases**, both themes, source/fallback transitions, limit/reorder,
keyboard overflow action, native retirement and remount. Added PNG readback
checks the public stable key `ada` (palette index 7, independently fixed by Core
expect tests): background, glyph foreground and border are found in both Light
and Dark. Sampling excludes the half obscured by the following avatar and uses
the actual window/pixel scale. Channel tolerance is six bytes; at least three
matching pixels per color are required. Observed counts are:

| Appearance | Background | Foreground | Border |
| --- | ---: | ---: | ---: |
| Light | 2,834 | 93 | 490 |
| Dark | 2,840 | 109 | 528 |

Screenshots were inspected. The independent palette audit still reproduces all
24 fixed color triples and their nominal contrast; the new render check samples
one identity in both appearances, not every identity or a calibrated display.

The same installed executable also passes the **41-case Rating walkthrough**,
including native keyboard/pointer requests, themes/sizes/maxima, owner retention,
read-only/disabled policy and page teardown. Each driver and application exits
successfully, with one owned GUI process at a time.

## Commands and limits

```sh
GPUIO_JOBS=2 ./scripts/gpuio exec cargo test --locked -p gpuio-native \
  --features native-image-tests --test native_image_views
GPUIO_JOBS=2 python3 scripts/test_extension_consumer.py --example gallery \
  --run --gallery-section avatar-groups \
  --workspace scratch/agents/root-20261004-resumed/gallery-installed-4792413
python3 scripts/test_gallery.py --section avatar-groups \
  --executable scratch/agents/root-20261004-resumed/gallery-installed-4792413/consumer/_build/default/main.exe \
  --images scratch/agents/root-20261004-resumed/gallery-installed-palette-001/images
python3 scripts/test_gallery.py --section rating \
  --executable scratch/agents/root-20261004-resumed/gallery-installed-4792413/consumer/_build/default/main.exe \
  --images scratch/agents/root-20261004-resumed/gallery-installed-rating-001/images
python3 scripts/audit_avatar_palette.py
```

This is one real macOS environment and a fresh installed consumer on the developer
machine. It is not clean-machine distribution, final-source hosted acceptance,
physical monitor migration, process/GPU resource benchmarking or whole-family
accessibility certification. Linux GUI remains deferred to OCH-47. VoiceOver
remains on the owner's hold. Milestone 07/OCH-41/OCH-17 remain open.
