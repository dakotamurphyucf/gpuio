# Native popup icon evidence — OCH-41

Local macOS qualification, 2026-10-06. Source starts at `da4ae4c` plus the
implementation recorded in this evidence archive. This is native icon coverage,
not completion of the component catalog or macOS release.

[Source overlay, logs and captures](native-menu-icons-och41/evidence.tar.gz) are
retained with a [SHA-256 manifest](native-menu-icons-och41/manifest.json).

## Public behavior and ownership

`View.with_menu_item_icons` in Core and Bonsai attaches registered decorative SVGs
to positions in an opt-in platform context menu. Command text and accessible
names stay intact. Paths, formats and passive native slot admission are checked.
The Feedback gallery includes command/submenu icons; the positioned-menu example
demonstrates explicit clear/remount and asset release with public OCaml APIs.
Its [adjacent walkthrough](../../examples/menu_controller/component.md) explains
the independent Bonsai, controller, asset and native ownership layers.

The existing workers rasterize directly to the 16-point menu target. AppKit sees
ready alpha pixels, with no encoded-image parsing, synchronous OCaml callback or
GPUI atlas reservation. Its template bitmap payload is bounded to 8 MiB across
one active popup and 256 physical pixels per side. The snapshot remains stable
during tracking. Decode/resource failures omit decoration while keeping commands
usable. [Design and limits](../design/native-popup-menu.md#decorative-icon-contract-implementation-in-progress).

## Automated and actual native results

- The complete native TestPlatform/unit suite passes **985 tests, two existing
  skips**. New cases cover bitmap alpha/template/logical size and aggregate bounds;
  no atlas charge for native snapshots; accepted source release before decode;
  fixed-size rendering of a 10,000-pixel-intrinsic SVG; and rerasterization at
  density 3 through the existing lease after registration retirement.
  A presentation-transition regression first failed because changing the parent
  menu left its unchanged icon descendants in the old rendering mode. Synchronizing
  those descendants and resampling through their retained leases fixes the exact
  regression; the complete native suite passes again after that correction.
- The five native menu-content admission tests pass, including atomic rejection
  of active/labelled/non-icon content and invalid slot mutation. The public OCaml
  expect suite validates SVG-only handles, duplicate/missing/separator paths,
  receiver constraints, target retention and explicit clearing.
- Strict native Clippy passes with all targets and native image/canvas features.
- The existing real macOS controller walkthrough passes placement, same/other
  owner Busy, Close, selection, observer retirement and stale-definition behavior
  with the added artwork. This used the repository build before the final
  registration-error status-message refinement.
- A fresh installed-library consumer builds independently after the presentation
  transition fix and passes the complete
  icon driver: ready triangle, clear, restore, source release while tracking,
  retained icon on reopen, missing decoration for a newly mounted retired reader,
  actual keyboard command selection, and exit status zero.

Final installed binary SHA-256:
`f62867c546db1d00d04f2068d2abd47a73b863eb5a78ecc51766783ac38f2962`.
The final driver records a 123×54-point AppKit popup and a 2× pixel capture on the
local M1 Max/macOS 14.5 display. Ready/retained template shape agreement is
0.9921; cleared/new-retired-reader agreement is 0.6627. Physical ownership of the
captured menu region is checked before each capture. The test reaps its process;
it does not use or change clipboard or VoiceOver settings.

The first visual harness incorrectly counted foreground pixels in the icon
position: AppKit shifts the text into that position when all icons are removed.
Its retained failure image shows no icon. The corrected harness compares the
known triangle silhouette, distinguishing it from shifted text. Original failure
and corrected captures are retained, rather than reporting the original check
as a pass. Initial test-only compilation mistakes are also preserved in logs.

## Reproduction

```sh
GPUIO_JOBS=2 ./scripts/gpuio exec cargo test -p gpuio-native --locked \
  --features native-image-tests,native-canvas-tests --lib -j 2
GPUIO_JOBS=2 ./scripts/gpuio exec cargo test -p gpuio-native --locked --test menu_content -j 2
GPUIO_JOBS=2 ./scripts/gpuio exec dune build -j 2 @all @fmt @runtest
GPUIO_JOBS=2 ./scripts/gpuio exec cargo clippy -p gpuio-native --all-targets \
  --features native-image-tests,native-canvas-tests --locked -j 2 -- -D warnings
GPUIO_JOBS=2 python3 scripts/test_extension_consumer.py --example menu_controller \
  --workspace scratch/menu-icons-consumer
python3 scripts/test_native_menu_icons_macos.py \
  --binary scratch/menu-icons-consumer/consumer/_build/default/main.exe \
  --output scratch/menu-icons-native
```

The workflow now runs the icon driver alongside the native popup/controller
checks. Hosted results for this source are pending. Linux fallback retains icon
plus text; new-source Linux build/unit/consumer qualification remains required,
and actual Linux desktop qualification remains OCH-47. No VoiceOver, all-menu
family parity, native bar icon, nested icon pixel, multi-window editor-retargeting,
physical presentation-time or release signing acceptance is inferred here.
